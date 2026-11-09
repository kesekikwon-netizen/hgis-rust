//! `dialogs_download_print` 에서 옮긴 「NewSurvey」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use strata_storage::gpkg::{validate_survey_name_ko, KA_LONGITUDE_RULER_CITIES};
use strata_types::crs::WorkCrs;

pub(crate) fn render_new_survey(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_new_survey")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([
            460.0_f32.min(ctx.screen_rect().width() - 32.0),
            440.0_f32.min(ctx.screen_rect().height() - 32.0),
        ])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "새 조사 - Strata", open);
            ui.label(egui::RichText::new("조사명").strong().size(13.0));
            ui.add_sized(
                [440.0, 28.0],
                egui::TextEdit::singleline(&mut app.modals.new_survey_name)
                    .hint_text("예: 병산동"),
            );
            if let Err(err_msg) = validate_survey_name_ko(&app.modals.new_survey_name) {
                ui.label(
                    egui::RichText::new(err_msg)
                        .size(11.5)
                        .color(Color32::from_rgb(185, 28, 28)),
                );
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("작업 좌표계").strong().size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("동경 128°를 경계로 나뉩니다")
                            .size(11.0)
                            .color(Color32::from_rgb(107, 114, 128)),
                    );
                });
            });
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
            // KaLongitudeRuler.cpp:40-107 — 0.1° 눈금. 정수(7px, 진함),
            // 반(5px, 옅음), 나머지(3px, 옅음)로 높이와 색을 나눈다.
            for d10 in 1260..=1300 {
                let deg = f64::from(d10) / 10.0;
                let is_whole = d10 % 10 == 0;
                let is_half = d10 % 5 == 0;
                let tick_h = if is_whole {
                    7.0
                } else if is_half {
                    5.0
                } else {
                    3.0
                };
                let tick_c = if is_whole {
                    Color32::from_rgb(31, 35, 40)
                } else {
                    Color32::from_rgb(209, 205, 197)
                };
                let tx = x_of(deg);
                rp.line_segment(
                    [Pos2::new(tx, bar_y - tick_h), Pos2::new(tx, bar_y)],
                    Stroke::new(if is_whole { 1.2 } else { 1.0 }, tick_c),
                );
            }
            // KaLongitudeRuler.cpp:92-106 — 지도 중심(동경 129.2°) 클레이 마커
            let center_lon = 129.2_f64;
            let cx = x_of(center_lon);
            let clay = Color32::from_rgb(181, 87, 58);
            rp.line_segment(
                [
                    Pos2::new(cx, ruler_r.top() + 2.0),
                    Pos2::new(cx, bar_y + 2.0),
                ],
                Stroke::new(1.5, clay),
            );
            rp.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(cx, bar_y + 1.0),
                    Pos2::new(cx - 4.0, bar_y - 6.0),
                    Pos2::new(cx + 4.0, bar_y - 6.0),
                ],
                clay,
                Stroke::NONE,
            ));
            rp.text(
                Pos2::new(cx + 6.0, bar_y - 10.0),
                egui::Align2::LEFT_CENTER,
                "129.2°",
                egui::FontId::proportional(10.0),
                clay,
            );
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
            // KaNewSurveyDialog.cpp:160-176 updateState — 지도 중심 경도를 실제로
            // 계산해 「지금 지도 중심(동경 N.N°)은 ○○원점(NNNN) 구역입니다.」 를 만든다.
            let map_lon = {
                let [cx, cy] = app.home_map.viewport.center_xy;
                match strata_geo::crs_transform::TmParams::for_crs(
                    strata_types::crs::CrsCode::Epsg5187,
                ) {
                    Some(p) => strata_geo::crs_transform::tm_to_lonlat(cx, cy, &p)[0],
                    None => 129.2,
                }
            };
            // KaNewSurveyDialog.cpp:205-212 suggestedWorkCrs — 126~128 중부, 128~130 동부
            let suggested = if (126.0..128.0).contains(&map_lon) {
                Some(WorkCrs::Epsg5186)
            } else if (128.0..130.0).contains(&map_lon) {
                Some(WorkCrs::Epsg5187)
            } else {
                None
            };
            if let Some(sug) = suggested {
                let zone = match sug {
                    WorkCrs::Epsg5186 => "중부원점(5186)",
                    _ => "동부원점(5187)",
                };
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "지금 지도 중심(동경 {map_lon:.1}°)은 {zone} 구역입니다."
                    ))
                    .size(11.5)
                    .strong()
                    .color(Color32::from_rgb(31, 35, 40)),
                );
            }
            ui.label(
                egui::RichText::new(
                    "경계 가까운 곳은 발주처·측량 성과의 원점을 따르세요.",
                )
                .size(11.0)
                .color(Color32::from_rgb(107, 114, 128)),
            );
            ui.label(
                egui::RichText::new(
                    "작업 좌표계는 나중에 화면 아래에서 바꿀 수 있습니다.",
                )
                .size(11.0)
                .color(Color32::from_rgb(107, 114, 128)),
            );
            ui.label(
                egui::RichText::new("제출 파일은 언제나 5179로 바뀝니다.")
                    .size(11.0)
                    .color(Color32::from_rgb(107, 114, 128)),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
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
                        egui::RichText::new("다음")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(104.0, 30.0));
                    if ui.add_enabled(valid, start_btn).clicked() {
                        // C++ `MainWindowSession.cpp:690‑701` — 이름 확인 뒤 저장 폴더를
                        // 고르고, 그 폴더에 새 .gpkg 를 만든다. 폴더 선택을 취소하면
                        // 아무 것도 바꾸지 않는다(현재 조사를 그대로 둔다).
                        let dir = rfd::FileDialog::new()
                            .set_title("저장 폴더")
                            .set_directory(&app.modals.new_survey_dir)
                            .pick_folder();
                        if let Some(dir) = dir {
                            let name = app.modals.new_survey_name.trim().to_owned();
                            let path = dir.join(format!("{name}.gpkg"));
                            let crs = app.modals.new_survey_crs;
                            match strata_storage::gpkg::GpkgRepository::create_new_survey(
                                &path, crs,
                            ) {
                                Ok(_) => {
                                    app.start_new_survey_project_with_path(&name, &path, crs);
                                    app.modals.new_survey_dir =
                                        dir.to_string_lossy().into_owned();
                                    app.modals.active = None;
                                }
                                Err(e) => {
                                    app.modals.new_survey_error =
                                        Some(format!("새 조사를 만들지 못했습니다: {e}"));
                                }
                            }
                        }
                    }
                });
            });
        });
}
