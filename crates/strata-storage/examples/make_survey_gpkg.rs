//! 검증용: Rust 가 만든/저장한 조사 파일을 폴더에 남긴다. 다른 GIS 도구(GDAL, QGIS, C++ 앱)로
//! 열어 보려고 쓴다.
//!
//! `cargo run -p strata-storage --example make_survey_gpkg -- <출력 폴더> <C++ demo.gpkg>`

use std::path::PathBuf;

use strata_storage::survey_doc::SurveyFeature;
use strata_storage::survey_gpkg_read::read_survey_gpkg;
use strata_storage::survey_gpkg_write::{create_standard_survey, save_survey_gpkg};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey as K;

fn main() -> StrataResult<()> {
    let mut args = std::env::args().skip(1);
    let (Some(out), Some(demo)) = (args.next(), args.next()) else {
        return Err(StrataError::Storage("인자: <출력 폴더> <demo.gpkg>".into()));
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out)?;

    // 1) Rust 가 처음부터 만든 조사.
    let fresh = out.join("rust_new.gpkg");
    let _ = std::fs::remove_file(&fresh);
    let mut doc = create_standard_survey(&fresh, 5187)?;
    let mut area = SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![vec![
        [219_700.0, 360_900.0],
        [219_800.0, 360_900.0],
        [219_800.0, 361_000.0],
        [219_700.0, 361_000.0],
        [219_700.0, 360_900.0],
    ]]));
    area.set_text("survey_name", "러스트 조사");
    doc.add_feature(K::SurveyArea, area);
    let mut line = SurveyFeature::new_drawn(WkbGeometry::LineString(vec![
        [219_710.0, 360_910.0],
        [219_760.0, 360_950.0],
    ]));
    line.set_text("kind", "구");
    doc.add_feature(K::FeatureLine, line);
    let mut gcp = SurveyFeature::new_drawn(WkbGeometry::Point([219_705.0, 360_905.0]));
    gcp.set_text("point_id", "GCP-1");
    doc.add_feature(K::ControlPoints, gcp);
    save_survey_gpkg(&fresh, &mut doc)?;

    // 2) C++ 쪽 파일의 사본에 Rust 가 도형을 더해 저장한 것.
    let edited = out.join("cpp_demo_saved_by_rust.gpkg");
    std::fs::copy(&demo, &edited)?;
    let mut doc = read_survey_gpkg(&edited)?;
    let mut pit = SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![vec![
        [200_050.0, 450_050.0],
        [200_055.0, 450_050.0],
        [200_055.0, 450_055.0],
        [200_050.0, 450_055.0],
        [200_050.0, 450_050.0],
    ]]));
    pit.set_text("kind", "수혈");
    doc.add_feature(K::FeaturePoly, pit);
    save_survey_gpkg(&edited, &mut doc)?;
    println!("{}\n{}", fresh.display(), edited.display());
    Ok(())
}
