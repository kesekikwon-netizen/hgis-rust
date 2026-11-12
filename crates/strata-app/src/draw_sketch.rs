//! 그리던 면의 꼭짓점 — C++ `KaCaptureMapTool`(KaCaptureMapTool.cpp)의 면(Polygon) 모드 규칙.
//! 쉽게 그리기·따라 그리기(snap trace)는 나중 지시다.

use crate::edit_geometry_repair::{close_polygon, same_point, PolygonResult};

/// `KaEditTolerance::kSketchVertexPx` — 새 점이 앞 점 위에 겹친 것으로 보는 화면 거리.
const SKETCH_VERTEX_PX: f64 = 10.0;

/// `nearPoint` 의 허용 거리 — 화면 10 px(최소 0.05 m), 점당 m 를 모르면 0.15 m.
pub(crate) fn vertex_tolerance(map_units_per_pixel: f64) -> f64 {
    if map_units_per_pixel > 0.0 {
        (map_units_per_pixel * SKETCH_VERTEX_PX).max(0.05)
    } else {
        0.15
    }
}

/// 면을 마치지 못한 까닭.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinishShort {
    /// 찍은 점이 없다 — C++ 은 아무 말도 하지 않는다.
    Empty,
    /// 점이 모자라거나 면을 만들지 못했다 — 그리던 점은 그대로 둔다(captureCanceled).
    TooFew,
}

/// 그리던 면.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DrawSketch {
    pub(crate) points: Vec<[f64; 2]>,
}

impl DrawSketch {
    fn near(a: [f64; 2], b: [f64; 2], mupp: f64) -> bool {
        let tol = vertex_tolerance(mupp);
        (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) <= tol * tol
    }

    /// 왼쪽 누름(canvasPressEvent) — 앞 점과 겹치지 않으면 점을 더한다.
    pub(crate) fn press_left(&mut self, xy: [f64; 2], mupp: f64) {
        if self.points.last().is_none_or(|last| !Self::near(*last, xy, mupp)) {
            self.points.push(xy);
        }
    }

    /// 더블클릭(canvasDoubleClickEvent) — 앞 점과 다른 자리면 더한다. 그다음 마치기는 부르는 쪽이 한다.
    pub(crate) fn double_click(&mut self, xy: [f64; 2]) {
        if self.points.last().is_none_or(|last| !same_point(*last, xy)) {
            self.points.push(xy);
        }
    }

    /// `undoLastVertex` — 마지막 점을 뺀다. 뺀 점이 없으면 false.
    pub(crate) fn undo_last(&mut self) -> bool {
        self.points.pop().is_some()
    }

    /// `cancel` — 그리던 점을 모두 지운다. 지운 점이 있었으면 true(「그리던 도형을 지웠습니다.」).
    pub(crate) fn cancel(&mut self) -> bool {
        let had = !self.points.is_empty();
        self.points.clear();
        had
    }

    /// `finish` 의 면 만들기 — 점 3개 이상이면 `closePolygon`(간격 = 꼭짓점 허용 거리). 점은 지우지 않는다:
    /// 레이어에 넣은 뒤 부르는 쪽이 `points.clear()` 한다(옮기기 실패 때는 C++ 처럼 남긴다).
    pub(crate) fn close(&self, mupp: f64) -> Result<PolygonResult, FinishShort> {
        if self.points.len() < 3 {
            return Err(if self.points.is_empty() { FinishShort::Empty } else { FinishShort::TooFew });
        }
        let closed = close_polygon(&self.points, vertex_tolerance(mupp));
        if closed.rings.is_some() {
            Ok(closed)
        } else {
            Err(FinishShort::TooFew)
        }
    }
}

#[cfg(test)]
#[path = "draw_sketch_tests.rs"]
mod tests;
