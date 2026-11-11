//! 조사 문서에서 검수 상태를 실제로 계산한다.
//!
//! C++ `ProjectStateBuilder`·`ChecklistState*.cpp` 가 레이어를 훑어 만드는 값들이다.
//! 예전 Rust 판은 도형 유효성·필수 속성·번호 중복 등 15개쯤을 고정 `true` 로 넘겨서
//! 검수가 실제 자료와 상관없이 통과했다. 여기서는 `SurveyDoc` 의 도형·속성으로 계산한다.
//! 도면(용지) 관련 값은 호출하는 쪽이 더한다.

use std::collections::HashSet;

use geo::{Area, Contains, Intersects, LineString, MultiPolygon, Point, Polygon};
use serde_json::{json, Map, Value};

use strata_storage::survey_doc::{SurveyDoc, SurveyFeature};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::{DomainLayerKey as K, SurveySchemaV3};

fn line(ring: &[[f64; 2]]) -> LineString<f64> {
    LineString::from(ring.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>())
}

fn polygon(rings: &[Vec<[f64; 2]>]) -> Option<Polygon<f64>> {
    let (outer, holes) = rings.split_first()?;
    Some(Polygon::new(line(outer), holes.iter().map(|h| line(h)).collect()))
}

/// 두 선분이 끝점 말고 다른 곳에서 만나는지.
fn segments_cross(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2) = (orient(b[0], b[1], a[0]), orient(b[0], b[1], a[1]));
    let (d3, d4) = (orient(a[0], a[1], b[0]), orient(a[0], a[1], b[1]));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// 고리가 닫혀 있고 스스로 엇갈리지 않는지(자기교차 검사).
fn ring_is_simple(ring: &[[f64; 2]]) -> bool {
    if ring.len() < 4 || ring.first() != ring.last() {
        return false;
    }
    let segs: Vec<[[f64; 2]; 2]> = ring.windows(2).map(|w| [w[0], w[1]]).collect();
    for i in 0..segs.len() {
        for j in i + 2..segs.len() {
            if segments_cross(segs[i], segs[j]) {
                return false;
            }
        }
    }
    true
}

fn geom_valid(geom: &WkbGeometry) -> bool {
    match geom {
        WkbGeometry::Point(p) => p[0].is_finite() && p[1].is_finite(),
        WkbGeometry::LineString(pts) => pts.len() >= 2,
        WkbGeometry::Polygon(rings) => !rings.is_empty() && rings.iter().all(|r| ring_is_simple(r)),
    }
}

fn geom_nonempty(feature: &SurveyFeature) -> bool {
    match feature.geom.as_ref() {
        Some(WkbGeometry::Point(_)) => true,
        Some(WkbGeometry::LineString(pts)) => !pts.is_empty(),
        Some(WkbGeometry::Polygon(rings)) => rings.first().is_some_and(|r| !r.is_empty()),
        None => false,
    }
}

fn all_filled(features: &[SurveyFeature], column: &str) -> bool {
    features.iter().all(|f| !f.text(column).trim().is_empty())
}

fn unique_non_empty(features: &[SurveyFeature], column: &str) -> bool {
    let mut seen = HashSet::new();
    features
        .iter()
        .map(|f| f.text(column).trim().to_owned())
        .filter(|v| !v.is_empty())
        .all(|v| seen.insert(v))
}

