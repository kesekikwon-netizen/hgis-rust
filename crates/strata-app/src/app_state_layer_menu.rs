//! 레이어 메뉴의 동작 — C++ `LayerLabelControls::setField`·`setArea`(core/LayerLabelControls.cpp:176-199),
//! `LayerOps::setLabelFontSize`(LabelOps.cpp:130-150), `applyLayerOpacity`(LayerStyleOpacity.cpp:23-29), 「같은 묶음의 맨 아래로」
//! (MainWindowContextMenus.cpp:389-400), `renameSelectedLayer`(MainWindow.cpp:1634-1673). 레이어마다의 값은 layer_key 로 둔다.
//! C++ 은 이 동작들 뒤에 프로젝트를 고친 것으로 표시한다(refreshViews → setDirty) — 「 *」.

use crate::app_state::StrataAppState;
use crate::survey_style::{managed_rule, LabelRule};

/// 면적 식(managedArea) — 면적만·이름+면적.
fn is_managed_area(rule: &LabelRule) -> bool {
    matches!(rule, LabelRule::Area1 | LabelRule::Area2 | LabelRule::NameWithArea(_))
}

/// 열린 「이름 바꾸기」 창(QInputDialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameLayer {
    pub(crate) key: String,
    pub(crate) text: String,
    pub(crate) keys_armed: bool,
}

impl StrataAppState {
    /// 지금 그 조사 레이어의 라벨 모양(메뉴로 바꾼 것이 있으면 그것).
    pub(crate) fn effective_label_rule(&self, key: &str) -> Option<LabelRule> {
        let t = self.survey_tables.iter().find(|t| t.table == key)?;
        let kind = crate::survey_style::GeomKind::from_type_name(&t.geometry_type)?;
        Some(self.home_map.real_survey.label_views.get(key).and_then(|v| v.rule.clone()).unwrap_or_else(|| crate::survey_style::label_rule(key, kind, &t.columns)))
    }

    fn set_label_rule(&mut self, key: &str, rule: LabelRule) {
        self.home_map.real_survey.label_views.entry(key.to_owned()).or_default().rule = Some(rule);
        self.is_dirty = true;
    }

    /// 표시할 항목(setField) — 라벨이 있으면 면적 식은 그대로 두고 항목만 바꾼다. 같은 항목이면 아무것도.
    pub(crate) fn layer_label_field(&mut self, key: &str, field: &str) -> bool {
        let (Some(input), Some(rule)) = (self.layer_menu_input(key), self.effective_label_rule(key)) else { return false };
        if !input.label.field_editable || !input.fields.iter().any(|f| f == field) {
            return false;
        }
        if input.has_labeling {
            if input.label.field == field {
                return false;
            }
            self.set_label_rule(key, managed_rule(field, input.label.area_editable && is_managed_area(&rule)));
        } else {
            self.set_label_rule(key, LabelRule::Field(field.to_owned()));
        }
        true
    }

    /// 면적(㎡) 함께 표시(setArea) — 지금과 반대로. 라벨이 없을 때 켜면 면적 라벨(applyAreaM2Labels).
    pub(crate) fn layer_label_area(&mut self, key: &str) -> bool {
        let (Some(input), Some(rule)) = (self.layer_menu_input(key), self.effective_label_rule(key)) else { return false };
        if !input.label.area_editable {
            return false;
        }
        let rule = if input.has_labeling { managed_rule(&input.label.field, !is_managed_area(&rule)) } else { LabelRule::Area2 };
        self.set_label_rule(key, rule);
        true
    }

    /// 글자 크기(setLabelFontSize).
    pub(crate) fn layer_label_size(&mut self, key: &str, pt: f64) -> bool {
        if !(pt.is_finite() && pt > 0.0) {
            return false;
        }
        self.home_map.real_survey.label_views.entry(key.to_owned()).or_default().size_tenths = Some((pt * 10.0).round() as u16);
        self.is_dirty = true;
        true
    }

    /// 투명도(applyLayerOpacity) — 지도에서만, 그 레이어만.
    pub(crate) fn layer_set_transparency(&mut self, key: &str, transparency: u8) -> bool {
        let Some(e) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == key) else { return false };
        e.opacity = (1.0 - f32::from(transparency) / 100.0).clamp(0.0, 1.0);
        self.is_dirty = true;
        true
    }

    /// 같은 묶음의 맨 아래로 — 그 묶음의 마지막 줄 다음으로 옮긴다.
    pub(crate) fn layer_to_group_bottom(&mut self, key: &str) -> bool {
        let entries = &mut self.left_layers.entries;
        let Some(at) = entries.iter().position(|e| e.layer_key == key) else { return false };
        let group = entries[at].group;
        let last = entries.iter().rposition(|e| e.group == group).unwrap_or(at);
        if last == at {
            return false;
        }
        let entry = entries.remove(at);
        entries.insert(last, entry);
        self.is_dirty = true;
        true
    }

    /// 「이름 바꾸기 (두 번 클릭)」 — 지금 이름을 넣은 창을 연다.
    pub(crate) fn open_rename_layer(&mut self, key: &str) {
        if let Some(e) = self.left_layers.entries.iter().find(|e| e.layer_key == key) {
            self.home_map.rename_layer = Some(RenameLayer { key: key.to_owned(), text: e.label_ko.clone(), keys_armed: false });
        }
    }

    /// 창의 「확인」 — 비면 알림(KaUserError, 창이 생기기 전까지 상태줄), 같으면 아무것도, 아니면 이름을 바꾸고
    /// 「이름 변경: %1 → %2」. 이름으로 묶여 있던 표시·라벨 체크는 새 이름으로 옮긴다.
    pub(crate) fn apply_rename_layer(&mut self) {
        let Some(r) = self.home_map.rename_layer.take() else { return };
        let new = r.text.trim().to_owned();
        let Some(e) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == r.key) else { return };
        if new.is_empty() {
            self.show_status_message("이름을 바꾸지 못했습니다 — 이름은 비울 수 없습니다. 한 글자 이상 넣으세요.".to_owned());
            return;
        }
        let old = std::mem::replace(&mut e.label_ko, new.clone());
        if old == new {
            return;
        }
        let rs = &mut self.home_map.real_survey;
        for map in [&mut rs.layer_visible, &mut rs.label_visible] {
            if let Some(v) = map.remove(&old) {
                map.insert(new.clone(), v);
            }
        }
        if self.right_inspector.selected_layer_key.as_deref() == Some(old.as_str()) {
            self.right_inspector.selected_layer_key = Some(new.clone());
        }
        self.show_status_message(format!("이름 변경: {old} → {new}"));
    }
}

#[cfg(test)]
#[path = "app_state_layer_menu_tests.rs"]
mod tests;
