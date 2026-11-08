use eframe::egui;
use strata_types::theme_tokens::ThemeTokens;

pub fn configure_korean_fonts_and_theme(ctx: &egui::Context, tokens: &ThemeTokens) {
    // C++ Qt6 와 동일하게 모니터의 OS DPI 배율(100% / 125% / 150% / 200%)을 그대로 따르며,
    // 제목 표시줄(Title Bar)도 C++ 앱과 동일하게 밝은(Light) 테마로 고정한다.
    ctx.set_theme(egui::Theme::Light);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Light));
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
