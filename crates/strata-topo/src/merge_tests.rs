//! Small sheets written here, merged for a survey area in their middle: which lines are kept, in which
//! coordinates, with which fields (`KaTopographicImportDialog.cpp`, `SurveyScopeClip.cpp`).

use strata_engine::{read_table, Cell, Scope, ShapeKind, Shapes, Transformer};
use strata_net::ngii_topographic::TopographicSheets;

use super::*;
use crate::sheet::test_dxf::{drawing, Entity};

/// Sheet 367154 as drawn in EPSG:5186: `at(fx, fy)` is the point that far across the sheet (0 to 1). The
/// sheet is about 11 km wide and 14 km tall.
fn at(fx: f64, fy: f64) -> [f64; 2] {
    let e = TopographicSheets::resolve("367154").expect("sheet").extent();
    Transformer::new(4737, 5186).expect("crs").transform([e[0] + (e[2] - e[0]) * fx, e[1] + (e[3] - e[1]) * fy]).expect("point")
}

/// The 5 km scope of a 100 m survey area in the middle of the sheet, in `work_epsg`.
fn scope(work_epsg: u32) -> Scope {
    let [x, y] = at(0.5, 0.5);
    let square = format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 100.0, x + 100.0, y + 100.0, y + 100.0);
    let scope = Scope::around(&[square], 5_000.0).expect("scope").expect("some");
    if work_epsg == 5186 {
        scope
    } else {
        scope.reprojected(5186, work_epsg).expect("carried")
    }
}

fn write_sheet(dir: &std::path::Path, name: &str, entities: &[Entity<'_>]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, drawing(entities)).expect("write");
    path
}

fn shown(layers: &[&str]) -> BTreeSet<String> {
    layers.iter().map(|l| (*l).to_owned()).collect()
}

/// The stored lines: points, CAD layer, height, sheet.
fn stored(gpkg: &Path, epsg: u32) -> Vec<(Vec<[f64; 2]>, Vec<Cell>)> {
    let mut rows = Vec::new();
    let facts = read_table(gpkg, TABLE, &[LAYER_COLUMN, ELEVATION_COLUMN, SHEET_COLUMN], &mut |row| {
        let Shapes::Lines(mut lines) = row.shapes else {
            panic!("lines");
        };
        assert_eq!(lines.len(), 1);
        rows.push((lines.remove(0), row.cells));
        true
    })
    .expect("read");
    assert_eq!((facts.epsg, facts.kind), (Some(epsg), ShapeKind::Lines));
    rows
}

fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

fn never() -> bool {
    false
}

/// A contour beside the survey area and a road that leaves the 5 km scope are kept, the road whole; a
/// contour 7 km away is not; a line of a layer that is not shown is not.
#[test]
fn lines_that_touch_the_scope_are_kept_whole_with_their_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (near, road, far, wood) = ([at(0.45, 0.5), at(0.5, 0.55), at(0.55, 0.5)], [at(0.5, 0.5), at(0.98, 0.5)], [at(0.02, 0.02), at(0.05, 0.02)], [at(0.5, 0.45), at(0.52, 0.45)]);
    let dxf = write_sheet(dir.path(), "367154.dxf", &[Entity::Line("F0017111", Some(120.0), &near), Entity::Line("A0013110", None, &road), Entity::Line("F0017111", Some(80.0), &far), Entity::Line("D0010000", None, &wood)]);
    let sheets = [SheetLines { dxf, number: "367154".to_owned(), epsg: 5186, shown: shown(&["A0013110", "F0017111"]) }];
    let out = dir.path().join("합침").join("수치지형도-합침-1.gpkg");
    std::fs::create_dir_all(out.parent().expect("parent")).expect("folder");
    assert_eq!(merge(&sheets, &scope(5186), 5186, &out, &never), Ok(2));

    let rows = stored(&out, 5186);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, [text("F0017111"), Cell::Real(120.0), text("367154")]);
    assert_eq!(rows[1].1, [text("A0013110"), Cell::Null, text("367154")]);
    for (got, want) in rows[0].0.iter().zip(near) {
        assert!((got[0] - want[0]).abs() < 0.001 && (got[1] - want[1]).abs() < 0.001);
    }
    // The road's far end, 5.3 km east of the survey area, is still there.
    let end = rows[1].0.last().expect("end");
    assert!((end[0] - road[1][0]).abs() < 0.001 && end[0] - at(0.5, 0.5)[0] > 5_200.0, "{end:?}");
}

