//! 레이어 메뉴를 만들 때 보는 레이어 상태 — 범례 줄(layer_key)과 그 조사 표에서. C++ showLayerTreeContextMenu(:216-238)이
//! QGIS 레이어에서 읽는 값(벡터·유효·도형 수·형식·조사 파일·참조·xyz·묶음 맨 아래·불투명도·라벨)을 같은 뜻으로 모은다.

use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::layer_menu_label::{describe_label, LabelSetting};
use crate::layer_menu_model::{menu_kind, LayerMenuInput, MenuGeom};
use crate::survey_style::{detect_name_field, label_rule, GeomKind, LabelRule, LABEL_SIZE_PT};

/// 우리 라벨 모양 → QGIS 라벨이 하는 일.
fn label_setting(rule: &LabelRule) -> LabelSetting {
    match rule {
        LabelRule::Area1 | LabelRule::Area2 => LabelSetting::Area(None),
        LabelRule::NameWithArea(f) => LabelSetting::Area(Some(f.clone())),
        LabelRule::Field(f) => LabelSetting::Field(f.clone()),
        LabelRule::Empty => LabelSetting::Empty,
        LabelRule::None => LabelSetting::Unconfigured,
    }
}

impl StrataAppState {
    /// 범례의 그 레이어(layer_key)로 메뉴 입력을 만든다. 없는 레이어면 None.
    pub(crate) fn layer_menu_input(&self, key: &str) -> Option<LayerMenuInput> {
        let entry = self.left_layers.entries.iter().find(|e| e.layer_key == key)?;
        let table = self.survey_tables.iter().find(|t| t.table == key);
        // 배경 타일과 VWorld 지적 그림은 래스터(QgsRasterLayer), 나머지(조사 표·주변 거리 고리)는 벡터.
        let vector = entry.group != LegendGroup::Basemap && key != "vworld_cadastral_wms";
        let geom = match table.map(|t| t.geometry_type.to_ascii_uppercase()) {
            Some(g) if g.contains("POLYGON") => Some(MenuGeom::Polygon),
            Some(g) if g.contains("LINE") => Some(MenuGeom::Line),
            Some(_) => Some(MenuGeom::Point),
            None if vector => Some(MenuGeom::Polygon),
            None => None,
        };
        let reference = entry.group != LegendGroup::SurveyData;
        let kind = menu_kind(key, vector, reference);
        let columns = table.map(|t| t.columns.clone()).unwrap_or_default();
        // QGIS 는 GPKG 의 fid 를 맨 앞 항목으로 보인다.
        let fields: Vec<String> = if table.is_some() { std::iter::once("fid".to_owned()).chain(columns.iter().map(|(c, _)| c.clone())).collect() } else { Vec::new() };
        let geom_kind = match geom {
            Some(MenuGeom::Polygon) => GeomKind::Polygon,
            Some(MenuGeom::Line) => GeomKind::Line,
            _ => GeomKind::Point,
        };
        // 메뉴로 바꾼 라벨이 있으면 그것(replaceManagedContent), 없으면 처음 모양.
        let view = self.home_map.real_survey.label_views.get(key);
        let rule = match (table, view.and_then(|v| v.rule.clone())) {
            (Some(_), Some(r)) => r,
            (Some(_), None) => label_rule(key, geom_kind, &columns),
            (None, _) => LabelRule::None,
        };
        let setting = label_setting(&rule);
        let label = describe_label(&setting, &fields, vector, geom == Some(MenuGeom::Polygon), key, entry.group == LegendGroup::Cadastral);
        let same_group: Vec<&str> = self.left_layers.entries.iter().filter(|e| e.group == entry.group).map(|e| e.layer_key.as_str()).collect();
        Some(LayerMenuInput {
            kind,
            vector,
            valid: true,
            feature_count: table.map_or(entry.feature_count, |t| t.layer.features.len()),
            geom,
            busy: false,
            survey_open: self.survey_path.is_some(),
            own_survey: table.is_some() && self.survey_path.is_some(),
            read_only: entry.read_only,
            reference_or_cadastral: reference,
            shp: false,
            xyz: entry.group == LegendGroup::Basemap,
            at_group_bottom: same_group.last() == Some(&key),
            opacity: f64::from(entry.opacity),
            has_labeling: setting != LabelSetting::Unconfigured,
            labels_visible: setting != LabelSetting::Unconfigured && self.home_map.real_survey.is_label_visible(&entry.label_ko),
            current_label_field: match &rule {
                LabelRule::Field(f) | LabelRule::NameWithArea(f) => f.clone(),
                _ if table.is_some() => detect_name_field(&columns),
                _ => String::new(),
            },
            show_area: matches!(rule, LabelRule::Area1 | LabelRule::Area2 | LabelRule::NameWithArea(_)),
            label_size_pt: view.and_then(|v| v.size_tenths).map_or(f64::from(LABEL_SIZE_PT), |t| f64::from(t) / 10.0),
            label,
            fields,
        })
    }
}

#[cfg(test)]
#[path = "layer_menu_input_tests.rs"]
mod tests;
