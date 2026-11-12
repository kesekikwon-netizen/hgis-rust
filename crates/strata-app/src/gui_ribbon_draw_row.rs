//! 그리기 보조 도구줄 — C++ `showSubToolsDraw`(MainWindowRibbon.cpp:841-952)·`KaSnapSettingsWidget`·
//! `addDrawSketchButtons`(KaDrawSketchTools.cpp:42-131)·`updateSubToolbarChecks`(MainWindowRibbon.cpp:739-765).
//! 자리·크기는 C++ 실물 캡처(화면 배율 1.5, 1 논리 px = 1.5 실제 px)에서 잰 실제 px 를 1.5 로 나눈 값이다
//! (왼쪽 맞춤 글은 egui 글자 여백만큼 앞에서 그린다). 바탕 250,249,245, 선·테두리 135,134,127, 타일 240,238,230.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

const PX: f32 = 1.0 / 1.5;
const ROW_INNER: f32 = 76.0;
const BG: Color32 = Color32::from_rgb(250, 249, 245);
const EDGE: Color32 = Color32::from_rgb(135, 134, 127);
const INK: Color32 = Color32::from_rgb(20, 20, 19);
const MUTED: Color32 = Color32::from_rgb(61, 61, 58);
const TILE: Color32 = Color32::from_rgb(240, 238, 230);
const GLYPH: Color32 = Color32::from_rgb(48, 48, 46);
/// KaThemeSheet.cpp:156-163 — 고른 도구(tileOn·glyphOn), 켜진 토글(tileShown·tileShownBorder), 마우스 위(hover).
const ON_FACE: Color32 = Color32::from_rgb(0x30, 0x30, 0x2E);
const ON_INK: Color32 = Color32::from_rgb(0xFA, 0xF9, 0xF5);
const TOGGLE_FACE: Color32 = Color32::from_rgb(0xE3, 0xDA, 0xCC);
const TOGGLE_EDGE: Color32 = Color32::from_rgb(0xB0, 0xAE, 0xA5);
const HOVER: Color32 = Color32::from_rgb(0xE8, 0xE6, 0xDC);
const FONT: f32 = 13.0;

/// 캡처의 실제 px 자리(줄 안쪽 위가 y 0) → 화면 사각형.
fn at(base: Pos2, x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(base + Vec2::new(x * PX, y * PX), Vec2::new(w * PX, h * PX))
}

#[derive(Clone, Copy, PartialEq)]
enum Look {
    Plain,
    Chosen,
    Toggled,
}

fn button(ui: &mut egui::Ui, rect: Rect, id: &str, look: Look, tip: &str) -> egui::Response {
    let resp = ui.interact(rect, ui.id().with(("ka_draw_row", id)), egui::Sense::click()).on_hover_text(tip);
    let (face, edge) = match look {
        Look::Chosen => (ON_FACE, ON_FACE),
        Look::Toggled => (TOGGLE_FACE, TOGGLE_EDGE),
        Look::Plain if resp.hovered() => (HOVER, EDGE),
        Look::Plain => (Color32::WHITE, EDGE),
    };
    let p = ui.painter();
    p.rect_filled(rect, 8.0, face);
    p.rect_stroke(rect, 8.0, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    resp
}

fn icon_button(ui: &mut egui::Ui, rect: Rect, icon: &str, chosen: bool, tip: &str) -> egui::Response {
    let resp = button(ui, rect, icon, if chosen { Look::Chosen } else { Look::Plain }, tip);
    let tile = Rect::from_center_size(rect.center(), Vec2::splat(30.0 * PX));
    let (tile_face, ink) = if chosen { (ON_FACE, ON_INK) } else { (TILE, GLYPH) };
    let p = ui.painter();
    p.rect_filled(tile, 5.0, tile_face);
    let glyph = Rect::from_center_size(tile.center(), Vec2::splat(tile.width() * crate::svg_path_paint::GLYPH_RATIO * 1.15));
    crate::svg_path_paint::paint_svg_icon(p, glyph, icon, ink);
    resp
}

fn text_button(ui: &mut egui::Ui, rect: Rect, label: &str, look: Look, tip: &str) -> egui::Response {
    let resp = button(ui, rect, label, look, tip);
    let ink = if look == Look::Chosen { ON_INK } else { INK };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(FONT), ink);
    resp
}

