//! The row of the received cadastral map: how it is drawn, the choice the user has about it (「선 색·지번
//! 표시」, `target/cpp-ref/src/app/MainWindowCadastral.cpp:56-80`), and the note that lets a table come back
//! with its look when the survey is opened again (the C++ keeps layer and look in its project file).

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Pos2};
use strata_refmap::cadastral::{JIBUN_COLUMN, TABLE};
use strata_types::crs::CrsCode;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_strata_modal_header, ActiveModalDialog};
use crate::ref_vector::data::{LabelLook, LoadSpec, Look, PolygonFill};

#[cfg(test)]
#[path = "cadastral_layer_tests.rs"]
mod tests;

/// `layer_key` and title of the legend row (`CadastralImport.cpp:273`).
pub(crate) const CADASTRAL_KEY: &str = "cadastral_near";
pub(crate) const CADASTRAL_TITLE: &str = "지적도 · 조사 주변 5km";
/// `MainWindowCadastral.cpp:60`: the C++ says it in a notice.
const NEEDS_LAYER: &str = "지적도를 먼저 받아 주세요.";
/// Which table was made for which survey areas and how its row looks, beside the tables
/// (`지적도/strata-kept.json`).
const KEPT_NOTE: &str = "strata-kept.json";

/// What the user chooses for the row: the colour of outlines and lot numbers, and whether the lot numbers
/// are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style {
    pub(crate) colour: [u8; 3],
    pub(crate) labels: bool,
}

impl Default for Style {
    /// `CadastralImport.h:25`: black, with lot numbers.
    fn default() -> Self {
        Self { colour: [0, 0, 0], labels: true }
    }
}

impl Style {
    /// Outlines of 0.2 mm without fill (`CadastralImport.cpp:246-249`).
    fn look(self) -> Look {
        let [red, green, blue] = self.colour;
        Look::Polygons { stroke: [red, green, blue, 255], width_mm: 0.2, fill: PolygonFill::None }
    }

    /// The lot numbers in Malgun Gothic 8 pt of the same colour, from 1:10,000 in (`:250-263`).
    fn label(self) -> Option<LabelLook> {
        self.labels.then_some(LabelLook { column: JIBUN_COLUMN, size_pt: 8.0, ink: self.colour, halo_mm: 0.0, halo: [255, 255, 255, 0], bold: false, closest_scale: Some(10_000.0), inside_only: false })
    }

    /// The choice a row is drawn by.
    pub(crate) fn of(spec: &LoadSpec) -> Self {
        let colour = if let Look::Polygons { stroke: [red, green, blue, _], .. } = spec.look { [red, green, blue] } else { Self::default().colour };
        Self { colour, labels: spec.label.is_some() }
    }
}

/// Notes which table was made for these survey areas and how its row looks. Without the note the next run
/// makes the table again; nothing else is lost.
pub(crate) fn write_kept(directory: &Path, areas: &[String], work_epsg: u32, radius_m: f64, table: &Path, style: Style) {
    let note = serde_json::json!({"radius_m": radius_m, "work_epsg": work_epsg, "areas": areas, "file": table.file_name().map(|name| name.to_string_lossy().into_owned()), "colour": style.colour, "labels": style.labels});
    let _ = std::fs::write(directory.join(KEPT_NOTE), note.to_string());
}

fn noted(directory: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(directory.join(KEPT_NOTE)).ok()?).ok()
}

/// The table an earlier run made for these very survey areas, when it is still in the folder, with the look
/// its row had.
pub(crate) fn kept_table(directory: &Path, areas: &[String], work_epsg: u32, radius_m: f64) -> Option<(PathBuf, Style)> {
    let note = noted(directory)?;
    let same = note["radius_m"].as_f64() == Some(radius_m) && note["work_epsg"].as_u64() == Some(u64::from(work_epsg)) && note["areas"].as_array().is_some_and(|kept| kept.iter().map(serde_json::Value::as_str).eq(areas.iter().map(|area| Some(area.as_str()))));
    let file = directory.join("표시").join(note["file"].as_str()?);
    let colour = serde_json::from_value::<[u8; 3]>(note["colour"].clone()).unwrap_or(Style::default().colour);
    (same && file.is_file()).then_some((file, Style { colour, labels: note["labels"].as_bool().unwrap_or(true) }))
}

