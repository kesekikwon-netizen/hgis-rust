//! Immutable snap input, target/cpp-ref/src/core/LayerOps.cpp:1278-1282,2866-2873.
use super::{data::RefVectorData, elevation::Drawn, RefVectorLayers, Shapes};
use std::sync::Arc;

pub(crate) enum GeometrySnapshot {
    Waiting,
    Ready(u32, Arc<RefVectorData>),
    Failed,
}

impl RefVectorLayers {
    pub(crate) fn geometry_snapshot(&self, key: &str) -> GeometrySnapshot {
        let Some(layer) = self.layers.iter().find(|l| l.key == key) else { return GeometrySnapshot::Failed };
        match &layer.shapes {
            Shapes::Ready(Drawn::Shapes(data)) => GeometrySnapshot::Ready(layer.spec.work_epsg, data.clone()),
            Shapes::Wanted | Shapes::Loading(_) => GeometrySnapshot::Waiting,
            Shapes::Failed | Shapes::Ready(Drawn::Heights(_)) => GeometrySnapshot::Failed,
        }
    }
}
