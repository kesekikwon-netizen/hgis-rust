//! Persistent connected tools, target/cpp-ref/src/app/MainWindowRibbon.cpp:768-812,841-951.
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};

pub(super) const CONTROL_HEIGHT: f32 = 34.0;

pub(super) fn context_card(ui: &mut egui::Ui, origin: Rect, desired_width: f32, add_controls: impl FnOnce(&mut egui::Ui)) {
    // The owner requests connected cards; keep the C++ toolbar's reserved map space.
    let width = desired_width.min((ui.available_width() - 16.0).max(1.0));
    let left = origin.left().clamp(ui.max_rect().left() + 8.0, (ui.max_rect().right() - width - 8.0).max(ui.max_rect().left() + 8.0));
    ui.add_space(10.0);
    let edge = Stroke::new(1.0, ui.visuals().window_stroke().color);
    let fill = ui.visuals().window_fill();
    let mut card = Rect::NOTHING;
    let max_height = (ui.ctx().screen_rect().bottom() - ui.cursor().top() - 51.0 - 32.0 - 120.0 - 26.0).max(CONTROL_HEIGHT);
    ui.horizontal(|ui| {
        ui.add_space((left - ui.cursor().left()).max(0.0));
        ui.allocate_ui_with_layout(Vec2::new(width, max_height + 18.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            card = egui::Frame::NONE
                .fill(fill)
                .stroke(edge)
                .corner_radius(10)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width((width - 22.0).max(1.0));
                    ui.spacing_mut().interact_size.y = CONTROL_HEIGHT;
                    ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    ui.style_mut().override_font_id = Some(egui::FontId::proportional(13.0));
                    // Reserve tabs, status and a usable map, even when the controls need scrolling.
                    egui::ScrollArea::vertical().id_salt("context-tools-scroll").max_height(max_height).auto_shrink([false, true]).show(ui, |ui| {
                        ui.horizontal_wrapped(add_controls);
                    });
                })
                .response
                .rect;
        });
    });
    let x = origin.center().x.clamp(card.left() + 18.0, card.right() - 18.0);
    let tip = Pos2::new(x, card.top() - 8.0);
    let a = Pos2::new(x - 8.0, card.top() + 1.0);
    let b = Pos2::new(x + 8.0, card.top() + 1.0);
    ui.painter().add(egui::Shape::convex_polygon(vec![tip, b, a], fill, Stroke::NONE));
    ui.painter().line_segment([a, tip], edge);
    ui.painter().line_segment([tip, b], edge);
    ui.add_space(8.0);
}

pub(super) fn tool_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(label).min_size(Vec2::new(0.0, CONTROL_HEIGHT)).corner_radius(8)
}

pub(crate) fn context_tool_id(icon: &str) -> egui::Id {
    egui::Id::new(("ka-context-tool", icon))
}

pub(super) fn icon_button(ui: &mut egui::Ui, icon: &str, chosen: bool, enabled: bool, tip: &str) -> egui::Response {
    // target/cpp-ref/src/app/MainWindowRibbon.cpp:856-910: each action keeps its identity when laid out again.
    let (rect, response) = ui
        .add_enabled_ui(enabled, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(CONTROL_HEIGHT), egui::Sense::hover());
            (rect, ui.interact(rect, context_tool_id(icon), egui::Sense::click()))
        })
        .inner;
    let response = if enabled { response.on_hover_text(tip) } else { response.on_disabled_hover_text(tip) };
    let visuals = ui.style().interact_selectable(&response, chosen);
    let ink = if enabled { visuals.fg_stroke.color } else { ui.visuals().weak_text_color() };
    ui.painter().rect_filled(rect, 8, visuals.weak_bg_fill);
    ui.painter().rect_stroke(rect, 8, visuals.bg_stroke, egui::StrokeKind::Inside);
    crate::svg_path_paint::paint_svg_icon(ui.painter(), rect.shrink(7.0), icon, ink);
    response
}
