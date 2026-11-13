//! README_submit.txt, the survey fingerprint and MANIFEST.sha256 of the submit package
//! ≡ cpp `SubmitReadme.cpp:28-31, 92-142` and `ExportService.cpp:23-72`.

use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

/// What README_submit.txt says (`SubmitReadme::Inputs`).
#[derive(Debug, Clone, Default)]
pub(crate) struct ReadmeInputs {
    pub(crate) app_line: String,
    pub(crate) encoding: String,
    pub(crate) survey_name: String,
    pub(crate) survey_file: String,
    pub(crate) survey_sha256: String,
    pub(crate) unsaved_edits_included: bool,
    pub(crate) checklist_errors_remain: bool,
    pub(crate) checklist_summary: String,
    pub(crate) field_notes: Vec<String>,
    pub(crate) layer_lines: Vec<String>,
    pub(crate) reference_lines: Vec<String>,
    pub(crate) has_sheet_pdf: bool,
}

fn or_dash(s: &str) -> &str {
    if s.is_empty() { "-" } else { s }
}

/// `SubmitReadme::build` (SubmitReadme.cpp:113-142), every line in cpp order; `\n` line ends.
pub(crate) fn build_readme(r: &ReadmeInputs) -> String {
    let mut t = String::from("KA-HGIS submission package\n");
    t += &format!("created: {}\n", now_with_offset());
    t += &format!("timezone: {}\n", system_time_zone_id());
    t += &format!("app: {}\n", r.app_line);
    t += "crs: EPSG:5179\n";
    t += &format!("shp_encoding: {}\n", r.encoding);
    t += &format!("survey_name: {}\n", or_dash(&r.survey_name));
    t += &format!("survey_file: {}\n", or_dash(&r.survey_file));
    t += &format!("survey_sha256: {}\n", or_dash(&r.survey_sha256));
    t += if r.unsaved_edits_included {
        "saved_state: unsaved_edits_included (저장하지 않은 편집이 들어 있습니다. survey_sha256은 마지막으로 저장한 조사 파일의 값입니다)\n"
    } else {
        "saved_state: saved\n"
    };
    if r.checklist_errors_remain {
        t += "checklist_errors_remain: yes (검수 오류가 남았거나 검수하지 못한 채로 만들었습니다. 아래 checklist의 [error] 항목을 확인하세요)\n";
    }
    t += "intranet: upload each domain SHP (feature_poly.shp = one file; merge polygons in-app first if required)\n\n";
    t += &format!("checklist:\n{}\n", r.checklist_summary);
    if !r.field_notes.is_empty() {
        t += &format!("\nshp_field_names:\n{}\n", r.field_notes.join("\n"));
    }
    if !r.layer_lines.is_empty() {
        t += &format!("\nshp_layers:\n{}\n", r.layer_lines.join("\n"));
    }
    t += "\nreference_data (도면에 그린 참조 지도):\n";
    if r.reference_lines.is_empty() {
        t += "- 없음\n";
    } else {
        t += &format!("{}\n", r.reference_lines.join("\n"));
    }
    t += "\nSee MANIFEST.sha256 for file hashes.\n";
    t += "도면 PDF는 도면만들기 용지(user_sheet)를 넣습니다.\n";
    t += if r.has_sheet_pdf {
        "- 조사도면.pdf (도면만들기 user_sheet)\n"
    } else {
        "조사도면.pdf 없음: 도면만들기에서 용지를 만든 뒤 다시 보내기 하세요.\n"
    };
    t
}

/// SubmitReadme.cpp:28-31 — local time with its UTC offset in ISO form (truth: `2026-11-13T10:42:15+09:00`);
/// offset 0 is written `Z` (no truth file shows that case).
fn now_with_offset() -> String {
    let now = chrono::Local::now();
    if now.offset().local_minus_utc() == 0 {
        now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    } else {
        now.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
    }
}

/// SubmitReadme.cpp:117 — the system time zone as an IANA id (truth: `Asia/Seoul`).
pub(crate) fn system_time_zone_id() -> String {
    iana_time_zone::get_timezone().unwrap_or_default()
}

/// ExportService.cpp:35, :216 write MANIFEST and README in text mode: `\n` becomes `\r\n` on Windows
/// (the truth files are CRLF).
pub(crate) fn text_mode(s: &str) -> Vec<u8> {
    if cfg!(windows) { s.replace('\n', "\r\n").into_bytes() } else { s.as_bytes().to_vec() }
}

/// Paths in the cpp texts use native separators (ExportService.cpp:29, :102, :106).
pub(crate) fn native(path: &Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(windows) { s.replace('/', "\\") } else { s.into_owned() }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Streams a file through SHA-256 in 64 KiB chunks; `Err(true)` = open failed, `Err(false)` = read failed.
fn sha256_hex(path: &Path) -> Result<String, (bool, std::io::Error)> {
    let mut f = std::fs::File::open(path).map_err(|e| (true, e))?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).map_err(|e| (false, e))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `SubmitReadme::sha256OfFile` (SubmitReadme.cpp:92-104) with its texts.
pub(crate) fn sha256_of_file(path: &Path) -> Result<String, String> {
    sha256_hex(path).map_err(|(open, e)| {
        if open {
            format!("{}을(를) 열 수 없습니다: {e}", file_name(path))
        } else {
            format!("{}을(를) 끝까지 읽지 못했습니다.", file_name(path))
        }
    })
}

/// `ExportService::writeSha256Manifest` (ExportService.cpp:23-72): every file of `dir` incl. hidden,
/// sorted by name, MANIFEST.sha256 itself skipped, `<hex>  <name>` per line, text mode.
pub(crate) fn write_sha256_manifest(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!(
            "해시 목록을 만들 폴더가 없습니다: {}. 폴더가 지워졌거나 옮겨졌는지 확인하세요.",
            native(dir)
        ));
    }
    // Enumerate before creating MANIFEST.sha256 (cpp: before opening QSaveFile).
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("MANIFEST.sha256 저장 실패: {e}"))?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != "MANIFEST.sha256")
        .collect();
    names.sort();
    let mut lines = String::new();
    for name in &names {
        let hex = sha256_hex(&dir.join(name)).map_err(|(open, _)| {
            if open {
                format!("해시를 계산하려고 {name}을(를) 열지 못했습니다. 다른 프로그램이 이 파일을 쓰고 있는지 확인하세요.")
            } else {
                format!("해시를 계산하다 {name} 읽기에 실패했습니다. 저장 장치를 확인하세요.")
            }
        })?;
        lines += &format!("{hex}  {name}\n");
    }
    let mut man = std::fs::File::create(dir.join("MANIFEST.sha256"))
        .map_err(|e| format!("MANIFEST.sha256을 쓸 수 없습니다({e}). 폴더 쓰기 권한과 남은 공간을 확인하세요."))?;
    man.write_all(&text_mode(&lines))
        .and_then(|()| man.sync_all())
        .map_err(|e| format!("MANIFEST.sha256 저장 실패: {e}"))
}
