//! 「수치지형도 좌표계 확인」 하단 단추 줄과 진행률 막대.
//!
//! `dlg_topographic_import.rs` 가 300줄 규칙에 맞춰 나눈 파일이다
//! (`KaTopographicImportDialog.cpp:307-321, 493-495, 524-537`). 원래 하단 블록을
//! **그대로** 옮겼고, 단추 순서·문구·동작은 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Rect, Vec2};

use crate::app_state::StrataAppState;

/// 하단 단추 줄 + 진행률 막대 + 상태 문구.
pub(crate) fn render_topo_import_actions(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    modal_w: f32,
    rows: &[crate::modal_types::TopoTableRow],
    sel_crs_idx: usize,
    sel_crs_lbl: &str,
) {
ui.add_space(6.0);
// 하단 컨트롤 행 (KaTopographicImportDialog.cpp:307-318)
ui.horizontal(|ui| {
    if ui
        .checkbox(&mut app.modals.topo_maps_enabled, "수치지형도 표시")
        .changed()
    {
        app.home_map.real_survey.layer_visible.insert(
            "수치지형도 1:5,000 (합침)".to_owned(),
            app.modals.topo_maps_enabled,
        );
    }
    if ui.button("선택 도엽 정합 미리보기").clicked() {
        if rows.is_empty() {
            app.modals.topo_status_text =
                "먼저 받은 폴더를 골라 자료를 확인하세요.".to_owned();
        } else if sel_crs_idx == 0 {
            app.modals.topo_status_text =
                "먼저 미리 볼 원본 좌표계를 선택하세요.".to_owned();
        } else {
            app.modals.topo_status_text = format!(
                "선택 좌표계({sel_crs_lbl})의 미리보기입니다. 위성·지적과 정합을 확인한 뒤 추가하세요."
            );
        }
    }

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button("닫기").clicked() {
            app.modals.active = None;
        }
        if ui.button("색인 취소").clicked() {
            app.modals.cancel_topo_scan();
            app.modals.active = None;
        }
        // KaTopographicImportDialog.cpp:493 — 올릴 항목이 없으면 누를 수 없다.
        let add_btn = egui::Button::new(
            egui::RichText::new("좌표계 확인 완료 · 지도에 추가")
                .color(Color32::WHITE)
                .strong()
                .size(13.0),
        )
        .fill(Color32::from_rgb(181, 87, 58))
        .corner_radius(6.0)
        .min_size(Vec2::new(210.0, 32.0));
        if ui.add_enabled(!rows.is_empty(), add_btn).clicked() {
            // KaTopographicImportDialog.cpp:524-537 — 범위·좌표계가 확인된
            // 항목만 올린다. 하나라도 걸리면 아무것도 올리지 않고 문구로 끝낸다.
            if let Some(reason) = app.modals.topo_blocking_message() {
                app.modals.topo_status_text = reason.to_owned();
            } else {
                let picked = app.modals.topo_checked_rows();
                for row in &picked {
                    if let Some(slot) =
                        app.modals.topo_row_already_loaded.get_mut(*row)
                    {
                        *slot = true;
                    }
                }
                app.modals.topo_imported_count = picked.len();
                if picked.is_empty() {
                    // C++ `:524-527` — 체크가 없으면 올릴 것이 없다.
                    app.modals.topo_status_text =
                        "추가할 항목에 체크하세요.".to_owned();
                } else {
                    app.modals.topo_maps_enabled = true;
                    app.home_map.real_survey.layer_visible.insert(
                        "수치지형도 1:5,000 (합침)".to_owned(),
                        true,
                    );
                    app.modals.topo_status_text = format!(
                        "수치지형도 레이어 {}개를 작업 좌표계(EPSG:5187)로 정합 변환하여 지도에 추가했습니다 ([이미 있음] 표시).",
                        picked.len()
                    );
                    app.home_map.draw_guide_message =
                        Some(app.modals.topo_status_text.clone());
                }
            }
        }
    });
});

ui.add_space(4.0);
// 진행률 바 + 상태 문구 (KaTopographicImportDialog.cpp:319-321, 494-495)
let (pbar_r, _) = ui
    .allocate_exact_size(Vec2::new(modal_w - 28.0, 14.0), egui::Sense::hover());
let bp = ui.painter_at(pbar_r);
bp.rect_filled(pbar_r, 3.0, Color32::from_rgb(232, 229, 220));
bp.rect_filled(
    Rect::from_min_size(
        pbar_r.min,
        Vec2::new(
            pbar_r.width() * app.modals.topo_progress_pct.clamp(0.0, 1.0),
            pbar_r.height(),
        ),
    ),
    3.0,
    Color32::from_rgb(44, 43, 41),
);
ui.label(
    egui::RichText::new(&app.modals.topo_status_text)
        .size(11.0)
        .color(Color32::from_rgb(55, 54, 50)),
);
}
