//! `dialogs_download_print` 에서 옮긴 「HeritageRegionSelect」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_download_print::render_ka_download_header;
use crate::dialogs_modal::ActiveModalDialog;

pub(crate) fn render_heritage_region_select(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_heritage_bg")
        .title_bar(false)
        .resizable(false)
        .order(egui::Order::Middle)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 22.0])
        .default_width(600.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "주변유적 다운로드 - Strata", open);
            render_ka_download_header(
                ui,
                "주변유적 다운로드",
                "국가유산 인트라넷 · 조사구역 주변 유적을 받습니다.",
                568.0,
            );
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(568.0);
                    ui.label(egui::RichText::new("준비 · 시·군 선택").size(14.5).strong());
                    ui.label(
                        egui::RichText::new("조사구역이 속한 시·군을 확인하고 있습니다.")
                            .size(13.5),
                    );
                    let (bar_r, _) = ui
                        .allocate_exact_size(Vec2::new(552.0, 26.0), egui::Sense::hover());
                    ui.painter().rect_stroke(
                        bar_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                        egui::StrokeKind::Middle,
                    );
                });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let _ = ui.add(
                    egui::Button::new(
                        egui::RichText::new("상세 보기")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(88.0, 34.0)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let _ = ui.add(
                        egui::Button::new("취소")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(62.0, 34.0)),
                    );
                });
            });
        });
    egui::Window::new("ka_modal_heritage_region")
        .title_bar(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [-12.0, 4.0])
        .default_width(480.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "주변유적 받을 시·군 선택 - Strata", open);
            ui.label(egui::RichText::new("조사구역이 속한 시·군을 확인하세요.\n자료는 선택한 시·군 단위로만 받습니다.").size(14.5).color(Color32::from_rgb(35, 35, 32)));
            ui.add_space(12.0);
            // KaHeritageSetupDialogs.cpp:99-107,144-166 — 시·도를 고르면 시·군 목록이 그것만 남는다.
            // KoreaRegionCatalog 가 정본이고, 둘 다 골라야 「이 지역 받기」가 열린다(같은 파일 :149-152).
            ui.horizontal(|ui| {
                ui.add_sized([88.0, 32.0], egui::Label::new(egui::RichText::new("시·도").size(14.0).color(Color32::from_rgb(80, 79, 74))));
                egui::ComboBox::from_id_salt("ka_heritage_region_sido")
                    .selected_text(if app.modals.heritage_sido.is_empty() {
                        "시·도 선택".to_owned()
                    } else {
                        app.modals.heritage_sido.clone()
                    })
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        let sido_list: Vec<String> =
                            strata_net::address_search::KoreaRegionCatalog::all_sido()
                                .iter()
                                .map(|s| s.name.to_owned())
                                .collect();
                        for name in sido_list {
                            if ui
                                .selectable_label(app.modals.heritage_sido == name, &name)
                                .clicked()
                            {
                                app.modals.heritage_sido = name;
                                // 시·군이 새 시·도에 없으면 비운다.
                                if !strata_net::address_search::KoreaRegionCatalog::cities_of(
                                    &app.modals.heritage_sido,
                                )
                                .contains(&app.modals.heritage_city)
                                {
                                    app.modals.heritage_city.clear();
                                }
                            }
                        }
                    });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_sized([88.0, 32.0], egui::Label::new(egui::RichText::new("시·군·구").size(14.0).color(Color32::from_rgb(80, 79, 74))));
                egui::ComboBox::from_id_salt("ka_heritage_region_city")
                    .selected_text(if app.modals.heritage_city.is_empty() {
                        "시·군·구 선택".to_owned()
                    } else {
                        app.modals.heritage_city.clone()
                    })
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for name in strata_net::address_search::KoreaRegionCatalog::cities_of(
                            &app.modals.heritage_sido,
                        ) {
                            if ui
                                .selectable_label(app.modals.heritage_city == name, &name)
                                .clicked()
                            {
                                app.modals.heritage_city = name;
                            }
                        }
                    });
            });
            ui.add_space(6.0);
            let mut reuse_recent = app.modals.heritage_reuse_recent;
            ui.checkbox(&mut reuse_recent, egui::RichText::new("최근 받은 자료 다시 쓰기").size(14.0));
            app.modals.heritage_reuse_recent = reuse_recent;
            ui.add_space(8.0);
            ui.label(egui::RichText::new("받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고,\n동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는\n동국문화재연구원 소유입니다. 도면·보고서에 넣거나 밖으로 보내기 전에 서약 범위를\n확인하세요.").size(12.5).color(Color32::from_rgb(75, 74, 70)));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_b = egui::Button::new(egui::RichText::new("취소").size(14.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134))).corner_radius(6.0).min_size(Vec2::new(66.0, 36.0));
                if ui.add(cancel_b).clicked() {
                    app.modals.active = None;
                }
                // :149-152 — 시·도와 시·군·구를 다 고르기 전에는 열리지 않는다.
                let ready = !app.modals.heritage_sido.is_empty()
                    && !app.modals.heritage_city.is_empty();
                let ok_b = egui::Button::new(egui::RichText::new("이 지역 받기").color(Color32::WHITE).strong().size(14.0)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(116.0, 36.0));
                if ui.add_enabled(ready, ok_b).clicked() {
                    app.modals.active = Some(ActiveModalDialog::HeritageBrowser);
                }
            });
        });
}
