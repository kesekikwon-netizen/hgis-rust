//! 최근 조사 목록이 C++ `RecentSurveys`(`RecentSurveys.cpp`)처럼 읽고 쓰는지 — 임시 폴더의 ini 만 쓴다.

use std::path::{Path, PathBuf};

use super::*;

/// 사용자 ini 와 같은 모양: 앞뒤 절, 같은 절의 facts 키, items 뒤에도 키가 있다.
fn shaped(items_line: &str) -> String {
    format!(
        "[MainWindow]\r\nstate=@ByteArray(\\0\\x1)\r\n\r\n[RecentSurveys]\r\nfactsOrder=a\r\n\
         facts\\0b2aeb2b959bb5e9=1, 0\r\n{items_line}\r\nskipAutoRestore=false\r\n\r\n[Survey]\r\n\
         LastDir=C:/old\r\n\r\n[network]\r\nproxy=none\r\n"
    )
}

fn touch(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"x").expect("touch");
    p
}

fn fwd(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// `remember` 은 items 한 줄만 바꾼다 — 새것이 맨 앞, 이름·경로·시각은 탭으로, 다른 바이트는 그대로.
#[test]
fn remember_rewrites_only_the_items_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let old = r"items=C:/gone/경주시.gpkg\t경주시\t1794142119965, D:/usb/andong.gpkg\tandong\t1791630752776";
    std::fs::write(&ini, shaped(old)).expect("fixture");
    let survey = touch(dir.path(), "새 조사.gpkg");

    remember(&ini, &survey, "").expect("remember");

    let after = std::fs::read_to_string(&ini).expect("read");
    let items = load_stored(&ini);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].path, fwd(&std::path::absolute(&survey).expect("abs")));
    assert_eq!(items[0].name, "새 조사", "empty name -> completeBaseName");
    assert!(items[0].last_opened_ms > 1_794_142_119_965);
    assert_eq!((items[1].name.as_str(), items[1].last_opened_ms), ("경주시", 1_794_142_119_965));
    assert_eq!(items[2].path, "D:/usb/andong.gpkg", "offline entries stay");
    let new_line = format!(
        r"items={}\t새 조사\t{}, C:/gone/경주시.gpkg\t경주시\t1794142119965, D:/usb/andong.gpkg\tandong\t1791630752776",
        items[0].path, items[0].last_opened_ms
    );
    assert_eq!(after, shaped(&new_line));
}

/// 없는 파일은 기억하지 않는다(파일을 건드리지 않음).
#[test]
fn remember_ignores_missing_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    std::fs::write(&ini, shaped(r"items=C:/x/a.gpkg\ta\t5")).expect("fixture");
    remember(&ini, &dir.path().join("없음.gpkg"), "없음").expect("no-op");
    assert_eq!(std::fs::read_to_string(&ini).expect("read"), shaped(r"items=C:/x/a.gpkg\ta\t5"));
}

/// 열두 개까지(`RecentSurveys.h:21` kMaxItems) — 열셋째를 기억하면 가장 오래된 것이 빠진다.
#[test]
fn the_thirteenth_survey_drops_the_oldest() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    for i in 0..13 {
        remember(&ini, &touch(dir.path(), &format!("s{i:02}.gpkg")), "").expect("remember");
    }
    let names: Vec<String> = load_stored(&ini).into_iter().map(|it| it.name).collect();
    assert_eq!(names.len(), MAX_ITEMS);
    assert_eq!(names.first().map(String::as_str), Some("s12"));
    assert_eq!(names.last().map(String::as_str), Some("s01"));
}

/// Windows 는 대소문자·구분자만 다른 경로를 같은 조사로 본다(samePath) — 한 줄로 합치고 맨 앞으로.
#[test]
fn case_and_separator_variants_are_one_survey() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let a = touch(dir.path(), "Site.gpkg");
    let b = touch(dir.path(), "other.gpkg");
    remember(&ini, &a, "").expect("a");
    remember(&ini, &b, "").expect("b");
    let shouted = PathBuf::from(a.to_string_lossy().to_uppercase().replace('/', "\\"));
    remember(&ini, &shouted, "현장").expect("a again");
    let items = load_stored(&ini);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "현장");
    assert!(same_path(&items[0].path, &a.to_string_lossy()));
    assert!(same_path("c:\\Temp\\A.gpkg", "C:/temp/a.gpkg"));
    assert!(!same_path("", ""));
}

/// 저장된 줄 읽기(`loadStored`): 탭·쉼표·따옴표·한글, 시각이 없거나 숫자가 아닌 줄, 겹친 줄.
#[test]
fn stored_lines_parse_like_load_stored() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let line = r#"items="D:/x,y/z.gpkg\tq;r\t5", C:/a/b.gpkg\t이름 \t 둘\tnot-a-number, C:/A/B.GPKG\tdup\t7, C:/only, \tnopath\t1"#;
    std::fs::write(&ini, shaped(line)).expect("fixture");
    let items = load_stored(&ini);
    assert_eq!(items.len(), 2);
    assert_eq!((items[0].path.as_str(), items[0].name.as_str(), items[0].last_opened_ms), ("D:/x,y/z.gpkg", "q;r", 5));
    assert_eq!((items[1].name.as_str(), items[1].last_opened_ms), ("이름 둘 not-a-number", 0));
}

