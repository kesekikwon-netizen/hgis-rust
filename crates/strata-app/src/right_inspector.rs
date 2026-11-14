//! 오른쪽 검사기(속성 카드·스타일 카드).
//!
//! 원래 1,079줄 한 파일이던 것을 300줄 규칙에 맞춰 나눴다.
//! 기존 `crate::right_inspector::<이름>` 경로는 그대로 동작한다.

use eframe::egui::{self, Color32};

use crate::app_state::StrataAppState;



pub use crate::ri_types::{KA_STYLE_PALETTE, HGIS_DEFAULT_ELEVATION_VAL, HGIS_MIN_ELEVATION_M,
    HGIS_MAX_ELEVATION_M, KA_BUILTIN_KINDS, KA_BUILTIN_PERIODS,
    KaPeriodInk, AttributeSaveResult, BatchUpdateResult, InspectorCardTab, RightInspectorState};

impl RightInspectorState {
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            active_card: InspectorCardTab::FeatureAttributeCard,
            selected_layer_key: None,
            selected_feature_id: Some(1),
            feature_no: "1호 주거지".to_owned(),
            feature_type: "주거지".to_owned(),
            style_color_index: 1,
            no_fill: false,
            no_stroke: false,
            stroke_width_mm: 1.5,
            fill_opacity: 0.35,
            show_draw_tooltip: false,
            left_panel_collapsed: false,
            right_panel_collapsed: false,
            map_focused: false,
            feature_name: "1호 주거지".to_owned(),
            site_name: "경주 월성북편 유적".to_owned(),
            control_point_id: "CP-01".to_owned(),
            control_x: 219_897.093,
            control_y: 361_622.354,
            control_z: 48.50,
            section_id: "A-A'".to_owned(),
            control_csv_path: String::new(),
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
                    .fill(Color32::from_rgb(245, 244, 237))
                    .inner_margin(egui::Margin::symmetric(8, 2)),
            )
            .show_inside(ui, |ui| {
            self.render_ri_body(ui);
            });
    }
}
