//! Windows DPAPI — 가짜 비밀번호만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::dpapi::{protect_bytes, unprotect_bytes};
use std::process::Command;

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// C++ protectBytes 꼴: DPAPI 머리, 설명 "ka-hgis"(UTF-16 7자 + 끝 0 = 16바이트), 엔트로피로 다시 풀린다.
#[test]
fn protect_writes_the_cpp_blob_form() {
    let fake = "FAKE-pw'5678\"";
    let blob = protect_bytes(fake.as_bytes()).unwrap();
    let raw = BASE64.decode(&blob).unwrap();
    assert!(raw.starts_with(WIN_DPAPI_HEADER));
    assert_eq!(&raw[44..48], &[16, 0, 0, 0], "description length");
    assert_eq!(&raw[48..62], utf16("ka-hgis").as_slice());
    assert_eq!(unprotect_bytes(&blob).unwrap(), fake.as_bytes());
}

/// SEC1 — C++ 앱이 쓴 진짜 Windows DPAPI 꾸러미(가짜 비밀번호, 설명 없음)를 풀고, PowerShell 명령줄에는 그 꾸러미가 없다.
#[test]
fn windows_dpapi_blob_unprotects_without_the_blob_on_the_command_line() {
    let protect = "Add-Type -AssemblyName System.Security; \
         $e = [System.Text.Encoding]::ASCII.GetBytes('ka-hgis-account-v1'); \
         $p = [System.Text.Encoding]::UTF8.GetBytes('FAKE-PW-1234'); \
         [Convert]::ToBase64String([System.Security.Cryptography.ProtectedData]::Protect($p, $e, 'CurrentUser'))";
    let out = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", protect]).output().expect("powershell");
    let blob = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    assert!(BASE64.decode(&blob).expect("b64").starts_with(WIN_DPAPI_HEADER));
    assert_eq!(unprotect_bytes(&blob).unwrap(), b"FAKE-PW-1234");
    let args: Vec<String> = dpapi_powershell().get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_eq!(args, ["-NoProfile", "-NonInteractive", "-Command", "-"]);
    assert!(unprotect_script(std::slice::from_ref(&blob)).contains(&blob), "the blob goes on stdin");
}

/// 여러 꾸러미를 한 번에 미리 풀면 그 뒤 읽기가 같은 값을 낸다.
#[test]
fn prefetch_then_read() {
    let blobs: Vec<String> = protect_verified_many(&[b"FAKE-a", b"FAKE-b"]).into_iter().map(Option::unwrap).collect();
    prefetch(&blobs);
    assert_eq!(unprotect_bytes(&blobs[0]).unwrap(), b"FAKE-a");
    assert_eq!(unprotect_bytes(&blobs[1]).unwrap(), b"FAKE-b");
}
