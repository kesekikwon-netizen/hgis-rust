//! 지도 탭 본문 — 좌우 패널·중앙 캔버스·HUD 구간 호출.

use eframe::egui::{self, Color32};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::right_inspector::InspectorCardTab;

use super::gui_tab_map_canvas::MapFrameInput;
use super::gui_tab_map_consts::UX_MAP_CANVAS_WIDTH;

impl StrataAppState {
    pub(crate) fn render_map_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let tab_clip = ui.max_rect();
        ui.set_clip_rect(tab_clip);
        let compact=ui.available_width()<1100.0 && !self.align_split_shown();
        if self.align_split_shown() {
            self.render_align_source(ui);
        } else if !compact {
            let _perf = crate::perf_recorder::begin_part(crate::perf_recorder::PerfPart::LeftPanel);
            self.render_cpp_left_layer_panel(ui, false);
        }
        if !self.align_split_shown() && !compact {
            let _perf =
                crate::perf_recorder::begin_part(crate::perf_recorder::PerfPart::RightPanel);
            self.render_cpp_right_inspector(ui);
        }
        if compact && !self.right_inspector.map_focused {
            egui::TopBottomPanel::top("compact_map_panels").frame(egui::Frame::NONE).exact_height(36.0).show_inside(ui,|ui| {
                ui.horizontal(|ui| {
                    self.paint_compact_layer_menu(ui,false);
                    crate::gui_shell::native_menu_button(ui, "선택한 유구",|ui| {
                        ui.set_width(280.0);
                        let height=(ui.ctx().screen_rect().height()-160.0).clamp(100.0,440.0);
                        egui::ScrollArea::vertical().max_height(height).show(ui,|ui| self.render_ri_body(ui));
                    });
                });
            });
        }

        let win_w = ctx.screen_rect().width();
        let avail_center_w = ui.available_width();
        let right_pad =
            if (1550.0..=1620.0).contains(&win_w) && avail_center_w > UX_MAP_CANVAS_WIDTH + 9.0 {
                ((avail_center_w - 9.0 - UX_MAP_CANVAS_WIDTH).round() as i8).max(4)
            } else {
                9
            };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(243, 241, 232))
                    .inner_margin(egui::Margin {
                        left: 9,
                        right: right_pad,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show_inside(ui, |ui| {
            let is_trench = self.modals.active == Some(ActiveModalDialog::TrenchGenerator)
                || self.active_tool == MapToolKind::TrenchPlace;
            let is_measure = self.active_tool == MapToolKind::Measure;
            // C++ 안내 띠(KaDrawGuideBand)는 그리기 도구가 실제로 켜졌을 때만 — 줄만 열면 도구는 이동이다.
            let is_draw_tool = matches!(
                self.active_tool,
                MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
            );
            let is_select_tool = self.active_tool == MapToolKind::Select;
            let is_style = self.right_inspector.active_card == InspectorCardTab::LayerStyleCard;

                self.paint_map_canvas(
                    ctx,
                    ui,
                    tab_clip,
                    MapFrameInput {
                        is_trench,
                        is_measure,
                        is_draw_tool,
                        is_select_tool,
                        is_style,
                    },
                );
            });
        // 방금 그린 지도 칸 크기로 한반도 전체 보기를 한 번 맞춘다(C++ ensureStartupViewReady).
        self.paint_align_links(ctx);
        self.home_map.apply_pending_korea_view(self.work_crs);
        self.apply_pending_survey_view();
    }
}
