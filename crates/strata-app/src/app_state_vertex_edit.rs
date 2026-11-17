//! The vertex handles of the selected shape — C++ `KaFeatureSelectTool` with its `KaVertexEditTool`
//! (KaFeatureSelectTool.cpp:71-119, 177-258; KaVertexEditTool.cpp:243-416) and `LayerOps::applyVertexMove`
//! (GeometryEditVertex.cpp:76-156). There is no tool of its own: whenever exactly one line or polygon of an
//! editable survey layer is selected in 「도형선택」, it has a handle on every vertex. The shape is changed in
//! the CRS of its table: only the place of the pointer is carried there.

use strata_engine::Transformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::LegendGroup;

use crate::app_state::vertex_edit_geom::{segment_near, vertex_near, vertices, with_vertex_inserted, with_vertex_moved, without_vertex};
use crate::app_state::StrataAppState;
use crate::app_state_layer_remove::UndoStep;
use crate::checklist_state_geometry::parts_wkt;

/// `kSharedVertexTolerance`: corners no further apart than a millimetre are one corner (GeometryEditVertex.cpp:20).
const SHARED_M: f64 = 0.001;

/// The shape that has the handles, as its table holds it.
pub(crate) struct VertexTarget {
    pub(crate) table: String,
    pub(crate) fid: i64,
    pub(crate) parts: Vec<WkbGeometry>,
    srs_id: i32,
}

impl StrataAppState {
    /// `syncVertexTarget`: the selected shape, when it is a line or a polygon of an editable survey layer.
    pub(crate) fn vertex_target(&self) -> Option<VertexTarget> {
        // Handles belong to one selected shape only (`syncVertexTarget`, KaFeatureSelectTool.cpp:111-119).
        if !self.home_map.also_selected.is_empty() {
            return None;
        }
        let (table, fid) = self.home_map.selected_shape.as_ref()?;
        let editable = self.left_layers.entries.iter().any(|e| e.layer_key == *table && e.group == LegendGroup::SurveyData && !e.read_only);
        let t = self.survey_tables.iter().find(|t| t.table == *table).filter(|_| editable)?;
        let parts = wkt_to_parts(&t.layer.features.iter().find(|f| f.fid == *fid)?.wkt_geom).ok()?;
        let handles = parts.iter().any(|part| !matches!(part, WkbGeometry::Point(_)));
        handles.then(|| VertexTarget { table: table.clone(), fid: *fid, parts, srs_id: t.srs_id })
    }

    fn carry(&self, srs_id: i32, into_table: bool) -> Option<Transformer> {
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let table = u32::try_from(srs_id).ok().filter(|srs| *srs > 0 && *srs != work)?;
        if into_table { Transformer::new(work, table).ok() } else { Transformer::new(table, work).ok() }
    }

    /// The vertices of the target on the map (work CRS), for the handles.
    pub(crate) fn vertex_points_on_map(&self, target: &VertexTarget) -> Vec<[f64; 2]> {
        let points = vertices(&target.parts);
        match self.carry(target.srs_id, false) {
            Some(to_map) => to_map.transform_all(&points).unwrap_or(points),
            None => points,
        }
    }

    /// A place of the map and a reach of `px` screen points, as the table of the target counts them
    /// (`toLayer`, `layerTolerance`).
    fn in_table(&self, target: &VertexTarget, work: [f64; 2], px: f64) -> ([f64; 2], f64) {
        let reach = self.home_map.viewport.meters_per_pixel * px;
        let Some(to_table) = self.carry(target.srs_id, true) else { return (work, reach) };
        match (to_table.transform(work), to_table.transform([work[0] + reach, work[1]])) {
            (Ok(place), Ok(edge)) => (place, (edge[0] - place[0]).hypot(edge[1] - place[1])),
            _ => (work, reach),
        }
    }

    /// The handle under a place of the map, within `px` screen points (`vertexNear`).
    pub(crate) fn vertex_at(&self, work: [f64; 2], px: f64) -> Option<usize> {
        let target = self.vertex_target()?;
        let (place, reach) = self.in_table(&target, work, px);
        vertex_near(&target.parts, place, reach)
    }

    /// The side under a place of the map, within `px` screen points: whether there is one (`segmentNear`).
    pub(crate) fn side_at(&self, work: [f64; 2], px: f64) -> bool {
        self.vertex_target().is_some_and(|target| {
            let (place, reach) = self.in_table(&target, work, px);
            segment_near(&target.parts, place, reach).is_some()
        })
    }

