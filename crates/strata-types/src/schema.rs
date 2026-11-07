use crate::crs::{SubmitCrs, WorkCrs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainLayerKey {
    SurveyArea,
    FeaturePoly,
    FeatureLine,
    SectionLine,
    ControlPoints,
    ArtifactPoint,
    TrialTrench,
}

impl DomainLayerKey {
    pub const ALL: [Self; 7] = [
        Self::SurveyArea,
        Self::FeaturePoly,
        Self::FeatureLine,
        Self::SectionLine,
        Self::ControlPoints,
        Self::ArtifactPoint,
        Self::TrialTrench,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::SurveyArea => "survey_area",
            Self::FeaturePoly => "feature_poly",
            Self::FeatureLine => "feature_line",
            Self::SectionLine => "section_line",
            Self::ControlPoints => "control_points",
            Self::ArtifactPoint => "artifact_point",
            Self::TrialTrench => "trial_trench",
        }
    }

    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::SurveyArea => "조사구역",
            Self::FeaturePoly => "유구_면",
            Self::FeatureLine => "유구_선",
            Self::SectionLine => "단면선",
            Self::ControlPoints => "기준점",
            Self::ArtifactPoint => "유물",
            Self::TrialTrench => "시굴격자",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegendGroup {
    SurveyData,
    Cadastral,
    ReferenceMap,
    Basemap,
}

impl LegendGroup {
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::SurveyData => "조사 데이터",
            Self::Cadastral => "지적도",
            Self::ReferenceMap => "주변·참조",
            Self::Basemap => "배경 지도",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryKind {
    Polygon,
    LineString,
    Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: &'static str,
    pub field_type: FieldType,
    pub required: bool,
}

impl FieldSpec {
    pub const fn str_field(name: &'static str, required: bool) -> Self {
        Self { name, field_type: FieldType::String, required }
    }
    pub const fn f64_field(name: &'static str, required: bool) -> Self {
        Self { name, field_type: FieldType::Double, required }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerSchema {
    pub key: DomainLayerKey,
    pub id: &'static str,
    pub geometry: GeometryKind,
    pub name_ko: &'static str,
    pub fields: Vec<FieldSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurveySchemaV3 {
    pub version: u32,
    pub default_work_crs: WorkCrs,
    pub work_crs_choices: Vec<WorkCrs>,
    pub submit_crs: SubmitCrs,
    pub working_format: &'static str,
    pub layers: Vec<LayerSchema>,
}

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
                    fields: vec![
                        s("name", false),
                        f("width", false),
                        f("length", false),
                    ],
                },
            ],
        }
    }
}

impl SurveySchemaV3 {
    pub fn layer(&self, key: DomainLayerKey) -> &LayerSchema {
        self.layers
            .iter()
            .find(|l| l.key == key)
            .unwrap_or(&self.layers[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_v3_matches_seven_domain_layers_and_yaml() {
        let schema = SurveySchemaV3::default();
        assert_eq!(schema.version, 3);
        assert_eq!(schema.default_work_crs, WorkCrs::Epsg5187);
        assert_eq!(schema.submit_crs, SubmitCrs::Epsg5179);
        assert_eq!(schema.layers.len(), 7);
        assert_eq!(schema.layer(DomainLayerKey::SurveyArea).name_ko, "조사구역");
        assert_eq!(schema.layer(DomainLayerKey::TrialTrench).name_ko, "시굴격자");
        assert_eq!(schema.layer(DomainLayerKey::FeaturePoly).fields.len(), 16);
        assert_eq!(schema.layer(DomainLayerKey::ControlPoints).fields.len(), 17);
    }
}
