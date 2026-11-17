//! Menu key ownership, target/cpp-ref/src/app/MainWindowContextMenus.cpp:192 and KaDrawingLayerSets.cpp:64-108.
use eframe::egui::{self, Event, InnerResponse, Key, Ui, UiKind, WidgetText};

pub(crate) fn menu_button<R>(ui: &mut Ui, title: impl Into<WidgetText>, contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<R>> {
    let root = !ui.stack().contained_in(UiKind::Menu);
    let was_open = root && egui::menu::BarState::load(ui.ctx(), ui.id()).is_some();
    let response = ui.menu_button(title, contents);
    // egui0.31.1 menu.rs:379-390 closes before rendering; a child must leave Escape for that root.
    if root && (was_open || response.response.clicked()) {
        consume_canvas_keys(ui);
    }
    response
}

pub(crate) fn popup_below_widget<R>(
    ui: &Ui,
    popup_id: egui::Id,
    response: &egui::Response,
    close_behavior: egui::PopupCloseBehavior,
    contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let inner = egui::popup::popup_below_widget(ui, popup_id, response, close_behavior, contents);
    // target/cpp-ref/src/app/KaSnapSettingsWidget.cpp:24-28; egui0.31.1 popup.rs closes after the rendered body.
    if inner.is_some() {
        consume_canvas_keys(ui);
    }
    inner
}

fn consume_canvas_keys(ui: &Ui) {
    ui.input_mut(|input| input.events.retain(|event| !matches!(event, Event::Key { key: Key::Escape | Key::Enter | Key::Delete | Key::Backspace, pressed: true, .. })));
}
