//! 그린 면 닫기·고치기 — C++ `EditGeometryRepair::closePolygon`·`notice`(core/EditGeometryRepair.cpp:38-104).
//! 검사·고치기·넓이·둘레는 C++ 과 같은 GEOS(strata-engine)로 한다.

use strata_engine::PolygonRings;

use crate::gui_tab_map::format_area_m2;

/// 가늘기 4πA/P² 는 원이 1, 한 줄에 놓인 점은 0 에 가깝다(EditGeometryRepair.cpp:12).
const MIN_THINNESS: f64 = 1e-3;
/// 1 cm² (미터 작업 좌표계).
const MIN_AREA: f64 = 1e-4;

/// C++ `PolygonResult`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PolygonResult {
    /// 넣을 면(바깥 고리 + 구멍). 만들지 못하면 None.
    pub(crate) rings: Option<PolygonRings>,
    pub(crate) repaired: bool,
    pub(crate) parts_area: f64,
    pub(crate) kept_area: f64,
    pub(crate) dropped_parts: usize,
    pub(crate) degenerate: bool,
}

/// QgsPointXY `==`(좌표마다 1e-8 안).
pub(crate) fn same_point(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() <= 1e-8 && (a[1] - b[1]).abs() <= 1e-8
}

fn sqr_dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

/// `closePolygon(sketch, minSpacing)` — 앞 점과 간격 안의 점은 빼고, 닫고, GEOS 로 틀리면 makeValid,
/// 면 조각 가운데 가장 큰 것 하나를 남긴다. GEOS 가 실패하면 도형 없음(C++ 은 빈 도형).
pub(crate) fn close_polygon(sketch: &[[f64; 2]], min_spacing: f64) -> PolygonResult {
    let mut result = PolygonResult::default();
    let tol2 = min_spacing * min_spacing;
    let mut ring: Vec<[f64; 2]> = Vec::new();
    for p in sketch {
        if ring.last().is_none_or(|last| sqr_dist(*last, *p) > tol2) {
            ring.push(*p);
        }
    }
    if ring.len() >= 3 && !same_point(ring[0], ring[ring.len() - 1]) {
        ring.push(ring[0]);
    }
    if ring.len() < 4 {
        return result;
    }
    let mut parts = vec![vec![ring]];
    match strata_engine::polygon_is_valid(&parts[0]) {
        Ok(true) => {}
        Ok(false) => match strata_engine::make_valid_polygon_parts(&parts[0]) {
            Ok(fixed) if !fixed.is_empty() => {
                parts = fixed;
                result.repaired = true;
            }
            Ok(_) => {}
            Err(_) => return result,
        },
        Err(_) => return result,
    }

    let mut best: Option<usize> = None;
    for (i, part) in parts.iter().enumerate() {
        let Ok(area) = strata_engine::polygon_area(part) else { return PolygonResult::default() };
        result.parts_area += area;
        if best.is_none() || area > result.kept_area {
            best = Some(i);
            result.kept_area = area;
        }
    }
    let Some(best) = best else { return result };
    result.dropped_parts = parts.len() - 1;
    let perimeter = strata_engine::polygon_perimeter(&parts[best]).unwrap_or(0.0);
    let thinness =
        if perimeter > 0.0 { 4.0 * std::f64::consts::PI * result.kept_area / (perimeter * perimeter) } else { 0.0 };
    result.degenerate = result.kept_area < MIN_AREA || thinness < MIN_THINNESS;
    result.rings = Some(parts.swap_remove(best));
    result
}

/// `notice(result)` — 그린 뒤 알릴 글(없으면 None).
pub(crate) fn notice(result: &PolygonResult) -> Option<String> {
    result.rings.as_ref()?;
    if result.dropped_parts > 0 {
        let dropped = (result.parts_area - result.kept_area).max(0.0);
        return Some(format!(
            "선이 서로 꼬인 면이라 가장 큰 조각만 넣었습니다. 빠진 조각 {}개({})를 도형선택으로 확인하세요.",
            result.dropped_parts,
            format_area_m2(dropped)
        ));
    }
    if result.degenerate {
        return Some(format!(
            "점이 거의 한 줄이라 면적이 {}뿐입니다. 제출 검수에서 걸리니 다시 그리세요.",
            format_area_m2(result.kept_area)
        ));
    }
    None
}

#[cfg(test)]
#[path = "edit_geometry_repair_tests.rs"]
mod tests;
