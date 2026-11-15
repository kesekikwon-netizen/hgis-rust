//! What curl is told for the VWorld site session (`CadastralPortal.cpp:31-105`). Nothing is sent here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

fn told(url: &str, form: Option<&str>, file: Option<&Path>) -> (Vec<String>, String) {
    let (command, config) = request(Path::new("jar.txt"), url, form, file);
    (command.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect(), config)
}

fn follows(args: &[String], option: &str) -> String {
    args.iter().position(|arg| arg == option).and_then(|at| args.get(at + 1)).cloned().unwrap_or_default()
}

/// The address and the login form reach curl on its standard input: neither is on the command line, where
/// any program of the PC could read the account from the list of processes.
#[test]
fn a_form_goes_to_curl_on_its_standard_input() {
    let (args, config) = told("https://www.vworld.kr/v4po_usrlogin_a004.do", Some("usrIdeE=YWJj&usrPwdE=c2VjcmV0%2B&nextUrl=x"), None);
    assert_eq!(config, "url = \"https://www.vworld.kr/v4po_usrlogin_a004.do\"\ndata = \"usrIdeE=YWJj&usrPwdE=c2VjcmV0%2B&nextUrl=x\"\n");
    assert!(args.iter().all(|arg| !arg.contains("usrPwdE") && !arg.contains("c2VjcmV0") && !arg.contains("vworld.kr/v4po")), "{args:?}");
    assert_eq!(args.last().map(String::as_str), Some("-"), "the config is read from the standard input");
    // A form is sent as the site's own page sends it (`:52-55`), in the session of the pages before it.
    assert!(args.contains(&"X-Requested-With: XMLHttpRequest".to_owned()));
    assert_eq!((follows(&args, "-b"), follows(&args, "-c")), ("jar.txt".to_owned(), "jar.txt".to_owned()));
    assert_eq!(follows(&args, "-A"), "Mozilla/5.0 ka-hgis");
    assert_eq!(follows(&args, "-e"), "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563");
}

/// A page or an answer of the API: 40 s without a byte and 40 s in all, at most 32 MiB (`:45-46, 75`); the
/// API is asked with the referer the key is registered for (`:49`); a page asks without the form's header.
#[test]
fn a_page_is_given_forty_seconds() {
    let (args, config) = told("https://api.vworld.kr/req/data?key=K", None, None);
    assert_eq!(config, "url = \"https://api.vworld.kr/req/data?key=K\"\n");
    assert_eq!((follows(&args, "--speed-time"), follows(&args, "--max-time"), follows(&args, "--max-filesize")), ("40".to_owned(), "40".to_owned(), "33554432".to_owned()));
    assert_eq!(follows(&args, "-e"), "https://localhost");
    assert!(!args.iter().any(|arg| arg.starts_with("X-Requested-With")) && !args.contains(&"-o".to_owned()));
}

/// A county archive: two minutes without a byte, half an hour in all, at most 512 MiB (`:45-46, 75`); it is
/// written beside its place and takes it only when it is whole.
#[test]
fn an_archive_is_given_half_an_hour() {
    let (args, _) = told("https://www.vworld.kr/dtmk/downloadResourceFile.do?ds_id=30563&fileNo=73", None, Some(Path::new("원본/73-1-2026-10-14-a.zip")));
    assert_eq!((follows(&args, "--speed-time"), follows(&args, "--max-time"), follows(&args, "--max-filesize")), ("120".to_owned(), "1800".to_owned(), "536870912".to_owned()));
    assert_eq!(follows(&args, "-o"), "원본/73-1-2026-10-14-a.zip.part");
}

/// The words of each failure are the C++ ones (`:67, 76, 80, 97, 100`); curl says which it was by its exit
/// code.
#[test]
fn the_words_of_a_failure() {
    assert_eq!(PortalFailure::Timeout.words(), "지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요.");
    assert_eq!(PortalFailure::TooBig.words(), "지적도 서버 응답이 허용 크기를 넘었습니다.");
    assert_eq!(PortalFailure::Disk.words(), "지적도 저장 공간을 확인하세요.");
    assert_eq!(PortalFailure::Network.words(), "VWorld 통신에 실패했습니다. 인터넷 연결과 계정 설정을 확인하세요.");
    assert_eq!(PortalFailure::NotZip.words(), "VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요.");
    assert_eq!([failure_of(Some(28)), failure_of(Some(63)), failure_of(Some(23)), failure_of(Some(22)), failure_of(None)], [PortalFailure::Timeout, PortalFailure::TooBig, PortalFailure::Disk, PortalFailure::Network, PortalFailure::Network]);
}

/// The answer of a page is its body when the site said 200, and a failure for any other status (`:93-98`).
#[test]
fn only_a_200_answer_has_a_body() {
    assert_eq!(body_of(b"{\"a\":1}\n__KA_HTTP_STATUS__:200".to_vec()), Ok(b"{\"a\":1}".to_vec()));
    assert_eq!(body_of(b"moved\n__KA_HTTP_STATUS__:204".to_vec()), Err(PortalFailure::Network));
    assert_eq!(body_of(b"no status".to_vec()), Err(PortalFailure::Network));
}

/// What was received is an archive only when it begins as one (`:99-101`).
#[test]
fn a_page_in_place_of_an_archive_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("a.zip.part");
    std::fs::write(&part, b"<html>login</html>").unwrap();
    assert_eq!(keep_archive(&part, &dir.path().join("a.zip")), Err(PortalFailure::NotZip));
    assert!(!part.exists() && !dir.path().join("a.zip").exists());
    std::fs::write(&part, b"PK\x03\x04rest").unwrap();
    assert_eq!(keep_archive(&part, &dir.path().join("a.zip")), Ok(()));
    assert_eq!(std::fs::read(dir.path().join("a.zip")).unwrap(), b"PK\x03\x04rest");
    assert!(!part.exists());
}