fn separator(p: &egui::Painter, base: Pos2, x: f32) {
    p.rect_filled(at(base, x, 22.0, 1.0, 32.0), 0.0, EDGE);
}

/// 「자석」·「공유 경계」 확인란(검게 채운 네모 + 흰 체크).
fn check_box(ui: &mut egui::Ui, base: Pos2, x: f32, text_x: f32, label: &str, on: bool, tip: &str) -> bool {
    let bx = at(base, x, 25.0, 23.0, 24.0);
    let text_w = ui.painter().layout_no_wrap(label.to_owned(), FontId::proportional(FONT), INK).size().x;
    let hit = Rect::from_min_max(bx.min, Pos2::new(base.x + text_x * PX + text_w, bx.max.y));
    let resp = ui.interact(hit, ui.id().with(("ka_draw_check", label)), egui::Sense::click()).on_hover_text(tip);
    let p = ui.painter();
    if on {
        p.rect_filled(bx, 3.0, INK);
        let (c, s) = (bx.center(), bx.width() * 0.5);
        let tick = [c + Vec2::new(-0.55 * s, 0.0), c + Vec2::new(-0.15 * s, 0.4 * s), c + Vec2::new(0.55 * s, -0.45 * s)];
        p.add(egui::Shape::line(tick.to_vec(), Stroke::new(1.6, Color32::WHITE)));
    } else {
        p.rect_filled(bx, 3.0, Color32::WHITE);
        p.rect_stroke(bx, 3.0, Stroke::new(1.0, EDGE), egui::StrokeKind::Inside);
    }
    p.text(Pos2::new(base.x + text_x * PX, bx.center().y), egui::Align2::LEFT_CENTER, label, FontId::proportional(FONT), INK);
    resp.clicked()
}

fn chevron(p: &egui::Painter, c: Pos2, down: bool, ink: Color32, w: f32, h: f32) {
    let h = if down { h } else { -h };
    p.line_segment([c + Vec2::new(-w, -h * 0.5), c + Vec2::new(0.0, h * 0.5)], Stroke::new(1.5, ink));
    p.line_segment([c + Vec2::new(0.0, h * 0.5), c + Vec2::new(w, -h * 0.5)], Stroke::new(1.5, ink));
}

/// 줄 이름(QLabel#subToolbarCaption, font-weight 700) — 맑은 고딕 굵게가 있으면 그것.
fn caption_font(ui: &egui::Ui) -> FontId {
    let bold = egui::FontFamily::Name(crate::gui_tab_map::LABEL_FONT_FAMILY.into());
    if ui.ctx().fonts(|f| f.families().contains(&bold)) { FontId::new(FONT, bold) } else { FontId::proportional(FONT) }
}

fn field_box(p: &egui::Painter, rect: Rect) {
    p.rect_filled(rect, 6.0, Color32::WHITE);
    p.rect_stroke(rect, 6.0, Stroke::new(1.0, EDGE), egui::StrokeKind::Inside);
}

