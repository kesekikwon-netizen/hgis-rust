//! A table written must come back exactly: reference layers are drawn from these files.

use super::*;

fn square(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]
}

fn geology_layout<'a>() -> TableLayout<'a> {
    TableLayout {
        table: "geology_map",
        epsg: 5186,
        kind: ShapeKind::Polygons,
        columns: &[("시대", ColumnKind::Text), ("기호", ColumnKind::Text), ("area_m2", ColumnKind::Real), ("symnum", ColumnKind::Integer)],
    }
}

fn geology_rows() -> Vec<Row> {
    let with_hole = vec![square(200_000.0, 450_000.0, 300.0), square(200_100.0, 450_100.0, 50.0)];
    vec![
        Row {
            shapes: Shapes::Polygons(vec![with_hole, vec![square(201_000.0, 451_000.0, 10.5)]]),
            cells: vec![Cell::Text("현생누대 신생대 제4기".to_owned()), Cell::Text("Qa".to_owned()), Cell::Real(1234.5), Cell::Integer(352_001)],
        },
        Row {
            shapes: Shapes::Polygons(vec![vec![vec![[0.125, 0.25], [10.0, 0.0], [5.0, 7.75], [0.125, 0.25]]]]),
            cells: vec![Cell::Null, Cell::Text("Kswt".to_owned()), Cell::Null, Cell::Null],
        },
    ]
}

fn topo_layout<'a>() -> TableLayout<'a> {
    TableLayout {
        table: "topo",
        epsg: 5187,
        kind: ShapeKind::Lines,
        columns: &[("Layer", ColumnKind::Text), ("elevation", ColumnKind::Real), ("z_valid", ColumnKind::Integer), ("sheet", ColumnKind::Text)],
    }
}

fn line_row(sheet: &str, x: f64) -> Row {
    Row {
        shapes: Shapes::Lines(vec![vec![[x, 370_000.0], [x + 10.0, 370_010.0], [x + 20.0, 370_005.0]]]),
        cells: vec![Cell::Text("F0017111".to_owned()), Cell::Real(50.0), Cell::Integer(1), Cell::Text(sheet.to_owned())],
    }
}

fn read_all(path: &std::path::Path, table: &str, columns: &[&str]) -> (TableFacts, Vec<Row>) {
    let mut rows = Vec::new();
    let facts = read_table(path, table, columns, &mut |row| {
        rows.push(row);
        true
    })
    .expect("read");
    (facts, rows)
}

#[test]
fn polygons_with_their_cells_come_back_as_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("한글 폴더");
    std::fs::create_dir_all(&folder).expect("folder");
    let path = folder.join("지질도_KIGAM.gpkg");
    let written = write_new_table(&path, &geology_layout(), &mut geology_rows().into_iter()).expect("write");
    assert_eq!(written, 2);

    let (facts, rows) = read_all(&path, "geology_map", &["시대", "기호", "area_m2", "symnum"]);
    assert_eq!(facts.epsg, Some(5186));
    assert_eq!(facts.kind, ShapeKind::Polygons);
    assert_eq!(facts.rows, 2);
    let names: Vec<(&str, ColumnKind)> = facts.columns.iter().map(|(n, k)| (n.as_str(), *k)).collect();
    assert_eq!(names, geology_layout().columns);
    assert_eq!(rows, geology_rows());
    // The same facts without reading a row.
    assert_eq!(table_facts(&path, "geology_map").expect("facts"), facts);
    assert!(table_facts(&path, "topo").is_err());
}

#[test]
fn rows_can_be_added_to_a_table_later_and_columns_chosen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("수치지형도-합침-1.gpkg");
    assert_eq!(write_new_table(&path, &topo_layout(), &mut vec![line_row("359012", 220_000.0)].into_iter()).expect("write"), 1);
    let more = vec![line_row("359014", 221_000.0), line_row("359021", 222_000.0)];
    assert_eq!(append_to_table(&path, "topo", &mut more.into_iter()).expect("append"), 3);

    let (facts, rows) = read_all(&path, "topo", &["sheet", "Layer"]);
    assert_eq!((facts.epsg, facts.kind, facts.rows), (Some(5187), ShapeKind::Lines, 3));
    let sheets: Vec<&Cell> = rows.iter().map(|r| &r.cells[0]).collect();
    assert_eq!(sheets, [&Cell::Text("359012".to_owned()), &Cell::Text("359014".to_owned()), &Cell::Text("359021".to_owned())]);
    assert!(rows.iter().all(|r| r.cells.len() == 2 && r.cells[1] == Cell::Text("F0017111".to_owned())));
    assert_eq!(rows[2].shapes, line_row("359021", 222_000.0).shapes);
    // No column asked: the shapes alone.
    let (_, bare) = read_all(&path, "topo", &[]);
    assert!(bare.iter().all(|r| r.cells.is_empty()) && bare.len() == 3);
}

