//! WKT 로 주는 아무 도형의 GEOS 검사 — C++ 검수 상태(ChecklistStateGeometry.cpp)가 QGIS 로 하는 것과 같은 GEOS 함수.
//! `isGeosValid`(GEOSisValid), `area`, 조사구역 안(`QgsGeometry::unaryUnion(구역들).buffer(허용치, 4)` 를 만든 뒤
//! QgsGeometryEngine::contains = GEOSContains).

use geos::{Geom, Geometry};

use crate::EngineError;

/// GEOS isValid.
///
/// # Errors
/// WKT 를 읽지 못함.
pub fn wkt_is_valid(wkt: &str) -> Result<bool, EngineError> {
    Ok(Geometry::new_from_wkt(wkt)?.is_valid()?)
}

/// 평면 넓이(면이 아니면 0).
///
/// # Errors
/// WKT 를 읽지 못함.
pub fn wkt_area(wkt: &str) -> Result<f64, EngineError> {
    Ok(Geometry::new_from_wkt(wkt)?.area()?)
}

/// 점이 도형 안에 있는가(GEOSContains)와 점까지 거리(GEOSDistance) — C++ FeaturePick 의 contains·distance.
///
/// # Errors
/// WKT 를 읽지 못함.
pub fn wkt_point_relation(wkt: &str, xy: [f64; 2]) -> Result<(bool, f64), EngineError> {
    let g = Geometry::new_from_wkt(wkt)?;
    let p = Geometry::new_from_wkt(&format!("POINT ({} {})", xy[0], xy[1]))?;
    Ok((g.contains(&p)?, g.distance(&p)?))
}

/// 도형들을 합친(unaryUnion) 넓이 — 겹친 곳은 한 번만 센다(C++ pickAutoFillArea 의 OGR Union 넓이).
///
/// # Errors
/// WKT 를 읽지 못하거나 GEOS 가 합치지 못함.
pub fn union_area(wkts: &[String]) -> Result<f64, EngineError> {
    if wkts.is_empty() {
        return Ok(0.0);
    }
    let parts = wkts.iter().map(|w| Geometry::new_from_wkt(w)).collect::<Result<Vec<_>, _>>()?;
    Ok(Geometry::create_geometry_collection(parts)?.unary_union()?.area()?)
}

/// 조사구역들을 합쳐 허용치만큼 넓힌 면.
pub struct SurveyArea {
    area: Geometry,
}

impl std::fmt::Debug for SurveyArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SurveyArea")
    }
}

impl SurveyArea {
    /// 면 WKT 들의 unaryUnion → buffer(허용치, 사분원 4 조각). 면이 없으면 None.
    ///
    /// # Errors
    /// WKT 를 읽지 못하거나 GEOS 가 합치지 못함.
    pub fn new(polygon_wkts: &[String], tolerance: f64) -> Result<Option<Self>, EngineError> {
        if polygon_wkts.is_empty() {
            return Ok(None);
        }
        let parts = polygon_wkts.iter().map(|w| Geometry::new_from_wkt(w)).collect::<Result<Vec<_>, _>>()?;
        let union = Geometry::create_geometry_collection(parts)?.unary_union()?;
        let area = union.buffer(tolerance, 4)?;
        Ok((!area.is_empty()?).then_some(Self { area }))
    }

    /// 도형이 넓힌 조사구역 안에 다 들어가는가(GEOSContains).
    ///
    /// # Errors
    /// WKT 를 읽지 못함.
    pub fn contains(&self, wkt: &str) -> Result<bool, EngineError> {
        Ok(self.area.contains(&Geometry::new_from_wkt(wkt)?)?)
    }
}

#[cfg(test)]
#[path = "geom_wkt_tests.rs"]
mod tests;
