//! `render_ri_body` 의 카드 본문(속성 카드 / 레이어 카드 분기).
//!
//! `right_inspector.rs` 가 300줄을 넘어 옮겼다. 실행 순서는 호출부가 보장한다.
use eframe::egui::{self};
use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn render_ri_body_cards(&mut self, ui: &mut egui::Ui, is_attr: bool) {
        if is_attr {
            self.render_ri_attribute_card(ui);
        } else {
            self.render_ri_layer_card(ui);
        }
    }
}
