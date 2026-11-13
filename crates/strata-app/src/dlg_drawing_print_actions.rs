//! 「도면 인쇄」 대화상자 아래 단추 — 나눈 장 PDF 저장·인쇄·닫기.
//!
//! `dlg_drawing_print.rs` 가 300줄 규칙에 맞춰 나눈 파일이다 (`KaPrintDialog.cpp:193-204`).
//! 원래 아래쪽 단추 줄과 그 앞의 장 수 계산을 **그대로** 옮겼고, 순서·문구·수치는
//! 하나도 바꾸지 않았다. 장 수는 원문과 똑같이 여기서 다시 계산한다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;

/// Tooltip of a print feature not yet ported (rules_v1 deferred-control text).
const DEFERRED_W50: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)";
use strata_layout::tile_print::{
    compute_tile_print_plan_with_overlap, format_print_dispatched_message,
    verify_requested_device_paper,
};

/// 용지·겹침으로 장 수를 다시 센다(원문과 같은 식).
fn print_page_count(app: &StrataAppState) -> u32 {
    let tile_plan = compute_tile_print_plan_with_overlap(
        &app.drawing.sheet,
        app.modals.print_paper,
        app.modals.print_overlap_mm,
    );
    if app.modals.print_single_sheet {
        1
    } else {
        (tile_plan.rows * tile_plan.cols).max(2)
    }
}

/// 아래 단추 줄 — PDF 저장 · 인쇄 · 닫기.
pub(crate) fn render_print_actions(app: &mut StrataAppState, ui: &mut egui::Ui) {
    let page_cnt = print_page_count(app);
            // KaPrintDialog.cpp:193-204 — 「나눈 장을 PDF로 저장…」은 왼쪽, 인쇄·닫기는 오른쪽
            ui.horizontal(|ui| {
                let save_pdf =
                    egui::Button::new(egui::RichText::new("나눈 장을 PDF로 저장…").size(12.5))
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(150.0, 34.0));
                // Deferred until W-50: cpp saves the real tiled PDF (KaPrintDialog.cpp:592-620);
                // the Rust writer drew empty rectangles.
                ui.add_enabled(false, save_pdf).on_disabled_hover_text(DEFERRED_W50);
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let close_b = egui::Button::new("닫기")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(58.0, 34.0));
                        if ui.add(close_b).clicked() {
                            app.modals.active = None;
                        }
                        let prt = egui::Button::new(
                            egui::RichText::new(format!("인쇄 · {page_cnt}장"))
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(96.0, 34.0));
                        if ui.add(prt).clicked() {
                            let ori = app.drawing.sheet.orientation;
                            if verify_requested_device_paper(
                                app.modals.print_paper,
                                ori,
                                app.modals.print_paper,
                                ori,
                            )
                            .is_ok()
                            {
                                app.home_map.draw_guide_message =
                                    Some(format_print_dispatched_message(
                                        app.modals.print_paper,
                                        ori,
                                        page_cnt,
                                    ));
                            }
                            app.modals.active = None;
                        }
                    },
                );
            });
}

#[cfg(test)]
#[path = "dlg_drawing_print_actions_tests.rs"]
mod tests;
