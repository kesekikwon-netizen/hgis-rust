//! 제출 꾸러미 — 조사 문서의 실제 도형을 EPSG:5179 Shapefile 로 내보낸다.
//!
//! C++ `ExportService.cpp:81-240`: 도형이 있는 도메인 레이어마다 `<레이어 키>.shp` 를
//! EPSG:5179 로 바꿔 쓰고 다시 열어 도형 수를 확인한 뒤, `encoding.txt`·(조판한 도면이 있을 때만)
//! `조사도면.pdf`·`README_submit.txt`·`MANIFEST.sha256` 를 쓴다. 검수 오류는 막지 않고
//! README 에 적는다. 도형이 하나도 없으면 꾸러미를 만들지 않는다.
//!
//! 예전 `submit_export::export_submission_package` 는 고정 사각형 글자를 쓴 가짜 SHP 를 만들었다.

use std::path::Path;

use strata_geo::crs_transform::CrsTransformer;
use strata_storage::shp_file_writer::{count_shp_records, write_shapefile, ShpField, ShpKind, ShpRecord};
use strata_storage::survey_doc::{FieldValue, SurveyDoc};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::{CrsCode, SubmitCrs};
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::{DomainLayerKey, FieldType, GeometryKind, SurveySchemaV3};

use crate::checklist_engine::{ChecklistFinding, ChecklistSeverity};
use crate::submit_manifest::{stream_file_sha256_hex, SubmissionManifest};

/// EPSG:5179 의 `.prj` 글자. GDAL(PROJ)이 ESRI WKT 로 내는 것과 같은 글자라서
/// QGIS·ArcGIS·GDAL 이 이 파일을 EPSG:5179 로 알아본다.
pub const EPSG_5179_ESRI_WKT: &str = r#"PROJCS["KGD2002_Unified_Coordinate_System",GEOGCS["GCS_KGD2002",DATUM["D_Korea_Geodetic_Datum_2002",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",1000000.0],PARAMETER["False_Northing",2000000.0],PARAMETER["Central_Meridian",127.5],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",38.0],UNIT["Meter",1.0]]"#;

/// C++ `ExportService.cpp:121-124` 의 레이어 순서.
const EXPORT_ORDER: [DomainLayerKey; 7] = [
    DomainLayerKey::SurveyArea,
    DomainLayerKey::FeaturePoly,
    DomainLayerKey::FeatureLine,
    DomainLayerKey::ControlPoints,
    DomainLayerKey::SectionLine,
    DomainLayerKey::ArtifactPoint,
    DomainLayerKey::TrialTrench,
];

fn source_crs(srs_id: i32) -> StrataResult<CrsCode> {
    match srs_id {
        5186 => Ok(CrsCode::Epsg5186),
        5187 => Ok(CrsCode::Epsg5187),
        5179 => Ok(CrsCode::Epsg5179),
        other => Err(StrataError::Storage(format!(
            "작업 좌표계 EPSG:{other} 는 EPSG:5179 로 바꿀 수 없습니다."
        ))),
    }
}

fn reproject(geom: &WkbGeometry, t: &CrsTransformer) -> WkbGeometry {
    let pt = |p: &[f64; 2]| t.transform_xy(p[0], p[1]);
    match geom {
        WkbGeometry::Point(p) => WkbGeometry::Point(pt(p)),
        WkbGeometry::LineString(pts) => WkbGeometry::LineString(pts.iter().map(pt).collect()),
        WkbGeometry::Polygon(rings) => {
            WkbGeometry::Polygon(rings.iter().map(|r| r.iter().map(pt).collect()).collect())
        }
    }
}

/// 꾸러미에 들어간 레이어 하나의 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedLayer {
    pub file_name: String,
    pub features: usize,
}

