//! Tests for [`super::no_window`] and workspace-wide `Command::new(` enforcement (`WIN1`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::no_window;

#[test]
fn no_window_runs_console_child_with_redirected_output_and_exit_code() {
    let mut ok_cmd = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", "echo strata-win1-ok"]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", "printf 'strata-win1-ok\\n'"]);
        c
    };
    no_window(&mut ok_cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let ok_out = ok_cmd.output().expect("run short console child (exit 0)");
    assert_eq!(ok_out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&ok_out.stdout).trim(), "strata-win1-ok");

    let mut fail_cmd = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", "echo strata-win1-err 1>&2 & exit /B 7"]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", "printf 'strata-win1-err\\n' >&2; exit 7"]);
        c
    };
    no_window(&mut fail_cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let fail_out = fail_cmd.output().expect("run short console child (exit 7)");
    assert_eq!(fail_out.status.code(), Some(7));
    assert_eq!(String::from_utf8_lossy(&fail_out.stderr).trim(), "strata-win1-err");
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn is_test_source_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("_tests.rs")
        || name.contains("_tests_")
        || path.components().any(|c| c.as_os_str() == "tests")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && !is_test_source_file(&path)
        {
            out.push(path);
        }
    }
}

/// Strips `//` comments, `/* */` comments, and inline `#[cfg(test)] mod ... { ... }` blocks,
/// returning `(line_number, cleaned_line)` pairs so line numbers stay true to the source file.
fn non_test_lines(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_block_comment = false;
    let mut pending_cfg_test = false;
    let mut skipping_test_mod = false;
    let mut mod_brace_depth: i32 = 0;

    for (idx, raw) in src.lines().enumerate() {
        let line_no = idx + 1;
        let mut clean = String::new();
        let bytes = raw.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if in_block_comment {
                if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    in_block_comment = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                break;
            }
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                in_block_comment = true;
                i += 2;
                continue;
            }
            clean.push(bytes[i] as char);
            i += 1;
        }

        let trimmed = clean.trim();
        if !skipping_test_mod {
            if trimmed == "#[cfg(test)]" {
                pending_cfg_test = true;
                continue;
            }
            if pending_cfg_test {
                if trimmed.starts_with("#[") {
                    continue;
                }
                if trimmed.starts_with("mod ") && trimmed.contains('{') {
                    skipping_test_mod = true;
                    mod_brace_depth = 0;
                    for ch in trimmed.chars() {
                        if ch == '{' {
                            mod_brace_depth += 1;
                        } else if ch == '}' {
                            mod_brace_depth -= 1;
                        }
                    }
                    pending_cfg_test = false;
                    if mod_brace_depth <= 0 {
                        skipping_test_mod = false;
                    }
                    continue;
                }
                pending_cfg_test = false;
            }
            out.push((line_no, clean));
        } else {
            for ch in trimmed.chars() {
                if ch == '{' {
                    mod_brace_depth += 1;
                } else if ch == '}' {
                    mod_brace_depth -= 1;
                }
            }
            if mod_brace_depth <= 0 {
                skipping_test_mod = false;
            }
        }
    }
    out
}

fn has_no_window_before_next_launch(lines: &[(usize, String)], idx: usize) -> bool {
    let start_line = &lines[idx].1;
    if start_line.contains("no_window(") {
        return true;
    }
    for (_, line) in lines.iter().skip(idx + 1).take(35) {
        if line.contains("Command::new(") {
            return false;
        }
        if line.contains("no_window(") {
            return true;
        }
        if line.starts_with('}') {
            return false;
        }
    }
    false
}

#[test]
fn every_non_test_command_new_is_prepared_with_no_window() {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    let mut src_files = Vec::new();
    let mut crate_dirs: Vec<PathBuf> = fs::read_dir(&crates_dir)
        .expect("read crates/")
        .flatten()
        .map(|e| e.path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    crate_dirs.sort();
    for dir in &crate_dirs {
        collect_rs_files(dir, &mut src_files);
    }

    let mut found: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut raw_flags: Vec<String> = Vec::new();

    for file in &src_files {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(file).expect("read source file");
        let lines = non_test_lines(&content);
        for (idx, (line_no, line)) in lines.iter().enumerate() {
            if line.contains("creation_flags(") && rel != "crates/strata-types/src/process.rs" {
                raw_flags.push(format!("{rel}:{line_no}"));
            }
            if line.contains("Command::new(") {
                let loc = format!("{rel}:{line_no}");
                found.push(loc.clone());
                if !has_no_window_before_next_launch(&lines, idx) {
                    missing.push(loc);
                }
            }
        }
    }

    assert!(!found.is_empty(), "expected non-test Command::new( sites in workspace");
    for loc in &found {
        println!("Command::new site: {loc}");
    }
    assert!(
        raw_flags.is_empty(),
        "raw creation_flags outside strata_types::process::no_window: {raw_flags:?}\nfound Command::new sites: {found:?}"
    );
    assert!(
        missing.is_empty(),
        "Command::new( without no_window(&mut cmd): {missing:?}\nall Command::new( sites found ({}): {found:?}",
        found.len()
    );

    let net_src = fs::read_to_string(root.join("crates/strata-net/src/network.rs"))
        .expect("read network.rs");
    for fn_sig in ["fn run_output(", "fn run_spawn(", "fn spawn_with_stdin("] {
        let start = net_src.find(fn_sig).expect(fn_sig);
        let body = &net_src[start..];
        let end = body.find("\n}\n").unwrap_or(body.len());
        assert!(
            body[..end].contains("no_window("),
            "strata_net::network `{fn_sig}` must apply no_window"
        );
    }
}
