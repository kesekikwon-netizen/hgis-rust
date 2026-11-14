//! Config generation, `curl` process launch, `.part` staging, and `stderr` record reader.

use std::{
    collections::HashMap, ffi::OsStr, fmt::Write as _, fs,
    io::{self, BufRead, BufReader, Read, Write}, panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf}, process::{Child, ChildStderr, Command, Stdio},
    sync::{atomic::Ordering, mpsc::{self, Sender}, Arc, Mutex},
    thread, time::Duration,
};

use crate::curl_batch::{
    BatchError, BatchHandle, BatchItem, BatchResult, SharedState,
    EXIT_NO_RECORD, EXIT_RENAME_FAILED, MAX_PARALLEL, RECORD_PREFIX,
};

pub(crate) fn part_path(destination: &Path) -> PathBuf {
    let mut os = destination.as_os_str().to_os_string();
    os.push(".part");
    PathBuf::from(os)
}

pub(crate) fn normalize_match_path(raw: &str) -> String {
    let unified = raw.replace('\\', "/");
    let leading_unc = unified.starts_with("//");
    let mut segs: Vec<&str> = Vec::new();
    for (i, seg) in unified.split('/').enumerate() {
        if seg == "." || (seg.is_empty() && i > 0) {
            continue;
        }
        if seg == ".." && segs.last().is_some_and(|s| !s.is_empty() && *s != ".." && !s.ends_with(':')) {
            segs.pop();
            continue;
        }
        segs.push(seg);
    }
    let joined = if leading_unc { format!("/{}", segs.join("/")) } else { segs.join("/") };
    if cfg!(target_os = "windows") { joined.to_lowercase() } else { joined }
}

fn escape_quoted(raw: &str) -> String { raw.replace('\\', "\\\\").replace('"', "\\\"") }

pub(crate) fn build_curl_command(program: &OsStr) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(["--config", "-"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    strata_types::process::no_window(&mut cmd);
    cmd
}

pub(crate) fn build_batch_config(
    items: &[BatchItem],
    headers: &[(&str, &str)],
    timeout_per_file: Duration,
    parallel: usize,
) -> Result<String, BatchError> {
    if !(1..=MAX_PARALLEL).contains(&parallel) {
        return Err(BatchError::InvalidParallel(parallel));
    }
    if timeout_per_file.is_zero() {
        return Err(BatchError::InvalidInput("timeout_per_file must be positive"));
    }
    let (mut cfg, secs) = (String::new(), timeout_per_file.as_secs_f64().max(0.05));
    let _ = writeln!(cfg, "silent\ngloboff\nmax-redirs = 0\nparallel\nparallel-immediate\nparallel-max = {parallel}\nmax-time = {secs:.3}");
    for &(name, value) in headers {
        if name.trim().is_empty() || name.contains(['\r', '\n', '\0']) || value.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid request header"));
        }
        let _ = writeln!(cfg, "header = \"{}\"", escape_quoted(&format!("{name}: {value}")));
    }
    let _ = writeln!(cfg, "write-out = \"%{{stderr}}{RECORD_PREFIX}%{{http_code}}\\t%{{exitcode}}\\t%{{filename_effective}}\\n\"");
    let mut seen_dests: HashMap<String, usize> = HashMap::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        if item.url.is_empty() || item.url.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid item url"));
        }
        let part = part_path(&item.destination);
        let Some(part_str) = part.to_str() else {
            return Err(BatchError::InvalidInput("destination path is not valid UTF-8"));
        };
        if item.destination.as_os_str().is_empty() || part_str.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid destination path"));
        }
        if let Some(first_index) = seen_dests.insert(normalize_match_path(part_str), idx) {
            return Err(BatchError::DuplicateDestination { index: idx, first_index });
        }
        let _ = writeln!(cfg, "url = \"{}\"\noutput = \"{}\"", escape_quoted(&item.url), escape_quoted(part_str));
    }
    Ok(cfg)
}

/// Single launch point for the `curl` batch process.
fn spawn_curl_process(program: &OsStr, config: Vec<u8>) -> Result<(Child, ChildStderr), BatchError> {
    let mut child = build_curl_command(program).spawn().map_err(|e| BatchError::CurlStartFailed(e.to_string()))?;
    let (Some(stderr), Some(mut stdin)) = (child.stderr.take(), child.stdin.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(BatchError::CurlStartFailed("missing curl stdio pipe".to_owned()));
    };
    thread::spawn(move || { let _ = stdin.write_all(&config); });
    Ok((child, stderr))
}

