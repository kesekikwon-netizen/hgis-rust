//! Owned hidden helper process; target/cpp-ref/src/app/KaPrintDialog.cpp:620-669.
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const SCRIPT: &str = include_str!("../../../scripts/print-device.ps1");

pub(super) fn run(mode: &str, request: Option<&Path>, cancelled: &AtomicBool) -> Result<Vec<u8>, String> {
    let mut command = Command::new("powershell.exe");
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Sta", "-Command", SCRIPT]);
    command.env("STRATA_PRINT_MODE", mode).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(path) = request {
        command.env("STRATA_PRINT_REQUEST", path);
    }
    strata_types::process::no_window(&mut command);
    let mut child = command.spawn().map_err(|why| format!("Printer helper: {why}"))?;
    let stdout = child.stdout.take().ok_or("Missing printer output pipe")?;
    let stderr = child.stderr.take().ok_or("Missing printer error pipe")?;
    let read = |pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.take(16 * 1024 * 1024).read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let output = read(Box::new(stdout));
    let error = read(Box::new(stderr));
    let started = Instant::now();
    let timeout = Duration::from_secs(if mode == "Catalog" { 30 } else { 300 });
    let ended = loop {
        if cancelled.load(Ordering::Acquire) || started.elapsed() >= timeout {
            drop(child.kill());
            drop(child.wait());
            break Err("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.".to_owned());
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(why) => {
                drop(child.kill());
                drop(child.wait());
                break Err(format!("Printer helper: {why}"));
            }
        }
    };
    let bytes = output.join().map_err(|_| "Printer output reader stopped")?.map_err(|why| why.to_string())?;
    let errors = error.join().map_err(|_| "Printer error reader stopped")?.map_err(|why| why.to_string())?;
    let status = ended?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&errors).trim().to_owned());
    }
    Ok(bytes)
}
