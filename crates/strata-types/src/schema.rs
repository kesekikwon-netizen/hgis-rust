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
        Self {
            name,
            field_type: FieldType::String,
            required,
        }
    }
    pub const fn f64_field(name: &'static str, required: bool) -> Self {
        Self {
            name,
            field_type: FieldType::Double,
            required,
        }
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

impl SurveySchemaV3 {
    pub fn layer(&self, key: DomainLayerKey) -> &LayerSchema {
        self.layers
            .iter()
            .find(|l| l.key == key)
            .unwrap_or(&self.layers[0])
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
