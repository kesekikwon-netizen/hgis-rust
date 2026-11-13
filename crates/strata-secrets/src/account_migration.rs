//! SEC2 — 예전 Rust 판(96712e6, 2026-11-07 ~ SEC2)은 계정 비밀번호를 `password_dpapi` 에 C++ 이 못 읽는 꼴
//! (Base64("DPAPI1:" + XOR))로 썼다. 앱을 보통으로 켤 때 한 번, 그 값만 진짜 DPAPI(C++ `KaSecretStore::writePassword`
//! 꼴, KaSecretStore.cpp:138-164)로 바꾼다: 그 줄의 값 바이트만 바꾸고 BOM·줄 끝·따옴표·다른 줄은 그대로, 같은 폴더의
//! 임시 파일에 쓴 뒤 이름 바꾸기. 암호화·확인·쓰기 중 하나라도 못 하면 그 파일은 손대지 않는다(「설정 필요」로 보인다).
//! 값은 어디에도 남기지 않는다(옛 값의 사본도 만들지 않는다).

use std::ops::Range;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::accounts::AccountKind;
use crate::dpapi::{protect_verified_many, wipe, xor_entropy};

#[cfg(test)]
use crate::accounts::AccountSettings;

const LEGACY_PREFIX: &[u8] = b"DPAPI1:";
const KEY: &str = "password_dpapi";

/// 바꾼 값·못 바꾼 값의 수(값은 담지 않는다).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationReport {
    pub migrated: usize,
    pub failed: usize,
}

type Protect<'a> = &'a dyn Fn(&[&[u8]]) -> Vec<Option<String>>;
type Hit = (Range<usize>, Vec<u8>);

/// 이 사용자의 계정 파일들(`config_dir`)에 남은 예전 Rust 꼴 비밀번호를 진짜 DPAPI 로 바꾼다.
#[must_use]
pub fn migrate_legacy_rust_passwords(config_dir: &Path) -> MigrationReport {
    migrate_with(config_dir, &protect_verified_many)
}

/// 계정 파일 이름(지금 이름과 예전 이름).
fn account_files() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = [AccountKind::VworldCadastral, AccountKind::NgiiTopographic, AccountKind::HeritageIntranet]
        .iter()
        .flat_map(|k| [k.file_name(), k.alt_file_name()])
        .collect();
    names.dedup();
    names
}

pub(crate) fn migrate_with(config_dir: &Path, protect: Protect<'_>) -> MigrationReport {
    let mut report = MigrationReport::default();
    let files: Vec<(PathBuf, Vec<u8>, Vec<Hit>)> = account_files()
        .into_iter()
        .filter_map(|name| {
            let path = config_dir.join(name);
            let mut content = std::fs::read(&path).ok()?;
            let hits = legacy_spans(&content);
            if hits.is_empty() {
                wipe(&mut content);
                return None;
            }
            Some((path, content, hits))
        })
        .collect();
    if files.is_empty() {
        return report;
    }
    let blobs = {
        let plains: Vec<&[u8]> = files.iter().flat_map(|f| f.2.iter().map(|h| h.1.as_slice())).collect();
        protect(&plains)
    };
    let mut blobs = blobs.into_iter();
    for (path, content, hits) in &files {
        let mine: Vec<Option<String>> = hits.iter().map(|_| blobs.next().flatten()).collect();
        if mine.iter().any(Option::is_none) {
            report.failed += hits.len();
            continue;
        }
        let mut out = Vec::with_capacity(content.len() + 512);
        let mut at = 0;
        for ((span, _), blob) in hits.iter().zip(mine.iter().flatten()) {
            out.extend_from_slice(&content[at..span.start]);
            out.extend_from_slice(blob.as_bytes());
            at = span.end;
        }
        out.extend_from_slice(&content[at..]);
        if replace_file(path, &out).is_ok() {
            report.migrated += hits.len();
        } else {
            report.failed += hits.len();
        }
        wipe(&mut out);
    }
    for (_, mut content, hits) in files {
        wipe(&mut content);
        for (_, mut plain) in hits {
            wipe(&mut plain);
        }
    }
    report
}

/// 파일 안 `password_dpapi=` 값 가운데 예전 Rust 꼴인 것의 바이트 자리(앞뒤 빈칸·따옴표 안)와 평문.
fn legacy_spans(content: &[u8]) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut start = 0;
    while start < content.len() {
        let end = content[start..].iter().position(|&b| b == b'\n').map_or(content.len(), |i| start + i);
        if let Some(hit) = value_span(content, start, end) {
            hits.push(hit);
        }
        start = end + 1;
    }
    hits
}

fn value_span(content: &[u8], line_start: usize, line_end: usize) -> Option<Hit> {
    let line = &content[line_start..line_end];
    let eq = line.iter().position(|&b| b == b'=')?;
    let key = std::str::from_utf8(&line[..eq]).ok()?.trim().trim_start_matches('\u{feff}');
    if key != KEY && !key.ends_with("/password_dpapi") {
        return None;
    }
    let (mut a, mut b) = (line_start + eq + 1, line_end);
    while a < b && content[a].is_ascii_whitespace() {
        a += 1;
    }
    while b > a && content[b - 1].is_ascii_whitespace() {
        b -= 1;
    }
    if b - a >= 2 && content[a] == b'"' && content[b - 1] == b'"' {
        a += 1;
        b -= 1;
    }
    let plain = legacy_plain(std::str::from_utf8(&content[a..b]).ok()?)?;
    Some((a..b, plain))
}

/// 예전 Rust 꼴 값의 평문 — 그 꼴이 아니거나, 비었거나, UTF-8 이 아니면 None.
fn legacy_plain(value: &str) -> Option<Vec<u8>> {
    let mut decoded = BASE64.decode(value).ok()?;
    let plain = decoded.strip_prefix(LEGACY_PREFIX).map(|p| xor_entropy(p.to_vec()));
    wipe(&mut decoded);
    let mut plain = plain?;
    if plain.is_empty() || std::str::from_utf8(&plain).is_err() {
        wipe(&mut plain);
        return None;
    }
    Some(plain)
}

/// 같은 폴더의 임시 파일에 쓰고 이름을 바꾼다(못 하면 임시 파일을 지우고 원래 파일은 그대로).
fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.sec2.tmp"));
    let result = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
#[path = "account_migration_tests.rs"]
mod tests;
