//! DEM 내려받기 창 그리기 (`ActiveModalDialog::DemDownload`).
//!
//! `dem_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Stroke, Vec2};
use strata_net::dem_copernicus;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

use super::dem_download_ui_state::{dem_state, poll, start_download};

/// `ActiveModalDialog::DemDownload` 창 (C++ `startDemDownload` + `DemDownloadService::prepare`)
pub fn render_dem_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    poll(app, ctx);
    let mut start = false;
    let mut cancel = false;
    let mut close = false;
    let mut state = match dem_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    egui::Window::new("ka_modal_dem_download")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(510.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "국토지리정보원 DEM 다운로드 - Strata", open);
            ui.label(
                egui::RichText::new(
                    "조사구역 주변 공개 DEM 30m(Copernicus)를 받아 조사 폴더 DEM/원본 에 한 장으로 저장합니다.",
                )
                .size(14.0)
                .color(Color32::from_rgb(25, 25, 24)),
            );
            ui.add_space(10.0);
            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(480.0, 32.0), egui::Sense::hover());
            let painter = ui.painter_at(bar_rect);
            painter.rect_filled(bar_rect, 6.0, Color32::WHITE);
            painter.rect_stroke(
                bar_rect,
                6.0,
                Stroke::new(1.2, Color32::from_rgb(150, 148, 142)),
                egui::StrokeKind::Middle,
            );
            let fraction = state.progress.clamp(0.0, 1.0);
            if fraction > 0.0 {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        bar_rect.min + Vec2::new(2.0, 2.0),
                        Vec2::new(476.0 * fraction, 28.0),
                    ),
                    5.0,
                    Color32::from_rgb(20, 20, 20),
                );
            }
            let pct = (fraction * 100.0).round() as u32;
            painter.text(
                bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{pct}%"),
                egui::FontId::proportional(12.5),
                Color32::from_rgb(55, 54, 50),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&state.status)
                    .size(13.0)
                    .color(Color32::from_rgb(60, 59, 55)),
            );
            if state.tile_total > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "받을 칸 {}개 (1° 격자) · 한 번에 최대 {}칸",
                        state.tile_total,
                        dem_copernicus::DEM_MAXIMUM_TILES
                    ))
                    .size(12.0)
                    .color(Color32::from_rgb(95, 94, 88)),
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let start_btn = egui::Button::new(
                    egui::RichText::new(if state.result.is_some() {
                        "다시 받기"
                    } else {
                        "DEM 받기"
                    })
                    .color(Color32::WHITE)
                    .strong()
                    .size(13.5),
                )
                .fill(Color32::from_rgb(44, 43, 41))
                .corner_radius(6.0)
                .min_size(Vec2::new(132.0, 36.0));
                if ui
                    .add_enabled(!state.running, start_btn)
                    .on_hover_text("지금 화면 범위를 덮는 1° 공개 DEM 타일을 받습니다.")
                    .clicked()
                {
                    start = true;
                }
                if ui
                    .add_enabled(state.running, egui::Button::new("내려받기 취소"))
                    .clicked()
                {
                    cancel = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // C++ `MainWindowDownloads.cpp:88` `QProgressDialog::canceled` → 창 닫기
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("닫기")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(14.0),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(72.0, 36.0)),
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
        state.status = "DEM 내려받기를 취소하는 중입니다.".to_owned();
    }
    if close {
        // 닫아도 진행 중인 작업은 취소한다 (C++ 취소 버튼과 같은 결과).
        state.cancel.store(true, Ordering::Relaxed);
        *open = false;
    }
}
