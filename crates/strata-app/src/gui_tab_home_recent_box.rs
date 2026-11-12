//! 「홈」 탭 — 최근 조사 목록 상자(행·필터 표시).
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 행은 C++ `KaHomeRecentCard::setItems`
//! (`KaHomeRecentCard.cpp:117-170`)처럼 저장된 최근 조사 목록 그대로다. 조사구역·유구·검수 칸은
//! C++ 이 기록(SurveyFacts)이 없을 때 보이는 「—」·「검수 전」이다 — 기록 읽기는 아직 없다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_storage::recent_surveys::RecentSurvey;

use crate::app_state::StrataAppState;

/// 한 행에 쓰는 글(C++ 칸 순서).
struct RowText {
    name: String,
    folder: String,
    state: &'static str,
    cells: [&'static str; 3],
    opened: String,
    tip: String,
}

fn row_text(it: &RecentSurvey) -> RowText {
    let (path, name) = (&it.path, it.name.clone());
    let folder = crate::home_text::folder_end(&crate::home_text::folder(path));
    if it.available {
        RowText {
            name,
            folder,
            state: "저장됨",
            cells: ["—", "—", "검수 전"],
            opened: crate::home_text::when(it.last_opened_ms),
            tip: format!("누르면 이 조사를 엽니다.\n{path}"),
        }
    } else {
        RowText {
            name,
            folder,
            state: "원본 없음",
            cells: ["—", "—", "—"],
            opened: "찾을 수 없음".to_owned(),
            tip: format!("지금 찾을 수 없습니다. USB·네트워크 드라이브를 연결하면 다시 열 수 있습니다.\n{path}"),
        }
    }
}

/// `applyFilter`(:172-185) — 이름·상태·경로에 찾는 글이 들어 있는가(대소문자 무시).
fn matches_filter(it: &RecentSurvey, state: &str, needle: &str) -> bool {
    let n = needle.to_lowercase();
    n.is_empty() || it.name.to_lowercase().contains(&n) || state.contains(needle) || it.path.to_lowercase().contains(&n)
}

impl StrataAppState {
    pub(crate) fn render_recent_survey_box(&mut self, ui: &mut egui::Ui, width: f32) {
        if self.recent_surveys.is_empty() {
            // :79-83 — 목록 대신 안내 한 줄.
            ui.label(
                egui::RichText::new("아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요.")
                    .size(12.0)
                    .color(Color32::from_rgb(95, 94, 88)),
            );
            return;
        }
        let needle = self.home_map.recent_filter.trim().to_owned();
        let rows: Vec<(String, RowText)> = self
            .recent_surveys
            .iter()
            .filter_map(|it| {
                let t = row_text(it);
                matches_filter(it, t.state, &needle).then(|| (it.path.clone(), t))
            })
            .collect();
        let box_h = 30.0 + 43.0 * rows.len() as f32 + 4.0;
        let (box_r, _) = ui.allocate_exact_size(Vec2::new(width, box_h), egui::Sense::hover());
        let p = ui.painter_at(box_r);
        p.rect_filled(box_r, 4.0, Color32::WHITE);
        p.rect_stroke(box_r, 4.0, Stroke::new(1.0, Color32::from_rgb(45, 44, 40)), egui::StrokeKind::Middle);

        let col_x = [
            box_r.left() + 14.0,
            box_r.left() + width * 0.62,
            box_r.left() + width * 0.71,
            box_r.left() + width * 0.78,
            box_r.left() + width * 0.84,
            box_r.left() + width * 0.91,
        ];
        for (i, h) in ["조사", "상태", "조사구역", "유구", "검수", "마지막 열림"].iter().enumerate() {
            p.text(
                Pos2::new(col_x[i], box_r.top() + 15.0),
                egui::Align2::LEFT_CENTER,
                *h,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(95, 94, 88),
            );
        }
        p.line_segment(
            [Pos2::new(box_r.left(), box_r.top() + 28.0), Pos2::new(box_r.right(), box_r.top() + 28.0)],
            Stroke::new(1.0, Color32::from_rgb(222, 220, 210)),
        );

        let mut open: Option<String> = None;
        let mut forget: Option<String> = None;
        for (idx, (path, t)) in rows.iter().enumerate() {
            let ry = box_r.top() + 30.0 + (idx as f32) * 43.0;
            let row_r = Rect::from_min_size(Pos2::new(box_r.left() + 1.0, ry), Vec2::new(width - 2.0, 42.0));
            if idx == 0 {
                p.rect_filled(row_r, 0.0, Color32::from_rgb(239, 236, 226));
            }
            p.text(
                Pos2::new(col_x[0], ry + 13.0),
                egui::Align2::LEFT_CENTER,
                &t.name,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(35, 35, 32),
            );
            p.text(
                Pos2::new(col_x[0], ry + 30.0),
                egui::Align2::LEFT_CENTER,
                &t.folder,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(115, 113, 106),
            );
            let missing = t.state == "원본 없음";
            let state = if missing { "⊗ 원본 없음" } else { t.state };
            let st_col = if missing { Color32::from_rgb(181, 87, 58) } else { Color32::from_rgb(65, 64, 60) };
            let [area, feat, chk] = t.cells;
            for (ci, val) in [(1, state), (2, area), (3, feat), (4, chk), (5, t.opened.as_str())] {
                let c = if ci == 1 { st_col } else { Color32::from_rgb(65, 64, 60) };
                p.text(
                    Pos2::new(col_x[ci], ry + 21.0),
                    egui::Align2::LEFT_CENTER,
                    val,
                    egui::FontId::proportional(12.0),
                    c,
                );
            }
            let resp = ui.interact(row_r, ui.id().with(("recent_row", path)), egui::Sense::click()).on_hover_text(&t.tip);
            if resp.clicked() {
                open = Some(path.clone());
            }
            // :213-235 오른쪽 단추 메뉴 — 복구 사본 항목은 아직 없다.
            resp.context_menu(|ui| {
                if ui.button("목록에서 제거").clicked() {
                    forget = Some(path.clone());
                    ui.close_menu();
                }
            });
        }
        if let Some(path) = forget {
            self.forget_recent_survey(&path);
        }
        if let Some(path) = open {
            self.open_recent_survey(&path);
        }
    }
}
