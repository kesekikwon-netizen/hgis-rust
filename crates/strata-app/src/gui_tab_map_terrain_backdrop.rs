//! 지형 음영·지표모델(DSM) 배경 — 조사 데이터 **아래**에 깔린다.
//!
//! C++ `LayerOps::addElevationHillshadeMap` 은 실제 GeoTIFF 래스터가 있을 때만
//! 지형 레이어를 만든다. 여기서도 같게 한다: `dem_grid` 가 없으면 아무 것도 안 그린다.
//!
//! 예전에는 데이터가 없어도 `survey_center` 기준 사각형에 합성 등고선을 칠해서
//! 새 조사 화면 한가운데에 초록 사각형이 떴다(사용자 지적 2026-11-09).
//! 그 규칙을 지킨다.
//!
//! 레이어 엔진으로 갈아탈 때 이 배경이 사라졌으므로 따로 되살린다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::viewport::ViewportTransform;

use crate::app_state::StrataAppState;

use super::gui_tab_map_format::modulate_color_with_opacity_brightness;
use super::gui_tab_map_geom::w2s;

/// 불투명도·밝기를 입힌 색.
fn layer_color(
    ctx: &egui::Context,
    rs: &crate::real_survey::RealSurveyData,
    sel_layer: Option<&str>,
    name: &str,
    base: Color32,
) -> Color32 {
    let (op, br) = if sel_layer == Some(name) {
        (rs.layer_opacity, rs.layer_brightness)
    } else {
        ctx.data_mut(|d| {
            d.get_temp::<(f32, f32)>(egui::Id::new(("ka_layer_op_br", name)))
                .unwrap_or((rs.layer_opacity, rs.layer_brightness))
        })
    };
    modulate_color_with_opacity_brightness(base, op, br)
}

impl StrataAppState {
    /// 지형 음영 배경. 실제 DEM 이 없으면 아무 것도 그리지 않는다.
    ///
    /// 레이어 엔진보다 **먼저** 부른다(배경이 데이터를 덮지 않도록).
    pub(crate) fn paint_terrain_backdrop(
        &self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: Rect,
        vp: &ViewportTransform,
    ) {
        let rs = &self.home_map.real_survey;
        let [cx, cy] = rs.survey_center;

        // 실제 DEM 이 있을 때만 그린다 — 없으면 지형 음영 자체가 없다.
        if self.home_map.dem_grid.is_none() {
            return;
        }
        if !(rs.is_layer_visible("지형 음영")
            || rs.is_layer_visible("지표모델(DSM) 30m · Coperni..."))
        {
            return;
        }

        let sel = self.right_inspector.selected_layer_key.as_deref();
        let color = |name: &str, base: Color32| layer_color(ctx, rs, sel, name, base);

        let tl = w2s(rect, vp, [cx - 1050.0, cy + 720.0]);
        let br = w2s(rect, vp, [cx + 1050.0, cy - 720.0]);
        let dsm_r = Rect::from_min_max(tl, br);

        match self.modals.dem_applied_mode_idx {
            0 => {
                // 등고 모드: 연한 지형 배경 + 등고선 띠.
                let base = color("지형 음영", Color32::from_rgb(228, 224, 208));
                p.rect_filled(dsm_r, 0.0, base);
                let contour_col = color("지형 음영", Color32::from_rgb(138, 102, 62));
                for idx in 1..=7 {
                    let ratio = (idx as f32) / 8.0;
                    let r_x = dsm_r.width() * 0.46 * ratio;
                    let r_y = dsm_r.height() * 0.44 * ratio;
                    let mut ring = Vec::with_capacity(25);
                    for step in 0..=24 {
                        let ang = (step as f32) / 24.0 * std::f32::consts::TAU;
                        ring.push(Pos2::new(
                            dsm_r.center().x + ang.cos() * r_x,
                            dsm_r.center().y + ang.sin() * r_y,
                        ));
                    }
                    p.add(egui::Shape::line(
                        ring,
                        Stroke::new(if idx % 2 == 0 { 1.4 } else { 0.8 }, contour_col),
                    ));
                }
            }
            2 => {
                // 고도 모드: 색 표현 프리셋별 단계 색 띠.
                let bands = 10usize;
                let bh = dsm_r.height() / (bands as f32);
                for bi in 0..bands {
                    let t = (bi as f32) / ((bands - 1) as f32);
                    let raw_col = match self.modals.dem_applied_preset_idx {
                        0 => Color32::from_rgb(
                            (90.0 + t * 110.0) as u8,
                            (175.0 - t * 35.0) as u8,
                            (95.0 + t * 60.0) as u8,
                        ),
                        1 => Color32::from_rgb(
                            (58.0 + t * 145.0) as u8,
                            (152.0 - t * 48.0) as u8,
                            (118.0 - t * 55.0) as u8,
                        ),
                        _ => Color32::from_rgb(
                            (68.0 + t * 165.0) as u8,
                            (168.0 - (t - 0.4).abs() * 120.0).clamp(65.0, 210.0) as u8,
                            (74.0 - t * 32.0) as u8,
                        ),
                    };
                    let seg_r = Rect::from_min_size(
                        Pos2::new(dsm_r.left(), dsm_r.top() + (bi as f32) * bh),
                        Vec2::new(dsm_r.width(), bh + 1.0),
                    );
                    p.rect_filled(seg_r, 0.0, color("지형 음영", raw_col));
                }
            }
            _ => {
                // 기본 음영 모드.
                let dsm_col = color("지형 음영", Color32::from_rgb(154, 198, 120));
                p.rect_filled(dsm_r, 0.0, dsm_col);
            }
        }
    }
}
