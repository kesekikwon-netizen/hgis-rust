//! 조사 면적 통계와 CSV 내보내기.
//!

pub use crate::trench_types::*;

pub const HGIS_SQM_TO_PYEONG_FACTOR: f64 = 0.3025;
pub const HGIS_PYEONG_TO_SQM_FACTOR: f64 = 3.305_785;

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchStatRow {
    pub trench_no: String,
    pub area_m2: f64,
    pub area_pyeong: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurveyAreaStats {
    pub survey_area_m2: f64,
    pub survey_area_pyeong: f64,
    pub trench_area_m2: f64,
    pub trench_area_pyeong: f64,
    pub trench_coverage_ratio_pct: f64,
    pub trench_count: usize,
    pub trench_rows: Vec<TrenchStatRow>,
}

/// C++ `KaSurveyStatisticsEngine::calculateSurveyStatistics` — 조사구역·트렌치 면적(㎡), 평수(`× 0.3025`) 및 시굴 비율(`trenchArea / surveyArea * 100.0`) 산출 (`surveyArea <= 1e-6` 시 0.0% 폴백)
#[must_use]
pub fn calculate_survey_statistics(
    survey_areas_m2: &[f64],
    trenches: &[(&str, f64)],
) -> SurveyAreaStats {
    let survey_area_m2: f64 = survey_areas_m2
        .iter()
        .copied()
        .filter(|v| *v > 0.0 && v.is_finite())
        .sum();
    let survey_area_pyeong = survey_area_m2 * HGIS_SQM_TO_PYEONG_FACTOR;
    let mut trench_area_m2 = 0.0;
    let mut trench_rows = Vec::with_capacity(trenches.len());
    for &(no, area) in trenches {
        let a = if area > 0.0 && area.is_finite() {
            area
        } else {
            0.0
        };
        trench_area_m2 += a;
        trench_rows.push(TrenchStatRow {
            trench_no: no.to_owned(),
            area_m2: a,
            area_pyeong: a * HGIS_SQM_TO_PYEONG_FACTOR,
        });
    }
    let trench_area_pyeong = trench_area_m2 * HGIS_SQM_TO_PYEONG_FACTOR;
    let trench_coverage_ratio_pct = if survey_area_m2 <= 1e-6 {
        0.0
    } else {
        (trench_area_m2 / survey_area_m2) * 100.0
    };
    SurveyAreaStats {
        survey_area_m2,
        survey_area_pyeong,
        trench_area_m2,
        trench_area_pyeong,
        trench_coverage_ratio_pct,
        trench_count: trench_rows.len(),
        trench_rows,
    }
}

/// C++ `KaAreaTableExporter::exportStatsToCsv` — UTF-8 BOM(`\xEF\xBB\xBF`) 프리픽스 포함 측량 통계 요약 CSV 바이트 생성
#[must_use]
pub fn export_survey_stats_to_csv_utf8_bom(stats: &SurveyAreaStats) -> Vec<u8> {
    let mut out = vec![0xEF_u8, 0xBB, 0xBF];
    let body = format!(
        "SurveyAreaM2,SurveyAreaPyeong,TrenchAreaM2,TrenchAreaPyeong,CoverageRatioPct,TrenchCount\n{:.2},{:.2},{:.2},{:.2},{:.2},{}\n",
        stats.survey_area_m2,
        stats.survey_area_pyeong,
        stats.trench_area_m2,
        stats.trench_area_pyeong,
        stats.trench_coverage_ratio_pct,
        stats.trench_count
    );
    out.extend_from_slice(body.as_bytes());
    out
}
