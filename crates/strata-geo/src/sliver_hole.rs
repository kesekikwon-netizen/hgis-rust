//! 내부 구멍 뚫기와 가는 조각(sliver) 정리.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use geo::{Area, Coord, LineString, Polygon};

/// 병합 후 제거할 최소 슬리버 구멍 면적 (㎡, C++ `HGIS_MIN_SLIVER_AREA_M2`)
pub const HGIS_MIN_SLIVER_AREA_M2: f64 = 0.05;
/// 슬리버 정리용 스냅 허용 오차 (m, C++ `HGIS_SLIVER_SNAP_TOLERANCE_M`)
pub const HGIS_SLIVER_SNAP_TOLERANCE_M: f64 = 0.005;

/// 유구 폴리곤 내부에 교란 구역 내측 링(Hole)을 펀칭 추가 (C++ `KaHolePunchTool::punchInteriorHole`)
#[must_use]
pub fn punch_interior_hole_polygon(target: &Polygon<f64>, hole_ring: &[[f64; 2]]) -> Polygon<f64> {
    if hole_ring.len() < 3 {
        return target.clone();
    }
    let mut coords: Vec<Coord<f64>> = hole_ring.iter().map(|&[x, y]| Coord { x, y }).collect();
    if coords.first() != coords.last() {
        if let Some(&first) = coords.first() {
            coords.push(first);
        }
    }
    let mut interiors = target.interiors().to_vec();
    interiors.push(LineString::new(coords));
    Polygon::new(target.exterior().clone(), interiors)
}

/// 다각형 내부의 미세 슬리버 구멍(`area < min_sliver_area_m2`)을 제거하여 정리
/// (C++ `KaBooleanPolygonEditor::cleanSliversAndSpikes`)
#[must_use]
pub fn clean_sliver_interior_rings(poly: &Polygon<f64>, min_sliver_area_m2: f64) -> Polygon<f64> {
    let threshold = min_sliver_area_m2.max(0.0);
    let cleaned_interiors: Vec<LineString<f64>> = poly
        .interiors()
        .iter()
        .filter(|ring| {
            let hole_poly = Polygon::new((*ring).clone(), vec![]);
            hole_poly.unsigned_area() >= threshold
        })
        .cloned()
        .collect();
    Polygon::new(poly.exterior().clone(), cleaned_interiors)
}
