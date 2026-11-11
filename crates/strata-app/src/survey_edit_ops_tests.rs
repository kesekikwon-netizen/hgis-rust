//! 겹친 곳 지우기·폴리곤 묶기·되돌리기 시험. 넓이로 결과를 확인한다.

use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey as K;

use crate::app_state::StrataAppState;
use crate::real_survey::ring_area_m2;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
}

fn app_with(shapes: &[(K, Vec<[f64; 2]>)]) -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    for (key, ring) in shapes {
        app.draw_target = *key;
        assert!(app.commit_drawn_shape(MapToolKind::DrawPolygon, ring.clone()).is_some());
    }
    app.sync_view_into_survey_doc();
    app
}

/// 구멍을 뺀 넓이.
fn area(app: &StrataAppState, key: K, idx: usize) -> f64 {
    match app.survey_doc.layer(key).features[idx].geom.as_ref() {
        Some(WkbGeometry::Polygon(rings)) => {
            let outer = ring_area_m2(&rings[0]);
            outer - rings[1..].iter().map(|r| ring_area_m2(r)).sum::<f64>()
        }
        _ => 0.0,
    }
}

fn count(app: &StrataAppState, key: K) -> usize {
    app.survey_doc.layer(key).features.len()
}

fn message(app: &StrataAppState) -> &str {
    app.home_map.draw_guide_message.as_deref().unwrap_or("")
}

#[test]
fn erasing_with_one_picked_shape_cuts_a_hole_in_the_survey_area_and_removes_the_cutter() {
    // 조사구역 100×100 안에 제외 구역 20×20 을 유구 면으로 그렸다.
    let mut app = app_with(&[
        (K::SurveyArea, rect(0.0, 0.0, 100.0, 100.0)),
        (K::FeaturePoly, rect(40.0, 40.0, 20.0, 20.0)),
    ]);
    app.select_doc_feature(K::FeaturePoly, 0);
    assert!(app.erase_overlap_with_selection(), "{}", message(&app));

    assert_eq!(count(&app, K::FeaturePoly), 0, "위에 그린 면은 없어진다");
    assert_eq!(count(&app, K::SurveyArea), 1);
    assert!((area(&app, K::SurveyArea, 0) - 9_600.0).abs() < 1e-6, "10000 − 400");
    assert!(message(&app).contains("겹친 자리를 지웠습니다"));
    assert!(app.is_dirty);
    // 화면이 쓰는 조사구역 고리도 따라온다.
    assert_eq!(app.home_map.real_survey.survey_rings.len(), 1);
}

#[test]
fn erasing_across_a_shape_splits_it_into_two_features() {
    // 유구 30×10 을 가운데 세로 띠 10×20 으로 자르면 10×10 두 조각이 된다.
    let mut app = app_with(&[
        (K::FeaturePoly, rect(0.0, 0.0, 30.0, 10.0)),
        (K::FeaturePoly, rect(10.0, -5.0, 10.0, 20.0)),
    ]);
    app.survey_doc.layer_mut(K::FeaturePoly).features[0].set_text("kind", "구");
    app.select_doc_feature(K::FeaturePoly, 1);
    assert!(app.erase_overlap_with_selection(), "{}", message(&app));
    assert_eq!(count(&app, K::FeaturePoly), 2);
    assert!((area(&app, K::FeaturePoly, 0) - 100.0).abs() < 1e-6);
    assert!((area(&app, K::FeaturePoly, 1) - 100.0).abs() < 1e-6);
    // 갈라진 조각도 같은 기록을 가진다.
    assert_eq!(app.survey_doc.layer(K::FeaturePoly).features[1].text("kind"), "구");
    assert!(message(&app).contains("2개로 갈라졌습니다"));
}

#[test]
fn erasing_explains_why_when_it_cannot_and_changes_nothing() {
    // 조사구역 둘 위에 둘을 모두 덮는 큰 유구 면 하나.
    let mut app = app_with(&[
        (K::SurveyArea, rect(0.0, 0.0, 100.0, 100.0)),
        (K::SurveyArea, rect(150.0, 150.0, 100.0, 100.0)),
        (K::FeaturePoly, rect(-10.0, -10.0, 300.0, 300.0)),
    ]);
    let undo_depth = app.undo_stack.len();
    assert!(!app.erase_overlap_with_selection());
    assert_eq!(message(&app), "위에 그린 도형을 [도형선택]으로 고른 뒤 누르세요.");

    // 큰 면 하나만 골랐는데 아래에 겹친 면이 둘이다 → 함께 고르라고 알린다.
    app.select_doc_feature(K::FeaturePoly, 0);
    assert!(!app.erase_overlap_with_selection());
    assert!(message(&app).starts_with("아래에 겹친 도형이 2개입니다."), "{}", message(&app));
    assert_eq!((count(&app, K::SurveyArea), count(&app, K::FeaturePoly)), (2, 1));
    assert_eq!(app.undo_stack.len(), undo_depth, "실패한 편집은 기록에 남지 않는다");
}

