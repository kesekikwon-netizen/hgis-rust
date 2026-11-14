//! Right-inspector `스타일` tab (`target/cpp-ref/src/app/KaInspectorStyle.cpp:27-225`,
//! `target/cpp-ref/src/app/MainWindowChrome.cpp:100-118`).

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::gui_tab_home::saved_gpkg_layer_title;
use crate::right_inspector::{read_simple_vector_style, KA_STYLE_PALETTE};
use crate::survey_paint_layers::base_style_for_table;
use crate::survey_style::{mm_to_points, DomainStyle, GeomKind, MARKER_OUTLINE_MM};

const INK: Color32 = Color32::from_rgb(0x1F, 0x23, 0x28);
const MUTED: Color32 = Color32::from_rgb(0x65, 0x6D, 0x76);
const BORDER: Color32 = Color32::from_rgb(0xD0, 0xCD, 0xC2);

const EMPTY_SENTENCE: &str =
    "지도에서 도형을 고르거나 왼쪽 목록에서 레이어를 고르면 그 레이어의 색과 굵기를 여기서 바로 바꿉니다.";
const LINE_MAP_SENTENCE: &str =
    "선으로 된 지도의 선 색은 왼쪽 목록에서 그 레이어를 오른쪽 클릭해 「면·외곽선 색」으로 바꿉니다.";

#[derive(Clone, Default)]
struct StyleSourceMemory {
    last_list: Option<String>,
    last_card: Option<(String, i64)>,
    prefer_card: bool,
}

enum StyleTarget {
    Empty,
    LineMap,
    Editable { table: String, name: String, kind: GeomKind },
}

fn infer_geom_kind(key: &str) -> GeomKind {
    match key {
        "feature_line" | "section_line" => GeomKind::Line,
        k if k.starts_with("user:buffer_ring_") => GeomKind::Line,
        "control_points" | "artifact_point" => GeomKind::Point,
        k if k.starts_with("user:buffer_label_") => GeomKind::Point,
        _ => GeomKind::Polygon,
    }
}

/// Plate `20x14` layer symbol swatch (`target/cpp-ref/src/app/KaInspectorStyle.cpp:47-53, 141-144`).
fn paint_layer_swatch(ui: &mut egui::Ui, kind: GeomKind, s: DomainStyle) {
    let (frame, _) = ui.allocate_exact_size(Vec2::new(20.0, 14.0), egui::Sense::hover());
    let rgba = |c: [u8; 4]| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    let clip = ui.painter().with_clip_rect(frame);
    match kind {
        GeomKind::Polygon => {
            if s.fill[3] > 0 {
                clip.rect_filled(frame, 0.0, rgba(s.fill));
            }
            if s.stroke[3] > 0 {
                let st = Stroke::new(mm_to_points(s.stroke_width_mm), rgba(s.stroke));
                clip.rect_stroke(frame, 0.0, st, egui::StrokeKind::Middle);
            }
        }
        GeomKind::Line => {
            let ends = [Pos2::new(frame.left(), frame.center().y), Pos2::new(frame.right(), frame.center().y)];
            if let Some((c, w)) = s.casing {
                clip.line_segment(ends, Stroke::new(mm_to_points(w), rgba(c)));
            }
            if s.stroke[3] > 0 {
                clip.line_segment(ends, Stroke::new(mm_to_points(s.stroke_width_mm), rgba(s.stroke)));
            }
        }
        GeomKind::Point => {
            let r = mm_to_points(s.marker_size_mm) * 0.5;
            if s.fill[3] > 0 {
                clip.circle_filled(frame.center(), r, rgba(s.fill));
            }
            if s.stroke[3] > 0 {
                clip.circle_stroke(frame.center(), r, Stroke::new(mm_to_points(MARKER_OUTLINE_MM), rgba(s.stroke)));
            }
        }
    }
    ui.painter().rect_stroke(frame, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Inside);
}

