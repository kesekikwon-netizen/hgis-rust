//! `KaDrawGuideBand.cpp:55-69` — `[↶]` · `[↷]` 정사각형 아이콘 단추 쌍.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
/// `KaDrawGuideBand.cpp:55-69` — `[↶]` · `[↷]` 정사각형 아이콘 버튼 쌍 (68×34, 실제 undo/redo 호출)
impl StrataAppState {
    pub(crate) fn paint_undo_redo_pill(&mut self, ui: &mut egui::Ui, p: &egui::Painter, min: Pos2) {
    let ur_r = Rect::from_min_size(min, Vec2::new(68.0, 34.0));
    p.rect_filled(ur_r, 6.0, Color32::WHITE);
    p.rect_stroke(
        ur_r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
        egui::StrokeKind::Middle,
    );
    p.line_segment(
        [
            Pos2::new(ur_r.center().x, ur_r.top() + 5.0),
            Pos2::new(ur_r.center().x, ur_r.bottom() - 5.0),
        ],
        Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
    );
    let undo_r = Rect::from_min_max(ur_r.min, Pos2::new(ur_r.center().x, ur_r.max.y));
    let redo_r = Rect::from_min_max(Pos2::new(ur_r.center().x, ur_r.min.y), ur_r.max);
    let arrow_ink = Color32::from_rgb(45, 44, 40);
    // 벡터 Undo 아이콘 (왼쪽으로 굽은 화살표)
    {
        let c = undo_r.center();
        p.line_segment(
            [
                Pos2::new(c.x - 6.0, c.y - 2.0),
                Pos2::new(c.x + 3.0, c.y - 2.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x + 3.0, c.y - 2.0),
                Pos2::new(c.x + 6.0, c.y + 1.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x + 6.0, c.y + 1.0),
                Pos2::new(c.x + 3.0, c.y + 5.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x + 3.0, c.y + 5.0),
                Pos2::new(c.x - 3.0, c.y + 5.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 6.0, c.y - 2.0),
                Pos2::new(c.x - 2.5, c.y - 5.5),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 6.0, c.y - 2.0),
                Pos2::new(c.x - 2.5, c.y + 1.5),
            ],
            Stroke::new(1.6, arrow_ink),
        );
    }
    // 벡터 Redo 아이콘 (오른쪽으로 굽은 화살표)
    {
        let c = redo_r.center();
        p.line_segment(
            [
                Pos2::new(c.x + 6.0, c.y - 2.0),
                Pos2::new(c.x - 3.0, c.y - 2.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 3.0, c.y - 2.0),
                Pos2::new(c.x - 6.0, c.y + 1.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 6.0, c.y + 1.0),
                Pos2::new(c.x - 3.0, c.y + 5.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 3.0, c.y + 5.0),
                Pos2::new(c.x + 3.0, c.y + 5.0),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x + 6.0, c.y - 2.0),
                Pos2::new(c.x + 2.5, c.y - 5.5),
            ],
            Stroke::new(1.6, arrow_ink),
        );
        p.line_segment(
            [
                Pos2::new(c.x + 6.0, c.y - 2.0),
                Pos2::new(c.x + 2.5, c.y + 1.5),
            ],
            Stroke::new(1.6, arrow_ink),
        );
    }

    if ui
        .interact(
            undo_r,
            ui.id().with("ka_canvas_undo_btn"),
            egui::Sense::click(),
        )
        .on_hover_text("마지막 그리기·정점·삭제를 되돌립니다 (Ctrl+Z)")
        .clicked()
    {
        // KaDrawGuideBand::setActions(m_actUndo, m_actRedo) — Ctrl+Z 와 같은 undoMapAction.
        self.undo_map_action();
    }
    if ui
        .interact(
            redo_r,
            ui.id().with("ka_canvas_redo_btn"),
            egui::Sense::click(),
        )
        .on_hover_text("다시 실행 (Ctrl+Y) — 되돌린 편집을 다시 적용합니다")
        .clicked()
    {
        self.redo_map_action();
    }
}
}
