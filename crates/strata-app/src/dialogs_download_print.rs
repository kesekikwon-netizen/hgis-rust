use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::{render_strata_modal_header, ActiveModalDialog};

/// C++ `KaDownloadUi::configure` 1:1 — 베이지 헤더 배너(`#EFECE4`)
fn render_ka_download_header(ui: &mut egui::Ui, title: &str, subtitle: &str, min_w: f32) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(239, 236, 228))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_min_width(min_w);
            ui.label(egui::RichText::new(title).size(19.0).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new(subtitle).size(13.5).color(Color32::from_rgb(80, 79, 74)));
        });
    ui.add_space(10.0);
}

/// C++ 자료 받기·시굴격자·도면 인쇄 대화상자 (`KaDownloadUi.cpp`, `KaTrenchDialog.cpp`, `KaPrintDialogPlate.cpp`) 1:1 재현
pub fn render_download_and_print_dialogs(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    kind: ActiveModalDialog,
    open: &mut bool,
) {
    match kind {
        ActiveModalDialog::HeritageRegionSelect => {
            egui::Window::new("ka_modal_heritage_bg")
                .title_bar(false)
                .resizable(false)
                .order(egui::Order::Middle)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 22.0])
                .default_width(600.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "주변유적 다운로드 - Strata", open);
                    render_ka_download_header(ui, "주변유적 다운로드", "국가유산 인트라넷 · 조사구역 주변 유적을 받습니다.", 568.0);
                    egui::Frame::NONE.fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(8.0).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
                        ui.set_min_width(568.0);
                        ui.label(egui::RichText::new("준비 · 시·군 선택").size(14.5).strong());
                        ui.label(egui::RichText::new("조사구역이 속한 시·군을 확인하고 있습니다.").size(13.5));
                        let (bar_r, _) = ui.allocate_exact_size(Vec2::new(552.0, 26.0), egui::Sense::hover());
                        ui.painter().rect_stroke(bar_r, 4.0, Stroke::new(1.0, Color32::from_rgb(160, 158, 150)), egui::StrokeKind::Middle);
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let _ = ui.add(egui::Button::new(egui::RichText::new("상세 보기").color(Color32::WHITE).strong()).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(88.0, 34.0)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let _ = ui.add(egui::Button::new("취소").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134))).corner_radius(6.0).min_size(Vec2::new(62.0, 34.0)));
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
                    for (lbl, val, hi) in [("시·도", "경상북도                                                       ▼", true), ("시·군·구", "경주시                                                          ▼", false)] {
                        ui.horizontal(|ui| {
                            ui.add_sized([88.0, 32.0], egui::Label::new(egui::RichText::new(lbl).size(14.0).color(Color32::from_rgb(80, 79, 74))));
                            let stroke = if hi { Stroke::new(1.5, Color32::from_rgb(38, 37, 34)) } else { Stroke::new(1.0, Color32::from_rgb(165, 162, 152)) };
                            let _ = ui.add(egui::Button::new(egui::RichText::new(val).size(14.0)).fill(Color32::WHITE).stroke(stroke).corner_radius(6.0).min_size(Vec2::new(260.0, 32.0)));
                        });
                        ui.add_space(6.0);
                    }
                    let mut reuse_recent = false;
                    ui.checkbox(&mut reuse_recent, egui::RichText::new("최근 받은 자료 다시 쓰기").size(14.0));
                    ui.label(egui::RichText::new("2026-10-25 08:52 받음 · 없는 자료만 서버에서 받음").size(12.5).color(Color32::from_rgb(95, 94, 88)));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고,\n동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는\n동국문화재연구원 소유입니다. 도면·보고서에 넣거나 밖으로 보내기 전에 서약 범위를\n확인하세요.").size(12.5).color(Color32::from_rgb(75, 74, 70)));
                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel_b = egui::Button::new(egui::RichText::new("취소").size(14.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134))).corner_radius(6.0).min_size(Vec2::new(66.0, 36.0));
                        if ui.add(cancel_b).clicked() { app.modals.active = None; }
                        let ok_b = egui::Button::new(egui::RichText::new("이 지역 받기").color(Color32::WHITE).strong().size(14.0)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(116.0, 36.0));
                        if ui.add(ok_b).clicked() { app.modals.active = None; }
                    });
                });
        }
        ActiveModalDialog::NgiiTopoDownload => {
            egui::Window::new("ka_modal_ngii_topo").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(540.0).show(ctx, |ui| {
                render_strata_modal_header(ui, "수치지형도 다운로드 - Strata", open);
                render_ka_download_header(ui, "수치지형도 다운로드", "국토정보플랫폼 · 조사 주변 도엽을 확인하고 지도 자료를 받습니다.", 508.0);
                egui::Frame::NONE.fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(8.0).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
                    ui.set_min_width(508.0);
                    ui.label(egui::RichText::new("[1/6] 도엽 확인").size(14.5).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("현재 단계 0초 경과 · 지도 작업을 계속할 수 있습니다.").size(13.5));
                    ui.add_space(8.0);
                    let (bar_r, _) = ui.allocate_exact_size(Vec2::new(492.0, 26.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 4.0, Color32::WHITE);
                    p.rect_stroke(bar_r, 4.0, Stroke::new(1.0, Color32::from_rgb(160, 158, 150)), egui::StrokeKind::Middle);
                    p.text(bar_r.center(), egui::Align2::CENTER_CENTER, "진행률 확인 중", egui::FontId::proportional(13.0), Color32::from_rgb(35, 35, 32));
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let _ = ui.add(egui::Button::new(egui::RichText::new("상세 보기").color(Color32::WHITE).strong().size(13.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(88.0, 34.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel_b = egui::Button::new(egui::RichText::new("취소").size(13.5)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134))).corner_radius(6.0).min_size(Vec2::new(62.0, 34.0));
                        if ui.add(cancel_b).clicked() { app.modals.active = None; }
                    });
                });
            });
        }
        ActiveModalDialog::CadastralDownload => {
            egui::Window::new("ka_modal_cadastral").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(540.0).show(ctx, |ui| {
                render_strata_modal_header(ui, "지적도 다운로드 - Strata", open);
                render_ka_download_header(ui, "지적도 다운로드", "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.", 508.0);
                egui::Frame::NONE.fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(8.0).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
                    ui.set_min_width(508.0);
                    ui.label(egui::RichText::new("자료 준비 및 다운로드").size(14.5).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("VWorld 계정을 확인하고 있습니다...").size(13.5));
                    ui.add_space(8.0);
                    let (bar_r, _) = ui.allocate_exact_size(Vec2::new(492.0, 26.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 4.0, Color32::WHITE);
                    p.rect_stroke(bar_r, 4.0, Stroke::new(1.0, Color32::from_rgb(160, 158, 150)), egui::StrokeKind::Middle);
                    p.rect_filled(Rect::from_min_size(bar_r.min + Vec2::new(1.0, 1.0), Vec2::new(40.0, 24.0)), 4.0, Color32::from_rgb(25, 25, 24));
                    p.text(bar_r.center(), egui::Align2::CENTER_CENTER, "8%", egui::FontId::proportional(13.0), Color32::from_rgb(35, 35, 32));
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("다운로드 중에도 지도 작업을 계속할 수 있습니다.").size(13.5).color(Color32::from_rgb(75, 74, 70)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel_b = egui::Button::new(egui::RichText::new("취소").color(Color32::WHITE).strong().size(13.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(62.0, 34.0));
                        if ui.add(cancel_b).clicked() { app.modals.active = None; }
                    });
                });
            });
        }
        ActiveModalDialog::TrenchGenerator => {
            egui::Window::new("ka_modal_trench").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [60.0, 20.0]).default_width(360.0).show(ctx, |ui| {
                render_strata_modal_header(ui, "시굴격자 속성 - Strata", open);
                ui.checkbox(&mut app.modals.trench_auto_place, egui::RichText::new("지금 그린(또는 선택한) 조사구역에 자동 배치").size(13.0));
                ui.horizontal(|ui| { ui.label(egui::RichText::new("조사 종류").size(13.0)); let _ = ui.add_sized([260.0, 28.0], egui::Button::new("시굴조사 — 전체 면적의 10%             ▼")); });
                let mut slope_align = false;
                ui.checkbox(&mut slope_align, egui::RichText::new("지형 사면 방향으로 넣기(등고선 직교)").size(13.0).color(Color32::from_rgb(130, 128, 120)));
                ui.label(egui::RichText::new("평지(평균 경사 0.4%)라 남북으로 깝니다. 방위 칸에 다른").size(11.5).color(Color32::from_rgb(95, 94, 88)));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("모양").size(13.0));
                    let shape_lbl = format!("{}  ▼", app.modals.trench_shape.label_ko());
                    if ui.add_sized([286.0, 28.0], egui::Button::new(shape_lbl)).clicked() {
                        let idx = TrenchShapeKind::ALL.iter().position(|&k| k == app.modals.trench_shape).unwrap_or(0);
                        app.modals.trench_shape = TrenchShapeKind::ALL[(idx + 1) % TrenchShapeKind::ALL.len()];
                    }
                });
                if app.modals.trench_shape == TrenchShapeKind::Straight {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("트렌치 길이").size(13.0));
                        for len_opt in [TrenchStandardLength::M20, TrenchStandardLength::M15, TrenchStandardLength::M10] {
                            ui.radio_value(&mut app.modals.trench_length, len_opt, egui::RichText::new(len_opt.label_ko()).size(13.0));
                        }
                    });
                }
                ui.checkbox(&mut app.modals.trench_grid_snap, egui::RichText::new("좌표격자에 맞추기").size(13.0));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("회전(방위)").size(13.0));
                    let _ = ui.add_sized([110.0, 28.0], egui::Button::new(format!("{:.2} °             ↕", app.modals.trench_azimuth_deg)));
                    let (cr, _) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::hover());
                    let cp = ui.painter_at(cr);
                    cp.circle_stroke(cr.center(), 9.0, Stroke::new(1.2, Color32::from_rgb(120, 118, 110)));
                    cp.line_segment([cr.center(), Pos2::new(cr.center().x, cr.top() + 2.0)], Stroke::new(1.8, Color32::BLACK));
                    cp.circle_filled(cr.center(), 2.0, Color32::BLACK);
                    ui.label(egui::RichText::new("위가 북쪽입니다").size(12.0).color(Color32::from_rgb(95, 94, 88)));
                });
                ui.add_space(6.0);
                let trench_count = app.home_map.real_survey.trenches.len().max(1);
                let total_trench_area: f64 = app.home_map.real_survey.trenches.iter().map(|t| t.area_m2).sum();
                let survey_area: f64 = app.home_map.real_survey.survey_area_m2.max(1.0);
                let ratio_pct = (total_trench_area / survey_area * 100.0).clamp(0.0, 100.0);
                let shape_desc = if app.modals.trench_shape.arm_bend().is_some() {
                    format!("{} {}개", app.modals.trench_shape.label_ko(), trench_count)
                } else {
                    format!("{} 트렌치 {}개", app.modals.trench_length.label_ko(), trench_count)
                };
                let summary_text = format!(
                    "시굴조사 · {shape_desc} · 폭 2 m · 둑 10 m ·\n총 {total_trench_area:.2}㎡ · 비율 {ratio_pct:.1}% (목표 10%) · 방위 {:.0}°",
                    app.modals.trench_azimuth_deg
                );
                egui::Frame::NONE.fill(Color32::from_rgb(225, 240, 218)).stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145))).corner_radius(6.0).inner_margin(egui::Margin::symmetric(10, 8)).show(ui, |ui| {
                    ui.label(egui::RichText::new(summary_text).size(12.5).strong().color(Color32::from_rgb(28, 72, 32)));
                });
                ui.add_space(4.0);
                let (bar_r, _) = ui.allocate_exact_size(Vec2::new(340.0, 6.0), egui::Sense::hover());
                ui.painter().rect_filled(bar_r, 3.0, Color32::from_rgb(222, 220, 210));
                let fill_w = (340.0_f32 * (ratio_pct as f32 / 20.0_f32)).clamp(12.0_f32, 340.0_f32);
                ui.painter().rect_filled(Rect::from_min_size(bar_r.min, Vec2::new(fill_w, 6.0)), 3.0, Color32::from_rgb(35, 35, 32));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("0").size(10.5));
                    ui.add_space(130.0);
                    ui.label(egui::RichText::new("목표 10%").size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new("20%").size(10.5)); });
                });
                ui.label(egui::RichText::new("깐 격자는 맵에서 끌고 옮깁니다. 개별 편집은 트렌치 하나\n이동·삭제입니다").size(11.5).color(Color32::from_rgb(95, 94, 88)));
                ui.horizontal(|ui| {
                    let _ = ui.add_sized([88.0, 28.0], egui::Button::new("개별 편집"));
                    if ui.add_sized([88.0, 28.0], egui::Button::new("전체 이동")).clicked() {
                        let snap = if app.modals.trench_grid_snap { Some(5.0) } else { None };
                        app.home_map.real_survey.translate_all_trenches(5.0, 0.0, snap);
                        app.is_dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    let b1 = egui::Button::new(egui::RichText::new("구역에 깔기").color(Color32::WHITE).strong()).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(104.0, 32.0));
                    if ui.add(b1).clicked() {
                        let (count, _, _) = app.home_map.real_survey.apply_trench_plan(
                            app.modals.trench_length,
                            app.modals.trench_shape,
                            app.modals.trench_grid_snap,
                        );
                        app.home_map.draw_guide_message = Some(format!(
                            "시굴격자 {}개를 배치했습니다 ({})",
                            count,
                            app.modals.trench_shape.label_ko()
                        ));
                        app.is_dirty = true;
                        app.modals.active = None;
                    }
                    let _ = ui.add_sized([88.0, 32.0], egui::Button::new("맵에 찍기"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([58.0, 32.0], egui::Button::new("닫기")).clicked() { app.modals.active = None; }
                    });
                });
            });
        }
        ActiveModalDialog::PdfPreparing => {
            egui::Window::new("ka_modal_pdf_prep").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).fixed_size([360.0, 128.0]).show(ctx, |ui| {
                render_strata_modal_header(ui, "인쇄 준비 - Strata", open);
                ui.label(egui::RichText::new("PDF를 그리는 중... (용지 210x297 mm · 300 DPI)").size(13.0));
                ui.label(egui::RichText::new("취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.").size(12.0).color(Color32::from_rgb(95, 94, 88)));
                ui.add_space(6.0);
                let (bar_r, _) = ui.allocate_exact_size(Vec2::new(340.0, 18.0), egui::Sense::hover());
                let p = ui.painter_at(bar_r);
                p.rect_filled(bar_r, 3.0, Color32::WHITE);
                p.rect_stroke(bar_r, 3.0, Stroke::new(1.0, Color32::from_rgb(160, 158, 150)), egui::StrokeKind::Middle);
                p.rect_filled(Rect::from_center_size(bar_r.center(), Vec2::new(160.0, 14.0)), 2.0, Color32::from_rgb(25, 25, 24));
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cb = egui::Button::new("취소").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142))).corner_radius(5.0).min_size(Vec2::new(58.0, 28.0));
                    if ui.add(cb).clicked() { app.modals.active = None; }
                });
            });
        }
        ActiveModalDialog::DrawingPrint => {
            egui::Window::new("ka_modal_drawing_print").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(760.0).show(ctx, |ui| {
                render_strata_modal_header(ui, "도면 인쇄 - Strata", open);
                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.add_sized([68.0, 30.0], egui::Label::new("프린터"));
                        let _ = ui.add(egui::Button::new("ezViewX Virtual Image Printer              ▼").fill(Color32::WHITE).stroke(Stroke::new(1.3, Color32::from_rgb(35, 35, 32))).corner_radius(5.0).min_size(Vec2::new(258.0, 30.0)));
                    });
                    cols[0].horizontal(|ui| {
                        ui.add_space(74.0);
                        let _ = ui.add(egui::Button::new("프린터 속성...").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152))).corner_radius(5.0).min_size(Vec2::new(84.0, 24.0)));
                    });
                    cols[0].add_space(8.0);
                    let mut single_sheet = true;
                    cols[0].horizontal_top(|ui| {
                        ui.add_sized([68.0, 24.0], egui::Label::new("방법"));
                        ui.vertical(|ui| {
                            ui.radio_value(&mut single_sheet, false, "여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)");
                            ui.radio_value(&mut single_sheet, true, "한 장에 인쇄 (용지에 들어가면 실제 크기로)");
                        });
                    });
                    cols[0].add_space(8.0);
                    cols[0].horizontal(|ui| {
                        ui.add_sized([68.0, 28.0], egui::Label::new("용지"));
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let _ = ui.add(egui::Button::new("A4").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142))).corner_radius(4.0).min_size(Vec2::new(38.0, 28.0)));
                        let _ = ui.add(egui::Button::new("A3").fill(Color32::from_rgb(214, 208, 194)).stroke(Stroke::new(1.0, Color32::from_rgb(125, 122, 114))).corner_radius(4.0).min_size(Vec2::new(38.0, 28.0)));
                    });
                    cols[0].add_space(8.0);
                    cols[0].horizontal(|ui| {
                        ui.add_sized([68.0, 30.0], egui::Label::new("해상도"));
                        let _ = ui.add(egui::Button::new("300 DPI (보통)                                      ▼").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152))).corner_radius(5.0).min_size(Vec2::new(258.0, 30.0)));
                    });
                    cols[0].add_space(126.0);
                    let (top_rule, _) = cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), 2.0), egui::Sense::hover());
                    cols[0].painter().line_segment([top_rule.left_center(), top_rule.right_center()], Stroke::new(1.2, Color32::from_rgb(45, 44, 40)));
                    for (k, v) in [("용지", "A3 세로 · 한 장"), ("인쇄 크기", "실제 크기 100%"), ("축척", "1:6,532")] {
                        cols[0].horizontal(|ui| {
                            ui.label(k);
                            ui.label(egui::RichText::new("....................................................").color(Color32::from_rgb(165, 162, 152)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.strong(v); });
                        });
                    }
                    cols[0].separator();
                    cols[1].horizontal(|ui| {
                        ui.label("미리보기");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("도면 · A3 세로 · 한 장 · 1:6,532").size(12.0).color(Color32::from_rgb(95, 94, 88)));
                        });
                    });
                    let (pr, _) = cols[1].allocate_exact_size(Vec2::new(350.0, 370.0), egui::Sense::hover());
                    let p = cols[1].painter_at(pr);
                    p.rect_filled(pr, 2.0, Color32::from_rgb(236, 234, 226));
                    let paper_r = pr.shrink2(Vec2::new(34.0, 14.0));
                    p.rect_filled(paper_r, 0.0, Color32::WHITE);
                    p.rect_stroke(paper_r, 0.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Middle);
                    let map_r = Rect::from_min_max(paper_r.min + Vec2::new(28.0, 44.0), Pos2::new(paper_r.right() - 28.0, paper_r.bottom() - 56.0));
                    p.rect_filled(map_r, 0.0, Color32::from_rgb(154, 198, 120));
                    for g in 1..12 {
                        let gx = map_r.left() + (g as f32) * (map_r.width() / 12.0);
                        let gy = map_r.top() + (g as f32) * (map_r.height() / 12.0);
                        p.line_segment([Pos2::new(gx, map_r.top()), Pos2::new(gx, map_r.bottom())], Stroke::new(0.7, Color32::from_rgb(30, 30, 28)));
                        p.line_segment([Pos2::new(map_r.left(), gy), Pos2::new(map_r.right(), gy)], Stroke::new(0.7, Color32::from_rgb(30, 30, 28)));
                    }
                    p.rect_stroke(map_r, 0.0, Stroke::new(1.2, Color32::BLACK), egui::StrokeKind::Middle);
                    for i in 0..85 {
                        let (fx, fy) = (((i * 37 + 13) % 100) as f32 / 100.0 - 0.5, ((i * 61 + 29) % 100) as f32 / 100.0 - 0.5);
                        let dp = Pos2::new(map_r.center().x + fx * map_r.width() * 0.88, map_r.center().y + fy * map_r.height() * 0.88);
                        let dc = if i % 4 == 0 { Color32::from_rgb(45, 165, 75) } else { Color32::from_rgb(198, 28, 118) };
                        p.circle_filled(dp, 2.2, dc);
                        p.circle_stroke(dp, 2.2, Stroke::new(0.6, Color32::BLACK));
                    }
                    let sr = Rect::from_center_size(map_r.center(), Vec2::new(14.0, map_r.height() * 0.56));
                    p.rect_filled(sr, 0.0, Color32::from_rgba_unmultiplied(214, 92, 32, 130));
                    p.rect_stroke(sr, 0.0, Stroke::new(1.4, Color32::from_rgb(214, 78, 22)), egui::StrokeKind::Middle);
                    p.line_segment([Pos2::new(map_r.left() + 4.0, map_r.bottom() + 10.0), Pos2::new(map_r.left() + 68.0, map_r.bottom() + 10.0)], Stroke::new(2.5, Color32::BLACK));
                    p.text(Pos2::new(map_r.left() + 36.0, map_r.bottom() + 20.0), egui::Align2::CENTER_CENTER, "축척 1 : 6532", egui::FontId::proportional(7.5), Color32::BLACK);
                    p.text(Pos2::new(map_r.right() - 18.0, map_r.bottom() + 10.0), egui::Align2::RIGHT_CENTER, "동부원점(GRS80) · EPSG:5187", egui::FontId::proportional(7.0), Color32::BLACK);
                    p.text(Pos2::new(map_r.right() - 5.0, map_r.bottom() + 12.0), egui::Align2::CENTER_CENTER, "N\n▲", egui::FontId::proportional(8.5), Color32::BLACK);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_b = egui::Button::new("닫기").fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142))).corner_radius(6.0).min_size(Vec2::new(58.0, 34.0));
                    if ui.add(close_b).clicked() { app.modals.active = None; }
                    let prt = egui::Button::new(egui::RichText::new("인쇄 · 1장").color(Color32::WHITE).strong()).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(88.0, 34.0));
                    if ui.add(prt).clicked() { app.modals.active = None; }
                });
            });
        }
        _ => {}
    }
}
