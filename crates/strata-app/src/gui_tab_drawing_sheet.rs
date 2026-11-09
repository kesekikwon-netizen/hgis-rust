//! 도면 용지 한 장의 바깥 틀·축척 계산과 구간 호출 순서.
//!
//! 원문 `render_drawing_tab_ui` 의 CentralPanel 클로저 앞부분(용지 사각형·축척 계산)이다.
//! 계산이 끝나면 `SheetFrame` 을 만들어 구간 메서드들을 **원문 순서대로** 부른다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 용지 사각형을 잡고 조판 구간들을 순서대로 그린다.
    pub(crate) fn paint_drawing_sheet(&mut self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, Color32::from_rgb(245, 243, 236));

        let hint_r = Rect::from_min_size(
            Pos2::new(rect.left() + 18.0, rect.top() + 16.0),
            Vec2::new(196.0, 30.0),
        );
        p.rect_filled(hint_r, 6.0, Color32::WHITE);
        p.rect_stroke(
            hint_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(190, 188, 178)),
            egui::StrokeKind::Middle,
        );
        p.text(
            hint_r.center(),
            egui::Align2::CENTER_CENTER,
            "투명도·밝기: 조절할 레이어를 고르세요",
            egui::FontId::proportional(11.0),
            Color32::from_rgb(75, 74, 70),
        );

        // 1. 용지(mm) 비율에 맞춘 화면 용지 사각형 (상단 힌트 알약과 하단 7버튼 툴바 사이에 100% 수용)
        let paper_w_mm = (self.drawing.sheet.width_mm.max(100.0)) as f32;
        let paper_h_mm = (self.drawing.sheet.height_mm.max(100.0)) as f32;
        let compact_v = rect.height() < 460.0;
        let top_bound = if compact_v {
            rect.top() + 8.0
        } else {
            rect.top() + 48.0
        };
        // 하단 7버튼 툴바는 bar_cy = rect.bottom() - bar_h/2 - 4 에 그려지므로
        // 그 위쪽(bottom_bound)까지만 용지를 쓴다. 예전 값 -52 는 툴바 높이(42)
        // 보다 작아서 용지 하단이 툴바를 덮었다 (실화면 확인).
        let toolbar_h = if compact_v { 34.0 } else { 42.0 };
        let bottom_bound = (rect.bottom() - toolbar_h - 16.0).max(top_bound + 120.0);
        let avail_w = (rect.width() - 28.0).max(80.0);
        let avail_h = (bottom_bound - top_bound).max(80.0);
        let px_per_mm = (avail_w / paper_w_mm).min(avail_h / paper_h_mm).clamp(0.25, 3.2);
        let sheet_w = paper_w_mm * px_per_mm;
        let sheet_h = paper_h_mm * px_per_mm;
        // 용지는 이 CentralPanel 안에서만 그린다. rect 는 이미 좌우 패널이
        // 빠진 영역이지만, 종이 비율 때문에 세로가 먼저 차면 가로가 남는다.
        // 넘칠 때 화면 밖으로 나가지 않도록 rect 로 자른다.
        let raw_sheet_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, 0.5 * (top_bound + bottom_bound)),
            Vec2::new(sheet_w, sheet_h),
        );
        let sheet_rect = Rect::from_min_max(
            Pos2::new(
                raw_sheet_rect.left().max(rect.left()),
                raw_sheet_rect.top().max(rect.top()),
            ),
            Pos2::new(
                raw_sheet_rect.right().min(rect.right()),
                raw_sheet_rect.bottom().min(bottom_bound.min(rect.bottom())),
            ),
        );
        p.rect_filled(
            sheet_rect.translate(Vec2::new(4.0, 5.0)),
            1.0,
            Color32::from_rgba_unmultiplied(0, 0, 0, 38),
        );
        p.rect_filled(sheet_rect, 1.0, Color32::WHITE);
        p.rect_stroke(
            sheet_rect,
            1.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );

        let sheet_p = p.with_clip_rect(sheet_rect);
        let mm_to_rect = |r: strata_layout::sheet_composer::SheetRectMm| -> Rect {
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
        };
        let chrome = self.drawing.sheet.standard_chrome();
        let map_rect = mm_to_rect(chrome.map);

        let (map_bg, [scx, scy]) = {
            let rs = &self.home_map.real_survey;
            let map_bg = if rs.is_layer_visible("지형 음영")
                || rs.is_layer_visible("지표모델(DSM) 30m · Coperni...")
            {
                Color32::from_rgb(241, 244, 235)
            } else {
                Color32::from_rgb(252, 251, 248)
            };
            (map_bg, rs.survey_center)
        };
        sheet_p.rect_filled(map_rect, 0.0, map_bg);
        // 2. 실제 조사 레이어(조사구역·시굴격자·지적도·국가유산·500m 버퍼·작도 유구)를 축척(scale_denominator)에 맞춰 렌더링
        let scale_denom = f64::from(self.drawing.sheet.scale_denominator.max(1));
        let px_per_meter = (1000.0 / scale_denom) * f64::from(px_per_mm);
        let half_w_m = f64::from(map_rect.width()) * 0.5 / px_per_meter.max(1e-6) + 250.0;
        let half_h_m = f64::from(map_rect.height()) * 0.5 / px_per_meter.max(1e-6) + 250.0;
        let ink = Color32::from_rgb(17, 24, 39);
        let mc = map_rect.center();
        let map_p = sheet_p.with_clip_rect(map_rect);
        let f = SheetFrame {
            sheet_rect,
            map_rect,
            px_per_mm,
            px_per_meter,
            scale_denom,
            mc,
            scx,
            scy,
            half_w_m,
            half_h_m,
            chrome,
            ink,
            rect,
            p,
            sheet_p,
            map_p,
            compact_v,
        };
        // 원문 순서 그대로: 지도 레이어 → 도곽 격자 → 범례 → 장식띠(표제란·축척·방위)
        //                 → 7버튼 툴바 → 팝업
        self.paint_sheet_map_layers(&f);
        self.paint_sheet_grid(&f);
        self.paint_sheet_legend(&f);
        self.paint_sheet_chrome(&f);
        self.paint_sheet_toolbar(ui, &f);
        self.paint_sheet_popups(ui, &f);
    }
}
