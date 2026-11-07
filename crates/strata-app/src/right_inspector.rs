use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_net::tile_fetcher::BasemapSource;

use crate::app_state::StrataAppState;

/// C++ `KaStylePalette.h` 8대 표준 제도 색상 (빨, 주, 노, 초, 파, 남, 보, 갈)
pub const KA_STYLE_PALETTE: [(u8, u8, u8, &str); 8] = [
    (0xDC, 0x26, 0x26, "빨강"),
    (0xEA, 0x58, 0x0C, "주황"),
    (0xCA, 0x8A, 0x04, "노랑"),
    (0x16, 0xA3, 0x4A, "초록"),
    (0x25, 0x63, 0xEB, "파랑"),
    (0x1E, 0x3A, 0x8A, "남색"),
    (0x7C, 0x3A, 0xED, "보라"),
    (0x92, 0x40, 0x0E, "갈색"),
];

pub const HGIS_DEFAULT_ELEVATION_VAL: f64 = 0.0;
pub const HGIS_MIN_ELEVATION_M: f64 = -500.0;
pub const HGIS_MAX_ELEVATION_M: f64 = 9000.0;

pub const HGIS_STANDARD_PERIOD_CODES: [&str; 11] = [
    "구석기",
    "신석기",
    "청동기",
    "초기철기",
    "원삼국",
    "삼국",
    "통일신라",
    "고려",
    "조선",
    "근현대",
    "미상",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeSaveResult {
    Success,
    MissingRequiredField,
    InvalidNumericElevation,
}

/// C++ `FeaturePresets.cpp:19-44` — 기본 유구 종류 8종
pub const KA_BUILTIN_KINDS: [&str; 8] = [
    "주거지", "수혈", "구", "분묘", "주초", "가마", "수전", "기타",
];

/// C++ `FeaturePresets.cpp:19-44` — 기본 시대 10종과 시대 띠 색
/// 시대 띠 색 (r,g,b). None 이면 색 없음(미상).
pub type KaPeriodInk = Option<(u8, u8, u8)>;

pub const KA_BUILTIN_PERIODS: [(&str, KaPeriodInk); 10] = [
    ("구석기", Some((0x53, 0x29, 0x1B))),
    ("신석기", Some((0x67, 0x40, 0x28))),
    ("청동기", Some((0x7A, 0x57, 0x37))),
    ("초기철기", Some((0x8B, 0x70, 0x48))),
    ("원삼국", Some((0x9C, 0x8A, 0x5B))),
    ("삼국", Some((0xAB, 0xA5, 0x71))),
    ("통일신라", Some((0xBA, 0xC0, 0x8A))),
    ("고려", Some((0xC7, 0xDC, 0xA6))),
    ("조선", Some((0xD3, 0xF8, 0xC4))),
    ("미상", None),
];

/// C++ `KaFeatureCardEditors.cpp:157-170` — "다음 번호" 제안.
#[must_use]
pub fn next_feature_number(current: &str) -> String {
    let digits: String = current.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = &current[digits.len()..];
    let n = digits.parse::<u32>().unwrap_or(1);
    format!("{}{}", n + 1, rest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchUpdateResult {
    pub updated_count: usize,
    pub failed_count: usize,
    pub failed_fids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorCardTab {
    #[default]
    FeatureAttributeCard,
    LayerStyleCard,
    BasemapQuickCard,
}

#[derive(Debug, Clone, Default)]
pub struct RightInspectorState {
    pub active_card: InspectorCardTab,
    pub selected_layer_key: Option<String>,
    pub selected_feature_id: Option<i64>,
    pub survey_name: String,
    pub feature_no: String,
    pub feature_type: String,
    pub feature_period: String,
    pub feature_memo: String,
    pub survey_card_modal_open: bool,
    pub style_color_index: usize,
    pub no_fill: bool,
    pub no_stroke: bool,
    pub stroke_width_mm: f32,
    pub fill_opacity: f32,
    pub show_draw_tooltip: bool,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub map_focused: bool,
}

impl RightInspectorState {
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            active_card: InspectorCardTab::FeatureAttributeCard,
            selected_layer_key: Some("조사구역".to_owned()),
            selected_feature_id: Some(1),
            survey_name: "경주 월성북편 시굴조사".to_owned(),
            feature_no: "1호 주거지".to_owned(),
            feature_type: "주거지".to_owned(),
            feature_period: "삼국".to_owned(),
            feature_memo: "경주 인왕동 76번지 일원 지표·시굴조사 구역 및 유구 기록".to_owned(),
            survey_card_modal_open: false,
            style_color_index: 1,
            no_fill: false,
            no_stroke: false,
            stroke_width_mm: 1.5,
            fill_opacity: 0.35,
            show_draw_tooltip: false,
            left_panel_collapsed: false,
            right_panel_collapsed: false,
            map_focused: false,
        }
    }

    #[must_use]
    pub fn active_stroke_color(&self) -> Color32 {
        let (r, g, b, _) = KA_STYLE_PALETTE[self.style_color_index.min(7)];
        Color32::from_rgb(r, g, b)
    }

    /// C++ `KaFeatureCard::bindFeature` 면적(`#define PYEONG_FACTOR 3.305785`) 및 둘레 포맷팅 1:1
    #[must_use]
    pub fn format_bound_ring_metrics(ring: &[[f64; 2]]) -> (String, String) {
        if ring.len() < 3 {
            return ("0.00 ㎡ (약 0.0 평)".to_owned(), "0.00 m".to_owned());
        }
        let area_m2 = crate::real_survey::ring_area_m2(ring);
        let pyeong = area_m2 / 3.305_785;
        let perim_m = crate::real_survey::ring_perimeter_m(ring);
        (
            format!("{area_m2:.2} ㎡ (약 {pyeong:.1} 평)"),
            format!("{perim_m:.2} m"),
        )
    }

    /// C++ `KaFeatureCard::saveCurrentAttributes() -> bool` 필수 항목(`FEAT_NO`, `FEAT_TYPE`) 검증 1:1
    #[must_use]
    pub fn save_current_attributes(&self) -> bool {
        !self.feature_no.trim().is_empty() && !self.feature_type.trim().is_empty()
    }

    /// C++ `KaAttributeInspector::saveAttributes` — 숫자형 표고/깊이 파싱 및 NaN/Infinity/범위(`[-500.0, 9000.0]`) 이탈 시 기존 유효값 복원
    #[must_use]
    pub fn validate_and_parse_elevation(raw: &str, previous_valid: f64) -> (f64, bool) {
        if let Ok(val) = raw.trim().parse::<f64>() {
            if val.is_finite() && (HGIS_MIN_ELEVATION_M..=HGIS_MAX_ELEVATION_M).contains(&val) {
                return (val, true);
            }
        }
        (previous_valid, false)
    }

    /// C++ `KaAttributeInspector::applyBatchAttributeUpdate` — 다중 선택 피처 속성 일괄 트랜잭션 수정
    pub fn apply_batch_attribute_update(
        fids: &[i64],
        feature_table: &mut std::collections::BTreeMap<
            i64,
            std::collections::BTreeMap<String, String>,
        >,
        changes: &std::collections::BTreeMap<String, String>,
    ) -> BatchUpdateResult {
        if fids.is_empty() {
            return BatchUpdateResult {
                updated_count: 0,
                failed_count: 0,
                failed_fids: Vec::new(),
            };
        }
        let mut updated_count = 0usize;
        let mut failed_fids = Vec::new();
        for &fid in fids {
            if let Some(attrs) = feature_table.get_mut(&fid) {
                for (k, v) in changes {
                    attrs.insert(k.clone(), v.clone());
                }
                updated_count += 1;
            } else {
                failed_fids.push(fid);
            }
        }
        BatchUpdateResult {
            updated_count,
            failed_count: failed_fids.len(),
            failed_fids,
        }
    }
}

impl StrataAppState {
    /// C++ `KaInspectorPanel.cpp` + `KaInspectorStyle.cpp` + `KaStylePalette.h` 1:1 재현 (Images 2, 16)
    pub(crate) fn render_cpp_right_inspector(&mut self, ui: &mut egui::Ui) {
        if self.right_inspector.right_panel_collapsed {
            return;
        }
        egui::SidePanel::right("ka_map_right_inspector")
            .resizable(false)
            .exact_width(crate::gui_tab_map::UX_MAP_RIGHT_WIDTH)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(244, 246, 248))
                    .inner_margin(egui::Margin::symmetric(8, 2)),
            )
            .show_inside(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("선택한 유구").size(13.5).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = egui::Button::new("›")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(22.0, 22.0));
                        if ui.add(b).on_hover_text("오른쪽 패널 접기 (F10)").clicked() {
                            self.right_inspector.right_panel_collapsed = true;
                        }
                    });
                });
                ui.add_space(4.0);

                let is_attr = self.right_inspector.active_card != InspectorCardTab::LayerStyleCard;
                ui.columns(2, |cols| {
                    for (ci, (lbl, active, target)) in [
                        ("속성", is_attr, InspectorCardTab::FeatureAttributeCard),
                        ("스타일", !is_attr, InspectorCardTab::LayerStyleCard),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let (tr, resp) = cols[ci].allocate_exact_size(
                            Vec2::new(cols[ci].available_width(), 24.0),
                            egui::Sense::click(),
                        );
                        let tp = cols[ci].painter_at(tr);
                        tp.text(
                            tr.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(12.5),
                            if active {
                                Color32::from_rgb(30, 30, 28)
                            } else {
                                Color32::from_rgb(105, 103, 96)
                            },
                        );
                        let line_col = if active {
                            Color32::from_rgb(35, 35, 32)
                        } else {
                            Color32::from_rgb(212, 209, 200)
                        };
                        let line_w = if active { 2.0 } else { 1.0 };
                        tp.line_segment(
                            [
                                Pos2::new(tr.left(), tr.bottom()),
                                Pos2::new(tr.right(), tr.bottom()),
                            ],
                            Stroke::new(line_w, line_col),
                        );
                        if resp.clicked() {
                            self.right_inspector.active_card = target;
                        }
                    }
                });
                ui.add_space(4.0);

                if is_attr {
                    let active_ring = self
                        .home_map
                        .selected_feature_idx
                        .and_then(|idx| self.home_map.real_survey.survey_rings.get(idx))
                        .or_else(|| self.home_map.real_survey.survey_rings.last());
                    if let Some(ring) = active_ring {
                        let area_m2 = crate::real_survey::ring_area_m2(ring);
                        let pyeong = area_m2 / 3.305_785;
                        let perim_m = crate::real_survey::ring_perimeter_m(ring);
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(245, 243, 236))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(212, 209, 200)))
                            .corner_radius(5.0)
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "면적: {area_m2:.2} m² ({pyeong:.1} 평)"
                                    ))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_rgb(30, 30, 28)),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "둘레: {perim_m:.2} m · {}",
                                        self.work_crs.auth_id()
                                    ))
                                    .size(11.5)
                                    .color(Color32::from_rgb(80, 79, 74)),
                                );
                            });
                        ui.add_space(6.0);
                        egui::Grid::new("ka_feature_card_grid")
                            .num_columns(2)
                            .spacing([6.0, 5.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("조사명").size(11.5));
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(
                                            &mut self.right_inspector.survey_name,
                                        )
                                        .desired_width(155.0),
                                    )
                                    .changed()
                                {
                                    self.is_dirty = true;
                                }
                                ui.end_row();
                                ui.label(egui::RichText::new("유구번호").size(11.5));
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.right_inspector.feature_no,
                                            )
                                            .desired_width(96.0),
                                        )
                                        .changed()
                                    {
                                        self.is_dirty = true;
                                    }
                                    // KaFeatureCardEditors.cpp:157-170
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("다음 번호").size(11.0),
                                            )
                                            .min_size(Vec2::new(50.0, 20.0)),
                                        )
                                        .on_hover_text("지금까지 쓴 번호 다음 값을 넣습니다.")
                                        .clicked()
                                    {
                                        let next =
                                            next_feature_number(&self.right_inspector.feature_no);
                                        self.right_inspector.feature_no = next;
                                        self.is_dirty = true;
                                    }
                                });
                                ui.end_row();
                                ui.label(egui::RichText::new("종류").size(11.5));
                                egui::ComboBox::from_id_salt("ka_feat_type_combo")
                                    .selected_text(&self.right_inspector.feature_type)
                                    .width(155.0)
                                    .show_ui(ui, |ui| {
                                        for ft in [
                                            "조사구역",
                                            "주거지",
                                            "수혈",
                                            "분묘",
                                            "구",
                                            "주초",
                                            "유물포함층",
                                        ] {
                                            if ui
                                                .selectable_value(
                                                    &mut self.right_inspector.feature_type,
                                                    ft.to_owned(),
                                                    ft,
                                                )
                                                .changed()
                                            {
                                                self.is_dirty = true;
                                            }
                                        }
                                    });
                                ui.end_row();
                                ui.label(egui::RichText::new("시대").size(11.5));
                                egui::ComboBox::from_id_salt("ka_feat_period_combo")
                                    .selected_text(&self.right_inspector.feature_period)
                                    .width(155.0)
                                    .show_ui(ui, |ui| {
                                        for pd in [
                                            "미상",
                                            "구석기",
                                            "신석기",
                                            "청동기",
                                            "원삼국",
                                            "삼국",
                                            "통일신라",
                                            "고려",
                                            "조선",
                                        ] {
                                            if ui
                                                .selectable_value(
                                                    &mut self.right_inspector.feature_period,
                                                    pd.to_owned(),
                                                    pd,
                                                )
                                                .changed()
                                            {
                                                self.is_dirty = true;
                                            }
                                        }
                                    });
                                ui.end_row();
                            });
                            // KaFeatureCardEditors.cpp:38-75 — 10칸 시대 띠 (168x13)
                            {
                                let (band_r, _) = ui.allocate_exact_size(
                                    Vec2::new(168.0, 13.0),
                                    egui::Sense::hover(),
                                );
                                let bp = ui.painter_at(band_r);
                                bp.rect_stroke(
                                    band_r,
                                    2.0,
                                    Stroke::new(1.0, Color32::from_rgb(31, 35, 40)),
                                    egui::StrokeKind::Middle,
                                );
                                let cell_w = 168.0 / KA_BUILTIN_PERIODS.len() as f32;
                                let sel_idx = KA_BUILTIN_PERIODS
                                    .iter()
                                    .position(|(n, _)| *n == self.right_inspector.feature_period);
                                for (i, (nm, rgb)) in KA_BUILTIN_PERIODS.iter().enumerate() {
                                    let x0 = band_r.left() + cell_w * (i as f32);
                                    let cell = Rect::from_min_size(
                                        Pos2::new(x0 + 1.0, band_r.top() + 2.0),
                                        Vec2::new(cell_w - 2.0, 8.0),
                                    );
                                    let col = match rgb {
                                        Some((r, g, b)) => Color32::from_rgb(*r, *g, *b),
                                        None => Color32::from_rgb(230, 228, 220),
                                    };
                                    let cell_resp = ui.interact(
                                        cell,
                                        egui::Id::new(("ka_period_band", i)),
                                        egui::Sense::click(),
                                    );
                                    bp.rect_filled(cell, 1.0, col);
                                    if sel_idx == Some(i) {
                                        bp.line_segment(
                                            [
                                                Pos2::new(x0 + 2.0, band_r.bottom() - 3.0),
                                                Pos2::new(x0 + cell_w - 2.0, band_r.bottom() - 3.0),
                                            ],
                                            Stroke::new(2.0, Color32::from_rgb(31, 35, 40)),
                                        );
                                    }
                                    if cell_resp.on_hover_text(*nm).clicked() {
                                        self.right_inspector.feature_period = (*nm).to_owned();
                                        self.is_dirty = true;
                                    }
                                }
                            }
                        ui.add_space(4.0);
                        let (pit_depth, pit_caption) =
                            strata_terrain3d::trench_profile3d::compute_pit_elevation_depth_and_caption(
                                Some(48.50),
                                Some(47.20),
                                None,
                            );
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(245, 240, 232))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(212, 204, 191)))
                            .corner_radius(5.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("단면 표고 (KaElevationFigure)")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_rgb(107, 78, 61)),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "상면 EL. 48.50 · 바닥 EL. 47.20 · 깊이 {:.2} m",
                                        pit_depth.unwrap_or(0.0)
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_rgb(47, 42, 38)),
                                )
                                .on_hover_text(pit_caption);
                            });
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("메모").size(11.5));
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut self.right_inspector.feature_memo)
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed()
                        {
                            self.is_dirty = true;
                        }
                        ui.add_space(4.0);
                        // KaFeatureCard.cpp:240-244,298 — 저장 상태 칩
                        if self.is_dirty {
                            ui.label(
                                egui::RichText::new("● 변경됨 · 아직 저장 안 됨")
                                    .size(11.0)
                                    .color(Color32::from_rgb(181, 87, 58)),
                            )
                            .on_hover_text("저장(Ctrl+S)으로 파일에 씁니다.");
                        } else {
                            ui.label(
                                egui::RichText::new("● 저장된 값입니다")
                                    .size(11.0)
                                    .color(Color32::from_rgb(105, 103, 96)),
                            );
                        }
                        let card_btn = egui::Button::new(
                            egui::RichText::new("조사카드 열기")
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(5.0)
                        .min_size(Vec2::new(ui.available_width(), 28.0));
                        if ui
                            .add(card_btn)
                            .on_hover_text("이 유구의 전체 조사카드(모든 칸)를 창으로 엽니다.")
                            .clicked()
                        {
                            self.right_inspector.survey_card_modal_open = true;
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "지도에서 도형 하나를 고르면 기록이\n여기에 나옵니다.",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                } else {
                    let layer_name = self
                        .right_inspector
                        .selected_layer_key
                        .as_deref()
                        .unwrap_or("조사구역");
                    let stroke_col = self.right_inspector.active_stroke_color();
                    ui.horizontal(|ui| {
                        let (sr, _) =
                            ui.allocate_exact_size(Vec2::new(16.0, 12.0), egui::Sense::hover());
                        let sp = ui.painter_at(sr);
                        sp.rect_filled(
                            sr,
                            1.0,
                            Color32::from_rgba_unmultiplied(
                                stroke_col.r(),
                                stroke_col.g(),
                                stroke_col.b(),
                                70,
                            ),
                        );
                        sp.rect_stroke(
                            sr,
                            1.0,
                            Stroke::new(2.0, stroke_col),
                            egui::StrokeKind::Middle,
                        );
                        ui.label(
                            egui::RichText::new(layer_name)
                                .size(12.5)
                                .color(Color32::from_rgb(35, 35, 32)),
                        );
                    });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("색")
                            .size(12.0)
                            .color(Color32::from_rgb(80, 79, 74)),
                    );
                    ui.add_space(2.0);
                    egui::Grid::new("ka_inspector_style_palette")
                        .spacing([5.0, 5.0])
                        .show(ui, |ui| {
                            for (idx, (r, g, b, name)) in KA_STYLE_PALETTE.iter().enumerate() {
                                let (rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(46.0, 24.0),
                                    egui::Sense::click(),
                                );
                                let p = ui.painter_at(rect);
                                let col = Color32::from_rgb(*r, *g, *b);
                                p.rect_filled(rect, 4.0, col);
                                let chosen = self.right_inspector.style_color_index == idx;
                                let border = if chosen {
                                    Stroke::new(2.6, Color32::from_rgb(28, 28, 26))
                                } else {
                                    Stroke::new(1.0, Color32::from_rgb(175, 172, 162))
                                };
                                p.rect_stroke(rect, 4.0, border, egui::StrokeKind::Middle);
                                if resp.on_hover_text(*name).clicked() {
                                    self.right_inspector.style_color_index = idx;
                                }
                                if idx % 4 == 3 {
                                    ui.end_row();
                                }
                            }
                        });
                    ui.add_space(6.0);
                    let mut nf = self.right_inspector.no_fill;
                    if ui
                        .add(egui::Checkbox::new(&mut nf, "채우기 없음 (선만)"))
                        .changed()
                    {
                        self.right_inspector.no_fill = nf;
                        if nf {
                            self.right_inspector.no_stroke = false;
                        }
                    }
                    let mut ns = self.right_inspector.no_stroke;
                    if ui
                        .add(egui::Checkbox::new(&mut ns, "선 없음"))
                        .changed()
                    {
                        self.right_inspector.no_stroke = ns;
                        if ns {
                            self.right_inspector.no_fill = false;
                        }
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("외곽선 굵기")
                                .size(12.0)
                                .color(Color32::from_rgb(80, 79, 74)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // KaInspectorStyle.cpp:87-94 — QDoubleSpinBox 0.2..=12.0
                            ui.add(
                                egui::DragValue::new(&mut self.right_inspector.stroke_width_mm)
                                    .range(0.2..=12.0)
                                    .speed(0.2)
                                    .fixed_decimals(1)
                                    .suffix(" mm"),
                            );
                        });
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (lbl, src) in [
                            ("위성", BasemapSource::VworldSatellite),
                            ("지형", BasemapSource::VworldBase),
                            ("옛 지도 ▼", BasemapSource::Daedongyeojido),
                        ] {
                            let btn =
                                egui::Button::new(egui::RichText::new(lbl).size(12.0).strong())
                                    .fill(Color32::WHITE)
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                    .corner_radius(5.0)
                                    .min_size(Vec2::new(66.0, 30.0));
                            if ui.add(btn).clicked() {
                                self.home_map.active_basemap = src;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("배경 지도")
                            .size(11.5)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.separator();
                });
            });

        if self.right_inspector.survey_card_modal_open {
            let mut open = self.right_inspector.survey_card_modal_open;
            let mut close_requested = false;
            egui::Window::new("조사카드 상세 기록 — Strata")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(380.0)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "조사명: {} · 유구번호: {}",
                            self.right_inspector.survey_name, self.right_inspector.feature_no
                        ))
                        .size(13.0)
                        .strong(),
                    );
                    ui.label(format!(
                        "종류: {} · 시대: {}",
                        self.right_inspector.feature_type, self.right_inspector.feature_period
                    ));
                    ui.separator();
                    ui.label("현장 조사 메모 및 층위·출토유물 기록:");
                    if ui
                        .add(
                            egui::TextEdit::multiline(&mut self.right_inspector.feature_memo)
                                .desired_rows(5)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        self.is_dirty = true;
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let save_btn = egui::Button::new(
                            egui::RichText::new("조사카드 저장")
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(5.0);
                        if ui.add(save_btn).clicked() {
                            self.home_map.draw_guide_message = Some(format!(
                                "「{}」 조사카드를 저장했습니다.",
                                self.right_inspector.feature_no
                            ));
                            self.is_dirty = false;
                            close_requested = true;
                        }
                        if ui.button("닫기").clicked() {
                            close_requested = true;
                        }
                    });
                });
            self.right_inspector.survey_card_modal_open = open && !close_requested;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // D9 KaFeatureCardEditors / KaFeatureCard / KaInspectorStyle
    #[test]
    fn test_d9_feature_card_period_band_next_number_and_style_drag() {
        assert_eq!(KA_BUILTIN_PERIODS.len(), 10);
        let names: Vec<&str> = KA_BUILTIN_PERIODS.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            names,
            vec![
                "구석기", "신석기", "청동기", "초기철기", "원삼국", "삼국", "통일신라",
                "고려", "조선", "미상"
            ]
        );
        assert_eq!(KA_BUILTIN_PERIODS[0].1, Some((0x53, 0x29, 0x1B)));
        assert_eq!(KA_BUILTIN_PERIODS[9].1, None);
        assert_eq!(
            KA_BUILTIN_PERIODS.iter().filter(|(_, c)| c.is_some()).count(),
            9
        );
        assert_eq!(KA_BUILTIN_KINDS.len(), 8);
        assert_eq!(KA_BUILTIN_KINDS[0], "주거지");
        assert_eq!(KA_BUILTIN_KINDS[7], "기타");
        assert_eq!(next_feature_number("1호 주거지"), "2호 주거지");
        assert_eq!(next_feature_number("12호 수혈"), "13호 수혈");
        assert_eq!(next_feature_number(""), "2");
        assert_eq!(next_feature_number("3"), "4");
        let d = RightInspectorState::new_default();
        assert!((0.2..=12.0).contains(&d.stroke_width_mm));
    }

    #[test]
    fn test_feature_card_area_calculation_10x20_rect_formats_exact_200m2_605_pyeong_and_60m_perim()
    {
        let rect_10x20 = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 20.0],
            [0.0, 20.0],
            [0.0, 0.0],
        ];
        let (area_label, perim_label) = RightInspectorState::format_bound_ring_metrics(&rect_10x20);
        assert_eq!(area_label, "200.00 ㎡ (약 60.5 평)");
        assert_eq!(perim_label, "60.00 m");

        let mut card = RightInspectorState::new_default();
        assert!(card.save_current_attributes());
        card.feature_no.clear();
        assert!(!card.save_current_attributes());
    }

    #[test]
    fn test_batch_period_update_5_features_and_elevation_nan_revert() {
        let mut table = std::collections::BTreeMap::new();
        for fid in 1..=5_i64 {
            let mut attrs = std::collections::BTreeMap::new();
            attrs.insert("FEAT_NO".to_owned(), format!("유구-{fid}"));
            attrs.insert("PERIOD".to_owned(), "미상".to_owned());
            table.insert(fid, attrs);
        }
        let mut changes = std::collections::BTreeMap::new();
        changes.insert("PERIOD".to_owned(), "삼국".to_owned());

        let res = RightInspectorState::apply_batch_attribute_update(
            &[1, 2, 3, 4, 5],
            &mut table,
            &changes,
        );
        assert_eq!(res.updated_count, 5);
        assert_eq!(res.failed_count, 0);
        for fid in 1..=5_i64 {
            assert_eq!(table[&fid]["PERIOD"], "삼국");
        }

        let (val_ok, ok) = RightInspectorState::validate_and_parse_elevation("48.25", 12.0);
        assert!(ok);
        assert!((val_ok - 48.25).abs() < 1e-9);

        let (val_reverted, ok_nan) =
            RightInspectorState::validate_and_parse_elevation("NaN", 48.25);
        assert!(!ok_nan);
        assert!((val_reverted - 48.25).abs() < 1e-9);
    }
}
