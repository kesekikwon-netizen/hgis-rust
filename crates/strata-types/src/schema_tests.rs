use crate::crs::{SubmitCrs, WorkCrs};
use crate::schema::{DomainLayerKey, SurveySchemaV3};

#[test]
fn schema_v3_matches_seven_domain_layers_and_yaml() {
    let schema = SurveySchemaV3::default();
    assert_eq!(schema.version, 3);
    assert_eq!(schema.default_work_crs, WorkCrs::Epsg5187);
    assert_eq!(schema.submit_crs, SubmitCrs::Epsg5179);
    assert_eq!(schema.layers.len(), 7);
    assert_eq!(schema.layer(DomainLayerKey::SurveyArea).name_ko, "조사구역");
    assert_eq!(
        schema.layer(DomainLayerKey::TrialTrench).name_ko,
        "시굴격자"
    );
    assert_eq!(schema.layer(DomainLayerKey::FeaturePoly).fields.len(), 16);
    assert_eq!(schema.layer(DomainLayerKey::ControlPoints).fields.len(), 17);
}
