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

/// 지도 칸 가운데 `mc` 가 지상 `center` 이고 1 m 가 `ppm` 화면 점일 때, 지상 점의 용지 위 자리(북쪽이 위).
pub(crate) fn world_to_sheet_px(mc: Pos2, center: [f64; 2], ppm: f64, pt: [f64; 2]) -> Pos2 {
    Pos2::new(mc.x + ((pt[0] - center[0]) * ppm) as f32, mc.y - ((pt[1] - center[1]) * ppm) as f32)
}

/// `world_to_sheet_px` 의 역.
pub(crate) fn sheet_px_to_world(mc: Pos2, center: [f64; 2], ppm: f64, pos: Pos2) -> [f64; 2] {
    let ppm = ppm.max(1e-9);
    [center[0] + f64::from(pos.x - mc.x) / ppm, center[1] - f64::from(pos.y - mc.y) / ppm]
}

impl crate::app_state::StrataAppState {
    /// 도면 지도의 가운데 — C++ `studioMapExtent`(KaDrawingStudio.cpp:188-195)는 지도 화면 범위를 쓰고,
    /// 「레이어를 가운데」로 옮긴 뒤에는 그 자리.
    pub(crate) fn sheet_map_center(&self) -> [f64; 2] {
        self.drawing.sheet_center.unwrap_or(self.home_map.viewport.center_xy)
    }

    /// 「레이어를 가운데」 — C++ `centerSurveyInMap`(KaDrawingStudio.cpp:3708-3755): 고른 레이어의 범위 가운데,
    /// 없으면 조사구역 범위 가운데로 축척은 두고 옮긴다. 둘 다 없으면 고르라고 알린다.
    pub(crate) fn center_layer_in_sheet(&mut self) {
        let finite = |b: &[f64; 4]| b.iter().all(|v| v.is_finite()) && b[2] - b[0] > 0.0;
        let layer = self.current_legend_layer().map(|e| e.layer_key.clone());
        let layer_ext = layer
            .and_then(|k| self.survey_tables.iter().find(|t| t.table == k))
            .and_then(|t| crate::survey_paint_layers::table_extent_in(t, self.work_crs))
            .filter(|b| finite(b) && b[2] - b[0] < 2_000_000.0);
        let Some(b) = layer_ext.or_else(|| crate::survey_area_setup::survey_area_extent(self).filter(finite)) else {
            self.home_map.draw_guide_message = Some("가운데로 둘 레이어를 왼쪽에서 고르세요.".to_owned());
            return;
        };
        self.drawing.sheet_center = Some([(b[0] + b[2]) * 0.5, (b[1] + b[3]) * 0.5]);
        self.home_map.draw_guide_message = Some(format!(
            "축척 1 : {} 을 유지한 채 용지 가운데로 옮겼습니다.",
            self.drawing.sheet.scale_denominator.max(10)
        ));
    }
}

impl SheetFrame {
    /// mm 좌표 → 화면 사각형(자유함수에 위임). 호출부는 이 메서드를 쓴다.
    pub(crate) fn mm_to_rect(&self, r: SheetRectMm) -> Rect {
        mm_to_rect(self.sheet_rect, self.px_per_mm, r)
    }

    /// 월드 좌표 → 용지 위 화면 좌표.
    pub(crate) fn w2m(&self, pt: [f64; 2]) -> Pos2 {
        world_to_sheet_px(self.mc, [self.scx, self.scy], self.px_per_meter, pt)
    }

    /// 용지 위 화면 좌표 → 월드 좌표(`w2m` 의 역 — 좌표점 콜아웃).
    pub(crate) fn m2w(&self, pos: Pos2) -> [f64; 2] {
        sheet_px_to_world(self.mc, [self.scx, self.scy], self.px_per_meter, pos)
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

#[cfg(test)]
#[path = "gui_tab_drawing_frame_tests.rs"]
mod tests;
