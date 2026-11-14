//! 「수치지형」 도엽 확인 줄 그리기.
//!
//! C++ embeds a browser view running `data/providers/ngii-topographic.js`
//! (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:509-537`); until WB2 connects WB0,
//! the lookup button is disabled and sends no scripted HTTP request.

use eframe::egui::{self, Color32, Vec2};
use strata_net::ngii_topographic::NgiiTopographicClient;

use crate::app_state::StrataAppState;

/// Deferred until WB2 embeds the browser view (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:509-537`).
const DEFERRED_WB2: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 WB2)";

const IDLE_STATUS: &str =
    "「도엽 확인」을 누르면 조사 주변 5km의 1:25,000 DXF 도엽을 국토정보플랫폼에 묻습니다.";

/// 도엽 확인 줄 — 창 안에 넣는다.
pub fn render_sheet_lookup(_app: &mut StrataAppState, ui: &mut egui::Ui, _ctx: &egui::Context) {
    ui.horizontal(|ui| {
        let btn = egui::Button::new(
            egui::RichText::new("도엽 확인")
                .color(Color32::WHITE)
                .strong()
                .size(12.5),
        )
        .fill(Color32::from_rgb(44, 43, 41))
        .corner_radius(5.0)
        .min_size(Vec2::new(96.0, 30.0));
        ui.add_enabled(false, btn)
            .on_disabled_hover_text(DEFERRED_WB2);
        ui.label(
            egui::RichText::new(IDLE_STATUS)
                .size(11.5)
                .color(Color32::from_rgb(95, 94, 88)),
        );
    });
    // 로그인이 필요한 다음 단계는 C++ 이 내장 브라우저로 처리한다 — 같은 안내를 보인다.
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(NgiiTopographicClient::login_stage_blocked_message())
            .size(11.5)
            .color(Color32::from_rgb(120, 118, 110)),
    );
}

#[cfg(test)]
#[path = "ngii_topographic_state_tests.rs"]
mod tests;
