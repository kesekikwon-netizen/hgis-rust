//! A reference table read for drawing: every point in the work CRS, the look of each shape from its row.

use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout, Transformer};

use super::*;

fn square(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]
}

fn world(data: &RefVectorData, run: Run) -> Vec<[f64; 2]> {
    data.coords[run.start as usize..(run.start + run.len) as usize].iter().map(|c| [data.origin[0] + f64::from(c[0]), data.origin[1] + f64::from(c[1])]).collect()
}

fn geology_file(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("지질도_KIGAM.gpkg");
    let layout = TableLayout { table: "geology_map", epsg: 5186, kind: ShapeKind::Polygons, columns: &[("기호", ColumnKind::Text), ("ka_color", ColumnKind::Text)] };
    let rows = vec![
        Row {
            shapes: Shapes::Polygons(vec![vec![square(334_000.0, 270_000.0, 400.0), square(334_100.0, 270_100.0, 100.0)], vec![square(335_000.0, 271_000.0, 50.0)]]),
            cells: vec![Cell::Text("Qa".to_owned()), Cell::Text("#FAF4B4".to_owned())],
        },
        Row { shapes: Shapes::Polygons(vec![vec![square(336_000.0, 272_000.0, 25.0)]]), cells: vec![Cell::Text("Kswt".to_owned()), Cell::Null] },
        Row { shapes: Shapes::Polygons(vec![vec![square(337_000.0, 273_000.0, 25.0)]]), cells: vec![Cell::Text("x".to_owned()), Cell::Text("not a colour".to_owned())] },
    ];
    write_new_table(&path, &layout, &mut rows.into_iter()).expect("write");
    path
}

fn geology_look() -> Look {
    Look::Polygons { stroke: [80, 80, 80, 130], width_mm: 0.12, fill: PolygonFill::Column { name: "ka_color", alpha: 220, missing: [200, 200, 200] } }
}

/// Polygons keep their rings and parts, and their points arrive in the work CRS through PROJ: the stored
/// offsets are single precision around an origin near the data, which costs at most 2 mm over 30 km.
#[test]
fn polygons_arrive_in_the_work_crs_with_rings_and_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = load(&LoadSpec { file: geology_file(dir.path()), table: "geology_map".to_owned(), work_epsg: 5187, look: geology_look() }).expect("load");
    assert_eq!(data.rows, 3);
    assert!(data.closed);
    // Row one has two polygons (the first with a hole), rows two and three one each.
    assert_eq!(data.items.len(), 4);
    assert_eq!(data.items[0].run_count, 2, "outer ring and hole");
    assert_eq!(data.items[1].run_count, 1);
    let carry = Transformer::new(5186, 5187).expect("transformer");
    let hole = world(&data, data.runs[(data.items[0].first_run + 1) as usize]);
    let expected = carry.transform_all(&square(334_100.0, 270_100.0, 100.0)).expect("points");
    assert_eq!(hole.len(), 5);
    for (got, want) in hole.iter().zip(&expected) {
        assert!((got[0] - want[0]).abs() < 0.003 && (got[1] - want[1]).abs() < 0.003, "{got:?} against {want:?}");
    }
    // The layer's box holds every point.
    for run in &data.runs {
        for p in world(&data, *run) {
            assert!(p[0] >= data.bbox[0] - 0.003 && p[0] <= data.bbox[2] + 0.003 && p[1] >= data.bbox[1] - 0.003 && p[1] <= data.bbox[3] + 0.003);
        }
    }
    // Each shape has its own box, as offsets from the origin.
    let item = data.items[3];
    let run = world(&data, data.runs[item.first_run as usize]);
    let (min_x, max_x) = run.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
    assert!((data.origin[0] + f64::from(item.bbox[0]) - min_x).abs() < 0.003 && (data.origin[0] + f64::from(item.bbox[2]) - max_x).abs() < 0.003);
}

/// The fill of a polygon is the colour its row names, with the layer's opacity; a row without a readable
/// colour gets the layer's stand-in colour. Shapes that look alike share one paint.
#[test]
fn the_fill_comes_from_the_colour_column() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data = load(&LoadSpec { file: geology_file(dir.path()), table: "geology_map".to_owned(), work_epsg: 5186, look: geology_look() }).expect("load");
    let paint = |i: usize| data.paints[data.items[i].paint as usize];
    assert_eq!(paint(0).fill, [0xFA, 0xF4, 0xB4, 220]);
    assert_eq!(paint(1).fill, [0xFA, 0xF4, 0xB4, 220]);
    assert_eq!(data.items[0].paint, data.items[1].paint);
    assert_eq!(paint(2).fill, [200, 200, 200, 220], "no colour in the row");
    assert_eq!(paint(3).fill, [200, 200, 200, 220], "a text that is no colour");
    assert_eq!(data.paints.len(), 2);
    assert!(data.paints.iter().all(|p| p.stroke == [80, 80, 80, 130] && (p.width_mm - 0.12).abs() < 1e-6));
    // The same CRS on both sides: the points are the stored ones.
    let outer = world(&data, data.runs[data.items[0].first_run as usize]);
    assert!((outer[2][0] - 334_400.0).abs() < 0.003 && (outer[2][1] - 270_400.0).abs() < 0.003);
}

