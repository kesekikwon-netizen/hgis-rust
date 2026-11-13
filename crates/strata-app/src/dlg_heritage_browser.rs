//! `dialogs_download_print` 에서 옮긴 「HeritageBrowser」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.
//! 300줄 규칙에 맞춰 진행 카드(`dlg_heritage_browser_progress`)와
//! 상세 로그·하단 단추(`dlg_heritage_browser_footer`)로 나눴다. 견본 유적 결과 표는 F1 에서 없앴다(C++ 에 없음).
//! `crate::dlg_heritage_browser::render_heritage_browser` 경로와 위젯 순서는 그대로다.

use eframe::egui::{self, Color32, Pos2, Stroke};

use crate::app_state::StrataAppState;
use crate::dialogs_download_print::render_ka_download_header;
use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_modal::ActiveModalDialog;

#[path = "dlg_heritage_browser_footer.rs"]
mod dlg_heritage_browser_footer;
#[path = "dlg_heritage_browser_progress.rs"]
mod dlg_heritage_browser_progress;

use dlg_heritage_browser_footer::render_heritage_footer;
use dlg_heritage_browser_progress::render_heritage_progress;

pub(crate) fn render_heritage_browser(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
) {
    // KaHeritageBrowser.cpp:203-295, KaHeritageSetupDialogs.cpp:81-200, HeritagePledge.cpp:41-73
    // 창 크기를 정할 때 이미 DPI를 반영한 논리 픽셀을 쓴다. 여기서 다시 나누면
    // 150% 화면에서 대화상자가 2/3 크기로 줄어 글자가 넘친다.
    let full = ctx.screen_rect();
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 940.0_f32.min(vis_w - 32.0);
    let modal_h = 620.0_f32.min(vis_h - 32.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_heritage_browser")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 24.0);
            ui.set_max_width(modal_w - 24.0);
            render_strata_modal_header(ui, "주변유적 받기", open);
            render_ka_download_header(
                ui,
                "주변유적 다운로드",
                "국가유산 공간정보 인트라넷 · 선택한 시·군의 자료를 순서대로 받습니다.",
                modal_w - 48.0,
            );

            // 1) 계정 설정 상태 안내 (KaHeritageAccountDialog)
            if !app.accounts.heritage_configured {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(254, 242, 242))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 28, 28)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                Color32::from_rgb(185, 28, 28),
                                egui::RichText::new(
                                    "▲ 국가유산 인트라넷 아이디·비밀번호가 설정되지 않았습니다. [설정]을 눌러 계정을 입력해 주세요.",
                                )
                                .size(12.0)
                                .strong(),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("설정").clicked() {
                                    app.modals.active = Some(ActiveModalDialog::AccountSettings);
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }

            // 2) 시·군 및 검색 범위 선택 ( 반경 1 / 3 / 5 / 10 km 또는 현재 지도 화면 기준 )
            egui::Frame::NONE
                .fill(Color32::from_rgb(248, 247, 242))
                .stroke(Stroke::new(1.0, Color32::from_rgb(205, 202, 192)))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("시·도").size(12.0).strong());
                        egui::ComboBox::from_id_salt("ka_hb_sido")
                            .selected_text(&app.modals.heritage_sido)
                            .width(110.0)
                            .show_ui(ui, |ui| {
                                for s in strata_net::address_search::KoreaRegionCatalog::all_sido() {
                                    ui.selectable_value(
                                        &mut app.modals.heritage_sido,
                                        s.name.to_owned(),
                                        s.name,
                                    );
                                }
                            });
                        ui.label(egui::RichText::new("시·군·구").size(12.0).strong());
                        egui::ComboBox::from_id_salt("ka_hb_city")
                            .selected_text(&app.modals.heritage_city)
                            .width(96.0)
                            .show_ui(ui, |ui| {
                                for c in strata_net::address_search::KoreaRegionCatalog::cities_of(
                                    &app.modals.heritage_sido,
                                ) {
                                    ui.selectable_value(
                                        &mut app.modals.heritage_city,
                                        c.clone(),
                                        &c,
                                    );
                                }
                            });
                        ui.separator();
                        ui.checkbox(
                            &mut app.modals.heritage_reuse_recent,
                            egui::RichText::new("최근 받은 자료 다시 쓰기").size(11.5),
                        );
                    });

                    ui.add_space(4.0);
                    // 3) 받을 항목 체크 목록 (7종)
                    let ds_names = [
                        ("국가등록문화유산", Color32::from_rgb(0x16, 0xA3, 0x4A)),
                        ("국가지정유산", Color32::from_rgb(0x8E, 0x44, 0xAD)),
                        ("국가지정유산보호구역", Color32::from_rgb(0x7C, 0x3A, 0xED)),
                        ("시도지정유산", Color32::from_rgb(0xC2, 0x18, 0x7D)),
                        ("시도지정유산보호구역", Color32::from_rgb(0xE6, 0x7E, 0x22)),
                        ("매장유산유존지역", Color32::from_rgb(0x2E, 0x86, 0xC1)),
                        ("문화유적분포지도", Color32::from_rgb(0x27, 0xAE, 0x60)),
                    ];
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("받을 항목:").size(12.0).strong());
                        for (di, (d_name, d_col)) in ds_names.into_iter().enumerate() {
                            ui.checkbox(
                                &mut app.modals.heritage_dataset_checks[di],
                                egui::RichText::new(d_name).size(11.5).color(d_col).strong(),
                            );
                        }
                    });
                });

            render_heritage_progress(app, ui, modal_w);


            // F1: C++ 창에는 유적 결과 표가 없다(받은 자료는 지도에 올라간다) — 견본 유적 표를 없앴다.

            render_heritage_footer(app, ui);
        });
}

#[cfg(test)]
#[path = "dlg_heritage_browser_tests.rs"]
mod tests;
