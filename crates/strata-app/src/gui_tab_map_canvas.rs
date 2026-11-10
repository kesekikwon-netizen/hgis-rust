//! 지도 캔버스 한 프레임 — 입력 처리, 배경, 캔버스 그리기, HUD 구간 호출.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::real_survey::ring_area_m2;

use super::gui_tab_map_hud::MapToolFlags;

use super::gui_tab_map_format::modulate_color_with_opacity_brightness;
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

        let effective_tool = if is_draw_tool && self.active_tool == MapToolKind::Pan {
            MapToolKind::DrawPolygon
        } else {
            self.active_tool
        };

        if let Some(hp) = response.hover_pos() {
            self.home_map.cursor_world_xy = s2w(rect, &self.home_map.viewport, hp);
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
        if response.dragged() {
            let d = response.drag_delta();
            // 꼭짓점 편집 중에 핸들을 잡고 있으면 **핸들을 끈다**(지도를 옮기지 않는다).
            // C++ `KaVertexEditTool.cpp:334 moveVertexTo`.
            if self.snap_drag_vertex(ctx, rect, response.interact_pointer_pos()) {
                // 핸들을 끄는 중 — 화면 이동 없음
            } else {
                self.home_map
                    .viewport
                    .pan_by_pixels(f64::from(d.x), f64::from(d.y));
            }
        }
        // 손을 떼면 잡고 있던 핸들을 놓는다.
        if response.drag_stopped() {
            self.snap_release_vertex();
        }
        // Del 키 — 편집 중인 정점을 지운다. C++ `KaVertexEditTool.cpp:357 deleteVertexAt`.
        if self.home_map.vertex_edit.is_some()
            && ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
        {
            self.snap_delete_selected_vertex();
        }
        if response.clicked() {
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
        if response.secondary_clicked() {
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
                let ring = self.home_map.scene.active_rubberband.clone();
                if self
                    .home_map
                    .cad
                    .on_right_click(effective_tool, &mut self.home_map.scene)
                    && ring.len() >= 3
                {
                    let area_m2 = ring_area_m2(&ring);
                    self.home_map.real_survey.save_drawn_polygon(ring);
                    self.is_dirty = true;
                    self.home_map.draw_guide_message =
                        Some(format!("조사구역·유구 다각형을 저장했습니다 ({area_m2:.2} m²)."));
                }
            } else if let Some(pos) = response.interact_pointer_pos() {
                // MainWindowContextMenus.cpp:112-161 mapContextMenu
                let wxy = self.home_map.viewport.screen_to_world([pos.x as f64, pos.y as f64]);
                self.map_context_menu_open = true;
                self.map_context_menu_pos = [pos.x, pos.y];
                self.map_context_menu_world = wxy;
            }
        }

        let rect = rect.intersect(tab_clip);
        // 검증 제어 통로가 화면 좌표 → 지도 좌표 변환에 쓴다(--snap 의 @draw 등).
        self.last_canvas_rect = Some(rect);
        let p = ui.painter_at(rect);
        let bg_col = modulate_color_with_opacity_brightness(
            Color32::from_rgb(251, 250, 246),
            1.0,
            self.home_map.real_survey.layer_brightness,
        );
        p.rect_filled(rect, 0.0, bg_col);

        // 배경지도 타일(위성·일반도). C++ 은 QGIS 가 그리지만 Rust 는 직접 받아 칠한다.
        self.paint_basemap_layer(ctx, &p, rect);
        self.paint_cadastral_layer(ctx, &p, rect);

        // 지형 음영 — 실제 DEM 이 있을 때만. 조사 데이터 **아래**에 깐다.
        self.paint_terrain_backdrop(ctx, &p, rect, &self.home_map.viewport);

        // 조사 데이터는 **레이어 엔진**이 그린다.
        // C++ 은 QGIS 레이어가 그리지만 Rust 는 layer_engine 이 도형 목록을 낸다.
        // 이렇게 해야 레이어를 끄면 진짜로 꺼지고, z순서가 데이터로 정해진다.
        let stage = crate::map_stage_bridge::stage_from_survey(&self.home_map.real_survey);
        let xf = crate::map_engine_painter::screen_transform_for(&self.home_map.viewport, rect);
        let draws = strata_canvas2d::layer_engine::render_stage(
            &stage,
            &xf,
            strata_canvas2d::layer_engine::RenderOptions {
                labels: true,
                ..Default::default()
            },
        );
        let _painted = crate::map_engine_painter::paint_layer_draws(&p, &draws);

        // 겹침은 데이터 **위**에 얹는다 — 좌표격자·수치지형도·기준점·스냅·러버밴드.
        self.paint_scene_overlays(super::gui_tab_map_scene_overlay::SceneOverlayCtx {
            ctx,
            p: &p,
            rect,
            vp: &self.home_map.viewport,
            center: self.home_map.real_survey.survey_center,
        });

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
    }

    /// 꼭짓점 편집 중 핸들 끌기를 처리한다. 핸들을 잡았으면 `true`.
    ///
    /// C++ `KaVertexEditTool` 대응:
    /// - `:243 vertexNear` / `:262 segmentNear` — 핸들 찾기
    /// - `:334 moveVertexTo` — 정점 이동
    /// - `:381 insertVertexAt` — 선분 중점을 끌면 새 정점
    /// - `:225-241 snapMapPointExcludingTarget` — **자기 자신에게 붙지 않게** 제외
    fn snap_drag_vertex(
        &mut self,
        _ctx: &egui::Context,
        rect: Rect,
        pointer: Option<Pos2>,
    ) -> bool {
        use strata_canvas2d::vertex_handle::{hit_handle, HandleKind, VertexEdit};

        let Some(pos) = pointer else {
            return false;
        };
        let Some(ve) = self.home_map.vertex_edit.as_ref().cloned() else {
            return false;
        };
        let idx = ve.feature_index;
        if idx >= self.home_map.real_survey.survey_rings.len() {
            return false;
        }
        let xf = crate::map_engine_painter::screen_transform_for(&self.home_map.viewport, rect);

        // 아직 잡은 핸들이 없으면 이번 프레임에 잡는다.
        if ve.dragging.is_none() {
            let ring = &self.home_map.real_survey.survey_rings[idx];
            let Some(kind) = hit_handle(ring, [pos.x, pos.y], &xf, 8.0) else {
                return false;
            };
            if let Some(v) = self.home_map.vertex_edit.as_mut() {
                v.dragging = Some(kind);
            }
            return true;
        }

        let kind = ve.dragging;
        let mut target = xf.screen_to_world([pos.x, pos.y]);

        // 스냅: 다른 정점·선분에 붙인다. **자기 자신은 제외**한다
        // (제외하지 않으면 끄는 점이 제자리에 붙어 안 움직인다).
        if self.home_map.cad.snap_enabled {
            let stage = crate::map_stage_bridge::stage_from_survey(&self.home_map.real_survey);
            let settings = strata_canvas2d::snap_locator::SnapSettings {
                enabled: true,
                tolerance_px: 10.0,
                vertex: true,
                endpoint: true,
                edge: true,
            };
            let layer_index = stage
                .all()
                .iter()
                .position(|l| l.key == crate::map_stage_bridge::KEY_SURVEY_AREA);
            if let Some(hit) = strata_canvas2d::snap_locator::locate(
                target,
                self.home_map.viewport.meters_per_pixel,
                &stage,
                &settings,
                layer_index.map(|li| (li, idx)),
            ) {
                target = hit.world;
            }
        }

        let edit = match kind {
            Some(HandleKind::Vertex(i)) => Some(VertexEdit::Move { index: i, to: target }),
            Some(HandleKind::Midpoint(s)) => Some(VertexEdit::Insert {
                segment: s,
                at: target,
            }),
            None => None,
        };
        if let Some(edit) = edit {
            let ring = &mut self.home_map.real_survey.survey_rings[idx];
            if ve.apply_to(ring, &edit).is_ok() {
                self.is_dirty = true;
            }
        }
        true
    }

    /// 꼭짓점 편집을 끝낸다(놓았을 때).
    fn snap_release_vertex(&mut self) {
        if let Some(v) = self.home_map.vertex_edit.as_mut() {
            v.dragging = None;
        }
    }

    /// Del 키 — 지금 잡고 있던(또는 마지막에 옮긴) 정점을 지운다.
    ///
    /// C++ 은 클릭으로 고른 정점을 Del 로 지운다(`KaVertexEditTool.cpp:357`).
    /// 여기서는 "잡고 있던 정점"을 지운다. 최소 점 수에 걸리면 안내만 남긴다.
    fn snap_delete_selected_vertex(&mut self) {
        use strata_canvas2d::vertex_handle::{HandleKind, VertexEdit};
        let Some(ve) = self.home_map.vertex_edit.as_ref().cloned() else {
            return;
        };
        let Some(HandleKind::Vertex(i)) = ve.dragging else {
            self.home_map.draw_guide_message =
                Some("지울 점을 먼저 클릭한 뒤 Del 을 누르세요.".to_owned());
            return;
        };
        let idx = ve.feature_index;
        let Some(ring) = self.home_map.real_survey.survey_rings.get_mut(idx) else {
            return;
        };
        if ve.apply_to(ring, &VertexEdit::Delete { index: i }).is_ok() {
            self.is_dirty = true;
            if let Some(v) = self.home_map.vertex_edit.as_mut() {
                v.dragging = None;
            }
        } else {
            self.home_map.draw_guide_message =
                Some("점을 더 지울 수 없습니다 — 면은 최소 3점이 필요합니다.".to_owned());
        }
    }
}
