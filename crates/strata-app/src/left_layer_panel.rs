use strata_types::schema::{DomainLayerKey, LegendGroup};

pub const HGIS_BADGE_DEBOUNCE_MS: u64 = 150;

#[derive(Debug, Clone, PartialEq)]
pub struct LayerTreeEntry {
    pub layer_key: String,
    pub label_ko: String,
    pub group: LegendGroup,
    pub visible: bool,
    pub opacity: f32,
    pub read_only: bool,
    pub snap_enabled: bool,
    pub feature_count: usize,
}

impl LayerTreeEntry {
    /// C++ `KaLeftLayerPanel::updateFeatureCountBadge` — `"레이어명 [N]"` 배지 문자열 생성
    #[must_use]
    pub fn display_label_with_badge(&self) -> String {
        format!("{} [{}]", self.label_ko, self.feature_count)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LeftLayerPanel {
    pub entries: Vec<LayerTreeEntry>,
}

impl LeftLayerPanel {
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn ensure_domain_layer(&mut self, key: DomainLayerKey) {
        let key_str = key.id();
        if self.entries.iter().any(|e| e.layer_key == key_str) {
            return;
        }
        self.entries.push(LayerTreeEntry {
            layer_key: key_str.to_owned(),
            label_ko: key.name_ko().to_owned(),
            group: LegendGroup::SurveyData,
            visible: true,
            opacity: 1.0,
            read_only: false,
            snap_enabled: true,
            feature_count: 0,
        });
    }

    pub fn add_cadastral_wms_layer(&mut self) {
        if self.entries.iter().any(|e| e.layer_key == "vworld_cadastral_wms") {
            return;
        }
        self.entries.push(LayerTreeEntry {
            layer_key: "vworld_cadastral_wms".to_owned(),
            label_ko: "연속지적도 (VWorld WMS)".to_owned(),
            group: LegendGroup::Cadastral,
            visible: true,
            opacity: 0.75,
            read_only: true,
            snap_enabled: false, // VWorld 지적 그림 스냅 제외 규칙
            feature_count: 0,
        });
    }

    pub fn remove_by_key(&mut self, layer_key: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.layer_key != layer_key);
        self.entries.len() < before
    }

    pub fn set_all_visible(&mut self, visible: bool) {
        for e in &mut self.entries {
            e.visible = visible;
        }
    }

    /// C++ `KaLeftLayerPanel::updateFeatureCountBadge` — 레이어별 피처 개수 `[N]` 원자적 갱신
    pub fn update_feature_count_badge(&mut self, layer_key: &str, count: usize) -> bool {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.layer_key == layer_key) {
            entry.feature_count = count;
            true
        } else {
            false
        }
    }

    /// C++ `KaLeftLayerPanel::filterLayersByKeyword` — 정규식 메타문자 리터럴 이스케이프 및 대소문자 무시 부분 문자열 필터링, 매칭 자식이 있는 부모 그룹 자동 확장
    #[must_use]
    pub fn filter_layers_by_keyword(&self, keyword: &str) -> (Vec<&LayerTreeEntry>, Vec<LegendGroup>) {
        let needle = keyword.trim().to_lowercase();
        if needle.is_empty() {
            let matched: Vec<&LayerTreeEntry> = self.entries.iter().collect();
            return (matched, Vec::new());
        }
        let mut matched = Vec::new();
        let mut expanded_groups = Vec::new();
        for entry in &self.entries {
            let label_lower = entry.label_ko.to_lowercase();
            let key_lower = entry.layer_key.to_lowercase();
            let badge_lower = entry.display_label_with_badge().to_lowercase();
            if label_lower.contains(&needle)
                || key_lower.contains(&needle)
                || badge_lower.contains(&needle)
            {
                matched.push(entry);
                if !expanded_groups.contains(&entry.group) {
                    expanded_groups.push(entry.group);
                }
            }
        }
        (matched, expanded_groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_layers_by_keyword_trench_auto_expands_group_and_escapes_regex_meta() {
        let mut panel = LeftLayerPanel::new_empty();
        panel.ensure_domain_layer(DomainLayerKey::SurveyArea);
        panel.ensure_domain_layer(DomainLayerKey::FeaturePoly);
        panel.ensure_domain_layer(DomainLayerKey::TrialTrench);
        panel.ensure_domain_layer(DomainLayerKey::ArtifactPoint);
        assert!(panel.update_feature_count_badge(DomainLayerKey::TrialTrench.id(), 15));

        let (matched, expanded) = panel.filter_layers_by_keyword("시굴");
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].label_ko, "시굴격자");
        assert_eq!(matched[0].display_label_with_badge(), "시굴격자 [15]");
        assert_eq!(expanded, vec![LegendGroup::SurveyData]);

        // 영문 키("trench") 대소문자 무시 필터링
        let (key_matched, _) = panel.filter_layers_by_keyword("TRENCH");
        assert_eq!(key_matched.len(), 1);

        // 정규식 메타문자('[', ']', '*', '+')가 포함되어도 패닉 없이 리터럴 비교
        let (meta_matched, _) = panel.filter_layers_by_keyword("[15]");
        assert_eq!(meta_matched.len(), 1);
        let (none_matched, none_expanded) = panel.filter_layers_by_keyword(".*[+(?");
        assert!(none_matched.is_empty());
        assert!(none_expanded.is_empty());
    }
}
