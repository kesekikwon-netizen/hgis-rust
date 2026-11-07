use strata_types::schema::{DomainLayerKey, LegendGroup};

#[derive(Debug, Clone, PartialEq)]
pub struct LayerTreeEntry {
    pub layer_key: String,
    pub label_ko: String,
    pub group: LegendGroup,
    pub visible: bool,
    pub opacity: f32,
    pub read_only: bool,
    pub snap_enabled: bool,
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
}
