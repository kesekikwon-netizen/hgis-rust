//! `dialogs_download_print` 에서 옮긴 「TrenchGenerator」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};

pub(crate) fn render_trench_generator(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
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
            )
            .on_hover_text(
                "조사구역이 여러 개면 선택한 곳만, 선택이 없으면 마지막에 그린 곳만 깝니다.",
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
            })
            .response
            .on_hover_text(
                "시굴은 조사구역의 10%, 표본은 2%로 배치합니다. 폭 2 m·길이 20 m 이내에서 길이와 둑 간격을 맞추며, 모든 격자가 구역 안에 들어갑니다.",
            );

            ui.checkbox(
                &mut app.modals.trench_slope_align,
                egui::RichText::new("지형 사면 방향으로 넣기(등고선 직교)").size(13.0),
            )
            .on_hover_text(
                "트렌치 장축을 오르막 쪽으로 돌립니다. 등고선과 나란히 넣으면 같은 층만 따라가 층서를 못 읽습니다. DEM을 올려야 씁니다.",
            );
            // KaTrenchDialog.cpp:54-57,227-237 — m_terrainInfo
            ui.label(
                egui::RichText::new(
                    if app.modals.trench_slope_align {
                        "지형: 오르막 135° · 경사 4.2% — 트렌치 장축을 이 방향으로 넣습니다."
                    } else {
                        "지형 자료가 없어 남북으로 깝니다(DEM·수치지형도 등고선 없음). 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."
                    },
                )
                .size(11.5)
                .color(Color32::from_rgb(95, 94, 88)),
            );

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("모양").size(13.0));
                let shape_lbl = format!("{}  ▼", app.modals.trench_shape.label_ko());
                if ui
                    .add_sized([286.0, 28.0], egui::Button::new(shape_lbl))
                    .on_hover_text(
                        "ㄱ·ㄴ자는 팔 15 m 그대로 구역 안에 온전히 들어가는 것만 놓아, 비율이 목표를 조금 넘습니다.",
                    )
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
            )
            .on_hover_text(
                "ㄱ·ㄴ자 시굴·표본에서 꺾인 바깥 모서리를 좌표격자 20 m 교점에, 방향을 좌표격자 방향에 맞춥니다. 트렌치끼리는 1 m 이상 띄웁니다.",
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
                    )
                    .on_hover_text(
                        "트렌치 사이 간격입니다. 간격을 키우면 시굴 비율이 내려갑니다.",
                    );
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("이름 접두").size(13.0));
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
                )
                .on_hover_text(
                    "격자 회전(북 기준 시계 방향). 「구역에 깔기」를 누르면 회전된 격자로 다시 배치합니다.",
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
                    "위가 북쪽입니다".to_owned()
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
                // KaTrenchDialog.cpp:114-135 — m_afterPlace (배치 후 편집)
                if !app.home_map.real_survey.trenches.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "깐 격자는 맵에서 끌어 옮깁니다. 개별 편집은 트렌치 하나 이동·삭제입니다.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .button("개별 편집")
                            .on_hover_text(
                                "그래픽처럼 트렌치를 하나씩 선택·이동·삭제합니다.",
                            )
                            .clicked()
                        {
                            app.home_map.draw_guide_message = Some(
                                "시굴격자 개별 편집 모드 — 트렌치를 하나씩 선택해 이동하거나 삭제하세요."
                                    .to_owned(),
                            );
                        }
                        if ui
                            .button("전체 이동")
                            .on_hover_text(
                                "격자를 끌어다 놓아 전체를 옮깁니다. Ctrl+Z로 되돌립니다.",
                            )
                            .clicked()
                        {
                            app.home_map.draw_guide_message = Some(
                                "시굴격자 전체 이동 모드 — 격자를 끌어 전체 위치를 옮기세요."
                                    .to_owned(),
                            );
                        }
                    });
                }
            }
            // KaTrenchDialog.cpp:137-154 — btnRow (하단 실행 버튼)
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let place_btn = egui::Button::new(
                    egui::RichText::new("구역에 깔기")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(120.0, 30.0));
                if ui
                    .add(place_btn)
                    .on_hover_text(
                        "조사구역 위에 격자를 다시 깝니다. 깐 뒤 맵에서 끌어 옮기세요.",
                    )
                    .clicked()
                {
                    app.home_map
                        .real_survey
                        .layer_visible
                        .insert("시굴격자".to_owned(), true);
                    app.home_map.draw_guide_message = Some(format!(
                        "시굴격자 {}개를 조사구역 위에 배치했습니다 (방위 {:.0}°).",
                        app.home_map.real_survey.trenches.len(),
                        app.modals.trench_azimuth_deg
                    ));
                }
                if ui
                    .button("맵에 찍기")
                    .on_hover_text("맵을 한 번 찍어 그 점을 원점으로 놓습니다.")
                    .clicked()
                {
                    app.modals.trench_auto_place = false;
                    app.home_map.draw_guide_message = Some(
                        "맵을 한 번 찍어 시굴격자 원점을 지정하세요.".to_owned(),
                    );
                    *open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        *open = false;
                    }
                });
            });
        });
}
