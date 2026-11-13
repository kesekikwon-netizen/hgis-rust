//! 「주변유적 받기」 상세 로그와 하단 단추 — 상세 보기·설정·받기·취소.
//!
//! `dlg_heritage_browser.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 창 안의 상세 로그·하단 단추 줄을 **그대로** 옮겼고, 순서·문구·동작은
//! 하나도 바꾸지 않았다. 견본 결과 표의 「검색 결과 N건」 글은 F1 에서 표와 함께 없앴다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_download_print::heritage_start_fetch;
use crate::dialogs_modal::ActiveModalDialog;
use strata_net::heritage_intranet::HeritageStage;

/// 상세 로그(켜져 있을 때)와 하단 단추 줄.
pub(crate) fn render_heritage_footer(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
) {
if app.modals.heritage_show_details {
    ui.add_space(4.0);
    egui::Frame::NONE
        .fill(Color32::from_rgb(244, 242, 236))
        .stroke(Stroke::new(1.0, Color32::from_rgb(190, 188, 178)))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            for line in &app.modals.heritage_request_log {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .size(10.5)
                        .color(Color32::from_rgb(65, 64, 60)),
                );
            }
        });
}

ui.add_space(6.0);
ui.separator();
// 6) 하단 버튼 행: [상세 보기] [설정] ··· [받기] [취소]
ui.horizontal(|ui| {
    let detail_btn = egui::Button::new(
        egui::RichText::new("상세 보기")
            .color(Color32::WHITE)
            .strong(),
    )
    .fill(Color32::from_rgb(181, 87, 58))
    .corner_radius(6.0)
    .min_size(Vec2::new(84.0, 32.0));
    if ui.add(detail_btn).clicked() {
        app.modals.heritage_show_details = !app.modals.heritage_show_details;
    }
    if ui.button("설정").clicked() {
        app.modals.active = Some(ActiveModalDialog::AccountSettings);
    }

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let cancel_btn = egui::Button::new("취소")
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
            .corner_radius(6.0)
            .min_size(Vec2::new(68.0, 32.0));
        if ui.add(cancel_btn).clicked() {
            app.modals.active = None;
        }
        let fetch_btn = egui::Button::new(
            egui::RichText::new("받기")
                .color(Color32::WHITE)
                .strong()
                .size(13.5),
        )
        .fill(Color32::from_rgb(33, 33, 31))
        .corner_radius(6.0)
        .min_size(Vec2::new(96.0, 32.0));
        if ui.add(fetch_btn).clicked() {
            // KaHeritageBrowser.cpp:571-629 — 막는 조건은 요청 전에 문구로 끝낸다.
            if !app.accounts.heritage_configured {
                app.modals.heritage_stage_idx =
                    HeritageStage::Failed.ui_index();
                app.modals.heritage_progress_pct = 0.0;
                app.modals.heritage_status_detail =
                    strata_net::heritage_intranet::messages::NO_ACCOUNT
                        .to_owned();
            } else {
                // 체크된 자료 종류를 지도 레이어 가시성에 반영한다.
                for (di, lname) in [
                    "국가등록문화유산",
                    "국가지정유산",
                    "국가지정유산보호구역",
                    "시도지정유산",
                    "시도지정유산보호구역",
                ]
                .into_iter()
                .enumerate()
                {
                    app.home_map.real_survey.layer_visible.insert(
                        lname.to_owned(),
                        app.modals.heritage_dataset_checks[di],
                    );
                }
                // No pledge receipt here: cpp writes one only after the intranet answers
                // status=agreed with the terms text (target/cpp-ref/src/app/MainWindow.cpp:3227-3244,
                // KaHeritageBrowser.cpp:988-993). The Rust HTTP flow has no agree step, so nothing is written.
                // 실제 인트라넷 작업: 화면 → 목록 → 폼 → 시·군 검색 → ZIP
                heritage_start_fetch(app);
            }
        }
    });
});
}

#[cfg(test)]
#[path = "dlg_heritage_browser_footer_tests.rs"]
mod tests;
