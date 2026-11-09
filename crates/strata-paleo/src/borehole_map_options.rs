//! C++ `GeoInfoBoreholeService::MapOptions` (`GeoInfoBoreholeService.h:60`) — 시추 5종
//! GIS 도면 생성 토글과 분석 반경(2km / 5km / 10km).

use serde::{Deserialize, Serialize};
use strata_types::error::{StrataError, StrataResult};

/// C++ `MapOptions::radiusMeters` 허용값 — `GeoInfoBoreholeParser.cpp:211`
/// (`r == 2000 || r == 5000 || r == 10000`).
pub const BOREHOLE_MAP_RADIUS_CHOICES_M: [i32; 3] = [2000, 5000, 10000];

/// C++ `MapOptions::radiusMeters = 5000` 기본값 (`GeoInfoBoreholeService.h:61`).
pub const BOREHOLE_MAP_DEFAULT_RADIUS_M: i32 = 5000;

/// C++ `GeoInfoBoreholeService::MapOptions` 1:1 대응.
///
/// 각 토글은 `GeoInfoBoreholeMaps.cpp:179-294` 의 도면 생성 분기와 다음처럼 대응한다.
/// - `marker_map` → `:188` 시추공 마커 레이어 표시(`++created`)
/// - `alluvial_isopach_map` → `:251-263` 자연 퇴적층 두께 래스터 + 등층후선 GPKG
/// - `fill_thickness_map` → `:264-270` 성토·매립층 두께 분포도
/// - `bedrock_depth_map` → `:271-276` 풍화암·기반암 출현 깊이(GL-m)
/// - `bedrock_elev_map` → `:277-285` 풍화암·기반암 해발고도(EL.m)
/// - `groundwater_map` → `:286-291` 평균 지하수위(GL-m)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoreholeMapOptions {
    /// 분석 반경(m). `BOREHOLE_MAP_RADIUS_CHOICES_M` 중 하나만 유효하다.
    #[serde(default = "default_radius_meters")]
    pub radius_meters: i32,
    /// ① 시추공 분포·주상도 요약 마커 맵
    #[serde(default = "default_true")]
    pub marker_map: bool,
    /// ② 충적층 두께 등층후선도(래스터 + 등치선)
    #[serde(default = "default_true")]
    pub alluvial_isopach_map: bool,
    /// ③ 성토·매립층(교란층) 두께 분포도
    #[serde(default = "default_true")]
    pub fill_thickness_map: bool,
    /// ④-A 풍화암·기반암 출현 깊이(GL-m)
    #[serde(default = "default_true")]
    pub bedrock_depth_map: bool,
    /// ④-B 풍화암·기반암 해발고도(EL.m)
    #[serde(default)]
    pub bedrock_elev_map: bool,
    /// ⑤ 평균 지하수위(GL-m) — 시추 + KIGAM 수위 결합
    #[serde(default = "default_true")]
    pub groundwater_map: bool,
}

const fn default_true() -> bool {
    true
}

const fn default_radius_meters() -> i32 {
    BOREHOLE_MAP_DEFAULT_RADIUS_M
}

impl Default for BoreholeMapOptions {
    fn default() -> Self {
        Self {
            radius_meters: BOREHOLE_MAP_DEFAULT_RADIUS_M,
            marker_map: true,
            alluvial_isopach_map: true,
            fill_thickness_map: true,
            bedrock_depth_map: true,
            bedrock_elev_map: false,
            groundwater_map: true,
        }
    }
}

impl BoreholeMapOptions {
    /// 반경 허용값 여부 — `GeoInfoBoreholeParser.cpp:211` 과 같은 3개 값만 인정한다.
    #[must_use]
    pub const fn radius_is_valid(&self) -> bool {
        matches!(self.radius_meters, 2000 | 5000 | 10000)
    }

    /// 도면 생성을 시작하기 전 반경 검증. C++ 읽기 경로는 조용히 5000 으로 되돌리지만
    /// (`GeoInfoBoreholeParser.cpp:209-211`), 라이브러리 경계에서는 잘못된 값을 알린다.
    pub fn validate(&self) -> StrataResult<()> {
        if self.radius_is_valid() {
            Ok(())
        } else {
            Err(StrataError::InvalidInput(format!(
                "시추 도면 분석 반경 {}m 는 지원하지 않습니다 (2000 / 5000 / 10000 중 하나)",
                self.radius_meters
            )))
        }
    }

    /// C++ `MapOptions::fromProject` (`GeoInfoBoreholeParser.cpp:209-211`) 의 반경 보정:
    /// 허용값이 아니면 기본 5000m 를 유지한다.
    #[must_use]
    pub const fn normalized_radius(radius_meters: i32) -> i32 {
        match radius_meters {
            2000 | 5000 | 10000 => radius_meters,
            _ => BOREHOLE_MAP_DEFAULT_RADIUS_M,
        }
    }

    /// 반경만 보정한 사본 — 프로젝트 설정에서 읽은 값을 안전하게 적용할 때 쓴다.
    #[must_use]
    pub const fn with_normalized_radius(mut self) -> Self {
        self.radius_meters = Self::normalized_radius(self.radius_meters);
        self
    }

    /// ① 마커 맵만 켜진 상태인지 (C++ `registerAlluvialIsopachLayer` 의 `{5000,true,true,...}` 는 제외)
    #[must_use]
    pub const fn any_raster_map_enabled(&self) -> bool {
        self.alluvial_isopach_map
            || self.fill_thickness_map
            || self.bedrock_depth_map
            || self.bedrock_elev_map
            || self.groundwater_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_types::error::StrataError;

    #[test]
    fn default_matches_cpp_map_options_5km_all_on_except_bedrock_elev() {
        let o = BoreholeMapOptions::default();
        assert_eq!(o.radius_meters, 5000);
        assert!(o.radius_is_valid());
        assert!(o.marker_map);
        assert!(o.alluvial_isopach_map);
        assert!(o.fill_thickness_map);
        assert!(o.bedrock_depth_map);
        assert!(!o.bedrock_elev_map);
        assert!(o.groundwater_map);
        assert!(o.validate().is_ok());
        assert!(o.any_raster_map_enabled());
    }

    #[test]
    fn normalized_radius_keeps_2000_5000_10000_and_falls_back_to_5000() {
        assert_eq!(BoreholeMapOptions::normalized_radius(2000), 2000);
        assert_eq!(BoreholeMapOptions::normalized_radius(5000), 5000);
        assert_eq!(BoreholeMapOptions::normalized_radius(10000), 10000);
        assert_eq!(BoreholeMapOptions::normalized_radius(3000), 5000);
        assert_eq!(BoreholeMapOptions::normalized_radius(-1), 5000);
    }

    #[test]
    fn validate_rejects_radius_outside_choices() {
        let bad = BoreholeMapOptions {
            radius_meters: 7500,
            ..BoreholeMapOptions::default()
        };
        assert!(!bad.radius_is_valid());
        match bad.validate() {
            Err(StrataError::InvalidInput(msg)) => assert!(msg.contains("7500")),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
        assert_eq!(
            bad.clone().with_normalized_radius().radius_meters,
            BOREHOLE_MAP_DEFAULT_RADIUS_M
        );
    }

    #[test]
    fn marker_only_option_has_no_raster_maps() {
        let o = BoreholeMapOptions {
            marker_map: true,
            alluvial_isopach_map: false,
            fill_thickness_map: false,
            bedrock_depth_map: false,
            bedrock_elev_map: false,
            groundwater_map: false,
            ..BoreholeMapOptions::default()
        };
        assert!(!o.any_raster_map_enabled());
    }
}
