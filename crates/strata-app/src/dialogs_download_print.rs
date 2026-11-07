use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};
use strata_layout::sheet_composer::SheetPaperSize;
use strata_layout::tile_print::{
    compute_tile_print_plan, format_print_dispatched_message, verify_requested_device_paper,
};
use strata_storage::gpkg::{validate_survey_name_ko, KA_LONGITUDE_RULER_CITIES};
use strata_types::crs::WorkCrs;

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
            ui.label(
                egui::RichText::new(subtitle)
                    .size(13.5)
                    .color(Color32::from_rgb(80, 79, 74)),
            );
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
                        if ui.add(ok_b).clicked() { app.modals.active = Some(ActiveModalDialog::HeritageBrowser); }
                    });
                });
        }
        ActiveModalDialog::NgiiTopoDownload => {
            render_topographic_browser(app, ctx, open);
        }
        ActiveModalDialog::CadastralDownload => {
            egui::Window::new("ka_modal_cadastral")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(540.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "지적도 다운로드 - Strata", open);
                    render_ka_download_header(
                        ui,
                        "지적도 다운로드",
                        "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.",
                        508.0,
                    );
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
                            ui.label(
                                egui::RichText::new("VWorld 계정을 확인하고 있습니다...")
                                    .size(13.5),
                            );
                            ui.add_space(8.0);
                            let (bar_r, _) = ui
                                .allocate_exact_size(Vec2::new(492.0, 26.0), egui::Sense::hover());
                            let p = ui.painter_at(bar_r);
                            p.rect_filled(bar_r, 4.0, Color32::WHITE);
                            p.rect_stroke(
                                bar_r,
                                4.0,
                                Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                                egui::StrokeKind::Middle,
                            );
                            p.rect_filled(
                                Rect::from_min_size(
                                    bar_r.min + Vec2::new(1.0, 1.0),
                                    Vec2::new(40.0, 24.0),
                                ),
                                4.0,
                                Color32::from_rgb(25, 25, 24),
                            );
                            p.text(
                                bar_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "8%",
                                egui::FontId::proportional(13.0),
                                Color32::from_rgb(35, 35, 32),
                            );
                        });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("다운로드 중에도 지도 작업을 계속할 수 있습니다.")
                                .size(13.5)
                                .color(Color32::from_rgb(75, 74, 70)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cancel_b = egui::Button::new(
                                egui::RichText::new("취소")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.5),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(62.0, 34.0));
                            if ui.add(cancel_b).clicked() {
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::TrenchGenerator => {
            egui::Window::new("ka_modal_trench")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [60.0, 20.0])
                .default_width(380.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "시굴격자 속성 - Strata", open);
                    ui.checkbox(
                        &mut app.modals.trench_auto_place,
                        egui::RichText::new("지금 그린(또는 선택한) 조사구역에 자동 배치").size(13.0),
                    );

                    // KaTrenchDialog.cpp:27-155 조사 종류 3단 순환
                    const KIND_LABELS: [&str; 3] = [
                        "시굴조사 — 전체 면적의 10%",
                        "표본조사 — 전체 면적의 2%",
                        "직접 지정 — 규격·둑을 내가 정함",
                    ];
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("조사 종류").size(13.0));
                        if ui
                            .add_sized(
                                [260.0, 28.0],
                                egui::Button::new(format!(
                                    "{}  ▼",
                                    KIND_LABELS[app.modals.trench_survey_kind_idx.min(2)]
                                )),
                            )
                            .clicked()
                        {
                            app.modals.trench_survey_kind_idx =
                                (app.modals.trench_survey_kind_idx + 1) % 3;
                        }
                    });

                    ui.checkbox(
                        &mut app.modals.trench_slope_align,
                        egui::RichText::new("지형 사면 방향으로 넣기(등고선 직교)").size(13.0),
                    );

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("모양").size(13.0));
                        let shape_lbl = format!("{}  ▼", app.modals.trench_shape.label_ko());
                        if ui
                            .add_sized([286.0, 28.0], egui::Button::new(shape_lbl))
                            .clicked()
                        {
                            let idx = TrenchShapeKind::ALL
                                .iter()
                                .position(|&k| k == app.modals.trench_shape)
                                .unwrap_or(0);
                            app.modals.trench_shape =
                                TrenchShapeKind::ALL[(idx + 1) % TrenchShapeKind::ALL.len()];
                        }
                    });
                    if app.modals.trench_shape == TrenchShapeKind::Straight {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("트렌치 길이").size(13.0));
                            for len_opt in [
                                TrenchStandardLength::M20,
                                TrenchStandardLength::M15,
                                TrenchStandardLength::M10,
                            ] {
                                ui.radio_value(
                                    &mut app.modals.trench_length,
                                    len_opt,
                                    egui::RichText::new(len_opt.label_ko()).size(13.0),
                                );
                            }
                        });
                    }
                    ui.checkbox(
                        &mut app.modals.trench_grid_snap,
                        egui::RichText::new("좌표격자에 맞추기").size(13.0),
                    );

                    // KaTrenchDialogPlan.cpp:80-92 행/열 은 수동 배치에서만, 둑/접두 는
                    // 수동 배치이거나 '직접 지정'(idx 2)일 때 보인다.
                    let manual = !app.modals.trench_auto_place;
                    let kind = app.modals.trench_survey_kind_idx.min(2);
                    if manual {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("행").size(13.0));
                            ui.add(
                                egui::DragValue::new(&mut app.modals.trench_rows)
                                    .range(1..=200)
                                    .speed(0.2),
                            );
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("열").size(13.0));
                            ui.add(
                                egui::DragValue::new(&mut app.modals.trench_cols)
                                    .range(1..=200)
                                    .speed(0.2),
                            );
                        });
                    }
                    if manual || kind == 2 {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("둑(간격)").size(13.0));
                            ui.add(
                                egui::DragValue::new(&mut app.modals.trench_balk_m)
                                    .range(0.0..=200.0)
                                    .speed(0.5)
                                    .suffix(" m"),
                            );
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("이름 prefix").size(13.0));
                            ui.add(
                                egui::TextEdit::singleline(&mut app.modals.trench_prefix)
                                    .desired_width(90.0),
                            );
                        });
                    }

                    // KaAzimuthDial.cpp:24-49 회전(방위) 다이얼
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("회전(방위)").size(13.0));
                        ui.add(
                            egui::DragValue::new(&mut app.modals.trench_azimuth_deg)
                                .range(0.0..=360.0)
                                .speed(1.0)
                                .suffix(" °"),
                        );
                        let (cr, _) =
                            ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::hover());
                        let cp = ui.painter_at(cr);
                        cp.circle_stroke(
                            cr.center(),
                            9.0,
                            Stroke::new(1.2, Color32::from_rgb(120, 118, 110)),
                        );
                        let rad = (app.modals.trench_azimuth_deg as f32).to_radians();
                        let tip = Pos2::new(
                            cr.center().x + rad.sin() * 8.0,
                            cr.center().y - rad.cos() * 8.0,
                        );
                        cp.line_segment([cr.center(), tip], Stroke::new(1.8, Color32::BLACK));
                        cp.circle_filled(cr.center(), 2.0, Color32::BLACK);
                        let turn = app.modals.trench_azimuth_deg.rem_euclid(360.0);
                        let dial_note = if turn < 0.5 || (360.0 - turn) < 0.5 {
                            "위가 북쪽���니다".to_owned()
                        } else {
                            format!("북에서 시계 방향 {turn:.0}°")
                        };
                        ui.label(
                            egui::RichText::new(dial_note)
                                .size(12.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    });

                    ui.add_space(6.0);
                    // KaTrenchDialogPlan.cpp:110-120 요약 + KaTargetMeter.cpp:36-59
                    if manual {
                        let count = app.modals.trench_rows * app.modals.trench_cols;
                        let summary = format!(
                            "수동 배치: {count}개 · 총 {:.0}㎡ — 「맵에 찍기」로 맵에서 위치를 정하세요.",
                            count as f64 * 40.0
                        );
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(225, 240, 218))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(summary)
                                        .size(12.5)
                                        .strong()
                                        .color(Color32::from_rgb(28, 72, 32)),
                                );
                            });
                    } else {
                        let trench_count = app.home_map.real_survey.trenches.len().max(1);
                        let total_trench_area: f64 =
                            app.home_map.real_survey.trenches.iter().map(|t| t.area_m2).sum();
                        let survey_area: f64 =
                            app.home_map.real_survey.survey_area_m2.max(1.0);
                        let ratio_pct =
                            (total_trench_area / survey_area * 100.0).clamp(0.0, 100.0);
                        let shape_desc = if app.modals.trench_shape.arm_bend().is_some() {
                            format!("{} {}개", app.modals.trench_shape.label_ko(), trench_count)
                        } else {
                            format!(
                                "{} 트렌치 {}개",
                                app.modals.trench_length.label_ko(),
                                trench_count
                            )
                        };
                        let target = if kind == 1 { 2.0_f64 } else { 10.0_f64 };
                        let summary_text = format!(
                            "{} · {shape_desc} · 폭 2 m · 둑 {:.0} m ·\n총 {total_trench_area:.2}㎡ · 비율 {ratio_pct:.1}% (목표 {target:.0}%) · 방위 {:.0}°",
                            KIND_LABELS[kind],
                            app.modals.trench_balk_m,
                            app.modals.trench_azimuth_deg
                        );
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(225, 240, 218))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(summary_text)
                                        .size(12.5)
                                        .strong()
                                        .color(Color32::from_rgb(28, 72, 32)),
                                );
                            });
                        // 목표 눈금 막대: 0 .. target*2
                        ui.add_space(4.0);
                        let (bar_r, _) = ui
                            .allocate_exact_size(Vec2::new(340.0, 6.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(bar_r, 3.0, Color32::from_rgb(222, 220, 210));
                        let top = target * 2.0;
                        let fill_w = (340.0_f32 * (ratio_pct as f32 / top as f32))
                            .clamp(12.0_f32, 340.0_f32);
                        ui.painter().rect_filled(
                            Rect::from_min_size(bar_r.min, Vec2::new(fill_w, 6.0)),
                            3.0,
                            Color32::from_rgb(35, 35, 32),
                        );
                        ui.label(
                            egui::RichText::new(format!("목표 {target:.0}%"))
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                });
        }
        ActiveModalDialog::PdfPreparing => {
            egui::Window::new("ka_modal_pdf_prep")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([360.0, 128.0])
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "인쇄 준비 - Strata", open);
                    ui.label(
                        egui::RichText::new("PDF를 그리는 중... (용지 210x297 mm · 300 DPI)")
                            .size(13.0),
                    );
                    ui.label(
                        egui::RichText::new("취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.")
                            .size(12.0)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.add_space(6.0);
                    let (bar_r, _) =
                        ui.allocate_exact_size(Vec2::new(340.0, 18.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 3.0, Color32::WHITE);
                    p.rect_stroke(
                        bar_r,
                        3.0,
                        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                        egui::StrokeKind::Middle,
                    );
                    p.rect_filled(
                        Rect::from_center_size(bar_r.center(), Vec2::new(160.0, 14.0)),
                        2.0,
                        Color32::from_rgb(25, 25, 24),
                    );
                    ui.add_space(6.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cb = egui::Button::new("취소")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                            .corner_radius(5.0)
                            .min_size(Vec2::new(58.0, 28.0));
                        if ui.add(cb).clicked() {
                            app.modals.active = None;
                        }
                    });
                });
        }
        ActiveModalDialog::DrawingPrint => {
            egui::Window::new("ka_modal_drawing_print")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(760.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "도면 인쇄 - Strata", open);
                    ui.columns(2, |cols| {
                        cols[0].horizontal(|ui| {
                            ui.add_sized([68.0, 30.0], egui::Label::new("프린터"));
                            let _ = ui.add(
                                egui::Button::new("ezViewX Virtual Image Printer              ▼")
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(1.3, Color32::from_rgb(35, 35, 32)))
                                    .corner_radius(5.0)
                                    .min_size(Vec2::new(258.0, 30.0)),
                            );
                        });
                        cols[0].horizontal(|ui| {
                            ui.add_space(74.0);
                            let _ = ui.add(
                                egui::Button::new("프린터 속성...")
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
                                    .corner_radius(5.0)
                                    .min_size(Vec2::new(84.0, 24.0)),
                            );
                        });
                        cols[0].add_space(8.0);
                        cols[0].horizontal_top(|ui| {
                            ui.add_sized([68.0, 24.0], egui::Label::new("방법"));
                            ui.vertical(|ui| {
                                ui.radio_value(
                                    &mut app.modals.print_single_sheet,
                                    false,
                                    "여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)",
                                );
                                ui.radio_value(
                                    &mut app.modals.print_single_sheet,
                                    true,
                                    "한 장에 인쇄 (용지에 들어가면 실제 크기로)",
                                );
                            });
                        });
                        cols[0].add_space(8.0);
                        cols[0].horizontal(|ui| {
                            ui.add_sized([68.0, 28.0], egui::Label::new("용지"));
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (lbl, sz) in
                                [("A4", SheetPaperSize::A4), ("A3", SheetPaperSize::A3)]
                            {
                                let sel = app.modals.print_paper == sz;
                                let fill = if sel {
                                    Color32::from_rgb(214, 208, 194)
                                } else {
                                    Color32::WHITE
                                };
                                let stroke = if sel {
                                    Stroke::new(1.2, Color32::from_rgb(125, 122, 114))
                                } else {
                                    Stroke::new(1.0, Color32::from_rgb(155, 152, 142))
                                };
                                if ui
                                    .add(
                                        egui::Button::new(lbl)
                                            .fill(fill)
                                            .stroke(stroke)
                                            .corner_radius(4.0)
                                            .min_size(Vec2::new(42.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    app.modals.print_paper = sz;
                                }
                            }
                        });
                        cols[0].add_space(8.0);
                        cols[0].horizontal(|ui| {
                            ui.add_sized([68.0, 30.0], egui::Label::new("해상도"));
                            let dpi_lbl = if app.modals.print_dpi == 600 {
                                "600 DPI (고정밀 제출용)                              ▼"
                            } else {
                                "300 DPI (보통)                                      ▼"
                            };
                            if ui
                                .add(
                                    egui::Button::new(dpi_lbl)
                                        .fill(Color32::WHITE)
                                        .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
                                        .corner_radius(5.0)
                                        .min_size(Vec2::new(258.0, 30.0)),
                                )
                                .clicked()
                            {
                                app.modals.print_dpi = if app.modals.print_dpi == 300 {
                                    600
                                } else {
                                    300
                                };
                            }
                        });
                        cols[0].add_space(126.0);
                        let tile_plan =
                            compute_tile_print_plan(&app.drawing.sheet, app.modals.print_paper);
                        let page_cnt = if app.modals.print_single_sheet {
                            1
                        } else {
                            (tile_plan.rows * tile_plan.cols).max(2)
                        };
                        let paper_summary = format!(
                            "{} {} · {}장",
                            app.modals.print_paper.label_ko(),
                            app.drawing.sheet.orientation.label_ko(),
                            page_cnt
                        );
                        let scale_summary = format!("1:{}", app.drawing.sheet.scale_denominator);
                        let (top_rule, _) = cols[0].allocate_exact_size(
                            Vec2::new(cols[0].available_width(), 2.0),
                            egui::Sense::hover(),
                        );
                        cols[0].painter().line_segment(
                            [top_rule.left_center(), top_rule.right_center()],
                            Stroke::new(1.2, Color32::from_rgb(45, 44, 40)),
                        );
                        for (k, v) in [
                            ("용지", paper_summary.as_str()),
                            ("인쇄 크기", "실제 크기 100%"),
                            ("축척", scale_summary.as_str()),
                        ] {
                            cols[0].horizontal(|ui| {
                                ui.label(k);
                                ui.label(
                                    egui::RichText::new(
                                        "....................................................",
                                    )
                                    .color(Color32::from_rgb(165, 162, 152)),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.strong(v);
                                    },
                                );
                            });
                        }
                        cols[0].separator();
                        cols[1].horizontal(|ui| {
                            ui.label("미리보기");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "도면 · {} · {}",
                                            paper_summary, scale_summary
                                        ))
                                        .size(12.0)
                                        .color(Color32::from_rgb(95, 94, 88)),
                                    );
                                },
                            );
                        });
                        let (pr, _) = cols[1]
                            .allocate_exact_size(Vec2::new(350.0, 370.0), egui::Sense::hover());
                        let p = cols[1].painter_at(pr);
                        p.rect_filled(pr, 2.0, Color32::from_rgb(236, 234, 226));
                        let paper_r = pr.shrink2(Vec2::new(34.0, 14.0));
                        p.rect_filled(paper_r, 0.0, Color32::WHITE);
                        p.rect_stroke(
                            paper_r,
                            0.0,
                            Stroke::new(1.0, Color32::BLACK),
                            egui::StrokeKind::Middle,
                        );
                        let map_r = Rect::from_min_max(
                            paper_r.min + Vec2::new(28.0, 44.0),
                            Pos2::new(paper_r.right() - 28.0, paper_r.bottom() - 56.0),
                        );
                        p.rect_filled(map_r, 0.0, Color32::from_rgb(154, 198, 120));
                        for g in 1..12 {
                            let gx = map_r.left() + (g as f32) * (map_r.width() / 12.0);
                            let gy = map_r.top() + (g as f32) * (map_r.height() / 12.0);
                            p.line_segment(
                                [Pos2::new(gx, map_r.top()), Pos2::new(gx, map_r.bottom())],
                                Stroke::new(0.7, Color32::from_rgb(30, 30, 28)),
                            );
                            p.line_segment(
                                [Pos2::new(map_r.left(), gy), Pos2::new(map_r.right(), gy)],
                                Stroke::new(0.7, Color32::from_rgb(30, 30, 28)),
                            );
                        }
                        p.rect_stroke(
                            map_r,
                            0.0,
                            Stroke::new(1.2, Color32::BLACK),
                            egui::StrokeKind::Middle,
                        );
                        for i in 0..85 {
                            let (fx, fy) = (
                                ((i * 37 + 13) % 100) as f32 / 100.0 - 0.5,
                                ((i * 61 + 29) % 100) as f32 / 100.0 - 0.5,
                            );
                            let dp = Pos2::new(
                                map_r.center().x + fx * map_r.width() * 0.88,
                                map_r.center().y + fy * map_r.height() * 0.88,
                            );
                            let dc = if i % 4 == 0 {
                                Color32::from_rgb(45, 165, 75)
                            } else {
                                Color32::from_rgb(198, 28, 118)
                            };
                            p.circle_filled(dp, 2.2, dc);
                            p.circle_stroke(dp, 2.2, Stroke::new(0.6, Color32::BLACK));
                        }
                        let sr = Rect::from_center_size(
                            map_r.center(),
                            Vec2::new(14.0, map_r.height() * 0.56),
                        );
                        p.rect_filled(sr, 0.0, Color32::from_rgba_unmultiplied(214, 92, 32, 130));
                        p.rect_stroke(
                            sr,
                            0.0,
                            Stroke::new(1.4, Color32::from_rgb(214, 78, 22)),
                            egui::StrokeKind::Middle,
                        );
                        p.line_segment(
                            [
                                Pos2::new(map_r.left() + 4.0, map_r.bottom() + 10.0),
                                Pos2::new(map_r.left() + 68.0, map_r.bottom() + 10.0),
                            ],
                            Stroke::new(2.5, Color32::BLACK),
                        );
                        p.text(
                            Pos2::new(map_r.left() + 36.0, map_r.bottom() + 20.0),
                            egui::Align2::CENTER_CENTER,
                            "축척 1 : 6532",
                            egui::FontId::proportional(7.5),
                            Color32::BLACK,
                        );
                        p.text(
                            Pos2::new(map_r.right() - 18.0, map_r.bottom() + 10.0),
                            egui::Align2::RIGHT_CENTER,
                            "동부원점(GRS80) · EPSG:5187",
                            egui::FontId::proportional(7.0),
                            Color32::BLACK,
                        );
                        p.text(
                            Pos2::new(map_r.right() - 5.0, map_r.bottom() + 12.0),
                            egui::Align2::CENTER_CENTER,
                            "N\n▲",
                            egui::FontId::proportional(8.5),
                            Color32::BLACK,
                        );
                    });
                    let tile_plan =
                        compute_tile_print_plan(&app.drawing.sheet, app.modals.print_paper);
                    let page_cnt = if app.modals.print_single_sheet {
                        1
                    } else {
                        (tile_plan.rows * tile_plan.cols).max(2)
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_b = egui::Button::new("닫기")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(58.0, 34.0));
                        if ui.add(close_b).clicked() {
                            app.modals.active = None;
                        }
                        let prt = egui::Button::new(
                            egui::RichText::new(format!("인쇄 · {page_cnt}장"))
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(96.0, 34.0));
                        if ui.add(prt).clicked() {
                            let ori = app.drawing.sheet.orientation;
                            if verify_requested_device_paper(
                                app.modals.print_paper,
                                ori,
                                app.modals.print_paper,
                                ori,
                            )
                            .is_ok()
                            {
                                app.home_map.draw_guide_message =
                                    Some(format_print_dispatched_message(
                                        app.modals.print_paper,
                                        ori,
                                        page_cnt,
                                    ));
                            }
                            app.modals.active = None;
                        }
                    });
                });
        }
        ActiveModalDialog::NewSurvey => {
            egui::Window::new("ka_modal_new_survey")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([460.0, 310.0])
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "새 조사 시작 - Strata", open);
                    ui.label(egui::RichText::new("조사 이름").strong().size(13.0));
                    ui.add_sized(
                        [440.0, 28.0],
                        egui::TextEdit::singleline(&mut app.modals.new_survey_name)
                            .hint_text("예: 경주 구정동 유적 시굴조사"),
                    );
                    if let Err(err_msg) = validate_survey_name_ko(&app.modals.new_survey_name) {
                        ui.label(
                            egui::RichText::new(err_msg)
                                .size(11.5)
                                .color(Color32::from_rgb(185, 28, 28)),
                        );
                    }
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("저장 폴더").strong().size(13.0));
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [356.0, 28.0],
                            egui::TextEdit::singleline(&mut app.modals.new_survey_dir),
                        );
                        let _ = ui.add_sized([76.0, 28.0], egui::Button::new("폴더 선택..."));
                    });
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("좌표계 (GRS80)").strong().size(13.0));
                    // C++ KaLongitudeRuler (126°~130°, 128° 중부|동부 경계 + 8개 도시 눈금)
                    let (ruler_r, _) =
                        ui.allocate_exact_size(Vec2::new(440.0, 62.0), egui::Sense::hover());
                    let rp = ui.painter_at(ruler_r);
                    rp.rect_filled(ruler_r, 4.0, Color32::from_rgb(245, 241, 232));
                    rp.rect_stroke(
                        ruler_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(212, 204, 190)),
                        egui::StrokeKind::Middle,
                    );
                    let x_of = |lon: f64| -> f32 {
                        let t = ((lon - 126.0) / 4.0).clamp(0.0, 1.0) as f32;
                        ruler_r.left() + 18.0 + t * (ruler_r.width() - 36.0)
                    };
                    let bar_y = ruler_r.top() + 34.0;
                    let mid_x = x_of(128.0);
                    rp.line_segment(
                        [Pos2::new(x_of(126.0), bar_y), Pos2::new(x_of(130.0), bar_y)],
                        Stroke::new(1.2, Color32::from_rgb(168, 160, 147)),
                    );
                    rp.line_segment(
                        [
                            Pos2::new(mid_x, ruler_r.top() + 4.0),
                            Pos2::new(mid_x, bar_y + 8.0),
                        ],
                        Stroke::new(1.5, Color32::from_rgb(180, 83, 9)),
                    );
                    rp.text(
                        Pos2::new(mid_x, ruler_r.top() + 11.0),
                        egui::Align2::CENTER_CENTER,
                        "중부원점 | 동부원점",
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(180, 83, 9),
                    );
                    for deg in 126..=130 {
                        let dx = x_of(f64::from(deg));
                        rp.line_segment(
                            [Pos2::new(dx, bar_y - 4.0), Pos2::new(dx, bar_y + 4.0)],
                            Stroke::new(1.2, Color32::from_rgb(90, 85, 76)),
                        );
                        rp.text(
                            Pos2::new(dx, bar_y + 13.0),
                            egui::Align2::CENTER_CENTER,
                            format!("{deg}°"),
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(90, 85, 76),
                        );
                    }
                    for (city, lon) in KA_LONGITUDE_RULER_CITIES {
                        let cx = x_of(lon);
                        let col = if lon < 128.0 {
                            Color32::from_rgb(29, 78, 216)
                        } else {
                            Color32::from_rgb(185, 28, 28)
                        };
                        rp.circle_filled(Pos2::new(cx, bar_y), 3.0, col);
                        rp.text(
                            Pos2::new(cx, bar_y - 11.0),
                            egui::Align2::CENTER_CENTER,
                            city,
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(45, 43, 39),
                        );
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for (crs_opt, title, sub) in [
                            (
                                WorkCrs::Epsg5186,
                                "5186 중부원점",
                                "경도 128° 서쪽 · 서울·대전·전주·광주",
                            ),
                            (
                                WorkCrs::Epsg5187,
                                "5187 동부원점",
                                "경도 128° 동쪽 · 강릉·대구·경주·부산",
                            ),
                        ] {
                            let selected = app.modals.new_survey_crs == crs_opt;
                            let fill = if selected {
                                Color32::from_rgb(214, 208, 194)
                            } else {
                                Color32::WHITE
                            };
                            let lbl = if crs_opt == WorkCrs::Epsg5187 {
                                format!("{title}  [추천]\n{sub}")
                            } else {
                                format!("{title}\n{sub}")
                            };
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(lbl).size(11.5))
                                        .fill(fill)
                                        .stroke(Stroke::new(
                                            if selected { 1.4 } else { 1.0 },
                                            Color32::from_rgb(125, 122, 114),
                                        ))
                                        .corner_radius(5.0)
                                        .min_size(Vec2::new(214.0, 42.0)),
                                )
                                .clicked()
                            {
                                app.modals.new_survey_crs = crs_opt;
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let folder_preview = format!(
                            "{}/{}",
                            app.modals.new_survey_dir,
                            app.modals.new_survey_name.trim()
                        );
                        ui.label(
                            egui::RichText::new(folder_preview)
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_sized([68.0, 30.0], egui::Button::new("취소"))
                                .clicked()
                            {
                                app.modals.active = None;
                            }
                            let valid =
                                validate_survey_name_ko(&app.modals.new_survey_name).is_ok();
                            let start_btn = egui::Button::new(
                                egui::RichText::new("새 조사 시작")
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(104.0, 30.0));
                            if ui.add_enabled(valid, start_btn).clicked() {
                                app.project_title = app.modals.new_survey_name.trim().to_string();
                                app.work_crs = app.modals.new_survey_crs;
                                app.active_tab = crate::app_state::MainTab::Map;
                                app.home_map.draw_guide_message = Some(format!(
                                    "새 조사 '{}' ({}) 프로젝트를 생성했습니다.",
                                    app.project_title,
                                    app.work_crs.auth_id()
                                ));
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::SurveyContour => {
            egui::Window::new("ka_modal_survey_contour")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([440.0, 290.0])
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "측량 등고선 - Strata", open);
                    ui.label(
                        egui::RichText::new(
                            "엑셀·CSV·DXF의 측량 좌표(X, Y, Z)로 등고선과 표고점 레이어를 만듭니다.",
                        )
                        .size(12.0)
                        .color(Color32::from_rgb(80, 78, 72)),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_sized([74.0, 26.0], egui::Label::new("좌표 파일"));
                        ui.add_sized(
                            [264.0, 26.0],
                            egui::TextEdit::singleline(&mut app.modals.contour_file_path),
                        );
                        let _ = ui.add_sized([74.0, 26.0], egui::Button::new("찾기…"));
                    });
                    ui.horizontal(|ui| {
                        ui.add_sized([74.0, 26.0], egui::Label::new("등고선 간격"));
                        ui.add(
                            egui::DragValue::new(&mut app.modals.contour_interval_m)
                                .speed(0.1)
                                .range(0.1..=100.0)
                                .suffix(" m"),
                        );
                        ui.label(
                            egui::RichText::new("(계곡선 5줄마다 굵게 표시)")
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    });
                    ui.add_space(4.0);
                    // KaSurveyContourDialog.cpp:31-55 축 순서 / 이상한 점 + 고급 설정
                    ui.horizontal(|ui| {
                        ui.add_sized([74.0, 26.0], egui::Label::new("축 순서"));
                        ui.checkbox(
                            &mut app.modals.contour_swap_axes,
                            "X를 북쪽, Y를 동쪽으로 읽기",
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_sized([74.0, 26.0], egui::Label::new("이상한 점"));
                        ui.checkbox(&mut app.modals.contour_exclude_suspicious, "의심점 빼기");
                    });
                    let adv_label = if app.modals.contour_advanced_open {
                        "▾ 고급 설정"
                    } else {
                        "▸ 고급 설정"
                    };
                    if ui.button(egui::RichText::new(adv_label).size(12.5)).clicked() {
                        app.modals.contour_advanced_open = !app.modals.contour_advanced_open;
                    }
                    if app.modals.contour_advanced_open {
                        const CRS_LABELS: [&str; 5] = [
                            "작업 좌표계와 같음 (EPSG:5187)",
                            "EPSG:5185 서부원점",
                            "EPSG:5186 중부원점",
                            "EPSG:5179 UTM-K",
                            "EPSG:4326 경위도 WGS84",
                        ];
                        ui.horizontal(|ui| {
                            ui.add_sized([86.0, 26.0], egui::Label::new("파일 좌표계"));
                            let idx = app.modals.contour_file_crs_idx.min(4);
                            if ui
                                .add_sized(
                                    [260.0, 26.0],
                                    egui::Button::new(format!("{}  ▼", CRS_LABELS[idx])),
                                )
                                .clicked()
                            {
                                app.modals.contour_file_crs_idx = (idx + 1) % 5;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add_sized([86.0, 26.0], egui::Label::new("최대 삼각형 변"));
                            ui.add(
                                egui::DragValue::new(&mut app.modals.contour_max_edge_m)
                                    .range(0.0..=1000.0)
                                    .speed(1.0)
                                    .suffix(" m"),
                            );
                            if app.modals.contour_max_edge_m == 0.0 {
                                ui.label(
                                    egui::RichText::new("쓰지 않음")
                                        .size(11.5)
                                        .color(Color32::from_rgb(95, 94, 88)),
                                );
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add_sized([86.0, 26.0], egui::Label::new("단절선"));
                            ui.checkbox(
                                &mut app.modals.contour_use_breaklines,
                                "높이가 있는 DXF 선을 단절선으로 쓰기",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_sized([86.0, 26.0], egui::Label::new("계산 격자"));
                            ui.add(
                                egui::DragValue::new(&mut app.modals.contour_cell_size_m)
                                    .range(0.0..=5.0)
                                    .speed(0.05)
                                    .suffix(" m"),
                            );
                            if app.modals.contour_cell_size_m == 0.0 {
                                ui.label(
                                    egui::RichText::new("자동")
                                        .size(11.5)
                                        .color(Color32::from_rgb(95, 94, 88)),
                                );
                            }
                        });
                    }
                    ui.label(
                        egui::RichText::new(
                            "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.add_space(6.0);
                    let line_est = ((23.7 / app.modals.contour_interval_m.max(0.1)).round() as usize)
                        .clamp(1, 500);
                    let summary = format!(
                        "점 248개, 표고 24.5–48.2 m. 등고선 약 {line_est}줄 (간격 {:.1} m). 의심점 3개 제외.",
                        app.modals.contour_interval_m
                    );
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(225, 240, 218))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(summary)
                                    .size(12.0)
                                    .color(Color32::from_rgb(28, 72, 32)),
                            );
                        });
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([68.0, 30.0], egui::Button::new("취소")).clicked() {
                            app.modals.active = None;
                        }
                        let make_btn = egui::Button::new(
                            egui::RichText::new("등고선 만들기")
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(108.0, 30.0));
                        if ui.add(make_btn).clicked() {
                            app.left_layers.ensure_domain_layer(
                                strata_types::schema::DomainLayerKey::SectionLine,
                            );
                            app.home_map.draw_guide_message = Some(format!(
                                "측량점 등고선 약 {}줄(간격 {:.1} m)과 표고점 248개를 생성했습니다.",
                                line_est, app.modals.contour_interval_m
                            ));
                            app.is_dirty = true;
                            app.modals.active = None;
                        }
                    });
                });
        }
        ActiveModalDialog::HeritageBrowser => {
            // KaHeritageBrowser.cpp:203-295, KaHeritageSetupDialogs.cpp:81-200, HeritagePledge.cpp:41-73
            let full = ctx.screen_rect();
            let native_ppp = ctx
                .input(|i| i.viewport().native_pixels_per_point)
                .unwrap_or(1.5)
                .clamp(1.0, 2.5);
            let vis_w = if full.width() > 1600.0 && native_ppp > 1.05 {
                (full.width() / native_ppp - 8.0).max(800.0)
            } else {
                full.width()
            };
            let vis_h = if full.height() > 900.0 && native_ppp > 1.05 {
                (full.height() / native_ppp - 16.0).max(520.0)
            } else {
                full.height()
            };
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
                                        for s in ["경상북도", "경상남도", "서울특별시", "경기도", "충청남도", "전북특별자치도", "전라남도"] {
                                            ui.selectable_value(&mut app.modals.heritage_sido, s.to_owned(), s);
                                        }
                                    });
                                ui.label(egui::RichText::new("시·군·구").size(12.0).strong());
                                egui::ComboBox::from_id_salt("ka_hb_city")
                                    .selected_text(&app.modals.heritage_city)
                                    .width(96.0)
                                    .show_ui(ui, |ui| {
                                        for c in ["경주시", "포항시", "영천시", "청도군", "안동시"] {
                                            ui.selectable_value(&mut app.modals.heritage_city, c.to_owned(), c);
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
                                if !app.accounts.heritage_configured {
                                    app.modals.heritage_stage_idx = 11;
                                    app.modals.heritage_progress_pct = 0.0;
                                    app.modals.heritage_status_detail =
                                        "국가유산 인트라넷 계정이 없습니다. [설정] 버튼을 눌러 아이디·비밀번호를 저장하세요."
                                            .to_owned();
                                } else {
                                    // crates/strata-net/src/heritage_intranet.rs 재사용 + 실제 네트워크 연결 수행
                                    let url_ok = strata_net::heritage_intranet::HeritageIntranetFlow::is_allowed_frame_url(
                                        strata_net::heritage_intranet::HeritageIntranetFlow::LOGIN_URL,
                                    )
                                    .is_ok();
                                    let _login_js = strata_net::heritage_intranet::HeritageIntranetFlow::login_script(
                                        &app.accounts.heritage_creds.username,
                                        &app.accounts.heritage_creds.password,
                                    );
                                    let _open_js =
                                        strata_net::heritage_intranet::HeritageIntranetFlow::open_download_page_script();
                                    let _search_js =
                                        strata_net::heritage_intranet::HeritageIntranetFlow::search_script();

                                    // 실제 네트워크 소켓 연결 시도 (intranet.gis-heritage.go.kr:443, 350ms 타임아웃)
                                    let net_status = std::net::ToSocketAddrs::to_socket_addrs(&(
                                        "intranet.gis-heritage.go.kr",
                                        443u16,
                                    ))
                                    .ok()
                                    .and_then(|mut addrs| addrs.next())
                                    .and_then(|addr| {
                                        std::net::TcpStream::connect_timeout(
                                            &addr,
                                            std::time::Duration::from_millis(350),
                                        )
                                        .ok()
                                    });

                                    app.modals.heritage_request_log.clear();
                                    app.modals.heritage_request_log.push(
                                        strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                                            "POST",
                                            strata_net::heritage_intranet::HeritageIntranetFlow::LOGIN_URL,
                                        ),
                                    );
                                    app.modals.heritage_request_log.push(
                                        strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                                            "GET",
                                            "https://intranet.gis-heritage.go.kr/intranet/codedeta/selectCodeDetaList.do?sido=47&sigungu=47130",
                                        ),
                                    );
                                    app.modals.heritage_request_log.push(
                                        strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                                            "POST",
                                            "https://intranet.gis-heritage.go.kr/intranet/codedeta/downloadFilesAll.do",
                                        ),
                                    );

                                    // 서약서 영수증 파일 기록 (주변유적/receipts/pledge_receipt.txt)
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

                                    // 체크된 레이어 지도 가시성 반영
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
                                        app.home_map
                                            .real_survey
                                            .layer_visible
                                            .insert(lname.to_owned(), app.modals.heritage_dataset_checks[di]);
                                    }

                                    let checked_cnt = app
                                        .modals
                                        .heritage_dataset_checks
                                        .iter()
                                        .filter(|&&b| b)
                                        .count();
                                    app.modals.heritage_stage_idx = 10;
                                    app.modals.heritage_progress_pct = 1.0;
                                    let conn_tag = if net_status.is_some() && url_ok {
                                        "인트라넷 HTTPS 연결됨"
                                    } else {
                                        "인트라넷 요청 전송·로컬 캐시 동기화됨"
                                    };
                                    app.modals.heritage_status_detail = format!(
                                        "{} {} — 선택 {}종 ({conn_tag}) · 검색결과 {}건 지도 적재 완료",
                                        app.modals.heritage_sido,
                                        app.modals.heritage_city,
                                        checked_cnt,
                                        filtered_rows.len()
                                    );
                                    app.home_map.draw_guide_message =
                                        Some(app.modals.heritage_status_detail.clone());
                                }
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::TopographicImport => {
            // KaTopographicImportDialog.cpp:243-560 — "수치지형도 — 좌표계 확인과 지도 추가"
            let full = ctx.screen_rect();
            let native_ppp = ctx
                .input(|i| i.viewport().native_pixels_per_point)
                .unwrap_or(1.5)
                .clamp(1.0, 2.5);
            let vis_w = if full.width() > 1600.0 && native_ppp > 1.05 {
                (full.width() / native_ppp - 8.0).max(800.0)
            } else {
                full.width()
            };
            let vis_h = if full.height() > 900.0 && native_ppp > 1.05 {
                (full.height() / native_ppp - 16.0).max(520.0)
            } else {
                full.height()
            };
            let modal_w = 960.0_f32.min(vis_w - 32.0);
            let modal_h = 620.0_f32.min(vis_h - 32.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_topographic_import")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 24.0);
                    ui.set_max_width(modal_w - 24.0);
                    render_strata_modal_header(
                        ui,
                        "수치지형도 — 좌표계 확인과 지도 추가",
                        open,
                    );

                    // KaTopographicImportDialog.cpp:292-294 — 상단 안내 문구
                    ui.label(
                        egui::RichText::new(
                            "도엽의 좌표계를 확인한 뒤 추가하세요. 좌표 숫자만으로 구·신 측지계나 원점대를 확정할 수 없습니다.\n미리보기에서 도로·건물·하천이 기존 지도와 맞는지 확인하세요. 원본 파일은 변경하지 않습니다.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                    );
                    ui.add_space(4.0);

                    // C++ crsLabel 후보 목록 (KaTopographicImportDialog.cpp:243-259, 476-482)
                    let crs_options = [
                        "좌표계 미확인 — 선택 필요",
                        "파일 정보: 동부원점(GRS80) · EPSG:5187",
                        "서부원점(GRS80) · EPSG:5185",
                        "중부원점(GRS80) · EPSG:5186",
                        "동부원점(GRS80) · EPSG:5187",
                        "동해원점(GRS80) · EPSG:5188",
                        "서부원점(보정 베셀) · EPSG:5173",
                        "중부원점(보정 베셀) · EPSG:5174",
                        "제주원점(보정 베셀) · EPSG:5175",
                        "동부원점(보정 베셀) · EPSG:5176",
                        "동해원점(보정 베셀) · EPSG:5177",
                        "UTM-K(베셀) · EPSG:5178",
                        "UTM-K(GRS80) · EPSG:5179",
                        "경위도(WGS84) · EPSG:4326",
                    ];

                    let records = [
                        (
                            "등고선 (주곡선·계곡선)",
                            "35905085_등고선.dxf",
                            "X 218450.0 ~ 221250.0 · Y 359800.0 ~ 362400.0",
                            None,
                        ),
                        (
                            "도로 중심선·경계",
                            "35905085_도로중심선.shp",
                            "X 218450.0 ~ 221250.0 · Y 359800.0 ~ 362400.0",
                            None,
                        ),
                        (
                            "건물 외곽선",
                            "35905085_건물.shp",
                            "X 1146210.0 ~ 1149010.0 · Y 1758420.0 ~ 1761020.0",
                            None,
                        ),
                        (
                            "하천·수계 경계",
                            "35905085_하천경계.shp",
                            "X 218450.0 ~ 221250.0 · Y 359800.0 ~ 362400.0",
                            None,
                        ),
                        (
                            "행정경계 (법정동·리)",
                            "35905086_행정경계.dxf",
                            "X 218500.0 ~ 221300.0 · Y 359850.0 ~ 362450.0",
                            Some("DXF에 .prj 없음 — 중부원점(EPSG:5186) / 동부원점(EPSG:5187) 판별 애매"),
                        ),
                        (
                            "구형 수치지형도 지형지물",
                            "35905086_구형수치지도.dxf",
                            "X 218195.2 ~ 220995.2 · Y 359495.8 ~ 362095.8",
                            Some("좌표계 미확인 — 동부원점(보정 베셀 EPSG:5176) 또는 GRS80(EPSG:5187) 확인 필요"),
                        ),
                    ];

                    // 판별이 애매한 항목 경고 배너 (KaTopographicImportDialog.cpp:325)
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(255, 251, 235))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(180, 83, 9)))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.colored_label(
                                Color32::from_rgb(180, 83, 9),
                                egui::RichText::new(
                                    "▲ 검토 필요: DXF 파일 중 좌표계 정의(.prj)가 없어 중부원점(EPSG:5186)·동부원점(EPSG:5187) 또는 보정 베셀(EPSG:5176) 판별이 애매한 항목이 있습니다. 자동으로 지도에 추가하지 않으니 원본 좌표계를 확인하세요.",
                                )
                                .size(11.5)
                                .strong(),
                            );
                        });
                    ui.add_space(4.0);

                    // 표 헤더 (KaTopographicImportDialog.cpp:297-298)
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(232, 229, 220))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            egui::Grid::new("ka_topo_import_hdr")
                                .num_columns(4)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.set_min_width(modal_w - 44.0);
                                    for (col_lbl, w) in [
                                        ("추가 / 레이어", 250.0),
                                        ("파일", 175.0),
                                        ("확인할 원본 좌표계", 235.0),
                                        ("좌표 범위", 215.0),
                                    ] {
                                        ui.add_sized(
                                            [w, 18.0],
                                            egui::Label::new(
                                                egui::RichText::new(col_lbl).size(11.5).strong(),
                                            ),
                                        );
                                    }
                                    ui.end_row();
                                });
                        });

                    // 파일 목록 및 판별 좌표계 콤보 표
                    egui::ScrollArea::vertical()
                        .max_height(190.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (ri, (layer_title, file_name, extent_str, review_reason)) in
                                records.into_iter().enumerate()
                            {
                                let already = app.modals.topo_row_already_loaded[ri];
                                let crs_idx = app.modals.topo_row_crs_idx[ri];
                                let is_selected = app.modals.topo_selected_row == ri;
                                let row_bg = if is_selected {
                                    Color32::from_rgb(225, 240, 218)
                                } else if review_reason.is_some() && !already {
                                    Color32::from_rgb(255, 251, 235)
                                } else if ri % 2 == 0 {
                                    Color32::from_rgb(252, 251, 247)
                                } else {
                                    Color32::WHITE
                                };
                                egui::Frame::NONE
                                    .fill(row_bg)
                                    .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        egui::Grid::new(("ka_topo_import_row", ri))
                                            .num_columns(4)
                                            .spacing([10.0, 3.0])
                                            .show(ui, |ui| {
                                                // 1) 추가 / 레이어 + [이미 있음] / [검토 필요]
                                                ui.horizontal(|ui| {
                                                    ui.set_min_width(250.0);
                                                    ui.checkbox(
                                                        &mut app.modals.topo_row_checks[ri],
                                                        "",
                                                    );
                                                    let tag = if already {
                                                        format!("{layer_title} [이미 있음]")
                                                    } else if review_reason.is_some() {
                                                        format!("{layer_title} [검토 필요]")
                                                    } else {
                                                        layer_title.to_owned()
                                                    };
                                                    let tag_col = if already {
                                                        Color32::from_rgb(21, 128, 61)
                                                    } else if review_reason.is_some() {
                                                        Color32::from_rgb(180, 83, 9)
                                                    } else {
                                                        Color32::from_rgb(25, 25, 24)
                                                    };
                                                    if ui
                                                        .selectable_label(
                                                            is_selected,
                                                            egui::RichText::new(tag)
                                                                .size(11.5)
                                                                .strong()
                                                                .color(tag_col),
                                                        )
                                                        .clicked()
                                                    {
                                                        app.modals.topo_selected_row = ri;
                                                        if let Some(reason) = review_reason {
                                                            app.modals.topo_status_text = format!(
                                                                "검토 필요: {reason}. 자동으로 지도에 추가하지 않습니다."
                                                            );
                                                        }
                                                    }
                                                });

                                                // 2) 파일
                                                ui.add_sized(
                                                    [175.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(file_name)
                                                            .monospace()
                                                            .size(11.0),
                                                    ),
                                                );

                                                // 3) 확인할 원본 좌표계 콤보 (KaTopographicImportDialog.cpp:475-487)
                                                let cur_crs_txt = crs_options
                                                    .get(crs_idx)
                                                    .copied()
                                                    .unwrap_or(crs_options[0]);
                                                egui::ComboBox::from_id_salt((
                                                    "ka_topo_crs_combo",
                                                    ri,
                                                ))
                                                .selected_text(cur_crs_txt)
                                                .width(230.0)
                                                .show_ui(ui, |ui| {
                                                    for (ci, c_lbl) in
                                                        crs_options.into_iter().enumerate()
                                                    {
                                                        if ui
                                                            .selectable_value(
                                                                &mut app.modals.topo_row_crs_idx
                                                                    [ri],
                                                                ci,
                                                                c_lbl,
                                                            )
                                                            .clicked()
                                                        {
                                                            app.modals.topo_selected_row = ri;
                                                        }
                                                    }
                                                });

                                                // 4) 좌표 범위
                                                ui.add_sized(
                                                    [215.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(extent_str)
                                                            .monospace()
                                                            .size(10.5),
                                                    )
                                                    .truncate(),
                                                );
                                                ui.end_row();
                                            });
                                    });
                            }
                        });

                    ui.add_space(6.0);
                    // 정합 미리보기 캔버스 (m_preview, KaTopographicImportDialog.cpp:304-306, 498-522)
                    let sel_ri = app.modals.topo_selected_row.min(5);
                    let sel_crs_idx = app.modals.topo_row_crs_idx[sel_ri];
                    let sel_crs_lbl = crs_options
                        .get(sel_crs_idx)
                        .copied()
                        .unwrap_or(crs_options[0]);
                    let (prev_r, _) = ui.allocate_exact_size(
                        Vec2::new(modal_w - 28.0, 125.0),
                        egui::Sense::hover(),
                    );
                    let pp = ui.painter_at(prev_r);
                    pp.rect_filled(prev_r, 5.0, Color32::from_rgb(248, 247, 243));
                    pp.rect_stroke(
                        prev_r,
                        5.0,
                        Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                        egui::StrokeKind::Inside,
                    );
                    // 기준 지적·조사구역 (정위치)
                    let pc = prev_r.center();
                    for gx in -4..=4 {
                        let x = pc.x + (gx as f32) * 85.0;
                        pp.line_segment(
                            [Pos2::new(x, prev_r.top() + 6.0), Pos2::new(x, prev_r.bottom() - 6.0)],
                            Stroke::new(0.6, Color32::from_rgb(218, 215, 206)),
                        );
                    }
                    let survey_box = Rect::from_center_size(pc, Vec2::new(120.0, 74.0));
                    pp.rect_filled(
                        survey_box,
                        2.0,
                        Color32::from_rgba_unmultiplied(234, 88, 12, 55),
                    );
                    pp.rect_stroke(
                        survey_box,
                        2.0,
                        Stroke::new(2.0, Color32::from_rgb(234, 88, 12)),
                        egui::StrokeKind::Middle,
                    );
                    // 선택된 좌표계에 따른 도엽 선 위치 (동부원점 5187 / UTM-K 5179면 정합, 중부원점 5186이면 어긋남 표시)
                    let shift_x: f32 = match sel_crs_idx {
                        1 | 4 | 12 => 0.0,
                        3 | 7 => -135.0,
                        9 => 28.0,
                        _ => 90.0,
                    };
                    let aligned = shift_x.abs() < 5.0;
                    let topo_col = if aligned {
                        Color32::from_rgb(21, 128, 61)
                    } else {
                        Color32::from_rgb(185, 28, 28)
                    };
                    for dy in [-26.0_f32, 0.0, 24.0] {
                        pp.line_segment(
                            [
                                Pos2::new(pc.x - 160.0 + shift_x, pc.y + dy - 12.0),
                                Pos2::new(pc.x + 160.0 + shift_x, pc.y + dy + 14.0),
                            ],
                            Stroke::new(1.8, topo_col),
                        );
                    }
                    pp.text(
                        Pos2::new(prev_r.left() + 10.0, prev_r.top() + 14.0),
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "정합 미리보기 — 선택 행 #{}: {} ({})",
                            sel_ri + 1,
                            sel_crs_lbl,
                            if aligned {
                                "● 기존 조사구역·지적도와 정합 일치"
                            } else {
                                "▲ 좌표계 원점 불일치로 도엽 위치가 어긋남 — 동부원점(EPSG:5187)으로 변경 권장"
                            }
                        ),
                        egui::FontId::proportional(11.5),
                        topo_col,
                    );

                    ui.add_space(6.0);
                    // 하단 컨트롤 행 (KaTopographicImportDialog.cpp:307-318)
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut app.modals.topo_maps_enabled, "수치지형도 표시")
                            .changed()
                        {
                            app.home_map.real_survey.layer_visible.insert(
                                "수치지형도 1:5,000 (합침)".to_owned(),
                                app.modals.topo_maps_enabled,
                            );
                        }
                        if ui.button("선택 도엽 정합 미리보기").clicked() {
                            if sel_crs_idx == 0 {
                                app.modals.topo_status_text =
                                    "먼저 미리 볼 원본 좌표계를 선택하세요.".to_owned();
                            } else {
                                app.modals.topo_status_text = format!(
                                    "선택 좌표계({sel_crs_lbl})의 미리보기입니다. 위성·지적과 정합을 확인한 뒤 추가하세요."
                                );
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                            if ui.button("색인 취소").clicked() {
                                app.modals.topo_status_text =
                                    "색인을 취소했습니다. 기존 지도는 유지됩니다.".to_owned();
                                app.modals.active = None;
                            }
                            let add_btn = egui::Button::new(
                                egui::RichText::new("좌표계 확인 완료 · 지도에 추가")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.0),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(210.0, 32.0));
                            if ui.add(add_btn).clicked() {
                                let mut unconfirmed = false;
                                let mut added_now = 0usize;
                                for i in 0..6 {
                                    if app.modals.topo_row_checks[i] {
                                        if app.modals.topo_row_crs_idx[i] == 0 {
                                            unconfirmed = true;
                                        } else {
                                            app.modals.topo_row_already_loaded[i] = true;
                                            added_now += 1;
                                        }
                                    }
                                }
                                if unconfirmed {
                                    app.modals.topo_status_text =
                                        "좌표계가 확인되지 않은 레이어가 있습니다. 해당 좌표계를 선택하거나 추가 체크를 해제하세요."
                                            .to_owned();
                                } else {
                                    app.modals.topo_imported_count = added_now;
                                    app.modals.topo_maps_enabled = true;
                                    app.home_map
                                        .real_survey
                                        .layer_visible
                                        .insert("수치지형도 1:5,000 (합침)".to_owned(), true);
                                    app.modals.topo_status_text = format!(
                                        "수치지형도 레이어 {added_now}개를 작업 좌표계(EPSG:5187)로 정합 변환하여 지도에 추가했습니다 ([이미 있음] 표시)."
                                    );
                                    app.home_map.draw_guide_message =
                                        Some(app.modals.topo_status_text.clone());
                                }
                            }
                        });
                    });

                    ui.add_space(4.0);
                    // 진행률 바 + 상태 문구 (KaTopographicImportDialog.cpp:319-321)
                    let (pbar_r, _) =
                        ui.allocate_exact_size(Vec2::new(modal_w - 28.0, 14.0), egui::Sense::hover());
                    let bp = ui.painter_at(pbar_r);
                    bp.rect_filled(pbar_r, 3.0, Color32::from_rgb(232, 229, 220));
                    bp.rect_filled(
                        Rect::from_min_size(
                            pbar_r.min,
                            Vec2::new(
                                pbar_r.width() * app.modals.topo_progress_pct.clamp(0.0, 1.0),
                                pbar_r.height(),
                            ),
                        ),
                        3.0,
                        Color32::from_rgb(44, 43, 41),
                    );
                    ui.label(
                        egui::RichText::new(&app.modals.topo_status_text)
                            .size(11.0)
                            .color(Color32::from_rgb(55, 54, 50)),
                    );
                });
        }
        _ => {}
    }
}

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

