//! 「도면 인쇄」 대화상자 왼쪽 칸 — 프린터·방법·용지·해상도·자름 표시·요약 줄.
//!
//! `dlg_drawing_print.rs` 가 300줄 규칙에 맞춰 나눈 파일이다 (`KaPrintDialog.cpp:126-166`).
//! 원래 `ui.columns(2, …)` 의 왼쪽 칸 본문을 **그대로** 옮겼고, 위젯 순서·수치·문구는
//! 하나도 바꾸지 않았다. 오른쪽 칸 미리보기가 쓰는 요약 두 줄만 돌려준다.
//!
//! `cols[0]` 이던 수신자를 `left` 로 이름만 바꿨다(같은 칸을 가리킨다).

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use strata_layout::sheet_composer::SheetPaperSize;
use strata_layout::tile_print::compute_tile_print_plan_with_overlap;

/// 왼쪽 칸을 그리고 `(용지 요약, 축척 요약)` 을 돌려준다.
pub(crate) fn render_print_options(
    app: &mut StrataAppState,
    left: &mut egui::Ui,
) -> (String, String) {
                left.horizontal(|ui| {
                    ui.add_sized([68.0, 30.0], egui::Label::new("프린터"));
                    let _ = ui.add(
                        egui::Button::new("ezViewX Virtual Image Printer              ▼")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.3, Color32::from_rgb(35, 35, 32)))
                            .corner_radius(5.0)
                            .min_size(Vec2::new(258.0, 30.0)),
                    );
                });
                left.horizontal(|ui| {
                    ui.add_space(74.0);
                    let _ = ui.add(
                        egui::Button::new("프린터 속성...")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
                            .corner_radius(5.0)
                            .min_size(Vec2::new(84.0, 24.0)),
                    );
                });
                left.add_space(8.0);
                left.horizontal_top(|ui| {
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
                left.add_space(8.0);
                left.horizontal(|ui| {
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
                left.add_space(8.0);
                left.horizontal(|ui| {
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
                    left.horizontal(|ui| {
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
                left.add_space(126.0);
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
                let (top_rule, _) = left.allocate_exact_size(
                    Vec2::new(left.available_width(), 2.0),
                    egui::Sense::hover(),
                );
                left.painter().line_segment(
                    [top_rule.left_center(), top_rule.right_center()],
                    Stroke::new(1.2, Color32::from_rgb(45, 44, 40)),
                );
                for (k, v) in [
                    ("용지", paper_summary.as_str()),
                    ("인쇄 크기", "실제 크기 100%"),
                    ("축척", scale_summary.as_str()),
                ] {
                    left.horizontal(|ui| {
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
                left.separator();
    (paper_summary, scale_summary)
}
