//! The card of the startup notice — C++ `KaSplashStrata::paint` (KaSplashStrata.cpp:229-272), the shadow of
//! `KaSplashArt::paintShadow` (KaSplashArt.cpp:113-124) and the words of `KaSplashCredits`. Everything is
//! laid out on a card of 700 x 400 and scaled with it.

use eframe::egui::{self, epaint::CornerRadiusF32, Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};

use super::CardFrame;

pub(crate) const PAPER: Color32 = Color32::from_rgb(0xFA, 0xF9, 0xF5);
pub(crate) const INK: Color32 = Color32::from_rgb(0x14, 0x14, 0x13);
const INK_2: Color32 = Color32::from_rgb(0x3D, 0x3D, 0x3A);
pub(crate) const MUTED: Color32 = Color32::from_rgb(0x5E, 0x5D, 0x59);
pub(crate) const HAIR: Color32 = Color32::from_rgb(0x87, 0x86, 0x7F);
pub(crate) const RING: Color32 = Color32::from_rgb(0xB0, 0xAE, 0xA5);
pub(crate) const CLAY: Color32 = Color32::from_rgb(0xD9, 0x77, 0x57);
pub(crate) const BEDROCK: Color32 = Color32::from_rgb(0x26, 0x26, 0x24);
/// The paper card has corners of 12 px (the blue card of the old look: 16).
const RADIUS: f32 = 12.0;

/// Font families of the notice. They are registered only while the notice shows (`notice_fonts`).
const SERIF: &str = "notice_serif";
const MONO: &str = "notice_mono";
const LIGHT: &str = "notice_light";

/// `KaSplashCredits`: the notice names 권영인 only (the About box names three: the owner's decision).
const COPYRIGHT: &str = "© 2026 동국문화재연구원 · 권영인 · GNU GPL v2 이상으로 배포";
const DATA: &str = "지도·자료  VWorld · 국토정보플랫폼 · 국가유산 공간정보 · 흙토람 · KIGAM · 국사편찬위원회 — 제공처의 이용조건을 따릅니다";

/// The fonts of the app with the three faces of the notice: a serif for the name and Ⅰ–Ⅴ (C++: Noto Serif
/// KR, else Batang), Consolas for the staff, Malgun Gothic Semilight for the second notice line. A face
/// that is not on the PC is left out and the app's own font stands in.
pub(crate) fn notice_fonts() -> egui::FontDefinitions {
    let mut fonts = crate::gui_theme_fonts::font_definitions();
    for (family, file, fallback) in [(SERIF, "batang.ttc", FontFamily::Proportional), (MONO, "consola.ttf", FontFamily::Monospace), (LIGHT, "malgunsl.ttf", FontFamily::Proportional)] {
        let Ok(bytes) = std::fs::read(std::path::Path::new(r"C:\Windows\Fonts").join(file)) else { continue };
        fonts.font_data.insert(family.to_owned(), egui::FontData::from_owned(bytes).into());
        let mut faces = vec![family.to_owned()];
        faces.extend(fonts.families.get(&fallback).cloned().unwrap_or_default());
        fonts.families.insert(FontFamily::Name(family.into()), faces);
    }
    fonts
}

/// The app icon of the paper look, kept by the window while the notice shows.
pub(crate) fn icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    const RGBA: &[u8] = include_bytes!("../../../data/theme/ka-hgis-app-paper-96.rgba");
    let id = egui::Id::new("startup_notice_icon");
    if let Some(kept) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return Some(kept);
    }
    let texture = ctx.load_texture("startup_notice_icon", egui::ColorImage::from_rgba_unmultiplied([96, 96], RGBA), egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}

/// The card as a drawing surface: `at` takes a place of the 700 x 400 card to the screen.
pub(crate) struct Card<'a> {
    pub(crate) painter: &'a egui::Painter,
    pub(crate) rect: Rect,
    pub(crate) unit: f32,
}