/// 조사 문서의 검수 상태. 열쇠 이름은 `check_engine` 이 읽는 것과 같다.
#[must_use]
pub fn state_from_doc(doc: &SurveyDoc) -> Map<String, Value> {
    let layer = |k: K| &doc.layer(k).features;
    let every = || doc.layers.iter().flat_map(|l| l.features.iter());
    let gcp = layer(K::ControlPoints);
    let polys = layer(K::FeaturePoly);

    let survey_union = MultiPolygon::new(
        layer(K::SurveyArea)
            .iter()
            .filter_map(|f| match f.geom.as_ref()? {
                WkbGeometry::Polygon(rings) => polygon(rings),
                _ => None,
            })
            .collect(),
    );
    // 유구·유물이 조사구역 안(경계 포함)에 있는지. 조사구역이 없으면 판정할 수 없어 통과로 둔다.
    let within = |f: &SurveyFeature| -> bool {
        if survey_union.0.is_empty() {
            return true;
        }
        match f.geom.as_ref() {
            Some(WkbGeometry::Point(p)) => survey_union.intersects(&Point::new(p[0], p[1])),
            Some(WkbGeometry::LineString(pts)) => survey_union.contains(&line(pts)),
            Some(WkbGeometry::Polygon(rings)) => polygon(rings).is_some_and(|g| survey_union.contains(&g)),
            None => true,
        }
    };
    let features_within = [K::FeaturePoly, K::FeatureLine, K::ArtifactPoint]
        .into_iter()
        .flat_map(|k| layer(k).iter())
        .all(within);

    // 필수 속성(스키마의 required)이 모두 채워졌는지.
    let schema = SurveySchemaV3::default();
    let required_filled = doc.layers.iter().all(|l| {
        schema
            .layer(l.key)
            .fields
            .iter()
            .filter(|f| f.required)
            .all(|f| all_filled(&l.features, f.name))
    });

    let polygons_ok = |k: K| layer(k).iter().all(|f| matches!(f.geom, Some(WkbGeometry::Polygon(_))));
    let nonzero_area = [K::SurveyArea, K::FeaturePoly, K::TrialTrench]
        .into_iter()
        .flat_map(|k| layer(k).iter())
        .all(|f| match f.geom.as_ref() {
            Some(WkbGeometry::Polygon(rings)) => polygon(rings).is_some_and(|g| g.unsigned_area() > 0.0),
            _ => true,
        });

    let mut state = Map::new();
    for (key, value) in [
        ("survey_area_count", json!(layer(K::SurveyArea).len())),
        ("control_points_count", json!(gcp.len())),
        ("feature_poly_count", json!(polys.len())),
        ("feature_line_count", json!(layer(K::FeatureLine).len())),
        ("project_crs_set", json!(doc.srs_id > 0)),
        ("project_crs_authid", json!(format!("EPSG:{}", doc.srs_id))),
        ("has_datum", json!(all_filled(gcp, "datum"))),
        ("has_ellipsoid", json!(all_filled(gcp, "ellipsoid"))),
        ("has_projection", json!(all_filled(gcp, "projection"))),
        ("has_origin", json!(all_filled(gcp, "origin"))),
        (
            "has_accuracy",
            json!(gcp.iter().all(|f| !f.text("accuracy").trim().is_empty() || !f.text("accuracy_m").trim().is_empty())),
        ),
        ("has_kind_period", json!(all_filled(polys, "kind") && all_filled(polys, "period"))),
        ("has_abstract_marker", json!(false)),
        ("survey_is_polygon", json!(polygons_ok(K::SurveyArea))),
        ("features_within_survey", json!(features_within)),
        ("features_real_geometry", json!(polygons_ok(K::FeaturePoly) && layer(K::FeatureLine).iter().all(|f| matches!(f.geom, Some(WkbGeometry::LineString(_)))))),
        ("geometries_valid", json!(every().all(|f| f.geom.as_ref().is_none_or(geom_valid)))),
        ("geometries_nonempty", json!(every().all(geom_nonempty))),
        ("geometries_nonzero_area", json!(nonzero_area)),
        ("export_crs_valid", json!(matches!(doc.srs_id, 5186 | 5187 | 5179))),
        ("required_fields_filled", json!(required_filled)),
        ("control_points_point_id_unique", json!(unique_non_empty(gcp, "point_id"))),
        ("feature_poly_feature_no_unique", json!(unique_non_empty(polys, "feature_no"))),
    ] {
        state.insert(key.to_owned(), value);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check_engine::ChecklistEngine;
    use std::path::PathBuf;
    use strata_storage::survey_gpkg_read::read_survey_gpkg;

    /// 도면(용지)과 상관없는, 자료에서 나오는 규칙만 본다.
    const SHEET_RULES: [&str; 7] = [
        "REQ_DRAWING_SITE_LOCATION",
        "REQ_DRAWING_FEATURE_PLAN",
        "REQ_DRAWING_FEATURE_DETAIL",
        "SHEET_COVERS_TARGETS",
        "SHEET_ELEMENTS",
        "HERITAGE_PLEDGE_SCOPE",
        "REQ_DRAWING_SURVEY_AREA",
    ];

    fn failed_data_rules(sample: &str) -> Vec<String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples").join(sample);
        let doc = read_survey_gpkg(&path).unwrap_or_else(|_| SurveyDoc::new(0));
        let results = ChecklistEngine::with_default_rules().evaluate(&Value::Object(state_from_doc(&doc)));
        let mut failed: Vec<String> = results
            .into_iter()
            .filter(|r| !r.passed && !SHEET_RULES.contains(&r.id.as_str()))
            .map(|r| r.id)
            .collect();
        failed.sort();
        failed
    }

    #[test]
    fn the_demo_survey_has_no_data_findings() {
        // demo.ka-survey.json: "expected_data_errors": []
        assert_eq!(failed_data_rules("demo_survey/demo.gpkg"), Vec::<String>::new());
    }

    #[test]
    fn the_bad_survey_fails_exactly_the_rules_its_fixture_names() {
        // bad.ka-survey.json: 기준점 0개(GCP_MIN_TWO), 유구 종류·시대 빈칸(FEATURE_LEGEND_FIELDS).
        // 종류·시대는 스키마의 필수 속성이기도 해서 REQUIRED_FIELDS(warn)도 함께 걸린다.
        assert_eq!(
            failed_data_rules("bad_survey/bad.gpkg"),
            vec!["FEATURE_LEGEND_FIELDS".to_owned(), "GCP_MIN_TWO".to_owned(), "REQUIRED_FIELDS".to_owned()]
        );
    }

    #[test]
    fn a_bow_tie_polygon_and_a_duplicate_number_are_caught() {
        use strata_storage::survey_doc::SurveyFeature;
        let mut doc = SurveyDoc::new(5187);
        let bow_tie = vec![[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0], [0.0, 0.0]];
        let mut a = SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![bow_tie]));
        a.set_text("feature_no", "1호");
        let square = vec![[20.0, 0.0], [30.0, 0.0], [30.0, 10.0], [20.0, 10.0], [20.0, 0.0]];
        let mut b = SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![square]));
        b.set_text("feature_no", "1호");
        doc.add_feature(K::FeaturePoly, a);
        doc.add_feature(K::FeaturePoly, b);
        let state = state_from_doc(&doc);
        assert_eq!(state["geometries_valid"], json!(false));
        assert_eq!(state["feature_poly_feature_no_unique"], json!(false));
        assert_eq!(state["has_kind_period"], json!(false));
    }
}
