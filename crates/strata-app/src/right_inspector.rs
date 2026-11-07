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
    pub stroke_width_mm: f32,
    pub fill_opacity: f32,
}

impl RightInspectorState {
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            active_card: InspectorCardTab::FeatureAttributeCard,
            selected_layer_key: None,
            selected_feature_id: None,
            stroke_width_mm: 0.35,
            fill_opacity: 0.30,
        }
    }
}
