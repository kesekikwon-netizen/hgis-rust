//! `dialogs_modal` 에서 옮긴 대화상자 묶음: DemDownload, PaleoSourceDownload.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use crate::dialogs_modal::render_strata_modal_header;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Vec2};


pub(crate) fn render_dem_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // C++ `MainWindowDownloads.cpp:123-197` `startDemDownload` — 조사 폴더
    // `DEM/원본` 에 Copernicus 30m 타일을 실제로 받는다. 타일 규칙·상한·404
    // 건너뛰기는 `DemDownloadService.cpp:85-305` 를 `strata-net::dem_copernicus`
    // 가 그대로 옮겼다.
    crate::dem_download_ui::render_dem_download(app, ctx, open);
}

pub(crate) fn render_paleo_source_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaPaleoStudio / MainWindowPaleo.cpp:115-173 (commit ceb5fd2)
    // 창 520x250, 반경 ComboBox 3항목(2/5/10 km, 기본 5), 3개 개별 버튼 순서 고정.
    egui::Window::new("ka_modal_paleo_sources")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([520.0, 250.0])
        .show(ctx, |ui| {
            render_strata_modal_header(
                ui,
                "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 (도면 범례 지원)",
                open,
            );
            // MainWindowPaleo.cpp:124-129 — QComboBox 3항목, 기본 index 1 (5 km)
            const RADIUS_LABELS: [&str; 3] = [
                "반경 2 km (2,000 m)",
                "반경 5 km (5,000 m · 권장)",
                "반경 10 km (10,000 m)",
            ];
            const RADIUS_KM: [f64; 3] = [2.0, 5.0, 10.0];
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("조사구역 중심 수집 반경:").size(12.5));
                let cur_idx = RADIUS_KM
                    .iter()
                    .position(|k| (k - app.paleo_3d.source_dl_radius_km).abs() < 0.01)
                    .unwrap_or(1);
                egui::ComboBox::from_id_salt("ka_paleo_radius_combo")
                    .selected_text(RADIUS_LABELS[cur_idx])
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        for (i, lbl) in RADIUS_LABELS.iter().enumerate() {
                            if ui
                                .selectable_label(cur_idx == i, *lbl)
                                .clicked()
                            {
                                app.paleo_3d.source_dl_radius_km = RADIUS_KM[i];
                            }
                        }
                    });
            });
            ui.add_space(4.0);
            // MainWindowPaleo.cpp:143-146 — 적재 현황
            ui.label(
                egui::RichText::new(format!(
                    "현재 적재 현황 — 수치지질도(1:5만): {}건 · 시추공(퇴적층두께): {}공 · KIGAM 수문·단구연대: 6건 · 단층: 3건",
                    app.paleo_3d.geology_feature_count,
                    app.paleo_3d.declustered_borehole_count
                ))
                .size(12.0),
            );
            ui.add_space(6.0);
            // MainWindowPaleo.cpp:154-157 — btnPrimary
            let btn_all = egui::Button::new(
                egui::RichText::new(
                    "수치지질도(1:5만) + 시추주상도(퇴적층) + KIGAM 연속 받기",
                )
                .color(Color32::WHITE)
                .strong()
                .size(13.0),
            )
            .fill(Color32::from_rgb(181, 87, 58))
            .corner_radius(6.0)
            .min_size(Vec2::new(488.0, 32.0));
            if ui.add(btn_all).clicked() {
                app.paleo_3d.run_sample_10km_analysis();
                app.home_map.draw_guide_message = Some(format!(
                    "반경 {:.0} km 수치지질도(1:5만)·시추주상도(디클러스터링 {}공)·KIGAM 수문·단층 자료를 적재했습니다.",
                    app.paleo_3d.source_dl_radius_km,
                    app.paleo_3d.declustered_borehole_count
                ));
                app.modals.active = None;
            }
            ui.add_space(4.0);
            // MainWindowPaleo.cpp:158-171 — 순서: 지질도 → 시추 → KIGAM
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("수치지질도(1:5만)만 받기").size(12.0))
                    .clicked()
                {
                    app.paleo_3d.run_sample_10km_analysis();
                    app.home_map.draw_guide_message = Some(format!(
                        "수치지질도(1:5만)만 반경 {:.0} km로 받았습니다.",
                        app.paleo_3d.source_dl_radius_km
                    ));
                    app.modals.active = None;
                }
                if ui
                    .button(egui::RichText::new("시추주상도·퇴적층만 받기").size(12.0))
                    .clicked()
                {
                    app.paleo_3d.run_sample_10km_analysis();
                    app.home_map.draw_guide_message = Some(format!(
                        "시추주상도·퇴적층만 반경 {:.0} km로 받았습니다.",
                        app.paleo_3d.source_dl_radius_km
                    ));
                    app.modals.active = None;
                }
                if ui
                    .button(egui::RichText::new("KIGAM 단층·수문만 받기").size(12.0))
                    .clicked()
                {
                    app.paleo_3d.run_sample_10km_analysis();
                    app.home_map.draw_guide_message = Some(format!(
                        "KIGAM 단층·수문만 반경 {:.0} km로 받았습니다.",
                        app.paleo_3d.source_dl_radius_km
                    ));
                    app.modals.active = None;
                }
            });
        });
}