fn sentence(ui: &mut egui::Ui, text: &str) {
    ui.add_space(16.0);
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(MUTED)).wrap());
    ui.add_space(16.0);
}

impl StrataAppState {
    /// Resolves the most recently chosen layer between feature card and legend list
    /// (`target/cpp-ref/src/app/MainWindowChrome.cpp:100-118`, `KaInspectorStyle.cpp:117-127`).
    fn resolve_style_target(&mut self, ctx: &egui::Context) -> StyleTarget {
        self.drop_card_if_gone();
        let cur_list = self.right_inspector.selected_layer_key.clone();
        let cur_card = self.home_map.card.as_ref().map(|c| (c.table.clone(), c.fid));
        let id = egui::Id::new("ka_ri_style_source");
        let prefer_card = ctx.data_mut(|d| {
            let mut m = d.get_temp::<StyleSourceMemory>(id).unwrap_or_default();
            let list_changed = cur_list != m.last_list;
            let card_changed = cur_card != m.last_card;
            if list_changed {
                m.prefer_card = false;
            } else if card_changed && cur_card.is_some() {
                m.prefer_card = true;
            } else if cur_card.is_none() {
                m.prefer_card = false;
            }
            m.last_list = cur_list.clone();
            m.last_card = cur_card.clone();
            let out = m.prefer_card;
            d.insert_temp(id, m);
            out
        });
        if prefer_card {
            let key = cur_card.map(|(t, _)| t);
            self.lookup_style_layer(key.as_deref(), true)
        } else {
            self.lookup_style_layer(cur_list.as_deref(), false)
        }
    }

    fn lookup_style_layer(&self, raw: Option<&str>, by_key: bool) -> StyleTarget {
        let Some(sel) = raw.filter(|s| !s.is_empty()) else {
            return StyleTarget::Empty;
        };
        let entry = if by_key {
            self.left_layers.entries.iter().find(|e| e.layer_key == sel)
        } else {
            self.left_layers
                .entries
                .iter()
                .find(|e| e.label_ko == sel)
                .or_else(|| self.left_layers.entries.iter().find(|e| e.layer_key == sel))
        };
        if let Some(e) = entry {
            // Raster basemap / VWorld WMS picture: QgsRasterLayer -> nullptr (`MainWindowChrome.cpp:103`).
            if e.group == LegendGroup::Basemap || e.layer_key == "vworld_cadastral_wms" {
                return StyleTarget::Empty;
            }
            // Reference or downloaded cadastral vector layer (`KaInspectorStyle.cpp:119`).
            if e.group != LegendGroup::SurveyData {
                return StyleTarget::LineMap;
            }
            let kind = self
                .survey_tables
                .iter()
                .find(|t| t.table == e.layer_key)
                .and_then(|t| GeomKind::from_type_name(&t.geometry_type))
                .unwrap_or_else(|| infer_geom_kind(&e.layer_key));
            let name = if e.label_ko.is_empty() { saved_gpkg_layer_title(&e.layer_key) } else { e.label_ko.clone() };
            return StyleTarget::Editable { table: e.layer_key.clone(), name, kind };
        }
        let Some(t) = self
            .survey_tables
            .iter()
            .find(|t| t.table == sel || (!by_key && saved_gpkg_layer_title(&t.table) == sel))
        else {
            return StyleTarget::Empty;
        };
        let Some(kind) = GeomKind::from_type_name(&t.geometry_type) else {
            return StyleTarget::Empty;
        };
        StyleTarget::Editable { table: t.table.clone(), name: saved_gpkg_layer_title(&t.table), kind }
    }

