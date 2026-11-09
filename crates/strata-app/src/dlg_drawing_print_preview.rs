//! 「도면 인쇄」 대화상자 오른쪽 칸 — 용지 미리보기(모눈·점·단면 띠·축척자·방위).
//!
//! `dlg_drawing_print.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `ui.columns(2, …)` 의 오른쪽 칸 본문을 **그대로** 옮겼고, 좌표·색·문구는
//! 하나도 바꾸지 않았다. 요약 두 줄은 왼쪽 칸에서 계산한 값을 그대로 받는다.
//!
//! `cols[1]` 이던 수신자를 `right` 로 이름만 바꿨다(같은 칸을 가리킨다).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// 오른쪽 칸 미리보기. `paper_summary`·`scale_summary` 는 왼쪽 칸이 만든 문구다.
pub(crate) fn render_print_preview(
    right: &mut egui::Ui,
    paper_summary: &str,
    scale_summary: &str,
) {
                right.horizontal(|ui| {
                    ui.label("미리보기");
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "도면 · {} · {}",
                                    paper_summary, scale_summary
                                ))
                                .size(12.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                            );
                        },
                    );
                });
                let (pr, _) = right
                    .allocate_exact_size(Vec2::new(350.0, 370.0), egui::Sense::hover());
                let p = right.painter_at(pr);
                p.rect_filled(pr, 2.0, Color32::from_rgb(236, 234, 226));
                let paper_r = pr.shrink2(Vec2::new(34.0, 14.0));
                p.rect_filled(paper_r, 0.0, Color32::WHITE);
                p.rect_stroke(
                    paper_r,
                    0.0,
                    Stroke::new(1.0, Color32::BLACK),
                    egui::StrokeKind::Middle,
                );
                let map_r = Rect::from_min_max(
                    paper_r.min + Vec2::new(28.0, 44.0),
                    Pos2::new(paper_r.right() - 28.0, paper_r.bottom() - 56.0),
                );
                p.rect_filled(map_r, 0.0, Color32::from_rgb(154, 198, 120));
                for g in 1..12 {
                    let gx = map_r.left() + (g as f32) * (map_r.width() / 12.0);
                    let gy = map_r.top() + (g as f32) * (map_r.height() / 12.0);
                    p.line_segment(
                        [Pos2::new(gx, map_r.top()), Pos2::new(gx, map_r.bottom())],
                        Stroke::new(0.7, Color32::from_rgb(30, 30, 28)),
                    );
                    p.line_segment(
                        [Pos2::new(map_r.left(), gy), Pos2::new(map_r.right(), gy)],
                        Stroke::new(0.7, Color32::from_rgb(30, 30, 28)),
                    );
                }
                p.rect_stroke(
                    map_r,
                    0.0,
                    Stroke::new(1.2, Color32::BLACK),
                    egui::StrokeKind::Middle,
                );
                for i in 0..85 {
                    let (fx, fy) = (
                        ((i * 37 + 13) % 100) as f32 / 100.0 - 0.5,
                        ((i * 61 + 29) % 100) as f32 / 100.0 - 0.5,
                    );
                    let dp = Pos2::new(
                        map_r.center().x + fx * map_r.width() * 0.88,
                        map_r.center().y + fy * map_r.height() * 0.88,
                    );
                    let dc = if i % 4 == 0 {
                        Color32::from_rgb(45, 165, 75)
                    } else {
                        Color32::from_rgb(198, 28, 118)
                    };
                    p.circle_filled(dp, 2.2, dc);
                    p.circle_stroke(dp, 2.2, Stroke::new(0.6, Color32::BLACK));
                }
                let sr = Rect::from_center_size(
                    map_r.center(),
                    Vec2::new(14.0, map_r.height() * 0.56),
                );
                p.rect_filled(sr, 0.0, Color32::from_rgba_unmultiplied(214, 92, 32, 130));
                p.rect_stroke(
                    sr,
                    0.0,
                    Stroke::new(1.4, Color32::from_rgb(214, 78, 22)),
                    egui::StrokeKind::Middle,
                );
                p.line_segment(
                    [
                        Pos2::new(map_r.left() + 4.0, map_r.bottom() + 10.0),
                        Pos2::new(map_r.left() + 68.0, map_r.bottom() + 10.0),
                    ],
                    Stroke::new(2.5, Color32::BLACK),
                );
                p.text(
                    Pos2::new(map_r.left() + 36.0, map_r.bottom() + 20.0),
                    egui::Align2::CENTER_CENTER,
                    "축척 1 : 6532",
                    egui::FontId::proportional(7.5),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(map_r.right() - 18.0, map_r.bottom() + 10.0),
                    egui::Align2::RIGHT_CENTER,
                    "동부원점(GRS80) · EPSG:5187",
                    egui::FontId::proportional(7.0),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(map_r.right() - 5.0, map_r.bottom() + 12.0),
                    egui::Align2::CENTER_CENTER,
                    "N\n▲",
                    egui::FontId::proportional(8.5),
                    Color32::BLACK,
                );
}
