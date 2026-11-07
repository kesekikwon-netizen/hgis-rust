use eframe::egui;
use strata_types::theme_tokens::ThemeTokens;

pub fn configure_korean_fonts_and_theme(ctx: &egui::Context, tokens: &ThemeTokens) {
    // egui turns logical points into physical pixels with pixels_per_point.
    // Leaving ppp at the OS value (1.5 here) makes the logical canvas only
    // ~1529x593 inside a 2293x889 client area, so wide 3-column layouts lose
    // their right panel and tall content runs under the status bar.
    // Re-anchor on 96 DPI and let egui keep its own scale, so one logical point
    // equals one pixel of the window: the layout then has the room it expects.
    ctx.set_pixels_per_point(1.0);
    let _ = tokens;
    let mut fonts = egui::FontDefinitions::default();
    let candidate_paths = [
        r"C:\Windows\Fonts\malgun.ttf",
        r"C:\Windows\Fonts\NanumGothic.ttf",
        "/mnt/c/Windows/Fonts/malgun.ttf",
        "/mnt/c/Windows/Fonts/NanumGothic.ttf",
        "data/fonts/NotoSansKR-Regular.ttf",
    ];
    for path in candidate_paths {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "korean_sys".to_owned(),
                egui::FontData::from_owned(bytes).into(),
            );
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                fam.insert(0, "korean_sys".to_owned());
            }
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                fam.push("korean_sys".to_owned());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
    apply_theme_tokens(ctx, tokens);
}

pub fn apply_theme_tokens(ctx: &egui::Context, tokens: &ThemeTokens) {
    let bg = tokens.desk;
    let surf = tokens.surface;
    let bdr = tokens.border;
    let acc = tokens.accent;
    ctx.all_styles_mut(|style| {
        style.visuals = egui::Visuals::light();
        style.visuals.panel_fill = egui::Color32::from_rgb(bg[0], bg[1], bg[2]);
        style.visuals.window_fill = egui::Color32::from_rgb(surf[0], surf[1], surf[2]);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(bdr[0], bdr[1], bdr[2]));
        style.visuals.selection.bg_fill =
            egui::Color32::from_rgba_unmultiplied(acc[0], acc[1], acc[2], 48);
        style.visuals.selection.stroke =
            egui::Stroke::new(1.2, egui::Color32::from_rgb(acc[0], acc[1], acc[2]));
    });
}
