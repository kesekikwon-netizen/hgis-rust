//! Upslope aspect, high-side numbering and ratio search (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:392-522`).

use geo::{Area, Geometry};

use crate::trench_plan::{
    total_area, valid_cell, valid_dimensions, Cell, ElevSample, RatioFill, SlopeAspect, Spec,
    MAX_LENGTH,
};
use crate::trench_plan_area::{area_problem, as_multipolygon, AREA_NOT_POLYGON};
use crate::trench_plan_fill::build_in_valid_area;

/// Default minimum slope percentage (`target/cpp-ref/src/core/TrenchGridGenerator.h:104`).
pub const DEFAULT_MIN_SLOPE_PCT: f64 = 0.5;

const BAD_RATIO_INPUT: &str =
    "폭은 0 초과 2 m 이하, 목표 비율은 0 초과 100% 이하의 유효한 값이어야 합니다.";
const BAD_AREA_CALC: &str = "조사구역 면적과 목표 면적을 계산하지 못했습니다.";
const BAD_RATIO_CELL: &str = "목표 면적에 맞는 유효한 시굴격자를 계산하지 못했습니다.";

/// Formats `v` like Qt `QString::arg(double)` (`'g'`, precision 6) (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:494-495`).
fn fmt_qstring_arg(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    let sci = format!("{:.5e}", v.abs());
    let (mantissa, exp_str) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp = exp_str.parse::<i32>().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|&ch| ch != '.').collect();
    let sign = if v.is_sign_negative() { "-" } else { "" };
    if (-4..6).contains(&exp) {
        let raw = if exp < 0 {
            format!("0.{}{digits}", "0".repeat((-exp - 1) as usize))
        } else {
            let split = (exp + 1) as usize;
            if split >= digits.len() {
                format!("{digits}{}", "0".repeat(split - digits.len()))
            } else {
                format!("{}.{}", &digits[..split], &digits[split..])
            }
        };
        let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
        format!("{sign}{trimmed}")
    } else {
        let head = &digits[..1];
        let tail = digits[1..].trim_end_matches('0');
        let m = if tail.is_empty() {
            head.to_owned()
        } else {
            format!("{head}.{tail}")
        };
        let exp_sign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{m}e{exp_sign}{:02}", exp.unsigned_abs())
    }
}

/// Least-squares plane fit for upslope direction (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:392-441`).
#[must_use]
pub fn upslope_aspect(samples: &[ElevSample], min_slope_pct: f64) -> SlopeAspect {
    let mut out = SlopeAspect {
        samples: samples.len(),
        ..SlopeAspect::default()
    };
    if samples.len() < 3 {
        return out;
    }

    let n = samples.len() as f64;
    let mut mx = 0.0;
    let mut my = 0.0;
    let mut mz = 0.0;
    for p in samples {
        mx += p.x;
        my += p.y;
        mz += p.z;
    }
    mx /= n;
    my /= n;
    mz /= n;

    let (mut sxx, mut sxy, mut syy, mut sxz, mut syz) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for p in samples {
        let dx = p.x - mx;
        let dy = p.y - my;
        let dz = p.z - mz;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
        sxz += dx * dz;
        syz += dy * dz;
    }
    let det = sxx * syy - sxy * sxy;
    if det.abs() < 1e-9 {
        return out;
    }

    let a = (sxz * syy - syz * sxy) / det;
    let b = (syz * sxx - sxz * sxy) / det;
    let grad = a.hypot(b);
    out.slope_pct = grad * 100.0;
    if out.slope_pct < min_slope_pct {
        return out;
    }

    // Match C++ `std::atan2(a, b) * 180.0 / M_PI` (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:435`).
    let mut az = a.atan2(b) * 180.0 / std::f64::consts::PI;
    if az < 0.0 {
        az += 360.0;
    }
    out.azimuth_deg = az;
    out.valid = true;
    out
}