    /// A click on a handle: the vertex is picked and nothing changes.
    pub(crate) fn pick_vertex(&mut self, index: usize) {
        self.home_map.active_vertex = Some(index);
        self.show_status_message("점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다.");
    }

    /// One undo step of the target's table for these shapes.
    fn commit_vertex_edit(&mut self, table: &str, changes: &[(i64, String)]) -> bool {
        let Some(t) = self.survey_tables.iter_mut().find(|t| t.table == table) else { return false };
        if t.layer.change_add_delete(changes, Vec::new(), &[]).is_none() {
            return false;
        }
        self.redo_tables.clear();
        self.undo_tables.push(UndoStep::Table(table.to_owned()));
        self.is_dirty = true;
        self.after_layer_edit(table);
        true
    }

    /// The vertex goes to a place of the map (「꼭짓점 이동」). With 「공유 경계」 on, the same corner of the
    /// other shapes of the layer goes along in the same step.
    pub(crate) fn move_vertex_to(&mut self, index: usize, work: [f64; 2]) -> bool {
        let Some(target) = self.vertex_target() else { return false };
        let (to, _) = self.in_table(&target, work, 0.0);
        let (Some(from), Some(moved)) = (vertices(&target.parts).get(index).copied(), with_vertex_moved(&target.parts, index, to)) else {
            self.show_status_message("수정점을 옮기지 못했습니다.");
            return false;
        };
        let mut changes = vec![(target.fid, parts_wkt(&moved))];
        if self.home_map.cad.snap_topological {
            let neighbours = self.survey_tables.iter().find(|t| t.table == target.table).map(|t| &t.layer.features[..]).unwrap_or_default();
            for shape in neighbours.iter().filter(|f| f.fid != target.fid) {
                let Ok(mut parts) = wkt_to_parts(&shape.wkt_geom) else { continue };
                let shared: Vec<usize> = vertices(&parts).iter().enumerate().filter(|(_, p)| (p[0] - from[0]).hypot(p[1] - from[1]) <= SHARED_M).map(|(at, _)| at).collect();
                for at in &shared {
                    parts = with_vertex_moved(&parts, *at, to).unwrap_or(parts);
                }
                if !shared.is_empty() {
                    changes.push((shape.fid, parts_wkt(&parts)));
                }
            }
        }
        let done = self.commit_vertex_edit(&target.table, &changes);
        self.show_status_message(if done { "수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다." } else { "수정점을 옮기지 못했습니다." });
        done
    }

    /// 「점추가」: a vertex goes into the side under a place of the map (within 10 screen points).
    pub(crate) fn insert_vertex_at(&mut self, work: [f64; 2]) -> bool {
        let Some(target) = self.vertex_target() else { return false };
        let (place, reach) = self.in_table(&target, work, 10.0);
        let Some(wider) = segment_near(&target.parts, place, reach).and_then(|(before, at)| with_vertex_inserted(&target.parts, before, at)) else { return false };
        let done = self.commit_vertex_edit(&target.table, &[(target.fid, parts_wkt(&wider))]);
        if done {
            self.home_map.active_vertex = None;
            self.show_status_message("점을 넣었습니다.");
        }
        done
    }

    /// 「점삭제」: the vertex is taken out; a shape at its fewest points says so and stays.
    pub(crate) fn delete_vertex(&mut self, index: usize) -> bool {
        let Some(target) = self.vertex_target() else { return false };
        let fewer = match without_vertex(&target.parts, index) {
            Ok(fewer) => fewer,
            Err(why) => {
                self.show_status_message(why);
                return false;
            }
        };
        let done = self.commit_vertex_edit(&target.table, &[(target.fid, parts_wkt(&fewer))]);
        if done {
            // The numbers after the vertex moved down: no vertex is picked.
            self.home_map.active_vertex = None;
            self.show_status_message("점을 지웠습니다.");
        }
        done
    }

    /// target/cpp-ref/src/app/KaFeatureSelectTool.cpp:71-82: an owned vertex key is consumed even if the edit fails.
    pub(crate) fn handle_vertex_delete_key(&mut self) -> bool {
        let Some(index) = self.home_map.active_vertex.filter(|_| self.vertex_target().is_some()) else { return false };
        let done = self.delete_vertex(index);
        if done {
            self.show_status_message("점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다.");
        }
        true
    }
}

#[cfg(test)]
#[path = "app_state_vertex_edit_tests.rs"]
mod tests;
