//! Restore buffers through PROJ before clearing the old project, target/cpp-ref/src/app/MainWindowSession.cpp:246-254.
use crate::app_state::StrataAppState;
use std::collections::BTreeMap;
use strata_types::buffer::DistanceRing;

impl StrataAppState {
    pub(crate) fn buffers_in_work_crs(&self, mut buffers: BTreeMap<u32, DistanceRing>) -> Result<BTreeMap<u32, DistanceRing>, String> {
        for ring in buffers.values_mut() {
            if ring.crs != self.work_crs {
                let bad = |_| strata_storage::survey_gpkg_buffers::INVALID_BUFFER_KO.to_owned();
                let transformer = strata_engine::Transformer::new(ring.crs.epsg_code(), self.work_crs.epsg_code()).map_err(bad)?;
                for line in &mut ring.lines {
                    *line = transformer.transform_all(line).map_err(bad)?;
                }
                ring.labels = transformer.transform_all(&ring.labels).map_err(bad)?;
                ring.crs = self.work_crs;
            }
        }
        Ok(buffers)
    }
}
