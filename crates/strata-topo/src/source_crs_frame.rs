//! The frame of a sheet as evidence of its belt (`sourceFrame` and `matchSourceFrame`,
//! `target/cpp-ref/src/core/TopographicSourceCrs.cpp:52-177`): in the right belt the closed polyline on CAD
//! layer H0017334 is a square of 7.5 minutes each way, every vertex on its sides to a quarter of a metre.

use strata_engine::{area_of_use, polygon_is_valid, Transformer};

use super::{box_of, grown, holds, GEOGRAPHIC, INVENTORY, MARGIN_M};

/// A frame that fits a belt.
pub(super) struct FrameMatch {
    /// The frame's box in the sheet's coordinates, grown by the margin: where the layers may lie.
    pub(super) allowed: [f64; 4],
    /// The farthest a vertex lies from the sides of the square, metres.
    pub(super) residual: f64,
    /// How far the frame's centre lies from the centre of the official index rectangle, metres.
    pub(super) displacement: f64,
}

/// The frame of a sheet: the one closed polyline on its layer, of 5 to 4096 points (`:89-93`; real frames
/// have some tens of vertices). Anything else is no frame.
pub(super) fn source_frame(lines: &[Vec<[f64; 2]>]) -> Option<&[[f64; 2]]> {
    let [points] = lines else {
        return None;
    };
    ((5..=4096).contains(&points.len()) && points.first() == points.last()).then_some(points.as_slice())
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Whether the frame is the neat line of a 1:25,000 sheet when its points are taken as coordinates of the
/// belt `epsg`, near the official rectangle (EPSG:5179).
///
/// # Errors
/// PROJ cannot carry a point (the C++ gives the whole sheet up then, `:335-338`).
pub(super) fn match_source_frame(points: &[[f64; 2]], epsg: u32, official: &[f64; 4]) -> Result<Option<FrameMatch>, String> {
    // 7.5 minutes each side. Jeju sheets sit 30 seconds off a lattice of 0.025 degrees, so no lattice is asked.
    const SHEET_DEGREES: f64 = 0.125;
    const SHEET_DEGREE_TOLERANCE: f64 = 1e-5;
    const MAX_RESIDUAL_M: f64 = 0.25;
    // Jeju frames of 2024 lie about 4 km from the index; 50 km is the cap of the official-footprint check.
    const MAX_INDEX_SEPARATION_M: f64 = 50_000.0;
    let failed = |e: strata_engine::EngineError| format!("원본 도곽 좌표 검증 실패: {}", e.0);
    let (Ok(to_geographic), Ok(to_source), Ok(to_inventory)) = (Transformer::new(epsg, GEOGRAPHIC), Transformer::new(GEOGRAPHIC, epsg), Transformer::new(GEOGRAPHIC, INVENTORY)) else {
        return Ok(None);
    };
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let geographic = to_geographic.transform_all(points).map_err(failed)?;
    let square = box_of(&geographic);
    let centre = [(square[0] + square[2]) / 2.0, (square[1] + square[3]) / 2.0];
    let is_sheet = square.iter().all(|v| v.is_finite()) && (square[2] - square[0] - SHEET_DEGREES).abs() <= SHEET_DEGREE_TOLERANCE && (square[3] - square[1] - SHEET_DEGREES).abs() <= SHEET_DEGREE_TOLERANCE;
    if !is_sheet || !area_of_use(epsg).ok().flatten().is_some_and(|area| holds(&area, centre)) {
        return Ok(None);
    }
    let official_centre = [(official[0] + official[2]) / 2.0, (official[1] + official[3]) / 2.0];
    let separation = distance(to_inventory.transform(centre).map_err(failed)?, official_centre);
    if !separation.is_finite() || separation > MAX_INDEX_SEPARATION_M {
        return Ok(None);
    }

    let corners = [[square[0], square[1]], [square[2], square[1]], [square[2], square[3]], [square[0], square[3]]];
    let corners_in_source = to_source.transform_all(&corners).map_err(failed)?;
    let (mut residual, mut found_corners, mut sides) = (0.0_f64, 0_u8, Vec::with_capacity(points.len()));
    for (point, on_globe) in points.iter().zip(&geographic) {
        let (x, y) = (on_globe[0].clamp(square[0], square[2]), on_globe[1].clamp(square[1], square[3]));
        // The nearest place on each side of the square: west, east, south, north.
        let nearest = to_source.transform_all(&[[square[0], y], [square[2], y], [x, square[1]], [x, square[3]]]).map_err(failed)?;
        let (mut on_sides, mut closest) = (0_u8, f64::INFINITY);
        for side in 0..4 {
            let off = distance(*point, nearest[side]);
            if off <= MAX_RESIDUAL_M {
                on_sides |= 1 << side;
            }
            closest = closest.min(off);
            if distance(*point, corners_in_source[side]) <= MAX_RESIDUAL_M {
                found_corners |= 1 << side;
            }
        }
        if on_sides == 0 || !closest.is_finite() {
            return Ok(None);
        }
        residual = residual.max(closest);
        sides.push(on_sides);
    }
    // All four corners, no shortcut from one side to another, and a ring that does not cross itself.
    let whole = found_corners == 15 && sides.windows(2).all(|pair| pair[0] & pair[1] != 0) && polygon_is_valid(&vec![geographic]).unwrap_or(false);
    Ok(whole.then(|| FrameMatch { allowed: grown(box_of(points), MARGIN_M), residual, displacement: separation }))
}
