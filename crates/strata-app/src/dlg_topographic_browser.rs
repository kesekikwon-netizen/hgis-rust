//! 수치지형도 가져오기 브라우저 (`KaTopographicBrowser.cpp:117-260`).
//!
//! `dialogs_download_print.rs` 가 300줄을 넘어 이 파일로 옮겼고, 다시 300줄 규칙에 맞춰
//! 단계 띠(`dlg_topo_browser_rail`)와 본문(`dlg_topo_browser_body`)을 나눴다.
//! `crate::dlg_topographic_browser::render_topographic_browser` 경로와 위젯 순서는 그대로다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_download_print::render_ka_download_header;
use crate::dialogs_modal::ActiveModalDialog;
use crate::dialogs_modal::render_strata_modal_header;

#[path = "dlg_topo_browser_body.rs"]
mod dlg_topo_browser_body;
#[path = "dlg_topo_browser_rail.rs"]
mod dlg_topo_browser_rail;

use dlg_topo_browser_body::render_topo_browser_body;
use dlg_topo_browser_rail::paint_stage_rail;

// C++ KaTopographicBrowser.cpp:117-260 — "수치지형도 내려받기"
// 6-step KaStageRail, compact/expanded toggle, NGII login form, downloads table.
const TOPO_STAGES: [&str; 6] = [
    "도엽 확인",
    "로그인",
    "신청",
    "파일 준비",
    "다운로드",
    "지도에 올리기",
];

pub(crate) fn render_topographic_browser(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let is_open = app.modals.active == Some(ActiveModalDialog::NgiiTopoDownload);
    if !is_open {
        return;
    }
    app.poll_topographic_download(ctx);
    let stage = app.modals.topo_browser_stage.min(5);
    let detail = app.modals.topo_browser_detail;

    egui::Window::new("ka_modal_topo_browser")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(620.0)
        .show(ctx, |ui| {
            // KaTopographicBrowser.cpp:120 setWindowTitle("수치지형도 내려받기")
            render_strata_modal_header(ui, "수치지형도 내려받기 - Strata", open);
            // KaTopographicBrowser.cpp:126-127
            render_ka_download_header(
                ui,
                "수치지형도 다운로드",
                "국토정보플랫폼 · 조사 주변 도엽을 확인하고 지도 자료를 받습니다.",
                588.0,
            );

            ui.add_space(8.0);
            // KaStageRail.cpp — 6단계 원형 배지 + 연결선
            // (완료 ●테라코타, 현재 ◉강조 테두리, 대기 ○회색)
            // KaStageRail.cpp — 6단계 원형 배지 + 연결선
            paint_stage_rail(ui, &TOPO_STAGES, stage);
            ui.label(
                egui::RichText::new(TOPO_STAGES.join(" → "))
                    .size(10.5)
                    .color(Color32::from_rgb(120, 118, 110)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "[{}/6] {}",
                    stage + 1,
                    TOPO_STAGES[stage]
                ))
                .size(14.5)
                .strong(),
            );

            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(556.0);
                    // KaTopographicBrowser.cpp:152 hint
                    ui.label(
                        egui::RichText::new(
                            "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요.",
                        )
                        .size(12.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.add_space(6.0);
                    // 1단계 「도엽 확인」은 로그인 없이 실제 HTTP 로 된다
                    // (`ngii-topographic.js:43,63` → `combineBufferQuery.do` → `suchiQuery.do`).
                    crate::ngii_topographic_ui::render_sheet_lookup(app, ui, ctx);
                    ui.add_space(6.0);
                    // 진행 로그
                    for line in app.modals.topo_browser_log.iter().rev().take(3) {
                        ui.label(
                            egui::RichText::new(format!("· {line}"))
                                .size(11.5)
                                .color(Color32::from_rgb(110, 108, 102)),
                        );
                    }
                });
            render_topo_browser_body(app, ui, stage, detail, &TOPO_STAGES);

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                // KaTopographicBrowser.cpp:206 expand toggle
                let toggle_label = if detail { "상세 접기" } else { "상세 보기" };
                let tb = egui::Button::new(
                    egui::RichText::new(toggle_label)
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(96.0, 32.0));
                if ui.add(tb).clicked() {
                    app.modals.topo_browser_detail = !detail;
                }
                let step_b = egui::Button::new(egui::RichText::new("다음 단계").size(13.0))
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(96.0, 32.0));
                if ui.add(step_b).clicked() {
                    let next = (stage + 1).min(5);
                    app.modals.topo_browser_stage = next;
                    app.modals
                        .topo_browser_log
                        .push(format!("단계 진행 — {}", TOPO_STAGES[next]));
                    if next == 5 {
                        app.modals.active = Some(ActiveModalDialog::TopographicImport);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cb = egui::Button::new(egui::RichText::new("취소").size(13.0))
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(62.0, 32.0));
                    if ui.add(cb).clicked() {
                        app.modals.active = None;
                    }
                });
            });
        });
}
