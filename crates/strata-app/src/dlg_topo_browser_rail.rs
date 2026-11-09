//! 수치지형도 브라우저의 6단계 단계 띠(`KaStageRail.cpp`).
//!
//! `dlg_topographic_browser.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_topographic_browser` 안의 띠 그리기 블록을 **그대로** 옮겼다.
//! 단계 이름·색·문구는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Vec2};

/// `KaStageRail.cpp` — 6단계 원형 배지 + 연결선 (완료 ●테라코타, 현재 ◉강조, 대기 ○회색).
pub(crate) fn paint_stage_rail(ui: &mut egui::Ui, stages: &[&str], stage: usize) {
{
    let (rail_r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(560.0), 30.0),
        egui::Sense::hover(),
    );
    let rp = ui.painter_at(rail_r);
    let n = stages.len();
    let step = rail_r.width() / n as f32;
    for i in 0..n {
        let cx = rail_r.left() + step * (i as f32 + 0.5);
        let cy = rail_r.top() + 9.0;
        let center = egui::Pos2::new(cx, cy);
        // 연결선
        if i + 1 < n {
            rp.line_segment(
                [center, egui::Pos2::new(cx + step, cy)],
                egui::Stroke::new(
                    1.5,
                    if i < stage {
                        Color32::from_rgb(181, 87, 58)
                    } else {
                        Color32::from_rgb(210, 207, 198)
                    },
                ),
            );
        }
        if i < stage {
            // 완료 — 테라코타 채움
            rp.circle_filled(center, 6.0, Color32::from_rgb(181, 87, 58));
            rp.text(
                center,
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(8.5),
                Color32::WHITE,
            );
        } else if i == stage {
            // 현재 — 강조 테두리
            rp.circle_filled(center, 7.5, Color32::WHITE);
            rp.circle_stroke(
                center,
                7.5,
                egui::Stroke::new(2.2, Color32::from_rgb(181, 87, 58)),
            );
            rp.circle_filled(center, 3.4, Color32::from_rgb(181, 87, 58));
        } else {
            // 대기 — 회색
            rp.circle_filled(center, 6.0, Color32::from_rgb(238, 236, 228));
            rp.circle_stroke(
                center,
                6.0,
                egui::Stroke::new(1.0, Color32::from_rgb(190, 187, 178)),
            );
        }
        rp.text(
            egui::Pos2::new(cx, rail_r.top() + 23.0),
            egui::Align2::CENTER_CENTER,
            format!("{}", i + 1),
            egui::FontId::proportional(9.0),
            if i <= stage {
                Color32::from_rgb(45, 44, 40)
            } else {
                Color32::from_rgb(150, 148, 140)
            },
        );
    }
}
}