impl StrataAppState {
    /// The row of the received parcels: a row of its own at the top of the legend (`LayerOps.cpp:1091-1101`).
    /// `style: None` keeps the look of the row that is there (`CadastralImport.cpp:277-286`), or the usual one.
    pub(crate) fn show_cadastral_layer(&mut self, file: PathBuf, style: Option<Style>) {
        let style = style.or_else(|| self.ref_vectors.spec_of(CADASTRAL_KEY).map(Style::of)).unwrap_or_default();
        let spec = LoadSpec { file, table: TABLE.to_owned(), work_epsg: CrsCode::from(self.work_crs).epsg_number(), look: style.look(), label: style.label() };
        self.show_top_vector_layer(CADASTRAL_KEY, CADASTRAL_TITLE, spec);
    }

    /// 「선 색·지번 표시」 of the 지적 button's menu (`configureCadastralStyle`, `:56-80`): the dialog opens
    /// with the row's present colour and switch; without the parcels it says to receive them first.
    pub(crate) fn open_cadastral_style(&mut self) {
        let Some(style) = self.ref_vectors.spec_of(CADASTRAL_KEY).map(Style::of) else {
            return self.show_status_message(NEEDS_LAYER);
        };
        self.active_tab = MainTab::Map;
        self.cadastral.style = style;
        self.modals.active = Some(ActiveModalDialog::CadastralStyle);
    }

    /// 확인 of the dialog: the row takes the chosen colour and switch (`:77-79`), and the note keeps them.
    pub(crate) fn apply_cadastral_style(&mut self) {
        let style = self.cadastral.style;
        self.ref_vectors.set_look(CADASTRAL_KEY, style.look());
        self.ref_vectors.set_label(CADASTRAL_KEY, style.label());
        let directory = crate::survey_folder::survey_folder(self.survey_path.as_deref()).map(|folder| folder.join("지적도"));
        if let Some((directory, mut note)) = directory.and_then(|directory| noted(&directory).map(|note| (directory, note))) {
            (note["colour"], note["labels"]) = (serde_json::json!(style.colour), serde_json::json!(style.labels));
            let _ = std::fs::write(directory.join(KEPT_NOTE), note.to_string());
        }
    }
}

/// The dialog 「지적도 선 색·지번」 (`:61-75`): the colour of the outline of 0.2 mm, 「지번 표시」, what the
/// dialog says about the scale, 확인 and 취소.
pub(crate) fn render_cadastral_style(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let full = ctx.screen_rect();
    let (modal_w, modal_h) = (360.0_f32.min(full.width() - 40.0), 150.0_f32.min(full.height() - 40.0));
    let modal_pos = Pos2::new(((full.width() - modal_w) * 0.5).max(16.0), ((full.height() - modal_h) * 0.5).max(16.0));
    egui::Window::new("ka_modal_cadastral_style").title_bar(false).resizable(false).fixed_pos(modal_pos).fixed_size([modal_w, modal_h]).show(ctx, |ui| {
        ui.set_min_width(modal_w - 20.0);
        ui.set_max_width(modal_w - 20.0);
        render_strata_modal_header(ui, "지적도 선 색·지번", open);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("경계선 0.2mm").size(12.5).strong());
            // The C++ button shows the colour's name and opens 「경계선 색」; here the swatch opens the chooser.
            ui.color_edit_button_srgb(&mut app.cadastral.style.colour).on_hover_text("경계선 색");
            let [red, green, blue] = app.cadastral.style.colour;
            ui.label(egui::RichText::new(format!("#{red:02x}{green:02x}{blue:02x}")).size(12.0));
        });
        ui.checkbox(&mut app.cadastral.style.labels, "지번 표시");
        ui.label(egui::RichText::new("지번은 1:10,000보다 확대했을 때 표시됩니다.").size(11.5).color(Color32::from_rgb(0x47, 0x55, 0x69)));
        ui.add_space(8.0);
        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("취소").clicked() {
                app.modals.active = None;
            }
            if ui.button("확인").clicked() {
                app.apply_cadastral_style();
                app.modals.active = None;
            }
        });
    });
}