/// 홈 목록(`loadAll`)은 없는 조사도 「찾을 수 없음」으로 남기고, `load`·`lastPath` 는 열 수 있는 것만.
#[test]
fn load_all_keeps_missing_and_last_path_skips_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let here = touch(dir.path(), "here.gpkg");
    let line = format!(r"items=C:/nowhere-strata/gone.gpkg\tgone\t9, {}\t\t8", fwd(&here));
    std::fs::write(&ini, shaped(&line)).expect("fixture");
    let all = load_all(&ini);
    assert_eq!(all.len(), 2);
    assert!(!all[0].available);
    assert!(all[1].available);
    assert_eq!(all[1].name, "here", "empty stored name -> completeBaseName");
    assert_eq!(last_path(&ini), fwd(&here));
    assert!(load_all(&dir.path().join("none.ini")).is_empty());
}

/// `forget` 은 그 조사만 뺀다. 빈 목록은 Qt 처럼 `@Invalid()`.
#[test]
fn forget_removes_one_and_empty_lists_store_invalid() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    std::fs::write(&ini, shaped(r"items=C:/x/a.gpkg\ta\t5, C:/x/b.gpkg\tb\t4")).expect("fixture");
    forget(&ini, "c:\\x\\A.gpkg").expect("forget a");
    assert_eq!(std::fs::read_to_string(&ini).expect("read"), shaped(r"items=C:/x/b.gpkg\tb\t4"));
    forget(&ini, "C:/x/b.gpkg").expect("forget b");
    assert_eq!(std::fs::read_to_string(&ini).expect("read"), shaped("items=@Invalid()"));
}

/// `Survey/LastDir` — 조사 파일의 폴더(앞으로 빗금), 그 줄만 바꾼다.
#[test]
fn last_dir_reads_and_writes_one_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    std::fs::write(&ini, shaped(r"items=C:/x/a.gpkg\ta\t5")).expect("fixture");
    assert_eq!(last_dir(&ini), "C:/old");
    set_last_dir(&ini, "C:/Users/u/바탕 화면/현장").expect("set");
    let want = shaped(r"items=C:/x/a.gpkg\ta\t5").replace("LastDir=C:/old", "LastDir=C:/Users/u/바탕 화면/현장");
    assert_eq!(std::fs::read_to_string(&ini).expect("read"), want);
    assert_eq!(last_dir(&dir.path().join("none.ini")), "");
}

/// `normalizedPath`·`QFileInfo::absoluteFilePath` — 앞으로 빗금, `.`·`..` 정리, 드라이브 글자 대문자.
#[test]
fn paths_normalize_like_qfileinfo() {
    assert_eq!(absolute_file_path(Path::new(r"c:\a\.\b\..\c.gpkg")), "C:/a/c.gpkg");
    assert_eq!(normalized_path("  C:\\x\\y  "), "C:/x/y");
    assert_eq!(normalized_path("   "), "");
    assert_eq!(complete_base_name("C:/a/site.v2.gpkg"), "site.v2");
    assert_eq!(complete_base_name("C:/a/noext"), "noext");
}

/// 읽지 못하는 ini(잠김·UTF-8 아님)에는 기억·빼기가 아무 것도 쓰지 않는다 — 사용자 목록을 지우지 않는다.
#[test]
fn unreadable_ini_is_never_overwritten() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let mut odd = shaped(r"items=C:/x/a.gpkg\ta\t5").into_bytes();
    odd.push(0xff);
    std::fs::write(&ini, &odd).expect("fixture");
    let survey = touch(dir.path(), "new.gpkg");
    assert!(remember(&ini, &survey, "").is_err());
    assert!(forget(&ini, "C:/x/a.gpkg").is_err());
    assert!(set_last_dir(&ini, "C:/y").is_err());
    assert_eq!(std::fs::read(&ini).expect("read"), odd);
}

/// `topographic/folder` (MainWindowTopographic.cpp:105) — an absent key gives `None` so the caller uses Documents.
#[test]
fn topographic_folder_reads_the_cpp_key() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    std::fs::write(&ini, "[topographic]\nfolder=C:/Users/u/지형 자료\n").expect("fixture");
    assert_eq!(topographic_folder(&ini).as_deref(), Some("C:/Users/u/지형 자료"));
    std::fs::write(&ini, "[Survey]\nLastDir=C:/old\n").expect("fixture");
    assert_eq!(topographic_folder(&ini), None);
    assert_eq!(topographic_folder(&dir.path().join("none.ini")), None);
}
