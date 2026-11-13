//! 「조사구역 그리기 설정」 — C++ `KaSurveyAreaDialog`(KaSurveyAreaDialog.cpp)의 값과 이어 그릴 조사구역 고르기
//! `layerToContinue`(:23-45), 확인 뒤 `LayerOps::createSurveyAreaLayer`(LayerOps.cpp:1446-1532)와
//! `startEditSurveyArea`(MainWindowEditing.cpp:1100-1126). 창 모양은 `dlg_survey_area.rs`.

use strata_storage::edit_buffer::LayerEditBuffer;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::{DomainLayerKey, LegendGroup};

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::survey_style::DomainStyle;

/// `KaStylePalette::colors()`·`names()`(KaStylePalette.h:11-19).
pub(crate) const PALETTE: [([u8; 3], &str); 8] = [
    ([0xDC, 0x26, 0x26], "빨강"),
    ([0xEA, 0x58, 0x0C], "주황"),
    ([0xCA, 0x8A, 0x04], "노랑"),
    ([0x16, 0xA3, 0x4A], "초록"),
    ([0x25, 0x63, 0xEB], "파랑"),
    ([0x1E, 0x3A, 0x8A], "남색"),
    ([0x7C, 0x3A, 0xED], "보라"),
    ([0x92, 0x40, 0x0E], "갈색"),
];
/// 새 모양: 색 단추는 한 줄에 넷(colorGrid i / 4, i % 4).
pub(crate) const SWATCHES_PER_ROW: usize = 4;
/// 선 굵기 미리 정한 단추(:110-111).
pub(crate) const WIDTH_PRESETS: [(&str, f64); 4] = [("보통 1.0", 1.0), ("기본 1.5", 1.5), ("굵게 2.0", 2.0), ("강조 3.0", 3.0)];

/// 창에 적은 값.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SurveyAreaSetup {
    pub(crate) name: String,
    pub(crate) color: usize,
    pub(crate) width_mm: f64,
}

impl Default for SurveyAreaSetup {
    /// 이름 「조사구역」, 첫 조사구역은 주황(selectColorIndex(1)), 굵기 1.5 mm.
    fn default() -> Self {
        Self { name: "조사구역".to_owned(), color: 1, width_mm: 1.5 }
    }
}

impl SurveyAreaSetup {
    /// `layerName` — 비우면 「조사구역」.
    pub(crate) fn layer_name(&self) -> String {
        let typed = self.name.trim();
        if typed.is_empty() { "조사구역".to_owned() } else { typed.to_owned() }
    }

    pub(crate) fn color_name(&self) -> &'static str {
        PALETTE[self.color].1
    }

    /// 굵기가 미리 정한 값과 같으면 그 단추(직접 바꾸면 모두 풀림).
    pub(crate) fn chosen_preset(&self) -> Option<usize> {
        WIDTH_PRESETS.iter().position(|(_, w)| (*w - self.width_mm).abs() < 1e-9)
    }

    /// 선 색 그대로, 채움은 같은 색 알파 50(selectColorIndex), 굵기 mm(applySimpleVectorStyle).
    pub(crate) fn style(&self) -> DomainStyle {
        let [r, g, b] = PALETTE[self.color].0;
        DomainStyle { fill: [r, g, b, 50], stroke: [r, g, b, 255], stroke_width_mm: self.width_mm as f32, marker_size_mm: 3.5, casing: None }
    }
}

fn is_survey_area_key(key: &str) -> bool {
    key == "survey_area" || key.starts_with("survey_area_")
}