/// Lines: each part of a row is one shape, all drawn alike.
#[test]
fn lines_are_one_shape_for_each_part() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("수치지형도-합침-1.gpkg");
    let layout = TableLayout { table: "topo", epsg: 5187, kind: ShapeKind::Lines, columns: &[("Layer", ColumnKind::Text)] };
    let rows = vec![
        Row { shapes: Shapes::Lines(vec![vec![[220_000.0, 370_000.0], [220_010.0, 370_010.0]], vec![[220_100.0, 370_000.0], [220_110.0, 370_010.0], [220_120.0, 370_000.0]]]), cells: vec![Cell::Text("F0017111".to_owned())] },
        Row { shapes: Shapes::Lines(vec![vec![[221_000.0, 371_000.0], [221_500.0, 371_000.0]]]), cells: vec![Cell::Text("A0013115".to_owned())] },
    ];
    write_new_table(&path, &layout, &mut rows.into_iter()).expect("write");
    let look = Look::Lines { stroke: [128, 128, 128, 255], width_mm: 0.2 };
    let data = load(&LoadSpec { file: path, table: "topo".to_owned(), work_epsg: 5187, look }).expect("load");
    assert_eq!((data.rows, data.items.len(), data.runs.len()), (2, 3, 3));
    assert!(!data.closed);
    assert_eq!(data.paints, [PaintOf { fill: [0, 0, 0, 0], stroke: [128, 128, 128, 255], width_mm: 0.2 }]);
    assert_eq!(world(&data, data.runs[1]).len(), 3);
    assert_eq!(data.bbox, [220_000.0, 370_000.0, 221_500.0, 371_000.0]);
}

/// What cannot be drawn is said, not drawn wrongly: a missing file, a table of the other kind of shape, a
/// table without the colour column, a CRS that PROJ does not know.
#[test]
fn a_table_that_does_not_fit_the_look_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = geology_file(dir.path());
    let spec = |table: &str, look: Look, work_epsg: u32| LoadSpec { file: file.clone(), table: table.to_owned(), work_epsg, look };
    assert!(load(&spec("none", geology_look(), 5187)).is_err());
    assert!(load(&spec("geology_map", Look::Lines { stroke: [0, 0, 0, 255], width_mm: 0.2 }, 5187)).is_err());
    let other_column = Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.1, fill: PolygonFill::Column { name: "colour", alpha: 255, missing: [0, 0, 0] } };
    assert!(load(&spec("geology_map", other_column, 5187)).is_err());
    assert!(load(&spec("geology_map", geology_look(), 999_999)).is_err());
    let missing = LoadSpec { file: dir.path().join("none.gpkg"), table: "geology_map".to_owned(), work_epsg: 5187, look: geology_look() };
    assert!(load(&missing).is_err());
    // Without a fill no column is asked for.
    let outline = Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.1, fill: PolygonFill::None };
    let data = load(&spec("geology_map", outline, 5186)).expect("load");
    assert_eq!(data.paints, [PaintOf { fill: [0, 0, 0, 0], stroke: [0, 0, 0, 255], width_mm: 0.1 }]);
}

/// The kind of shape is checked before any row: an empty table of polygons is no layer of lines either.
#[test]
fn an_empty_table_of_the_other_kind_is_refused_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.gpkg");
    let layout = TableLayout { table: "geology_map", epsg: 5186, kind: ShapeKind::Polygons, columns: &[("기호", ColumnKind::Text)] };
    write_new_table(&path, &layout, &mut std::iter::empty()).expect("write");
    let lines = LoadSpec { file: path.clone(), table: "geology_map".to_owned(), work_epsg: 5187, look: Look::Lines { stroke: [0, 0, 0, 255], width_mm: 0.2 } };
    assert!(load(&lines).is_err());
    let polygons = LoadSpec { look: Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.2, fill: PolygonFill::None }, ..lines };
    let data = load(&polygons).expect("an empty layer of the right kind is a layer");
    assert!(data.items.is_empty() && data.closed && data.rows == 0);
}
