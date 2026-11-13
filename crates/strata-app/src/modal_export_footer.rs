//! `render_export_submit_package` 의 하단 안내·버튼 구간.
//!
//! `modal_export.rs` 가 300줄을 넘어 한 함수의 구간을 옮겼다.
//! 실행 순서는 호출부가 보장한다.
use crate::app_state::{StrataAppState};
use eframe::egui::{self, Color32, Vec2};

/// 아직 옮기지 않은 제출 기능의 말풍선(hgis-2e 규칙).
const DEFERRED_R2: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 R2)";

pub(crate) fn paint_export_submit_footer(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    _open: &mut bool,
    status_txt: &str,
) {
        // KaSubmitDialog.cpp:85-105 — 하단 버튼 행 (다시 검수 / 레이어 5179 변환… / 제출 꾸러미 만들기… / 닫기)
        ui.horizontal(|ui| {
            if ui.button("다시 검수").clicked() {
                app.home_map.draw_guide_message = Some(status_txt.to_owned());
            }
            // F1b: 변환하지 않고 「변환했습니다」만 띄우던 단추 — R2(실제 5179 SHP) 전까지 흐림.
            ui.add_enabled(false, egui::Button::new("레이어 5179 변환…"))
                .on_disabled_hover_text(DEFERRED_R2);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("닫기").clicked() {
                    app.modals.active = None;
                }
                let export_btn = egui::Button::new(
                    egui::RichText::new("제출 꾸러미 만들기…")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(160.0, 32.0));
                // F1b: 100 m 정사각형 「현장경계」 SHP·빈 조사도면.pdf 를 임시 폴더에 쓰고 「생성 완료」라던 단추 —
                // R2(SubmitPackageExporter 실물) 전까지 흐림.
                ui.add_enabled(false, export_btn).on_disabled_hover_text(DEFERRED_R2);
            });
        });
}

#[cfg(test)]
#[path = "modal_export_footer_tests.rs"]
mod tests;
