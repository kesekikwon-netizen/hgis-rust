//! 「주변유적 받기」 결과 표 — 유적명·종류·거리·지정번호·위치 이동.
//!
//! `dlg_heritage_browser.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 창 안의 결과 표 블록을 **그대로** 옮겼고, 순서·수치·문구는 하나도 바꾸지 않았다.
//! `max_dist_m`·`catalog`·`list_h` 는 이 블록 안에서만 쓰이므로 함께 옮겼다.
//! 하단 줄이「검색 결과 N건」을 쓰므로 걸러진 행 수를 돌려준다.

use eframe::egui::{self, Color32, Stroke};

use crate::app_state::StrataAppState;

/// 결과 표를 그리고 걸러진 행 수를 돌려준다. `modal_w` 는 창 폭이다.
pub(crate) fn render_heritage_results(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    modal_w: f32,
) -> usize {
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
    filtered_rows.len()
}