impl StrataAppState {
    /// 지금 고른 그리기 도구(updateSubToolbarChecks) — 도구와 편집 대상 레이어로만 정한다.
    fn draw_row_active(&self) -> &'static str {
        match (self.active_tool, self.edit_layer_key.as_deref()) {
            (MapToolKind::Select, _) => "select",
            (MapToolKind::DrawPolygon, Some(k)) if k == "survey_area" || k.starts_with("survey_area_") => "area",
            (MapToolKind::DrawPolygon, Some("feature_poly")) => "poly",
            (MapToolKind::DrawLine, Some("feature_line")) => "line",
            (MapToolKind::PlacePoint, Some(k)) if k.starts_with("artifact") => "artifact",
            _ => "",
        }
    }

    /// 그리기 줄 한 줄(위아래 1 px 선 + 안쪽 76 실제 px).
    pub(crate) fn paint_draw_sub_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let (full, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_INNER * PX + 2.0), egui::Sense::hover());
        let p = ui.painter().clone();
        p.rect_filled(full, 0.0, BG);
        p.hline(full.x_range(), full.top() + 0.5, Stroke::new(1.0, EDGE));
        p.hline(full.x_range(), full.bottom() - 0.5, Stroke::new(1.0, EDGE));
        let base = Pos2::new(full.left(), full.top() + 1.0);
        let mid = base.y + ROW_INNER * PX * 0.5;
        let active = self.draw_row_active();

        p.text(Pos2::new(base.x + 55.0 * PX, mid), egui::Align2::LEFT_CENTER, "그리기 ›", caption_font(ui), MUTED);
        let select_tip = "도형을 클릭하면 수정점이 나옵니다.\n점을 끌면 그 점만 옮겨집니다.\n점 위에서 우클릭하면 그 점을 지우고, 선 위에서 우클릭하면 점을 넣습니다.\nCtrl을 누른 채 끌면 다른 도형의 점·선에 붙습니다. 점을 클릭해 고른 뒤 Delete로 지웁니다.\n겹친 도형은 같은 자리를 다시 누르면 다음 도형이 골라집니다. 단축키 A. 면 안쪽 도형은 클릭 후 Delete.";
        if icon_button(ui, at(base, 155.0, 10.0, 55.0, 55.0), "select", active == "select", select_tip).clicked() {
            self.draw_sketch = crate::draw_sketch::DrawSketch::default();
            self.active_tab = MainTab::Map;
            self.active_tool = MapToolKind::Select;
        }
        separator(&p, base, 220.0);
        self.paint_snap_settings(ctx, ui, base);
        separator(&p, base, 938.0);

        let tools: [(&str, &str, &str, f32); 5] = [
            ("easy", "easy_draw", "쉽게그리기 — 이미 찍은 점 가까이에 마우스를 가져가면 그 점에 딱 붙어 자동으로 찍힙니다.\n잘못 찍었으면 Ctrl+Z를 누르세요. 바로 앞 점으로 되돌아갑니다.", 949.0),
            ("area", "draw_area", "조사구역(발굴 범위) 면을 그립니다. 점을 차례로 찍고 Enter로 닫습니다.\nCtrl+Z는 마지막 점을 되돌립니다.", 1009.0),
            ("poly", "draw_poly", "유구의 면(수혈·주거지 등)을 그립니다. 점을 차례로 찍고 Enter로 닫습니다.\nCtrl+Z는 마지막 점을 되돌립니다.", 1069.0),
            ("line", "draw_line", "유구의 선(석렬·구 등)을 그립니다. 점을 차례로 찍고 Enter로 끝냅니다.", 1129.0),
            ("artifact", "artifact", "유물이 나온 자리를 점으로 찍어 표시합니다. 한 번 클릭에 한 점입니다.", 1189.0),
        ];
        for (id, icon, tip, x) in tools {
            if icon_button(ui, at(base, x, 10.0, 55.0, 55.0), icon, active == id, tip).clicked() {
                match id {
                    "area" => self.start_edit_survey_area(),
                    "poly" => self.start_edit_feature_poly(),
                    "line" => self.start_edit_feature_line(),
                    "artifact" => {
                        // 유물 위치(점) 그리기는 지시 3-5 — 그때까지 예전 점 찍기 도구.
                        self.active_tab = MainTab::Map;
                        self.active_tool = MapToolKind::PlacePoint;
                        self.home_map.update_guide_for_tool(MapToolKind::PlacePoint);
                    }
                    _ => {} // 쉽게그리기는 뒤 지시(따로 메모리 레이어에 그린다).
                }
            }
        }
        separator(&p, base, 1253.0);

        // 완료·되돌리기·취소는 그리는 동안만 보인다(syncDrawSketchButtons) — 보이면 뒤 단추가 180 실제 px 밀린다.
        let sketching = self.polygon_capture_active() && !self.draw_sketch.points.is_empty();
        let shift = if sketching {
            let sketch: [(&str, &str, f32); 3] = [
                ("check", "그리던 도형을 마칩니다. 우클릭·Enter·더블클릭과 같습니다.", 1264.0),
                ("undo", "마지막 점 하나를 지웁니다. Ctrl+Z·Backspace와 같습니다.", 1324.0),
                ("stop", "그리던 도형을 지웁니다. Esc와 같습니다. 이미 넣은 도형은 그대로입니다.", 1384.0),
            ];
            for (icon, tip, x) in sketch {
                if icon_button(ui, at(base, x, 10.0, 55.0, 55.0), icon, false, tip).clicked() {
                    match icon {
                        "check" => self.sketch_finish(),
                        "undo" => {
                            self.sketch_undo_vertex();
                        }
                        _ => self.sketch_cancel(),
                    }
                }
            }
            180.0
        } else {
            0.0
        };
        let continuous_tip = "켜면 도형을 마칠 때마다 이름·번호 창을 띄우지 않고 바로 다음 도형을 그립니다.\n이름·번호는 나중에 도형을 우클릭해 「이 도형 기록 입력」으로 넣습니다.\n끄면 도형을 마친 뒤 창이 뜨고, Esc로 건너뛸 수 있습니다.";
        let look = if self.ribbon.continuous_draw { Look::Toggled } else { Look::Plain };
        if text_button(ui, at(base, 1264.0 + shift, 12.0, 145.0, 50.0), "연속 그리기", look, continuous_tip).clicked() {
            self.ribbon.continuous_draw = !self.ribbon.continuous_draw;
            self.show_status_message(if self.ribbon.continuous_draw {
                "연속 그리기 — 이름·번호 창 없이 이어서 그립니다."
            } else {
                "도형을 마칠 때마다 이름·번호 창을 띄웁니다(Esc로 건너뜀)."
            });
        }
        separator(&p, base, 1418.0 + shift);
        self.paint_draw_edit_buttons(ui, base, shift);
    }

    /// 폴리곤 묶기·나누기·구간 분리·겹친 곳 지우기(지시 4 — 지금은 누르지 못함)와 닫기.
    fn paint_draw_edit_buttons(&mut self, ui: &mut egui::Ui, base: Pos2, shift: f32) {
        let edits: [(&str, &str, f32, f32); 4] = [
            ("폴리곤 묶기", "여러 면을 하나로 묶습니다.\n쓰는 법: [도형선택]을 누르고 Shift를 누른 채 지도에서 면을 두 개 이상 고른 뒤\n[폴리곤 묶기]를 누르면 고른 면들이 하나로 합쳐집니다.", 1429.0, 145.0),
            ("폴리곤 나누기", "[폴리곤 묶기]로 하나가 된 면을 다시 나눕니다.\n쓰는 법: 면을 가로지르는 선을 클릭해 그리고 우클릭하면 그 선을 따라 갈라집니다.\n(Shift로 도형 두 개를 고른 뒤 눌러도 겹친 구간에서 나뉩니다.)", 1579.0, 164.0),
            ("구간 분리", "겹쳐 있는 두 레이어에서 겹치는 구간만 새 레이어로 떼어 냅니다. 원본은 그대로 남습니다.", 1748.0, 126.0),
            ("겹친 곳 지우기", "위에 그린 도형 모양대로 아래 도형에서 그 자리만 지웁니다(클립).\n쓰는 법: 같은 레이어의 도형 위에 겹쳐 그리고 우클릭으로 마치면 선택창에서 고를 수 있습니다.\n이미 그린 도형은 [도형선택](A)으로 고른 뒤 이 단추를 누릅니다.\n아래 도형에 구멍이 나고 위에 그린 도형은 없어집니다. Ctrl+Z로 되돌립니다.", 1879.0, 172.0),
        ];
        for (label, tip, x, w) in edits {
            // 가짜로 「*」만 켜던 동작은 뺐다 — 진짜 묶기·나누기는 지시 4.
            let _ = text_button(ui, at(base, x + shift, 12.0, w, 50.0), label, Look::Plain, tip);
        }
        if text_button(ui, at(base, 2056.0 + shift, 12.0, 79.0, 50.0), "닫기", Look::Plain, "그리기 도구 모음을 닫고 지도 이동으로 돌아갑니다").clicked() {
            self.hide_sub_tools();
        }
    }

    /// `KaSnapSettingsWidget` — 자석·공유 경계·허용치·붙을 대상, 그 옆 ▾ 자석 보조.
    fn paint_snap_settings(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, base: Pos2) {
        let snap_tip = "켜면 조사구역·유구·지적 선에 붙습니다. 위성 그림에는 붙지 않습니다";
        let topo_tip = "켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집";
        let mut changed = false;
        if check_box(ui, base, 235.0, 273.0, "자석", self.home_map.cad.snap_enabled, snap_tip) {
            self.home_map.cad.snap_enabled = !self.home_map.cad.snap_enabled;
            changed = true;
        }
        if check_box(ui, base, 319.0, 357.0, "공유 경계", self.home_map.cad.snap_topological, topo_tip) {
            self.home_map.cad.snap_topological = !self.home_map.cad.snap_topological;
            changed = true;
        }
        let p = ui.painter().clone();
        p.text(at(base, 450.0, 13.0, 1.0, 48.0).left_center(), egui::Align2::LEFT_CENTER, "허용치", FontId::proportional(FONT), INK);

        let spin = at(base, 517.0, 13.0, 143.0, 48.0);
        field_box(&p, spin);
        let spin_resp = ui.interact(spin, ui.id().with("ka_draw_spin"), egui::Sense::click()).on_hover_text("자석이 붙는 화면 거리");
        let tol = &mut self.home_map.cad.snap_tolerance_px;
        if spin_resp.clicked() {
            if let Some(pos) = spin_resp.interact_pointer_pos() {
                *tol = (*tol + if pos.y < spin.center().y { 1.0 } else { -1.0 }).clamp(4.0, 64.0);
                changed = true;
            }
        }
        if spin_resp.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                *tol = (*tol + f64::from(scroll.signum())).clamp(4.0, 64.0);
                changed = true;
            }
        }
        p.text(Pos2::new(base.x + 537.0 * PX, spin.center().y), egui::Align2::LEFT_CENTER, format!("{:.0} px", *tol), FontId::proportional(FONT), INK);
        let cx = base.x + 641.0 * PX;
        chevron(&p, Pos2::new(cx, spin.center().y - 4.5), false, MUTED, 3.5, 3.0);
        chevron(&p, Pos2::new(cx, spin.center().y + 4.5), true, MUTED, 3.5, 3.0);

        let combo = at(base, 670.0, 13.0, 206.0, 48.0);
        field_box(&p, combo);
        let combo_resp = ui.interact(combo, ui.id().with("ka_draw_combo"), egui::Sense::click()).on_hover_text("조사 도형과 지적 선에 붙습니다. 위성 그림은 제외");
        let current_only = self.home_map.cad.snap_target_current_only;
        let text = if current_only { "현재 레이어" } else { "모든 조사 레이어" };
        p.text(Pos2::new(base.x + 687.0 * PX, combo.center().y), egui::Align2::LEFT_CENTER, text, FontId::proportional(FONT), INK);
        chevron(&p, Pos2::new(base.x + 853.0 * PX, combo.center().y), true, MUTED, 4.5, 4.5);
        let popup_id = ui.id().with("ka_draw_combo_popup");
        if combo_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup_id));
        }
        egui::popup::popup_below_widget(ui, popup_id, &combo_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(combo.width());
            for (value, label) in [(false, "모든 조사 레이어"), (true, "현재 레이어")] {
                if ui.selectable_label(current_only == value, label).clicked() && current_only != value {
                    self.home_map.cad.snap_target_current_only = value;
                    changed = true;
                }
            }
        });
        if changed {
            self.show_status_message(if self.home_map.cad.snap_enabled {
                "자석 켜짐 — 조사 도형·지적 선에 붙습니다. 위성 그림은 제외"
            } else {
                "자석 꺼짐"
            });
        }

        let assist = at(base, 886.0, 10.0, 43.0, 55.0);
        let assist_tip = "자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기";
        if button(ui, assist, "assist", Look::Plain, assist_tip).clicked() {
            self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
        }
        chevron(ui.painter(), assist.center(), true, MUTED, 4.5, 4.5);
        self.paint_snap_assist_popup(ctx, ui, assist);
    }
}
