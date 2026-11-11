//! 「홈」 탭 — 최근 조사 목록 상자(행·필터 표시).
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_recent_survey_box` 를 그대로 옮겼고, 순서·수치·문구는 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn render_recent_survey_box(&mut self, ui: &mut egui::Ui, width: f32) {
        let (box_r, _) = ui.allocate_exact_size(Vec2::new(width, 206.0), egui::Sense::hover());
        let p = ui.painter_at(box_r);
        p.rect_filled(box_r, 4.0, Color32::WHITE);
        p.rect_stroke(
            box_r,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(45, 44, 40)),
            egui::StrokeKind::Middle,
        );

        let col_x = [
            box_r.left() + 14.0,
            box_r.left() + width * 0.62,
            box_r.left() + width * 0.71,
            box_r.left() + width * 0.78,
            box_r.left() + width * 0.84,
            box_r.left() + width * 0.91,
        ];
        for (i, h) in ["조사", "상태", "조사구역", "유구", "검수", "마지막 열림"]
            .iter()
            .enumerate()
        {
            p.text(
                Pos2::new(col_x[i], box_r.top() + 15.0),
                egui::Align2::LEFT_CENTER,
                *h,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(95, 94, 88),
            );
        }
        p.line_segment(
            [
                Pos2::new(box_r.left(), box_r.top() + 28.0),
                Pos2::new(box_r.right(), box_r.top() + 28.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(222, 220, 210)),
        );

        // C++ `KaHomeRecentCard` — 실제로 연 조사만 보인다. 없으면 그렇다고 말한다.
        let rows = self.home_map.recent_rows.clone();
        if rows.is_empty() {
            p.text(
                Pos2::new(col_x[0], box_r.top() + 52.0),
                egui::Align2::LEFT_CENTER,
                "아직 연 조사가 없습니다. 「새 조사」나 「조사 열기」로 시작하세요.",
                egui::FontId::proportional(12.0),
                Color32::from_rgb(115, 113, 106),
            );
        }
        let filter = self.home_map.recent_filter.trim().to_owned();
        let mut clicked_row: Option<usize> = None;
        for (idx, row) in rows
            .iter()
            .filter(|r| filter.is_empty() || r.name.contains(&filter))
            .take(4)
            .enumerate()
        {
            let (name, folder, st, area, feat, chk, opened, ok) = (
                row.name.as_str(),
                row.folder.as_str(),
                row.status.as_str(),
                row.areas.as_str(),
                row.features.as_str(),
                if row.exists { "검수 전" } else { "—" },
                row.opened.as_str(),
                row.exists,
            );
            let ry = box_r.top() + 30.0 + (idx as f32) * 43.0;
            let row_r = Rect::from_min_size(
                Pos2::new(box_r.left() + 1.0, ry),
                Vec2::new(width - 2.0, 42.0),
            );
            if idx == 0 {
                p.rect_filled(row_r, 0.0, Color32::from_rgb(239, 236, 226));
            }
            p.text(
                Pos2::new(col_x[0], ry + 13.0),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(35, 35, 32),
            );
            p.text(
                Pos2::new(col_x[0], ry + 30.0),
                egui::Align2::LEFT_CENTER,
                folder,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(115, 113, 106),
            );
            let st_col = if ok {
                Color32::from_rgb(65, 64, 60)
            } else {
                Color32::from_rgb(181, 87, 58)
            };
            for (ci, val) in [(1, st), (2, area), (3, feat), (4, chk), (5, opened)] {
                let c = if ci == 1 {
                    st_col
                } else {
                    Color32::from_rgb(65, 64, 60)
                };
                p.text(
                    Pos2::new(col_x[ci], ry + 21.0),
                    egui::Align2::LEFT_CENTER,
                    val,
                    egui::FontId::proportional(12.0),
                    c,
                );
            }
            if ui
                .interact(
                    row_r,
                    ui.id().with(("recent_row", idx)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                clicked_row = rows.iter().position(|r| r.path == row.path);
            }
        }
        if let Some(i) = clicked_row {
            self.open_recent_survey_row(i);
        }
    }
}
