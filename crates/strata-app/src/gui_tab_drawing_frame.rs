//! 도면 용지 한 장을 그리는 동안 쓰는 값 묶음(`SheetFrame`).
//!
//! 원래 `render_drawing_tab_ui` 안의 지역변수들이었다. 구간을 여러 메서드로 나누면
//! 인자 10개짜리 함수가 생기므로 한 구조체에 담아 넘긴다. 필드는 **원문에서 계산된
//! 순서대로** 채운다(`gui_tab_drawing_sheet` 의 `SheetFrame { .. }` 참고).
//!
//! 원문의 캡처 클로저 셋(`mm_to_rect`, `w2m`, `in_view_bbox`)은 여기 자유함수/메서드가
//! 되었다. 본문은 그대로이고 호출부만 `이름(` → `f.이름(` 로 바뀌었다.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use strata_layout::sheet_composer::{SheetChromeRects, SheetRectMm};

/// 용지 사각형·축척·장식띠 좌표. 모두 한 프레임 안에서 계산되는 값이다.
#[derive(Clone)]
pub(crate) struct SheetFrame {
    pub sheet_rect: Rect,
    pub map_rect: Rect,
    pub px_per_mm: f32,
    pub px_per_meter: f64,
    pub scale_denom: f64,
    pub mc: Pos2,
    pub scx: f64,
    pub scy: f64,
    pub half_w_m: f64,
    pub half_h_m: f64,
    pub chrome: SheetChromeRects,
    pub ink: Color32,
    pub rect: Rect,
    pub p: egui::Painter,
    pub sheet_p: egui::Painter,
    pub map_p: egui::Painter,
    pub compact_v: bool,
}

/// `egui::Painter` 에는 `Debug` 가 없어 손으로 적는다.
impl std::fmt::Debug for SheetFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SheetFrame")
            .field("sheet_rect", &self.sheet_rect)
            .field("map_rect", &self.map_rect)
            .field("px_per_mm", &self.px_per_mm)
            .field("px_per_meter", &self.px_per_meter)
            .finish_non_exhaustive()
    }
}

/// mm 좌표 → 화면 사각형. 원문 `render_drawing_tab_ui` 의 `mm_to_rect` 클로저 본문 그대로.
pub(crate) fn mm_to_rect(sheet_rect: Rect, px_per_mm: f32, r: SheetRectMm) -> Rect {
    Rect::from_min_size(
        Pos2::new(
            sheet_rect.left() + (r.x_mm as f32) * px_per_mm,
            sheet_rect.top() + (r.y_mm as f32) * px_per_mm,
        ),
        Vec2::new(
            (r.width_mm as f32) * px_per_mm,
            (r.height_mm as f32) * px_per_mm,
        ),
    )
}
/// 장식띠(축척·좌표계·방위)가 쓰는 화면 사각형 묶음.
///
/// 원문 `render_drawing_tab_ui` 안의 지역변수들이었다. 인자가 8개가 되어
/// clippy `too_many_arguments` 에 걸리므로 하나로 묶는다.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChromeRects {
    pub north: Rect,
    pub crs: Rect,
    pub bar: Rect,
    pub label: Rect,
}

impl SheetFrame {
    /// mm 좌표 → 화면 사각형(자유함수에 위임). 호출부는 이 메서드를 쓴다.
    pub(crate) fn mm_to_rect(&self, r: SheetRectMm) -> Rect {
        mm_to_rect(self.sheet_rect, self.px_per_mm, r)
    }

    /// 월드 좌표 → 용지 위 화면 좌표.
    pub(crate) fn w2m(&self, pt: [f64; 2]) -> Pos2 {
        let mc = self.mc;
        let scx = self.scx;
        let scy = self.scy;
        let px_per_meter = self.px_per_meter;
        Pos2::new(
            mc.x + ((pt[0] - scx) * px_per_meter) as f32,
            mc.y - ((pt[1] - scy) * px_per_meter) as f32,
        )
    }
    /// 이 점이 지도 칸(±250m 여유) 안인가.
    pub(crate) fn in_view_bbox(&self, pt: [f64; 2]) -> bool {
        let scx = self.scx;
        let scy = self.scy;
        let half_w_m = self.half_w_m;
        let half_h_m = self.half_h_m;
        (pt[0] - scx).abs() <= half_w_m && (pt[1] - scy).abs() <= half_h_m
    }
}
