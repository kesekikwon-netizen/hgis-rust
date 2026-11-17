//! Window-free UI evidence, using the PDF renderer (target/cpp-ref/src/app/KaDrawingStudio.cpp:3798).
use eframe::egui::{epaint::Shape, Context, FullOutput, Pos2, RawInput, Rect, Vec2};

pub(crate) struct Rendered {
    ctx: Context,
    output: FullOutput,
    textures: crate::gui_tab_drawing::sheet_raster::Textures,
    size: [u32; 2],
}

pub(crate) fn render(size: [u32; 2], mut paint: impl FnMut(&Context)) -> Rendered {
    let ctx = Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &strata_types::theme_tokens::ThemeTokens::default());
    let mut textures = crate::gui_tab_drawing::sheet_raster::Textures::default();
    let mut output = FullOutput::default();
    for _ in 0..3 {
        output = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(size[0] as f32, size[1] as f32))), ..Default::default() }, |ctx| paint(ctx));
        textures.apply(&output.textures_delta);
    }
    Rendered { ctx, output, textures, size }
}

fn collect_text(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(text) => out.push(text.galley.text().to_owned()),
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect_text(shape, out)),
        _ => {}
    }
}

impl Rendered {
    pub(crate) fn text_clips(&self) -> Vec<(String, Rect, Rect)> {
        self.output.shapes.iter().filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) => Some((text.galley.text().to_owned(), text.galley.rect.translate(text.pos.to_vec2()), clipped.clip_rect)),
            _ => None,
        }).collect()
    }
    pub(crate) fn text_rects(&self) -> Vec<(String, Rect)> {
        self.output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some((text.galley.text().to_owned(), text.galley.rect.translate(text.pos.to_vec2()))),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn texts(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.output.shapes.iter().for_each(|shape| collect_text(&shape.shape, &mut out));
        out
    }

    pub(crate) fn save_if_requested(&self, name: &str) {
        let Some(directory) = std::env::var_os("STRATA_UI_SNAP_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("snapshot directory");
        let primitives = self.ctx.tessellate(self.output.shapes.clone(), self.output.pixels_per_point);
        let picture = crate::gui_tab_drawing::sheet_raster::paint(&primitives, &self.textures, 1.0, self.size.map(|v| v as usize));
        assert_eq!(picture.unknown_pictures, 0, "snapshot omitted a real texture");
        image::save_buffer(directory.join(format!("{name}.png")), &picture.rgb, self.size[0], self.size[1], image::ColorType::Rgb8).expect("actual UI PNG");
    }
}
