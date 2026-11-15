//! `render_dem_class` — 「DEM 표현」 (`target/cpp-ref/src/app/KaDemClassDialog.cpp:18-76`): the colour preset
//! with 「현재 화면에 색 맞추기」, 「지형 음영 합성」, 수직과장, 음영 세기, what the dialog says, 적용 and 닫기.
//! What 적용 does is in `dem_look.rs`.
use eframe::egui::{self, Color32, Pos2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use crate::topo_download::dem::look::{DemLook, DemPreset};

/// `KaDemClassDialog.cpp:25-26`.
const HINT: &str = "표고에 따라 색이 부드럽게 이어집니다. 유적처럼 높이차가 작은 곳은 「현재 화면에 색 맞추기」로 화면 안의 높이만으로 색을 나누세요.";
/// `KaDemClassDialog.cpp:38-39`.
const FIT_TIP: &str = "지금 지도 화면에 보이는 가장 낮은 곳과 높은 곳으로 색띠를 다시 나눕니다. 지도를 옮기면 따라 바뀝니다.";
/// The place of 「현재 화면 맞춤」 in the list of presets.
const FITTED: usize = 2;

/// What the fields of the dialog ask for.
fn asked(app: &StrataAppState) -> DemLook {
    let preset = DemPreset::ALL.get(app.modals.dem_preset_idx).map_or(DemPreset::National, |(preset, _)| *preset);
    DemLook { preset, relief: app.modals.dem_relief_enabled, z_factor: app.modals.dem_exaggeration, strength_pct: app.modals.dem_relief_strength_pct }
}

/// 적용 (`KaDemClassDialog::applyStyle`): the DEM takes what the fields ask for, and the dialog says how it went.
fn apply(app: &mut StrataAppState) {
    app.modals.dem_status_text = app.apply_dem_look(asked(app)).to_owned();
}

pub(crate) fn render_dem_class(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 460.0_f32.min(vis_w - 40.0);
    let modal_h = 300.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(((vis_w - modal_w) * 0.5).max(16.0), ((vis_h - modal_h) * 0.5).max(16.0));
    egui::Window::new("ka_modal_dem_class").title_bar(false).resizable(false).fixed_pos(modal_pos).fixed_size([modal_w, modal_h]).show(ctx, |ui| {
        ui.set_min_width(modal_w - 20.0);
        ui.set_max_width(modal_w - 20.0);
        render_strata_modal_header(ui, "DEM 표현", open);
        ui.label(egui::RichText::new(HINT).size(12.0).color(Color32::from_rgb(0x47, 0x55, 0x69)));
        ui.add_space(8.0);

        let mut fit = false;
        egui::Grid::new("ka_dem_class_form").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
            // 색 표현: the preset list and 「현재 화면에 색 맞추기」 (`:28-44`).
            ui.label(egui::RichText::new("색 표현").size(12.5).strong());
            ui.horizontal(|ui| {
                let chosen = DemPreset::ALL.get(app.modals.dem_preset_idx).map_or(DemPreset::ALL[0].1, |(_, name)| *name);
                egui::ComboBox::from_id_salt("ka_dem_preset_combo").selected_text(chosen).width(175.0).show_ui(ui, |ui| {
                    for (index, (_, name)) in DemPreset::ALL.iter().enumerate() {
                        ui.selectable_value(&mut app.modals.dem_preset_idx, index, *name);
                    }
                });
                fit = ui.button("현재 화면에 색 맞추기").on_hover_text(FIT_TIP).clicked();
            });
            ui.end_row();

            // 「지형 음영 합성」 (`:45-48`).
            ui.label("");
            ui.checkbox(&mut app.modals.dem_relief_enabled, "지형 음영 합성");
            ui.end_row();

            // 수직과장, 0.1 to 5 times in steps of 0.1 (`:49-55`).
            ui.label(egui::RichText::new("수직과장").size(12.5).strong());
            ui.add(egui::DragValue::new(&mut app.modals.dem_exaggeration).range(0.1..=5.0).speed(0.1).fixed_decimals(1).suffix(" 배"));
            ui.end_row();

            // 음영 세기, 0 to 80 per cent (`:56-61`).
            ui.label(egui::RichText::new("음영 세기").size(12.5).strong());
            ui.add(egui::DragValue::new(&mut app.modals.dem_relief_strength_pct).range(0..=80).speed(1.0).suffix(" %"));
            ui.end_row();
        });
        // `KaDemClassDialog::fitToView`: the preset becomes 「현재 화면 맞춤」 and is applied at once.
        if fit {
            app.modals.dem_preset_idx = FITTED;
            apply(app);
        }

        ui.add_space(6.0);
        // What the dialog says (`:62-65`).
        ui.colored_label(Color32::from_rgb(0x7A, 0x5A, 0x00), egui::RichText::new(&app.modals.dem_status_text).size(11.5));
        ui.add_space(8.0);
        ui.separator();
        // 적용 and 닫기 (`:66-72`).
        ui.horizontal(|ui| {
            if ui.button("적용").clicked() {
                apply(app);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("닫기").clicked() {
                    app.modals.active = None;
                }
            });
        });
    });
}
