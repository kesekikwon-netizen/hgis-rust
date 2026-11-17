//! `dialogs_download_print` 에서 옮긴 「HeritageRegionSelect」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

pub(crate) fn render_heritage_region_select(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_heritage_region")
        .title_bar(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [-12.0, 4.0])
        .default_width(480.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "주변유적 받을 시·군 선택 - Strata", open);
            ui.label(egui::RichText::new(app.heritage_region_words()).size(14.5).color(Color32::from_rgb(35, 35, 32)));
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
            // KaHeritageSetupDialogs.cpp:230-254 — 같은 시·군을 최근 30일 안에 받아 둔 자료가 있을 때만 보인다.
            // 자동으로 쓰지 않고 사용자가 고른다.
            let (sido, city) = (app.modals.heritage_sido.clone(), app.modals.heritage_city.clone());
            match app.heritage_reuse_when(&sido, &city) {
                Some(received) => {
                    ui.checkbox(&mut app.modals.heritage_reuse_recent, egui::RichText::new("최근 받은 자료 다시 쓰기").size(14.0));
                    ui.label(egui::RichText::new(strata_refmap::heritage_recent::reuse_note(received)).size(12.5).color(Color32::from_rgb(110, 108, 100)));
                }
                None => app.modals.heritage_reuse_recent = false,
            }
            ui.add_space(8.0);
            ui.label(egui::RichText::new("받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고,\n동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는\n동국문화재연구원 소유입니다. 도면·보고서에 넣거나 밖으로 보내기 전에 서약 범위를\n확인하세요.").size(12.5).color(Color32::from_rgb(75, 74, 70)));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_b = egui::Button::new(egui::RichText::new("취소").size(14.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134))).corner_radius(6.0).min_size(Vec2::new(66.0, 36.0));
                if ui.add(cancel_b).clicked() {
                    app.modals.active = None;
                    // MainWindow.cpp:3157
                    app.show_status_message("취소했습니다.");
                }
                // :149-152 — 시·도와 시·군·구를 다 고르기 전에는 열리지 않는다.
                let ready = !app.modals.heritage_sido.is_empty()
                    && !app.modals.heritage_city.is_empty();
                let ok_b = egui::Button::new(egui::RichText::new("이 지역 받기").color(Color32::WHITE).strong().size(14.0)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(116.0, 36.0));
                if ui.add_enabled(ready, ok_b).clicked() {
                    app.modals.active = None;
                    if app.modals.heritage_reuse_recent {
                        // 「최근 받은 자료 다시 쓰기」: the archives already on disk go back on the
                        // map through the same loader, in a thread (`heritage_run.rs`).
                        app.heritage.wanted = Some(crate::heritage_run::reuse::Wanted { sido, city, reuse: true });
                    } else {
                        // The C++ goes to the intranet here (`MainWindow.cpp:3071-3225`); the run
                        // is `strata-heritage-run`, started from the frame loop where the window
                        // handle exists (`heritage_run_start.rs`).
                        crate::heritage_run_start::start_heritage_fetch(app);
                    }
                }
            });
        });
}