    /// Renders `KaInspectorStyle` (`target/cpp-ref/src/app/KaInspectorStyle.cpp:27-225`).
    pub(crate) fn render_ri_layer_card(&mut self, ui: &mut egui::Ui) {
        let target = self.resolve_style_target(ui.ctx());
        let (table, name, kind) = match target {
            StyleTarget::Empty => return sentence(ui, EMPTY_SENTENCE),
            StyleTarget::LineMap => return sentence(ui, LINE_MAP_SENTENCE),
            StyleTarget::Editable { table, name, kind } => (table, name, kind),
        };
        let enabled = !self.modal_window_open();
        ui.add_enabled_ui(enabled, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let domain = self
                .home_map
                .real_survey
                .layer_styles
                .get(&table)
                .copied()
                .unwrap_or_else(|| base_style_for_table(&table));
            // Plate name row: 20x14 layer swatch + non-bold layer name (`KaInspectorStyle.cpp:43-54, 141-147`).
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                paint_layer_swatch(ui, kind, domain);
                ui.add(egui::Label::new(RichText::new(&name).size(13.0).color(INK)).wrap());
            });
            // Colour swatches: label "색" above 2x4 grid of 44x24 buttons (`KaInspectorStyle.cpp:55-70, 213-225`).
            ui.label(RichText::new("색").size(12.0).color(INK));
            let simple = read_simple_vector_style(&table, &self.home_map.real_survey.layer_styles);
            let current = if kind == GeomKind::Point || simple.no_stroke { simple.fill } else { simple.stroke };
            let mut picked: Option<[u8; 3]> = None;
            for row in KA_STYLE_PALETTE.chunks(4) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for &(r, g, b, tip) in row {
                        let chosen = current[3] > 0 && [r, g, b] == [current[0], current[1], current[2]];
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(44.0, 24.0), egui::Sense::click());
                        let stroke = if chosen { Stroke::new(3.0, INK) } else { Stroke::new(1.0, BORDER) };
                        ui.painter().rect(rect, 4.0, Color32::from_rgb(r, g, b), stroke, egui::StrokeKind::Inside);
                        if resp.on_hover_text(tip).clicked() {
                            picked = Some([r, g, b]);
                        }
                    }
                });
            }
            if let Some(rgb) = picked {
                self.style_tab_pick_color(&table, kind, rgb);
            }
            // Checkboxes hidden on Line layers (`KaInspectorStyle.cpp:72-79, 131-133, 156-157`).
            if kind != GeomKind::Line {
                let cur = read_simple_vector_style(&table, &self.home_map.real_survey.layer_styles);
                let mut nf = cur.no_fill;
                if ui.checkbox(&mut nf, "채우기 없음 (선만)").changed() {
                    self.style_tab_set_off(&table, kind, true, nf);
                }
                let cur = read_simple_vector_style(&table, &self.home_map.real_survey.layer_styles);
                let mut ns = cur.no_stroke;
                if ui.checkbox(&mut ns, "선 없음").changed() {
                    self.style_tab_set_off(&table, kind, false, ns);
                }
            }
            // Plate width row: label left, spin right max 96 px (`KaInspectorStyle.cpp:80-98, 134, 148-149, 158`).
            let cur = read_simple_vector_style(&table, &self.home_map.real_survey.layer_styles);
            let width_label = match kind {
                GeomKind::Point => "점 크기",
                GeomKind::Line => "선 굵기",
                GeomKind::Polygon => "외곽선 굵기",
            };
            let raw_mm = if kind == GeomKind::Point { cur.marker_size_mm } else { cur.stroke_width_mm };
            let clamped = f64::from(raw_mm).clamp(0.2, 12.0);
            let mut val = clamped;
            ui.horizontal(|ui| {
                ui.label(RichText::new(width_label).size(12.0).color(INK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let drag = egui::DragValue::new(&mut val)
                        .range(0.2..=12.0)
                        .speed(0.2)
                        .fixed_decimals(1)
                        .suffix(" mm");
                    let resp = ui.add_sized([78.0, 22.0], drag);
                    if resp.changed() && (val - clamped).abs() > 1e-6 {
                        self.style_tab_set_width(&table, kind, val as f32);
                    }
                });
            });
        });
    }
}

#[cfg(test)]
#[path = "ri_layer_card_tests.rs"]
mod tests;
