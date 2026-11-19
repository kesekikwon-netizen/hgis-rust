//! Four POINT entities come back as four survey points. A file that is not a DXF does not.

#![allow(clippy::unwrap_used)]

use super::read_dxf_survey_points;

const DXF: &str = "\
0\nSECTION\n2\nENTITIES\n\
0\nPOINT\n8\nA\n10\n0.0\n20\n0.0\n30\n0.0\n\
0\nPOINT\n8\nA\n10\n100.0\n20\n0.0\n30\n0.0\n\
0\nPOINT\n8\nA\n10\n0.0\n20\n100.0\n30\n10.0\n\
0\nPOINT\n8\nA\n10\n100.0\n20\n100.0\n30\n10.0\n\
0\nENDSEC\n0\nEOF\n";

#[test]
fn four_dxf_points_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("slope.dxf");
    std::fs::write(&path, DXF).unwrap();
    let mut z: Vec<i32> = read_dxf_survey_points(&path)
        .unwrap()
        .into_iter()
        .map(|p| (p[2] * 100.0).round() as i32)
        .collect();
    z.sort();
    assert_eq!(z, vec![0, 0, 1000, 1000]);
}

#[test]
fn a_text_file_named_dxf_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.dxf");
    std::fs::write(&path, b"not a drawing").unwrap();
    assert!(read_dxf_survey_points(&path).is_err());
}
