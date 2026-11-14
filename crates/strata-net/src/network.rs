//! Process-wide network switch (`NET0c`).
//!
//! Normal desktop startup allows external HTTP requests. Automated runs (`STRATA_OFFLINE=1`,
//! `--snap` without `--snap-online`, `--verify-all`, `--smoke-quit`, `--smoke`) turn requests
//! off for the whole process before any work starts.

use std::cell::RefCell;
use std::io::{self, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Environment variable that disables external requests when set to `"1"`.
pub const OFFLINE_ENV: &str = "STRATA_OFFLINE";

static DISABLED: AtomicBool = AtomicBool::new(false);
static ALLOWED_LAUNCHES: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_PROBES: AtomicUsize = AtomicUsize::new(0);
static DRY_RUN_PROCESS: AtomicBool = AtomicBool::new(false);

/// Command captured by [`probe_dry_run`] without starting an OS process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
}

#[derive(Debug, Default)]
struct ProbeState {
    allow: bool,
    launches: usize,
    commands: Vec<RecordedCommand>,
}

thread_local! {
    static PROBE: RefCell<Option<ProbeState>> = const { RefCell::new(None) };
}

struct ProbeGuard;

impl Drop for ProbeGuard {
    fn drop(&mut self) {
        ACTIVE_PROBES.fetch_sub(1, Ordering::SeqCst);
        PROBE.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

/// Whether `STRATA_OFFLINE` disables requests (`Some("1")`).
#[must_use]
pub fn offline_from_env(value: Option<&str>) -> bool {
    value == Some("1")
}

/// Pure startup policy for CLI arguments and `STRATA_OFFLINE`.
#[must_use]
pub fn startup_allows_network<S: AsRef<str>>(args: &[S], offline_env: Option<&str>) -> bool {
    if offline_from_env(offline_env) {
        return false;
    }
    let has = |flag: &str| args.iter().any(|a| a.as_ref() == flag);
    if has("--verify-all") || has("--smoke-quit") || has("--smoke") {
        return false;
    }
    if has("--snap") {
        return has("--snap-online");
    }
    true
}

/// Applies [`startup_allows_network`] to the process switch at the start of `main`.
pub fn apply_startup_args<S: AsRef<str>>(args: &[S]) {
    let env_val = std::env::var(OFFLINE_ENV).ok();
    if !startup_allows_network(args, env_val.as_deref()) {
        disable();
    }
}

/// Turns external requests off for the rest of the process lifetime.
pub fn disable() {
    DISABLED.store(true, Ordering::SeqCst);
}

/// Whether external site requests are currently allowed.
#[must_use]
pub fn is_allowed() -> bool {
    if let Some(allow) = PROBE.with(|slot| slot.borrow().as_ref().map(|p| p.allow)) {
        return allow;
    }
    !DISABLED.load(Ordering::SeqCst)
        && !offline_from_env(std::env::var(OFFLINE_ENV).ok().as_deref())
}

/// Total number of external command launches allowed in this process.
#[must_use]
pub fn allowed_launches() -> usize {
    ALLOWED_LAUNCHES.load(Ordering::SeqCst)
}

/// Runs `f` on the current thread with a dry-run probe that never starts `curl`.
pub fn probe_dry_run<F: FnOnce()>(allow: bool, f: F) -> (usize, Vec<RecordedCommand>) {
    DRY_RUN_PROCESS.store(true, Ordering::SeqCst);
    ACTIVE_PROBES.fetch_add(1, Ordering::SeqCst);
    PROBE.with(|slot| {
        *slot.borrow_mut() = Some(ProbeState {
            allow,
            launches: 0,
            commands: Vec::new(),
        });
    });
    let _guard = ProbeGuard;
    f();
    PROBE.with(|slot| {
        slot.borrow_mut()
            .take()
            .map_or((0, Vec::new()), |s| (s.launches, s.commands))
    })
}

fn record_cmd(cmd: &Command, stdin: &[u8]) -> RecordedCommand {
    RecordedCommand {
        program: cmd.get_program().to_string_lossy().into_owned(),
        args: cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect(),
        stdin: stdin.to_vec(),
    }
}

fn check_and_record(cmd: &Command, stdin: &[u8]) -> Result<bool, io::Error> {
    if !is_allowed() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "network disabled",
        ));
    }
    ALLOWED_LAUNCHES.fetch_add(1, Ordering::SeqCst);
    let intercepted = PROBE.with(|slot| {
        let mut borrow = slot.borrow_mut();
        if let Some(probe) = borrow.as_mut() {
            probe.launches += 1;
            probe.commands.push(record_cmd(cmd, stdin));
            true
        } else {
            DRY_RUN_PROCESS.load(Ordering::SeqCst) || ACTIVE_PROBES.load(Ordering::SeqCst) > 0
        }
    });
    Ok(intercepted)
}

/// Runs `cmd.output()` when allowed (or records without spawning under [`probe_dry_run`]).
///
/// # Errors
/// Returns an error if network access is disabled or process execution fails.
pub fn run_output(cmd: &mut Command) -> io::Result<Output> {
    if check_and_record(cmd, &[])? {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "dry-run probe",
        ));
    }
    cmd.output()
}

/// Runs `cmd.spawn()` when allowed (or records without spawning under [`probe_dry_run`]).
///
/// # Errors
/// Returns an error if network access is disabled or process spawning fails.
pub fn run_spawn(cmd: &mut Command) -> io::Result<Child> {
    if check_and_record(cmd, &[])? {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "dry-run probe",
        ));
    }
    cmd.spawn()
}

/// Spawns `cmd`, writes `stdin_bytes` to its stdin, and waits for [`Output`].
///
/// # Errors
/// Returns an error if network access is disabled, spawning fails, or writing stdin fails.
pub fn run_output_with_stdin(mut cmd: Command, stdin_bytes: &[u8]) -> io::Result<Output> {
    if check_and_record(&cmd, stdin_bytes)? {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "dry-run probe",
        ));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes)?;
    }
    child.wait_with_output()
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod network_tests;
