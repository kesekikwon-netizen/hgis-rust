//! 검수 상태의 도형 검사 — C++ `ChecklistStateChecks::scanGeometry`·`isAbstractMarker`(ChecklistStateGeometry.cpp).
//! 유효성·넓이·조사구역 안은 같은 GEOS(strata-engine), 좌표계 옮기기는 같은 PROJ.
//! 도면 판정용 범위(DomainExtents)는 조판 용지가 생기면 쓴다 — 지금은 만들지 않는다.

use serde_json::{json, Map, Value};
use strata_engine::{SurveyArea, Transformer};
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::checklist_state::{feature_label, DomainLayer, Offenders, DOMAIN_KEYS};
use crate::edit_geometry_repair::same_point;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Point,
    Line,
    Polygon,
}

fn kind_of(parts: &[WkbGeometry]) -> Kind {
    match parts.first() {
        Some(WkbGeometry::Point(_)) => Kind::Point,
        Some(WkbGeometry::LineString(_)) => Kind::Line,
        _ => Kind::Polygon,
    }
}

/// 같은 점이 이어진 것을 하나로 보고, 닫는 점을 뺀 꼭짓점 수(distinctVertices).
fn distinct_vertices(ring: &[[f64; 2]]) -> usize {
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for p in ring {
        if pts.last().is_none_or(|l| !same_point(*l, *p)) {
            pts.push(*p);
        }
    }
    if pts.len() > 1 && same_point(pts[0], pts[pts.len() - 1]) {
        pts.pop();
    }
    pts.len()
}

/// 점을 둘러 버퍼한 원 — 꼭짓점 12개 이상이 모두 중심에서 같은 거리(2 % 안)(isCircleLike).
fn is_circle_like(ring: &[[f64; 2]]) -> bool {
    let closed = ring.len() > 1 && same_point(ring[0], ring[ring.len() - 1]);
    let n = ring.len() - usize::from(closed);
    if n < 12 {
        return false;
    }
    let pts = &ring[..n];
    let cx = pts.iter().map(|p| p[0]).sum::<f64>() / n as f64;
    let cy = pts.iter().map(|p| p[1]).sum::<f64>() / n as f64;
    let mean = pts.iter().map(|p| (p[0] - cx).hypot(p[1] - cy)).sum::<f64>() / n as f64;
    mean > 0.0 && pts.iter().all(|p| (((p[0] - cx).hypot(p[1] - cy) - mean).abs() / mean) <= 0.02)
}

/// `isAbstractMarker` — 점, 또는 바깥 고리가 세 꼭짓점 이하이거나 원 모양인 면.
pub(crate) fn is_abstract_marker(parts: &[WkbGeometry]) -> bool {
    parts.iter().any(|p| match p {
        WkbGeometry::Point(_) => true,
        WkbGeometry::LineString(_) => false,
        WkbGeometry::Polygon(rings) => rings.first().is_some_and(|r| distinct_vertices(r) <= 3 || is_circle_like(r)),
    })
}

fn coords(pts: &[[f64; 2]]) -> String {
    pts.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", ")
}

fn part_body(p: &WkbGeometry) -> String {
    match p {
        WkbGeometry::Point(xy) => format!("({} {})", xy[0], xy[1]),
        WkbGeometry::LineString(l) => format!("({})", coords(l)),
        WkbGeometry::Polygon(r) => format!("({})", r.iter().map(|ring| format!("({})", coords(ring))).collect::<Vec<_>>().join(", ")),
    }
}

/// 조각들 → WKT(한 조각이면 그 형식, 여럿이면 MULTI…).
pub(crate) fn parts_wkt(parts: &[WkbGeometry]) -> String {
    let name = match kind_of(parts) {
        Kind::Point => "POINT",
        Kind::Line => "LINESTRING",
        Kind::Polygon => "POLYGON",
    };
    if parts.len() == 1 {
        return format!("{name} {}", part_body(&parts[0]));
    }
    format!("MULTI{name} ({})", parts.iter().map(part_body).collect::<Vec<_>>().join(", "))
}

pub(crate) fn moved(parts: &[WkbGeometry], tr: &Transformer) -> Option<Vec<WkbGeometry>> {
    let all = |pts: &[[f64; 2]]| tr.transform_all(pts).ok();
    parts
        .iter()
        .map(|p| match p {
            WkbGeometry::Point(xy) => tr.transform(*xy).ok().map(WkbGeometry::Point),
            WkbGeometry::LineString(l) => all(l).map(WkbGeometry::LineString),
            WkbGeometry::Polygon(r) => r.iter().map(|ring| all(ring)).collect::<Option<Vec<_>>>().map(WkbGeometry::Polygon),
        })
        .collect()
}