/// 제출 꾸러미를 `out_dir` 에 만든다. `sheet_pdf` 는 조판한 도면이 있을 때만 준다.
///
/// 돌려주는 값: 매니페스트와 레이어별 도형 수.
pub fn export_survey_package(
    out_dir: &Path,
    doc: &SurveyDoc,
    findings: &[ChecklistFinding],
    sheet_pdf: Option<&[u8]>,
) -> StrataResult<(SubmissionManifest, Vec<ExportedLayer>)> {
    if doc.total_features() == 0 {
        return Err(StrataError::Storage(
            "내보낼 조사 도형이 없습니다. 조사구역이나 유구를 그린 뒤 다시 만드세요.".into(),
        ));
    }
    let transformer = CrsTransformer::new(source_crs(doc.srs_id)?, CrsCode::Epsg5179);
    std::fs::create_dir_all(out_dir)?;
    let schema = SurveySchemaV3::default();
    let mut layers = Vec::new();
    let mut tracked: Vec<String> = Vec::new();

    for key in EXPORT_ORDER {
        let layer = doc.layer(key);
        let layer_schema = schema.layer(key);
        let kind = match layer_schema.geometry {
            GeometryKind::Polygon => ShpKind::Polygon,
            GeometryKind::LineString => ShpKind::PolyLine,
            GeometryKind::Point => ShpKind::Point,
        };
        let fields: Vec<ShpField> = layer_schema
            .fields
            .iter()
            .map(|f| ShpField {
                name: f.name.to_owned(),
                numeric: f.field_type == FieldType::Double,
            })
            .collect();
        let records: Vec<ShpRecord> = layer
            .features
            .iter()
            .filter_map(|f| {
                let geom = reproject(f.geom.as_ref()?, &transformer);
                let values = layer_schema
                    .fields
                    .iter()
                    .map(|fs| f.attrs.get(fs.name).cloned().unwrap_or(FieldValue::Null))
                    .collect();
                Some(ShpRecord { geom, values })
            })
            .collect();
        if records.is_empty() {
            continue;
        }
        let base = out_dir.join(key.id());
        write_shapefile(&base, kind, &fields, &records, EPSG_5179_ESRI_WKT)?;
        // 쓴 파일을 다시 열어 도형 수가 맞는지 본다(C++ `SubmitShp::verifyWritten`).
        let written = count_shp_records(&base.with_extension("shp"))?;
        if written != records.len() {
            return Err(StrataError::Storage(format!(
                "{}.shp 에 도형이 {written}개만 들어갔습니다({}개여야 합니다).",
                key.id(),
                records.len()
            )));
        }
        for ext in ["shp", "shx", "dbf", "prj", "cpg"] {
            tracked.push(format!("{}.{ext}", key.id()));
        }
        layers.push(ExportedLayer {
            file_name: format!("{}.shp", key.id()),
            features: records.len(),
        });
    }
    if layers.is_empty() {
        return Err(StrataError::Storage(
            "내보낼 조사 도형이 없습니다. 조사구역이나 유구를 그린 뒤 다시 만드세요.".into(),
        ));
    }

    std::fs::write(out_dir.join("encoding.txt"), "UTF-8\n")?;
    tracked.push("encoding.txt".to_owned());
    if let Some(pdf) = sheet_pdf {
        std::fs::write(out_dir.join("조사도면.pdf"), pdf)?;
        tracked.push("조사도면.pdf".to_owned());
    }

    let crs = SubmitCrs::Epsg5179;
    let mut readme = format!(
        "Strata 국가유산청 제출 패키지\n좌표계: {} (UTM-K / GRS80)\n글자 인코딩: UTF-8\n\n[내보낸 레이어]\n",
        crs.auth_id()
    );
    for l in &layers {
        readme.push_str(&format!("- {} features={} crs=EPSG:5179 verified\n", l.file_name, l.features));
    }
    readme.push_str(if sheet_pdf.is_some() {
        "\n[도면]\n- 조사도면.pdf\n"
    } else {
        "\n[도면]\n- 조사도면.pdf 없음 (조판한 도면이 없습니다)\n"
    });
    readme.push_str("\n[도면 점검 항목 기록]\n");
    for f in findings {
        let status = if f.passed {
            "통과"
        } else if f.severity == ChecklistSeverity::Error {
            "error"
        } else {
            "warn"
        };
        readme.push_str(&format!("- [{}] {}: {}\n", status, f.rule_id, f.message_ko));
    }
    let readme_path = out_dir.join("README_submit.txt");
    std::fs::write(&readme_path, readme.as_bytes())?;
    tracked.push("README_submit.txt".to_owned());

    let mut files_sha256 = Vec::with_capacity(tracked.len());
    let mut manifest_txt = String::new();
    for file_name in tracked {
        let digest_hex = stream_file_sha256_hex(&out_dir.join(&file_name))?;
        manifest_txt.push_str(&format!("{digest_hex}  {file_name}\n"));
        files_sha256.push((file_name, digest_hex));
    }
    std::fs::write(out_dir.join("MANIFEST.sha256"), manifest_txt.as_bytes())?;

    Ok((
        SubmissionManifest {
            crs,
            files_sha256,
            readme_path,
        },
        layers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use strata_storage::survey_gpkg_read::read_survey_gpkg;

    fn demo_doc() -> SurveyDoc {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
        read_survey_gpkg(&path).unwrap_or_else(|_| SurveyDoc::new(5186))
    }

    #[test]
    fn the_package_holds_every_non_empty_layer_as_a_real_5179_shapefile() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let (manifest, layers) = export_survey_package(dir.path(), &demo_doc(), &[], None)?;
        let got: Vec<(&str, usize)> = layers.iter().map(|l| (l.file_name.as_str(), l.features)).collect();
        assert_eq!(
            got,
            vec![
                ("survey_area.shp", 1),
                ("feature_poly.shp", 2),
                ("feature_line.shp", 1),
                ("control_points.shp", 2),
                ("section_line.shp", 1),
                ("artifact_point.shp", 1),
                ("trial_trench.shp", 1),
            ]
        );
        // 7 레이어 × 5 파일 + encoding.txt + README
        assert_eq!(manifest.files_sha256.len(), 37);
        assert!(!dir.path().join("조사도면.pdf").exists(), "조판한 도면이 없으면 PDF 도 없다");
        let readme = std::fs::read_to_string(dir.path().join("README_submit.txt"))?;
        assert!(readme.contains("feature_poly.shp features=2 crs=EPSG:5179 verified"));
        assert!(readme.contains("조사도면.pdf 없음"));

        // 기준점 GCP1 (5186: 200000, 450000) 이 5179 로 바뀌어 들어갔는지 .shp 바이트에서 읽는다.
        let shp = std::fs::read(dir.path().join("control_points.shp"))?;
        let f = |at: usize| f64::from_le_bytes(shp[at..at + 8].try_into().unwrap_or([0; 8]));
        let (x, y) = (f(112), f(120));
        // PROJ: EPSG:5186 (200000, 450000) → EPSG:5179 (955307.8296, 1850176.4017)
        assert!((x - 955_307.829_6).abs() < 0.001 && (y - 1_850_176.401_7).abs() < 0.001, "({x:.4}, {y:.4})");
        Ok(())
    }

    #[test]
    fn nothing_is_written_when_there_is_nothing_to_export() {
        let dir = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
        let out = dir.path().join("제출");
        let err = export_survey_package(&out, &SurveyDoc::new(5187), &[], None);
        assert!(err.is_err());
        assert!(!out.exists(), "빈 꾸러미 폴더를 남기지 않는다");
    }
}
