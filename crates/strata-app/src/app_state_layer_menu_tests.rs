//! 레이어 메뉴 동작 — 그 레이어(layer_key)에만, 메뉴의 체크가 따라오고, 지도 라벨이 바뀐다. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::survey_style::LabelRule;

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app
}

/// 표시할 항목(setField): 면적 식은 두고 항목만 → 「이름\n(면적 ㎡)」, 체크가 그 항목으로, 지도 라벨도. 같은 항목은 아무것도.
#[test]
fn label_field_keeps_the_area_and_moves_the_check() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    assert!(app.layer_label_field("feature_poly", "feature_no"));
    assert_eq!(app.effective_label_rule("feature_poly"), Some(LabelRule::NameWithArea("feature_no".to_owned())));
    let i = app.layer_menu_input("feature_poly").unwrap();
    assert_eq!((i.label.caption.as_str(), i.current_label_field.as_str()), ("이름·면적", "feature_no"));
    assert!(app.is_dirty);
    assert!(!app.layer_label_field("feature_poly", "feature_no"), "same field: nothing");
    let layers = crate::survey_paint_layers::build_paint_layers_with(&app.survey_tables, CrsCode::Epsg5187, &app.home_map.real_survey.label_views);
    let poly = layers.iter().find(|l| l.table == "feature_poly").unwrap();
    assert!(poly.features.iter().any(|f| f.label.as_deref().is_some_and(|t| t.starts_with("1호\n("))), "map label follows");
}

/// 면적(㎡) 함께 표시(setArea): 끄면 항목만, 다시 켜면 이름+면적. 다른 레이어는 그대로.
#[test]
fn label_area_toggles_on_that_layer_only() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.layer_label_field("feature_poly", "feature_no");
    assert!(app.layer_label_area("feature_poly"));
    assert_eq!(app.effective_label_rule("feature_poly"), Some(LabelRule::Field("feature_no".to_owned())));
    assert!(!app.layer_menu_input("feature_poly").unwrap().show_area);
    assert!(app.layer_label_area("feature_poly"));
    assert_eq!(app.effective_label_rule("feature_poly"), Some(LabelRule::NameWithArea("feature_no".to_owned())));
    assert!(!app.home_map.real_survey.label_views.contains_key("survey_area"));
}

/// 글자 크기·투명도: 그 레이어만, 체크가 따라온다.
#[test]
fn size_and_transparency_are_per_layer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    assert!(app.layer_label_size("feature_poly", 12.0));
    assert!(app.layer_set_transparency("feature_poly", 40));
    let i = app.layer_menu_input("feature_poly").unwrap();
    assert_eq!(i.label_size_pt, 12.0);
    assert!((i.opacity - 0.6).abs() < 1e-6);
    let other = app.layer_menu_input("survey_area").unwrap();
    assert_eq!((other.label_size_pt, other.opacity), (8.0, 1.0));
}

/// 같은 묶음의 맨 아래로: 묶음의 마지막 줄 다음으로, 이미 맨 아래면 아무것도.
#[test]
fn move_to_group_bottom() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let first = app.left_layers.entries.iter().find(|e| e.group == strata_types::schema::LegendGroup::SurveyData).unwrap().layer_key.clone();
    assert!(app.layer_to_group_bottom(&first));
    let survey: Vec<&str> = app.left_layers.entries.iter().filter(|e| e.group == strata_types::schema::LegendGroup::SurveyData).map(|e| e.layer_key.as_str()).collect();
    assert_eq!(survey.last(), Some(&first.as_str()));
    assert!(app.layer_menu_input(&first).unwrap().at_group_bottom);
    assert!(!app.layer_to_group_bottom(&first));
}

/// 이름 바꾸기: 창에 지금 이름, 「확인」이면 이름이 바뀌고 C++ 상태 글, 꺼 둔 표시는 새 이름으로 따라간다. 빈 이름은 알림.
#[test]
fn rename_keeps_the_layer_settings() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let old = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko.clone();
    app.home_map.real_survey.toggle_layer_visible(&old);
    app.open_rename_layer("feature_poly");
    assert_eq!(app.home_map.rename_layer.as_ref().unwrap().text, old);
    assert!(app.modal_window_open());
    app.home_map.rename_layer.as_mut().unwrap().text = "  유구 면 (북쪽)  ".to_owned();
    app.apply_rename_layer();
    assert_eq!(app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko, "유구 면 (북쪽)");
    assert_eq!(app.status_left_message(), format!("이름 변경: {old} → 유구 면 (북쪽)"));
    assert!(!app.home_map.real_survey.is_layer_visible("유구 면 (북쪽)"), "the hidden state follows the new name");
    app.open_rename_layer("feature_poly");
    app.home_map.rename_layer.as_mut().unwrap().text = "   ".to_owned();
    app.apply_rename_layer();
    assert!(app.status_left_message().starts_with("이름을 바꾸지 못했습니다"));
}
