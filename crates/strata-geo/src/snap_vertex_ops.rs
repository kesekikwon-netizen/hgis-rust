//! 정점 스냅(직교·45°), 정점 삭제, 선분 기반 폴리곤 분할 가드.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use geo::{Area, Polygon};
use crate::boolean_split::split_polygon_by_line;

pub const HGIS_SNAP_ANGLE_ORTHO: f64 = 90.0;
pub const HGIS_SNAP_ANGLE_45: f64 = 45.0;
pub const HGIS_MIN_POLYGON_VERTICES: usize = 4;
pub const HGIS_MIN_LINE_VERTICES: usize = 2;

/// C++ `snapToOrtho45` 템플릿 — Shift 키 누름 시 원점 기준 45도 배수 각도로 커서 좌표 스냅
#[must_use]
pub fn snap_to_ortho_45(origin: [f64; 2], cursor: [f64; 2]) -> [f64; 2] {
    let dx = cursor[0] - origin[0];
    let dy = cursor[1] - origin[1];
    let dist = dx.hypot(dy);
    if dist < 1e-12 {
        return origin;
    }
    let angle_deg = dy.atan2(dx).to_degrees();
    let snapped_deg = (angle_deg / HGIS_SNAP_ANGLE_45).round() * HGIS_SNAP_ANGLE_45;
    let rad = snapped_deg.to_radians();
    [origin[0] + dist * rad.cos(), origin[1] + dist * rad.sin()]
}

/// C++ `FeatureEditController::deleteVertex` (Polygon) — 폐합 정점 포함 최소 4점(삼각형) 이하일 때 정점 삭제 차단
pub fn delete_polygon_vertex_checked(ring: &mut Vec<[f64; 2]>, vertex_index: usize) -> bool {
    if ring.len() <= HGIS_MIN_POLYGON_VERTICES || vertex_index >= ring.len() {
        return false;
    }
    let is_closed = ring.first() == ring.last();
    if is_closed && (vertex_index == 0 || vertex_index + 1 == ring.len()) {
        ring.remove(0);
        if let Some(&new_first) = ring.first() {
            if let Some(last_mut) = ring.last_mut() {
                *last_mut = new_first;
            }
        }
    } else {
        ring.remove(vertex_index);
    }
    true
}

/// C++ `FeatureEditController::deleteVertex` (LineString) — 최소 2점 이하일 때 정점 삭제 차단
pub fn delete_line_vertex_checked(line: &mut Vec<[f64; 2]>, vertex_index: usize) -> bool {
    if line.len() <= HGIS_MIN_LINE_VERTICES || vertex_index >= line.len() {
        return false;
    }
    line.remove(vertex_index);
    true
}

/// C++ `PolygonOps::splitPolygonWithLine` — 절단선 교차점이 2개 미만이어서 분할 조각이 2개 미만일 때 에러 반환 및 최대 면적 조각 FID 승계
pub fn split_polygon_with_line_checked(
    subject: &Polygon<f64>,
    line_start: [f64; 2],
    line_end: [f64; 2],
    orig_feat_no: &str,
) -> Result<Vec<(String, Polygon<f64>)>, String> {
    let mut pieces = split_polygon_by_line(subject, line_start, line_end);
    if pieces.len() < 2 {
        return Err("절단선이 폴리곤 경계를 2회 이상 완전 관통해야 합니다.".to_owned());
    }
    pieces.sort_by(|a, b| {
        b.unsigned_area()
            .partial_cmp(&a.unsigned_area())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = Vec::with_capacity(pieces.len());
    for (idx, poly) in pieces.into_iter().enumerate() {
        let feat_no = if idx == 0 {
            orig_feat_no.to_owned()
        } else {
            format!("{orig_feat_no}-{idx}")
        };
        out.push((feat_no, poly));
    }
    Ok(out)
}
