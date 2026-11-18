//! Owned alignment sources, target/cpp-ref/src/core/LayerOps.cpp:2847-2897.
use crate::app_state::StrataAppState;
use crate::ref_vector::{data::RefVectorData, snap::GeometrySnapshot};
use std::sync::Arc;
use strata_storage::edit_buffer::FeatureSnapshot;
use strata_types::schema::LegendGroup;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Scope {
    All,
    Current(Option<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Policy {
    pub enabled: bool,
    pub tolerance: f64,
    pub scope: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Availability {
    Ready,
    Waiting,
    Failed(String),
}

#[derive(Clone, Debug)]
pub(super) enum Source {
    Survey { key: String, label: String, epsg: i32, rows: FeatureSnapshot },
    Reference { key: String, label: String, epsg: u32, data: Arc<RefVectorData> },
}

impl Source {
    fn same_revision(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Survey { key, label, epsg, rows }, Self::Survey { key: k, label: l, epsg: e, rows: r }) => key == k && label == l && epsg == e && rows.same_revision(r),
            (Self::Reference { key, label, epsg, data }, Self::Reference { key: k, label: l, epsg: e, data: d }) => key == k && label == l && epsg == e && Arc::ptr_eq(data, d),
            (Self::Survey { .. }, Self::Reference { .. }) | (Self::Reference { .. }, Self::Survey { .. }) => false,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Sources {
    pub work: u32,
    pub policy: Policy,
    pub availability: Availability,
    pub layers: Vec<Source>,
}

impl Sources {
    pub fn same_revision(&self, other: &Self) -> bool {
        self.work == other.work && self.policy == other.policy && self.availability == other.availability && self.layers.len() == other.layers.len() && self.layers.iter().zip(&other.layers).all(|(a, b)| a.same_revision(b))
    }
    pub fn capture(app: &StrataAppState) -> Self {
        let selected = app.right_inspector.selected_layer_key.as_deref();
        let current = app
            .left_layers
            .entries
            .iter()
            .find(|r| Some(r.layer_key.as_str()) == selected)
            .or_else(|| app.left_layers.entries.iter().find(|r| Some(r.label_ko.as_str()) == selected))
            .map(|r| r.layer_key.clone());
        let scope = if app.home_map.cad.snap_target_current_only { Scope::Current(current) } else { Scope::All };
        let mut sources = Self {
            work: strata_types::crs::CrsCode::from(app.work_crs).epsg_number(),
            policy: Policy {
                enabled: app.home_map.cad.snap_enabled,
                tolerance: app.home_map.cad.snap_tolerance_px,
                scope,
            },
            availability: Availability::Ready,
            layers: Vec::new(),
        };
        if !sources.policy.enabled {
            return sources;
        }
        // Metadata/Arc copies only, O(rows * (tables + references)); heavy work runs on the worker.
        // target/cpp-ref/src/core/LayerOps.cpp:1278-1282,2863-2873: ActiveLayer differs from survey scope.
        for row in app.left_layers.entries.iter().filter(|r| app.home_map.real_survey.is_layer_visible(&r.label_ko)) {
            let table = app.survey_tables.iter().find(|t| t.table == row.layer_key);
            let vector = app.ref_vectors.has(&row.layer_key);
            let eligible = match &sources.policy.scope {
                Scope::Current(key) => key.as_deref() == Some(row.layer_key.as_str()) && (table.is_some() || vector),
                Scope::All => row.snap_enabled || (vector && matches!(row.group, LegendGroup::Cadastral | LegendGroup::SurveyData)),
            };
            if !eligible {
                continue;
            }
            if let Some(table) = table {
                sources.layers.push(Source::Survey {
                    key: row.layer_key.clone(),
                    label: row.label_ko.clone(),
                    epsg: table.srs_id,
                    rows: table.layer.snapshot(),
                });
            } else {
                match app.ref_vectors.geometry_snapshot(&row.layer_key) {
                    GeometrySnapshot::Ready(epsg, data) => sources.layers.push(Source::Reference {
                        key: row.layer_key.clone(),
                        label: row.label_ko.clone(),
                        epsg,
                        data,
                    }),
                    GeometrySnapshot::Waiting if sources.availability == Availability::Ready => sources.availability = Availability::Waiting,
                    GeometrySnapshot::Waiting => {}
                    GeometrySnapshot::Failed => sources.availability = Availability::Failed(format!("{}: {}", row.label_ko, crate::ref_vector::OPEN_FAILED)),
                }
            }
        }
        sources
    }
}
