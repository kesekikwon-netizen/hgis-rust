//! The box of a rectangle carried into another CRS.
//!
//! The straight sides of a rectangle bend when they are carried between projections, so the four corners
//! alone do not give the box that holds it. As QGIS does for the C++ app (`QgsCoordinateTransform::
//! transformBoundingBox`, called at `target/cpp-ref/src/core/GeologyMapService.cpp:755`), points spread
//! evenly over the rectangle are carried and their extremes taken. The spread here is this crate's own: 33
//! points to a side, 1089 in all.

use crate::{EngineError, Transformer};

#[cfg(test)]
#[path = "crs_box_tests.rs"]
mod tests;

/// Points to a side of the rectangle.
const SIDE: u32 = 33;

impl Transformer {
    /// The box `[min x, min y, max x, max y]` that holds the rectangle after the transformation. A rectangle
    /// without width or height is carried as the points it has.
    ///
    /// # Errors
    /// The rectangle is not finite, or PROJ can carry none of its points.
    pub fn transform_box(&self, rect: [f64; 4]) -> Result<[f64; 4], EngineError> {
        if rect.iter().any(|v| !v.is_finite()) || rect[2] < rect[0] || rect[3] < rect[1] {
            return Err(EngineError("the rectangle to carry over is not a rectangle".to_owned()));
        }
        let steps = |size: f64| if size > 0.0 { SIDE } else { 1 };
        let (nx, ny) = (steps(rect[2] - rect[0]), steps(rect[3] - rect[1]));
        let at = |low: f64, high: f64, i: u32, n: u32| if n == 1 { low } else { low + (high - low) * f64::from(i) / f64::from(n - 1) };
        let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for j in 0..ny {
            for i in 0..nx {
                // A point PROJ cannot carry (outside what the projection covers) is left out, as QGIS leaves it out.
                if let Ok(p) = self.transform([at(rect[0], rect[2], i, nx), at(rect[1], rect[3], j, ny)]) {
                    if p[0].is_finite() && p[1].is_finite() {
                        out = [out[0].min(p[0]), out[1].min(p[1]), out[2].max(p[0]), out[3].max(p[1])];
                    }
                }
            }
        }
        if out[0] > out[2] {
            return Err(EngineError("no point of the rectangle could be carried over".to_owned()));
        }
        Ok(out)
    }
}
