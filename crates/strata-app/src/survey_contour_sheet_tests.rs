//! Counts and the preview sentence for a coordinate table.
//! `target/cpp-ref/src/core/SurveyPointSchema.cpp`, `SurveyPointReader.cpp:234-250`,
//! `target/cpp-ref/src/app/KaSurveyContourDialog.cpp:211-232`.

#![allow(clippy::unwrap_used)]

use super::parse::parse_number;
use super::{contour_preview, format_cm};

fn preview(csv: &str, interval: f64, exclude: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("points.csv");
    std::fs::write(&path, csv).unwrap();
    contour_preview(&path.display().to_string(), interval, exclude)
}

#[test]
fn four_rows_report_count_and_rounded_range() {
    let text = preview(
        "점번호,X,Y,표고\n1,200000,500000,10\n2,200010,500000,12.5\n3,200000,500010,11\n4,200010,500010,15.25\n",
        0.5,
        true,
    );
    assert_eq!(text, "점 4개, 표고 10.0–15.3 m. 등고선 약 10줄.");
    assert!(!text.contains("248"));
    assert!(!text.contains("24.5"));
}

#[test]
fn wider_interval_lowers_the_line_estimate() {
    let text = preview(
        "점번호,X,Y,표고\n1,1,2,10\n2,3,4,15.25\n3,5,6,11\n",
        1.0,
        false,
    );
    assert_eq!(text, "점 3개, 표고 10.0–15.3 m. 등고선 약 5줄.");
}

#[test]
fn duplicate_millimetre_cell_is_one_averaged_point() {
    let text = preview("X,Y,Z\n10,20,10\n10,20,12\n30,40,11\n", 0.5, false);
    assert_eq!(text, "점 2개, 표고 11.0–11.0 m. 등고선 약 0줄.");
}

#[test]
fn excluding_suspicious_drops_zero_and_999() {
    let csv = "X,Y,표고\n1,2,0\n3,4,10\n5,6,999\n7,8,12.5\n";
    assert_eq!(
        preview(csv, 0.5, true),
        "점 2개, 표고 10.0–12.5 m. 등고선 약 5줄."
    );
    assert_eq!(
        preview(csv, 0.5, false),
        "점 4개, 표고 0.0–999.0 m. 등고선 약 1998줄.\n줄이 많습니다. 간격을 넓히세요."
    );
}

#[test]
fn headerless_numeric_first_row_is_kept() {
    let text = preview(
        "1,200000,500000,10.5\n2,200010,500000,11\n3,200000,500010,12\n",
        0.5,
        false,
    );
    assert_eq!(text, "점 3개, 표고 10.5–12.0 m. 등고선 약 3줄.");
}

#[test]
fn comma_rules_match_the_schema() {
    assert_eq!(parse_number("10,5"), Some(10.5));
    assert_eq!(parse_number("1,234.5"), Some(1234.5));
    assert_eq!(parse_number("1,234"), Some(1.234));
    assert_eq!(parse_number("1,234,567"), Some(1234567.0));
    assert_eq!(parse_number(""), None);
    assert_eq!(format_cm(1525), "15.3");
    assert_eq!(format_cm(-1525), "-15.3");
    assert_eq!(format_cm(1000), "10.0");
}

#[test]
fn bad_rows_are_not_counted() {
    let text = preview(
        "X,Y,Z\n1,2,10\n,,,\n3,abc,4\n5,6,nope\n7,8,11\n",
        1.0,
        false,
    );
    assert_eq!(text, "점 2개, 표고 10.0–11.0 m. 등고선 약 1줄.");
}

#[test]
fn missing_file_and_missing_columns_use_the_cpp_lines() {
    let missing = contour_preview("no-such-contour-file.csv", 0.5, true);
    assert_eq!(
        missing,
        "파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요."
    );
    let text = preview("이름,값\n가,1\n", 0.5, false);
    assert_eq!(text, "X·Y·표고 칸을 찾지 못했습니다. 칸을 직접 지정하세요.");
}

#[test]
fn excel_and_dxf_are_not_pretended() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["book.xlsx", "book.xls"] {
        let path = dir.path().join(name);
        std::fs::write(&path, b"not a table").unwrap();
        let text = contour_preview(&path.display().to_string(), 0.5, true);
        assert_eq!(text, "엑셀은 아직 읽지 않습니다 (지시 W-29).");
        assert!(!text.contains("248"));
    }
    let dxf = dir.path().join("points.dxf");
    std::fs::write(&dxf, b"not a drawing").unwrap();
    let text = contour_preview(&dxf.display().to_string(), 0.5, true);
    assert_eq!(
        text,
        "파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요."
    );
    assert!(!text.contains("248"));
}