/// Orders and numbers cells from the high side down, left to right (`target/cpp-ref/src/core/TrenchTerrain.cpp:76-110`).
pub fn number_from_high_side(cells: &mut Vec<Cell>, azimuth_deg: f64, prefix: &str) {
    let rad = azimuth_deg.to_radians();
    let (ux, uy) = (rad.sin(), rad.cos());
    let (vx, vy) = (rad.cos(), -rad.sin());

    let mut keys: Vec<(f64, f64, usize)> = Vec::with_capacity(cells.len());
    for (i, cell) in cells.iter().enumerate() {
        let corners = cell.ring.len().saturating_sub(1);
        if corners == 0 {
            keys.push((0.0, 0.0, i));
            continue;
        }
        let mut cx = 0.0;
        let mut cy = 0.0;
        for pt in &cell.ring[..corners] {
            cx += pt[0];
            cy += pt[1];
        }
        cx /= corners as f64;
        cy /= corners as f64;
        let along = ((cx * ux + cy * uy) * 100.0).round();
        let across = cx * vx + cy * vy;
        keys.push((along, across, i));
    }

    keys.sort_by(|a, b| {
        if a.0 != b.0 {
            b.0.total_cmp(&a.0)
        } else {
            a.1.total_cmp(&b.1).then_with(|| a.2.cmp(&b.2))
        }
    });

    let mut ordered = Vec::with_capacity(cells.len());
    for (idx, &(_, _, at)) in keys.iter().enumerate() {
        let mut cell = cells[at].clone();
        cell.name = format!("{prefix}{}", idx + 1);
        ordered.push(cell);
    }
    *cells = ordered;
}

/// Searches straight lengths and balks, shortening cells to match `target_pct` (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:443-522`).
#[must_use]
pub fn build_for_target_ratio(
    area: Option<&Geometry<f64>>,
    target_pct: f64,
    width: f64,
    azimuth_deg: f64,
) -> RatioFill {
    let mut best = RatioFill::default();
    let Some(geom) = area else {
        best.error = BAD_RATIO_INPUT.to_owned();
        return best;
    };
    if !valid_dimensions(width, MAX_LENGTH)
        || !target_pct.is_finite()
        || target_pct <= 0.0
        || target_pct > 100.0
        || !azimuth_deg.is_finite()
    {
        best.error = BAD_RATIO_INPUT.to_owned();
        return best;
    }
    if let Some(problem) = area_problem(Some(geom)) {
        best.error = problem;
        return best;
    }
    let Some(area_mp) = as_multipolygon(geom) else {
        best.error = AREA_NOT_POLYGON.to_owned();
        return best;
    };

    let area_m2 = geom.unsigned_area();
    let target_area = area_m2 * (target_pct / 100.0);
    if !area_m2.is_finite() || area_m2 <= 1e-6 || !target_area.is_finite() || target_area <= 0.0 {
        best.error = BAD_AREA_CALC.to_owned();
        return best;
    }
    best.area_m2 = area_m2;

    let mut s = Spec {
        trench_width: width,
        azimuth_deg,
        name_prefix: if target_pct <= 3.5 {
            "Sp-".to_owned()
        } else {
            "Tr-".to_owned()
        },
        ..Spec::default()
    };

    let mut best_area = f64::INFINITY;
    let exact_eps = 1e-9_f64.max(target_area * 1e-10);
    for len in [8.0, 10.0, 12.0, 16.0, 20.0] {
        s.trench_length = len;
        for step in 1..=60 {
            let balk = f64::from(step * 2);
            s.balk_width = balk;
            let Ok(cells) = build_in_valid_area(&s, &area_mp) else {
                continue;
            };
            let candidate_area = total_area(&cells);
            if candidate_area >= target_area && candidate_area < best_area {
                best_area = candidate_area;
                best.cells = cells;
                best.balk = balk;
                best.azimuth_deg = azimuth_deg;
            }
            if best_area - target_area < exact_eps {
                break;
            }
        }
        if best_area - target_area < exact_eps {
            break;
        }
    }

    if best.cells.is_empty() {
        best.error = format!(
            "현재 구역과 방향에서 폭 {} m·길이 20 m 이하로 목표 {}%를 배치할 수 없습니다. 기존 격자는 유지됩니다.",
            fmt_qstring_arg(width),
            fmt_qstring_arg(target_pct)
        );
        return best;
    }

    let factor = target_area / best_area;
    for cell in &mut best.cells {
        let original = cell.ring.clone();
        let dx = (original[3][0] - original[0][0]) * ((1.0 - factor) * 0.5);
        let dy = (original[3][1] - original[0][1]) * ((1.0 - factor) * 0.5);
        cell.ring[0] = [original[0][0] + dx, original[0][1] + dy];
        cell.ring[1] = [original[1][0] + dx, original[1][1] + dy];
        cell.ring[2] = [original[2][0] - dx, original[2][1] - dy];
        cell.ring[3] = [original[3][0] - dx, original[3][1] - dy];
        cell.ring[4] = cell.ring[0];
        cell.length *= factor;
        if !valid_cell(cell) {
            best.cells.clear();
            best.length = 0.0;
            best.error = BAD_RATIO_CELL.to_owned();
            return best;
        }
        best.length = best.length.max(cell.length);
    }
    best.ratio_pct = total_area(&best.cells) / area_m2 * 100.0;
    best
}
