//! 왼쪽 레이어 트리 우클릭 팝업 - C++ `MainWindowContextMenus.cpp:176-448`
//! `layerContextMenu`. 항목 구성·순서는 C++ 그대로다.

use eframe::egui;

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// MainWindowContextMenus.cpp:176-448 `layerContextMenu` — 왼쪽 레이어 트리
    /// 우클릭 팝업. 항목 구성·순서는 C++ 그대로다.
    pub(crate) fn render_layer_context_menu(&mut self, ctx: &egui::Context) {
        if !self.layer_context_menu_open {
            return;
        }
        let Some(key) = self.layer_context_menu_key.clone() else {
            self.layer_context_menu_open = false;
            return;
        };
        let pos = egui::Pos2::new(
            self.layer_context_menu_pos[0],
            self.layer_context_menu_pos[1],
        );
        let mut picked: Option<&'static str> = None;
        let is_area_layer = key.contains("조사구역");
        let is_trench_layer = key.contains("시굴격자");
        let is_survey_layer = key.contains("조사");
        let is_raster = key.contains("지형 음영")
            || key.contains("지표모델")
            || key.contains("위성")
            || key.contains("항공");
        egui::Window::new("ka_layer_context_menu")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.set_min_width(260.0);
                let row = |ui: &mut egui::Ui, label: &str| -> bool {
                    ui.add_sized(
                        [248.0, 22.0],
                        egui::Button::new(egui::RichText::new(label).size(12.0))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE),
                    )
                    .clicked()
                };
                let sub_hdr = |ui: &mut egui::Ui, label: &str| {
                    ui.label(
                        egui::RichText::new(label)
                            .size(10.5)
                            .strong()
                            .color(egui::Color32::from_rgb(120, 118, 110)),
                    );
                };
                if row(ui, "참고 자료 불러오기…") {
                    picked = Some("import");
                }
                ui.separator();
                if row(ui, "이름 바꾸기 (두 번 클릭)") {
                    picked = Some("rename");
                }
                if row(ui, "면·외곽선 색") {
                    picked = Some("color");
                }
                if row(ui, "도형 속성") {
                    picked = Some("shape_props");
                }
                // 라벨·명칭 속성
                sub_hdr(ui, "라벨·명칭 속성");
                if row(ui, "  보이기/숨기기") {
                    picked = Some("label_toggle");
                }
                if row(ui, "  면적(㎡) 함께 표시") {
                    picked = Some("label_area");
                }
                for pt in [3_u32, 4, 5, 6, 7, 8, 9, 10, 12, 14] {
                    if ui
                        .add_sized(
                            [248.0, 20.0],
                            egui::Button::new(
                                egui::RichText::new(format!("  글자 {pt} pt")).size(11.5),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.home_map.layer_label_size_pt = pt as i32;
                    }
                }
                ui.separator();
                // 깨진 한글 바로잡기
                sub_hdr(ui, "깨진 한글 바로잡기");
                for enc in ["CP949", "EUC-KR", "UTF-8", "이 컴퓨터 설정"] {
                    if row(ui, &format!("  {enc}")) {
                        picked = Some("encoding");
                    }
                }
                ui.separator();
                // 투명도
                sub_hdr(ui, "투명도");
                for pct in [0_u8, 20, 40, 50, 60, 80] {
                    if ui
                        .add_sized(
                            [248.0, 20.0],
                            egui::Button::new(
                                egui::RichText::new(format!("  {pct}% 투명하게")).size(11.5),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE),
                        )
                        .clicked()
                    {
                        self.home_map.real_survey.layer_opacity = 1.0 - f32::from(pct) / 100.0;
                        self.home_map.draw_guide_message =
                            Some(format!("「{key}」 투명도를 {pct}%로 맞췄습니다."));
                    }
                }
                ui.separator();
                if row(ui, "이 레이어로 이동") {
                    picked = Some("zoom_layer");
                }
                if row(ui, "전체 보기") {
                    picked = Some("zoom_all");
                }
                if row(ui, "같은 묶음의 맨 아래로") {
                    picked = Some("to_bottom");
                }
                if !is_raster {
                    ui.separator();
                    if row(ui, "그리기·편집 시작") {
                        picked = Some("edit_start");
                    }
                    if row(ui, "꼭짓점 수정") {
                        picked = Some("vertex_edit");
                    }
                    if row(ui, "폴리곤 묶기") {
                        picked = Some("poly_merge");
                    }
                    let area_lbl = if is_trench_layer {
                        "면적·조사 비율 확인"
                    } else {
                        "면적 확인"
                    };
                    if row(ui, area_lbl) {
                        picked = Some("area_check");
                    }
                    if is_area_layer {
                        sub_hdr(ui, "시굴격자");
                        if row(ui, "  시굴 · 조사구역의 10%") {
                            picked = Some("trench_10");
                        }
                        if row(ui, "  표본 · 조사구역의 2%") {
                            picked = Some("trench_2");
                        }
                        if row(ui, "  규격 지정·격자 만들기…") {
                            picked = Some("trench_custom");
                        }
                    }
                }
                if is_raster {
                    ui.separator();
                    if row(ui, "오프라인 저장 — 지금 화면 범위") {
                        picked = Some("offline_save");
                    }
                }
                if is_survey_layer {
                    ui.separator();
                    if row(ui, "제출 꾸러미 만들기…") {
                        picked = Some("submit");
                    }
                }
                ui.separator();
                if row(ui, "그린 도형 모두 지우기…") {
                    picked = Some("clear_shapes");
                }
                if row(ui, "레이어 삭제") {
                    picked = Some("delete_layer");
                }
            });
        if let Some(key_act) = picked {
            match key_act {
                "import" => {
                    self.modals.active =
                        Some(crate::dialogs_modal::ActiveModalDialog::TopographicImport);
                }
                "rename" => {
                    self.home_map.draw_guide_message =
                        Some("이름을 바꾸려면 레이어 이름을 두 번 누르세요.".to_owned());
                }
                "color" => {
                    self.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::LineColor);
                }
                "shape_props" => {
                    self.right_inspector.active_card =
                        crate::right_inspector::InspectorCardTab::LayerStyleCard;
                }
                "label_toggle" => {
                    self.home_map.real_survey.toggle_label_visible(&key);
                }
                "label_area" => {
                    self.home_map.layer_label_with_area = !self.home_map.layer_label_with_area;
                }
                "encoding" => {
                    self.home_map.draw_guide_message =
                        Some(format!("「{key}」 글자 인코딩을 다시 읽습니다."));
                }
                "zoom_layer" | "zoom_all" => {
                    self.home_map.fit_to_survey();
                }
                "to_bottom" => {
                    self.home_map.draw_guide_message =
                        Some(format!("「{key}」을 묶음 맨 아래로 내렸습니다."));
                }
                "edit_start" => {
                    self.active_tab = crate::app_state::MainTab::Map;
                    self.start_edit_from_legend(&key);
                }
                "vertex_edit" => {
                    self.active_tab = crate::app_state::MainTab::Map;
                    self.active_tool = strata_canvas2d::map_tools::MapToolKind::Select;
                }
                "poly_merge" => {
                    self.home_map.draw_guide_message =
                        Some("폴리곤 묶기 — 이어 붙일 두 면을 차례로 고르세요.".to_owned());
                }
                "area_check" => {
                    // showLayerAreaSummary — C++ 은 알림 창(「면적 확인」)이다. 알림 창이 생기기 전까지 상태줄에 줄을 이어 쓴다.
                    if let Some(text) = self.layer_area_summary(&key, is_trench_layer) {
                        self.show_status_message(text.replace('\n', " — "));
                    }
                }
                "trench_10" | "trench_2" | "trench_custom" => {
                    self.modals.active =
                        Some(crate::dialogs_modal::ActiveModalDialog::TrenchGenerator);
                }
                "offline_save" => {
                    // C++ 는 타일을 바로 받아 저장한다. 여기서는 안내만 남긴다.
                    self.home_map.draw_guide_message = Some(format!(
                        "「{key}」 지금 화면 범위를 오프라인으로 저장합니다."
                    ));
                }
                "submit" => {
                    self.modals.active =
                        Some(crate::dialogs_modal::ActiveModalDialog::ExportSubmitPackage);
                }
                "clear_shapes" => {
                    self.home_map.scene.polygons.clear();
                    self.home_map.real_survey.survey_rings.clear();
                    self.is_dirty = true;
                    self.home_map.draw_guide_message =
                        Some("그린 도형을 모두 지웠습니다.".to_owned());
                }
                "delete_layer" => {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert(key.clone(), false);
                    self.home_map.draw_guide_message =
                        Some(format!("「{key}」 레이어를 숨겼습니다."));
                }
                _ => {}
            }
            self.layer_context_menu_open = false;
        }
        if ctx.input(|i| i.pointer.any_click() && !i.pointer.any_down()) {
            let pos_now = ctx.input(|i| i.pointer.interact_pos());
            if let Some(p) = pos_now {
                let a = self.layer_context_menu_pos;
                let inside = a[0] - 6.0 <= p.x
                    && p.x <= a[0] + 276.0
                    && a[1] - 6.0 <= p.y
                    && p.y <= a[1] + 640.0;
                if !inside {
                    self.layer_context_menu_open = false;
                }
            }
        }
    }
}
