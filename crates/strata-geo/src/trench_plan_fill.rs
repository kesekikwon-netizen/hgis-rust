//! Rotated straight-trench grid fill and boundary trimming (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:232-390`).

use geo::algorithm::bool_ops::BooleanOps;
use geo::{
    BoundingRect, Contains, Coord, CoordsIter, Geometry, HasDimensions, Intersects, LineString,
    Polygon,
};

use crate::trench_plan::{valid_cell, valid_spec, Cell, Spec, MAX_CELLS};
use crate::trench_plan_area::{area_problem, as_multipolygon, AREA_NOT_POLYGON, AREA_UNREADABLE};
use crate::trench_plan_ratio::number_from_high_side;

/// Minimum length kept when trimming at the boundary (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:116`).
const MIN_TRIM_LEN: f64 = 4.0;

const AREA_MISSING: &str = "조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.";
const BAD_SPEC: &str =
    "트렌치 규격이 맞지 않습니다. 폭은 2 m 이하, 길이는 20 m 이하, 둑은 0 m 이상이어야 합니다.";
const TOO_MANY_CELLS: &str =
    "구역에 비해 트렌치 간격이 좁아 칸 수가 한도를 넘습니다. 둑(간격)을 키우세요.";
const NOTHING_FITS: &str = "현재 규격과 방향으로는 구역 안에 들어가는 트렌치가 없습니다. 구역이 좁거나 짧으면 회전이나 규격을 바꿔 보세요.";

fn rect_polygon(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> Polygon<f64> {
    Polygon::new(
        LineString::from(vec![
            Coord { x: p0[0], y: p0[1] },
            Coord { x: p1[0], y: p1[1] },
            Coord { x: p2[0], y: p2[1] },
            Coord { x: p3[0], y: p3[1] },
            Coord { x: p0[0], y: p0[1] },
        ]),
        Vec::new(),
    )
}

/// Fills `area` with straight trenches, trimming boundary cells (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:232-390`).
///
/// # Errors
/// Returns the Korean user-facing reason when `area` or `spec` cannot yield any trench.
pub fn build_in_area(spec: &Spec, area: Option<&Geometry<f64>>) -> Result<Vec<Cell>, String> {
    let Some(geom) = area else {
        return Err(AREA_MISSING.to_owned());
    };
    if !valid_spec(spec) {
        return Err(BAD_SPEC.to_owned());
    }
    if let Some(problem) = area_problem(Some(geom)) {
        return Err(problem);
    }
    let Some(area_mp) = as_multipolygon(geom) else {
        return Err(AREA_NOT_POLYGON.to_owned());
    };
    let Some(env) = area_mp.bounding_rect() else {
        return Err(AREA_UNREADABLE.to_owned());
    };

    let ax = (env.min().x + env.max().x) * 0.5;
    let ay = (env.min().y + env.max().y) * 0.5;
    let a = spec.azimuth_deg.to_radians();
    let (c, s) = (a.cos(), a.sin());
    let world = |lx: f64, ly: f64| -> [f64; 2] { [lx * c + ly * s + ax, -lx * s + ly * c + ay] };
    let to_local = |wx: f64, wy: f64| -> (f64, f64) {
        let (dx, dy) = (wx - ax, wy - ay);
        (dx * c - dy * s, dx * s + dy * c)
    };

    let mut u_min = f64::INFINITY;
    let mut u_max = f64::NEG_INFINITY;
    let mut v_min = f64::INFINITY;
    let mut v_max = f64::NEG_INFINITY;
    for cx in [env.min().x, env.max().x] {
        for cy in [env.min().y, env.max().y] {
            let (lu, lv) = to_local(cx, cy);
            u_min = u_min.min(lu);
            u_max = u_max.max(lu);
            v_min = v_min.min(lv);
            v_max = v_max.max(lv);
        }
    }

    let (w, len, balk) = (spec.trench_width, spec.trench_length, spec.balk_width);
    let step_u = w + balk;
    let step_v = len + balk;
    let count_u = (u_max - u_min) / step_u;
    let count_v = (v_max - v_min) / step_v;
    if !count_u.is_finite()
        || !count_v.is_finite()
        || count_u < 0.0
        || count_v < 0.0
        || count_u > (MAX_CELLS as f64) - 2.0
        || count_v > (MAX_CELLS as f64) - 2.0
    {
        return Err(TOO_MANY_CELLS.to_owned());
    }
    let nu = (count_u as i64) + 2;
    let nv = (count_v as i64) + 2;
    if nu.saturating_mul(nv) > MAX_CELLS as i64 {
        return Err(TOO_MANY_CELLS.to_owned());
    }

    let mut out = Vec::new();
    let mut n = 1usize;
    for j in 0..nv {
        let v0 = v_min + (j as f64) * step_v;
        for i in 0..nu {
            let u0 = u_min + (i as f64) * step_u;
            let poly = rect_polygon(
                world(u0, v0),
                world(u0 + w, v0),
                world(u0 + w, v0 + len),
                world(u0, v0 + len),
            );
            let mut keep_v0 = v0;
            let mut keep_len = len;
            if !area_mp.contains(&poly) {
                if !area_mp.intersects(&poly) {
                    continue;
                }
                let inter = area_mp.intersection(&poly);
                if inter.is_empty() {
                    continue;
                }
                let mut lo_v = 1e300_f64;
                let mut hi_v = -1e300_f64;
                for coord in inter.coords_iter() {
                    let lv = to_local(coord.x, coord.y).1;
                    lo_v = lo_v.min(lv);
                    hi_v = hi_v.max(lv);
                }
                lo_v = lo_v.max(v0);
                hi_v = hi_v.min(v0 + len);
                let mut cut = hi_v - lo_v;
                if !cut.is_finite() || cut < MIN_TRIM_LEN || cut > len {
                    continue;
                }
                let mut fits = false;
                let mut attempt = 0;
                while attempt < 4 && cut >= MIN_TRIM_LEN {
                    let tp = rect_polygon(
                        world(u0, lo_v),
                        world(u0 + w, lo_v),
                        world(u0 + w, lo_v + cut),
                        world(u0, lo_v + cut),
                    );
                    if area_mp.contains(&tp) {
                        fits = true;
                        break;
                    }
                    let shrink = 0.25_f64.max(cut * 0.05);
                    lo_v += shrink * 0.5;
                    cut -= shrink;
                    attempt += 1;
                }
                if !fits || cut < MIN_TRIM_LEN {
                    continue;
                }
                keep_v0 = lo_v;
                keep_len = cut;
            }
            let k0 = world(u0, keep_v0);
            let cell = Cell {
                name: format!("{}{n}", spec.name_prefix),
                width: w,
                length: keep_len,
                ring: vec![
                    k0,
                    world(u0 + w, keep_v0),
                    world(u0 + w, keep_v0 + keep_len),
                    world(u0, keep_v0 + keep_len),
                    k0,
                ],
            };
            n += 1;
            if valid_cell(&cell) {
                out.push(cell);
            }
        }
    }

    if out.is_empty() {
        return Err(NOTHING_FITS.to_owned());
    }
    number_from_high_side(&mut out, spec.azimuth_deg, &spec.name_prefix);
    Ok(out)
}