#[test]
fn with_several_picked_the_largest_is_the_target_and_the_rest_are_cutters() {
    let mut app = app_with(&[
        (K::FeaturePoly, rect(0.0, 0.0, 50.0, 50.0)),
        (K::FeaturePoly, rect(0.0, 0.0, 10.0, 10.0)),
        (K::FeaturePoly, rect(40.0, 40.0, 10.0, 10.0)),
    ]);
    app.select_doc_feature(K::FeaturePoly, 1);
    app.extra_selected = vec![(K::FeaturePoly, 0), (K::FeaturePoly, 2)];
    assert!(app.erase_overlap_with_selection(), "{}", message(&app));
    assert_eq!(count(&app, K::FeaturePoly), 1);
    assert!((area(&app, K::FeaturePoly, 0) - 2_300.0).abs() < 1e-6, "2500 − 100 − 100");
}

#[test]
fn merging_joins_touching_shapes_keeps_the_first_record_and_refuses_separate_ones() {
    let mut app = app_with(&[
        (K::FeaturePoly, rect(0.0, 0.0, 10.0, 10.0)),
        (K::FeaturePoly, rect(10.0, 0.0, 10.0, 10.0)),
        (K::FeaturePoly, rect(100.0, 0.0, 10.0, 10.0)),
    ]);
    app.survey_doc.layer_mut(K::FeaturePoly).features[0].set_text("feature_no", "1호");
    app.survey_doc.layer_mut(K::FeaturePoly).features[1].set_text("feature_no", "2호");

    // 하나만 고르면 안내만 한다.
    app.select_doc_feature(K::FeaturePoly, 0);
    assert!(!app.merge_selected_polygons());
    assert!(message(&app).starts_with("묶을 면을 2개 이상 고른 뒤 누르세요."));

    // 떨어진 면은 묶지 않는다.
    app.extra_selected = vec![(K::FeaturePoly, 2)];
    assert!(!app.merge_selected_polygons());
    assert_eq!(count(&app, K::FeaturePoly), 3);

    // 맞닿은 두 면은 하나가 된다.
    app.select_doc_feature(K::FeaturePoly, 1);
    app.extra_selected = vec![(K::FeaturePoly, 0)];
    assert!(app.merge_selected_polygons(), "{}", message(&app));
    assert_eq!(count(&app, K::FeaturePoly), 2);
    assert!((area(&app, K::FeaturePoly, 0) - 200.0).abs() < 1e-6);
    assert_eq!(app.survey_doc.layer(K::FeaturePoly).features[0].text("feature_no"), "1호");
    assert_eq!(message(&app), "「유구_면」의 폴리곤 2개를 1개로 묶었습니다. Ctrl+Z로 되돌릴 수 있습니다.");
}

#[test]
fn undo_and_redo_walk_back_and_forth_through_edits_including_drawing_and_delete() {
    let mut app = app_with(&[
        (K::SurveyArea, rect(0.0, 0.0, 100.0, 100.0)),
        (K::FeaturePoly, rect(40.0, 40.0, 20.0, 20.0)),
    ]);
    app.select_doc_feature(K::FeaturePoly, 0);
    assert!(app.erase_overlap_with_selection());
    assert_eq!(count(&app, K::FeaturePoly), 0);

    assert!(app.undo_edit());
    assert_eq!(count(&app, K::FeaturePoly), 1, "지운 면이 돌아온다");
    assert!((area(&app, K::SurveyArea, 0) - 10_000.0).abs() < 1e-6, "구멍이 사라진다");

    assert!(app.redo_edit());
    assert_eq!(count(&app, K::FeaturePoly), 0);
    assert!((area(&app, K::SurveyArea, 0) - 9_600.0).abs() < 1e-6);

    // 그리기와 삭제도 되돌릴 수 있다.
    app.draw_target = K::FeaturePoly;
    let _ = app.commit_drawn_shape(MapToolKind::DrawPolygon, rect(1.0, 1.0, 2.0, 2.0));
    assert_eq!(count(&app, K::FeaturePoly), 1);
    app.select_doc_feature(K::FeaturePoly, 0);
    assert!(app.delete_selected_feature());
    assert_eq!(count(&app, K::FeaturePoly), 0);
    assert!(app.undo_edit());
    assert_eq!(count(&app, K::FeaturePoly), 1, "삭제를 되돌린다");
    assert!(app.undo_edit());
    assert_eq!(count(&app, K::FeaturePoly), 0, "그리기를 되돌린다");

    // 새 편집을 하면 다시하기 기록은 비워진다.
    let _ = app.commit_drawn_shape(MapToolKind::DrawPolygon, rect(5.0, 5.0, 2.0, 2.0));
    assert!(!app.redo_edit());
    assert_eq!(message(&app), "다시 할 편집이 없습니다.");
}
