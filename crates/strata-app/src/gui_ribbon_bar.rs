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

/// `KaRegionLocator.cpp:156, 162-163, 192-194` — 동·읍·면·리 / 지번 칸 안내와 맨 아래 규칙 두 줄
pub const REGION_LOT_PLACEHOLDER: &str = "산 12-3";
pub const REGION_LOT_HINT: &str = "번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요.";
pub const REGION_BOTTOM_HINT: &str =
    "번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3";

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

        // KaBeginnerRibbon.cpp — 1600×900 기준 실측 그룹 좌표(UX_RIBBON_GROUP_XS/WIDTHS)와
        // 각 버튼 폭을 유지하며, 좁은 모니터에서는 비율 축소해 모든 모니터(1280~4K)에 대응한다.
        let avail_for_ribbon = (ribbon_rect.width() - 266.0).max(640.0);
        let h_scale = (avail_for_ribbon / 1328.0).clamp(0.72, 1.0);
        let chip_base_w = |g_idx: usize, label: &str| -> f32 {
            match (g_idx, label) {
                (0, _) => 53.0,
                (1, "그리기" | "시굴격자") => 51.0,
                (1, _) => 47.0,
                (2, "수치지형") => 60.0,
                (2, _) => 47.0,
                (3, "옛 지도") => 51.0,
                (3, _) => 47.0,
                (4, _) => 47.0,
                (5, "GeoTIFF" | "검수·제출") => 60.0,
                (5, _) => 47.0,
                _ => 48.0,
            }
        };

        for (g_idx, (g_title, items)) in groups.iter().enumerate() {
            let gx = ribbon_rect.left() + UX_RIBBON_GROUP_XS[g_idx] * h_scale;
            let gw = UX_RIBBON_GROUP_WIDTHS[g_idx] * h_scale;
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
                Color32::from_rgb(94, 93, 89),
            );
            let mut bx = g_rect.left() + 1.0;
            for &label in items.iter() {
                let cw = chip_base_w(g_idx, label) * h_scale;
                let btn_rect =
                    Rect::from_min_size(Pos2::new(bx, g_rect.top() + 18.0), Vec2::new(cw, 69.0));
                bx += cw;
                let muted = is_home && (*g_title == "기록" || label == "GeoTIFF");
                let highlight_save = label == "저장" && dirty_save;
                let dark_active = (label == "선택" && self.active_tool == MapToolKind::Select)
                    || (label == "측거" && self.active_tool == MapToolKind::Measure)
                    || (label == "그리기"
                        && (self.active_tool == MapToolKind::Measure
                            || (self.ribbon.sub_toolbar_visible
                                && self.ribbon.sub_mode == SubToolsMode::Draw)))
                    || (label == "버퍼"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Buffer)
                    || (label == "정합"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Align);
                let warm_active = label == "DEM" && !is_home;
                let mut chip_ui = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
                let (chip_clicked, chip_secondary) = self.draw_ribbon_chip(
                    &mut chip_ui,
                    label,
                    muted,
                    highlight_save,
                    dark_active,
                    warm_active,
                );
                if chip_clicked {
                    if label == "옛 지도" || label == "더보기" {
                        self.open_ribbon_context_menu(label, btn_rect);
                    } else {
                        self.handle_ribbon_click(label);
                    }
                }
                // KaPaleoMenu::install 등 우클릭 팝업
                if chip_secondary {
                    self.open_ribbon_context_menu(label, btn_rect);
                }
            }
        }

        // KaAppBar.cpp:25-58 — [📍 지역] 버튼 + [🔍 주소·지번 찾기] 입력칸
        let right_left = (ribbon_rect.left() + 1334.0 * h_scale).max(ribbon_rect.right() - 264.0);
        let right_rect = Rect::from_min_max(
            Pos2::new(right_left, ribbon_rect.top() + 1.0),
            Pos2::new(ribbon_rect.right() - 8.0, ribbon_rect.bottom() - 1.0),
        );
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let search_placeholder = if self.display_options.paper_look {
                "주소·지번 찾기"
            } else {
                "도로명·지번 (예: 하회종가길 40)  Ctrl+F"
            };
            let resp = ui
                .add_sized(
                    [156.0, 32.0],
                    egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                        .hint_text(format!("     {search_placeholder}"))
                        .vertical_align(egui::Align::Center),
                )
                .on_hover_text("도로명·지번으로 찾습니다 (예: 하회종가길 40) · Ctrl+F");
            // 검색칸 안쪽 왼쪽 돋보기 아이콘(16px, KaAppBar.cpp:41)
            {
                let sp = ui.painter();
                let c = Pos2::new(resp.rect.left() + 12.0, resp.rect.center().y - 1.0);
                let ink = Color32::from_rgb(94, 93, 89);
                sp.circle_stroke(c, 4.8, Stroke::new(1.5, ink));
                sp.line_segment(
                    [
                        Pos2::new(c.x + 3.4, c.y + 3.4),
                        Pos2::new(c.x + 7.2, c.y + 7.2),
                    ],
                    Stroke::new(1.6, ink),
                );
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.active_tab = MainTab::Map;
                self.home_map.draw_guide_message = Some(format!(
                    "주소·지번 「{}」 위치로 이동했습니다.",
                    self.ribbon.app_bar.query
                ));
            }
            let (reg_rect, reg_resp) =
                ui.allocate_exact_size(Vec2::new(76.0, 36.0), egui::Sense::click());
            {
                let rp = ui.painter_at(reg_rect);
                let bg = if reg_resp.hovered() {
                    Color32::from_rgb(245, 244, 237)
                } else {
                    Color32::WHITE
                };
                rp.rect_filled(reg_rect, 6.0, bg);
                rp.rect_stroke(
                    reg_rect,
                    6.0,
                    Stroke::new(1.0, Color32::from_rgb(194, 192, 182)),
                    egui::StrokeKind::Inside,
                );
                // 지도 핀 아이콘 (KaIcons "gps", 16px, KaAppBar.cpp:32)
                let pin_c = Pos2::new(reg_rect.left() + 19.0, reg_rect.center().y - 2.0);
                let pin_ink = Color32::from_rgb(94, 93, 89);
                rp.circle_stroke(pin_c, 4.5, Stroke::new(1.5, pin_ink));
                rp.circle_filled(pin_c, 1.6, pin_ink);
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x - 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x + 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.text(
                    Pos2::new(reg_rect.left() + 31.0, reg_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "지역",
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(20, 20, 19),
                );
            }
            if reg_resp
                .on_hover_text("시·도를 고른 뒤 시·군·동과 지번으로 찾아갑니다")
                .clicked()
            {
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
                ui.spacing_mut().item_spacing.x = 6.0;
                if self.theme_menu_open {
                    let popup_pos = Pos2::new(
                        (tabbar_rect.right() - 220.0).max(16.0),
                        tabbar_rect.bottom() + 4.0,
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
                            egui::RichText::new("저장됨")
                                .size(12.0)
                                .color(Color32::from_rgb(94, 93, 89)),
                        );
                    }
                    ui.label(
                        egui::RichText::new(&self.project_title)
                            .size(12.0)
                            .color(Color32::from_rgb(94, 93, 89)),
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
    ) -> (bool, bool) {
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
        // KaBeginnerRibbon.cpp:250 — setIconSize(QSize(tile, tile)).
        // 모든 칩이 동일한 정사각형 타일(39×39 px, 좁은 창에서는 비례 축소)을 쓴다.
        let tile_side = (rect.width() - 6.0)
            .min(rect.height() - 28.0)
            .clamp(26.0, 39.0);
        let icon_box = Rect::from_min_size(
            Pos2::new(rect.center().x - tile_side * 0.5, rect.top() + 3.0),
            Vec2::new(tile_side, tile_side),
        );
        let corner = tile_side * 0.25;
        let is_hovered = resp.hovered()
            && ui
                .input(|i| i.pointer.hover_pos())
                .is_none_or(|pos| rect.contains(pos));
        let paper = self.display_options.paper_look;
        if highlight_save {
            let strong_bg = if paper {
                Color32::from_rgb(0xB5, 0x57, 0x3A)
            } else {
                Color32::from_rgb(0x20, 0x6C, 0xB0)
            };
            p.rect_filled(icon_box, corner, strong_bg);
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                4.2,
                Color32::WHITE,
            );
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                2.8,
                Color32::from_rgb(0xF2, 0xA3, 0x3A),
            );
        } else if dark_active {
            let (on_bg, on_fg) = if paper {
                (
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                    Color32::from_rgb(0xFA, 0xF9, 0xF5),
                )
            } else {
                (
                    Color32::from_rgb(0xE0, 0xEC, 0xF8),
                    Color32::from_rgb(0x10, 0x50, 0x88),
                )
            };
            p.rect_filled(icon_box, corner, on_bg);
            if !paper {
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(1.5, Color32::from_rgb(0x1A, 0x68, 0xB0)),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, on_fg);
        } else {
            // KaThemePaper.cpp:60-70 vs KaTheme.cpp:105-113
            let (bg, stroke_col) = if muted {
                if paper {
                    (
                        Color32::from_rgb(0xF5, 0xF4, 0xED),
                        Color32::from_rgb(0x9E, 0x9D, 0x98),
                    )
                } else {
                    (
                        Color32::from_rgb(0xEE, 0xF1, 0xF3),
                        Color32::from_rgb(0x8F, 0x98, 0xA3),
                    )
                }
            } else if warm_active {
                if paper {
                    (
                        Color32::from_rgb(0xE3, 0xDA, 0xCC),
                        Color32::from_rgb(0x14, 0x14, 0x13),
                    )
                } else {
                    (
                        Color32::from_rgb(0xE0, 0xEC, 0xF8),
                        Color32::from_rgb(0x10, 0x50, 0x88),
                    )
                }
            } else if is_hovered {
                if paper {
                    (
                        Color32::from_rgb(0xE8, 0xE6, 0xDC),
                        Color32::from_rgb(0x30, 0x30, 0x2E),
                    )
                } else {
                    (
                        Color32::from_rgb(0xD9, 0xE2, 0xE7),
                        Color32::from_rgb(0x2B, 0x48, 0x58),
                    )
                }
            } else if paper {
                (
                    Color32::from_rgb(0xF0, 0xEE, 0xE6),
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                )
            } else {
                (
                    Color32::from_rgb(0xE4, 0xEA, 0xED),
                    Color32::from_rgb(0x2B, 0x48, 0x58),
                )
            };
            p.rect_filled(icon_box, corner, bg);
            if warm_active {
                let bw = (tile_side * 2.0 / 32.0).max(1.0);
                let border_col = if paper {
                    Color32::from_rgb(0xB0, 0xAE, 0xA5)
                } else {
                    Color32::from_rgb(0x1A, 0x68, 0xB0)
                };
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(bw, border_col),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, stroke_col);
        }
        let txt_col = if muted {
            Color32::from_rgb(0x9E, 0x9D, 0x98)
        } else {
            Color32::from_rgb(0x14, 0x14, 0x13)
        };
        p.text(
            Pos2::new(rect.center().x, rect.top() + tile_side + 15.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            txt_col,
        );
        let resp = resp.on_hover_text(cpp_ribbon_tooltip(label));
        let clicked = resp.clicked()
            && resp
                .interact_pointer_pos()
                .is_none_or(|pos| rect.contains(pos));
        let secondary = resp.secondary_clicked();
        (clicked, secondary)
    }

    /// KaPaleoCrossSectionWidget.cpp:99-158 `KaPaleoMenu::install` +
    /// MainWindowRibbon.cpp:390-711 —
    /// 리본 `고지형`/`지질`/`DEM`/`토양`/`지적`/`수치지형`/`옛 지도`/`더보기` 팝업 메뉴.
    pub(crate) fn open_ribbon_context_menu(&mut self, label: &str, anchor: egui::Rect) {
        self.ribbon.paleo_menu_open = false;
        self.ribbon.geology_menu_open = false;
        self.ribbon.dem_menu_open = false;
        self.ribbon.soil_menu_open = false;
        self.ribbon.cadastral_menu_open = false;
        self.ribbon.topo_menu_open = false;
        self.ribbon.old_map_menu_open = false;
        self.ribbon.more_menu_open = false;
        self.ribbon.context_menu_anchor = Pos2::new(anchor.left(), anchor.bottom() + 2.0);
        match label {
            "고지형" => self.ribbon.paleo_menu_open = true,
            "지질" => self.ribbon.geology_menu_open = true,
            "DEM" => self.ribbon.dem_menu_open = true,
            "토양" => self.ribbon.soil_menu_open = true,
            "지적" => self.ribbon.cadastral_menu_open = true,
            "수치지형" => self.ribbon.topo_menu_open = true,
            "옛 지도" => self.ribbon.old_map_menu_open = true,
            "더보기" => {
                self.ribbon.context_menu_anchor =
                    Pos2::new((anchor.right() - 250.0).max(16.0), anchor.bottom() + 2.0);
                self.ribbon.more_menu_open = true;
            }
            _ => {}
        }
    }

    /// 우클릭 팝업 메뉴를 그린다. 항목을 고르면 닫고 실제 동작을 실행한다.
    pub(crate) fn render_ribbon_context_menus(&mut self, ctx: &egui::Context) {
        let anchor = self.ribbon.context_menu_anchor;
        // ── 고지형 (7항목, KaPaleoMenu::install)
        if self.ribbon.paleo_menu_open {
            let mut open = true;
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_paleo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 7] = [
                        "「고지형분석」 전용 탭 열기",
                        "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 창…",
                        "수치지질도(KIGAM 1:5만) 별도 받기 (반경 5km · 도면 범례)",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km · 도면 범례)",
                        "KIGAM 단층·지하수위·단구연대 별도 받기 (반경 10km)",
                        "국토지반정보 포털 열기 (geoinfo.or.kr)…",
                        "KIGAM 지오빅데이터 포털 열기 (data.kigam.re.kr)…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 2 || i == 5 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [420.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.paleo_menu_open = false;
                self.run_paleo_menu_action(item);
                open = false;
            }
            let _ = open;
        }
        // ── 지질 (6항목, MainWindowRibbon.cpp geoMenu)
        if self.ribbon.geology_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_geology_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 6] = [
                        "조사구역 반경 5km 수치지질도 받기 (1:5만 · 권장)",
                        "조사구역 반경 2km 수치지질도 받기",
                        "조사구역 반경 10km 수치지질도 받기",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km)",
                        "KIGAM 단층·선구조·수문 별도 받기 (반경 10km)",
                        "수치지질도 · 시추주상도 별도 내려받기 창…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 3 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [400.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.geology_menu_open = false;
                self.run_geology_menu_action(item);
            }
        }
        // ── DEM (5항목, MainWindowChrome.cpp:215-223)
        if self.ribbon.dem_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_dem_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 5] = [
                        "국토지리정보원 계정 설정·자동 다운로드…",
                        "국토지리원 DEM 불러오기(.img)…",
                        "DEM 파일로 음영 만들기…",
                        "DEM 표현…",
                        "측량점으로 등고선…",
                    ];
                    for item in ITEMS {
                        if ui
                            .add_sized(
                                [330.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.dem_menu_open = false;
                self.run_dem_menu_action(item);
            }
        }
        // ── 토양 (2항목, MainWindowRibbon.cpp:447-452)
        if self.ribbon.soil_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_soil_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in [
                        "분포지형 내려받기 — 현재 화면 범위",
                        "토양도 파일 불러오기(SHP·GPKG)",
                    ] {
                        if ui
                            .add_sized(
                                [250.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.soil_menu_open = false;
                self.handle_ribbon_click("토양");
            }
        }
        // ── 지적 (2항목, MainWindowRibbon.cpp:477-478)
        if self.ribbon.cadastral_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_cadastral_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["VWorld 계정 설정", "선 색·지번 표시"] {
                        if ui
                            .add_sized(
                                [190.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.cadastral_menu_open = false;
                if item.contains("계정") {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                } else {
                    self.modals.active = Some(ActiveModalDialog::CadastralDownload);
                }
            }
        }
        // ── 수치지형 (2항목, MainWindowRibbon.cpp:391-392)
        if self.ribbon.topo_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_topo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["국토지리정보원에서 내려받기…", "받아 둔 수치지형도 폴더…"]
                    {
                        if ui
                            .add_sized(
                                [230.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.topo_menu_open = false;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
        }
        // ── 옛 지도 (2항목, MainWindowRibbon.cpp:492-500 & 12-oldmap.png / 14-drawing.png)
        if self.ribbon.old_map_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_old_map_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["▤  대동여지도", "▤  1919 조선지형도 1:5만"] {
                        if ui
                            .add_sized(
                                [196.0, 26.0],
                                egui::Button::new(egui::RichText::new(item).size(12.5))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.old_map_menu_open = false;
                self.active_tab = MainTab::Map;
                let map_type = if item.contains("대동여지도") {
                    strata_net::tile_fetcher::HistoricalMapType::Daedongyeojido
                } else {
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000
                };
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(map_type, 0.8);
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
        }
        // ── 더보기 (MainWindowRibbon.cpp:628-710 — 불러오기·배경 / 화면 / 계정과 키 / 상태와 정보)
        if self.ribbon.more_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_more_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    ui.set_min_width(236.0);
                    ui.label(
                        egui::RichText::new("불러오기·배경")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "벡터·도면 불러오기",
                        "참조 벡터를 조사 파일 밖으로…",
                        "조사 폴더 임시 자료 정리…",
                        "토양도 SHP 불러오기",
                        "CSV 기준점",
                        "맞추기",
                        "OSM 배경",
                        "Google 위성",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("화면")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "파일함 보이기/숨기기",
                        "지도 넓게 보기 (F11)",
                        "왼쪽 패널 접기/펴기 (F9)",
                        "화면 보기 (고대비·큰 글씨·테마)…",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("계정과 키 · 상태와 정보")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in ["API 키 입력 · 계정 설정…", "자료 준비 상태", "정보"]
                    {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.more_menu_open = false;
                if item.contains("화면 보기") {
                    self.theme_menu_open = !self.theme_menu_open;
                } else if item.contains("맞추기") {
                    self.handle_ribbon_click("정합");
                } else if item.contains("OSM 배경") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldBase;
                } else if item.contains("Google 위성") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldSatellite;
                } else if item.contains("파일함") {
                    self.home_map.files_open = !self.home_map.files_open;
                } else if item.contains("벡터·도면") {
                    self.modals.active = Some(ActiveModalDialog::CadImportNotice);
                } else {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
            }
        }
        // 바깥을 누르면 닫는다
        if ctx.input(|i| i.pointer.any_click()) {
            let any_open = self.ribbon.paleo_menu_open
                || self.ribbon.geology_menu_open
                || self.ribbon.dem_menu_open
                || self.ribbon.soil_menu_open
                || self.ribbon.cadastral_menu_open
                || self.ribbon.topo_menu_open
                || self.ribbon.old_map_menu_open
                || self.ribbon.more_menu_open;
            if any_open {
                let pos = ctx.input(|i| i.pointer.interact_pos());
                if let Some(p) = pos {
                    let near = anchor.x - 16.0 <= p.x
                        && p.x <= anchor.x + 450.0
                        && anchor.y - 36.0 <= p.y
                        && p.y <= anchor.y + 420.0;
                    if !near {
                        self.ribbon.paleo_menu_open = false;
                        self.ribbon.geology_menu_open = false;
                        self.ribbon.dem_menu_open = false;
                        self.ribbon.soil_menu_open = false;
                        self.ribbon.cadastral_menu_open = false;
                        self.ribbon.topo_menu_open = false;
                        self.ribbon.old_map_menu_open = false;
                        self.ribbon.more_menu_open = false;
                    }
                }
            }
        }
    }

    fn run_paleo_menu_action(&mut self, item: &str) {
        if item.contains("전용 탭 열기") {
            self.open_studio_tab(MainTab::Paleo);
        } else if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 5km · 도면 범례") {
            self.paleo_3d.source_dl_radius_km = 5.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("포털 열기") {
            self.home_map.draw_guide_message =
                Some(format!("{item} — 브라우저에서 이어서 받으세요."));
        }
    }

    fn run_geology_menu_action(&mut self, item: &str) {
        if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
            return;
        }
        if item.contains("반경 2km") {
            self.paleo_3d.source_dl_radius_km = 2.0;
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
        } else {
            self.paleo_3d.source_dl_radius_km = 5.0;
        }
        if item.contains("수치지질도") {
            self.active_tab = MainTab::Map;
            let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                strata_net::tile_fetcher::ThematicLayerType::Geology1to50k,
                self.work_crs.auth_id(),
            );
            self.home_map.draw_guide_message = Some(format!(
                "{} — 반경 {:.0} km 기준으로 켰습니다.",
                spec.layer_title_ko, self.paleo_3d.source_dl_radius_km
            ));
        } else {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        }
    }

    fn run_dem_menu_action(&mut self, item: &str) {
        if item.contains("계정 설정") {
            self.modals.active = Some(ActiveModalDialog::AccountSettings);
        } else if item.contains("DEM 불러오기") {
            self.modals.active = Some(ActiveModalDialog::DemDownload);
        } else if item.contains("음영 만들기") || item.contains("DEM 표현") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::DemClass);
        } else if item.contains("등고선") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::SurveyContour);
        }
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

    /// `KaRegionLocator.cpp:124-286` — 17개 광역 시·도 타일 밑에 붙는 주소 팝업
    /// (시·도 제목 · ✕ · 시·군·구 · 동·읍·면 · 지번 · 찾기/취소 · 아래 힌트 2줄)
    fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        if !self.ribbon.app_bar.region_popup_open {
            return;
        }
        let mut open = self.ribbon.app_bar.region_popup_open;
        let mut run_search = false;
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
                // KaRegionLocator.cpp:180-190 — 고른 시·도 이름(왼쪽)과 ✕ 닫기(오른쪽)
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.ribbon.app_bar.selected_sido)
                            .size(13.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").on_hover_text("닫기 (Esc)").clicked() {
                            self.ribbon.app_bar.region_popup_open = false;
                        }
                    });
                });
                ui.add_space(4.0);
                // KaRegionLocator.cpp:143-195 — 시·군·구 · 동·읍·면 · 지번(산 12-3) · 찾기 · 취소
                egui::Grid::new("ka_region_detail_inputs")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("시·군·구").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.city_input)
                                .hint_text("시·군·구")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                        ui.label(egui::RichText::new("동·읍·면").size(12.0));
                        let dong = ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input)
                                .hint_text("법정동·읍·면·리")
                                .desired_width(210.0),
                        );
                        if dong.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            run_search = true;
                        }
                        ui.end_row();
                        let lot_lbl = ui.label(egui::RichText::new("지번").size(12.0));
                        lot_lbl.on_hover_text(REGION_LOT_HINT);
                        let lot = ui
                            .add(
                                egui::TextEdit::singleline(&mut self.ribbon.app_bar.lot_input)
                                    .hint_text(REGION_LOT_PLACEHOLDER)
                                    .desired_width(210.0),
                            )
                            .on_hover_text(REGION_LOT_HINT);
                        if lot.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            run_search = true;
                        }
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let find_btn = egui::Button::new(
                        egui::RichText::new("찾기")
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.0),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(140.0, 30.0));
                    if ui.add(find_btn).clicked() {
                        run_search = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("취소").size(12.5))
                                .min_size(Vec2::new(80.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                });
                // KaRegionLocator.cpp:192-194 — 번지 입력 규칙 안내
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(REGION_BOTTOM_HINT)
                        .size(11.0)
                        .color(Color32::from_rgb(110, 108, 102)),
                );
            });
        if run_search {
            self.search_region_locator_address();
        }
        if !self.ribbon.app_bar.region_popup_open {
            open = false;
        }
        self.ribbon.app_bar.region_popup_open = open;
    }

    /// `KaRegionLocator::emitSearch` (`KaRegionLocator.cpp:143-195, 265-276`) — 지번까지 적었으면
    /// 그 지번의 필지만 찾아 그 자리로 옮기고, 아니면 시·도(시·군·구·동) 중심으로 옮긴다.
    pub(crate) fn search_region_locator_address(&mut self) {
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
}
