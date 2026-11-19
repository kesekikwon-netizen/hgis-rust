//! `gui_tab_drawing` 은 도면 스튜디오 탭의 재수출 창구다.
//!
//! 원래 한 파일(1,579줄)이었고 `render_drawing_tab_ui` **한 함수가 1,497줄**이었다.
//! 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 함수 본문을 연속 구간으로 잘라 냈다.
//! 기존 `crate::gui_tab_drawing::<이름>` 경로와 시험 이름은 그대로다.
//!
//! 본문은 이름 하나 바뀌지 않았다. 각 메서드 머리의 alias 줄
//! (`let sheet_p = f.sheet_p.clone();`)이 원래 지역변수 이름을 되살리고,
//! 바깥 지역변수는 `SheetFrame` 하나로 모아 넘긴다(인자 10개짜리 함수 방지).

use crate::app_state::StrataAppState;

#[path = "gui_tab_drawing_adjust.rs"]
mod gui_tab_drawing_adjust;
#[path = "gui_tab_drawing_chrome.rs"]
mod gui_tab_drawing_chrome;
#[path = "gui_tab_drawing_coords.rs"]
mod gui_tab_drawing_coords;
#[path = "gui_tab_drawing_decoration.rs"]
pub(crate) mod gui_tab_drawing_decoration;
#[path = "gui_tab_drawing_frame.rs"]
mod gui_tab_drawing_frame;
#[path = "gui_tab_drawing_grid.rs"]
mod gui_tab_drawing_grid;
#[path = "gui_tab_drawing_items.rs"]
mod gui_tab_drawing_items;
#[path = "gui_tab_drawing_legend.rs"]
mod gui_tab_drawing_legend;
#[path = "gui_tab_drawing_legend_flow.rs"]
mod gui_tab_drawing_legend_flow;
#[path = "gui_tab_drawing_map_layers.rs"]
mod gui_tab_drawing_map_layers;
#[path = "gui_tab_drawing_popups.rs"]
mod gui_tab_drawing_popups;
#[path = "gui_tab_drawing_preview.rs"]
mod gui_tab_drawing_preview;
#[path = "gui_tab_drawing_print.rs"]
pub(crate) mod gui_tab_drawing_print;
#[path = "gui_tab_drawing_put.rs"]
mod gui_tab_drawing_put;
#[path = "gui_tab_drawing_refs.rs"]
mod gui_tab_drawing_refs;
#[path = "gui_tab_drawing_right.rs"]
mod gui_tab_drawing_right;
#[path = "gui_tab_drawing_right_b.rs"]
mod gui_tab_drawing_right_b;
#[path = "gui_tab_drawing_scale.rs"]
mod gui_tab_drawing_scale;
#[path = "gui_tab_drawing_sheet.rs"]
mod gui_tab_drawing_sheet;
#[path = "gui_tab_drawing_sheets.rs"]
pub(crate) mod gui_tab_drawing_sheets;
#[path = "gui_tab_drawing_toolbar.rs"]
mod gui_tab_drawing_toolbar;
#[path = "sheet_coord_points.rs"]
pub(crate) mod sheet_coord_points;
#[path = "sheet_pdf_file.rs"]
mod sheet_pdf_file;
#[path = "sheet_pdf_source.rs"]
mod sheet_pdf_source;
#[path = "sheet_raster.rs"]
pub(crate) mod sheet_raster;
#[path = "sheet_scene.rs"]
pub(crate) mod sheet_scene;

impl StrataAppState {
    /// C++ `KaDrawingStudio.cpp` 도면 스튜디오 (좌측 레이어/파일함 + 중앙 도면 용지 & 하단 7버튼 툴바 + 우측 도면 항목/방위/도면 정보) 1:1 재현
    pub(crate) fn render_drawing_tab_ui(&mut self, ui: &mut egui::Ui) {
        if ui.available_width()<1100.0 {
            eframe::egui::TopBottomPanel::top("drawing_compact_layers").frame(eframe::egui::Frame::NONE).exact_height(36.0)
                .show_inside(ui,|ui| self.paint_compact_layer_menu(ui,true));
        } else { self.render_cpp_left_layer_panel(ui, true); }
        self.paint_drawing_right_panel(ui);
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.paint_drawing_sheet(ui);
        });
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_tests.rs"]
mod tests;