impl Card<'_> {
    pub(crate) fn at(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + Vec2::new(x, y) * self.unit
    }

    pub(crate) fn font(&self, family: &str, pixels: f32) -> FontId {
        let named = FontFamily::Name(family.into());
        let known = self.painter.ctx().fonts(|fonts| fonts.families().contains(&named));
        FontId::new(pixels * self.unit, if known { named } else if family == MONO { FontFamily::Monospace } else { FontFamily::Proportional })
    }

    pub(crate) fn serif(&self, pixels: f32) -> FontId {
        self.font(SERIF, pixels)
    }

    pub(crate) fn mono(&self, pixels: f32) -> FontId {
        self.font(MONO, pixels)
    }

    pub(crate) fn ui(&self, pixels: f32) -> FontId {
        FontId::new(pixels * self.unit, FontFamily::Proportional)
    }

    /// Text with its baseline starting at a place of the card (Qt `drawText(QPointF)`).
    pub(crate) fn text(&self, x: f32, baseline: f32, words: &str, font: FontId, ink: Color32) {
        let galley = self.painter.layout_no_wrap(words.to_owned(), font, ink);
        let ascent = galley.rows.first().and_then(|row| row.glyphs.first()).map_or(0.0, |glyph| glyph.pos.y);
        self.painter.galley(self.at(x, baseline) - Vec2::new(0.0, ascent), galley, ink);
    }

    /// A notice line is never cut off: the letters shrink to the width instead (`drawFitted`).
    fn fitted(&self, x: f32, baseline: f32, words: &str, mut font: FontId, ink: Color32, width: f32) {
        while font.size > 8.0 * self.unit && self.painter.layout_no_wrap(words.to_owned(), font.clone(), ink).size().x > width * self.unit {
            font.size -= self.unit;
        }
        self.text(x, baseline, words, font, ink);
    }
}

/// Paints the shadow, the card and one frame of its picture into `card`.
pub(crate) fn paint_card(painter: &egui::Painter, card: Rect, icon: Option<&egui::TextureHandle>, frame: &CardFrame) {
    // Stacked, slightly lowered rounded rects read as one soft shadow.
    for layer in (1..=12u8).rev().map(f32::from) {
        let rect = Rect::from_min_max(card.min + Vec2::new(-layer, 5.0 - 0.5 * layer), card.max + Vec2::new(layer, layer + 5.0));
        painter.rect_filled(rect, RADIUS + layer, Color32::from_rgba_unmultiplied(4, 18, 34, 6));
    }
    painter.rect_filled(card, RADIUS, PAPER);
    let unit = card.height() / 400.0;
    // A shrunk notice can be wider than 700:400: the bedrock still reaches the right edge.
    let width = card.width() / unit;
    let on = Card { painter: &painter.with_clip_rect(card), rect: card, unit };

    super::section::paint_section(&on, frame.seconds, frame.progress);

    if let Some(icon) = icon {
        on.painter.image(icon.id(), Rect::from_min_size(on.at(32.0, 32.0), Vec2::splat(44.0 * unit)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    on.text(29.0, 140.0, "Strata", on.serif(60.0), INK);
    on.text(32.0, 167.0, &format!("필드고고학 GIS · v{}", frame.version), on.ui(14.0), MUTED);
    on.text(32.0, 192.0, frame.status, on.ui(13.0), INK_2);

    // The bedrock band carries the two notice lines.
    let band = Rect::from_min_max(on.at(0.0, 344.0), on.at(width, 400.0));
    on.painter.rect_filled(band, CornerRadiusF32 { nw: 0.0, ne: 0.0, sw: RADIUS, se: RADIUS }, BEDROCK);
    on.fitted(32.0, 367.0, COPYRIGHT, on.ui(12.0), PAPER.gamma_multiply(0.96), width - 64.0);
    on.fitted(32.0, 385.0, DATA, on.font(LIGHT, 11.0), PAPER.gamma_multiply(0.86), width - 64.0);
    // A hairline edge so the paper card reads against a light desktop.
    painter.rect_stroke(card.shrink(0.5), RADIUS, Stroke::new(1.0, Color32::from_rgba_unmultiplied(31, 30, 29, 40)), egui::StrokeKind::Middle);
}
