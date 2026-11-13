//! 레이어 메뉴 입력 — demo 임시 사본의 범례 줄에서.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::StrataAppState;
use crate::layer_menu_model::{LayerMenuKind, MenuGeom};

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app
}

/// 유구 면: Feature 가지, 우리 조사 파일, fid 가 맨 앞 항목, 면적 라벨(「면적」·면적 함께 표시), 글자 8 pt.
#[test]
fn survey_polygon_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let app = opened_demo(dir.path());
    let i = app.layer_menu_input("feature_poly").expect("entry");
    assert_eq!((i.kind, i.geom), (LayerMenuKind::Feature, Some(MenuGeom::Polygon)));
    assert!(i.vector && i.own_survey && !i.reference_or_cadastral && i.feature_count > 0);
    assert_eq!(i.fields.first().map(String::as_str), Some("fid"));
    assert_eq!(i.label.caption, "면적");
    assert!(i.label.area_editable && i.show_area && i.has_labeling);
    assert_eq!(i.label_size_pt, 8.0);
}

/// 위성: 참조 래스터, xyz(오프라인 저장 줄), 배경 묶음의 맨 아래.
#[test]
fn satellite_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let app = opened_demo(dir.path());
    let key = app.left_layers.entries.iter().find(|e| e.group == strata_types::schema::LegendGroup::Basemap).map(|e| e.layer_key.clone()).expect("basemap row");
    let i = app.layer_menu_input(&key).expect("entry");
    assert_eq!((i.kind, i.geom), (LayerMenuKind::Reference, None));
    assert!(!i.vector && i.xyz && i.at_group_bottom);
    assert!(app.layer_menu_input("no_such_layer").is_none());
}
