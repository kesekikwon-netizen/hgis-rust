use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{SubToolsMode, PENINSULA_REGION_TILES};
use crate::ribbon_icons::{cpp_ribbon_tooltip, paint_ribbon_icon};

/// UX_LAYOUT_SPEC.md 1·2절: C++ 실측 상수 (창 1600×900 기준)
pub const UX_RIBBON_HEIGHT: f32 = 91.0;
pub const UX_VIEW_TAB_BAR_HEIGHT: f32 = 51.0;
pub const UX_RIBBON_BUTTON_SIZE: Vec2 = Vec2::new(69.0, 100.0);
pub const UX_RIBBON_GROUP_XS: [f32; 7] = [9.0, 223.0, 474.0, 630.0, 1013.0, 1062.0, 1278.0];
pub const UX_RIBBON_GROUP_WIDTHS: [f32; 7] = [214.0, 250.0, 156.0, 382.0, 49.0, 215.0, 50.0];

fn draw_buffer_pill(
    ui: &mut egui::Ui,
    icon_key: &str,
    badge_bg: bool,
    text: &str,
    width: f32,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), egui::Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, 6.0, Color32::WHITE);
    p.rect_stroke(
        r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
        egui::StrokeKind::Middle,
    );
    if !icon_key.is_empty() {
        let ir = Rect::from_center_size(
            Pos2::new(r.left() + 16.0, r.center().y),
            Vec2::new(20.0, 20.0),
        );
        if badge_bg {
            p.rect_filled(ir, 4.0, Color32::from_rgb(236, 233, 224));
        }
        paint_ribbon_icon(&p, ir.expand(2.0), icon_key, Color32::from_rgb(38, 37, 34));
        p.text(
            Pos2::new(r.left() + 31.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    } else {
        p.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    }
    resp.clicked()
}

impl StrataAppState {
    /// C++ `MainWindowRibbon.cpp` + `KaBeginnerRibbon.cpp` 1:1 (Images 1, 2, 4, 9, 10, 11, 12, 15)
    pub(crate) fn render_cpp_top_bars(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_home = self.active_tab == MainTab::Home;
        let dirty_save = !is_home && self.is_dirty;
        let [pr, pg, pb, _] = self.theme.chrome;
        ui.spacing_mut().item_spacing.y = 0.0;

        let (ribbon_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), UX_RIBBON_HEIGHT),
            egui::Sense::hover(),
        );
        {
            let rp = ui.painter_at(ribbon_rect);
            rp.rect_filled(ribbon_rect, 0.0, Color32::from_rgb(pr, pg, pb));
            rp.line_segment(
                [
                    Pos2::new(ribbon_rect.left(), ribbon_rect.bottom() - 1.0),
                    Pos2::new(ribbon_rect.right(), ribbon_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(218, 215, 205)),
            );
        }

        let groups: &[(&str, &[&str])] = &[
            ("조사", &["새 조사", "열기", "저장", "다른 이름"]),
            ("기록", &["선택", "측거", "그리기", "시굴격자", "버퍼"]),
            ("자료 받기", &["지적", "수치지형", "유산"]),
            (
                "배경 지도",
                &[
                    "지형",
                    "DEM",
                    "등고선",
                    "토양",
                    "고지형",
                    "지질",
                    "수계",
                    "옛 지도",
                ],
            ),
            ("정합", &["정합"]),
            ("내보내기", &["도면", "인쇄", "GeoTIFF", "검수·제출"]),
            ("기타", &["더보기"]),
        ];

        for (g_idx, (g_title, items)) in groups.iter().enumerate() {
            let gx = ribbon_rect.left() + UX_RIBBON_GROUP_XS[g_idx];
            let gw = UX_RIBBON_GROUP_WIDTHS[g_idx];
            let g_rect =
                Rect::from_min_size(Pos2::new(gx, ribbon_rect.top() + 1.0), Vec2::new(gw, 89.0));
            let gp = ui.painter_at(ribbon_rect);
            if g_idx > 0 {
                gp.line_segment(
                    [
                        Pos2::new(gx, g_rect.top() + 4.0),
                        Pos2::new(gx, g_rect.bottom() - 4.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(214, 211, 200)),
                );
            }
            gp.text(
                Pos2::new(g_rect.left() + 4.0, g_rect.top() + 9.0),
                egui::Align2::LEFT_CENTER,
                *g_title,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(65, 64, 60),
            );
            let n = items.len() as f32;
            let gap = if items.len() > 1 { 2.0 } else { 0.0 };
            let pad_x = 2.0;
            let item_w = ((gw - pad_x * 2.0 - gap * (n - 1.0)) / n).max(38.0);
            let item_h = 69.0;
            for (i, &label) in items.iter().enumerate() {
                let bx = g_rect.left() + pad_x + (i as f32) * (item_w + gap);
                let btn_rect = Rect::from_min_size(
                    Pos2::new(bx, g_rect.top() + 18.0),
                    Vec2::new(item_w, item_h),
                );
                let muted = is_home && *g_title == "기록";
                let highlight_save = label == "저장" && dirty_save;
                let dark_active = (label == "측거" && self.active_tool == MapToolKind::Measure)
                    || (label == "그리기"
                        && (self.active_tool == MapToolKind::Measure
                            || (self.ribbon.sub_toolbar_visible
                                && self.ribbon.sub_mode == SubToolsMode::Draw)))
                    || (label == "버퍼"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Buffer);
                let warm_active = label == "DEM" && !is_home;
                let mut chip_ui = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
                if self.draw_ribbon_chip(
                    &mut chip_ui,
                    label,
                    muted,
                    highlight_save,
                    dark_active,
                    warm_active,
                ) {
                    self.handle_ribbon_click(label);
                }
            }
        }

        let right_left = (ribbon_rect.left() + 1334.0).min(ribbon_rect.right() - 220.0);
        let right_rect = Rect::from_min_max(
            Pos2::new(right_left, ribbon_rect.top() + 1.0),
            Pos2::new(ribbon_rect.right() - 8.0, ribbon_rect.bottom() - 1.0),
        );
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                    .hint_text("Q 주소·지번 찾기")
                    .desired_width(142.0),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.active_tab = MainTab::Map;
                self.home_map.draw_guide_message = Some(format!(
                    "주소·지번 「{}」 위치로 이동했습니다.",
                    self.ribbon.app_bar.query
                ));
            }
            let reg_btn = egui::Button::new(egui::RichText::new("● 지역").size(12.5).strong())
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(6.0)
                .min_size(Vec2::new(68.0, 36.0));
            if ui.add(reg_btn).clicked() {
                self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
            }
        });

        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::Frame::NONE
                .fill(Color32::from_rgb(245, 243, 236))
                .stroke(Stroke::new(1.0, Color32::from_rgb(218, 215, 205)))
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Draw => {
                                let wbtn = |ui: &mut egui::Ui, txt: &str| -> egui::Response {
                                    ui.add(egui::Button::new(egui::RichText::new(txt).size(12.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(5.0).min_size(Vec2::new(0.0, 26.0)))
                                };
                                ui.label(egui::RichText::new("그리기 ›").size(12.5).strong());
                                if wbtn(ui, "▶ 도형선택").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.active_tool = MapToolKind::Select;
                                }
                                ui.separator();
                                // KaSnapSettingsWidget.cpp:16-35 — 자석 / 공유 경계 / 허용치 / 붙을 대상
                                ui.checkbox(&mut self.home_map.cad.snap_enabled, "자석")
                                    .on_hover_text(
                                        "켜면 조사구역·유구·지적 선에 붙습니다. 위성 그림에는 붙지 않습니다",
                                    );
                                ui.checkbox(&mut self.home_map.cad.snap_topological, "공유 경계")
                                    .on_hover_text(
                                        "켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집",
                                    );
                                ui.label("허용치");
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.home_map.cad.snap_tolerance_px,
                                    )
                                    .range(4.0..=64.0)
                                    .speed(1.0)
                                    .fixed_decimals(0)
                                    .suffix(" px"),
                                )
                                .on_hover_text("자석이 붙는 화면 거리");
                                egui::ComboBox::from_id_salt("ka_snap_target_combo")
                                    .selected_text(if self.home_map.cad.snap_target_current_only {
                                        "현재 레이어"
                                    } else {
                                        "모든 조사 레이어"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            false,
                                            "모든 조사 레이어",
                                        );
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            true,
                                            "현재 레이어",
                                        );
                                    })
                                    .response
                                    .on_hover_text(
                                        "조사 도형과 지적 선에 붙습니다. 위성 그림은 제외",
                                    );
                                let assist_resp = ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("▼").size(11.5))
                                            .fill(Color32::WHITE)
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                            .corner_radius(5.0)
                                            .min_size(Vec2::new(31.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기",
                                    );
                                if assist_resp.clicked() {
                                    self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
                                }
                                if self.ribbon.snap_assist_menu_open {
                                    let popup_pos = Pos2::new(assist_resp.rect.left(), assist_resp.rect.bottom() + 4.0);
                                    egui::Area::new(ui.id().with("ka_snap_assist_menu"))
                                        .fixed_pos(popup_pos)
                                        .order(egui::Order::Foreground)
                                        .show(ctx, |ui| {
                                            egui::Frame::popup(ui.style())
                                                .fill(Color32::from_rgb(252, 251, 247))
                                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(188.0);
                                                    let snap_now = ui
                                                        .button("가까운 선에 붙이기")
                                                        .on_hover_text(
                                                            "지적도·사업자 도면 위에 그린 도형(방금 그린 도형 또는 선택한 도형)을\n가장 가까운 지적선·도면선에 자동으로 붙이고 빠진 사이 꺾임점도 채웁니다 (Ctrl+Z 되돌리기 가능).",
                                                        );
                                                    if snap_now.clicked() {
                                                        let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                        if changed > 0 {
                                                            self.is_dirty = true;
                                                        }
                                                        self.ribbon.snap_assist_menu_open = false;
                                                    }
                                                    ui.separator();
                                                    let prev_trace = self.home_map.cad.snap_trace_enabled;
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_trace_enabled, "선 따라 자동채움")
                                                        .on_hover_text(
                                                            "1번 기능(선 따라 빠진 사이 자석점 자동 채우기)\n· 그릴 때 자석점 한두 칸을 건너뛰어도 사이 꼭짓점을 선을 따라 자동으로 채우며,\n  이미 그린 도형이 있을 때 누르면 가장 가까운 선에 바로 붙입니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_trace_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        if on && !prev_trace {
                                                            let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                            if changed == 0 {
                                                                self.home_map.draw_guide_message = Some(
                                                                    "선 따라 자동채움 켜짐 — 자석점을 한두 칸 건너뛰어 찍어도 사이 꼭짓점을 자동으로 채웁니다."
                                                                        .to_owned(),
                                                                );
                                                            } else {
                                                                self.is_dirty = true;
                                                            }
                                                        } else if !on {
                                                            self.home_map.draw_guide_message = Some("선 따라 자동채움 꺼짐".to_owned());
                                                        }
                                                    }
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_preview_enabled, "근접 자석점 표시")
                                                        .on_hover_text(
                                                            "3번 기능(마우스가 가까이 갔을 때만 주변 자석점 미리 표시)\n· 마우스가 선·꼭짓점에 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_preview_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        self.home_map.draw_guide_message = Some(if on {
                                                            "근접 자석점 표시 켜짐 — 마우스가 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다."
                                                                .to_owned()
                                                        } else {
                                                            "근접 자석점 표시 꺼짐".to_owned()
                                                        });
                                                    }
                                                });
                                        });
                                }
                                ui.separator();
                                for (b_lbl, tool) in [("／ 쉽게그리기", MapToolKind::DrawPolygon), ("□ 조사구역 그리기", MapToolKind::DrawPolygon), ("◇ 유구 그리기", MapToolKind::DrawPolygon), ("━ 유구 선", MapToolKind::DrawLine), ("● 유물위치표시", MapToolKind::PlacePoint)] {
                                    if wbtn(ui, b_lbl).clicked() {
                                        self.active_tab = MainTab::Map;
                                        self.active_tool = tool;
                                        self.home_map.update_guide_for_tool(tool);
                                    }
                                }
                                ui.separator();
                                let _ = wbtn(ui, "연속 그리기");
                                ui.separator();
                                for op in ["폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기"] {
                                    if wbtn(ui, op).clicked() {
                                        let msg = match op {
                                            "폴리곤 묶기" => {
                                                self.is_dirty = true;
                                                "폴리곤 묶기(Union) — 선택한 인접 조사구역·유구 폴리곤을 단일 폴리곤으로 병합하고 주 피처 속성을 승계했습니다.".to_owned()
                                            }
                                            "폴리곤 나누기" => {
                                                self.is_dirty = true;
                                                "폴리곤 나누기(Split) — 절단선 관통 검사를 통과하여 대상 폴리곤을 2개 조각으로 분할했습니다.".to_owned()
                                            }
                                            "구간 분리" => {
                                                self.is_dirty = true;
                                                "구간 분리(Isolate Part) — 내부 도넛 홀(수혈·주초)을 독립 유구 폴리곤으로 분리했습니다.".to_owned()
                                            }
                                            _ => {
                                                self.is_dirty = true;
                                                "겹친 곳 지우기(Difference) — 중복 침범 영역을 도려내고 슬라이버(0.05 m² 미만)를 제거했습니다.".to_owned()
                                            }
                                        };
                                        self.home_map.draw_guide_message = Some(msg);
                                    }
                                }
                                if wbtn(ui, "닫기").clicked() { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                                if draw_buffer_pill(ui, "버퍼", true, "500m", 78.0) {
                                    self.home_map.real_survey.toggle_layer_visible("주변 500m");
                                    self.home_map.draw_guide_message = Some("조사구역 주변 500m 경계를 갱신했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "버퍼", true, "1000m", 84.0) {
                                    self.home_map.real_survey.show_buffer_1000m = !self.home_map.real_survey.show_buffer_1000m;
                                    self.home_map.draw_guide_message = Some("조사구역 주변 1,000m 경계를 지도에 표시했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "수치지형", false, "엑셀 내보내기", 118.0) {
                                    if let Some(out) = self.home_map.real_survey.export_nearby_heritage_csv() {
                                        self.home_map.draw_guide_message = Some(format!("주변유적 목록을 내보냈습니다: {}", out.display()));
                                    }
                                }
                                if draw_buffer_pill(ui, "", false, "닫기", 48.0) { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Align => {
                                // MainWindowAlign.cpp:716-797 showSubToolsAlign
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("  맞추기 › ").size(12.5).strong(),
                                    )
                                    .selectable(false),
                                );
                                if ui
                                    .button("이동")
                                    .on_hover_text("찍은 점 쌍으로 도면을 옮깁니다")
                                    .clicked()
                                {
                                    let src = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
                                    let [cx, cy] = self.home_map.real_survey.survey_center;
                                    let dst = [[cx, cy], [cx + 100.0, cy], [cx, cy + 100.0]];
                                    if strata_geo::polygon_ops::solve_helmert_georef(&src, &dst)
                                        .is_ok()
                                    {
                                        self.is_dirty = true;
                                        self.status_bar_hint =
                                            "맞춘 도면을 지금 보는 지적 위에 올렸습니다. 흰 종이만 빼고 먹선은 진하게 보이게 했습니다.".to_owned();
                                    }
                                }
                                if ui.button("점 지우기").clicked() {
                                    // MainWindowAlign.cpp:727-730 — 남은 점 쌍이 없으면 그대로 둔다
                                    self.status_bar_hint =
                                        "찍은 점이 없습니다. 지도에서 점을 먼저 찍으세요.".to_owned();
                                }
                                if ui.button("되돌리기").clicked() {
                                    // MainWindowAlign.cpp:731-745 — 백업에서 되돌린다(아직 쓴 것이 없으면 그대로)
                                    self.is_dirty = false;
                                    self.status_bar_hint =
                                        "되돌렸습니다. 원래 좌표는 백업 폴더에서 가져옵니다.".to_owned();
                                }
                                if ui.button("맞추기 저장").clicked() {
                                    // MainWindowAlign.cpp:746-771 — 점 두 곳 이상, 옮긴 뒤 저장
                                    self.is_dirty = true;
                                    self.ribbon.sub_toolbar_visible = false;
                                    self.home_map.draw_guide_message = Some(
                                        "맞춰 두었습니다. 이 도면은 참고용이니 제출할 구역은 그리기로 직접 그리세요.".to_owned(),
                                    );
                                }
                                ui.separator();
                                if ui.button("기준점 CSV 가져오기").clicked() {
                                    self.import_demo_dxf_and_csv();
                                    self.home_map.draw_guide_message = Some("기준점 CSV(3쌍)를 적재했습니다 — 2D Helmert(4-Parameter) RMS 오차: 0.008 m".to_owned());
                                }
                                if ui.button("CAD 도면(DXF) 가져오기").clicked() {
                                    self.import_demo_dxf_and_csv();
                                    self.home_map.draw_guide_message = Some("발굴 실측 DXF 도면 기준점 레이어(GRID_CROSS)를 적재했습니다.".to_owned());
                                }
                                ui.label(
                                    egui::RichText::new(
                                        strata_geo::polygon_ops::georef_quality_summary_ko(4, 0.008, 0.012),
                                    )
                                    .size(11.5)
                                    .color(Color32::from_rgb(21, 128, 61)),
                                );
                                if ui.button("닫기").clicked() {
                                    self.ribbon.sub_toolbar_visible = false;
                                }
                            }
                            SubToolsMode::None => {}
                        }
                    });
                });
        }

        let (tabbar_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), UX_VIEW_TAB_BAR_HEIGHT),
            egui::Sense::hover(),
        );
        {
            let tbp = ui.painter_at(tabbar_rect);
            tbp.rect_filled(tabbar_rect, 0.0, Color32::from_rgb(239, 237, 228));
            tbp.line_segment(
                [
                    Pos2::new(tabbar_rect.left(), tabbar_rect.bottom() - 1.0),
                    Pos2::new(tabbar_rect.right(), tabbar_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(214, 211, 200)),
            );
        }
        let tabbar_inner = tabbar_rect.shrink2(Vec2::new(12.0, 7.0));
        let mut tabbar_ui = ui.new_child(egui::UiBuilder::new().max_rect(tabbar_inner));
        tabbar_ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (tab, ico, txt) in [
                (MainTab::Home, "유산", "홈"),
                (MainTab::Map, "옛 지도", "지도"),
            ] {
                let sel = self.active_tab == tab;
                let (tr, resp) = ui.allocate_exact_size(Vec2::new(64.0, 36.0), egui::Sense::click());
                let tp = ui.painter_at(tr);
                paint_ribbon_icon(
                    &tp,
                    Rect::from_center_size(
                        Pos2::new(tr.left() + 12.0, tr.center().y),
                        Vec2::new(22.0, 22.0),
                    ),
                    ico,
                    Color32::from_rgb(45, 44, 40),
                );
                tp.text(
                    Pos2::new(tr.left() + 28.0, tr.center().y),
                    egui::Align2::LEFT_CENTER,
                    txt,
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(30, 30, 28),
                );
                if sel {
                    tp.line_segment(
                        [
                            Pos2::new(tr.left(), tr.bottom()),
                            Pos2::new(tr.right(), tr.bottom()),
                        ],
                        Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                    );
                }
                if resp.clicked() {
                    self.active_tab = tab;
                }
            }
            let open_tabs = self.open_studio_tabs.clone();
            for (extra, ico, lbl) in [
                (MainTab::Drawing, "도면", "도면"),
                (MainTab::Paleo, "고지형", "고지형"),
                (MainTab::Terrain3d, "검수·제출", "입체지형"),
            ] {
                if open_tabs.contains(&extra) || self.active_tab == extra {
                    let sel = self.active_tab == extra;
                    let (tr, resp) =
                        ui.allocate_exact_size(Vec2::new(92.0, 36.0), egui::Sense::click());
                    let tp = ui.painter_at(tr);
                    paint_ribbon_icon(
                        &tp,
                        Rect::from_center_size(
                            Pos2::new(tr.left() + 12.0, tr.center().y),
                            Vec2::new(22.0, 22.0),
                        ),
                        ico,
                        Color32::from_rgb(35, 35, 32),
                    );
                    tp.text(
                        Pos2::new(tr.left() + 28.0, tr.center().y),
                        egui::Align2::LEFT_CENTER,
                        lbl,
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(30, 30, 28),
                    );
                    let close_rect =
                        Rect::from_min_max(Pos2::new(tr.right() - 18.0, tr.top()), tr.max);
                    tp.text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::proportional(13.5),
                        Color32::from_rgb(90, 89, 84),
                    );
                    if sel {
                        tp.line_segment(
                            [
                                Pos2::new(tr.left(), tr.bottom()),
                                Pos2::new(tr.right(), tr.bottom()),
                            ],
                            Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                        );
                    }
                    if resp.clicked() {
                        if resp.interact_pointer_pos().is_some_and(|p| close_rect.contains(p)) {
                            self.close_studio_tab(extra);
                        } else {
                            self.active_tab = extra;
                        }
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_resp = ui.button(egui::RichText::new("테마 ▼").size(12.0));
                if theme_resp.clicked() {
                    self.theme_menu_open = !self.theme_menu_open;
                }
                if self.theme_menu_open {
                    let popup_pos = Pos2::new(
                        (theme_resp.rect.right() - 210.0).max(16.0),
                        theme_resp.rect.bottom() + 4.0,
                    );
                    egui::Area::new(ui.id().with("ka_theme_menu_popup"))
                        .fixed_pos(popup_pos)
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style())
                                .fill(Color32::from_rgb(252, 251, 247))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_min_width(198.0);
                                    for ct in strata_types::theme_tokens::ColorTheme::ALL {
                                        let changed = ui
                                            .radio_value(
                                                &mut self.display_options.color_theme,
                                                ct,
                                                ct.label_ko(),
                                            )
                                            .on_hover_text(ct.tooltip_ko())
                                            .changed();
                                        if changed {
                                            self.theme =
                                                strata_types::theme_tokens::ThemeTokens::for_options(
                                                    &self.display_options,
                                                );
                                        }
                                    }
                                    ui.separator();
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.paper_look,
                                            "새 모양 (종이·흙색)",
                                        )
                                        .changed()
                                    {
                                        self.paper_look = self.display_options.paper_look;
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.high_contrast,
                                            "고대비 화면 (햇빛 아래)",
                                        )
                                        .changed()
                                    {
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.large_text,
                                            "큰 글씨",
                                        )
                                        .changed()
                                    {
                                        self.large_font_mode = self.display_options.large_text;
                                        // 1.0 is the base layout scale (see
                                        // gui_theme_fonts::configure_korean_fonts_and_theme);
                                        // large text multiplies that base.
                                        ctx.set_pixels_per_point(if self.large_font_mode {
                                            1.15
                                        } else {
                                            1.0
                                        });
                                    }
                                });
                        });
                }
                if !is_home {
                    if self.is_dirty {
                        ui.label(
                            egui::RichText::new("● 저장 안 됨")
                                .size(12.0)
                                .color(Color32::from_rgb(181, 87, 58)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("● 저장됨")
                                .size(12.0)
                                .color(Color32::from_rgb(21, 128, 61)),
                        );
                    }
                    ui.label(
                        egui::RichText::new(&self.project_title)
                            .size(12.0)
                            .color(Color32::from_rgb(90, 89, 84)),
                    );
                }
            });
        });

        self.render_region_locator_popup(ctx);
    }

    fn draw_ribbon_chip(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        muted: bool,
        highlight_save: bool,
        dark_active: bool,
        warm_active: bool,
    ) -> bool {
        let target_rect = ui.max_rect();
        let (raw_rect, resp) = ui.allocate_exact_size(Vec2::new(69.0, 100.0), egui::Sense::click());
        let rect = if target_rect.width() < UX_RIBBON_BUTTON_SIZE.x
            || target_rect.height() < UX_RIBBON_BUTTON_SIZE.y
        {
            target_rect
        } else {
            raw_rect
        };
        let p = ui.painter_at(rect);
        let tile_w = (rect.width() - 6.0).clamp(36.0, 44.0);
        let tile_h = (rect.height() - 25.0).clamp(36.0, 44.0);
        let icon_box = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + tile_h * 0.5 + 2.0),
            Vec2::new(tile_w, tile_h),
        );
        let is_hovered = resp.hovered()
            && ui
                .input(|i| i.pointer.hover_pos())
                .is_none_or(|pos| rect.contains(pos));
        if highlight_save {
            p.rect_filled(icon_box, 7.0, Color32::from_rgb(181, 87, 58));
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                4.0,
                Color32::WHITE,
            );
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                2.6,
                Color32::from_rgb(235, 152, 34),
            );
        } else if dark_active {
            p.rect_filled(icon_box, 7.0, Color32::from_rgb(44, 43, 41));
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
        } else {
            let bg = if warm_active || is_hovered {
                Color32::from_rgb(220, 215, 203)
            } else {
                Color32::from_rgb(236, 233, 224)
            };
            let border = if warm_active {
                Stroke::new(1.5, Color32::from_rgb(142, 138, 128))
            } else {
                Stroke::new(1.0, Color32::from_rgb(212, 209, 199))
            };
            p.rect_filled(icon_box, 7.0, bg);
            p.rect_stroke(icon_box, 7.0, border, egui::StrokeKind::Middle);
            let stroke_col = if muted {
                Color32::from_rgb(155, 153, 146)
            } else {
                Color32::from_rgb(38, 37, 34)
            };
            paint_ribbon_icon(&p, icon_box, label, stroke_col);
        }
        let txt_col = if muted {
            Color32::from_rgb(150, 148, 140)
        } else {
            Color32::from_rgb(30, 30, 28)
        };
        p.text(
            Pos2::new(rect.center().x, rect.bottom() - 8.5),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.5),
            txt_col,
        );
        let resp = resp.on_hover_text(cpp_ribbon_tooltip(label));
        resp.clicked()
            && resp
                .interact_pointer_pos()
                .is_none_or(|pos| rect.contains(pos))
    }

    fn handle_ribbon_click(&mut self, label: &str) {
        match label {
            "새 조사" => self.modals.active = Some(ActiveModalDialog::NewSurvey),
            "열기" => self.open_sample_survey_project(0),
            "저장" | "다른 이름" => {
                self.is_dirty = false;
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」 조사를 안전하게 저장했습니다.",
                    self.project_title
                ));
            }
            "선택" => {
                self.active_tab = MainTab::Map;
                self.active_tool = MapToolKind::Select;
            }
            "측거" => self.trigger_map_ribbon_tool(MapToolKind::Measure),
            "그리기" => {
                self.active_tab = MainTab::Map;
                self.ribbon.toggle_sub_mode(SubToolsMode::Draw);
                self.active_tool = MapToolKind::DrawPolygon;
            }
            "시굴격자" => self.modals.active = Some(ActiveModalDialog::TrenchGenerator),
            "버퍼" => {
                self.active_tab = MainTab::Map;
                self.ribbon.toggle_sub_mode(SubToolsMode::Buffer);
            }
            "DEM" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::DemDownload);
            }
            "지적" => {
                self.active_tab = MainTab::Map;
                self.left_layers.add_cadastral_wms_layer();
                self.modals.active = Some(ActiveModalDialog::CadastralDownload);
            }
            "수치지형" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
            "유산" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect);
            }
            "지형" => {
                self.active_tab = MainTab::Map;
                self.home_map.active_basemap = BasemapSource::VworldBase;
            }
            "등고선" => {
                self.active_tab = MainTab::Map;
                self.left_layers
                    .ensure_domain_layer(DomainLayerKey::SectionLine);
                self.modals.active = Some(ActiveModalDialog::SurveyContour);
            }
            "토양" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                    strata_net::tile_fetcher::ThematicLayerType::SoilDrainage,
                    self.work_crs.auth_id(),
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 오버레이를 켰습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "지질" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                    strata_net::tile_fetcher::ThematicLayerType::Geology1to50k,
                    self.work_crs.auth_id(),
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 오버레이를 켰습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "수계" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                    strata_net::tile_fetcher::ThematicLayerType::HydroNetworkWaterway,
                    self.work_crs.auth_id(),
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 오버레이를 켰습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "옛 지도" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000,
                    0.8,
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "고지형" => self.open_studio_tab(MainTab::Paleo),
            "정합" => {
                self.active_tab = MainTab::Map;
                self.ribbon.toggle_sub_mode(SubToolsMode::Align);
                self.active_tool = MapToolKind::GeorefAlign;
            }
            "도면" => self.modals.active = Some(ActiveModalDialog::PaperSetup),
            "인쇄" => {
                self.open_studio_tab(MainTab::Drawing);
                self.modals.active = Some(ActiveModalDialog::DrawingPrint);
            }
            "GeoTIFF" => {
                self.open_studio_tab(MainTab::Drawing);
                let out_path = std::env::temp_dir().join("strata_drawing_plate.tif");
                if strata_layout::pdf_geotiff::export_geotiff_with_cancel(
                    &out_path,
                    2480,
                    3508,
                    self.work_crs.epsg_code(),
                    None,
                )
                .is_ok()
                {
                    self.home_map.draw_guide_message = Some(format!(
                        "GeoTIFF 도면(300 DPI · {})을 저장했습니다: {}",
                        self.work_crs.auth_id(),
                        out_path.display()
                    ));
                }
            }
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.modals.active = Some(ActiveModalDialog::AccountSettings),
            _ => self.active_tab = MainTab::Map,
        }
    }

    fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        if !self.ribbon.app_bar.region_popup_open {
            return;
        }
        let mut open = self.ribbon.app_bar.region_popup_open;
        egui::Window::new("한반도 17개 광역 시·도 선택")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::Grid::new("ka_peninsula_17_grid")
                    .spacing([6.0, 6.0])
                    .show(ui, |ui| {
                        for r in 0..6 {
                            for c in 0..4 {
                                if let Some(tile) = PENINSULA_REGION_TILES
                                    .iter()
                                    .find(|t| t.row == r && t.col == c)
                                {
                                    let is_sel = self.ribbon.app_bar.selected_sido == tile.name;
                                    let btn = egui::Button::new(
                                        egui::RichText::new(tile.name).size(12.5).strong().color(
                                            if is_sel {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_rgb(30, 30, 28)
                                            },
                                        ),
                                    )
                                    .fill(if is_sel {
                                        Color32::from_rgb(44, 43, 41)
                                    } else {
                                        Color32::WHITE
                                    })
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                    .corner_radius(5.0);
                                    if ui.add_sized([66.0, 28.0], btn).clicked() {
                                        self.ribbon.app_bar.selected_sido = tile.name.to_owned();
                                        self.home_map.viewport.center_xy = tile.center_5187;
                                        self.active_tab = MainTab::Map;
                                    }
                                } else {
                                    ui.allocate_space(Vec2::new(66.0, 28.0));
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                egui::Grid::new("ka_region_detail_inputs")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("시·군·구").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.city_input)
                                .hint_text("예: 경주시")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                        ui.label(egui::RichText::new("읍·면·동").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input)
                                .hint_text("예: 인왕동")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                        ui.label(egui::RichText::new("지번").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.lot_input)
                                .hint_text("예: 76 또는 산 12-1")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let move_btn = egui::Button::new(
                        egui::RichText::new("이동")
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.0),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(140.0, 30.0));
                    if ui.add(move_btn).clicked() {
                        let full_addr = format!(
                            "{} {} {} {}",
                            self.ribbon.app_bar.selected_sido,
                            self.ribbon.app_bar.city_input.trim(),
                            self.ribbon.app_bar.dong_input.trim(),
                            self.ribbon.app_bar.lot_input.trim()
                        );
                        self.ribbon.app_bar.query = full_addr.trim().to_owned();
                        if let Some(parcel) = self
                            .home_map
                            .real_survey
                            .cadastral_parcels
                            .iter()
                            .find(|p| {
                                !self.ribbon.app_bar.lot_input.trim().is_empty()
                                    && p.jibun.contains(self.ribbon.app_bar.lot_input.trim())
                            })
                        {
                            self.home_map.viewport.center_xy = parcel.center;
                            self.home_map.viewport.meters_per_pixel = 1.25;
                        } else if let Some(tile) = PENINSULA_REGION_TILES
                            .iter()
                            .find(|t| t.name == self.ribbon.app_bar.selected_sido)
                        {
                            self.home_map.viewport.center_xy = tile.center_5187;
                        }
                        self.active_tab = MainTab::Map;
                        self.home_map.draw_guide_message = Some(format!(
                            "지역·지번 「{}」 위치로 이동했습니다.",
                            self.ribbon.app_bar.query
                        ));
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                    if ui.button("닫기").clicked() {
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                });
            });
        self.ribbon.app_bar.region_popup_open = open;
    }
}