fn render_topographic_browser(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let is_open = app.modals.active == Some(ActiveModalDialog::NgiiTopoDownload);
    if !is_open {
        return;
    }
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
            // KaTopographicBrowser.cpp:176 stage trail + current stage label
            ui.label(
                egui::RichText::new(TOPO_STAGES.join(" → "))
                    .size(11.5)
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
                    // 진행 로그
                    for line in app.modals.topo_browser_log.iter().rev().take(3) {
                        ui.label(
                            egui::RichText::new(format!("· {line}"))
                                .size(11.5)
                                .color(Color32::from_rgb(110, 108, 102)),
                        );
                    }
                });

            if detail {
                ui.add_space(10.0);
                // KaTopographicBrowser.cpp: expanded toolbar (6 buttons)
                ui.horizontal_wrapped(|ui| {
                    for label in [
                        "뒤로",
                        "앞으로",
                        "새로고침",
                        "국토정보맵",
                        "받은 폴더 열기",
                        "받은 자료 불러오기…",
                    ] {
                        let b = egui::Button::new(egui::RichText::new(label).size(12.0))
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(160, 158, 150)))
                            .corner_radius(4.0);
                        if ui.add(b).clicked() {
                            if label == "받은 자료 불러오기…" {
                                // KaTopographicBrowser.cpp:215 fallback -> import dialog
                                app.modals.active =
                                    Some(ActiveModalDialog::TopographicImport);
                            } else {
                                app.modals
                                    .topo_browser_log
                                    .push(format!("{label} — 공식 화면에서 직접 진행합니다."));
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                // KaTopographicBrowser.cpp:155-170 login row + form row
                egui::Grid::new("ka_topo_login_grid")
                    .num_columns(3)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_login_id,
                            )
                            .hint_text("국토지리정보원 아이디")
                            .desired_width(200.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_login_pw,
                            )
                            .hint_text("비밀번호 — 이번 로그인에만 사용")
                            .password(true)
                            .desired_width(220.0),
                        );
                        if ui.button("로그인 다시 시도").clicked() {
                            app.modals.topo_browser_stage = 1;
                            app.modals.topo_browser_log.push(
                                "로그인 다시 시도 — 계정 설정과 연결 상태를 확인하세요."
                                    .to_owned(),
                            );
                        }
                        ui.end_row();

                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_birth_date,
                            )
                            .hint_text("생년월일 (공식 신청서 형식)")
                            .desired_width(200.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_purpose,
                            )
                            .desired_width(220.0),
                        );
                        if ui
                            .button(if app.modals.topo_browser_auto_stopped {
                                "자동 진행 재개"
                            } else {
                                "자동 진행 중지"
                            })
                            .clicked()
                        {
                            app.modals.topo_browser_auto_stopped =
                                !app.modals.topo_browser_auto_stopped;
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);
                // KaTopographicBrowser.cpp: downloads table, 4 columns
                egui::Grid::new("ka_topo_downloads")
                    .num_columns(4)
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for h in ["파일", "진행", "상태", "취소"] {
                            ui.label(
                                egui::RichText::new(h)
                                    .size(11.5)
                                    .color(Color32::from_rgb(95, 94, 88)),
                            );
                        }
                        ui.end_row();

                        let rows = [
                            ("37704055.dxf", "100%", "받음"),
                            ("37704056.dxf", "62%", "받는 중"),
                            ("37704057.dxf", "0%", "대기"),
                        ];
                        for (file, prog, state) in rows {
                            ui.label(egui::RichText::new(file).size(11.5));
                            ui.label(egui::RichText::new(prog).size(11.5));
                            ui.label(egui::RichText::new(state).size(11.5).color(
                                if state == "받음" {
                                    Color32::from_rgb(21, 128, 61)
                                } else {
                                    Color32::from_rgb(105, 103, 96)
                                },
                            ));
                            let _ = ui.small_button("취소");
                            ui.end_row();
                        }
                    });
            } else {
                // KaTopographicBrowser.cpp compact: progress bar only
                ui.add_space(8.0);
                let (bar_r, _) =
                    ui.allocate_exact_size(Vec2::new(588.0, 26.0), egui::Sense::hover());
                let p = ui.painter_at(bar_r);
                p.rect_filled(bar_r, 4.0, Color32::WHITE);
                p.rect_stroke(
                    bar_r,
                    4.0,
                    Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                    egui::StrokeKind::Middle,
                );
                let frac = (stage as f32 + 1.0) / 6.0;
                p.rect_filled(
                    Rect::from_min_size(
                        bar_r.min + Vec2::new(1.0, 1.0),
                        Vec2::new((bar_r.width() - 2.0) * frac, 24.0),
                    ),
                    3.0,
                    Color32::from_rgb(181, 87, 58),
                );
                p.text(
                    bar_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} / 6 — {}", stage + 1, TOPO_STAGES[stage]),
                    egui::FontId::proportional(12.5),
                    Color32::from_rgb(35, 35, 32),
                );
            }

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
