//! The 「유산」 button with data received earlier: the legend rows of a kind, what stops a run before it
//! starts, and what the gauge and the status line say (`target/cpp-ref/src/app/MainWindow.cpp:3071-3225`,
//! `core/HeritageStyle.cpp`). No received data is used: the layers are named here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use strata_engine::ShapeKind;
use strata_refmap::heritage::DATASETS;
use strata_refmap::heritage_load::{DatasetLoad, LoadFailure, Loaded};
use strata_types::schema::LegendGroup;

use super::*;
use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::ref_vector::data::{LabelLook, Look, PolygonFill};

fn layer(title: &str, sites: u64, named: bool) -> Loaded {
    Loaded { title: title.to_owned(), file: PathBuf::from(format!("C:/data/주변유적/SHP/UTF8/strata-1/{title}.gpkg")), sites, kind: ShapeKind::Polygons, named }
}

fn last_state(app: &mut StrataAppState) -> DownloadJobState {
    app.download_jobs.tick(1.0);
    app.download_jobs.jobs().last().expect("a job").state.clone()
}

/// Every shapefile of a kind is a read-only row under 참조 지도, named by the file: an outline of 0.35 mm in
/// the colour of the kind, never filled (`HeritageStyle.cpp:26-57, 134`); the names of the sites in bold 7 pt
/// with a white edge, only closer than 1:10,000 (`:136, 174-180`, `LabelOps.cpp:30-44`). A layer without
/// names has no labels.
#[test]
fn the_layers_of_a_kind_become_rows_in_its_colour() {
    let mut app = StrataAppState::new_startup();
    app.show_heritage_layers(&DATASETS[0], &[layer("국가지정유산", 72, true), layer("시도지정유산", 164, false)]);
    app.show_heritage_layers(&DATASETS[2], &[layer("매장유산유존지역", 185, true)]);

    let rows: Vec<(&str, &str, LegendGroup, bool)> = app.left_layers.entries.iter().filter(|e| e.layer_key.starts_with(KEY_PREFIX)).map(|e| (e.layer_key.as_str(), e.label_ko.as_str(), e.group, e.read_only)).collect();
    assert_eq!(
        rows,
        [
            ("ref_heritage:designated_heritage:국가지정유산", "국가지정유산", LegendGroup::ReferenceMap, true),
            ("ref_heritage:designated_heritage:시도지정유산", "시도지정유산", LegendGroup::ReferenceMap, true),
            ("ref_heritage:buried_heritage_area:매장유산유존지역", "매장유산유존지역", LegendGroup::ReferenceMap, true),
        ]
    );
    let named = app.ref_vectors.spec_of("ref_heritage:designated_heritage:국가지정유산").expect("layer");
    assert_eq!((named.table.as_str(), named.work_epsg), ("heritage", 5187));
    assert_eq!(named.look, Look::Polygons { stroke: [0x8E, 0x44, 0xAD, 255], width_mm: 0.35, fill: PolygonFill::None });
    assert_eq!(named.label, Some(LabelLook { column: "ka_name", size_pt: 7.0, ink: [31, 35, 40], halo_mm: 0.8, halo: [255, 255, 255, 230], bold: true, closest_scale: Some(10_000.0), inside_only: false }));
    assert_eq!(app.ref_vectors.spec_of("ref_heritage:designated_heritage:시도지정유산").expect("layer").label, None);
    let buried = app.ref_vectors.spec_of("ref_heritage:buried_heritage_area:매장유산유존지역").expect("layer");
    assert_eq!(buried.look, Look::Polygons { stroke: [0x2E, 0x86, 0xC1, 255], width_mm: 0.35, fill: PolygonFill::None });
    assert_eq!(colour_of_key("ref_heritage:buried_heritage_area:매장유산유존지역"), Some([0x2E, 0x86, 0xC1]));
    assert_eq!(colour_of_key("ref_topo"), None);

    // The rows of an earlier run do not stay under those of the next.
    app.remove_heritage_rows();
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key.starts_with(KEY_PREFIX)));
    assert!(app.ref_vectors.spec_of("ref_heritage:designated_heritage:국가지정유산").is_none());
}

