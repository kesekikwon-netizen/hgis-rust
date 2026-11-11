//! 도형 고르기·카드 속성·삭제가 실제 조사 자료(C++ 견본 사본)에서 되는지 본다.

use std::path::PathBuf;

use strata_storage::survey_gpkg_read::read_survey_gpkg;
use strata_types::schema::DomainLayerKey as K;

use crate::app_state::StrataAppState;

fn opened_demo(tag: &str) -> (StrataAppState, PathBuf) {
    let dir = std::env::temp_dir().join(format!("strata_selection_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("demo.gpkg");
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    std::fs::copy(src, &path).expect("demo.gpkg 사본");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.home_map.viewport.meters_per_pixel = 0.1;
    (app, path)
}

#[test]
fn clicking_picks_the_shape_under_the_cursor_points_before_lines_before_polygons() {
    let (app, _) = opened_demo("pick");
    // 기준점 GCP1 은 (200000, 450000) 에 있다. 10 px = 1 m 안쪽이면 잡힌다.
    assert_eq!(app.pick_doc_feature([200_000.4, 450_000.3]), Some((K::ControlPoints, 0)));
    // 유물은 1호 주거지 안(200015, 450014)에 있다 — 면보다 점이 먼저다.
    assert_eq!(app.pick_doc_feature([200_015.2, 450_014.1]), Some((K::ArtifactPoint, 0)));
    // 단면선 A-A' 는 y=450014.5, x 200008~200024.
    assert_eq!(app.pick_doc_feature([200_020.0, 450_014.9]), Some((K::SectionLine, 0)));
    // 점·선에서 떨어진 주거지 안쪽은 면이 잡힌다.
    assert_eq!(app.pick_doc_feature([200_012.0, 450_018.0]).map(|h| h.0), Some(K::FeaturePoly));
    // 아무것도 없는 곳.
    assert_eq!(app.pick_doc_feature([200_100.0, 450_085.0]), None);
}

#[test]
fn hidden_layers_are_not_picked() {
    let (mut app, _) = opened_demo("hidden");
    app.home_map.real_survey.layer_visible.insert("기준점".to_owned(), false);
    assert_eq!(app.pick_doc_feature([200_000.4, 450_000.3]), None);
}

#[test]
fn the_card_shows_the_picked_shapes_record_and_edits_reach_the_file() {
    let (mut app, path) = opened_demo("card");
    let (key, idx) = app.pick_doc_feature([200_012.0, 450_018.0]).expect("주거지");
    app.select_doc_feature(key, idx);
    // 파일에 있던 값이 카드에 뜬다.
    assert_eq!(app.right_inspector.feature_no, "1호");
    assert_eq!(app.right_inspector.feature_period, "청동기시대");
    assert!(!app.is_dirty);
    assert!(app.selection_summary_lines().expect("요약").0.starts_with("면적: "));

    // 카드에서 고친다 → 저장 → 파일에 들어간다.
    app.right_inspector.feature_no = "7호".to_owned();
    app.right_inspector.feature_memo = "바닥 확인".to_owned();
    assert!(app.save_survey_project());
    let doc = read_survey_gpkg(&path).expect("표준 조사 파일");
    let saved = &doc.layer(K::FeaturePoly).features[idx];
    assert_eq!(saved.text("feature_no"), "7호");
    assert_eq!(saved.text("note"), "바닥 확인");
    assert_eq!(saved.text("period"), "청동기시대", "고치지 않은 값은 그대로다");
    // 다른 도형은 건드리지 않았다.
    let other = &doc.layer(K::FeaturePoly).features[1 - idx];
    assert_ne!(other.text("feature_no"), "7호");
}

#[test]
fn picking_another_shape_keeps_the_edit_made_on_the_first() {
    let (mut app, _) = opened_demo("switch");
    app.select_doc_feature(K::FeaturePoly, 0);
    app.right_inspector.feature_no = "A".to_owned();
    app.select_doc_feature(K::FeaturePoly, 1);
    assert_eq!(app.survey_doc.layer(K::FeaturePoly).features[0].text("feature_no"), "A");
    assert_ne!(app.right_inspector.feature_no, "A", "카드는 두 번째 도형의 값을 보인다");
    assert!(app.is_dirty);
}

#[test]
fn delete_removes_only_the_picked_shape_and_the_file_follows_after_save() {
    let (mut app, path) = opened_demo("delete");
    assert!(!app.delete_selected_feature(), "고른 것이 없으면 아무것도 지우지 않는다");
    app.select_doc_feature(K::ControlPoints, 0);
    assert!(app.delete_selected_feature());
    assert_eq!(app.survey_doc.layer(K::ControlPoints).features.len(), 1);
    assert_eq!(app.survey_doc.layer(K::ControlPoints).features[0].text("point_id"), "GCP2");
    assert!(app.selected_doc.is_none() && app.is_dirty);

    assert!(app.save_survey_project());
    let doc = read_survey_gpkg(&path).expect("표준 조사 파일");
    assert_eq!(doc.layer(K::ControlPoints).features.len(), 1);
    assert_eq!(doc.layer(K::FeaturePoly).features.len(), 2, "다른 레이어는 그대로다");
}
