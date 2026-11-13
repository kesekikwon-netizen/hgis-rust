//! 조사 레이어 면적 — C++ showLayerAreaSummary(MainWindowContextMenus.cpp:450-481)·시굴격자 조사구역 고르기
//! (TrenchGridGenerator::pickAutoFillArea). demo.gpkg 는 임시 사본만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::{MainTab, StrataAppState};

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    app
}

fn label(app: &StrataAppState, key: &str) -> String {
    app.left_layers.entries.iter().find(|e| e.layer_key == key).expect(key).label_ko.clone()
}

/// 「면적 확인」: 레이어 이름, 도형 수, 합계 면적(천 단위 쉼표, 소수 둘). 시굴격자는 조사구역 대비 비율도.
#[test]
fn area_summary_follows_cpp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let app = opened_demo(dir.path());
    let area = label(&app, "survey_area");
    assert_eq!(app.layer_area_summary(&area, false).as_deref(), Some(format!("{area}\n도형 1개 · 합계 면적 12,000.00 ㎡").as_str()));
    let trench = label(&app, "trial_trench");
    assert_eq!(
        app.layer_area_summary(&trench, true).as_deref(),
        Some(
            format!("{trench}\n도형 1개 · 합계 면적 40.00 ㎡\n전체 조사구역 면적 합계 대비 0.33%\n(면이 겹치면 겹친 부분도 합계에 포함됩니다.)")
                .as_str()
        )
    );
}

/// 그린(저장 안 한) 조사구역도 합계에 들어간다(QGIS 편집 버퍼).
#[test]
fn area_summary_counts_an_unsaved_drawing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_survey_area();
    for p in [[200_300.0, 450_300.0], [200_400.0, 450_300.0], [200_400.0, 450_400.0], [200_300.0, 450_400.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    let area = label(&app, "survey_area");
    let text = app.layer_area_summary(&area, false).expect("summary");
    assert!(text.starts_with(&format!("{area}\n도형 2개 · 합계 면적 22,0")), "{text}");
}

/// 시굴격자 조사구역 면적 — 고른 조사구역, 없으면 fid 가 가장 큰 조사구역(C++ 은 저장 안 한 음수 fid 를 고르지 않는다).
#[test]
fn trench_survey_area_uses_the_selected_or_the_largest_fid() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    assert!((app.trench_survey_area_m2() - 12_000.0).abs() < 1e-6);
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_survey_area();
    for p in [[200_300.0, 450_300.0], [200_400.0, 450_300.0], [200_400.0, 450_400.0], [200_300.0, 450_400.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    assert!((app.trench_survey_area_m2() - 12_000.0).abs() < 1e-6, "unsaved fid -1 is not the largest fid");
    let new_fid = app.survey_tables.iter().find(|t| t.table == "survey_area").unwrap().layer.features.last().unwrap().fid;
    app.home_map.selected_shape = Some(("survey_area".to_owned(), new_fid));
    let a = app.trench_survey_area_m2();
    assert!((a - 10_000.0).abs() < 50.0, "the selected (drawn) area, about 100 m x 100 m: {a}");
}

fn square(fid: i64, x0: f64, y0: f64, x1: f64, y1: f64) -> strata_storage::edit_buffer::FeatureRecord {
    strata_storage::edit_buffer::FeatureRecord {
        fid,
        wkt_geom: format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))"),
        attributes: Default::default(),
    }
}

/// ported from tests/test_dem_trench.cpp:pickAutoFillArea_usesNewestNotAllUnion — 고른 것이 없으면 마지막(가장 큰 fid) 구역만.
#[test]
fn pick_auto_fill_area_uses_newest_not_all_union() {
    let feats = [square(1, 0.0, 0.0, 1000.0, 1000.0), square(2, 0.0, 0.0, 20.0, 20.0)];
    let a = super::pick_auto_fill_area_m2(&feats, &[]);
    assert!((a - 400.0).abs() < 1.0, "{a}");
}

/// ported from tests/test_dem_trench.cpp:pickAutoFillArea_usesSelectedOnly — 고른 구역만.
#[test]
fn pick_auto_fill_area_uses_selected_only() {
    let feats = [square(1, 0.0, 0.0, 100.0, 100.0), square(2, 200.0, 200.0, 220.0, 220.0)];
    let a = super::pick_auto_fill_area_m2(&feats, &[1]);
    assert!((a - 10_000.0).abs() < 1.0, "{a}");
}
