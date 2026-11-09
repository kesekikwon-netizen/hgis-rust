//! 지적 필지 포함 관계 분석.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use geo::{Coord, LineString, Polygon};

pub const HGIS_MIN_PARCEL_OVERLAP_M2: f64 = 0.1;
pub const HGIS_MAX_PARCEL_RECORDS_LIMIT: usize = 100_000;
pub const HGIS_AREA_ROUND_DECIMAL_PLACES: u32 = 2;
pub const HGIS_RATIO_ROUND_DECIMAL_PLACES: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct CadastralParcelInput {
    pub pnu: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub exterior_ring: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CadastralParcelRecord {
    pub pnu: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub total_geom_area_m2: f64,
    pub overlap_area_m2: f64,
    pub inclusion_ratio_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParcelInclusionSummary {
    pub total_parcels_count: usize,
    pub total_official_area_m2: f64,
    pub total_overlap_area_m2: f64,
    pub overall_inclusion_ratio_pct: f64,
    pub parcel_records: Vec<CadastralParcelRecord>,
}

/// C++ `KaCadastralOverlapAnalyzer::analyzeCadastralInclusion` — 조사구역 다각형과 지적 필지 다각형의 교차 면적(`min_intersection_area_m2 >= 0.1㎡` 슬리버 필터링), 소수점 둘째 자리 면적 반올림 및 소수점 첫째 자리 편입률 산출
#[must_use]
pub fn analyze_cadastral_inclusion(
    survey_boundary_ring: &[[f64; 2]],
    parcels: &[CadastralParcelInput],
    min_intersection_area_m2: f64,
) -> ParcelInclusionSummary {
    use geo::{Area, BooleanOps};

    if survey_boundary_ring.len() < 3 {
        return ParcelInclusionSummary {
            total_parcels_count: 0,
            total_official_area_m2: 0.0,
            total_overlap_area_m2: 0.0,
            overall_inclusion_ratio_pct: 0.0,
            parcel_records: Vec::new(),
        };
    }

    let survey_poly = Polygon::new(
        LineString::from(
            survey_boundary_ring
                .iter()
                .map(|&[x, y]| Coord { x, y })
                .collect::<Vec<_>>(),
        ),
        vec![],
    );
    let min_area = if min_intersection_area_m2.is_finite() && min_intersection_area_m2 >= 0.0 {
        min_intersection_area_m2
    } else {
        HGIS_MIN_PARCEL_OVERLAP_M2
    };

    let mut parcel_records = Vec::new();
    let mut total_official = 0.0;
    let mut total_overlap = 0.0;

    for p in parcels.iter().take(HGIS_MAX_PARCEL_RECORDS_LIMIT) {
        if p.exterior_ring.len() < 3 {
            continue;
        }
        let parcel_poly = Polygon::new(
            LineString::from(
                p.exterior_ring
                    .iter()
                    .map(|&[x, y]| Coord { x, y })
                    .collect::<Vec<_>>(),
            ),
            vec![],
        );
        let geom_area = parcel_poly.unsigned_area();
        if geom_area <= 1e-9 || !geom_area.is_finite() {
            continue;
        }
        let inter_mp = survey_poly.intersection(&parcel_poly);
        let raw_overlap = inter_mp.unsigned_area();
        if !raw_overlap.is_finite() || raw_overlap < min_area {
            continue;
        }

        let base_area = if p.official_area_m2 > 0.0 && p.official_area_m2.is_finite() {
            p.official_area_m2
        } else {
            geom_area
        };
        let overlap_rounded = (raw_overlap * 100.0).round() / 100.0;
        let raw_ratio = if base_area > 0.0 {
            (raw_overlap / base_area * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let ratio_rounded = (raw_ratio * 10.0).round() / 10.0;

        total_official += base_area;
        total_overlap += overlap_rounded;
        parcel_records.push(CadastralParcelRecord {
            pnu: p.pnu.clone(),
            jibun: p.jibun.clone(),
            jimok: p.jimok.clone(),
            official_area_m2: base_area,
            total_geom_area_m2: (geom_area * 100.0).round() / 100.0,
            overlap_area_m2: overlap_rounded,
            inclusion_ratio_pct: ratio_rounded,
        });
    }

    let overall_ratio = if total_official > 0.0 {
        ((total_overlap / total_official * 100.0).clamp(0.0, 100.0) * 10.0).round() / 10.0
    } else {
        0.0
    };

    ParcelInclusionSummary {
        total_parcels_count: parcel_records.len(),
        total_official_area_m2: (total_official * 100.0).round() / 100.0,
        total_overlap_area_m2: (total_overlap * 100.0).round() / 100.0,
        overall_inclusion_ratio_pct: overall_ratio,
        parcel_records,
    }
}
