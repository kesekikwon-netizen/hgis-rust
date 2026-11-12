//! 조사 도형 그리기 네 입구 — C++ startEditSurveyArea·startEditFeaturePoly·startEditFeatureLine·
//! startEditSectionLine(`MainWindowEditing.cpp:1087-1166`)이 서로 다른 레이어를 그릴 대상으로 잡는지.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;

type Entry = fn(&mut StrataAppState);

fn opened_demo(dir: &Path) -> StrataAppState {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let copy = dir.join("demo.gpkg");
    std::fs::copy(src, &copy).expect("copy");
    let mut app = StrataAppState::new_startup();
    assert!(app.open_survey_gpkg(&copy));
    app
}

/// 네 입구는 각자 다른 표를 그릴 대상으로 잡고, 그 레이어를 범례의 현재 레이어로, 도구는 도형 형식대로,
/// 상태줄은 C++ beginEdit 의 마지막 글(「<레이어> — 좌클릭 점 · 우클릭/Enter 완료 · Esc 취소」).
#[test]
fn four_entries_target_four_layers() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let cases: [(Entry, &str, &str, MapToolKind); 4] = [
        (StrataAppState::start_edit_survey_area, "survey_area", "조사구역", MapToolKind::DrawPolygon),
        (StrataAppState::start_edit_feature_poly, "feature_poly", "유구 (면)", MapToolKind::DrawPolygon),
        (StrataAppState::start_edit_feature_line, "feature_line", "유구 (선)", MapToolKind::DrawLine),
        (StrataAppState::start_edit_section_line, "section_line", "단면선", MapToolKind::DrawLine),
    ];
    for (start, key, title, tool) in cases {
        start(&mut app);
        assert_eq!(app.edit_layer_key.as_deref(), Some(key));
        assert_eq!(app.right_inspector.selected_layer_key.as_deref(), Some(title));
        assert_eq!(app.active_tool, tool);
        assert_eq!(
            app.home_map.draw_guide_message.as_deref(),
            Some(format!("{title} — 좌클릭 점 · 우클릭/Enter 완료 · Esc 취소").as_str())
        );
        assert!(app.left_layers.entries.iter().any(|e| e.layer_key == key), "legend row");
        assert!(app.home_map.real_survey.is_layer_visible(title), "beginEdit shows the target layer");
    }
}

/// 범례에 없는 빈 표는 C++ ensureDomainLayer 처럼 그 이름(「유구선」)으로 조사 데이터 맨 위에 올린다.
#[test]
fn an_empty_table_is_added_with_the_cpp_title() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("새 조사.gpkg");
    let mut app = StrataAppState::new_startup();
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, app.modals.new_survey_crs).expect("create");
    app.start_new_survey_project_with_path("새 조사", &path, app.modals.new_survey_crs);
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key == "feature_line"));

    app.start_edit_feature_line();

    let first_survey_row = app
        .left_layers
        .entries
        .iter()
        .find(|e| e.group == strata_types::schema::LegendGroup::SurveyData)
        .expect("survey row");
    assert_eq!((first_survey_row.layer_key.as_str(), first_survey_row.label_ko.as_str()), ("feature_line", "유구선"));
    assert_eq!(app.right_inspector.selected_layer_key.as_deref(), Some("유구선"));
    assert_eq!(app.active_tool, MapToolKind::DrawLine);
}

/// 조사 파일이 없으면 그리지 않는다(C++ 은 「새 조사」를 물어본다 — 그 창은 아직 없다).
#[test]
fn without_a_survey_nothing_is_targeted() {
    let mut app = StrataAppState::new_startup();
    app.start_edit_feature_poly();
    assert_eq!(app.edit_layer_key, None);
    assert_eq!(app.active_tool, MapToolKind::Pan);
}

/// 다른 조사를 열면 이전 조사의 그리기 대상이 남지 않는다(C++ m_editLayer = nullptr).
#[test]
fn opening_another_survey_clears_the_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.start_edit_section_line();
    assert_eq!(app.edit_layer_key.as_deref(), Some("section_line"));
    let other = dir.path().join("다른.gpkg");
    std::fs::copy(dir.path().join("demo.gpkg"), &other).expect("copy");
    assert!(app.open_survey_gpkg(&other));
    assert_eq!(app.edit_layer_key, None);
}
