//! Qt 6 ini 글 규칙 — 기댓값은 실제 Qt 6.11.1(OSGeo4W PyQt6 `QSettings`)이 쓴 바이트 그대로다.

use super::*;

fn esc(s: &str) -> String {
    escaped_string(s)
}

/// `iniEscapedString` — 쉼표·쌍반점·등호는 따옴표, 제어 글자는 `\t`·`\x1` 꼴, 뒤따르는 16진 글자도
/// `\x62` 로, 한글·NBSP·DEL 은 UTF-8 그대로, `@` 로 시작하면 `@@`, NUL 이 있으면 `@String(…)`.
#[test]
fn escaped_strings_match_qt6_bytes() {
    assert_eq!(esc("C:/only.gpkg\tonly\t1"), r"C:/only.gpkg\tonly\t1");
    assert_eq!(esc("say \"hi\" \\ back\t n\t2"), r#"say \"hi\" \\ back\t n\t2"#);
    assert_eq!(esc(" lead\ttrail \t3"), r#"" lead\ttrail \t3""#);
    assert_eq!(esc("a=b\tc\t4"), r#""a=b\tc\t4""#);
    assert_eq!(esc("a\u{1}b\u{7f}c\u{a0}d\t1"), "a\\x1\\x62\u{7f}c\u{a0}d\\t1");
    assert_eq!(esc("\u{1}a\u{0}1\t2"), r"@String(\x1\x61\0\x31\t2)");
    assert_eq!(esc("x;y"), r#""x;y""#);
    assert_eq!(esc("tail "), r#""tail ""#);
    assert_eq!(esc(""), "");
    assert_eq!(esc("\r\n\u{b}\u{c}\u{7}\u{8}"), r"\r\n\v\f\a\b");
    assert_eq!(esc("\u{1f}A"), r"\x1f\x41");
    assert_eq!(esc("@start"), "@@start");
    assert_eq!(esc("C:/Users/u/바탕 화면"), "C:/Users/u/바탕 화면");
}

/// `iniEscapedStringList` — 항목은 「, 」로 잇고, 빈 목록은 `@Invalid()`, 한 항목은 쉼표 없이.
#[test]
fn escaped_string_lists_match_qt6_bytes() {
    let two = ["C:/a b/경주시.gpkg\t경주시\t1794142119965".to_owned(), "D:/x,y/z.gpkg\tq;r\t5".to_owned()];
    assert_eq!(
        escaped_string_list(&two),
        r#"C:/a b/경주시.gpkg\t경주시\t1794142119965, "D:/x,y/z.gpkg\tq;r\t5""#
    );
    assert_eq!(escaped_string_list(&[]), "@Invalid()");
    assert_eq!(escaped_string_list(&["C:/only.gpkg\tonly\t1".to_owned()]), r"C:/only.gpkg\tonly\t1");
}

/// `iniUnescapedStringList` + `stringToVariant` 를 거친 `value().toStringList()`.
#[test]
fn string_lists_read_back_like_qt6() {
    assert_eq!(
        read_string_list(r#"C:/a b/경주시.gpkg\t경주시\t1794142119965, "D:/x,y/z.gpkg\tq;r\t5""#),
        vec!["C:/a b/경주시.gpkg\t경주시\t1794142119965".to_owned(), "D:/x,y/z.gpkg\tq;r\t5".to_owned()]
    );
    assert_eq!(read_string_list(r"C:/only.gpkg\tonly\t1"), vec!["C:/only.gpkg\tonly\t1".to_owned()]);
    assert!(read_string_list("@Invalid()").is_empty());
    assert_eq!(read_string_list(r#"" lead\ttrail \t3""#), vec![" lead\ttrail \t3".to_owned()]);
    assert_eq!(read_string_list(r#"say \"hi\" \\ back\t n\t2"#), vec!["say \"hi\" \\ back\t n\t2".to_owned()]);
    assert_eq!(read_string_list(r"@String(\x1\x61\0\x31\t2)"), vec!["\u{1}a\u{0}1\t2".to_owned()]);
    // Qt 5 가 쓰던 꼴(한글을 \x 로)도 읽는다. 따옴표 밖 끝 공백은 버린다.
    assert_eq!(read_string_list(r"\xac00\xb098  "), vec!["가나".to_owned()]);
    assert_eq!(read_string("@@start"), "@start");
    assert_eq!(read_string("C:/Users/u/바탕 화면"), "C:/Users/u/바탕 화면");
}

/// 실제 사용자 ini 와 같은 모양(CRLF, 앞뒤 다른 절, 같은 절의 다른 키)을 흉내 낸 글.
const SHAPED: &str = "[MainWindow]\r\nstate=@ByteArray(\\0\\0\\0\\xff)\r\n\r\n[RecentSurveys]\r\nfactsOrder=a, b\r\n\
facts\\d609dea299b3945e=1, 0\r\nitems=C:/x/a.gpkg\\ta\\t5\r\nskipAutoRestore=false\r\n\r\n[ui]\r\nlook=paper\r\n\r\n\
[Survey]\r\nLastDir=C:/x\r\n\r\n[network]\r\nproxy=none\r\n";

/// 바꾸는 한 줄 밖의 바이트는 그대로다(BOM·줄바꿈·다른 절·같은 절의 다른 키).
#[test]
fn set_value_rewrites_only_that_line() {
    let mut ini = IniFile::from_text(SHAPED.to_owned());
    assert_eq!(ini.value("RecentSurveys", "items"), Some(r"C:/x/a.gpkg\ta\t5"));
    assert_eq!(ini.value("Survey", "LastDir"), Some("C:/x"));
    assert_eq!(ini.value("Survey", "Missing"), None);
    ini.set_value("RecentSurveys", "items", r"C:/y/b.gpkg\tb\t9, C:/x/a.gpkg\ta\t5");
    ini.set_value("Survey", "LastDir", "C:/y");
    let want = SHAPED
        .replace("items=C:/x/a.gpkg\\ta\\t5", "items=C:/y/b.gpkg\\tb\\t9, C:/x/a.gpkg\\ta\\t5")
        .replace("LastDir=C:/x\r\n", "LastDir=C:/y\r\n");
    assert_eq!(ini.text(), want);
}

/// 없는 키는 그 절 끝(빈 줄 앞)에, 없는 절은 Qt 처럼 빈 줄 하나 띄우고 파일 끝에 붙인다.
#[test]
fn missing_keys_and_sections_are_added_like_qt() {
    let mut ini = IniFile::from_text("[Survey]\r\nOther=1\r\n\r\n[ui]\r\nlook=paper\r\n".to_owned());
    ini.set_value("Survey", "LastDir", "C:/y");
    ini.set_value("RecentSurveys", "items", "@Invalid()");
    assert_eq!(
        ini.text(),
        "[Survey]\r\nOther=1\r\nLastDir=C:/y\r\n\r\n[ui]\r\nlook=paper\r\n\r\n[RecentSurveys]\r\nitems=@Invalid()\r\n"
    );
    // 줄바꿈이 LF 인 파일은 LF 로, 빈 파일은 Windows Qt 처럼 CRLF 로 쓴다.
    let mut lf = IniFile::from_text("[ui]\nlook=paper".to_owned());
    lf.set_value("ui", "size", "2");
    assert_eq!(lf.text(), "[ui]\nlook=paper\nsize=2\n");
    let mut empty = IniFile::from_text(String::new());
    empty.set_value("Survey", "LastDir", "C:/y");
    assert_eq!(empty.text(), "[Survey]\r\nLastDir=C:/y\r\n");
}

/// 파일이 없으면 빈 글로 읽고, 쓰면 폴더까지 만든다. UTF-8 이 아니면 읽기를 거절한다.
#[test]
fn files_round_trip_and_odd_bytes_are_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ka-hgis").join("ka-hgis.ini");
    let mut ini = IniFile::read(&path).expect("missing file reads empty");
    assert_eq!(ini.text(), "");
    ini.set_value("Survey", "LastDir", "C:/y");
    ini.write(&path).expect("write");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "[Survey]\r\nLastDir=C:/y\r\n");
    std::fs::write(&path, [0xff, 0xfe, b'[']).expect("write odd");
    assert!(IniFile::read(&path).is_err());
}
