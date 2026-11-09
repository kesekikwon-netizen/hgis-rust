//! `dialogs_download_print` 에서 옮긴 「DrawingPrint」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use strata_layout::sheet_composer::SheetPaperSize;
use strata_layout::tile_print::{compute_tile_print_plan_with_overlap, format_print_dispatched_message, verify_requested_device_paper};

pub(crate) fn render_drawing_print(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
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
                    // KaPrintDialog.cpp:126-137 — 나눠 찍을 때만 이웃 장과 겹치는 폭을 고른다
                    if !app.modals.print_single_sheet {
                        ui.add_space(12.0);
                        ui.label("겹침");
                        ui.add(
                            egui::DragValue::new(&mut app.modals.print_overlap_mm)
                                .range(0.0..=30.0)
                                .speed(1.0)
                                .fixed_decimals(0)
                                .suffix(" mm"),
                        )
                        .on_hover_text(
                            "이웃 장과 같은 그림이 겹치는 폭입니다. 붙일 때 맞추기 쉽습니다.",
                        );
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
                // KaPrintDialog.cpp:157-166 — 자름 표시와 순서 안내는 나눠 찍을 때만 쓴다
                if !app.modals.print_single_sheet {
                    cols[0].horizontal(|ui| {
                        ui.add_space(74.0);
                        ui.vertical(|ui| {
                            ui.checkbox(
                                &mut app.modals.print_marks,
                                "자르는 점선·장 번호·5 cm 확인선 넣기",
                            );
                            ui.checkbox(
                                &mut app.modals.print_overview,
                                "맨 앞에 붙이는 순서 안내 한 장 넣기",
                            );
                        });
                    });
                }
                cols[0].add_space(126.0);
                let tile_plan =
                    compute_tile_print_plan_with_overlap(
                        &app.drawing.sheet,
                        app.modals.print_paper,
                        app.modals.print_overlap_mm,
                    );
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
                compute_tile_print_plan_with_overlap(
                        &app.drawing.sheet,
                        app.modals.print_paper,
                        app.modals.print_overlap_mm,
                    );
            let page_cnt = if app.modals.print_single_sheet {
                1
            } else {
                (tile_plan.rows * tile_plan.cols).max(2)
            };
            // KaPrintDialog.cpp:193-204 — 「나눈 장을 PDF로 저장…」은 왼쪽, 인쇄·닫기는 오른쪽
            ui.horizontal(|ui| {
                let save_pdf =
                    egui::Button::new(egui::RichText::new("나눈 장을 PDF로 저장…").size(12.5))
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(150.0, 34.0));
                if ui
                    .add(save_pdf)
                    .on_hover_text(
                        "프린터가 없는 PC나 출력소에서 찍을 수 있게 한 장이 한 쪽인 PDF로 저장합니다.",
                    )
                    .clicked()
                {
                    if let Ok(path) = app.drawing.export_pdf_to_temp() {
                        app.home_map.draw_guide_message = Some(format!(
                            "나눈 장을 PDF로 저장했습니다: {}",
                            path.display()
                        ));
                    }
                }
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
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
                    },
                );
            });
        });
}