#[test]
fn an_existing_file_is_never_overwritten() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("kept.gpkg");
    write_new_table(&path, &topo_layout(), &mut vec![line_row("359012", 220_000.0)].into_iter()).expect("write");
    let again = write_new_table(&path, &topo_layout(), &mut vec![line_row("359014", 1.0), line_row("359021", 2.0)].into_iter());
    assert!(again.is_err());
    let (facts, rows) = read_all(&path, "topo", &["sheet"]);
    assert_eq!(facts.rows, 1);
    assert_eq!(rows[0].cells, [Cell::Text("359012".to_owned())]);
}

#[test]
fn reading_stops_when_the_caller_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("three.gpkg");
    let rows = vec![line_row("a", 1.0), line_row("b", 2.0), line_row("c", 3.0)];
    write_new_table(&path, &topo_layout(), &mut rows.into_iter()).expect("write");
    let mut seen = 0;
    read_table(&path, "topo", &[], &mut |_| {
        seen += 1;
        false
    })
    .expect("read");
    assert_eq!(seen, 1);
}

#[test]
fn a_missing_file_table_or_column_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("one.gpkg");
    write_new_table(&path, &topo_layout(), &mut vec![line_row("a", 1.0)].into_iter()).expect("write");
    assert!(read_table(&path, "geology_map", &[], &mut |_| true).is_err());
    assert!(read_table(&path, "topo", &["기호"], &mut |_| true).is_err());
    assert!(read_table(&dir.path().join("none.gpkg"), "topo", &[], &mut |_| true).is_err());
    assert!(append_to_table(&dir.path().join("none.gpkg"), "topo", &mut vec![line_row("a", 1.0)].into_iter()).is_err());
    assert!(append_to_table(&path, "geology_map", &mut vec![line_row("a", 1.0)].into_iter()).is_err());
}

#[test]
fn a_write_that_fails_leaves_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    // More cells than the table has columns.
    let mut long = line_row("a", 1.0);
    long.cells.push(Cell::Integer(7));
    let path = dir.path().join("cells.gpkg");
    assert!(write_new_table(&path, &topo_layout(), &mut vec![line_row("a", 1.0), long].into_iter()).is_err());
    assert!(!path.exists());
    // A polygon handed to a table of lines.
    let path = dir.path().join("kind.gpkg");
    let wrong = Row { shapes: Shapes::Polygons(vec![vec![square(0.0, 0.0, 1.0)]]), cells: line_row("a", 1.0).cells };
    assert!(write_new_table(&path, &topo_layout(), &mut vec![wrong].into_iter()).is_err());
    assert!(!path.exists());
    // A cell of another kind than its column.
    let path = dir.path().join("cell.gpkg");
    let mut text_in_number = line_row("a", 1.0);
    text_in_number.cells[1] = Cell::Text("fifty".to_owned());
    assert!(write_new_table(&path, &topo_layout(), &mut vec![text_in_number].into_iter()).is_err());
    assert!(!path.exists());
}

#[test]
fn an_append_that_fails_keeps_the_rows_that_were_there() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("kept-rows.gpkg");
    write_new_table(&path, &topo_layout(), &mut vec![line_row("359012", 1.0)].into_iter()).expect("write");
    let mut long = line_row("b", 2.0);
    long.cells.push(Cell::Null);
    assert!(append_to_table(&path, "topo", &mut vec![line_row("359014", 3.0), long].into_iter()).is_err());
    let (facts, rows) = read_all(&path, "topo", &["sheet"]);
    assert_eq!(facts.rows, 1, "the half-done addition is taken back");
    assert_eq!(rows[0].cells, [Cell::Text("359012".to_owned())]);
}
