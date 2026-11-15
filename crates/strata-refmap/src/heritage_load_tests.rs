//! Loading one kind of received heritage data (`HeritageImport::loadDataset`), on made-up shapefiles. The
//! unpacking is handed in: here it lists a folder.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::{Scope, ShapeKind};

use super::*;
use crate::heritage::tests::{names_in, shapefile_of};
use crate::heritage::{DATASETS, NEAR_M};

const CENTRE: [f64; 2] = [220_000.0, 370_000.0];

/// A survey area of 100 m around `CENTRE` (EPSG:5187), grown by 5 km.
fn scope() -> Scope {
    let [x, y] = CENTRE;
    let area = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", x - 50.0, y - 50.0, x + 50.0, y - 50.0, x + 50.0, y + 50.0, x - 50.0, y + 50.0, x - 50.0, y - 50.0);
    Scope::around(&[area], NEAR_M).unwrap().unwrap()
}

/// The unpacking of an archive that held what lies in `folder`.
fn holds(folder: &Path) -> impl Fn(&Path) -> Result<Vec<PathBuf>, LoadFailure> + '_ {
    move |_| {
        let mut files: Vec<PathBuf> = std::fs::read_dir(folder).unwrap().flatten().map(|entry| entry.path()).filter(|path| path.is_file()).collect();
        files.sort();
        Ok(files)
    }
}

fn near_sites() -> [(&'static str, &'static str, f64); 3] {
    [("A-1", "가상 고분군", 0.0), ("B-2", "가상 산성", 5050.0), ("C-3", "가상 사지", 9000.0)]
}

/// One archive holds several shapefiles (지정유산 holds six): each becomes a layer named by its file, with the
/// sites near the survey area; a file with nothing near is left out without a word (`:226-262`). When a
/// neighbour city is received too, its name follows the layer's (`:237-240`).
#[test]
fn every_shapefile_with_sites_near_becomes_a_layer_named_by_its_file() {
    let (packed, work) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    shapefile_of(packed.path(), "국가지정유산", CENTRE, &near_sites(), true);
    shapefile_of(packed.path(), "시도지정유산", CENTRE, &[("D-4", "가상 요지", 20_000.0)], true);
    let received = [PathBuf::from("지정유산_1.zip")];
    let scope = scope();

    let load = load_dataset(&DATASETS[0], &received, &holds(packed.path()), Some((&scope, 5187)), "", &work.path().join("own")).unwrap();
    assert_eq!(load.layers.len(), 1);
    let layer = &load.layers[0];
    assert_eq!((layer.title.as_str(), layer.sites, layer.kind, layer.named), ("국가지정유산", 2, ShapeKind::Polygons, true));
    assert_eq!(names_in(&layer.file), [("가상 고분군".to_owned(), "A-1".to_owned()), ("가상 산성".to_owned(), "B-2".to_owned())]);
    assert!(layer.file.starts_with(work.path().join("own")));
    assert_eq!((load.messages.len(), load.empty_in_scope), (0, false));

    let neighbour = load_dataset(&DATASETS[0], &received, &holds(packed.path()), Some((&scope, 5187)), " 가상시 ", &work.path().join("neighbour")).unwrap();
    assert_eq!(neighbour.layers[0].title, "국가지정유산 · 가상시");
}

/// A kind whose shapes all lie beyond 5 km is sound data with nothing to show (`:285-295`).
#[test]
fn a_kind_wholly_beyond_5_km_says_so() {
    let (packed, work) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    shapefile_of(packed.path(), "매장유산유존지역", CENTRE, &[("D-4", "가상 요지", 20_000.0)], true);
    let received = [PathBuf::from("매장유산유존지역_1.zip")];
    let scope = scope();

    let load = load_dataset(&DATASETS[2], &received, &holds(packed.path()), Some((&scope, 5187)), "", work.path()).unwrap();
    assert!(load.layers.is_empty() && load.empty_in_scope);
    assert_eq!(load.messages, ["매장유산유존지역: 조사구역 주변 5km 안에 든 자료가 없습니다."]);

    let neighbour = load_dataset(&DATASETS[2], &received, &holds(packed.path()), Some((&scope, 5187)), "가상시", work.path()).unwrap();
    assert_eq!(neighbour.messages, ["매장유산유존지역 (가상시): 조사구역 주변 5km 안에 든 자료가 없습니다."]);
}

/// A file without a column that could name a site is shown without names, and says so (`:264-268`).
#[test]
fn a_shapefile_without_a_name_column_says_so() {
    let (packed, work) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    shapefile_of(packed.path(), "지표유적위치도", CENTRE, &near_sites(), false);
    let scope = scope();

    let load = load_dataset(&DATASETS[4], &[PathBuf::from("지표조사구역_1.zip")], &holds(packed.path()), Some((&scope, 5187)), "", work.path()).unwrap();
    assert_eq!((load.layers[0].title.as_str(), load.layers[0].sites, load.layers[0].named), ("지표유적위치도", 2, false));
    assert_eq!(load.messages, ["지표유적위치도: 유적명 컬럼을 찾지 못해 범례에 이름을 넣지 못했습니다."]);
}

/// What stops a kind (`:168-225, 296-298`), and whether receiving it again could help.
#[test]
fn what_cannot_be_loaded_is_refused_with_its_reason() {
    let (packed, work) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let scope = scope();
    let received = [PathBuf::from("folder").join("지정유산_1.zip")];
    let refused = |received: &[PathBuf], unpack: &dyn Fn(&Path) -> Result<Vec<PathBuf>, LoadFailure>| {
        let failure = load_dataset(&DATASETS[0], received, unpack, Some((&scope, 5187)), "", work.path()).unwrap_err();
        (failure.reason, failure.download_again)
    };

    assert_eq!(refused(&[], &holds(packed.path())), ("받은 파일이 없습니다.".to_owned(), false));
    let broken = |_: &Path| Err(LoadFailure { reason: "ZIP 자료를 열지 못했습니다. 다운로드 완료 여부를 확인하세요.".to_owned(), download_again: true });
    assert_eq!(refused(&received, &broken), ("지정유산_1.zip: ZIP 자료를 열지 못했습니다. 다운로드 완료 여부를 확인하세요.".to_owned(), true));

    std::fs::write(packed.path().join("읽어보기.txt"), "text").unwrap();
    assert_eq!(refused(&received, &holds(packed.path())), ("지정유산_1.zip에서 SHP를 찾지 못했습니다. 자료를 다시 받아야 합니다.".to_owned(), true));

    // A shapefile with no shape in it is no reason to stop; a kind with nothing but such files is.
    shapefile_of(packed.path(), "시도등록문화유산", CENTRE, &[], true);
    assert_eq!(refused(&received, &holds(packed.path())), ("지정유산 자료를 지도에 올리지 못했습니다.".to_owned(), false));

    shapefile_of(packed.path(), "국가지정유산", CENTRE, &near_sites(), true);
    std::fs::remove_file(packed.path().join("국가지정유산.prj")).unwrap();
    assert_eq!(refused(&received, &holds(packed.path())), ("국가지정유산.shp의 필수 파일 .prj가 없습니다. 적재를 중단합니다.".to_owned(), true));
}
