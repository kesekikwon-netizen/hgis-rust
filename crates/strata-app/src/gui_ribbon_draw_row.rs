//! Flowing draw tools, target/cpp-ref/src/app/MainWindowRibbon.cpp:841-952.
use super::gui_ribbon_context_card::{icon_button, tool_button};
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui;
use strata_canvas2d::map_tools::MapToolKind;

impl StrataAppState {
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

    pub(crate) fn paint_draw_sub_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("그리기 ›").strong());
        let active = self.draw_row_active();
        let select_tip = "도형을 클릭하면 수정점이 나옵니다.\n점을 끌면 그 점만 옮겨집니다.\n점 위에서 우클릭하면 그 점을 지우고, 선 위에서 우클릭하면 점을 넣습니다.\nCtrl을 누른 채 끌면 다른 도형의 점·선에 붙습니다. 점을 클릭해 고른 뒤 Delete로 지웁니다.\n겹친 도형은 같은 자리를 다시 누르면 다음 도형이 골라집니다. 단축키 A. 면 안쪽 도형은 클릭 후 Delete.";
        if icon_button(ui, "select", active == "select", true, select_tip).clicked() {
            self.draw_sketch = crate::draw_sketch::DrawSketch::default();
            self.active_tab = MainTab::Map;
            self.active_tool = MapToolKind::Select;
        }
        self.paint_snap_settings(ctx, ui);
        let tools: [(&str, &str, &str); 5] = [
            (
                "easy",
                "easy_draw",
                "쉽게그리기 — 이미 찍은 점 가까이에 마우스를 가져가면 그 점에 딱 붙어 자동으로 찍힙니다.\n잘못 찍었으면 Ctrl+Z를 누르세요. 바로 앞 점으로 되돌아갑니다.",
            ),
            ("area", "draw_area", "조사구역(발굴 범위) 면을 그립니다. 점을 차례로 찍고 Enter로 닫습니다.\nCtrl+Z는 마지막 점을 되돌립니다."),
            ("poly", "draw_poly", "유구의 면(수혈·주거지 등)을 그립니다. 점을 차례로 찍고 Enter로 닫습니다.\nCtrl+Z는 마지막 점을 되돌립니다."),
            ("line", "draw_line", "유구의 선(석렬·구 등)을 그립니다. 점을 차례로 찍고 Enter로 끝냅니다."),
            ("artifact", "artifact", "유물이 나온 자리를 점으로 찍어 표시합니다. 한 번 클릭에 한 점입니다."),
        ];
        for (id, icon, tip) in tools {
            let enabled = id != "easy";
            let tip = if enabled { tip } else { "아직 Rust 판에서 연결하지 않았습니다 (지시 MENU17)" };
            if icon_button(ui, icon, active == id, enabled, tip).clicked() {
                match id {
                    "area" => self.start_edit_survey_area(),
                    "poly" => self.start_edit_feature_poly(),
                    "line" => self.start_edit_feature_line(),
                    "artifact" => self.start_edit_artifact(),
                    _ => {}
                }
            }
        }
        // target/cpp-ref/src/app/KaDrawSketchTools.cpp:144-145: visible only during capture.
        if self.capture_active() && !self.draw_sketch.points.is_empty() {
            for (icon, tip) in [
                ("check", "그리던 도형을 마칩니다. 우클릭·Enter·더블클릭과 같습니다."),
                ("undo", "마지막 점 하나를 지웁니다. Ctrl+Z·Backspace와 같습니다."),
                ("stop", "그리던 도형을 지웁니다. Esc와 같습니다. 이미 넣은 도형은 그대로입니다."),
            ] {
                if icon_button(ui, icon, false, true, tip).clicked() {
                    match icon {
                        "check" => self.sketch_finish(),
                        "undo" => {
                            self.sketch_undo_vertex();
                        }
                        _ => self.sketch_cancel(),
                    }
                }
            }
        }
        let continuous_tip = "켜면 도형을 마칠 때마다 이름·번호 창을 띄우지 않고 바로 다음 도형을 그립니다.\n이름·번호는 나중에 도형을 우클릭해 「이 도형 기록 입력」으로 넣습니다.\n끄면 도형을 마친 뒤 창이 뜨고, Esc로 건너뛸 수 있습니다.";
        let mut continuous_button = tool_button("연속 그리기");
        if self.ribbon.continuous_draw {
            continuous_button = continuous_button.fill(ui.visuals().selection.bg_fill);
        }
        if ui.add(continuous_button).on_hover_text(continuous_tip).clicked() {
            self.ribbon.continuous_draw = !self.ribbon.continuous_draw;
            self.show_status_message(if self.ribbon.continuous_draw {
                "연속 그리기 — 이름·번호 창 없이 이어서 그립니다."
            } else {
                "도형을 마칠 때마다 이름·번호 창을 띄웁니다(Esc로 건너뜀)."
            });
        }
        self.paint_draw_edit_buttons(ui);
    }

    fn paint_draw_edit_buttons(&mut self, ui: &mut egui::Ui) {
        let edits: [(&str, &str); 4] = [
            ("폴리곤 묶기", "여러 면을 하나로 묶습니다.\n쓰는 법: [도형선택]을 누르고 Shift를 누른 채 지도에서 면을 두 개 이상 고른 뒤\n[폴리곤 묶기]를 누르면 고른 면들이 하나로 합쳐집니다."),
            ("폴리곤 나누기", "[폴리곤 묶기]로 하나가 된 면을 다시 나눕니다.\n쓰는 법: 면을 가로지르는 선을 클릭해 그리고 우클릭하면 그 선을 따라 갈라집니다.\n(Shift로 도형 두 개를 고른 뒤 눌러도 겹친 구간에서 나뉩니다.)"),
            ("구간 분리", "겹쳐 있는 두 레이어에서 겹치는 구간만 새 레이어로 떼어 냅니다. 원본은 그대로 남습니다."),
            ("겹친 곳 지우기", "위에 그린 도형 모양대로 아래 도형에서 그 자리만 지웁니다(클립).\n쓰는 법: 같은 레이어의 도형 위에 겹쳐 그리고 우클릭으로 마치면 선택창에서 고를 수 있습니다.\n이미 그린 도형은 [도형선택](A)으로 고른 뒤 이 단추를 누릅니다.\n아래 도형에 구멍이 나고 위에 그린 도형은 없어집니다. Ctrl+Z로 되돌립니다."),
        ];
        for (label, tip) in edits {
            let enabled = label != "구간 분리";
            let response = ui.add_enabled(enabled, tool_button(label));
            let clicked = if enabled {
                response.on_hover_text(tip)
            } else {
                response.on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 MENU17)")
            }
            .clicked();
            match label {
                "폴리곤 묶기" if clicked => self.merge_selected_polygons(),
                "폴리곤 나누기" if clicked => self.start_split_polygon_tool(),
                "겹친 곳 지우기" if clicked => self.erase_overlap_with_shape(),
                _ => {}
            }
        }
        if ui.add(tool_button("닫기")).on_hover_text("그리기 도구 모음을 닫고 지도 이동으로 돌아갑니다").clicked() {
            self.hide_sub_tools();
        }
    }
}
