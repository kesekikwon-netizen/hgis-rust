//! `dialogs_download_print` 에서 옮긴 「HeritageBrowser」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_download_print::render_ka_download_header;
use crate::dialogs_modal::ActiveModalDialog;
use crate::dialogs_download_print::heritage_start_fetch;
use strata_net::heritage_intranet::HeritageStage;

pub(crate) fn render_heritage_browser(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
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
                        ui.label(egui::RichText::new("범위 선택:").size(12.0).strong());
                        for (si, s_lbl) in [
                            "반경 1 km",
                            "반경 3 km",
                            "반경 5 km",
                            "반경 10 km",
                            "현재 지도 화면 기준",
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if ui
                                .selectable_label(app.modals.heritage_scope_idx == si, s_lbl)
                                .clicked()
                            {
                                app.modals.heritage_scope_idx = si;
                            }
                        }
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

            ui.add_space(6.0);
            // 4) 진행 상태 및 진행률 카드 (KaHeritageBrowser.cpp:211-229)
            let stage_enum = match app.modals.heritage_stage_idx {
                0 => strata_net::heritage_intranet::HeritageStage::Idle,
                1 => strata_net::heritage_intranet::HeritageStage::Login,
                2 => strata_net::heritage_intranet::HeritageStage::DismissTutorial,
                3 => strata_net::heritage_intranet::HeritageStage::OpenDownloadPage,
                4 => strata_net::heritage_intranet::HeritageStage::AgreeTerms,
                5 => strata_net::heritage_intranet::HeritageStage::SelectDataset,
                6 => strata_net::heritage_intranet::HeritageStage::SelectRegion,
                7 => strata_net::heritage_intranet::HeritageStage::Search,
                8 => strata_net::heritage_intranet::HeritageStage::CollectResults,
                9 => strata_net::heritage_intranet::HeritageStage::Download,
                11 => strata_net::heritage_intranet::HeritageStage::Failed,
                _ => strata_net::heritage_intranet::HeritageStage::Done,
            };
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "단계: {}",
                                stage_enum.stage_name_ko()
                            ))
                            .size(13.0)
                            .strong(),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(&app.modals.heritage_status_detail)
                                .size(12.0)
                                .color(Color32::from_rgb(55, 54, 50)),
                        );
                    });
                    ui.add_space(4.0);
                    let (bar_r, _) = ui.allocate_exact_size(
                        Vec2::new(modal_w - 52.0, 20.0),
                        egui::Sense::hover(),
                    );
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 4.0, Color32::from_rgb(238, 236, 228));
                    let fill_w = (bar_r.width() * app.modals.heritage_progress_pct.clamp(0.0, 1.0)).max(4.0);
                    p.rect_filled(
                        Rect::from_min_size(bar_r.min, Vec2::new(fill_w, bar_r.height())),
                        4.0,
                        Color32::from_rgb(181, 87, 58),
                    );
                    p.rect_stroke(
                        bar_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                        egui::StrokeKind::Inside,
                    );
                    p.text(
                        bar_r.center(),
                        egui::Align2::CENTER_CENTER,
                        format!(
                            "{:.0}% · {} {}",
                            app.modals.heritage_progress_pct * 100.0,
                            app.modals.heritage_sido,
                            app.modals.heritage_city
                        ),
                        egui::FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                    ui.add_space(3.0);
                    // HeritagePledge::preDisclosure()
                    ui.label(
                        egui::RichText::new(
                            "받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고, 동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는 동국문화재연구원 소유입니다.",
                        )
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                });

            ui.add_space(6.0);
            // 5) 결과 표 (유적명 · 종류 · 거리 · 지정번호 · 위치 이동)
            let max_dist_m: f64 = match app.modals.heritage_scope_idx {
                0 => 1000.0,
                1 => 3000.0,
                2 => 5000.0,
                3 => 10000.0,
                _ => 8000.0,
            };
            let [cx, cy] = app.home_map.real_survey.survey_center;
            let catalog = [
                ("경주 월성 (반월성)", "국가지정유산", 1, 145.0, "사적 제16호", [cx - 95.0, cy - 110.0]),
                ("경주 동궁과 월지 (안압지)", "국가지정유산", 1, 310.0, "사적 제18호", [cx + 220.0, cy + 185.0]),
                ("경주 첨성대", "국가지정유산", 1, 420.0, "국보 제31호", [cx - 320.0, cy + 160.0]),
                ("경주 월성 보호구역", "국가지정유산보호구역", 2, 190.0, "보호구역 제16-1호", [cx - 120.0, cy - 85.0]),
                ("경주 황룡사지", "국가지정유산", 1, 880.0, "사적 제6호", [cx + 680.0, cy + 410.0]),
                ("경주 인왕동 고분군", "시도지정유산", 3, 540.0, "기념물 제34호", [cx + 140.0, cy - 380.0]),
                ("경주 인왕동 보호구역", "시도지정유산보호구역", 4, 620.0, "시도보호 제34-1호", [cx + 180.0, cy - 410.0]),
                ("경주 인왕동·교동 매장유산 포괄보호구역", "매장유산유존지역", 5, 85.0, "매장유산 제47130-01호", [cx + 45.0, cy + 60.0]),
                ("경주 월성북편 문화유적분포지도 구역", "문화유적분포지도", 6, 110.0, "분포지도 제103-04호", [cx - 40.0, cy + 95.0]),
                ("구 경주역사 및 급수탑", "국가등록문화유산", 0, 1420.0, "등록문화유산 제292호", [cx - 980.0, cy + 790.0]),
            ];
            let filtered_rows: Vec<_> = catalog
                .into_iter()
                .filter(|(_, _, d_idx, dist_m, _, _)| {
                    app.modals.heritage_dataset_checks[*d_idx] && *dist_m <= max_dist_m
                })
                .collect();

            egui::Frame::NONE
                .fill(Color32::from_rgb(232, 229, 220))
                .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    egui::Grid::new("ka_hb_result_hdr")
                        .num_columns(5)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.set_min_width(modal_w - 44.0);
                            for (lbl, w) in [
                                ("유적명 (클릭 시 지도 이동)", 280.0),
                                ("종류", 150.0),
                                ("거리", 90.0),
                                ("지정번호", 160.0),
                                ("지도 위치", 110.0),
                            ] {
                                ui.add_sized(
                                    [w, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(lbl).size(11.5).strong(),
                                    ),
                                );
                            }
                            ui.end_row();
                        });
                });

            let list_h = if app.modals.heritage_show_details {
                120.0
            } else {
                185.0
            };
            egui::ScrollArea::vertical()
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (ri, (site_name, kind_name, _, dist_m, desig_no, target_xy)) in
                        filtered_rows.iter().enumerate()
                    {
                        let selected = app.modals.heritage_selected_row == Some(ri);
                        let bg = if selected {
                            Color32::from_rgb(225, 240, 218)
                        } else if ri % 2 == 0 {
                            Color32::from_rgb(252, 251, 247)
                        } else {
                            Color32::WHITE
                        };
                        egui::Frame::NONE
                            .fill(bg)
                            .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                egui::Grid::new(("ka_hb_row", ri))
                                    .num_columns(5)
                                    .spacing([10.0, 3.0])
                                    .show(ui, |ui| {
                                        let name_resp = ui.add_sized(
                                            [280.0, 20.0],
                                            egui::Button::new(
                                                egui::RichText::new(*site_name)
                                                    .size(11.5)
                                                    .strong()
                                                    .color(Color32::from_rgb(25, 25, 24)),
                                            )
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE),
                                        );
                                        ui.add_sized(
                                            [150.0, 20.0],
                                            egui::Label::new(
                                                egui::RichText::new(*kind_name).size(11.5),
                                            ),
                                        );
                                        let dist_txt = if *dist_m >= 1000.0 {
                                            format!("{:.2} km", dist_m / 1000.0)
                                        } else {
                                            format!("{dist_m:.0} m")
                                        };
                                        ui.add_sized(
                                            [90.0, 20.0],
                                            egui::Label::new(
                                                egui::RichText::new(dist_txt)
                                                    .monospace()
                                                    .size(11.0),
                                            ),
                                        );
                                        ui.add_sized(
                                            [160.0, 20.0],
                                            egui::Label::new(
                                                egui::RichText::new(*desig_no)
                                                    .monospace()
                                                    .size(11.0),
                                            ),
                                        );
                                        let move_clicked =
                                            ui.small_button("위치 보기").clicked();
                                        if name_resp.clicked() || move_clicked {
                                            app.modals.heritage_selected_row = Some(ri);
                                            app.home_map.viewport.center_xy = *target_xy;
                                            app.home_map.viewport.meters_per_pixel = 1.2;
                                            app.active_tab =
                                                crate::app_state::MainTab::Map;
                                            app.home_map.draw_guide_message = Some(format!(
                                                "주변유적 「{site_name}」 ({desig_no}, 거리 {dist_m:.0}m) 위치로 지도를 이동했습니다."
                                            ));
                                        }
                                        ui.end_row();
                                    });
                            });
                    }
                });

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
                ui.label(
                    egui::RichText::new(format!(
                        "검색 결과 {}건 표시 중 (지도 전체 {}건)",
                        filtered_rows.len(),
                        app.home_map.real_survey.heritage_polys.len()
                    ))
                    .size(11.5)
                    .color(Color32::from_rgb(80, 79, 74)),
                );

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
                            // 서약서 동의 영수증 (MainWindow.cpp:3246-3262).
                            let receipt_dir = app
                                .home_map
                                .real_survey
                                .survey_dir
                                .join("주변유적")
                                .join("receipts");
                            let _ = std::fs::create_dir_all(&receipt_dir);
                            let _ = std::fs::write(
                                receipt_dir.join("pledge_receipt.txt"),
                                "서약서에 자동 동의했습니다. 원문은 조사폴더 주변유적/receipts 에 남습니다.\n받은 자료는 동국문화재연구원 소유입니다.",
                            );
                            // 실제 인트라넷 작업: 화면 → 목록 → 폼 → 시·군 검색 → ZIP
                            heritage_start_fetch(app);
                        }
                    }
                });
            });
        });
}
