//! Loopback helper for native snapshots, using the existing fixture's fake account and city.
//! Site behavior reference: target/cpp-ref/src/core/HeritageIntranetFlow.cpp:135-148 and
//! target/cpp-ref/src/app/KaHeritageBrowser.cpp:848-869. This helper creates no survey records.
//!
//! Build through Invoke-Cargo.ps1: cargo build -p strata-heritage-fixture --example serve.
//! Launch the resulting debug/examples/serve.exe directly, never with cargo run during --snap:
//! `Start-Process -FilePath $serveExe -WindowStyle Hidden -PassThru -ArgumentList
//! @('--lifetime-secs','120','--download-delay-ms','5000') -RedirectStandardOutput $urlFile`.
//! The first stdout line is the actual loopback origin; fake login is user1/pass1 and city is
//! 경상북도/경주시. Add --zip <explicit-temporary-synthetic.zip> to serve those exact bytes.
//! Retain the returned Process and stop only that owned process, or let its lifetime expire.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use strata_heritage_fixture::Fixture;

const USAGE: &str = "serve [--lifetime-secs 1..3600] [--download-delay-ms N] [--zip PATH]\n\
    Defaults: 120 seconds, no download delay, no ZIP; fake account user1/pass1, city 경상북도/경주시.\n\
    Build with cargo build -p strata-heritage-fixture --example serve, then run the executable directly.\n\
    The first stdout line is the loopback origin. ZIP input must be an explicit temporary synthetic fixture.";

fn main() -> ExitCode {
    match serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("serve: {why}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn serve() -> Result<(), String> {
    let mut lifetime_secs = 120_u64;
    let mut download_delay_ms = 0_u64;
    let mut zip = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help") => {
                println!("{USAGE}");
                return Ok(());
            }
            Some(flag @ ("--lifetime-secs" | "--download-delay-ms")) => {
                let raw = args
                    .next()
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                let value = raw
                    .to_str()
                    .and_then(|value| value.parse::<u64>().ok())
                    .ok_or_else(|| format!("{flag} requires an unsigned integer"))?;
                if flag == "--lifetime-secs" {
                    lifetime_secs = value;
                } else {
                    download_delay_ms = value;
                }
            }
            Some("--zip") => {
                zip = Some(PathBuf::from(args.next().ok_or("--zip requires a path")?));
            }
            _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
        }
    }
    if !(1..=3600).contains(&lifetime_secs) {
        return Err("--lifetime-secs must be between 1 and 3600".to_owned());
    }
    let zip_bytes = zip
        .map(|path| {
            std::fs::read(&path).map_err(|why| format!("cannot read ZIP {}: {why}", path.display()))
        })
        .transpose()?;
    let fixture = Fixture::start(("user1", "pass1"), ("경상북도", "경주시"));
    fixture.set_download_delay(download_delay_ms);
    if let Some(bytes) = zip_bytes {
        fixture.give_zip("지정유산", bytes);
    }
    println!("{}", fixture.base_url());
    std::io::stdout()
        .flush()
        .map_err(|why| format!("cannot flush fixture URL: {why}"))?;
    std::thread::sleep(Duration::from_secs(lifetime_secs));
    drop(fixture);
    Ok(())
}
