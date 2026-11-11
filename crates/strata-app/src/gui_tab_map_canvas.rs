//! 지도 캔버스 한 프레임 — 입력 처리, 배경, 캔버스 그리기, HUD 구간 호출.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

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
            self.home_map
                .viewport
                .pan_by_pixels(f64::from(d.x), f64::from(d.y));
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
                        // 유물위치표시: 찍은 점을 유물 레이어에 넣는다(C++ `MainWindowEditing.cpp:1168`).
                        if effective_tool == MapToolKind::PlacePoint {
                            let _ = self.commit_drawn_shape(effective_tool, vec![wxy]);
                        }
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
                        // 조사 파일의 유구·단면선·기준점·유물을 먼저 찾는다(위에 그려지는 것부터).
                        let doc_hit = self.pick_doc_feature(wxy);
                        if doc_hit.is_none() {
                            self.clear_doc_selection();
                        }
                        if let Some((key, idx)) = doc_hit {
                            self.select_doc_feature(key, idx);
                        } else if let Some((idx, tr)) = self
                            .home_map
                            .real_survey
                            .trenches
                            .iter()
                            .enumerate()
                            .find(|(_, t)| {
                                let ([min_x, min_y], [max_x, max_y]) = crate::survey_selection::ring_bbox(&t.ring);
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
                                let ([min_x, min_y], [max_x, max_y]) = crate::survey_selection::ring_bbox(r);
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
                let _ = self
                    .home_map
                    .cad
                    .on_right_click(effective_tool, &mut self.home_map.scene);
                // C++ `onGeometryCaptured` — 마친 도형을 대상 레이어에 넣는다(면·선 모두).
                // 파일에는 「저장」을 눌러야 들어간다(C++ 과 같이 자동 저장 없음).
                let area_m2 = ring_area_m2(&ring);
                let is_polygon = effective_tool == MapToolKind::DrawPolygon;
                if matches!(effective_tool, MapToolKind::DrawPolygon | MapToolKind::DrawLine) {
                    if let Some(layer) = self.commit_drawn_shape(effective_tool, ring) {
                        self.home_map.draw_guide_message = Some(if is_polygon {
                            format!("「{layer}」에 도형을 그렸습니다 ({area_m2:.2} m²). 저장하려면 Ctrl+S.")
                        } else {
                            format!("「{layer}」에 선을 그렸습니다. 저장하려면 Ctrl+S.")
                        });
                    }
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
    }
}
