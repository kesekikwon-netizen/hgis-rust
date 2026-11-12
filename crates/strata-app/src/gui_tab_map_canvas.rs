//! 지도 캔버스 한 프레임 — 입력 처리, 배경, 캔버스 그리기, HUD 구간 호출.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;

use super::gui_tab_map_hud::MapToolFlags;

use super::gui_tab_map_geom::s2w;

/// `render_map_tab_ui` 가 넘기는 도구 상태 묶음.
pub(crate) struct MapFrameInput {
    pub(crate) is_trench: bool,
    pub(crate) is_measure: bool,
    pub(crate) is_draw_tool: bool,
    pub(crate) is_select_tool: bool,
    pub(crate) is_style: bool,
}

impl StrataAppState {
    /// 캔버스 한 프레임 (원문 CentralPanel 클로저 본문).
    pub(crate) fn paint_map_canvas(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tab_clip: Rect,
        input: MapFrameInput,
    ) {
        let MapFrameInput {
            is_trench,
            is_measure,
            is_draw_tool,
            is_select_tool,
            is_style,
        } = input;
        if is_trench {
            let trench_cnt = self.home_map.real_survey.trenches.len();
            let trench_area: f64 = self
                .home_map
                .real_survey
                .trenches
                .iter()
                .map(|t| t.area_m2)
                .sum();
            let survey_area: f64 = self.home_map.real_survey.survey_area_m2.max(1.0);
            let ratio_pct = (trench_area / survey_area * 100.0).clamp(0.0, 100.0);
            let shape_lbl = if self.modals.trench_shape.arm_bend().is_some() {
                self.modals.trench_shape.label_ko()
            } else {
                self.modals.trench_length.label_ko()
            };
            egui::Frame::NONE
                .fill(Color32::from_rgb(225, 240, 218))
                .stroke(Stroke::new(1.0, Color32::from_rgb(145, 195, 138)))
                .inner_margin(egui::Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let _ = ui.small_button("II");
                        ui.colored_label(Color32::from_rgb(22, 138, 42), "✔");
                        ui.label(
                            egui::RichText::new(format!(
                                "시굴격자: {shape_lbl} 시굴격자 {trench_cnt}개 · 총 {trench_area:.2}m² · 조사구역의 {ratio_pct:.1}% (시굴 기준 10%)"
                            ))
                            .size(12.0)
                            .strong(),
                        );
                    });
                });
        }

        let avail_sz = Vec2::new(
            ui.available_width(),
            (tab_clip.bottom() - ui.cursor().top()).max(120.0),
        );
        let (rect, response) =
            ui.allocate_exact_size(avail_sz, egui::Sense::click_and_drag());
        self.home_map.viewport.screen_size_px =
            [f64::from(rect.width()), f64::from(rect.height())];

        // C++ 은 그리기 줄을 열어도 도구가 이동 그대로다 — 그릴 대상을 눌러야 그리기 도구가 켜진다.
        let effective_tool = self.active_tool;
        let capture = self.handle_polygon_capture_input(ui, &response, rect);

        if let Some(hp) = response.hover_pos() {
            self.home_map.cursor_world_xy = s2w(rect, &self.home_map.viewport, hp);
            self.home_map.cursor_seen = true;
            if matches!(
                effective_tool,
                MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
            ) {
                self.home_map.update_cursor_snap_preview();
            } else {
                self.home_map.cad.preview_markers_world.clear();
            }
        }
        if response.hovered() {
            let sy = ui.input(|i| i.smooth_scroll_delta.y);
            let anchor = [
                f64::from(rect.width()) * 0.5,
                f64::from(rect.height()) * 0.5,
            ];
            if sy.abs() > 0.1 {
                self.home_map
                    .viewport
                    .zoom_at(if sy > 0.0 { 0.85 } else { 1.18 }, anchor);
            }
        }
        if response.dragged() && !(capture && response.dragged_by(egui::PointerButton::Primary)) {
            let d = response.drag_delta();
            self.home_map
                .viewport
                .pan_by_pixels(f64::from(d.x), f64::from(d.y));
        }
        if !capture && response.clicked() {
            if let Some(cp) = response.interact_pointer_pos() {
                let wxy = s2w(rect, &self.home_map.viewport, cp);
                match effective_tool {
                    MapToolKind::DrawPolygon
                    | MapToolKind::DrawLine
                    | MapToolKind::PlacePoint
                    | MapToolKind::Measure => {
                        let cur_layer = self.right_inspector.selected_layer_key.clone();
                        self.home_map.on_canvas_left_click_on_layer(
                            wxy,
                            effective_tool,
                            cur_layer.as_deref(),
                        );
                    }
                    MapToolKind::TrenchPlace => {
                        let len_m = self.modals.trench_length.meters();
                        let ring_5 = strata_geo::trench_grid::create_trench_rectangle_polygon(
                            wxy,
                            2.0,
                            len_m,
                            self.modals.trench_azimuth_deg,
                        );
                        let next_no = self.home_map.real_survey.trenches.len() + 1;
                        let name = format!("Tr-{next_no}");
                        let area_m2 = 2.0 * len_m;
                        self.home_map
                            .real_survey
                            .trenches
                            .push(crate::real_survey::RealTrenchPoly {
                                name: name.clone(),
                                ring: ring_5.0,
                                area_m2,
                                center: wxy,
                            });
                        self.is_dirty = true;
                        self.home_map.draw_guide_message = Some(format!(
                            "시굴격자 {name} ({area_m2:.1} m²)을(를) 배치했습니다 — 화살표 키(Shift: 1 m)로 미세 이동할 수 있습니다."
                        ));
                    }
                    MapToolKind::Select | MapToolKind::Pan => {
                        let mpp = self.home_map.viewport.meters_per_pixel;
                        if let Some((idx, tr)) = self
                            .home_map
                            .real_survey
                            .trenches
                            .iter()
                            .enumerate()
                            .find(|(_, t)| {
                                let min_x = t
                                    .ring
                                    .iter()
                                    .map(|p| p[0])
                                    .fold(f64::INFINITY, f64::min);
                                let max_x = t
                                    .ring
                                    .iter()
                                    .map(|p| p[0])
                                    .fold(f64::NEG_INFINITY, f64::max);
                                let min_y = t
                                    .ring
                                    .iter()
                                    .map(|p| p[1])
                                    .fold(f64::INFINITY, f64::min);
                                let max_y = t
                                    .ring
                                    .iter()
                                    .map(|p| p[1])
                                    .fold(f64::NEG_INFINITY, f64::max);
                                strata_canvas2d::canvas_painter::perform_canvas_hit_test_10px(
                                    Some(([min_x, min_y], [max_x, max_y], 1)),
                                    None,
                                    wxy,
                                    mpp,
                                    10.0,
                                )
                                .has_hit
                            })
                        {
                            let tr_name = tr.name.clone();
                            let tr_area = tr.area_m2;
                            self.home_map.selected_feature_idx = Some(0);
                            self.right_inspector.selected_layer_key =
                                Some("시굴격자".to_owned());
                            self.home_map.draw_guide_message = Some(format!(
                                "도형 선택: 시굴격자 #{idx} ({tr_name}, {tr_area:.1} m²) — 화살표 키(0.1 m) 또는 Shift+화살표(1.0 m)로 이동"
                            ));
                        } else if let Some((r_idx, _)) = self
                            .home_map
                            .real_survey
                            .survey_rings
                            .iter()
                            .enumerate()
                            .find(|(_, r)| {
                                let min_x = r.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                                let max_x = r.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                                let min_y = r.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                                let max_y = r.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
                                wxy[0] >= min_x && wxy[0] <= max_x && wxy[1] >= min_y && wxy[1] <= max_y
                            })
                        {
                            self.home_map.selected_feature_idx = Some(r_idx);
                            self.right_inspector.selected_layer_key =
                                Some("조사구역".to_owned());
                        } else if effective_tool == MapToolKind::Select {
                            self.home_map.selected_feature_idx = None;
                            self.home_map.draw_guide_message = Some(format!(
                                "클릭 좌표 (X {:.2}, Y {:.2}) 주변 10px 이내에 선택된 도형이 없습니다.",
                                wxy[0], wxy[1]
                            ));
                        } else {
                            self.home_map.selected_feature_idx = None;
                        }
                    }
                    _ => {}
                }
            }
        }
        if !capture && response.secondary_clicked() {
            // MainWindowContextMenus.cpp:163-166 — 그리기·측거 도구일 때는 메뉴 없이
            // 도구 쪽 우클릭(마침)을 그대로 쓴다.
            let drawing_tool = matches!(
                effective_tool,
                MapToolKind::DrawPolygon
                    | MapToolKind::DrawLine
                    | MapToolKind::Measure
                    | MapToolKind::PlacePoint
            );
            if drawing_tool {
                // 면 그리기는 위에서 처리한다. 선·점·줄자는 아직 예전 도구다(지시 3-4·3-5).
                let _ = self.home_map.cad.on_right_click(effective_tool, &mut self.home_map.scene);
            } else if let Some(pos) = response.interact_pointer_pos() {
                // MainWindowContextMenus.cpp:112-161 mapContextMenu
                let wxy = s2w(rect, &self.home_map.viewport, pos);
                self.map_context_menu_open = true;
                self.map_context_menu_pos = [pos.x, pos.y];
                self.map_context_menu_world = wxy;
            }
        }

        let rect = rect.intersect(tab_clip);
        let p = ui.painter_at(rect);
        // MainWindow.cpp:512 setCanvasColor(canvasNeutral) — KaTheme.cpp:75 흰색. 지도는 그림이라 테마·
        // 레이어 밝기와 상관없이 언제나 흰 바탕이다(KaThemeColors.cpp:152).
        p.rect_filled(rect, 0.0, Color32::WHITE);

        // 배경지도 타일(위성·일반도). C++ 은 QGIS 가 그리지만 Rust 는 직접 받아 칠한다.
        self.paint_basemap_layer(ctx, &p, rect);
        self.paint_cadastral_layer(ctx, &p, rect);

        self.paint_gyeongju_heritage_canvas(ctx, &p, rect, is_trench || is_measure || is_style);
        self.paint_map_hud(
            ctx,
            ui,
            &p,
            rect,
            MapToolFlags {
                is_trench,
                is_measure,
                is_draw_tool,
                is_select_tool,
                is_style,
                effective_tool,
            },
        );
        self.paint_map_overlays(ctx, ui, &p, rect);
        self.paint_polygon_sketch(&p, rect, response.hover_pos());
    }
}