impl StrataAppState {
    /// `layerToContinue` — 이 조사 파일의 조사구역 레이어 가운데 범례에서 고른 것, 아니면 처음 보이는 것.
    pub(crate) fn survey_area_layer_to_continue(&self) -> Option<(String, String)> {
        let own: Vec<&LayerTreeEntry> = self
            .left_layers
            .entries
            .iter()
            .filter(|e| is_survey_area_key(&e.layer_key) && self.survey_tables.iter().any(|t| t.table == e.layer_key))
            .collect();
        let current = self.right_inspector.selected_layer_key.as_deref();
        own.iter()
            .find(|e| Some(e.label_ko.as_str()) == current)
            .or_else(|| own.iter().find(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko)))
            .map(|e| (e.layer_key.clone(), e.label_ko.clone()))
    }

    /// `createSurveyAreaLayer` — 범례가 쓰지 않는 빈 survey_area 표가 있으면 그것, 아니면 survey_area_N 새 표.
    fn create_survey_area_layer(&mut self, setup: &SurveyAreaSetup) -> Result<String, String> {
        let path = self.survey_path.clone().filter(|p| p.exists()).ok_or("먼저 「새 조사」로 저장 경로를 만드세요.")?;
        let used: Vec<String> =
            self.left_layers.entries.iter().filter(|e| is_survey_area_key(&e.layer_key)).map(|e| e.layer_key.clone()).collect();
        let empty_default = self
            .survey_tables
            .iter()
            .any(|t| t.table == "survey_area" && t.baseline.is_empty() && t.layer.features.is_empty());
        let table = if !used.iter().any(|k| k == "survey_area") && empty_default {
            "survey_area".to_owned()
        } else {
            // 이미 있는 표(파일에 있는 것 포함)는 덮어쓰지 않는다.
            let taken = |name: &str| used.iter().any(|k| k == name) || self.survey_tables.iter().any(|t| t.table == name);
            let table = (2..).map(|n| format!("survey_area_{n}")).find(|n| !taken(n)).unwrap_or_default();
            let srs = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
            strata_storage::survey_gpkg_add_table::add_survey_area_table(&path, &table, srs)?;
            let fresh = strata_storage::survey_gpkg_read::read_survey_tables(&path).map_err(|e| e.to_string())?;
            let mut t: SurveyFileTable = fresh.into_iter().find(|t| t.table == table).ok_or("새 표를 읽지 못했습니다.")?;
            t.layer = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
            self.survey_tables.push(t);
            table
        };
        let name = setup.layer_name();
        self.left_layers.entries.retain(|e| e.layer_key != table);
        self.left_layers.entries.insert(
            0,
            LayerTreeEntry {
                layer_key: table.clone(),
                label_ko: name.clone(),
                group: LegendGroup::SurveyData,
                visible: true,
                opacity: 1.0,
                read_only: false,
                snap_enabled: true,
                feature_count: 0,
            },
        );
        self.home_map.real_survey.layer_visible.insert(name, true);
        self.home_map.real_survey.layer_styles.insert(table.clone(), setup.style());
        Ok(table)
    }

    /// 창의 「그리기 시작」 — 레이어를 만들고 그리기를 시작한다. 만들지 못하면 C++ 알림 글(창이 생기기 전까지 상태줄).
    pub(crate) fn accept_survey_area_setup(&mut self) {
        let Some(setup) = self.home_map.survey_area_setup.take() else { return };
        match self.create_survey_area_layer(&setup) {
            Ok(table) => {
                let name = setup.layer_name();
                self.right_inspector.selected_layer_key = Some(name.clone());
                self.show_status_message(format!("조사구역 [{name}] 그리기 시작 — 점을 찍고 Enter로 완성"));
                self.begin_edit_layer(&table, &name);
            }
            Err(e) => self.show_status_message(format!(
                "새 조사구역 레이어를 만들지 못했습니다. 조사 파일에 새 표를 쓰지 못했습니다. 파일이 다른 프로그램에서 열려 있거나 \
                 폴더에 쓸 권한이 없을 수 있습니다. 같은 파일을 연 프로그램을 닫고 다시 해 보세요. ({e})"
            )),
        }
    }
}

#[cfg(test)]
#[path = "survey_area_setup_tests.rs"]
mod tests;
