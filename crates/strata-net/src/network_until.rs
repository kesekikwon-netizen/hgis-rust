//! A curl launch that can be given up while it runs (the user cancels a download).

use std::io::{self, Read, Write};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

/// Reads a pipe of the child to its end in a thread of its own. With both pipes read while curl runs, a
/// large answer cannot fill a pipe and make curl wait for ever.
fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// As `run_curl_url_with_stdin`, through the same gate. While curl runs, `give_up` is asked every 50 ms;
/// when it says yes curl is ended at once and `Ok(None)` is returned.
pub(crate) fn run_curl_url_with_stdin_until(mut cmd: Command, url: &str, stdin_bytes: &[u8], give_up: &dyn Fn() -> bool) -> io::Result<Option<Output>> {
    if !super::is_allowed() {
        cmd = super::network_local::prepare_offline_local_curl(cmd, url, stdin_bytes)?;
    }
    if super::record_allowed_launch(&cmd, stdin_bytes) {
        return Err(super::dry_run_err());
    }
    strata_types::process::no_window(&mut cmd);
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_bytes)?;
    }
    let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if give_up() {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let (stdout, stderr) = (out.join().unwrap_or_default(), err.join().unwrap_or_default());
    Ok(status.map(|status| Output { status, stdout, stderr }))
}
