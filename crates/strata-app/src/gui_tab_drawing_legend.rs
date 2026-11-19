//! (1) 용지 범례 — `legend_visible` 토글 연동, `KaDrawingStudio::defaultItemRect(kIdLegend)`.
//!
//! The heritage sites that carry a number on the map of the sheet have a row each, 「(number badge) name」,
//! in the order of their numbers; a kind without a numbered site has no row
//! (`HeritageLayoutNumbers.cpp:1237-1316`). The rows flow into columns (`gui_tab_drawing_legend_flow.rs`).

use std::sync::Arc;

use eframe::egui::{self, text::LayoutJob, text::TextFormat, Color32, FontFamily, FontId, Galley, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::gui_layer_panel::gui_layer_panel_paint::SwatchKind;
use crate::gui_layer_panel::gui_layer_panel_row_spec::layer_rows;
use crate::gui_tab_map::LABEL_FONT_FAMILY;
use crate::sheet_numbers::{badge_look, legend_kinds, legend_site_rows, names_shown, Badge};

use super::gui_tab_drawing_frame::SheetFrame;
use super::gui_tab_drawing_legend_flow::{legend_cell, legend_column_rows, legend_columns};

/// The black edge of a badge in the legend, in millimetres of paper (`:1295-1301`).
const BADGE_EDGE_MM: f32 = 0.15;

/// A row of the legend.
enum LegendRow {
    /// A layer that is checked in the legend panel: the symbol the panel shows for it, and its name.
    Layer(SwatchKind, String),
    /// A numbered heritage site.
    Site(Badge),
    /// A kind of heritage data, when the names of its sites stand on the map: its colour and its name.
    Kind([u8; 3], &'static str),
}

/// The square of a kind in the legend, in millimetres of paper (`HeritageLayoutNumbers.cpp:1283-1291`).
const KIND_SQUARE_MM: f32 = 3.0;

/// The frame of the legend on the screen, its columns, and the scale of its rows.
pub(crate) struct LegendCard {
    pub(crate) rect: Rect,
    /// How far the user moved the legend from where the sheet places it.
    moved: Vec2,
    columns: usize,
    s: f32,
    leg_w: f32,
}

#[cfg(test)]
#[path = "gui_tab_drawing_legend_tests.rs"]
mod tests;

impl StrataAppState {
    /// Layer, kind and site names the sheet legend paints, for the vector PDF.
    pub(crate) fn sheet_legend_pdf_names(&self) -> Vec<String> {
        self.sheet_legend_survey_rows().into_iter().map(|row| match row {
            LegendRow::Layer(_, name) => name,
            LegendRow::Kind(_, name) => name.to_owned(),
            LegendRow::Site(badge) => format!("{} {}", badge.number, badge.name),
        }).collect()
    }

    /// The names of the layer rows of the legend.
    #[cfg(test)]
    pub(crate) fn sheet_legend_layer_names(&self) -> Vec<String> {
        self.sheet_legend_survey_rows().into_iter().filter_map(|row| if let LegendRow::Layer(_, name) = row { Some(name) } else { None }).collect()
    }

    /// The title of the legend as the C++ sets it (`KaDrawingStudio.cpp:2368-2369, 2372`): bold while 「B」 is
    /// on, slanted while 「I」 is on. The rows keep their plain letters (`:2375-2376`).
    fn legend_title_galley(&self, painter: &egui::Painter, size: f32, ink: Color32) -> Arc<Galley> {
        let bold = FontFamily::Name(LABEL_FONT_FAMILY.into());
        let family = if self.drawing.sheet.legend_bold && painter.ctx().fonts(|fonts| fonts.families().contains(&bold)) { bold } else { FontFamily::Proportional };
        let format = TextFormat { font_id: FontId::new(size, family), color: ink, italics: self.drawing.sheet.legend_italic, ..Default::default() };
        painter.layout_job(LayoutJob::single_section(self.drawing.sheet.legend_title.clone(), format))
    }

    /// The rows of the layers that are checked in the legend panel, top to bottom, each by the name and the
    /// symbol the panel shows (owner 2026-11-15: 「범례는 레이어가 선택일때 자동으로 배치되게하라」). A picture
    /// (base map, tile map) has no row, and the rows of the 「유산」 button have none here: their sites are
    /// listed one by one.
    fn sheet_legend_survey_rows(&self) -> Vec<LegendRow> {
        let rows = layer_rows(&self.left_layers, Color32::BLACK, Color32::BLACK).into_iter();
        // The distance letters of a ring are letters, not a layer with a symbol.
        let own_rows = |key: &str| !key.starts_with(crate::heritage_run::KEY_PREFIX) && !key.starts_with("user:buffer_label_");
        let listed = rows.filter(|row| self.home_map.real_survey.is_layer_visible(&row.name) && own_rows(&row.key));
        listed.filter(|row| !matches!(row.swatch, SwatchKind::RasterChecker | SwatchKind::HeritageSub)).map(|row| LegendRow::Layer(row.swatch, row.name)).collect()
    }

    /// The legend's frame for its survey rows and `site_rows` rows of numbered sites; `None` when the
    /// legend is hidden. The badges of the map keep off it (`LayoutBadgePlacer::legendBlocks`).
    pub(crate) fn sheet_legend_card(&self, f: &SheetFrame, site_rows: usize) -> Option<LegendCard> {
        if !self.drawing.sheet.legend_visible {
            return None;
        }
        let (map_rect, px_per_mm) = (f.map_rect, f.px_per_mm);
        let rows = self.sheet_legend_survey_rows().len() + site_rows;
        let s = (px_per_mm / 1.65).clamp(0.42, 1.3);
        let base_leg_r = f.mm_to_rect(self.drawing.sheet.legend_rect);
        let row_h = (self.drawing.sheet.legend_font_pt as f32 * 1.35).max(12.0) * s;
        let leg_w = base_leg_r.width().max(42.0 * s).min(map_rect.width() * 0.42);
        let columns = legend_columns(leg_w / px_per_mm, self.drawing.sheet.legend_font_pt as f32, rows);
        // Each column is wide enough for the longest layer name (its symbol, the gap and the margin: 16
        // units, and 10 for the frame), as far as 42 % of the map allows.
        let label = crate::gui_theme_fonts::layout_font(f.sheet_p.ctx(), ((self.drawing.sheet.legend_font_pt as f32 * s).clamp(4.5, 11.5) - 1.2 * s).max(4.0));
        let widths = self.sheet_legend_survey_rows().into_iter().filter_map(|row| if let LegendRow::Layer(_, name) = row { Some(name) } else { None });
        let longest = widths.map(|name| f.sheet_p.layout_no_wrap(name, label.clone(), Color32::BLACK).size().x).fold(0.0, f32::max);
        let leg_w = leg_w.max(columns as f32 * (longest + 16.0 * s) + 10.0 * s).min(map_rect.width() * 0.42);
        // The frame keeps its width and takes the height its rows need (`LayoutService.cpp:1531-1534`).
        let leg_h = (20.0 * s + (legend_column_rows(rows, columns) as f32) * row_h).max(24.0 * s).min(map_rect.height());
        let rect = Rect::from_min_size(Pos2::new((map_rect.right() - leg_w - 5.0 * s).max(map_rect.left() + 4.0), base_leg_r.top()), Vec2::new(leg_w, leg_h));
        let (rect, columns, s) = if let Some([width, height]) = self.drawing.legend_size_mm {
            let size = Vec2::new(width, height) * px_per_mm;
            let scale = (size.x / rect.width()).min(size.y / rect.height());
            (Rect::from_min_size(rect.min, size), legend_columns(width, self.drawing.sheet.legend_font_pt as f32, rows), s * scale)
        } else {
            (rect, columns, s)
        };
        let moved = self.sheet_item_offset(crate::tab_drawing::SheetItem::Legend, rect, f);
        Some(LegendCard { rect: rect.translate(moved), moved, columns, s, leg_w: rect.width() })
    }

    /// 범례 상자와 항목들.
    pub(crate) fn paint_sheet_legend(&mut self, f: &SheetFrame) {
        let map_rect = f.map_rect;
        let ink = f.ink;
        let sheet_p = f.sheet_p.clone();
        // The sites the map of this sheet numbers (the same view as the badges: nothing is counted twice).
        let (north_west, south_east) = (f.m2w(map_rect.left_top()), f.m2w(map_rect.right_bottom()));
        let badges = self.sheet_heritage_badges([north_west[0], south_east[1], south_east[0], north_west[1]]);
        // With the names on the map the legend lists the kinds only (`:1207, 1281-1293`).
        let names = names_shown(f.scale_denom, self.drawing.sheet.show_site_names_under_5000);
        let Some(LegendCard { rect: leg_r, moved, columns, s, leg_w, .. }) = self.sheet_legend_card(f, legend_site_rows(&badges, names)) else {
            return;
        };
        self.drawing.item_rects.push((crate::tab_drawing::SheetItem::Legend, leg_r.translate(-moved), moved));
        // 3. 용지 내 4요소 실물 조판: (1) 범례, (2) 표제란, (3) 축척막대·축척글·좌표계, (4) 방위표(도북 화살)
        let mut rows = self.sheet_legend_survey_rows();
        if names {
            rows.extend(legend_kinds(&badges).into_iter().map(|(name, colour)| LegendRow::Kind(colour, name)));
        } else {
            rows.extend(badges.into_iter().map(LegendRow::Site));
        }
        let font_pt = self.drawing.sheet.legend_font_pt as f32;
        // Measure every real name, including numbered sites, before fitting the box. Linear font scaling
        // is bounded in both dimensions (target/cpp-ref/src/core/LayoutService.cpp:1458-1534).
        let row_width = rows
            .iter()
            .map(|row| {
                let (symbol, name) = match row {
                    LegendRow::Layer(_, name) => (10.0, name.as_str()),
                    LegendRow::Kind(_, name) => (KIND_SQUARE_MM * 1.65, *name),
                    LegendRow::Site(badge) => (badge_look(badge.number, badge.colour, 1.65).diameter, badge.name.as_str()),
                };
                symbol + 14.0 + sheet_p.layout_no_wrap(name.to_owned(), crate::gui_theme_fonts::layout_font(sheet_p.ctx(), font_pt - 1.2), ink).size().x
            })
            .fold(1.0, f32::max);
        let title_width = self.legend_title_galley(&sheet_p, font_pt, ink).size().x + 10.0;
        let row_unit = (font_pt * 1.35).max(12.0);
        let heading = (font_pt * 1.7).max(20.0);
        let s =
            s.min(leg_w / columns as f32 / row_width).min(leg_w / title_width).min(leg_r.height() / (heading + legend_column_rows(rows.len(), columns) as f32 * row_unit + 4.0));
        let row_h = row_unit * s;
        // Owner UI10 rounded-frame decision; content follows target/cpp-ref/src/app/KaDrawingStudio.cpp:2364-2390.
        sheet_p.rect_filled(leg_r, 6.0, Color32::from_rgba_unmultiplied(255, 255, 255, 245));
        sheet_p.rect_stroke(leg_r, 6.0, Stroke::new(0.9, ink), egui::StrokeKind::Middle);
        let title_pt = font_pt * s;
        let title = self.legend_title_galley(&sheet_p, title_pt, ink);
        sheet_p.galley(Pos2::new(leg_r.left() + 5.0 * s, leg_r.top() + heading * s * 0.5 - title.size().y * 0.5), title, ink);
        sheet_p.line_segment(
            [Pos2::new(leg_r.left() + 3.5 * s, leg_r.top() + (heading - 2.0) * s), Pos2::new(leg_r.right() - 3.5 * s, leg_r.top() + (heading - 2.0) * s)],
            Stroke::new(0.5, Color32::from_rgb(200, 198, 190)),
        );

        let label_font = crate::gui_theme_fonts::layout_font(sheet_p.ctx(), (font_pt - 1.2) * s);
        let bold = FontFamily::Name(LABEL_FONT_FAMILY.into());
        let number_family = if sheet_p.ctx().fonts(|fonts| fonts.families().contains(&bold)) { bold } else { FontFamily::Proportional };
        let column_w = (leg_w - 10.0 * s) / columns as f32;
        let count = rows.len();
        for (index, row) in rows.into_iter().enumerate() {
            let (column, place) = legend_cell(index, count, columns);
            let left = leg_r.left() + 5.0 * s + column as f32 * column_w;
            let ry = leg_r.top() + heading * s + row_h * 0.5 + place as f32 * row_h;
            // A name longer than its column ends at the column.
            let cell = Rect::from_min_max(Pos2::new(left, ry - row_h * 0.5), Pos2::new(left + column_w - 2.0 * s, ry + row_h * 0.5));
            let p = sheet_p.with_clip_rect(cell.intersect(leg_r));
            let (symbol_w, name) = match row {
                LegendRow::Layer(symbol, name) => {
                    let swatch = Rect::from_min_size(Pos2::new(left, ry - 3.5 * s), Vec2::new(10.0 * s, 7.0 * s));
                    match symbol {
                        SwatchKind::DomainLine { stroke, .. } => {
                            p.line_segment([swatch.left_center(), swatch.right_center()], Stroke::new(1.6, stroke));
                        }
                        SwatchKind::DomainMarker { fill, stroke, .. } => {
                            p.circle(swatch.center(), swatch.height() * 0.42, fill, Stroke::new(1.0, stroke));
                        }
                        // A filled layer, the outline of the cadastral map, the ring of a distance.
                        other => {
                            let (fill, stroke) = match other {
                                SwatchKind::DomainFill { fill, stroke, .. } => (fill, stroke),
                                SwatchKind::BufferBand => (Color32::TRANSPARENT, Color32::from_rgb(184, 58, 24)),
                                _ => (Color32::TRANSPARENT, Color32::from_rgb(68, 66, 60)),
                            };
                            if fill != Color32::TRANSPARENT {
                                p.rect_filled(swatch, 1.0, fill);
                            }
                            p.rect_stroke(swatch, 1.0, Stroke::new(1.0, stroke), egui::StrokeKind::Middle);
                        }
                    }
                    (swatch.width(), name)
                }
                LegendRow::Kind([red, green, blue], name) => {
                    let side = KIND_SQUARE_MM * 1.65 * s;
                    let swatch = Rect::from_center_size(Pos2::new(left + side * 0.5, ry), Vec2::splat(side));
                    p.rect_filled(swatch, 0.0, Color32::from_rgb(red, green, blue));
                    p.rect_stroke(swatch, 0.0, Stroke::new(BADGE_EDGE_MM * 1.65 * s, Color32::BLACK), egui::StrokeKind::Middle);
                    (side, name.to_owned())
                }
                LegendRow::Site(badge) => {
                    let look = badge_look(badge.number, badge.colour, 1.65 * s);
                    let ([red, green, blue], [ink_red, ink_green, ink_blue]) = (badge.colour, look.ink);
                    let centre = Pos2::new(left + look.diameter * 0.5, ry);
                    p.circle(centre, look.diameter * 0.5, Color32::from_rgb(red, green, blue), Stroke::new(BADGE_EDGE_MM * 1.65 * s, Color32::BLACK));
                    p.text(
                        centre,
                        egui::Align2::CENTER_CENTER,
                        badge.number.to_string(),
                        FontId::new(look.letter, number_family.clone()),
                        Color32::from_rgb(ink_red, ink_green, ink_blue),
                    );
                    (look.diameter, badge.name)
                }
            };
            p.text(Pos2::new(left + symbol_w + 4.0 * s, ry), egui::Align2::LEFT_CENTER, name, label_font.clone(), ink);
        }
    }
}