/// `scanGeometry` — 도형 품질, 조사구역 모양, 유구 형식, 조사구역 안, 제출 좌표계.
pub(crate) fn scan_geometry(layers: &[DomainLayer<'_>], work_epsg: u32, st: &mut Map<String, Value>, off: &mut Offenders) {
    let (mut valid, mut no_empty, mut no_zero, mut survey_poly) = (true, true, true, true);
    let (mut marker, mut real_geometry, mut export_ok, mut within) = (false, true, true, true);
    let mut survey_wkts: Vec<String> = Vec::new();
    let mut placed: Vec<(&DomainLayer<'_>, i64, String, String)> = Vec::new();
    for key in DOMAIN_KEYS {
        for layer in layers.iter().filter(|l| l.table.table == key) {
            if layer.table.layer.features.is_empty() {
                continue;
            }
            let src = u32::try_from(layer.table.srs_id).ok().filter(|s| *s > 0);
            let tr = src.and_then(|s| Transformer::new(s, work_epsg).ok());
            if tr.is_none() {
                export_ok = false;
                off.add("export_crs_valid", layer, -1, &layer.name);
            }
            for f in &layer.table.layer.features {
                let label = feature_label(f);
                let parts = wkt_to_parts(&f.wkt_geom).unwrap_or_default();
                if parts.is_empty() {
                    no_empty = false;
                    off.add("geometries_nonempty", layer, f.fid, &label);
                    continue;
                }
                let kind = kind_of(&parts);
                if !strata_engine::wkt_is_valid(&f.wkt_geom).unwrap_or(false) {
                    valid = false;
                    off.add("geometries_valid", layer, f.fid, &label);
                }
                if kind == Kind::Polygon && strata_engine::wkt_area(&f.wkt_geom).map_or(true, |a| a <= 0.0) {
                    no_zero = false;
                    off.add("geometries_nonzero_area", layer, f.fid, &label);
                }
                // transformTo — 좌표계를 모르면(QGIS 무효 좌표계) 옮기지 않고 그대로 쓴다. 옮기기 실패만 빠진다.
                let in_common = match &tr {
                    _ if src == Some(work_epsg) => Some(parts.clone()),
                    Some(t) => moved(&parts, t),
                    None => Some(parts.clone()),
                };
                if key == "survey_area" {
                    if kind != Kind::Polygon {
                        survey_poly = false;
                        off.add("survey_is_polygon", layer, f.fid, &label);
                    }
                    if is_abstract_marker(&parts) {
                        marker = true;
                        off.add("has_abstract_marker", layer, f.fid, &label);
                    }
                    if let (Kind::Polygon, Some(g)) = (kind, &in_common) {
                        survey_wkts.push(parts_wkt(g));
                    }
                    continue;
                }
                let (poly, line) = (key == "feature_poly", key == "feature_line");
                if (poly && kind != Kind::Polygon) || (line && kind != Kind::Line) {
                    real_geometry = false;
                    off.add("features_real_geometry", layer, f.fid, &label);
                }
                if let (true, Some(g)) = (poly || line || key == "artifact_point", &in_common) {
                    placed.push((layer, f.fid, label, parts_wkt(g)));
                }
            }
        }
    }
    // 유구는 조사구역 상자가 아니라 조사구역 면 안에 있어야 한다(0.5 m 허용).
    if !survey_wkts.is_empty() && !placed.is_empty() {
        if let Ok(Some(area)) = SurveyArea::new(&survey_wkts, 0.5) {
            for (layer, fid, label, wkt) in &placed {
                if !area.contains(wkt).unwrap_or(false) {
                    within = false;
                    off.add("features_within_survey", layer, *fid, label);
                }
            }
        }
    }
    for (k, v) in [
        ("geometries_valid", valid),
        ("geometries_nonempty", no_empty),
        ("geometries_nonzero_area", no_zero),
        ("survey_is_polygon", survey_poly),
        ("has_abstract_marker", marker),
        ("features_real_geometry", real_geometry),
        ("features_within_survey", within),
        ("export_crs_valid", export_ok),
    ] {
        st.insert(k.to_owned(), json!(v));
    }
}
