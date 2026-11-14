//! (2)~(4) 용지 장식띠 — 표제란, 축척막대·축척글·좌표계, 방위표.
//!
//! 원문의 장식띠 좌표 계산 + 표제란 블록 + 장식띠 블록을 한 파일에 둔다. 셋은
//! 원문에서도 연달아 있었고 계산값(`north_rect`·`chrome_*_r`·`s`)을 공유한다.
//! 표제란과 장식띠는 각각 메서드로 나눠 300줄을 지킨다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_layout::sheet_composer::format_title_block_lines;

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::{ChromeRects, SheetFrame};

impl StrataAppState {
    /// 장식띠 좌표를 계산하고 표제란·축척·방위표를 원문 순서대로 그린다.
    pub(crate) fn paint_sheet_chrome(&mut self, f: &SheetFrame) {
        // target/cpp-ref/src/app/KaDrawingStudioSheets.cpp:60 — QDate::currentDate().
        self.paint_sheet_chrome_on(f, chrono::Local::now().date_naive());
    }

    pub(crate) fn paint_sheet_chrome_on(&mut self, f: &SheetFrame, date: chrono::NaiveDate) {
        let sheet_rect = f.sheet_rect;
        let map_rect = f.map_rect;
        let px_per_mm = f.px_per_mm;
        let chrome = f.chrome;
        // (2) 하단 장식띠: LayoutService::standardSheetChrome 사각형 (scale_bar, scale_label, crs, north) 및 KaTitleBlock::defaultRect
        let s = (px_per_mm / 1.65).clamp(0.42, 1.3);
        let chrome_north_r = f.mm_to_rect(chrome.north);
        // KaDrawingStudio.cpp:233-252 decorationSizePercent
        let deco_pct = self.drawing.decoration_size_percent as f32 / 100.0;
        let size_scale =
            ((self.drawing.sheet.north_arrow_size_mm as f32 / 28.0) * deco_pct)
                .clamp(0.8 * deco_pct, 1.35 * deco_pct);
        let max_north_h = (sheet_rect.bottom() - chrome_north_r.top() - 2.0).max(12.0);
        let north_sz = (chrome_north_r.width() * size_scale).clamp(12.0, max_north_h.min(36.0));
        let north_rect = Rect::from_min_size(
            Pos2::new(
                (chrome_north_r.right() - north_sz).max(sheet_rect.left() + 3.0),
                chrome_north_r.top().max(map_rect.bottom() + 3.5 * px_per_mm),
            ),
            Vec2::new(north_sz, north_sz),
        );
        let chrome_crs_r = f.mm_to_rect(chrome.crs);
        let chrome_bar_r = f.mm_to_rect(chrome.scale_bar);
        let chrome_lbl_r = f.mm_to_rect(chrome.scale_label);
        let ink = f.ink;
        self.paint_sheet_title_block(f, s, ink, north_rect, date);
        self.paint_sheet_decoration(
            f,
            s,
            ink,
            ChromeRects {
                north: north_rect,
                crs: chrome_crs_r,
                bar: chrome_bar_r,
                label: chrome_lbl_r,
            },
        );
    }

    // target/cpp-ref/src/app/KaTitleBlock.cpp:62-64 — date formatted as yyyy-MM-dd.
    pub(crate) fn sheet_title_block_lines_on(&self, date: chrono::NaiveDate) -> [String; 5] {
        let rs = &self.home_map.real_survey;
        let date_ymd = date.format("%Y-%m-%d").to_string();
        format_title_block_lines(
            &self.drawing.title_ko,
            &rs.survey_name,
            &rs.site_name,
            self.drawing.sheet.scale_denominator,
            &date_ymd,
        )
    }

    /// 표제란 (`title_block_visible` 토글 연동, `KaTitleBlock.cpp:57-111` 1:1).
    fn paint_sheet_title_block(
        &mut self,
        f: &SheetFrame,
        s: f32,
        ink: Color32,
        north_rect: Rect,
        date: chrono::NaiveDate,
    ) {
        let sheet_rect = f.sheet_rect;
        let map_rect = f.map_rect;
        let px_per_mm = f.px_per_mm;
        let sheet_p = f.sheet_p.clone();
        // 표제란 (title_block_visible 토글 연동, KaTitleBlock.cpp:57-111 1:1)
        // 도면명 / 조사명 / 유적명 / 축  척 / 작성일 (빈 값은 ―, 위치는 chrome.crs 아래 1.5mm 또는 지도 우하단)
        if self.drawing.sheet.title_block_visible {
            let raw_tb_r = f.mm_to_rect(self.drawing.sheet.default_title_block_rect());
            let tb_top = map_rect.bottom() + 8.2 * px_per_mm;
            let tb_bottom = (sheet_rect.bottom() - 2.0 * px_per_mm).max(tb_top + 16.0);
            let tb_right = north_rect.left() - 2.8 * px_per_mm;
            let tb_w = (tb_right - raw_tb_r.left()).max(64.0 * s);
            let tb_x = (tb_right - tb_w).clamp(
                sheet_rect.left() + 3.0,
                (sheet_rect.right() - tb_w - 3.0).max(sheet_rect.left() + 3.0),
            );
            let tb_h = (tb_bottom - tb_top).max(16.0);
            let tb_y = tb_top.clamp(
                map_rect.bottom() + 2.0,
                (sheet_rect.bottom() - tb_h - 1.5).max(map_rect.bottom() + 2.0),
            );
            let tb_r = Rect::from_min_size(Pos2::new(tb_x, tb_y), Vec2::new(tb_w, tb_h));
            sheet_p.rect_filled(tb_r, 0.0, Color32::WHITE);
            sheet_p.rect_stroke(tb_r, 0.0, Stroke::new(0.9, ink), egui::StrokeKind::Middle);
            let tb_p = sheet_p.with_clip_rect(tb_r.shrink(1.0));
            let tb_lines = self.sheet_title_block_lines_on(date);
            let tb_font_sz = (4.5 * px_per_mm).clamp(3.8, 7.6);
            let line_step = (tb_r.height() - 2.4 * s) / 5.0;
            for (li, line) in tb_lines.iter().enumerate() {
                let ly = tb_r.top() + 1.2 * s + ((li as f32) + 0.5) * line_step;
                tb_p.text(
                    Pos2::new(tb_r.left() + 3.5 * s, ly),
                    egui::Align2::LEFT_CENTER,
                    line,
                    egui::FontId::proportional(tb_font_sz),
                    ink,
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_chrome_tests.rs"]
mod tests;
