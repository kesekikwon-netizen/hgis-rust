//! 7개 도메인 레이어 기본 스키마(`SurveySchemaV3::default`).
//!
//! `schema` 에서 분리했다. 값은 1:1 그대로다.

use crate::crs::{SubmitCrs, WorkCrs};
use crate::schema::{DomainLayerKey, FieldSpec, GeometryKind, LayerSchema, SurveySchemaV3};

impl Default for SurveySchemaV3 {
    fn default() -> Self {
        let s = FieldSpec::str_field;
        let f = FieldSpec::f64_field;
        Self {
            version: 3,
            default_work_crs: WorkCrs::Epsg5187,
            work_crs_choices: vec![WorkCrs::Epsg5186, WorkCrs::Epsg5187],
            submit_crs: SubmitCrs::Epsg5179,
            working_format: "gpkg",
            layers: vec![
                LayerSchema {
                    key: DomainLayerKey::SurveyArea,
                    id: "survey_area",
                    geometry: GeometryKind::Polygon,
                    name_ko: "조사구역",
                    fields: vec![
                        s("survey_name", true),
                        s("site_name", false),
                        s("note", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::FeaturePoly,
                    id: "feature_poly",
                    geometry: GeometryKind::Polygon,
                    name_ko: "유구_면",
                    fields: vec![
                        s("kind", true),
                        s("period", true),
                        s("feature_no", false),
                        s("note", false),
                        s("layer_no", false),
                        f("depth_m", false),
                        f("top_el", false),
                        f("bottom_el", false),
                        s("relation", false),
                        s("status", false),
                        s("photo", false),
                        s("surveyor", false),
                        s("surv_date", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::FeatureLine,
                    id: "feature_line",
                    geometry: GeometryKind::LineString,
                    name_ko: "유구_선",
                    fields: vec![
                        s("kind", true),
                        s("period", false),
                        s("note", false),
                        s("feature_no", false),
                        s("layer_no", false),
                        s("relation", false),
                        s("status", false),
                        s("photo", false),
                        s("surveyor", false),
                        s("surv_date", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::SectionLine,
                    id: "section_line",
                    geometry: GeometryKind::LineString,
                    name_ko: "단면선",
                    fields: vec![
                        s("section_id", true),
                        s("note", false),
                        s("surveyor", false),
                        s("surv_date", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::ControlPoints,
                    id: "control_points",
                    geometry: GeometryKind::Point,
                    name_ko: "기준점",
                    fields: vec![
                        s("point_id", true),
                        f("x", false),
                        f("y", false),
                        f("z", false),
                        s("datum", true),
                        s("ellipsoid", true),
                        s("projection", true),
                        s("origin", true),
                        s("accuracy", false),
                        f("accuracy_m", false),
                        f("pdop", false),
                        s("fix_type", false),
                        f("pixel_x", false),
                        f("pixel_y", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::ArtifactPoint,
                    id: "artifact_point",
                    geometry: GeometryKind::Point,
                    name_ko: "유물",
                    fields: vec![
                        s("kind", false),
                        s("period", false),
                        s("artifact_no", false),
                        s("note", false),
                        s("in_feature", false),
                        f("z", false),
                        s("layer_no", false),
                        s("photo", false),
                        s("surveyor", false),
                        s("surv_date", false),
                        s("uid", false),
                        s("created_at", false),
                        s("updated_at", false),
                    ],
                },
                LayerSchema {
                    key: DomainLayerKey::TrialTrench,
                    id: "trial_trench",
                    geometry: GeometryKind::Polygon,
                    name_ko: "시굴격자",
                    fields: vec![s("name", false), f("width", false), f("length", false)],
                },
            ],
        }
    }
}
