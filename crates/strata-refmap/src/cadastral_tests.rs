//! The import of the digital cadastral map, as `target/cpp-ref/tests/test_cadastral.cpp:257-459` checks it.
//! The shapefiles are made here (made-up parcels in EPSG:5186, as the portal's county files are); no
//! received data is used.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell as Flag;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType};
use gdal::DriverManager;
use sha2::{Digest, Sha256};
use strata_engine::{polygons_area, read_table, table_facts, Cell, ColumnKind, Scope, ShapeKind, Shapes, Transformer};

use super::*;

pub(super) fn pnu(parcel: u32) -> String {
    format!("471901010010{parcel:07}")
}

pub(super) fn square(x: f64, y: f64, width: f64, height: f64) -> String {
    format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + width, x + width, y + height, y + height)
}

/// A county file of made-up parcels: `(number, lot number, WKT)`, with a column the import must leave out
/// (`test_cadastral.cpp:50-75`). `jibun: false` leaves the lot numbers out, `crs: false` writes no `.prj`.
pub(super) fn parcels(dir: &Path, stem: &str, rows: &[(u32, &str, String)], jibun: bool, crs: bool) -> PathBuf {
    let path = dir.join(format!("{stem}.shp"));
    let mut file = DriverManager::get_driver_by_name("ESRI Shapefile").unwrap().create_vector_only(&path).unwrap();
    let source = SpatialRef::from_epsg(5186).unwrap();
    let layer = file.create_layer(LayerOptions { name: stem, srs: crs.then_some(&source), ty: OGRwkbGeometryType::wkbPolygon, options: Some(&["ENCODING=UTF-8"]) }).unwrap();
    for name in ["PNU", "OWNER"].into_iter().chain(jibun.then_some("JIBUN")) {
        FieldDefn::new(name, OGRFieldType::OFTString).unwrap().add_to_layer(&layer).unwrap();
    }
    for (number, lot, wkt) in rows {
        let mut feature = Feature::new(layer.defn()).unwrap();
        feature.set_field_string(0, &pnu(*number)).unwrap();
        feature.set_field_string(1, "제외할 개인정보").unwrap();
        if jibun {
            feature.set_field_string(2, lot).unwrap();
        }
        feature.set_geometry(Geometry::from_wkt(wkt).unwrap()).unwrap();
        feature.create(&layer).unwrap();
    }
    drop(file);
    path
}

/// The files of a folder with their digests: nothing may change what was received.
pub(super) fn digests(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir).unwrap().map(Result::unwrap).filter(|e| e.path().is_file()).map(|e| (e.file_name().to_string_lossy().into_owned(), Sha256::digest(std::fs::read(e.path()).unwrap()).to_vec())).collect()
}

/// The polygons of a parcel, each as its rings.
pub(super) type Shape = Vec<Vec<Vec<[f64; 2]>>>;

/// The rows of a finished table: PNU, lot number and shape.
pub(super) fn written(file: &Path) -> Vec<(String, String, Shape)> {
    let mut rows = Vec::new();
    read_table(file, TABLE, &[PNU_COLUMN, JIBUN_COLUMN], &mut |row| {
        let text = |cell: &Cell| if let Cell::Text(text) = cell { text.clone() } else { String::new() };
        let Shapes::Polygons(polygons) = row.shapes else { panic!("polygons") };
        rows.push((text(&row.cells[0]), text(&row.cells[1]), polygons));
        true
    })
    .unwrap();
    rows
}

pub(super) fn wide() -> Scope {
    Scope::of(&[square(199_900.0, 549_900.0, 400.0, 400.0)]).unwrap().unwrap()
}

fn quiet() -> impl FnMut(u8, &str) {
    |_, _| {}
}