pub(crate) fn start_with_program_checked<F>(
    program: &OsStr,
    items: &[BatchItem],
    headers: &[(&str, &str)],
    timeout_per_file: Duration,
    parallel: usize,
    allow_launch: F,
) -> Result<BatchHandle, BatchError>
where
    F: FnOnce(&[&str]) -> Result<(), String>,
{
    let config = build_batch_config(items, headers, timeout_per_file, parallel)?;
    let urls: Vec<&str> = items.iter().map(|i| i.url.as_str()).collect();
    allow_launch(&urls).map_err(BatchError::LaunchBlocked)?;
    let destinations: Vec<PathBuf> = items.iter().map(|i| i.destination.clone()).collect();
    let mut by_part: HashMap<String, usize> = HashMap::with_capacity(destinations.len());
    for (idx, dest) in destinations.iter().enumerate() {
        if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| BatchError::CreateDirFailed(e.to_string()))?;
        }
        let part = part_path(dest);
        let _ = fs::remove_file(&part);
        by_part.insert(normalize_match_path(&part.to_string_lossy()), idx);
    }
    let (tx, rx) = mpsc::channel();
    if items.is_empty() {
        drop(tx);
        return Ok(BatchHandle { shared: SharedState::new(destinations, None, true), rx: Mutex::new(rx), worker: Mutex::new(None), #[cfg(test)] pid: None });
    }
    let (child, stderr) = spawn_curl_process(program, config.into_bytes())?;
    #[cfg(test)]
    let pid = Some(child.id());
    let shared = SharedState::new(destinations, Some(child), false);
    let bg = Arc::clone(&shared);
    let worker = thread::spawn(move || run_reader(stderr, tx, by_part, &bg));
    Ok(BatchHandle { shared, rx: Mutex::new(rx), worker: Mutex::new(Some(worker)), #[cfg(test)] pid })
}

fn send_result(tx: &Sender<BatchResult>, shared: &SharedState, res: BatchResult) -> bool {
    shared.pending_results.fetch_add(1, Ordering::AcqRel);
    if tx.send(res).is_err() {
        shared.pending_results.fetch_sub(1, Ordering::AcqRel);
        return false;
    }
    true
}

pub(crate) fn run_reader<R: Read>(stderr: R, tx: Sender<BatchResult>, mut by_part: HashMap<String, usize>, shared: &SharedState) {
    let mut reported = vec![false; shared.destinations.len()];
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let (mut reader, mut raw) = (BufReader::new(stderr), Vec::new());
        while !shared.cancelled.load(Ordering::Acquire) {
            raw.clear();
            match reader.read_until(b'\n', &mut raw) {
                Ok(0) => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
                Ok(_) if shared.cancelled.load(Ordering::Acquire) => break,
                Ok(_) => {}
            }
            if let Some(res) = handle_record(&String::from_utf8_lossy(&raw), &mut by_part, shared) {
                reported[res.index] = true;
                if !send_result(&tx, shared, res) { break; }
            }
        }
    }));
    let panicked = outcome.is_err();
    if panicked { shared.reader_panicked.store(true, Ordering::Release); }
    let proc_code = finish_child(shared, panicked || shared.cancelled.load(Ordering::Acquire));
    let fallback_exit = if panicked { EXIT_NO_RECORD } else { proc_code.filter(|&c| c != 0).unwrap_or(EXIT_NO_RECORD) };
    if !shared.cancelled.load(Ordering::Acquire) {
        for (idx, &done) in reported.iter().enumerate() {
            if !done {
                shared.missing_records.fetch_add(1, Ordering::AcqRel);
                if !send_result(&tx, shared, BatchResult { index: idx, http_status: 0, exit_code: fallback_exit }) { break; }
            }
        }
    }
    cleanup_parts(&shared.destinations);
    shared.worker_done.store(true, Ordering::Release);
    drop(tx);
}

fn handle_record(line: &str, by_part: &mut HashMap<String, usize>, shared: &SharedState) -> Option<BatchResult> {
    let rest = line.trim_end_matches(['\r', '\n']).strip_prefix(RECORD_PREFIX)?;
    let mut fields = rest.splitn(3, '\t');
    let http_status = fields.next()?.parse::<u16>().unwrap_or(0);
    let mut exit_code = fields.next()?.parse::<i32>().unwrap_or(EXIT_NO_RECORD);
    let idx = by_part.remove(&normalize_match_path(fields.next()?))?;
    let dest = shared.destinations.get(idx)?;
    if shared.cancelled.load(Ordering::Acquire) { return None; }
    let part = part_path(dest);
    if exit_code == 0 && http_status == 200 {
        let ready = part.exists() || fs::write(&part, b"").is_ok();
        if !ready || fs::rename(&part, dest).is_err() {
            let _ = fs::remove_file(&part);
            exit_code = EXIT_RENAME_FAILED;
        }
    } else {
        let _ = fs::remove_file(&part);
    }
    Some(BatchResult { index: idx, http_status, exit_code })
}

pub(crate) fn finish_child(shared: &SharedState, kill_first: bool) -> Option<i32> {
    let mut child = shared.child.lock().ok().and_then(|mut g| g.take())?;
    if kill_first { let _ = child.kill(); }
    child.wait().ok().and_then(|s| s.code())
}

pub(crate) fn cleanup_parts(destinations: &[PathBuf]) {
    for dest in destinations { let _ = fs::remove_file(part_path(dest)); }
}
