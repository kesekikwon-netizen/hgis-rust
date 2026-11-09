//! 지적 내려받기 창 그리기 (`ActiveModalDialog::CadastralDownload`).
//!
//! C++ `MainWindowCadastral.cpp:122-174` 1:1.
//! `cadastral_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};
use strata_net::vworld_client::VworldClient;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

use super::cadastral_download_ui_state::{cadastral_state, poll, start_download};

/// `ActiveModalDialog::CadastralDownload` 창 (C++ `MainWindowCadastral.cpp:122-174`)
pub fn render_cadastral_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    poll(app, ctx);
    let mut start = false;
    let mut cancel = false;
    let mut close = false;
    let key_present = !strata_secrets::vworld::VworldSettings::load_api_key()
        .trim()
        .is_empty();
    let mut state = match cadastral_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    egui::Window::new("ka_modal_cadastral")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(540.0)
        .show(ctx, |ui| {
            // C++ `MainWindowCadastral.cpp:122-123` 창 제목·부제
            render_strata_modal_header(ui, "지적도 다운로드 - Strata", open);
            ui.label(
                egui::RichText::new(VworldClient::CADASTRAL_DIALOG_TITLE)
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(25, 25, 24)),
            );
            ui.label(
                egui::RichText::new(VworldClient::CADASTRAL_DIALOG_SUBTITLE)
                    .size(12.5)
                    .color(Color32::from_rgb(95, 94, 88)),
            );
            ui.add_space(10.0);
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(508.0);
                    ui.label(
                        egui::RichText::new("자료 준비 및 다운로드")
                            .size(14.5)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&state.status).size(13.5));
                    // C++ `BasemapOps.cpp:1015` — 키가 없으면 같은 안내를 그대로 보인다.
                    if !key_present {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(VworldClient::NO_KEY_MESSAGE)
                                .size(12.5)
                                .color(Color32::from_rgb(181, 87, 58)),
                        );
                    }
                    ui.add_space(8.0);
                    let (bar_r, _) =
                        ui.allocate_exact_size(Vec2::new(492.0, 26.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 4.0, Color32::WHITE);
                    p.rect_stroke(
                        bar_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                        egui::StrokeKind::Middle,
                    );
                    let fraction = state.progress.clamp(0.0, 1.0);
                    if fraction > 0.0 {
                        p.rect_filled(
                            Rect::from_min_size(
                                bar_r.min + Vec2::new(1.0, 1.0),
                                Vec2::new((bar_r.width() - 2.0) * fraction, 24.0),
                            ),
                            3.0,
                            Color32::from_rgb(181, 87, 58),
                        );
                    }
                    let pct = (fraction * 100.0).round() as u32;
                    p.text(
                        bar_r.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{pct}%"),
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(35, 35, 32),
                    );
                    if !state.downloaded.is_empty() {
                        ui.add_space(6.0);
                        for path in state.downloaded.iter().take(4) {
                            ui.label(
                                egui::RichText::new(format!("· {path}"))
                                    .size(11.5)
                                    .color(Color32::from_rgb(110, 108, 102)),
                            );
                        }
                    }
                });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let start_btn = egui::Button::new(
                    egui::RichText::new("지적도 받기")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.5),
                )
                .fill(Color32::from_rgb(44, 43, 41))
                .corner_radius(6.0)
                .min_size(Vec2::new(124.0, 34.0));
                if ui
                    .add_enabled(!state.running, start_btn)
                    .on_hover_text("조사구역 주변 5km 시·군·구 지적도 ZIP 을 조사 폴더 지적도/원본 에 받습니다.")
                    .clicked()
                {
                    start = true;
                }
                if ui
                    .add_enabled(state.running, egui::Button::new("취소"))
                    .clicked()
                {
                    cancel = true;
                }
                if ui.button("계정 설정").clicked() {
                    app.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::AccountSettings);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("닫기")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.5),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(62.0, 34.0)),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });
    if start {
        start_download(app, &mut state);
    }
    if cancel {
        state.cancel.store(true, Ordering::Relaxed);
        state.status = "지적도 받기를 취소하는 중입니다.".to_owned();
    }
    if close {
        state.cancel.store(true, Ordering::Relaxed);
        *open = false;
    }
}
