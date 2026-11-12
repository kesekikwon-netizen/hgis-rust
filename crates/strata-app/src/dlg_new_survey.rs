//! `dialogs_download_print` 에서 옮긴 「NewSurvey」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use strata_storage::gpkg::validate_survey_name_ko;
use strata_types::crs::WorkCrs;
use strata_types::error::StrataError;

#[path = "dlg_new_survey_ruler.rs"]
mod dlg_new_survey_ruler;

use dlg_new_survey_ruler::paint_longitude_ruler;

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
            if ui
                .add_sized(
                    [440.0, 28.0],
                    egui::TextEdit::singleline(&mut app.modals.new_survey_name)
                        .hint_text("예: 병산동"),
                )
                .changed()
            {
                app.modals.new_survey_error = None;
            }
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
            paint_longitude_ruler(ui);
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
            // 새 조사 파일을 만들지 못한 까닭(같은 이름 파일 등) — 창은 닫지 않는다.
            if let Some(err_msg) = &app.modals.new_survey_error {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(err_msg)
                        .size(11.5)
                        .color(Color32::from_rgb(185, 28, 28)),
                );
            }
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
                                    app.modals.new_survey_error = None;
                                    app.modals.active = None;
                                }
                                Err(e) => {
                                    app.modals.new_survey_error = Some(new_survey_error_text(&e));
                                }
                            }
                        }
                    }
                });
            });
        });
}

/// 새 조사 파일을 만들지 못했을 때 창에 보일 문구. 같은 이름 거절(`InvalidInput`)은
/// C++ 문구를 앞말 없이 그대로 보인다.
pub(crate) fn new_survey_error_text(e: &StrataError) -> String {
    match e {
        StrataError::InvalidInput(msg) => msg.clone(),
        other => format!("새 조사를 만들지 못했습니다: {other}"),
    }
}
