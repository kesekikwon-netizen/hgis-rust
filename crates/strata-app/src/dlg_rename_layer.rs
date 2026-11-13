//! 「이름 바꾸기」 창 — C++ `renameSelectedLayer` 의 `QInputDialog::getText(this, "이름 바꾸기", "새 이름:", Normal, 지금 이름)`
//! (MainWindow.cpp:1653-1656). 단추는 Qt 기본 OK·Cancel → qtbase_ko 「확인」「취소」. Enter 확인, Esc 취소.

use eframe::egui::{self, RichText, Vec2};

use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn render_rename_layer(&mut self, ctx: &egui::Context) {
        let Some(mut r) = self.home_map.rename_layer.clone() else { return };
        let (mut ok, mut cancel) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        ok &= r.keys_armed;
        cancel &= r.keys_armed;
        let first = !r.keys_armed;
        r.keys_armed = true;
        egui::Window::new("이름 바꾸기").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).min_width(300.0).show(ctx, |ui| {
            ui.label(RichText::new("새 이름:").size(12.0));
            let edit = ui.add(egui::TextEdit::singleline(&mut r.text).desired_width(280.0));
            if first {
                edit.request_focus();
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(Vec2::new(280.0, 28.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel |= ui.add_sized([72.0, 26.0], egui::Button::new("취소")).clicked();
                ok |= ui.add_sized([72.0, 26.0], egui::Button::new("확인")).clicked();
            });
        });
        self.home_map.rename_layer = Some(r);
        if cancel {
            self.home_map.rename_layer = None;
        } else if ok {
            self.apply_rename_layer();
        }
    }
}
