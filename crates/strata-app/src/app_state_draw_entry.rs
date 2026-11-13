//! 조사 도형 그리기 입구 — C++ `startEditSurveyArea`·`startEditFeaturePoly`·`startEditFeatureLine`·
//! `startEditSectionLine`(`MainWindowEditing.cpp:1087-1166`)과 그 공통 길 `ensureDomainLayerForEdit`
//! (`MainWindowSession.cpp:800-825`)·`beginEdit`(`MainWindowEditing.cpp:932-1024`).
//!
//! 여기서는 그릴 대상 레이어를 정하고(범례에 없으면 C++ ensureDomainLayer 의 이름으로 맨 위에 올림),
//! 범례의 현재 레이어로 하고, 보이게 하고, 도형 형식대로 도구를 고른다. 그린 도형을 그 표에 넣는 것은
//! 지시 3-3a 다. C++ 의 「조사구역」 새 레이어 창(KaSurveyAreaDialog)과 「새 조사를 만들까요?」 질문은 없다.

use strata_canvas2d::map_tools::MapToolKind;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::survey_style::GeomKind;

impl StrataAppState {
    /// `startEditSurveyArea` — 이 조사의 조사구역 레이어(범례에서 고른 것, 아니면 처음 보이는 것)에 그린다.
    /// 이어 그릴 레이어가 없으면 C++ 은 새 조사구역 창을 띄운다 — 그 창 전에는 survey_area 표에 그린다.
    pub(crate) fn start_edit_survey_area(&mut self) {
        let current = self.right_inspector.selected_layer_key.clone();
        let own: Vec<&LayerTreeEntry> = self
            .left_layers
            .entries
            .iter()
            .filter(|e| e.layer_key == "survey_area" || e.layer_key.starts_with("survey_area_"))
            .collect();
        let picked = own
            .iter()
            .find(|e| current.as_deref() == Some(e.label_ko.as_str()))
            .or_else(|| own.iter().find(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko)))
            .map(|e| e.layer_key.clone());
        let key = picked.unwrap_or_else(|| "survey_area".to_owned());
        self.begin_edit_layer(&key, "조사구역");
    }

    /// `startEditFeaturePoly` — 범례의 현재 레이어가 유구 면이면 그것, 아니면 유구 면 레이어(없으면 「유구면」).
    pub(crate) fn start_edit_feature_poly(&mut self) {
        self.begin_edit_layer("feature_poly", "유구면");
    }

    /// `startEditFeatureLine` — 「유구선」.
    pub(crate) fn start_edit_feature_line(&mut self) {
        self.begin_edit_layer("feature_line", "유구선");
    }

    /// `startEditSectionLine` — 「단면선」.
    pub(crate) fn start_edit_section_line(&mut self) {
        self.begin_edit_layer("section_line", "단면선");
    }

    /// 범례 우클릭 「그리기·편집 시작」 — `beginEdit(layer)`(MainWindowContextMenus.cpp:261-262). 조사 표가 아닌
    /// 레이어면 그릴 대상을 비우고 그리기 도구를 켜지 않는다.
    pub(crate) fn start_edit_from_legend(&mut self, label: &str) {
        let key = self.left_layers.entries.iter().find(|e| e.label_ko == label).map(|e| e.layer_key.clone());
        match key.filter(|k| self.survey_tables.iter().any(|t| &t.table == k)) {
            Some(k) => self.begin_edit_layer(&k, label),
            None => {
                self.edit_layer_key = None;
                self.stop_capture_tool();
            }
        }
    }

    /// `ensureDomainLayerForEdit` + `beginEdit`.
    fn begin_edit_layer(&mut self, key: &str, title_ko: &str) {
        if self.survey_path.is_none() {
            // C++ 은 여기서 「새 조사」를 물어본다(:805-812).
            self.home_map.draw_guide_message = Some("먼저 「새 조사」로 저장 경로를 만드세요.".to_owned());
            return;
        }
        // LayerOps::ensureDomainLayer — 이미 있으면 그 레이어, 없으면 그 이름으로 맨 위에.
        if !self.left_layers.entries.iter().any(|e| e.layer_key == key) {
            self.left_layers.entries.insert(
                0,
                LayerTreeEntry {
                    layer_key: key.to_owned(),
                    label_ko: title_ko.to_owned(),
                    group: LegendGroup::SurveyData,
                    visible: true,
                    opacity: 1.0,
                    read_only: false,
                    snap_enabled: true,
                    feature_count: 0,
                },
            );
        }
        let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone()) else {
            return;
        };
        let kind = self
            .survey_tables
            .iter()
            .find(|t| t.table == key)
            .and_then(|t| GeomKind::from_type_name(&t.geometry_type))
            .unwrap_or(if key.ends_with("_line") { GeomKind::Line } else { GeomKind::Polygon });
        // KaCaptureMapTool::setMode → resetSession — 그리던 점은 지운다.
        self.draw_sketch = crate::draw_sketch::DrawSketch::default();
        // beginEdit — 보이게, 현재 레이어로, 도형 형식대로 그리기 도구.
        self.home_map.real_survey.layer_visible.insert(name.clone(), true);
        self.right_inspector.selected_layer_key = Some(name.clone());
        self.edit_layer_key = Some(key.to_owned());
        self.active_tool = match kind {
            GeomKind::Polygon => MapToolKind::DrawPolygon,
            GeomKind::Line => MapToolKind::DrawLine,
            GeomKind::Point => MapToolKind::PlacePoint,
        };
        let how = if kind == GeomKind::Point { "좌클릭으로 점 찍기" } else { "좌클릭 점 · 우클릭/Enter 완료 · Esc 취소" };
        self.home_map.draw_guide_message = Some(format!("{name} — {how}"));
    }
}

#[cfg(test)]
#[path = "app_state_draw_entry_tests.rs"]
mod tests;
