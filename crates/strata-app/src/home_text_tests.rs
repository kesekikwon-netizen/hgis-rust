//! 홈 화면 글 — C++ `KaHomeText.cpp` 의 folder·folderTail·folderEnd·when 과 같은지.

use chrono::NaiveDate;

use super::*;

const DESK: Option<&str> = Some("C:/Users/u/Desktop");
const DOCS: Option<&str> = Some("C:/Users/u/Documents");

fn f(path: &str) -> String {
    folder_in(path, DESK, DOCS)
}

/// 바탕 화면·문서 안이면 그 이름부터, 아니면 경로 마디를 「 › 」로 잇는다(대소문자 무시).
#[test]
fn folder_starts_at_desktop_or_documents() {
    assert_eq!(f("C:/Users/u/Documents/경주시/경주시.gpkg"), "문서 › 경주시");
    assert_eq!(f("C:/Users/u/Desktop/andong/andong.gpkg"), "바탕 화면 › andong");
    assert_eq!(f("C:/Users/u/Desktop/a.gpkg"), "바탕 화면");
    assert_eq!(f("c:/users/U/desktop/x/y/a.gpkg"), "바탕 화면 › x › y");
    assert_eq!(
        f("C:/Users/u/AppData/Local/Temp/ka-hgis-cojkmC/ka-hgis-survey-verify/verify1.gpkg"),
        "C: › Users › u › AppData › Local › Temp › ka-hgis-cojkmC › ka-hgis-survey-verify"
    );
    assert_eq!(f("C:/Users/u/Desktopper/a.gpkg"), "C: › Users › u › Desktopper");
    assert_eq!(folder_in("D:/현장/a.gpkg", None, None), "D: › 현장");
}

/// 목록의 폴더 줄은 마지막 마디만(`folderEnd`), 이어서 작업 표는 끝 두 마디와 조사 이름(`folderTail`).
#[test]
fn folder_end_and_tail_shorten_like_cpp() {
    assert_eq!(folder_end("문서 › 경주시"), "문서 › 경주시");
    assert_eq!(folder_end("C: › Users › u › Temp › ka-hgis-survey-verify"), "… › ka-hgis-survey-verify");
    assert_eq!(folder_tail("문서 › 경주시", "경주시"), "문서 › 경주시");
    assert_eq!(folder_tail("바탕 화면", "andong"), "바탕 화면 › andong");
    assert_eq!(folder_tail("문서 › 2026 › 현장", "현장A"), "… › 현장 › 현장A");
    assert_eq!(folder_tail("", ""), "");
}

/// 「마지막 열림」 — 오늘·어제·올해는 「3월 5일」, 그 전은 연-월-일, 시각은 24시간.
#[test]
fn when_reads_like_cpp() {
    let today = NaiveDate::from_ymd_opt(2026, 11, 13).expect("date");
    let at = |y, m, d, h, min| NaiveDate::from_ymd_opt(y, m, d).and_then(|x| x.and_hms_opt(h, min, 0)).expect("time");
    assert_eq!(when_at(at(2026, 11, 13, 9, 5), today), "오늘 09:05");
    assert_eq!(when_at(at(2026, 11, 12, 22, 26), today), "어제 22:26");
    assert_eq!(when_at(at(2026, 3, 5, 9, 7), today), "3월 5일 09:07");
    assert_eq!(when_at(at(2025, 12, 31, 23, 59), today), "2025-12-31 23:59");
    assert_eq!(when(0), "");
    assert_eq!(when(-5), "");
}
