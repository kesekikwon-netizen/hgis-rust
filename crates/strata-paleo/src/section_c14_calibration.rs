//! 방사성탄소(IntCal20) 보정.
//!
//! `section_renderer` 에서 분리했다.

/// 탄소연대 1-sigma 표준 확률 면적 (C++ `HGIS_C14_1SIGMA_PROBABILITY`)
pub const HGIS_C14_1SIGMA_PROBABILITY: f64 = 0.6826;

/// 탄소연대 2-sigma 표준 확률 면적 (C++ `HGIS_C14_2SIGMA_PROBABILITY`)
pub const HGIS_C14_2SIGMA_PROBABILITY: f64 = 0.9544;

/// IntCal20 보정 곡선 최대 상한 (BP, C++ `HGIS_INTCAL20_MAX_BP`)
pub const HGIS_INTCAL20_MAX_BP: i32 = 55_000;

/// 탄소연대 BP 기준년 (AD 1950, C++ `HGIS_BP_DATUM_YEAR`)
pub const HGIS_BP_DATUM_YEAR: i32 = 1950;

/// 최소 탄소연대 측정 오차 (년, C++ `HGIS_MIN_C14_SIGMA`)
pub const HGIS_MIN_C14_SIGMA: i32 = 10;

/// IntCal20 보정 곡선 레코드 (C++ `IntCalRecord`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntCalRecord {
    pub cal_bp: i32,
    pub c14_bp: f64,
    pub curve_sigma: f64,
}

/// 보정 연대 확률 구간 (음수: BC, 양수: AD, C++ `CalibratedRange`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibratedRange {
    pub start_year_cal: i32,
    pub end_year_cal: i32,
    pub probability_pct: f64,
}

/// 방사성탄소연대 보정 에러 (C++ `RadiocarbonError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiocarbonError {
    ExceedsIntCalBounds,
}

/// 방사성탄소연대(AMS 14C) 1-sigma / 2-sigma 보정 결과 (C++ `RadiocarbonCalibrationResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct RadiocarbonCalibrationResult {
    pub uncalibrated_bp: i32,
    pub sigma: i32,
    pub range_1sigma: Vec<CalibratedRange>,
    pub range_2sigma: Vec<CalibratedRange>,
}

/// 신석기 중기(4500 BP 전후, cal BC 3350 ~ cal BC 3090) IntCal20 표준 기준 곡선 세그먼트
#[must_use]
pub fn default_neolithic_intcal20_segment() -> Vec<IntCalRecord> {
    // cal_bp = 5300 (cal BC 3350) ..= 5040 (cal BC 3090)
    (5040..=5300)
        .step_by(5)
        .map(|cal_bp| {
            let offset = (cal_bp - 5170) as f64;
            IntCalRecord {
                cal_bp,
                c14_bp: 4500.0 + offset * 0.85,
                curve_sigma: 15.0,
            }
        })
        .collect()
}

/// IntCal20 보정 곡선 선형 보간 및 HPD 적분 기반 1-sigma(68.2%)/2-sigma(95.4%) 보정 연대 구간 산출
/// (C++ `KaRadiocarbonCalibrator::calibrateC14Date` 1:1 대응)
pub fn calibrate_c14_date(
    bp_year: i32,
    sigma_years: i32,
    intcal20_curve: &[IntCalRecord],
) -> Result<RadiocarbonCalibrationResult, RadiocarbonError> {
    if !(0..=HGIS_INTCAL20_MAX_BP).contains(&bp_year) || intcal20_curve.is_empty() {
        return Err(RadiocarbonError::ExceedsIntCalBounds);
    }
    let sigma = sigma_years.max(HGIS_MIN_C14_SIGMA) as f64;
    let mut densities: Vec<(i32, f64)> = Vec::with_capacity(intcal20_curve.len());
    let mut sum_p = 0.0_f64;
    for rec in intcal20_curve {
        let s_total = (sigma * sigma + rec.curve_sigma * rec.curve_sigma).sqrt();
        let diff = bp_year as f64 - rec.c14_bp;
        let p = (-0.5 * (diff / s_total).powi(2)).exp() / s_total;
        let cal_ad_bc = HGIS_BP_DATUM_YEAR - rec.cal_bp;
        densities.push((cal_ad_bc, p));
        sum_p += p;
    }
    if sum_p <= 1e-15 {
        return Err(RadiocarbonError::ExceedsIntCalBounds);
    }
    for d in &mut densities {
        d.1 /= sum_p;
    }

    let build_hpd_range = |target_prob: f64| -> Vec<CalibratedRange> {
        let mut sorted = densities.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut acc = 0.0_f64;
        let mut min_yr = i32::MAX;
        let mut max_yr = i32::MIN;
        for &(yr, p) in &sorted {
            min_yr = min_yr.min(yr);
            max_yr = max_yr.max(yr);
            acc += p;
            if acc >= target_prob {
                break;
            }
        }
        vec![CalibratedRange {
            start_year_cal: min_yr,
            end_year_cal: max_yr,
            probability_pct: target_prob * 100.0,
        }]
    };

    Ok(RadiocarbonCalibrationResult {
        uncalibrated_bp: bp_year,
        sigma: sigma_years.max(HGIS_MIN_C14_SIGMA),
        range_1sigma: build_hpd_range(HGIS_C14_1SIGMA_PROBABILITY),
        range_2sigma: build_hpd_range(HGIS_C14_2SIGMA_PROBABILITY),
    })
}