/// `importsWholeIntersectingParcels` (`:257-324`), for a file in the work CRS and for one that is carried
/// into it: a parcel that meets the scope is kept whole, also where it sticks out of it; one that only
/// meets the scope's box is left out; the table holds PNU and lot number and nothing else of the file.
#[test]
fn whole_parcels_that_meet_the_scope_are_kept() {
    for work in [5186_u32, 5187] {
        let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let boundary = square(200_045.0, 550_000.0, 40.0, 10.0);
        let inside = square(199_995.0, 549_995.0, 10.0, 10.0);
        let crossing = square(200_075.0, 550_000.0, 40.0, 10.0);
        let source = parcels(input.path(), "source", &[(1, "산 12-3", boundary.clone()), (2, "15", inside.clone()), (3, "999", square(200_080.0, 550_080.0, 10.0, 10.0))], true, true);
        let extra = parcels(input.path(), "crossing", &[(4, "16-1", crossing.clone())], true, true);
        let before = digests(input.path());
        let diamond = Scope::of(&["POLYGON ((200000 549900, 200100 550000, 200000 550100, 199900 550000, 200000 549900))".to_owned()]).unwrap().unwrap();
        let scope = if work == 5186 { diamond } else { diamond.reprojected(5186, work).unwrap() };
        let mut said = Vec::new();

        let ready = prepare(&[source, extra], Some(&scope), work, output.path(), &|| false, &mut |value, words| said.push((value, words.to_owned()))).unwrap();

        assert_eq!((ready.parcels, ready.reused), (3, false), "work CRS {work}");
        let facts = table_facts(&ready.file, TABLE).unwrap();
        assert_eq!((facts.epsg, facts.kind, facts.rows), (Some(work), ShapeKind::Polygons, 3));
        assert_eq!(facts.columns, [(PNU_COLUMN.to_owned(), ColumnKind::Text), (JIBUN_COLUMN.to_owned(), ColumnKind::Text)], "no owner column");
        let to_work = Transformer::new(5186, work).unwrap();
        let mut rows = written(&ready.file);
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        for ((number, lot, whole), (got_pnu, got_lot, shape)) in [(1, "산 12-3", &boundary), (2, "15", &inside), (4, "16-1", &crossing)].into_iter().zip(&rows) {
            assert_eq!((got_pnu.as_str(), got_lot.as_str()), (pnu(number).as_str(), lot));
            // The whole outline, carried into the work CRS: the same corners, wherever the ring starts.
            let corners = |ring: &[[f64; 2]]| {
                let mut points: Vec<[i64; 2]> = ring[..ring.len() - 1].iter().map(|p| [(p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64]).collect();
                points.sort_unstable();
                points
            };
            let mut xyz = Vec::new();
            Geometry::from_wkt(whole).unwrap().get_geometry(0).get_points(&mut xyz);
            let ring: Vec<[f64; 2]> = xyz.into_iter().map(|(x, y, _)| [x, y]).collect();
            let expected = to_work.transform_all(&ring).unwrap();
            assert_eq!((shape.len(), shape[0].len()), (1, 1));
            assert_eq!(corners(&shape[0][0]), corners(&expected), "parcel {number} in {work}");
        }
        assert_eq!(digests(input.path()), before, "the received files are as they were");
        assert_eq!(said.first(), Some(&(0, "조사 주변 필지와 지번을 정리하고 있습니다…".to_owned())));
        assert_eq!(said.last(), Some(&(100, "주변 필지 3개를 준비했습니다.".to_owned())));
        // The digest beside the table is that of the table (`CadastralImport.cpp:230-232`).
        let digest: String = Sha256::digest(std::fs::read(&ready.file).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(std::fs::read_to_string(format!("{}.sha256", ready.file.display())).unwrap(), digest);
        assert_eq!(ready.file.parent(), Some(output.path()));
    }
}

/// `overlappingSourcesDeduplicatePnu` (`:326-344`): a parcel that two county files hold is written once.
#[test]
fn a_parcel_in_two_sources_is_written_once() {
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (a, b) = ((1, "1", square(200_000.0, 550_000.0, 10.0, 10.0)), (2, "2", square(200_020.0, 550_000.0, 10.0, 10.0)));
    let first = parcels(input.path(), "first", std::slice::from_ref(&a), true, true);
    let second = parcels(input.path(), "second", &[a, b], true, true);
    let ready = prepare(&[first, second], Some(&wide()), 5186, output.path(), &|| false, &mut quiet()).unwrap();
    let mut numbers: Vec<String> = written(&ready.file).into_iter().map(|row| row.0).collect();
    numbers.sort();
    assert_eq!(numbers, [pnu(1), pnu(2)]);
}

/// `reusesCompletedCacheWithoutRewritingOriginalOrOutput` (`:346-376`): the same sources and the same
/// scope find the finished table again; neither it nor the received files are written a second time.
#[test]
fn a_finished_table_is_used_again_untouched() {
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let source = parcels(input.path(), "source", &[(1, "산 12-3", square(200_000.0, 550_000.0, 10.0, 10.0))], true, true);
    let originals = digests(input.path());
    let first = prepare(std::slice::from_ref(&source), Some(&wide()), 5186, output.path(), &|| false, &mut quiet()).unwrap();
    // An old time tells a table that was found from one that was written again just the same.
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(946_684_800);
    std::fs::File::options().write(true).open(&first.file).unwrap().set_modified(old).unwrap();
    let kept = digests(output.path());
    let mut said = Vec::new();

    let second = prepare(&[source], Some(&wide()), 5186, output.path(), &|| false, &mut |value, words| said.push((value, words.to_owned()))).unwrap();

    assert_eq!((second.file.clone(), second.parcels, second.reused), (first.file, 1, true));
    assert_eq!(std::fs::metadata(&second.file).unwrap().modified().unwrap(), old);
    assert_eq!(digests(output.path()), kept);
    assert_eq!(digests(input.path()), originals);
    assert_eq!(written(&second.file)[0].1, "산 12-3");
    assert_eq!(said, [(100, "저장된 조사 주변 지적도를 사용합니다.".to_owned())]);
}

/// `samePnuKeepsDistinctGeometryAndDeduplicatesEquivalentRings` (`:378-405`): one PNU may have several
/// pieces, which all stay; the same piece from a second file, its ring begun at another corner, does not.
#[test]
fn one_pnu_keeps_its_pieces_and_loses_the_same_ring_twice() {
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let first = parcels(input.path(), "first", &[(1, "1", square(200_000.0, 550_000.0, 10.0, 10.0))], true, true);
    let turned = "POLYGON ((200010 550010, 200010 550000, 200000 550000, 200000 550010, 200010 550010))".to_owned();
    let second = parcels(input.path(), "second", &[(1, "1", turned), (1, "1", square(200_030.0, 550_000.0, 10.0, 10.0))], true, true);
    let ready = prepare(&[first, second], Some(&wide()), 5186, output.path(), &|| false, &mut quiet()).unwrap();
    let rows = written(&ready.file);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.0 == pnu(1)));
    let area: f64 = rows.iter().map(|row| polygons_area(&row.2).unwrap()).sum();
    assert!((area - 200.0).abs() < 0.01, "{area}");
}

