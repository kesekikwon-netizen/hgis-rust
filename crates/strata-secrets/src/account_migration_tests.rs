//! SEC2 옮기기 — 임시 폴더·가짜 비밀번호만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::dpapi::xor_entropy;

/// 예전 Rust 판이 쓰던 꼴: Base64("DPAPI1:" + XOR).
fn legacy(plain: &[u8]) -> String {
    let mut v = b"DPAPI1:".to_vec();
    v.extend(xor_entropy(plain.to_vec()));
    BASE64.encode(v)
}

fn sample(value: &str) -> Vec<u8> {
    format!("\u{feff}; 손대지 않는 줄\r\n[ngii]\r\npassword_dpapi={value}\r\nusername=fake_user\r\n\r\n[other]\r\nkeep = 1\n").into_bytes()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

/// 진짜 DPAPI 로 바꾸고, 값 바이트 말고는(BOM·CRLF·주석·다른 칸) 그대로 — Rust 읽기로 같은 비밀번호, 두 번째는 할 일 없음.
#[cfg(windows)]
#[test]
fn legacy_rust_password_becomes_real_dpapi_and_the_rest_stays() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ngii-account.ini");
    let fake = "FAKE-pw'7\"x";
    let old_value = legacy(fake.as_bytes());
    let before = sample(&old_value);
    std::fs::write(&path, &before).unwrap();
    assert_eq!(migrate_legacy_rust_passwords(dir.path()), MigrationReport { migrated: 1, failed: 0 });
    let after = std::fs::read(&path).unwrap();
    let at = before.windows(old_value.len()).position(|w| w == old_value.as_bytes()).unwrap();
    let tail = &before[at + old_value.len()..];
    assert_eq!(&after[..at], &before[..at]);
    assert!(after.ends_with(tail));
    let new_value = std::str::from_utf8(&after[at..after.len() - tail.len()]).unwrap();
    assert!(BASE64.decode(new_value).unwrap().starts_with(crate::dpapi::WIN_DPAPI_HEADER));
    let creds = AccountSettings::read_ini(&path, AccountKind::NgiiTopographic).unwrap();
    assert_eq!((creds.username.as_str(), creds.password.as_str()), ("fake_user", fake));
    assert_eq!(names(dir.path()), ["ngii-account.ini"], "no temp or backup file left");
    assert_eq!(migrate_legacy_rust_passwords(dir.path()), MigrationReport::default());
    assert_eq!(std::fs::read(&path).unwrap(), after);
}

/// 암호화가 안 되면 파일은 한 바이트도 바뀌지 않는다.
#[test]
fn migration_failure_leaves_the_file_byte_equal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("heritage-account.ini");
    let before = sample(&legacy(b"FAKE-pw"));
    std::fs::write(&path, &before).unwrap();
    let report = migrate_with(dir.path(), &|p: &[&[u8]]| vec![None; p.len()]);
    assert_eq!(report, MigrationReport { migrated: 0, failed: 1 });
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(names(dir.path()), ["heritage-account.ini"]);
}

/// 예전 Rust 꼴만 — 포터블·평문·진짜 DPAPI·빈 값·UTF-8 이 아닌 값은 그대로 둔다.
#[test]
fn only_the_old_rust_form_is_rewritten() {
    let dir = tempfile::tempdir().unwrap();
    let real_like = BASE64.encode([crate::dpapi::WIN_DPAPI_HEADER, b"rest".as_slice()].concat());
    let files = [
        ("vworld-account.ini", format!("[vworld_account]\r\npassword_portable={}\r\nusername=u\r\n", BASE64.encode(xor_entropy(b"FAKE".to_vec())))),
        ("ngii-account.ini", "[ngii]\npassword=FAKE\nusername=u\n".to_owned()),
        ("heritage-account.ini", format!("[heritage]\npassword_dpapi={real_like}\nusername=u\n")),
        ("cadastral-account.ini", format!("[cadastral]\npassword_dpapi={}\n[x]\npassword_dpapi={}\n", BASE64.encode(b"DPAPI1:"), legacy(&[0xff, 0xfe]))),
    ];
    for (name, text) in &files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let report = migrate_with(dir.path(), &|_: &[&[u8]]| panic!("nothing should be protected"));
    assert_eq!(report, MigrationReport::default());
    for (name, text) in &files {
        assert_eq!(std::fs::read_to_string(dir.path().join(name)).unwrap(), *text, "{name}");
    }
}

/// 따옴표로 싼 값은 따옴표를 남기고 안의 값만 바꾼다.
#[test]
fn quoted_value_keeps_its_quotes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vworld-account.ini");
    let old_value = legacy(b"FAKE-pw");
    std::fs::write(&path, sample(&format!("\"{old_value}\""))).unwrap();
    let report = migrate_with(dir.path(), &|p: &[&[u8]]| vec![Some("AQAAANCMnd8BFAKE".to_owned()); p.len()]);
    assert_eq!(report, MigrationReport { migrated: 1, failed: 0 });
    assert_eq!(std::fs::read(&path).unwrap(), sample("\"AQAAANCMnd8BFAKE\""));
}
