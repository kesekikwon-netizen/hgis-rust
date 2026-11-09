//! 배경지도 종류와 고지도·주제도 레이어 명세 (C++ `BasemapOps.cpp`, `HistoricalMapOps`).
//!
//! `tile_fetcher` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasemapSource {
    VworldSatellite,
    VworldBase,
    VworldCadastralWms,
    Daedongyeojido,
    History1919,
}

impl BasemapSource {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::VworldSatellite,
            Self::VworldBase,
            Self::VworldCadastralWms,
            Self::Daedongyeojido,
            Self::History1919,
        ]
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::VworldSatellite => "VWorld 영상지도(위성)",
            Self::VworldBase => "VWorld 기본도",
            Self::VworldCadastralWms => "연속지적도 (VWorld WMS)",
            Self::Daedongyeojido => "대동여지도 (1861)",
            Self::History1919 => "1919 근대 오만분지일 지형도",
        }
    }
}

/// 1919 조선지형도 TMS URL 템플릿 (C++ `HGIS_CHOSUN1919_TMS_URL`)
pub const HGIS_CHOSUN1919_TMS_URL: &str =
    "type=xyz&url=http://hgis.local/tiles/1919/{z}/{x}/{-y}.png";

/// 대동여지도 TMS URL 템플릿 (C++ `HGIS_DAEDONG_TMS_URL`)
pub const HGIS_DAEDONG_TMS_URL: &str =
    "type=xyz&url=http://hgis.local/tiles/daedong/{z}/{x}/{-y}.png";

/// WMS 최대 연결 대기 시간 (5,000 ms, C++ `HGIS_WMS_TIMEOUT_MS`)
pub const HGIS_WMS_TIMEOUT_MS: u64 = 5000;

/// C++ `HistoricalMapType` 고지도 종류 1:1 대응
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalMapType {
    Chosun1919_1to50000,
    Daedongyeojido,
    Kwangbok1945,
    Cadastral1910Original,
}

/// C++ `ThematicLayerType` 주제도(토양·지질·수계) 종류 1:1 대응
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThematicLayerType {
    SoilDrainage,
    Geology1to50k,
    HydroNetworkWaterway,
}

/// C++ `HistoricalMapOps::loadHistoricalTileMap` 및 `SoilGeologyHydroLayerOps::loadWmsThematicLayer` 반환 레이어 명세
#[derive(Debug, Clone, PartialEq)]
pub struct RasterThematicLayerSpec {
    pub provider_type: &'static str,
    pub layer_title_ko: &'static str,
    pub source_url: String,
    pub opacity: f64,
}

/// 고지도 TMS 레이어 명세 생성 (C++ `HistoricalMapOps::loadHistoricalTileMap` 1:1 대응)
#[must_use]
pub fn build_historical_tms_layer_spec(
    map_type: HistoricalMapType,
    opacity: f64,
) -> RasterThematicLayerSpec {
    let clamped_opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.8
    };
    let (title, url) = match map_type {
        HistoricalMapType::Chosun1919_1to50000 => {
            ("1919 조선지형도 (1:50,000)", HGIS_CHOSUN1919_TMS_URL)
        }
        HistoricalMapType::Daedongyeojido => ("대동여지도 (1861 목판본)", HGIS_DAEDONG_TMS_URL),
        HistoricalMapType::Kwangbok1945 => (
            "1945 해방 직후 지형도",
            "type=xyz&url=http://hgis.local/tiles/1945/{z}/{x}/{-y}.png",
        ),
        HistoricalMapType::Cadastral1910Original => (
            "1910년대 토지조사사업 지적원도",
            "type=xyz&url=http://hgis.local/tiles/1910_cad/{z}/{x}/{-y}.png",
        ),
    };
    RasterThematicLayerSpec {
        provider_type: "wms",
        layer_title_ko: title,
        source_url: url.to_owned(),
        opacity: clamped_opacity,
    }
}

/// 토양·지질·수계 OGC WMS 1.3.0 레이어 명세 생성 (C++ `SoilGeologyHydroLayerOps::loadWmsThematicLayer` 1:1 대응)
#[must_use]
pub fn build_wms_thematic_layer_spec(
    layer_type: ThematicLayerType,
    crs_auth_id: &str,
) -> RasterThematicLayerSpec {
    let crs = if crs_auth_id.trim().is_empty() {
        "EPSG:5186"
    } else {
        crs_auth_id.trim()
    };
    let (title, layer_id) = match layer_type {
        ThematicLayerType::SoilDrainage => ("토양도 (배수등급)", "soil_drainage"),
        ThematicLayerType::Geology1to50k => ("수치지질도 (1:50,000)", "kigam_geology_50k"),
        ThematicLayerType::HydroNetworkWaterway => ("국가하천·수계망", "wamis_hydro_network"),
    };
    RasterThematicLayerSpec {
        provider_type: "wms",
        layer_title_ko: title,
        source_url: format!(
            "contextualWMSLegend=0&crs={crs}&format=image/png&layers={layer_id}&styles=default&transparent=true&version=1.3.0"
        ),
        opacity: 0.75,
    }
}
