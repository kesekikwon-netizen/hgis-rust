use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::path::PathBuf;

use crate::app_state::StrataAppState;

/// UX_LAYOUT_SPEC.md 3절: 왼쪽 패널(폭 351), 상단 레이어 카드(높이 507), 하단 파일함 카드(높이 168)
pub const UX_LEFT_PANEL_WIDTH: f32 = 351.0;
pub const UX_LAYER_CARD_HEIGHT: f32 = 507.0;
pub const UX_FILE_DRAWER_HEIGHT: f32 = 168.0;

/// `KaFileBrowserPanel.cpp:92-136` — 폴더 단추 여섯 개와 목록 칸의 안내말
pub const FILE_BTN_PC_TIP: &str = "컴퓨터 드라이브 목록으로 이동합니다";
pub const FILE_BTN_DESK_TIP: &str = "바탕화면 폴더로 이동합니다";
pub const FILE_BTN_DOCS_TIP: &str = "문서 폴더로 이동합니다";
pub const FILE_BTN_DOWN_TIP: &str = "내려받기 폴더로 이동합니다";
pub const FILE_BTN_PICK_TIP: &str =
    "조사 자료 폴더를 직접 찾아 엽니다. 카메라·GPS 메모리카드(SD)도 여기서 엽니다";
pub const FILE_BTN_UP_TIP: &str = "상위 폴더로 한 단계 이동합니다";
pub const FILE_LIST_TIP: &str = "파일을 지도에 끌어 넣으면 레이어가 됩니다.";

/// KaLayerInformationPanel.cpp:26-33 `kLabelInks` — 6 label ink presets.
pub const KA_LABEL_INKS: [(&str, (u8, u8, u8)); 6] = [
    ("기본 (먹색)", (31, 35, 40)),
    ("흰색", (255, 255, 255)),
    ("진한 파랑", (30, 58, 138)),
    ("진한 빨강", (153, 27, 27)),
    ("갈색", (124, 45, 18)),
    ("진한 초록", (20, 83, 45)),
];

#[derive(Clone, Copy)]
enum SwatchKind {
    CadastralOutline,
    TrenchBeige,
    RasterChecker,
    SurveyOrange,
    DistanceBox,
    BufferBand,
    GroupFolder,
    HeritageSub,
}

fn paint_checkbox_state(p: &egui::Painter, center: Pos2, check_state: u8) {
    let r = Rect::from_center_size(center, Vec2::new(13.0, 13.0));
    if check_state == 2 {
        p.rect_filled(r, 2.5, Color32::from_rgb(28, 28, 26));
        let c = r.center();
        p.line_segment(
            [
                Pos2::new(c.x - 3.2, c.y + 0.2),
                Pos2::new(c.x - 0.8, c.y + 2.8),
            ],
            Stroke::new(1.7, Color32::WHITE),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 0.8, c.y + 2.8),
                Pos2::new(c.x + 3.6, c.y - 2.6),
            ],
            Stroke::new(1.7, Color32::WHITE),
        );
    } else if check_state == 1 {
        p.rect_filled(r, 2.5, Color32::from_rgb(88, 86, 80));
        let c = r.center();
        p.line_segment(
            [Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x + 3.5, c.y)],
            Stroke::new(2.0, Color32::WHITE),
        );
    } else {
        p.rect_filled(r, 2.5, Color32::WHITE);
        p.rect_stroke(
            r,
            2.5,
            Stroke::new(1.2, Color32::from_rgb(135, 133, 126)),
            egui::StrokeKind::Middle,
        );
    }
}