/// `rejectsInvalidInputs` (`:407-428`) with the words of `CadastralImport.cpp:82, 111, 158, 165`.
#[test]
fn what_cannot_be_prepared_is_refused() {
    let lot = [(1, "1", square(200_000.0, 550_000.0, 10.0, 10.0))];
    let run = |jibun: bool, crs: bool, missing: bool, work: u32, scope: bool| {
        let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let source = parcels(input.path(), "source", &lot, jibun, crs);
        let source = if missing { input.path().join("absent.shp") } else { source };
        let wide = wide();
        prepare(&[source], scope.then_some(&wide), work, output.path(), &|| false, &mut quiet())
    };
    let failed = |words: &str| Err(Stopped::Failed(words.to_owned()));
    assert_eq!(run(false, true, false, 5186, true), failed("지적도 원본에 지번(JIBUN) 항목이 없습니다."));
    assert_eq!(run(true, false, false, 5186, true), failed("지적도 도형 또는 원본 좌표계를 확인하지 못했습니다."));
    assert_eq!(run(true, true, true, 5186, true), failed("지적도 원본 파일을 읽지 못했습니다."));
    assert_eq!(run(true, true, false, 0, true), failed("지적도 원본과 유효한 조사 범위·좌표계가 필요합니다."));
    assert_eq!(run(true, true, false, 5186, false), failed("지적도 원본과 유효한 조사 범위·좌표계가 필요합니다."));
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let far = Scope::of(&[square(100_000.0, 100_000.0, 10.0, 10.0)]).unwrap().unwrap();
    let source = parcels(input.path(), "source", &lot, true, true);
    assert_eq!(prepare(&[source], Some(&far), 5186, output.path(), &|| false, &mut quiet()), failed("조사 주변 범위와 겹치는 필지가 없습니다."));
    assert_eq!(prepare(&[], Some(&wide()), 5186, output.path(), &|| false, &mut quiet()), failed("지적도 원본과 유효한 조사 범위·좌표계가 필요합니다."));
}

/// `cancellationDoesNotReturnReady` (`:430-443`) and `cancellationDuringPreparationLeavesNoCompletedMap`
/// (`:445-459`): a stop, before the start or in the middle, ends as stopped and leaves no table behind.
#[test]
fn a_stop_leaves_no_table() {
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let source = parcels(input.path(), "source", &[(1, "1", square(200_000.0, 550_000.0, 10.0, 10.0))], true, true);
    let original = digests(input.path());
    let tables = |dir: &Path| std::fs::read_dir(dir).unwrap().map(Result::unwrap).filter(|e| e.path().extension().is_some_and(|x| x == "gpkg")).count();

    assert_eq!(prepare(std::slice::from_ref(&source), Some(&wide()), 5186, output.path(), &|| true, &mut quiet()), Err(Stopped::Cancelled));
    assert_eq!((digests(input.path()), tables(output.path())), (original, 0));

    let stop = Flag::new(false);
    let result = prepare(&[source], Some(&wide()), 5186, output.path(), &|| stop.get(), &mut |value, _| {
        if value < 100 {
            stop.set(true);
        }
    });
    assert!(stop.get());
    assert_eq!(result, Err(Stopped::Cancelled));
    assert_eq!(tables(output.path()), 0);
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0, "nor a draft, nor a lock");
}