/// A sheet drawn in another belt than the work CRS is carried into it.
#[test]
fn lines_are_carried_into_the_work_crs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let near = [at(0.45, 0.5), at(0.5, 0.55)];
    let dxf = write_sheet(dir.path(), "367154.dxf", &[Entity::Line("F0017111", Some(120.0), &near)]);
    let sheets = [SheetLines { dxf, number: "367154".to_owned(), epsg: 5186, shown: shown(&["F0017111"]) }];
    let out = dir.path().join("topo.gpkg");
    assert_eq!(merge(&sheets, &scope(5187), 5187, &out, &never), Ok(1));
    let rows = stored(&out, 5187);
    let wanted = Transformer::new(5186, 5187).expect("crs").transform_all(&near).expect("points");
    for (got, want) in rows[0].0.iter().zip(wanted) {
        assert!((got[0] - want[0]).abs() < 0.001 && (got[1] - want[1]).abs() < 0.001, "{got:?} for {want:?}");
    }
}

/// The lines of several sheets go into the one table, each with its sheet's number.
#[test]
fn several_sheets_share_the_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = write_sheet(dir.path(), "a.dxf", &[Entity::Line("F0017111", Some(100.0), &[at(0.5, 0.5), at(0.52, 0.5)])]);
    let empty = write_sheet(dir.path(), "b.dxf", &[Entity::Line("F0017111", Some(90.0), &[at(0.02, 0.02), at(0.05, 0.02)])]);
    let second = write_sheet(dir.path(), "c.dxf", &[Entity::Line("E0010001", None, &[at(0.5, 0.52), at(0.52, 0.52)]), Entity::Line("E0010001", None, &[at(0.5, 0.53), at(0.52, 0.53)])]);
    let sheet = |dxf: PathBuf, number: &str, layer: &str| SheetLines { dxf, number: number.to_owned(), epsg: 5186, shown: shown(&[layer]) };
    let sheets = [sheet(first, "367154", "F0017111"), sheet(empty, "367153", "F0017111"), sheet(second, "367152", "E0010001")];
    let out = dir.path().join("topo.gpkg");
    assert_eq!(merge(&sheets, &scope(5186), 5186, &out, &never), Ok(3));
    let sheets_of: Vec<Cell> = stored(&out, 5186).into_iter().map(|(_, cells)| cells[2].clone()).collect();
    assert_eq!(sheets_of, [text("367154"), text("367152"), text("367152")]);
}

/// Nothing to keep, a stop, a sheet that cannot be read: no file is left.
#[test]
fn without_lines_or_after_a_stop_or_a_failure_no_file_is_left() {
    let dir = tempfile::tempdir().expect("tempdir");
    let far = write_sheet(dir.path(), "far.dxf", &[Entity::Line("F0017111", Some(80.0), &[at(0.02, 0.02), at(0.05, 0.02)])]);
    let near = write_sheet(dir.path(), "near.dxf", &[Entity::Line("F0017111", Some(80.0), &[at(0.5, 0.5), at(0.52, 0.5)])]);
    let sheet = |dxf: PathBuf| SheetLines { dxf, number: "367154".to_owned(), epsg: 5186, shown: shown(&["F0017111"]) };
    let out = dir.path().join("topo.gpkg");
    assert_eq!(merge(&[sheet(far)], &scope(5186), 5186, &out, &never), Ok(0));
    assert!(!out.exists());
    assert_eq!(merge(&[sheet(near.clone())], &scope(5186), 5186, &out, &|| true), Err(MergeStop::Cancelled));
    assert!(!out.exists());
    // The second sheet is missing after the first was written: the file of the first does not stay.
    let stopped = merge(&[sheet(near), sheet(dir.path().join("none.dxf"))], &scope(5186), 5186, &out, &never);
    assert_eq!(stopped, Err(MergeStop::Failed(format!("{NOT_OPENED}: 367154. 기존 지도와 받은 원본은 유지됩니다."))));
    assert!(!out.exists());
}
