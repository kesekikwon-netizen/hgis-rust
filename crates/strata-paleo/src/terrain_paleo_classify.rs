//! 고고 지형 분류(TPI·경사 표준편차)와 폴리곤 요약 지표.
//!
//! `terrain_edt_tpi` 에서 분리했다. `PaleoDemMetrics`·
//! `classify_paleo_dem_polygon` 은 strata-app 이 직접 쓰므로 이 창구에 남긴다.

use crate::edt_landform::PaleoLandformType;

/// 미지형 TPI 국소 반경 (15.0m, C++ `HGIS_TPI_INNER_RADIUS_M`)
pub const HGIS_TPI_INNER_RADIUS_M: f64 = 15.0;

/// 미지형 TPI 광역 반경 (150.0m, C++ `HGIS_TPI_OUTER_RADIUS_M`)
pub const HGIS_TPI_OUTER_RADIUS_M: f64 = 150.0;

/// 고지형 폴리곤 DEM·수계·시추 통합 계측치 (C++ `PaleoDemClassifier::Metrics`)
#[derive(Debug, Clone, PartialEq)]
pub struct PaleoDemMetrics {
    pub has_dem: bool,
    pub mean_elev_m: f64,
    pub local_relief_m: f64,
    pub mean_slope_deg: f64,
    pub river_dist_m: f64,
    pub has_borehole: bool,
    pub borehole_code: String,
    pub alluvial_thickness_m: f64,
    pub weathered_depth_m: f64,
    pub deposit_env: String,
    pub dominant_uscs: String,
}

/// 고지형 폴리곤 자동 분류 결과 및 근거 주석 (C++ `PaleoDemClassifier::Classification`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaleoDemClassification {
    pub kind: String,
    pub note: String,
}

/// 표준화 TPI(`tpi_std`)와 경사도(`slope_deg`) 결정 트리로 고지형 미지형 분류
/// (C++ `PaleoDemClassifier::classifyLandformGrid` 1:1 대응)
#[must_use]
pub fn classify_tpi_std_slope_landform(tpi_std: f64, slope_deg: f64) -> PaleoLandformType {
    if !tpi_std.is_finite() || !slope_deg.is_finite() {
        return PaleoLandformType::NoData;
    }
    if (0.3..=1.5).contains(&tpi_std) && slope_deg < 3.0 {
        PaleoLandformType::NaturalLevee
    } else if tpi_std < -0.2 && slope_deg < 1.5 {
        PaleoLandformType::Backswamp
    } else if tpi_std > 1.5 && slope_deg < 5.0 {
        PaleoLandformType::FluvialTerrace
    } else if tpi_std < -0.5 {
        PaleoLandformType::PaleoChannel
    } else if (3.0..15.0).contains(&slope_deg) {
        PaleoLandformType::AlluvialFan
    } else if slope_deg >= 15.0 {
        PaleoLandformType::Hillslope
    } else {
        PaleoLandformType::Floodplain
    }
}

/// C++ `PaleoDemClassifier::classify(baseKind, Metrics)` 1:1 대응 — 하안단구 T1/T2/T3 단구면 세분, 시추 퇴적상(사질/점토) 기반 자연제방·구하도 보정 및 `"자동: ..."` 판독 근거 생성
#[must_use]
pub fn classify_paleo_dem_polygon(base_kind: &str, m: &PaleoDemMetrics) -> PaleoDemClassification {
    let mut out_kind = base_kind.to_owned();
    let mut notes = Vec::new();

    if base_kind == "하안단구" {
        let step = if m.has_dem {
            if m.local_relief_m >= 35.0 {
                format!("고위단구(T3, 비고 {:.1}m)", m.local_relief_m)
            } else if m.local_relief_m >= 15.0 {
                format!("중위단구(T2, 비고 {:.1}m)", m.local_relief_m)
            } else {
                format!("저위단구(T1, 비고 {:.1}m)", m.local_relief_m)
            }
        } else {
            "단구면".to_owned()
        };
        notes.push(step);
    } else if matches!(base_kind, "자연제방" | "구하도" | "미저지" | "미고지") {
        if m.has_borehole {
            if m.deposit_env.contains("사질") && matches!(base_kind, "미저지" | "자연제방")
            {
                out_kind = "자연제방".to_owned();
            } else if m.deposit_env.contains("점토") && matches!(base_kind, "미저지" | "구하도")
            {
                out_kind = "구하도".to_owned();
            }
        } else if m.has_dem
            && base_kind == "미저지"
            && m.local_relief_m >= 2.5
            && m.mean_slope_deg <= 5.0
        {
            out_kind = "미고지".to_owned();
        }
        notes.push(out_kind.clone());
    } else {
        notes.push(out_kind.clone());
    }

    if m.has_dem {
        notes.push(format!(
            "표고 {:.1}m(비고 +{:.1}m)·경사 {:.1}°",
            m.mean_elev_m, m.local_relief_m, m.mean_slope_deg
        ));
    }
    if m.river_dist_m >= 0.0 {
        notes.push(format!("하천거리 {:.0}m", m.river_dist_m));
    }
    if m.has_borehole {
        let sed = if m.dominant_uscs.trim().is_empty() {
            m.deposit_env.as_str()
        } else {
            m.dominant_uscs.as_str()
        };
        let mut bh = format!(
            "시추 {} 충적 {:.1}m({sed})",
            m.borehole_code, m.alluvial_thickness_m
        );
        if m.weathered_depth_m >= 0.0 {
            bh.push_str(&format!("·생토 {:.1}m", m.weathered_depth_m));
        }
        notes.push(bh);
    }

    PaleoDemClassification {
        kind: out_kind,
        note: format!("자동: {}", notes.join(" · ")),
    }
}
