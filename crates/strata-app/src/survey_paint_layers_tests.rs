//! 조사 표 → 지도에 그릴 레이어: 면적·라벨·자리·좌표 변환·그리는 차례.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_engine::Transformer;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;
use strata_types::schema::LegendGroup;

use super::*;
use crate::left_layer_panel::LayerTreeEntry;

fn demo_tables() -> Vec<strata_storage::survey_gpkg_read::SurveyFileTable> {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path)
        .expect("copy demo");
    strata_storage::survey_gpkg_read::read_survey_tables(&path).expect("read demo")
}

/// QGIS `area($geometry)` — 표의 좌표계에서 잰 평면 면적, 구멍은 뺀다.
#[test]
fn planar_area_subtracts_holes() {
    let ring = |pts: &[[f64; 2]]| pts.to_vec();
    let outer = ring(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]);
    let hole = ring(&[[2.0, 2.0], [2.0, 4.0], [4.0, 4.0], [4.0, 2.0], [2.0, 2.0]]);
    assert!((planar_area(&[WkbGeometry::Polygon(vec![outer, hole])]) - 96.0).abs() < 1e-9);
}

fn entry(key: &str, group: LegendGroup) -> LayerTreeEntry {
    LayerTreeEntry {
        layer_key: key.to_owned(),
        label_ko: String::new(),
        group,
        visible: true,
        opacity: 1.0,
        read_only: false,
        snap_enabled: true,
        feature_count: 1,
    }
}

/// 범례 맨 위 줄이 맨 위에 그려진다 — 아래 줄부터 그린다. 범례에 없는 표는 그리지 않는다.
#[test]
fn paint_order_is_legend_order_bottom_first() {
    let layers = build_paint_layers(&demo_tables(), CrsCode::Epsg5187);
    let entries = vec![
        entry("trial_trench", LegendGroup::SurveyData),
        entry("artifact_point", LegendGroup::SurveyData),
        entry("survey_area", LegendGroup::SurveyData),
        entry("vworld_satellite", LegendGroup::Basemap),
    ];
    let order: Vec<&str> = paint_order(&entries, &layers).into_iter().map(|i| layers[i].table.as_str()).collect();
    assert_eq!(order, vec!["survey_area", "artifact_point", "trial_trench"]);
}

/// demo.gpkg(5186)를 작업 좌표계 5187 로: 라벨 글은 C++ 식, 면 라벨은 무게중심, 선 라벨은 선 길이의 가운데.
#[test]
fn demo_tables_become_labelled_layers_in_the_work_crs() {
    let layers = build_paint_layers(&demo_tables(), CrsCode::Epsg5187);
    // 작업 좌표계 값은 C++ 과 같은 PROJ(strata-engine)로 — gdaltransform 과 1 mm 안.
    let proj = Transformer::new(5186, 5187).expect("PROJ 5186 -> 5187");
    let to5187 = |x: f64, y: f64| proj.transform([x, y]).expect("transform");
    let layer = |t: &str| layers.iter().find(|l| l.table == t).expect(t);
    let close = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-6;

    let area = &layer("survey_area").features[0];
    assert_eq!(area.label.as_deref(), Some("12,000.00 ㎡"));
    assert!(close(area.anchor.unwrap().xy, to5187(200_050.0, 450_040.0)));
    assert_eq!(layer("survey_area").title, "조사구역");

    let trench = &layer("trial_trench").features[0];
    assert_eq!(trench.label.as_deref(), Some("TR1\n(40.0 ㎡)"));

    let section = &layer("section_line").features[0];
    assert_eq!(section.label.as_deref(), Some("A-A'"));
    assert!(close(section.anchor.unwrap().xy, to5187(200_016.0, 450_014.5)));

    let gcp: Vec<&str> = layer("control_points").features.iter().filter_map(|f| f.label.as_deref()).collect();
    assert_eq!(gcp, vec!["GCP1", "GCP2"]);
    assert_eq!(layer("artifact_point").features[0].label.as_deref(), Some("토기편"));
    assert_eq!(layer("feature_line").features[0].label.as_deref(), Some("구"));

    let PaintPart::Polygon(rings) = &area.parts[0] else { panic!("polygon") };
    assert!(close(rings[0][0], to5187(199_990.0, 449_990.0)));
}

/// QGIS 처럼 PROJ 가 아는 좌표계면 옛 측지계(Bessel EPSG:5174)도 그린다 — 5174 → 5186 은 gdaltransform
/// 200000,450000 → 200069.633100671,550305.22053531. PROJ 가 모르는 번호의 표는 그리지 않는다.
#[test]
fn bessel_5174_table_is_painted_at_the_proj_position_and_unknown_srs_is_skipped() {
    let mut tables = demo_tables();
    tables.retain(|t| t.table == "control_points");
    tables[0].srs_id = 5174;
    for f in &mut tables[0].layer.features {
        f.wkt_geom = "POINT (200000 450000)".to_owned();
    }
    let mut unknown = tables[0].clone();
    unknown.table = "artifact_point".to_owned();
    unknown.srs_id = 999_999;
    tables.push(unknown);

    let layers = build_paint_layers(&tables, CrsCode::Epsg5186);
    assert_eq!(layers.len(), 1, "unknown srs table is skipped");
    let PaintPart::Point(p) = layers[0].features[0].parts[0] else { panic!("point") };
    assert!((p[0] - 200_069.633_100_671).hypot(p[1] - 550_305.220_535_31) <= 0.001, "{p:?}");
}

/// 선 라벨의 기울기는 글 폭만큼 걸친 구간의 양 끝으로 잰다 — 그 끝 점을 선을 따라 찾는다.
#[test]
fn point_along_walks_the_line_and_clamps_to_its_ends() {
    let l = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    assert_eq!(point_along(&l, 5.0), Some([5.0, 0.0]));
    assert_eq!(point_along(&l, 15.0), Some([10.0, 5.0]));
    assert_eq!(point_along(&l, -3.0), Some([0.0, 0.0]));
    assert_eq!(point_along(&l, 99.0), Some([10.0, 10.0]));
    assert!((line_length(&l) - 20.0).abs() < 1e-12);
}
