//! Survey-area validation and auto-fill polygon selection (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:127-230`).

use std::collections::HashSet;

use geo::algorithm::bool_ops::BooleanOps;
use geo::line_intersection::{line_intersection, LineIntersection};
use geo::{
    Area, Contains, CoordsIter, Geometry, HasDimensions, Intersects, Line, LineString,
    MultiPolygon, Polygon,
};

use crate::trench_plan::{PickedArea, SurveyPoly};

pub use crate::trench_plan_fill::build_in_area;

pub(crate) const AREA_UNREADABLE: &str =
    "조사구역 면을 읽지 못했습니다. 조사구역을 다시 그리거나 선택하세요.";
pub(crate) const AREA_NOT_POLYGON: &str =
    "조사구역이 면(폴리곤)이 아니어서 시굴격자를 놓을 수 없습니다.";
const AREA_INVALID: &str = "조사구역 경계가 스스로 겹치거나 꼬여 있어(유효하지 않은 도형) 시굴격자를 놓을 수 없습니다. 꼭짓점을 고쳐 경계를 바로잡은 뒤 다시 적용하세요.";

pub(crate) fn as_multipolygon(geom: &Geometry<f64>) -> Option<MultiPolygon<f64>> {
    match geom {
        Geometry::Polygon(p) => Some(MultiPolygon::new(vec![p.clone()])),
        Geometry::MultiPolygon(mp) => Some(mp.clone()),
        _ => None,
    }
}

fn ring_is_simple(ring: &LineString<f64>) -> bool {
    let coords = &ring.0;
    if coords.len() < 4 || coords.first() != coords.last() {
        return false;
    }
    let segs: Vec<Line<f64>> = coords
        .windows(2)
        .filter(|w| w[0] != w[1])
        .map(|w| Line::new(w[0], w[1]))
        .collect();
    if segs.len() < 3 || Polygon::new(ring.clone(), Vec::new()).unsigned_area() <= 0.0 {
        return false;
    }
    for i in 0..segs.len() {
        for j in (i + 1)..segs.len() {
            let Some(hit) = line_intersection(segs[i], segs[j]) else {
                continue;
            };
            match hit {
                LineIntersection::Collinear { .. } => return false,
                LineIntersection::SinglePoint {
                    intersection,
                    is_proper,
                } => {
                    if is_proper {
                        return false;
                    }
                    if j == i + 1 && intersection == segs[i].end {
                        continue;
                    }
                    if i == 0 && j + 1 == segs.len() && intersection == segs[0].start {
                        continue;
                    }
                    return false;
                }
            }
        }
    }
    true
}

fn rings_share_collinear_edge(a: &LineString<f64>, b: &LineString<f64>) -> bool {
    for wa in a.0.windows(2).filter(|w| w[0] != w[1]) {
        let sa = Line::new(wa[0], wa[1]);
        for wb in b.0.windows(2).filter(|w| w[0] != w[1]) {
            let sb = Line::new(wb[0], wb[1]);
            if matches!(line_intersection(sa, sb), Some(LineIntersection::Collinear { .. })) {
                return true;
            }
        }
    }
    false
}

fn polygon_is_valid(poly: &Polygon<f64>) -> bool {
    if poly.is_empty() || !ring_is_simple(poly.exterior()) {
        return false;
    }
    let shell = Polygon::new(poly.exterior().clone(), Vec::new());
    let mut holes: Vec<Polygon<f64>> = Vec::with_capacity(poly.interiors().len());
    for hole_ring in poly.interiors() {
        if !ring_is_simple(hole_ring) || rings_share_collinear_edge(poly.exterior(), hole_ring) {
            return false;
        }
        let hole = Polygon::new(hole_ring.clone(), Vec::new());
        if !shell.contains(&hole) {
            return false;
        }
        if holes.iter().any(|prev| {
            prev.intersection(&hole).unsigned_area() > 0.0
                || rings_share_collinear_edge(prev.exterior(), hole_ring)
        }) {
            return false;
        }
        holes.push(hole);
    }
    true
}

fn multipolygon_is_valid(mp: &MultiPolygon<f64>) -> bool {
    if mp.0.is_empty() || !mp.0.iter().all(polygon_is_valid) {
        return false;
    }
    for i in 0..mp.0.len() {
        for j in (i + 1)..mp.0.len() {
            if mp.0[i].intersection(&mp.0[j]).unsigned_area() > 0.0
                || rings_share_collinear_edge(mp.0[i].exterior(), mp.0[j].exterior())
            {
                return false;
            }
        }
    }
    true
}

/// Returns why `area` cannot hold trenches, or `None` when valid (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:127-137, 165-168`).
#[must_use]
pub fn area_problem(area: Option<&Geometry<f64>>) -> Option<String> {
    let Some(geom) = area else {
        return Some(AREA_UNREADABLE.to_owned());
    };
    if geom.is_empty() {
        return Some(AREA_UNREADABLE.to_owned());
    }
    if geom.coords_iter().any(|c| !c.x.is_finite() || !c.y.is_finite()) {
        return Some(AREA_INVALID.to_owned());
    }
    let valid = match geom {
        Geometry::Polygon(p) => polygon_is_valid(p),
        Geometry::MultiPolygon(mp) => multipolygon_is_valid(mp),
        _ => return Some(AREA_NOT_POLYGON.to_owned()),
    };
    if !valid {
        return Some(AREA_INVALID.to_owned());
    }
    None
}

/// Chooses selected survey polygons or the highest-`fid` polygon (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:170-230`).
#[must_use]
pub fn pick_auto_fill_area(
    features: &[SurveyPoly],
    selected_fids: &[i64],
    use_all: bool,
) -> PickedArea {
    let mut out = PickedArea::default();
    let valid: Vec<&SurveyPoly> = features.iter().filter(|f| f.area.is_some()).collect();
    out.total_count = valid.len();
    if valid.is_empty() {
        return out;
    }

    let mut used: Vec<&SurveyPoly> = Vec::new();
    let sel: HashSet<i64> = selected_fids.iter().copied().collect();
    if !sel.is_empty() {
        for &f in &valid {
            if sel.contains(&f.fid) {
                used.push(f);
            }
        }
        if !used.is_empty() {
            out.used_selection = true;
        }
    }
    if used.is_empty() {
        if use_all {
            used.clone_from(&valid);
        } else {
            let mut newest = valid[0];
            for &f in &valid {
                if f.fid > newest.fid {
                    newest = f;
                }
            }
            used.push(newest);
        }
        out.used_selection = false;
    }
    out.used_count = used.len();

    if used.len() == 1 {
        if let Some(geom) = &used[0].area {
            out.area_m2 = geom.unsigned_area();
            out.area = Some(geom.clone());
        }
        return out;
    }

    let mut acc: Option<MultiPolygon<f64>> = None;
    for f in used {
        let Some(geom) = &f.area else { continue };
        let Some(mp) = as_multipolygon(geom) else { continue };
        acc = Some(match acc {
            None => mp,
            Some(mut prev) => {
                if prev.intersects(&mp) {
                    prev.union(&mp)
                } else {
                    prev.0.extend(mp.0);
                    prev
                }
            }
        });
    }
    if let Some(mp) = acc {
        out.area_m2 = mp.unsigned_area();
        out.area = Some(Geometry::MultiPolygon(mp));
    }
    out
}