/// The press is stopped before the dialog, with the reason of `MainWindow.cpp:3082-3099`, on the gauge.
#[test]
fn a_press_without_a_survey_area_stops_with_the_reason() {
    let mut app = StrataAppState::new_startup();
    app.press_heritage();
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("조사구역 레이어가 없습니다. 조사구역을 먼저 그리세요."), "said at once");
    app.drive_heritage_wishes(0.0);
    assert_eq!(app.modals.active, None, "no dialog");
    assert_eq!(last_state(&mut app), DownloadJobState::Stopped { code: StopCode::Area1, reason: "조사구역 레이어가 없습니다. 조사구역을 먼저 그리세요.".to_owned() });
    assert_eq!(app.download_jobs.jobs().last().expect("job").title, "주변유적");
}

/// What the kinds of a run come to: each kind says how many sites it put on the map (`:3220`), with what
/// the loading had to say; the end closes the gauge with the words of `:3143`, and names the kinds that
/// were not there to use again.
#[test]
fn each_kind_is_said_and_the_end_closes_the_gauge() {
    let mut app = StrataAppState::new_startup();
    let (_, job) = app.download_jobs.add_job("주변유적", DownloadJobState::Running { fraction: None, stage: None }, 0.0);

    let load = DatasetLoad { layers: vec![layer("국가지정유산", 72, true), layer("시도지정유산", 164, true)], messages: vec!["시도지정유산: 유적명 컬럼을 찾지 못해 범례에 이름을 넣지 못했습니다.".to_owned()], empty_in_scope: false };
    assert!(!app.take_heritage_step(&job, Step::Kind(&DATASETS[0], Ok(load))));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("지정유산 236곳을 올렸습니다. 시도지정유산: 유적명 컬럼을 찾지 못해 범례에 이름을 넣지 못했습니다."));
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key.starts_with(KEY_PREFIX)).count(), 2);

    let beyond = DatasetLoad { layers: Vec::new(), messages: vec!["발굴조사구역: 조사구역 주변 5km 안에 든 자료가 없습니다.".to_owned()], empty_in_scope: true };
    assert!(!app.take_heritage_step(&job, Step::Kind(&DATASETS[5], Ok(beyond))));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("발굴조사구역: 조사구역 주변 5km 안에 든 자료가 없습니다."));

    assert!(app.take_heritage_step(&job, Step::End { missing: Vec::new() }));
    assert_eq!(last_state(&mut app), DownloadJobState::Done { result: "주변유적 자료 처리를 마쳤습니다. 받은 자료는 「참조 지도」에 있습니다.".to_owned() });

    let (_, job) = app.download_jobs.add_job("주변유적", DownloadJobState::Running { fraction: None, stage: None }, 0.0);
    assert!(app.take_heritage_step(&job, Step::End { missing: vec!["지표조사구역", "발굴조사구역"] }));
    let DownloadJobState::Done { result } = last_state(&mut app) else { panic!("done") };
    assert!(result.starts_with("주변유적 자료 처리를 마쳤습니다. 받은 자료는 「참조 지도」에 있습니다. 받아 둔 자료가 없는 종류: 지표조사구역, 발굴조사구역"), "{result}");
}

/// A kind that cannot be loaded stops the run with its reason (`fail()`, `KaHeritageBrowser.cpp:445-477`);
/// what is on the map already stays.
#[test]
fn a_kind_that_cannot_be_loaded_stops_the_run() {
    let mut app = StrataAppState::new_startup();
    let (_, job) = app.download_jobs.add_job("주변유적", DownloadJobState::Running { fraction: None, stage: None }, 0.0);
    app.show_heritage_layers(&DATASETS[0], &[layer("국가지정유산", 72, true)]);
    let failure = LoadFailure { reason: "매장유산유존지역_1.zip에서 SHP를 찾지 못했습니다. 자료를 다시 받아야 합니다.".to_owned(), download_again: true };
    assert!(app.take_heritage_step(&job, Step::Kind(&DATASETS[2], Err(failure))));
    assert_eq!(last_state(&mut app), DownloadJobState::Stopped { code: StopCode::Data1, reason: "매장유산유존지역_1.zip에서 SHP를 찾지 못했습니다. 자료를 다시 받아야 합니다.".to_owned() });
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "ref_heritage:designated_heritage:국가지정유산"));
}
