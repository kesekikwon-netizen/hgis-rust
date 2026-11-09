//! 「수치지형도 좌표계 확인」 정합 미리보기 캔버스.
//!
//! `dlg_topographic_import.rs` 가 300줄 규칙에 맞춰 나눈 파일이다
//! (`KaTopographicImportDialog.cpp:304-306, 498-522`). 원래 미리보기 블록을 **그대로**
//! 옮겼고, 좌표·색·문구는 하나도 바꾸지 않았다.
//! 하단 단추가 같은 좌표계 선택을 쓰므로 `(sel_crs_idx, sel_crs_lbl)` 을 돌려준다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::TOPO_CRS_UNKNOWN;

/// 미리보기를 그리고 `(선택 좌표계 번호, 그 이름)` 을 돌려준다.
pub(crate) fn render_topo_import_preview(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    modal_w: f32,
    rows: &[crate::modal_types::TopoTableRow],
    sel_ri: usize,
    row_count: usize,
) -> (usize, String) {
ui.add_space(6.0);
// 정합 미리보기 캔버스 (m_preview, KaTopographicImportDialog.cpp:304-306, 498-522)
let sel_crs_idx = app
    .modals
    .topo_row_crs_idx
    .get(sel_ri)
    .copied()
    .unwrap_or(0);
let sel_crs_lbl = rows
    .get(sel_ri)
    .map_or_else(|| TOPO_CRS_UNKNOWN.to_owned(), |row| row.choice_label(sel_crs_idx));
let aligned = rows
    .get(sel_ri)
    .is_some_and(|row| row.choice_is_work_crs_family(sel_crs_idx));
let (prev_r, _) = ui.allocate_exact_size(
    Vec2::new(modal_w - 28.0, 125.0),
    egui::Sense::hover(),
);
let pp = ui.painter_at(prev_r);
pp.rect_filled(prev_r, 5.0, Color32::from_rgb(248, 247, 243));
pp.rect_stroke(
    prev_r,
    5.0,
    Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
    egui::StrokeKind::Inside,
);
// 기준 지적·조사구역 (정위치)
let pc = prev_r.center();
for gx in -4..=4 {
    let x = pc.x + (gx as f32) * 85.0;
    pp.line_segment(
        [
            Pos2::new(x, prev_r.top() + 6.0),
            Pos2::new(x, prev_r.bottom() - 6.0),
        ],
        Stroke::new(0.6, Color32::from_rgb(218, 215, 206)),
    );
}
let survey_box = Rect::from_center_size(pc, Vec2::new(120.0, 74.0));
pp.rect_filled(
    survey_box,
    2.0,
    Color32::from_rgba_unmultiplied(234, 88, 12, 55),
);
pp.rect_stroke(
    survey_box,
    2.0,
    Stroke::new(2.0, Color32::from_rgb(234, 88, 12)),
    egui::StrokeKind::Middle,
);
// C++ `previewCurrent(:498-522)` 는 실제 도엽을 좌표변환해 겹쳐 그린다.
// 여기서는 WGSL/지도 캔버스 없이 같은 판단(원점대가 작업 좌표계 계열인가)만 보인다.
let shift_x: f32 = if aligned { 0.0 } else { 90.0 };
let topo_col = if aligned {
    Color32::from_rgb(21, 128, 61)
} else {
    Color32::from_rgb(185, 28, 28)
};
for dy in [-26.0_f32, 0.0, 24.0] {
    pp.line_segment(
        [
            Pos2::new(pc.x - 160.0 + shift_x, pc.y + dy - 12.0),
            Pos2::new(pc.x + 160.0 + shift_x, pc.y + dy + 14.0),
        ],
        Stroke::new(1.8, topo_col),
    );
}
pp.text(
    Pos2::new(prev_r.left() + 10.0, prev_r.top() + 14.0),
    egui::Align2::LEFT_CENTER,
    format!(
        "정합 미리보기 — 선택 행 #{}: {} ({})",
        if row_count == 0 { 0 } else { sel_ri + 1 },
        sel_crs_lbl,
        if aligned {
            "● 기존 조사구역·지적도와 정합 일치"
        } else {
            "▲ 좌표계 원점 불일치로 도엽 위치가 어긋남 — 원본 좌표계를 확인하세요"
        }
    ),
    egui::FontId::proportional(11.5),
    topo_col,
);
    (sel_crs_idx, sel_crs_lbl)
}
