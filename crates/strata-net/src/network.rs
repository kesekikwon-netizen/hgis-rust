//! Process-wide network switch (`NET0c`, `NET0c2`).
//!
//! Normal desktop startup allows external HTTP requests. Automated runs (`STRATA_OFFLINE=1`,
//! `--snap` without `--snap-online`, `--verify-all`, `--smoke-quit`, `--smoke`) turn external
//! requests off while permitting verified PC-local targets via
//! [`crate::curl_stdin::output_with_url_on_stdin`] and [`allow_launch_urls`].

#[cfg(any(test, feature = "test-probe"))]
use std::cell::RefCell;
use std::io::{self, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Environment variable that disables external requests when set to `"1"`.
pub const OFFLINE_ENV: &str = "STRATA_OFFLINE";

static DISABLED: AtomicBool = AtomicBool::new(false);
static ALLOWED_LAUNCHES: AtomicUsize = AtomicUsize::new(0);

/// Command captured by [`probe_dry_run`] without starting an OS process (tests only).
#[cfg(any(test, feature = "test-probe"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
}

#[cfg(any(test, feature = "test-probe"))]
#[derive(Debug, Default)]
struct ProbeState {
    allow: bool,
    launches: usize,
    commands: Vec<RecordedCommand>,
}

#[cfg(any(test, feature = "test-probe"))]
thread_local! { static PROBE: RefCell<Option<ProbeState>> = const { RefCell::new(None) }; }

#[cfg(any(test, feature = "test-probe"))]
struct ProbeGuard;

#[cfg(any(test, feature = "test-probe"))]
impl Drop for ProbeGuard {
    fn drop(&mut self) { PROBE.with(|slot| *slot.borrow_mut() = None); }
}

/// Whether `STRATA_OFFLINE` disables requests (`Some("1")`).
#[must_use]
pub fn offline_from_env(value: Option<&str>) -> bool { value == Some("1") }

/// Pure startup policy for CLI arguments and `STRATA_OFFLINE`.
#[must_use]
pub fn startup_allows_network<S: AsRef<str>>(args: &[S], offline_env: Option<&str>) -> bool {
    let has = |flag: &str| args.iter().any(|a| a.as_ref() == flag);
    if offline_from_env(offline_env) || has("--verify-all") || has("--smoke-quit") || has("--smoke") {
        return false;
    }
    if has("--snap") { has("--snap-online") } else { true }
}

/// Applies [`startup_allows_network`] to the process switch at the start of `main`.
pub fn apply_startup_args<S: AsRef<str>>(args: &[S]) {
    if !startup_allows_network(args, std::env::var(OFFLINE_ENV).ok().as_deref()) {
        disable();
    }
}

fn disable() { DISABLED.store(true, Ordering::SeqCst); }

/// Whether external site requests are currently allowed.
#[must_use]
pub fn is_allowed() -> bool {
    #[cfg(any(test, feature = "test-probe"))]
    if let Some(allow) = PROBE.with(|slot| slot.borrow().as_ref().map(|p| p.allow)) {
        return allow;
    }
    !DISABLED.load(Ordering::SeqCst)
        && !offline_from_env(std::env::var(OFFLINE_ENV).ok().as_deref())
}

/// Total number of command launches allowed by this module in the current process.
#[must_use]
pub fn allowed_launches() -> usize {
    ALLOWED_LAUNCHES.load(Ordering::SeqCst)
}

/// Decides from `url` alone whether a target cannot leave this PC (`NET0c2`).
#[must_use]
pub fn is_local_target(url: &str) -> bool {
    if url.is_empty() || url.chars().any(char::is_control) {
        return false;
    }
    let Some((scheme, rest)) = url.split_once("://") else { return false };
    if scheme.eq_ignore_ascii_case("file") {
        // No `\` and no `%`: curl decodes `%5C` / `%2F` in a file path, and `/\host/share` is a network share (NET0c5).
        let plain = !rest.contains('\\') && !rest.contains('%');
        let Some((auth, path)) = plain.then(|| rest.split_once('/')).flatten() else {
            return false;
        };
        return (auth.is_empty() || auth.eq_ignore_ascii_case("localhost"))
            && !path.is_empty()
            && !path.starts_with('/');
    }
    if !scheme.eq_ignore_ascii_case("http") || url.chars().any(char::is_whitespace) {
        return false;
    }
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    if authority.is_empty() || authority.contains('@') || authority.contains('\\') {
        return false;
    }
    let (host, port_opt) = if authority.starts_with('[') {
        let Some(close) = authority.find(']') else { return false };
        let after = &authority[close + 1..];
        let port = if after.is_empty() {
            None
        } else if let Some(p) = after.strip_prefix(':') {
            Some(p)
        } else {
            return false;
        };
        (&authority[..=close], port)
    } else if authority.contains(['[', ']']) {
        return false;
    } else {
        match authority.split_once(':') {
            None => (authority, None),
            Some((h, p)) => (h, Some(p)),
        }
    };
    if let Some(port) = port_opt {
        if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) || !port.parse::<u16>().is_ok_and(|p| p > 0) {
            return false;
        }
    }
    host == "127.0.0.1" || host == "[::1]" || host.eq_ignore_ascii_case("localhost")
}

