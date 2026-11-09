//! 「입체지형」 탭의 우측 3D 뷰 — 마우스 회전·휠 거리·Perspective 메시·축척자·방위표·표고 범례.
//!
//! `gui_tab_terrain3d.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_terrain3d_tab_ui` 안에 있던 2번 절(우측 중앙 3D 뷰)을 그대로 옮겼고,
//! 문장 순서·수치·문구는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use strata_terrain3d::viewport3d_renderer::render_terrain3d_rgba;

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// 2. 우측 중앙 3D 뷰 (`KaTerrain3dView.cpp:46-90`)
    pub(crate) fn render_terrain3d_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

            // KaTerrain3dView.cpp:63-73 — 마우스 드래그 회전 (m_yaw += d.x * 0.4f, m_pitch clamp 8.0..80.0)
            if response.dragged() {
                let d = response.drag_delta();
                self.paleo_3d.camera_3d.yaw_deg =
                    (self.paleo_3d.camera_3d.yaw_deg + d.x * 0.4).rem_euclid(360.0);
                self.paleo_3d.camera_3d.pitch_deg =
                    (self.paleo_3d.camera_3d.pitch_deg + d.y * 0.3).clamp(8.0, 80.0);
                self.paleo_3d.rebuild_3d_preview();
            }

            // KaTerrain3dView.cpp:83-90 — 마우스 휠 거리 조절
            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y.abs() > 0.5 {
                    let factor = if scroll_y > 0.0 { 1.08 } else { 0.92 };
                    self.paleo_3d.camera_3d.zoom =
                        (self.paleo_3d.camera_3d.zoom * factor).clamp(0.5, 2.5);
                    self.paleo_3d.rebuild_3d_preview();
                }
            }

            let painter = ui.painter_at(rect);

            // 고해상도(480x300) 3D 투시 메시 렌더링 (Terrain3dService::renderPerspective)
            let render_w = 480u32;
            let render_h = 300u32;
            self.paleo_3d.camera_3d.z_exaggeration = self.paleo_3d.z_exaggeration;
            let rgba_buf = if let Some(ref out) = self.paleo_3d.last_paleo_output {
                render_terrain3d_rgba(
                    &out.z_paleo,
                    out.transform.width,
                    out.transform.height,
                    &self.paleo_3d.camera_3d,
                    render_w,
                    render_h,
                )
            } else {
                render_terrain3d_rgba(
                    &[42.0; 64],
                    8,
                    8,
                    &self.paleo_3d.camera_3d,
                    render_w,
                    render_h,
                )
            };

            if rgba_buf.len() == (render_w as usize) * (render_h as usize) * 4 {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [render_w as usize, render_h as usize],
                    &rgba_buf,
                );
                let tex = ctx.load_texture("terrain3d_tex", img, egui::TextureOptions::LINEAR);
                painter.image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            // 상단 오버레이 배지
            let title_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
                Vec2::new(490.0, 28.0),
            );
            painter.rect_filled(title_rect, 4.0, Color32::from_rgba_unmultiplied(15, 23, 42, 185));
            painter.text(
                Pos2::new(title_rect.left() + 10.0, title_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!(
                    "입체지형 (KaTerrain3dView) — 높이 과장 {:.1}배 · 방위각 {:.0}° · 경사 {:.0}° (드래그 회전)",
                    self.paleo_3d.z_exaggeration,
                    self.paleo_3d.camera_3d.yaw_deg,
                    self.paleo_3d.camera_3d.pitch_deg
                ),
                FontId::proportional(12.5),
                Color32::WHITE,
            );

            // Terrain3dService::composeExport (Terrain3dService.cpp:505-545) — 하단 축척자 · 방위(North) · 표고 범례 바
            let footer_h = 36.0;
            let footer_rect = Rect::from_min_max(
                Pos2::new(rect.left(), rect.bottom() - footer_h),
                rect.max,
            );
            painter.rect_filled(
                footer_rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 235),
            );
            painter.line_segment(
                [
                    Pos2::new(footer_rect.left(), footer_rect.top()),
                    Pos2::new(footer_rect.right(), footer_rect.top()),
                ],
                Stroke::new(1.2, Color32::from_rgb(17, 24, 39)),
            );

            // 좌측 하단 축척자 (0 — 500 m)
            let bar_x0 = footer_rect.left() + 18.0;
            let bar_w = 140.0 / self.paleo_3d.camera_3d.zoom.clamp(0.5, 2.0);
            let bar_y = footer_rect.center().y + 4.0;
            painter.line_segment(
                [Pos2::new(bar_x0, bar_y), Pos2::new(bar_x0 + bar_w, bar_y)],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.line_segment(
                [
                    Pos2::new(bar_x0, bar_y - 5.0),
                    Pos2::new(bar_x0, bar_y + 4.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.line_segment(
                [
                    Pos2::new(bar_x0 + bar_w, bar_y - 5.0),
                    Pos2::new(bar_x0 + bar_w, bar_y + 4.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.text(
                Pos2::new(bar_x0, bar_y - 12.0),
                egui::Align2::LEFT_CENTER,
                "0",
                FontId::proportional(11.0),
                Color32::from_rgb(17, 24, 39),
            );
            painter.text(
                Pos2::new(bar_x0 + bar_w, bar_y - 12.0),
                egui::Align2::RIGHT_CENTER,
                "500 m",
                FontId::proportional(11.0),
                Color32::from_rgb(17, 24, 39),
            );

            // 중앙 하단 DEM 표고 범례 및 조사구역 표시
            let leg_x = bar_x0 + bar_w + 36.0;
            let leg_y = footer_rect.center().y;
            let elev_swatches = [
                (Color32::from_rgb(28, 92, 168), "하도(18.5m)"),
                (Color32::from_rgb(76, 138, 68), "충적평야(25m)"),
                (Color32::from_rgb(112, 134, 66), "구릉·선상지(45m)"),
                (Color32::from_rgb(138, 118, 96), "산지능선(95m)"),
                (Color32::from_rgb(178, 162, 144), "정상부(142.0m)"),
                (Color32::from_rgb(217, 42, 26), "조사구역(AOI)"),
            ];
            let mut cur_x = leg_x;
            for (col, label) in elev_swatches {
                if cur_x + 115.0 > footer_rect.right() - 60.0 {
                    break;
                }
                let sw = Rect::from_center_size(Pos2::new(cur_x + 6.0, leg_y), Vec2::new(11.0, 11.0));
                painter.rect_filled(sw, 2.0, col);
                painter.text(
                    Pos2::new(cur_x + 15.0, leg_y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(11.0),
                    Color32::from_rgb(30, 41, 59),
                );
                cur_x += 108.0;
            }

            // 우측 하단 회전 방위표 (Terrain3dService.cpp:530-543)
            let nx = footer_rect.right() - 34.0;
            let ny = footer_rect.center().y - 2.0;
            let rot = -self.paleo_3d.camera_3d.yaw_deg.to_radians();
            let dir_n = Vec2::new(rot.sin(), -rot.cos());
            let dir_perp = Vec2::new(-dir_n.y, dir_n.x);
            let center_n = Pos2::new(nx, ny);
            let tip = center_n + dir_n * 11.0;
            let tail = center_n - dir_n * 8.0;
            painter.line_segment(
                [tail, tip],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    center_n + dir_n * 3.0 - dir_perp * 4.0,
                    center_n + dir_n * 3.0 + dir_perp * 4.0,
                ],
                Color32::from_rgb(181, 87, 58),
                Stroke::NONE,
            ));
            painter.text(
                Pos2::new(nx + 14.0, ny),
                egui::Align2::LEFT_CENTER,
                "N",
                FontId::proportional(11.5),
                Color32::from_rgb(17, 24, 39),
            );
        });
    }
}
