//! GET for the public map services the reference maps are fetched from (KIGAM geology, later NAAS soil and
//! VWorld rivers and parcels).
//!
//! C++ `ReferenceMapPreparation::download` (`target/cpp-ref/src/core/PreparedReferenceMap.cpp:45-93`): a
//! request is given up when no byte arrives for the idle time, and in any case after the total time; a
//! timeout and any other failure are told apart, because a slow answer is worth asking for in smaller
//! pieces and a refused one is not. The request goes through this crate's one gate (`network.rs`), so the
//! offline switch holds, and the address is handed to curl on its standard input, never on the command line.

use std::process::Command;

#[cfg(test)]
#[path = "reference_http_tests.rs"]
mod tests;

/// Why a request brought no body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceFailure {
    /// No byte for the idle time, or the total time ran out.
    Timeout,
    /// Not reached, refused, an HTTP error status, or the network is switched off.
    Network,
}

fn command(user_agent: &str, idle_secs: u64, total_secs: u64) -> Command {
    let mut cmd = Command::new(if cfg!(target_os = "windows") { "curl.exe" } else { "curl" });
    // No console window flashes over the app while curl runs.
    strata_types::process::no_window(&mut cmd);
    // --fail: an HTTP error status is a failure without a body (Qt reports it as an error to the C++).
    // --speed-limit 1 --speed-time N: given up when less than a byte a second arrives for N seconds, which
    // is the C++ inactivity limit; --max-time is its absolute limit.
    cmd.args(["-sS", "--fail", "--connect-timeout"]).arg(idle_secs.to_string());
    cmd.args(["--speed-limit", "1", "--speed-time"]).arg(idle_secs.to_string());
    cmd.arg("--max-time").arg(total_secs.to_string());
    cmd.args(["-A", user_agent]);
    cmd
}

/// curl ends with 28 when a time limit ran out ("Operation timeout", also for --speed-time).
fn failure_of(exit_code: Option<i32>) -> ReferenceFailure {
    if exit_code == Some(28) {
        ReferenceFailure::Timeout
    } else {
        ReferenceFailure::Network
    }
}

/// Fetches `url` and returns the body of a successful answer.
///
/// # Errors
/// `Timeout` or `Network`, see [`ReferenceFailure`].
pub fn reference_get(url: &str, user_agent: &str, idle_secs: u64, total_secs: u64) -> Result<Vec<u8>, ReferenceFailure> {
    let output = crate::curl_stdin::output_with_url_on_stdin(command(user_agent, idle_secs, total_secs), url).map_err(|_| ReferenceFailure::Network)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(failure_of(output.status.code()))
    }
}
