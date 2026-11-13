//! 조사 레이어 면적 — C++ `MeasureOps::geometryAreaSquareMeters`(타원체 없음: 레이어 좌표계의 평면 넓이),
//! `showLayerAreaSummary`(MainWindowContextMenus.cpp:450-481), 시굴격자가 덮을 조사구역 고르기
//! `trenchFillFromSurveyLayer`·`TrenchGridGenerator::pickAutoFillArea`(MainWindow.cpp:1780-1798,
//! TrenchGridGenerator.cpp:170-228). 도형은 열어 둔 표의 편집 버퍼(저장 안 한 것 포함)에서 읽는다.

use strata_storage::edit_buffer::FeatureRecord;
use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::app_state::StrataAppState;

/// 도형 하나의 평면 넓이(면이 아니거나 비었으면 0).
pub(crate) fn feature_area_m2(f: &FeatureRecord) -> f64 {
    if f.wkt_geom.trim().is_empty() {
        return 0.0;
    }
    strata_engine::wkt_area(&f.wkt_geom).map_or(0.0, f64::abs)
}

fn layer_area_m2(t: &SurveyFileTable) -> f64 {
    t.layer.features.iter().map(feature_area_m2).sum()
}

/// QLocale(한국어) `%L1` 의 소수 둘 — 천 단위 쉼표.
fn grouped2(v: f64) -> String {
    let s = format!("{:.2}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{}{out}.{frac}", if v < 0.0 { "-" } else { "" })
}

impl StrataAppState {
    /// 범례 이름의 조사 표.
    fn table_for_label(&self, label: &str) -> Option<&SurveyFileTable> {
        let key = self.left_layers.entries.iter().find(|e| e.label_ko == label).map(|e| e.layer_key.as_str())?;
        self.survey_tables.iter().find(|t| t.table == key)
    }

    /// `LayerOps::findByLayerKey(project, "survey_area")` — 첫 조사구역 표.
    fn survey_area_table(&self) -> Option<&SurveyFileTable> {
        let key = self.left_layers.entries.iter().find(|e| e.layer_key == "survey_area").map(|e| e.layer_key.as_str())?;
        self.survey_tables.iter().find(|t| t.table == key)
    }

    /// 「면적 확인」·「면적·조사 비율 확인」 글. 조사 표가 아니면 None.
    pub(crate) fn layer_area_summary(&self, label: &str, show_ratio: bool) -> Option<String> {
        let t = self.table_for_label(label)?;
        let area = layer_area_m2(t);
        let mut text = format!("{label}\n도형 {}개 · 합계 면적 {} ㎡", t.layer.features.len(), grouped2(area));
        if show_ratio {
            let survey = self.survey_area_table().map_or(0.0, layer_area_m2);
            if survey > 0.0 && survey.is_finite() {
                text.push_str(&format!(
                    "\n전체 조사구역 면적 합계 대비 {}%\n(면이 겹치면 겹친 부분도 합계에 포함됩니다.)",
                    grouped2(100.0 * area / survey)
                ));
            } else {
                text.push_str("\n비율을 보려면 조사구역을 먼저 그려 주세요.");
            }
        }
        Some(text)
    }

    /// 시굴격자가 덮을 조사구역의 넓이 — 첫 조사구역 표에서 pickAutoFillArea. 조사구역이 없으면 0.
    pub(crate) fn trench_survey_area_m2(&self) -> f64 {
        let Some(t) = self.survey_area_table() else { return 0.0 };
        let selected: Vec<i64> = match &self.home_map.selected_shape {
            Some((table, fid)) if *table == t.table => vec![*fid],
            _ => Vec::new(),
        };
        pick_auto_fill_area_m2(&t.layer.features, &selected)
    }
}

/// `TrenchGridGenerator::pickAutoFillArea`(useAll = false)의 넓이 — 도형이 있는 조사구역 가운데 고른 것들(없거나
/// 하나도 안 맞으면 fid 가 가장 큰 하나)을 합친 넓이. 저장 안 한 도형의 음수 fid 는 「가장 큰 fid」가 되지 않는다(C++ 그대로).
pub(crate) fn pick_auto_fill_area_m2(features: &[FeatureRecord], selected: &[i64]) -> f64 {
    let valid: Vec<&FeatureRecord> = features.iter().filter(|f| !f.wkt_geom.trim().is_empty()).collect();
    let mut used: Vec<&FeatureRecord> = valid.iter().filter(|f| selected.contains(&f.fid)).copied().collect();
    if used.is_empty() {
        used.extend(valid.iter().max_by_key(|f| f.fid).copied());
    }
    let wkts: Vec<String> = used.iter().map(|f| f.wkt_geom.clone()).collect();
    strata_engine::union_area(&wkts).unwrap_or(0.0)
}

#[cfg(test)]
#[path = "survey_measure_tests.rs"]
mod tests;
