//! 엔진 연결부 — C++ 앱(QGIS)이 쓰는 것과 같은 엔진을 붙이는 유일한 크레이트.
//!
//! - 좌표 변환 = PROJ(georust `proj` 0.31, 소스 빌드 PROJ 9.6.2). QGIS `QgsCoordinateTransform` 처럼
//!   `proj_create_crs_to_crs` 뒤 `proj_normalize_for_visualization`(x = 경도·동, y = 위도·북).
//! - 도형 검사·고치기 = GEOS(georust `geos` 11.3, 정적 빌드 GEOS 3.15.1). QGIS `isGeosValid`·
//!   `makeValid()`(linework, keepCollapsed = false)와 같은 GEOS 함수.
//!
//! 다른 크레이트는 proj·geos 를 직접 쓰지 않고 이 크레이트의 함수만 부른다. 판 고정도 여기에 있다.

#![forbid(unsafe_code)]

mod crs;
mod geom;
mod geom_wkt;
mod rings;
#[cfg(feature = "gdal")]
mod submit_shp;
#[cfg(feature = "gdal")]
mod submit_shp_names;

pub use crs::Transformer;
pub use geom::{make_valid_polygon_parts, normalized_wkt, polygon_area, polygon_is_valid, polygon_perimeter, PolygonRings};
pub use geom_wkt::{union_area, wkt_area, wkt_is_valid, wkt_length, wkt_point_relation, SurveyArea};
pub use rings::{distance_rings, ring_label_points};
#[cfg(feature = "gdal")]
pub use submit_shp::{
    install_submit_proj_db, set_gdal_proj_search_paths, write_submit_shapefiles, ShpLayerReport, SubmitEncoding,
    SubmitSource, SUBMIT_KEYS,
};

/// 엔진 오류(PROJ·GEOS 가 돌려준 글).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

impl From<geos::Error> for EngineError {
    fn from(e: geos::Error) -> Self {
        Self(e.to_string())
    }
}
