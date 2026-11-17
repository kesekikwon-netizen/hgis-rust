//! Menu key ownership, target/cpp-ref/src/app/MainWindowContextMenus.cpp:192 and KaDrawingLayerSets.cpp:64-108.
use eframe::egui::{self, Event, InnerResponse, Key, Ui, UiKind, WidgetText};

pub(crate) fn menu_button<R>(ui: &mut Ui, title: impl Into<WidgetText>, contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<Option<R>> {
    let root = !ui.stack().contained_in(UiKind::Menu);
    let was_open = root && egui::menu::BarState::load(ui.ctx(), ui.id()).is_some();
    let response = ui.menu_button(title, contents);
    // egui0.31.1 menu.rs:379-390 closes before rendering; a child must leave Escape for that root.
    if root && (was_open || response.response.clicked()) {
        ui.input_mut(|input| input.events.retain(|event| !matches!(event, Event::Key { key: Key::Escape, pressed: true, .. })));
    }
    response
}
