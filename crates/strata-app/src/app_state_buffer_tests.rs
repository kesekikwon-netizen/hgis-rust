//! 주변유적경계 — 열어 둔 조사 표(저장 안 한 도형 포함)로 경계를 만든다. demo.gpkg 는 임시 사본만.

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

/// demo 조사구역(표 5186)을 작업 좌표계로 옮겨 500m 경계 하나·거리 글자 넷, 범례 맨 위에 「주변 500m 거리」·「주변 500m」.
#[test]
fn buffer_500m_rings_the_survey_area_and_adds_two_layers() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.run_site_buffer(500);
    assert_eq!(app.status_left_message(), "주변 500m 경계를 그렸습니다");
    let ring = app.home_map.real_survey.distance_rings.get(&500).expect("ring");
    assert_eq!((ring.lines.len(), ring.labels.len()), (1, 4));
    let top: Vec<&str> = app.left_layers.entries.iter().take(2).map(|e| e.label_ko.as_str()).collect();
    assert_eq!(top, vec!["주변 500m 거리", "주변 500m"]);
    app.run_site_buffer(500);
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key == "user:buffer_ring_500").count(), 1, "replaced, not doubled");
}

/// 그린(저장 안 한) 조사구역도 경계에 들어간다 — 3 km 떨어진 새 구역은 따로 두른다.
#[test]
fn buffer_includes_an_unsaved_survey_area() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_survey_area();
    for p in [[203_000.0, 450_000.0], [203_060.0, 450_000.0], [203_060.0, 450_060.0], [203_000.0, 450_060.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    app.run_site_buffer(1000);
    assert_eq!(app.home_map.real_survey.distance_rings.get(&1000).expect("ring").lines.len(), 2);
}

/// 면 레이어가 없으면 C++ 까닭으로 실패하고 아무 레이어도 올리지 않는다.
#[test]
fn buffer_without_a_polygon_layer_explains_why() {
    let mut app = StrataAppState::new_startup();
    app.run_site_buffer(500);
    assert_eq!(app.status_left_message(), "500m 경계를 그리지 못했습니다. 면 레이어를 먼저 선택하세요");
    assert!(app.left_layers.entries.iter().all(|e| !e.layer_key.starts_with("user:buffer_")));
}
