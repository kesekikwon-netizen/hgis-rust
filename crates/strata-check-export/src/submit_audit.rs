//! 제출 전 점검 — 인근 국가유산·지번 표·FLD01/GEO02 감사.
//!
//! `submit_package` 에서 분리했다.

use crate::checklist_engine::ChecklistSeverity;

/// C++ `HGIS_NEARBY_HERITAGE_BUFFER_M` — 주변 유적 기본 조사 반경 (`500.0 m`)
pub const HGIS_NEARBY_HERITAGE_BUFFER_M: f64 = 500.0;

/// C++ `HGIS_MIN_VALID_FEATURE_AREA_M2` — 최소 허용 유구 면적 슬리버 임계치 (`0.05 m²`)
pub const HGIS_MIN_VALID_FEATURE_AREA_M2: f64 = 0.05;

/// C++ `HGIS_DUPLICATE_VERTEX_TOL_M` — 중복 정점 허용 한계 (`0.001 m` = `1 mm`)
pub const HGIS_DUPLICATE_VERTEX_TOL_M: f64 = 0.001;

/// C++ `HGIS_DELIVERABLE_ZIP_COMPRESS` — 성과품 압축률 (`9`)
pub const HGIS_DELIVERABLE_ZIP_COMPRESS: u8 = 9;

/// C++ `HeritageSiteRecord` (`NearbyHeritageExport`) — 반경 500m 주변 유적 레코드
#[derive(Debug, Clone, PartialEq)]
pub struct HeritageSiteRecord {
    pub site_name: String,
    pub era: String,
    pub designation: String,
    pub distance_m: f64,
    pub centroid_coord: [f64; 2],
}

/// C++ `NearbyHeritageExport::extractNearbySites` — 조사구역 경계로부터 `buffer_radius_m` (기본 `500.0 m`) 이내 유적을 추출하고 거리순 오름차순 정렬 반환
#[must_use]
pub fn extract_nearby_heritage_sites(
    candidates: &[HeritageSiteRecord],
    buffer_radius_m: f64,
) -> Vec<HeritageSiteRecord> {
    let radius = if buffer_radius_m.is_finite() && buffer_radius_m > 0.0 {
        buffer_radius_m
    } else {
        HGIS_NEARBY_HERITAGE_BUFFER_M
    };
    let mut matched: Vec<HeritageSiteRecord> = candidates
        .iter()
        .filter(|s| s.distance_m.is_finite() && s.distance_m >= 0.0 && s.distance_m <= radius)
        .cloned()
        .collect();
    matched.sort_by(|a, b| {
        a.distance_m
            .partial_cmp(&b.distance_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matched
}

/// C++ `ParcelClipboard` — 지적 편입 필지 조서 행
#[derive(Debug, Clone, PartialEq)]
pub struct ParcelClipboardRow {
    pub pnu: String,
    pub location: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub included_area_m2: f64,
}

/// C++ `ParcelClipboard::copyParcelTableToClipboard` — 지적 편입 필지 조서 TSV 직렬화
#[must_use]
pub fn format_parcel_table_tsv(rows: &[ParcelClipboardRow]) -> String {
    let mut out =
        String::from("연번\tPNU\t소재지\t지번\t지목\t공부면적(㎡)\t편입면적(㎡)\t편입비율(%)\n");
    for (idx, r) in rows.iter().enumerate() {
        let ratio = if r.official_area_m2 > 0.0 {
            (r.included_area_m2 / r.official_area_m2 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.1}\n",
            idx + 1,
            r.pnu,
            r.location,
            r.jibun,
            r.jimok,
            r.official_area_m2,
            r.included_area_m2,
            ratio
        ));
    }
    out
}

/// C++ `ChecklistStateFields` (`FLD-01`) — 필수 속성 필드(`FEAT_NO`, `FEAT_TYPE`, `PERIOD`) 공백/누락 검사
#[must_use]
pub fn audit_required_feature_fields_fld01(
    feat_no: &str,
    feat_type: &str,
    period: &str,
) -> Option<(&'static str, ChecklistSeverity)> {
    if feat_no.trim().is_empty() || feat_type.trim().is_empty() || period.trim().is_empty() {
        Some(("FLD-01", ChecklistSeverity::Error))
    } else {
        None
    }
}

/// C++ `ChecklistStateGeometry` (`GEO-02`) — 유구 면적 슬리버(`area_m2 < 0.05 m²`) 검사
#[must_use]
pub fn audit_sliver_polygon_geo02(area_m2: f64) -> Option<(&'static str, ChecklistSeverity)> {
    if !area_m2.is_finite() || area_m2 < HGIS_MIN_VALID_FEATURE_AREA_M2 {
        Some(("GEO-02", ChecklistSeverity::Warning))
    } else {
        None
    }
}
