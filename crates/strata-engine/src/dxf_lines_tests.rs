//! A small drawing written here, read back through GDAL's DXF driver: lines keep their CAD layer, their
//! points and their height; what is no line is counted, not handed on.

use super::*;

/// A drawing in the form of a received sheet (AC1015, code page 949): a contour of three points at 120 m,
/// a closed contour of four at 95.5 m, a plain line without height, a line whose two ends differ in height,
/// a text and a point.
fn drawing() -> String {
    let groups: &[(&str, &str)] = &[
        ("0", "SECTION"),
        ("2", "HEADER"),
        ("9", "$ACADVER"),
        ("1", "AC1015"),
        ("9", "$DWGCODEPAGE"),
        ("3", "ANSI_949"),
        ("0", "ENDSEC"),
        ("0", "SECTION"),
        ("2", "ENTITIES"),
        ("0", "LWPOLYLINE"),
        ("8", "F0017111"),
        ("38", "120.0"),
        ("90", "3"),
        ("70", "0"),
        ("10", "211300.0"),
        ("20", "364200.0"),
        ("10", "211400.0"),
        ("20", "364200.0"),
        ("10", "211400.0"),
        ("20", "364300.5"),
        ("0", "LWPOLYLINE"),
        ("8", "F0017114"),
        ("38", "95.5"),
        ("90", "4"),
        ("70", "1"),
        ("10", "212000.0"),
        ("20", "365000.0"),
        ("10", "212100.0"),
        ("20", "365000.0"),
        ("10", "212100.0"),
        ("20", "365100.0"),
        ("10", "212000.0"),
        ("20", "365100.0"),
        ("0", "LWPOLYLINE"),
        ("8", "A0013110"),
        ("90", "2"),
        ("70", "0"),
        ("10", "213000.0"),
        ("20", "366000.0"),
        ("10", "213050.0"),
        ("20", "366025.0"),
        ("0", "LINE"),
        ("8", "E0032111"),
        ("10", "214000.0"),
        ("20", "367000.0"),
        ("30", "10.0"),
        ("11", "214100.0"),
        ("21", "367000.0"),
        ("31", "12.0"),
        ("0", "TEXT"),
        ("8", "H0040000"),
        ("10", "211350.0"),
        ("20", "364250.0"),
        ("40", "5.0"),
        ("1", "name"),
        ("0", "POINT"),
        ("8", "C0010000"),
        ("10", "211360.0"),
        ("20", "364260.0"),
        ("0", "ENDSEC"),
        ("0", "EOF"),
    ];
    groups.iter().map(|(code, value)| format!("{code}\n{value}\n")).collect()
}

fn read_all(path: &Path) -> (Vec<DxfLine>, DxfFacts) {
    let mut lines = Vec::new();
    let facts = read_dxf_lines(path, &mut |line| {
        lines.push(line);
        true
    })
    .expect("read");
    (lines, facts)
}

#[test]
fn lines_keep_their_layer_points_and_height() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sheet.dxf");
    std::fs::write(&path, drawing()).expect("write");
    let (lines, facts) = read_all(&path);
    assert_eq!(facts, DxfFacts { lines: 4, others: 2 });
    assert_eq!(lines.iter().map(|l| l.layer.as_str()).collect::<Vec<_>>(), ["F0017111", "F0017114", "A0013110", "E0032111"]);
    assert_eq!(lines[0], DxfLine { layer: "F0017111".to_owned(), points: vec![[211_300.0, 364_200.0], [211_400.0, 364_200.0], [211_400.0, 364_300.5]], elevation: Some(120.0) });
    // A closed polyline comes back to its first point.
    assert_eq!(lines[1].points, [[212_000.0, 365_000.0], [212_100.0, 365_000.0], [212_100.0, 365_100.0], [212_000.0, 365_100.0], [212_000.0, 365_000.0]]);
    assert_eq!(lines[1].elevation, Some(95.5));
    // A line without heights has none; one whose ends differ has none either.
    assert_eq!((lines[2].points.len(), lines[2].elevation), (2, None));
    assert_eq!((lines[3].points.clone(), lines[3].elevation), (vec![[214_000.0, 367_000.0], [214_100.0, 367_000.0]], None));
}

#[test]
fn reading_stops_when_the_caller_has_enough_and_a_file_that_is_no_drawing_is_said() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sheet.dxf");
    std::fs::write(&path, drawing()).expect("write");
    let mut seen = 0;
    read_dxf_lines(&path, &mut |_| {
        seen += 1;
        seen < 2
    })
    .expect("read");
    assert_eq!(seen, 2);

    assert!(read_dxf_lines(&dir.path().join("none.dxf"), &mut |_| true).is_err());
    let other = dir.path().join("table.dxf");
    std::fs::write(&other, b"not a drawing").expect("write");
    assert!(read_dxf_lines(&other, &mut |_| true).is_err());
}

/// The layers of the drawing with what they hold and where: lines, the text and the point each as their
/// own entry, sorted by layer.
#[test]
fn each_layer_has_its_count_and_its_box() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sheet.dxf");
    std::fs::write(&path, drawing()).expect("write");
    let layers = dxf_layer_extents(&path).expect("scan");
    let got: Vec<(&str, DxfShape, u64, [f64; 4])> = layers.iter().map(|l| (l.layer.as_str(), l.shape, l.count, l.extent)).collect();
    assert_eq!(
        got,
        [
            ("A0013110", DxfShape::Line, 1, [213_000.0, 366_000.0, 213_050.0, 366_025.0]),
            ("C0010000", DxfShape::Point, 1, [211_360.0, 364_260.0, 211_360.0, 364_260.0]),
            ("E0032111", DxfShape::Line, 1, [214_000.0, 367_000.0, 214_100.0, 367_000.0]),
            ("F0017111", DxfShape::Line, 1, [211_300.0, 364_200.0, 211_400.0, 364_300.5]),
            ("F0017114", DxfShape::Line, 1, [212_000.0, 365_000.0, 212_100.0, 365_100.0]),
            ("H0040000", DxfShape::Point, 1, [211_350.0, 364_250.0, 211_350.0, 364_250.0]),
        ]
    );
    assert!(dxf_layer_extents(&dir.path().join("none.dxf")).is_err());
}

/// Two entities of one layer and kind share an entry whose box holds both.
#[test]
fn entities_of_one_layer_and_kind_share_a_box() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("two.dxf");
    let more = drawing().replace("0\nENDSEC\n0\nEOF\n", "0\nLWPOLYLINE\n8\nF0017111\n38\n130.0\n90\n2\n70\n0\n10\n211000.0\n20\n364500.0\n10\n211350.0\n20\n364100.0\n0\nENDSEC\n0\nEOF\n");
    assert_ne!(more, drawing());
    std::fs::write(&path, more).expect("write");
    let layers = dxf_layer_extents(&path).expect("scan");
    let contour = layers.iter().find(|l| l.layer == "F0017111").expect("layer");
    assert_eq!((contour.shape, contour.count, contour.extent), (DxfShape::Line, 2, [211_000.0, 364_100.0, 211_400.0, 364_500.0]));
}