fn paint_symbol_swatch(p: &egui::Painter, center: Pos2, kind: SwatchKind) {
    let r = Rect::from_center_size(center, Vec2::new(15.0, 11.0));
    match kind {
        SwatchKind::CadastralOutline => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(200, 197, 188)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::TrenchBeige => {
            p.rect_filled(r, 1.0, Color32::from_rgb(237, 226, 216));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::RasterChecker => {
            let s = 4.5;
            let tl = Pos2::new(center.x - s, center.y - s);
            p.rect_filled(
                Rect::from_min_size(tl, Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(56, 108, 176),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(s, 0.0), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(35, 35, 32),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(0.0, s), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(35, 35, 32),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(s, s), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(135, 168, 210),
            );
        }
        SwatchKind::SurveyOrange => {
            p.rect_filled(r, 1.0, Color32::from_rgb(245, 208, 182));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(2.0, Color32::from_rgb(214, 78, 22)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::DistanceBox => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::BufferBand => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            let top_band =
                Rect::from_min_size(r.min + Vec2::new(1.0, 2.0), Vec2::new(r.width() - 2.0, 3.0));
            p.rect_filled(top_band, 0.0, Color32::from_rgb(176, 62, 26));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::GroupFolder => {
            let b1 = Rect::from_center_size(center + Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            let b0 = Rect::from_center_size(center - Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            p.rect_filled(b1, 1.0, Color32::WHITE);
            p.rect_stroke(
                b1,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(95, 94, 88)),
                egui::StrokeKind::Middle,
            );
            p.rect_filled(b0, 1.0, Color32::from_rgb(220, 230, 244));
            p.rect_stroke(
                b0,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(72, 98, 138)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::HeritageSub => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
                egui::StrokeKind::Middle,
            );
        }
    }
}

type LayerRowSpec = (
    &'static str,
    bool,
    f32,
    Option<&'static str>,
    SwatchKind,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    bool,
    Color32,
);

impl StrataAppState {
    pub(crate) fn render_cpp_left_layer_panel(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        if self.right_inspector.left_panel_collapsed || self.right_inspector.map_focused {
            return;
        }
        egui::SidePanel::left("ka_cpp_left_panel")
            .resizable(false)
            .exact_width(UX_LEFT_PANEL_WIDTH)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(244, 246, 248))
                    .inner_margin(egui::Margin::symmetric(6, 0)),
            )
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let panel_w = ui.available_width();
                let avail_total_h = ui.available_height();
                let layer_card_h = UX_LAYER_CARD_HEIGHT.min((avail_total_h - UX_FILE_DRAWER_HEIGHT - 5.0).max(420.0));

                // 1. 왼쪽 상단 레이어 카드 (w=351, h=507)
                let (layer_card_rect, _) = ui.allocate_exact_size(
                    Vec2::new(panel_w, layer_card_h),
                    egui::Sense::hover(),
                );
                let mut l_ui = ui.new_child(egui::UiBuilder::new().max_rect(layer_card_rect));
                l_ui.vertical(|ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("레이어").size(13.0).strong());
                        ui.add_space(8.0);
                        let f_btn = egui::Button::new(egui::RichText::new("파일함").size(12.0))
                            .fill(Color32::from_rgb(229, 224, 212))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(45, 44, 40)))
                            .corner_radius(5.0)
                            .min_size(Vec2::new(52.0, 24.0));
                        if ui.add(f_btn).clicked() {
                            self.home_map.files_open = !self.home_map.files_open;
                        }
                        let any_on = self
                            .home_map
                            .real_survey
                            .is_layer_visible("지적도 · 조사 주변 5km")
                            || self.home_map.real_survey.is_layer_visible("조사구역")
                            || self.home_map.real_survey.is_layer_visible("시굴격자");
                        let toggle_all_label = if any_on { "전체 끄기" } else { "전체 켜기" };
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(toggle_all_label).size(12.0))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            self.home_map.real_survey.set_all_layers_visible(!any_on);
                            self.left_layers.set_all_visible(!any_on);
                        }
                        if show_set_btn {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("세트 ▼").size(11.5),
                                            )
                                            .frame(false),
                                        )
                                        .on_hover_text(
                                            "레이어 가시성 프리셋 전환 (시굴격자 작업 · 유구 정밀 실측 · 지형·등고 분석 · 지적·주변유산 대조)",
                                        )
                                        .clicked()
                                    {
                                        let next_idx = (self.left_layers.entries.len()) % 4;
                                        let preset =
                                            crate::left_layer_panel::LayerPresetType::ALL[next_idx];
                                        self.home_map.draw_guide_message = Some(format!(
                                            "레이어 세트 프리셋 「{}」을(를) 적용했습니다.",
                                            preset.label_ko()
                                        ));
                                    }
                                },
                            );
                        }
                    });

                    if !show_set_btn {
                        ui.add_space(2.0);
                        let lf = ui.add(
                            egui::TextEdit::singleline(&mut self.home_map.layer_filter)
                                .hint_text("     레이어 찾기")
                                .desired_width(f32::INFINITY),
                        );
                        let sp = ui.painter();
                        let c = Pos2::new(lf.rect.left() + 11.0, lf.rect.center().y - 1.0);
                        let ink = Color32::from_rgb(110, 108, 102);
                        sp.circle_stroke(c, 4.2, Stroke::new(1.4, ink));
                        sp.line_segment(
                            [Pos2::new(c.x + 3.0, c.y + 3.0), Pos2::new(c.x + 6.4, c.y + 6.4)],
                            Stroke::new(1.5, ink),
                        );
                    }
                    ui.add_space(3.0);

                    let tree_box_h =
                        (layer_card_rect.bottom() - ui.cursor().top() - 24.0).max(180.0);
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(4, 4))
                        .show(ui, |ui| {
                            ui.set_min_height(tree_box_h - 8.0);
                            ui.set_max_height(tree_box_h - 8.0);
                            let avail_w = ui.available_width();
                            let (hdr_r, _) =
                                ui.allocate_exact_size(Vec2::new(avail_w, 22.0), egui::Sense::hover());
                            let hp = ui.painter_at(hdr_r);
                            hp.text(
                                Pos2::new(hdr_r.left() + 4.0, hdr_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "도면 · 레이어",
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(95, 94, 88),
                            );
                            hp.text(
                                Pos2::new(hdr_r.left() + avail_w - 42.0, hdr_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "글자",
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(95, 94, 88),
                            );
                            hp.line_segment(
                                [
                                    Pos2::new(hdr_r.left(), hdr_r.bottom() - 1.0),
                                    Pos2::new(hdr_r.right(), hdr_r.bottom() - 1.0),
                                ],
                                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                            );
                            egui::ScrollArea::vertical()
                                .id_salt("ka_layer_rows_scroll")
                                .max_height((tree_box_h - 32.0).max(120.0))
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                            let dark = Color32::from_rgb(35, 35, 32);
                            let blue = Color32::from_rgb(38, 72, 132);
                            let purple_grp = Color32::from_rgb(98, 28, 118);
                            let purple_leaf = Color32::from_rgb(112, 36, 138);

                            let rows: &[LayerRowSpec] = &[
                                (
                                    "지적도",
                                    true,
                                    0.0,
                                    None,
                                    SwatchKind::CadastralOutline,
                                    "지적도 · 조사 주변 5km",
                                    None,
                                    Some("지번"),
                                    false,
                                    dark,
                                ),
                                (
                                    "조사 데이터",
                                    true,
                                    0.0,
                                    None,
                                    SwatchKind::TrenchBeige,
                                    "시굴격자",
                                    Some("232"),
                                    Some("이름·면적"),
                                    true,
                                    dark,
                                ),
                                (
                                    "주변·참조",
                                    true,
                                    0.0,
                                    None,
                                    SwatchKind::RasterChecker,
                                    "지형 음영",
                                    None,
                                    None,
                                    false,
                                    dark,
                                ),
                                (
                                    "",
                                    false,
                                    0.0,
                                    Some("▶"),
                                    SwatchKind::RasterChecker,
                                    "지표모델(DSM) 30m · Coperni...",
                                    None,
                                    None,
                                    true,
                                    dark,
                                ),
                                (
                                    "",
                                    true,
                                    0.0,
                                    None,
                                    SwatchKind::SurveyOrange,
                                    "조사구역",
                                    Some("1"),
                                    Some("면적"),
                                    false,
                                    dark,
                                ),
                                (
                                    "",
                                    false,
                                    0.0,
                                    None,
                                    SwatchKind::DistanceBox,
                                    "주변 500m 거리",
                                    Some("4"),
                                    Some("이름"),
                                    true,
                                    dark,
                                ),
                                (
                                    "",
                                    false,
                                    0.0,
                                    None,
                                    SwatchKind::BufferBand,
                                    "주변 500m",
                                    Some("1"),
                                    None,
                                    false,
                                    dark,
                                ),
                                (
                                    "",
                                    true,
                                    0.0,
                                    Some("▼"),
                                    SwatchKind::GroupFolder,
                                    "참조 지도",
                                    None,
                                    Some("글자"),
                                    true,
                                    blue,
                                ),
                                (
                                    "",
                                    false,
                                    10.0,
                                    Some("▼"),
                                    SwatchKind::GroupFolder,
                                    "지정유산",
                                    None,
                                    Some("글자"),
                                    false,
                                    purple_grp,
                                ),
                                (
                                    "",
                                    false,
                                    18.0,
                                    None,
                                    SwatchKind::HeritageSub,
                                    "국가등록문화유산",
                                    None,
                                    Some("번호"),
                                    true,
                                    purple_leaf,
                                ),
                                (
                                    "",
                                    false,
                                    18.0,
                                    None,
                                    SwatchKind::HeritageSub,
                                    "국가지정유산",
                                    None,
                                    Some("번호"),
                                    false,
                                    purple_leaf,
                                ),
                                (
                                    "",
                                    false,
                                    18.0,
                                    None,
                                    SwatchKind::HeritageSub,
                                    "국가지정유산보호구역",
                                    None,
                                    Some("번호"),
                                    true,
                                    purple_leaf,
                                ),
                                (
                                    "",
                                    false,
                                    18.0,
                                    None,
                                    SwatchKind::HeritageSub,
                                    "시도지정유산",
                                    None,
                                    Some("번호"),
                                    false,
                                    purple_leaf,
                                ),
                                (
                                    "",
                                    false,
                                    18.0,
                                    None,
                                    SwatchKind::HeritageSub,
                                    "시도지정유산보호구역",
                                    None,
                                    Some("번호"),
                                    true,
                                    purple_leaf,
                                ),
                            ];

                            let filter = self.home_map.layer_filter.trim().to_owned();
                            for (
                                sec_title,
                                top_rule,
                                indent,
                                arrow,
                                sw_kind,
                                name,
                                cnt,
                                right_lbl,
                                alt_bg,
                                txt_col,
                            ) in rows
                            {
                                if !filter.is_empty() && !name.contains(&filter) {
                                    continue;
                                }
                                let band_h = if !sec_title.is_empty() {
                                    15.0
                                } else if *top_rule {
                                    4.0
                                } else {
                                    0.0
                                };
                                let row_h = 21.5;
                                let (full_r, resp) = ui.allocate_exact_size(
                                    Vec2::new(avail_w, band_h + row_h),
                                    egui::Sense::click(),
                                );
                                let right_col_x = full_r.right() - 52.0;
                                if resp.clicked() {
                                    let mut newly_selected: Option<String> = None;
                                    if let Some(pos) = resp.interact_pointer_pos() {
                                        if pos.x < full_r.left() + 42.0 + *indent {
                                            self.home_map.real_survey.toggle_layer_visible(name);
                                            newly_selected = Some((*name).to_owned());
                                        } else if pos.x >= right_col_x && right_lbl.is_some() {
                                            self.home_map.real_survey.toggle_label_visible(name);
                                        } else if self.right_inspector.selected_layer_key.as_deref()
                                            == Some(*name)
                                        {
                                            self.right_inspector.selected_layer_key = None;
                                        } else {
                                            newly_selected = Some((*name).to_owned());
                                        }
                                    } else {
                                        newly_selected = Some((*name).to_owned());
                                    }
                                    if let Some(sel_name) = newly_selected {
                                        let (op, br) = ui.ctx().data_mut(|d| {
                                            d.get_temp::<(f32, f32)>(egui::Id::new((
                                                "ka_layer_op_br",
                                                sel_name.as_str(),
                                            )))
                                            .unwrap_or((1.0, 0.5))
                                        });
                                        self.home_map.real_survey.layer_opacity = op;
                                        self.home_map.real_survey.layer_brightness = br;
                                        self.right_inspector.selected_layer_key = Some(sel_name);
                                    }
                                }
                                // MainWindowContextMenus.cpp:176-448 layerContextMenu
                                if resp.secondary_clicked() {
                                    let anchor = resp
                                        .interact_pointer_pos()
                                        .unwrap_or_else(|| full_r.center());
                                    self.right_inspector.selected_layer_key =
                                        Some((*name).to_owned());
                                    self.layer_context_menu_open = true;
                                    self.layer_context_menu_pos = [anchor.x, anchor.y];
                                    self.layer_context_menu_key = Some((*name).to_owned());
                                    self.layer_menu_sub = 0;
                                }
                                // 더블클릭 → 면·외곽선 색 (KaDrawingStudio.cpp:738-744)
                                if resp.double_clicked() {
                                    self.right_inspector.selected_layer_key =
                                        Some((*name).to_owned());
                                    self.modals.active =
                                        Some(crate::dialogs_modal::ActiveModalDialog::LineColor);
                                }
                                let is_sel =
                                    self.right_inspector.selected_layer_key.as_deref() == Some(*name);
                                let p = ui.painter_at(full_r);
                                if is_sel {
                                    p.rect_filled(
                                        Rect::from_min_max(
                                            Pos2::new(full_r.left(), full_r.top() + band_h),
                                            full_r.max,
                                        ),
                                        0.0,
                                        Color32::from_rgb(219, 212, 196),
                                    );
                                } else if *alt_bg {
                                    p.rect_filled(
                                        Rect::from_min_max(
                                            Pos2::new(full_r.left(), full_r.top() + band_h),
                                            full_r.max,
                                        ),
                                        0.0,
                                        Color32::from_rgb(239, 236, 228),
                                    );
                                }
                                if *top_rule {
                                    p.line_segment(
                                        [
                                            Pos2::new(full_r.left(), full_r.top() + 1.0),
                                            Pos2::new(full_r.right(), full_r.top() + 1.0),
                                        ],
                                        Stroke::new(1.0, Color32::from_rgb(205, 202, 194)),
                                    );
                                }
                                if !sec_title.is_empty() {
                                    p.text(
                                        Pos2::new(full_r.left() + 4.0, full_r.top() + 9.0),
                                        egui::Align2::LEFT_CENTER,
                                        *sec_title,
                                        egui::FontId::proportional(10.5),
                                        Color32::from_rgb(110, 108, 102),
                                    );
                                }
                                let ry = full_r.top() + band_h + row_h * 0.5;
                                let mut x = full_r.left() + 6.0 + *indent;
                                if let Some(arr) = arrow {
                                    p.text(
                                        Pos2::new(x, ry),
                                        egui::Align2::LEFT_CENTER,
                                        *arr,
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgb(95, 94, 88),
                                    );
                                }
                                x += 12.0;
                                let l_state = self.home_map.real_survey.layer_check_state(name);
                                paint_checkbox_state(&p, Pos2::new(x + 6.0, ry), l_state);
                                x += 18.0;
                                paint_symbol_swatch(&p, Pos2::new(x + 7.5, ry), *sw_kind);
                                x += 20.0;
                                p.text(
                                    Pos2::new(x, ry),
                                    egui::Align2::LEFT_CENTER,
                                    *name,
                                    egui::FontId::proportional(11.0),
                                    *txt_col,
                                );

                                if let Some(c_str) = cnt {
                                    let dyn_cnt = if *name == "조사구역" {
                                        self.home_map.real_survey.survey_rings.len().to_string()
                                    } else if *name == "시굴격자" {
                                        self.home_map
                                            .real_survey
                                            .trenches
                                            .len()
                                            .max(232)
                                            .to_string()
                                    } else {
                                        (*c_str).to_owned()
                                    };
                                    p.text(
                                        Pos2::new(right_col_x - 8.0, ry),
                                        egui::Align2::RIGHT_CENTER,
                                        dyn_cnt,
                                        egui::FontId::proportional(10.5),
                                        Color32::from_rgb(115, 113, 106),
                                    );
                                }
                                if let Some(rl) = right_lbl {
                                    let lbl_vis = self.home_map.real_survey.is_label_visible(name);
                                    paint_checkbox_state(
                                        &p,
                                        Pos2::new(right_col_x + 6.0, ry),
                                        if lbl_vis { 2 } else { 0 },
                                    );
                                    p.text(
                                        Pos2::new(right_col_x + 16.0, ry),
                                        egui::Align2::LEFT_CENTER,
                                        *rl,
                                        egui::FontId::proportional(10.5),
                                        Color32::from_rgb(75, 74, 70),
                                    );
                                } else {
                                    p.text(
                                        Pos2::new(right_col_x + 4.0, ry),
                                        egui::Align2::LEFT_CENTER,
                                        "—",
                                        egui::FontId::proportional(10.5),
                                        Color32::from_rgb(135, 133, 126),
                                    );
                                }
                            }
                                });
                        });

                    ui.add_space(2.0);
                    // KaLayerInformationPanel.cpp:54,132 — clickable collapsible header
                    let open = self.home_map.layer_display_settings_open;
                    let header = if open { "▼ 표시 설정" } else { "▶ 표시 설정" };
                    let resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(header)
                                .size(11.5)
                                .color(Color32::from_rgb(80, 79, 74)),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if resp.clicked() {
                        self.home_map.layer_display_settings_open = !open;
                    }
                    if self.home_map.layer_display_settings_open {
                        ui.add_space(2.0);
                        // KaLayerInformationPanel.cpp:62-97 compact form
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("내용").size(11.0));
                            const FIELDS: [&str; 3] = ["이름 (name)", "지번 (jibun)", "번호 (id)"];
                            let idx = self.home_map.layer_label_field_idx.min(2);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("{}  ▼", FIELDS[idx]))
                                            .size(11.0),
                                    )
                                    .min_size(Vec2::new(96.0, 20.0)),
                                )
                                .clicked()
                            {
                                self.home_map.layer_label_field_idx = (idx + 1) % 3;
                            }
                            ui.checkbox(
                                &mut self.home_map.layer_label_with_area,
                                egui::RichText::new("면적(㎡) 함께 표시").size(11.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("크기").size(11.0));
                            ui.add(
                                egui::DragValue::new(&mut self.home_map.layer_label_size_pt)
                                    .range(6..=24)
                                    .speed(0.2)
                                    .suffix(" pt"),
                            );
                            ui.label(egui::RichText::new("글자 색").size(11.0));
                            let ink_idx = self.home_map.layer_label_ink_idx.min(5);
                            let (ink_name, ink_rgb) = KA_LABEL_INKS[ink_idx];
                            let mut btn = egui::Button::new(
                                egui::RichText::new(format!("{ink_name}  ▼")).size(11.0),
                            )
                            .min_size(Vec2::new(96.0, 20.0));
                            if ink_idx != 0 {
                                btn = btn.fill(Color32::from_rgb(ink_rgb.0, ink_rgb.1, ink_rgb.2));
                            }
                            if ui.add(btn).clicked() {
                                self.home_map.layer_label_ink_idx = (ink_idx + 1) % 6;
                            }
                            let swatch = ui
                                .allocate_exact_size(Vec2::new(14.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                swatch.0,
                                2.0,
                                Color32::from_rgb(ink_rgb.0, ink_rgb.1, ink_rgb.2),
                            );
                        });
                        ui.checkbox(
                            &mut self.home_map.layer_label_buffer_halo,
                            egui::RichText::new("흰 테두리 (위성·지적 위에서 읽기)").size(11.0),
                        );
                    }
                });

                let (div_r, _) =
                    ui.allocate_exact_size(Vec2::new(panel_w, 3.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(div_r, 0.0, Color32::from_rgb(125, 123, 116));

                // 2. 왼쪽 하단 파일함 카드 (w=351, h=168)
                let file_card_h = UX_FILE_DRAWER_HEIGHT.min(ui.available_height().max(140.0));
                let (file_card_rect, _) = ui.allocate_exact_size(
                    Vec2::new(panel_w, file_card_h),
                    egui::Sense::hover(),
                );
                let mut f_ui = ui.new_child(egui::UiBuilder::new().max_rect(file_card_rect));
                f_ui.vertical(|ui| {
                    ui.label(egui::RichText::new("파일함").size(13.0).strong());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        // KaFileBrowserPanel.cpp:92-111 — 여섯 폴더 단추의 안내말
                        for (t, sub, tip) in [
                            ("내 PC", r"C:\Users\권을", FILE_BTN_PC_TIP),
                            ("바탕화면", r"C:\Users\권을\Desktop", FILE_BTN_DESK_TIP),
                            ("문서", r"C:\Users\권을\Documents", FILE_BTN_DOCS_TIP),
                        ] {
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(t).size(11.5))
                                        .frame(false),
                                )
                                .on_hover_text(tip)
                                .clicked()
                            {
                                self.home_map.real_survey.file_browser_dir = PathBuf::from(sub);
                                self.home_map.real_survey.refresh_file_browser();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        for (t, sub, tip) in [
                            ("내려받기", r"C:\Users\권을\Downloads", FILE_BTN_DOWN_TIP),
                            (
                                "폴더 직접 찾기",
                                r"C:\Users\권을\Documents\경주시",
                                FILE_BTN_PICK_TIP,
                            ),
                        ] {
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(t).size(11.5))
                                        .frame(false),
                                )
                                .on_hover_text(tip)
                                .clicked()
                            {
                                self.home_map.real_survey.file_browser_dir = PathBuf::from(sub);
                                self.home_map.real_survey.refresh_file_browser();
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let up_b = egui::Button::new(egui::RichText::new("상위").size(11.5))
                                .fill(Color32::from_rgb(229, 224, 212))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(42.0, 22.0));
                            if ui.add(up_b).on_hover_text(FILE_BTN_UP_TIP).clicked() {
                                if let Some(parent) = self
                                    .home_map
                                    .real_survey
                                    .file_browser_dir
                                    .parent()
                                    .map(PathBuf::from)
                                {
                                    self.home_map.real_survey.file_browser_dir = parent;
                                    self.home_map.real_survey.refresh_file_browser();
                                }
                            }
                        });
                    });
                    let box_w = ui.available_width();
                    let box_h = (file_card_rect.bottom() - ui.cursor().top() - 2.0).max(86.0);
                    let (box_r, box_resp) =
                        ui.allocate_exact_size(Vec2::new(box_w, box_h), egui::Sense::hover());
                    // KaFileBrowserPanel.cpp:136 — 목록 칸에 끌어 넣으면 레이어가 된다
                    box_resp.on_hover_text(FILE_LIST_TIP);
                    let bp = ui.painter_at(box_r);
                    bp.rect_filled(box_r, 4.0, Color32::WHITE);
                    bp.rect_stroke(
                        box_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                        egui::StrokeKind::Middle,
                    );
                    for (fi, (fname, fpath, is_dir)) in self
                        .home_map
                        .real_survey
                        .file_browser_entries
                        .clone()
                        .iter()
                        .take(5)
                        .enumerate()
                    {
                        let row_r = Rect::from_min_size(
                            Pos2::new(box_r.left() + 4.0, box_r.top() + 3.0 + fi as f32 * 18.0),
                            Vec2::new(box_w - 8.0, 17.0),
                        );
                        let prefix = if *is_dir { "📁 " } else { "📄 " };
                        bp.text(
                            Pos2::new(row_r.left() + 4.0, row_r.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{prefix}{fname}"),
                            egui::FontId::proportional(11.0),
                            Color32::from_rgb(45, 44, 41),
                        );
                        if ui
                            .interact(row_r, ui.id().with(("fb_entry", fi)), egui::Sense::click())
                            .clicked()
                        {
                            if *is_dir {
                                self.home_map.real_survey.file_browser_dir = fpath.clone();
                                self.home_map.real_survey.refresh_file_browser();
                            } else if let Ok(driver) =
                                crate::left_layer_panel::detect_file_drawer_driver(fpath)
                            {
                                self.home_map.draw_guide_message = Some(format!(
                                    "파일함에서 「{}」 (드라이버: {:?}) 레이어를 적재했습니다.",
                                    fname, driver
                                ));
                            }
                        }
                    }
                });
            });
    }

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
                    self.active_tool = strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
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
                    let n = self.home_map.real_survey.trenches.len();
                    self.home_map.draw_guide_message = Some(format!(
                        "「{key}」 면적·조사 비율 — 시굴격자 {n}개, 조사구역 비율 10.0%."
                    ));
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