/// Checks whether a launch for `urls` is allowed (all local when offline, or switch on) and
/// increments [`allowed_launches`] when allowed (`NET0c2`).
///
/// When [`is_allowed`] is `false` and this returns `true` (an offline local-only launch), the
/// caller must wrap its `curl` command with [`local_curl_guard_args`] (`first` before every other
/// argument and `last` at the end) and pass no redirect flags (`NET0c3`).
pub fn allow_launch_urls<I, S>(urls: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let allowed = is_allowed() || {
        let (mut any, mut all_local) = (false, true);
        for u in urls {
            any = true;
            if !is_local_target(u.as_ref()) {
                all_local = false;
                break;
            }
        }
        any && all_local
    };
    if !allowed {
        return false;
    }
    ALLOWED_LAUNCHES.fetch_add(1, Ordering::SeqCst);
    #[cfg(any(test, feature = "test-probe"))]
    PROBE.with(|slot| {
        if let Some(probe) = slot.borrow_mut().as_mut() {
            probe.launches += 1;
        }
    });
    true
}

/// Runs `f` on the current thread with a dry-run probe that never starts `curl` (tests only).
#[cfg(any(test, feature = "test-probe"))]
pub fn probe_dry_run<F: FnOnce()>(allow: bool, f: F) -> (usize, Vec<RecordedCommand>) {
    PROBE.with(|slot| *slot.borrow_mut() = Some(ProbeState { allow, launches: 0, commands: Vec::new() }));
    let _guard = ProbeGuard;
    f();
    PROBE.with(|slot| slot.borrow_mut().take().map_or((0, Vec::new()), |s| (s.launches, s.commands)))
}

#[cfg(any(test, feature = "test-probe"))]
fn record_cmd(cmd: &Command, stdin: &[u8]) -> RecordedCommand {
    RecordedCommand {
        program: cmd.get_program().to_string_lossy().into_owned(),
        args: cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect(),
        stdin: stdin.to_vec(),
    }
}

fn record_allowed_launch(cmd: &Command, stdin: &[u8]) -> bool {
    let _ = (cmd, stdin);
    ALLOWED_LAUNCHES.fetch_add(1, Ordering::SeqCst);
    #[cfg(any(test, feature = "test-probe"))]
    {
        PROBE.with(|slot| {
            slot.borrow_mut().as_mut().is_some_and(|probe| {
                probe.launches += 1;
                probe.commands.push(record_cmd(cmd, stdin));
                true
            })
        })
    }
    #[cfg(not(any(test, feature = "test-probe")))]
    {
        false
    }
}

fn check_and_record(cmd: &Command, stdin: &[u8]) -> Result<bool, io::Error> {
    if !is_allowed() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "network disabled"));
    }
    Ok(record_allowed_launch(cmd, stdin))
}

fn dry_run_err() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, "dry-run probe")
}

/// Runs `cmd.output()` when allowed.
pub fn run_output(cmd: &mut Command) -> io::Result<Output> {
    if check_and_record(cmd, &[])? { return Err(dry_run_err()); }
    cmd.output()
}

/// Runs `cmd.spawn()` when allowed.
pub fn run_spawn(cmd: &mut Command) -> io::Result<Child> {
    if check_and_record(cmd, &[])? { return Err(dry_run_err()); }
    cmd.spawn()
}

fn spawn_with_stdin(mut cmd: Command, stdin_bytes: &[u8]) -> io::Result<Output> {
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes)?;
    }
    child.wait_with_output()
}

/// Spawns `cmd`, writes `stdin_bytes` to its stdin, and waits for [`Output`].
pub fn run_output_with_stdin(cmd: Command, stdin_bytes: &[u8]) -> io::Result<Output> {
    if check_and_record(&cmd, stdin_bytes)? { return Err(dry_run_err()); }
    spawn_with_stdin(cmd, stdin_bytes)
}

/// Runs `cmd` with `stdin_bytes` for `url`, allowing verified local targets when offline (`NET0c2`–`NET0c4`).
///
/// Crate-private so [`crate::curl_stdin::output_with_url_on_stdin`] is the only public entry point;
/// offline local launches also verify `stdin_bytes` equals [`crate::curl_stdin::curl_url_config`].
pub(crate) fn run_curl_url_with_stdin(
    mut cmd: Command,
    url: &str,
    stdin_bytes: &[u8],
) -> io::Result<Output> {
    if !is_allowed() {
        cmd = network_local::prepare_offline_local_curl(cmd, url, stdin_bytes)?;
    }
    if record_allowed_launch(&cmd, stdin_bytes) { return Err(dry_run_err()); }
    spawn_with_stdin(cmd, stdin_bytes)
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod network_tests;

#[path = "network_local.rs"]
mod network_local;
pub use network_local::local_curl_guard_args;
