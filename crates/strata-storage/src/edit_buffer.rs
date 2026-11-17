use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strata_types::schema::DomainLayerKey;

#[path = "feature_snapshot.rs"]
mod feature_snapshot;
pub use feature_snapshot::{FeatureSnapshot, SharedFeatureRows};

/// C++ `HGIS_FIRST_TEMP_FID` (-1) — 미저장 신규 피처 최초 음수 임시 FID 상수
pub const HGIS_FIRST_TEMP_FID: i64 = -1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureRecord {
    pub fid: i64,
    pub wkt_geom: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerEditBuffer {
    pub key: Option<DomainLayerKey>,
    pub visible: bool,
    pub features: SharedFeatureRows,
    next_temp_id: i64,
    undo_stack: Vec<SharedFeatureRows>,
    redo_stack: Vec<SharedFeatureRows>,
}

impl LayerEditBuffer {
    pub fn snapshot(&self) -> FeatureSnapshot {
        FeatureSnapshot::new(self.features.clone())
    }

    pub fn new(key: DomainLayerKey) -> Self {
        Self {
            key: Some(key),
            visible: true,
            features: SharedFeatureRows::default(),
            next_temp_id: -1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn add_unsaved_feature(
        &mut self,
        wkt_geom: String,
        attributes: BTreeMap<String, String>,
    ) -> i64 {
        self.undo_stack.push(self.features.clone());
        self.redo_stack.clear();
        let fid = self.next_temp_id;
        self.next_temp_id -= 1;
        self.features.push(FeatureRecord {
            fid,
            wkt_geom,
            attributes,
        });
        fid
    }

    /// 도형만 바꾼다(QgsVectorLayer::changeGeometry, 되돌리기 한 단계). 그 fid 가 없으면 false.
    pub fn change_geometry(&mut self, fid: i64, wkt_geom: String) -> bool {
        let Some(i) = self.features.iter().position(|f| f.fid == fid) else {
            return false;
        };
        self.undo_stack.push(self.features.clone());
        self.redo_stack.clear();
        self.features[i].wkt_geom = wkt_geom;
        true
    }

    /// 속성만 바꾼다(QgsVectorLayer::changeAttributeValue 여러 번을 한 편집 명령으로, 되돌리기 한 단계).
    /// `None` 은 그 값을 지운다(NULL). 그 fid 가 없으면 false.
    pub fn change_attributes(&mut self, fid: i64, values: &[(String, Option<String>)]) -> bool {
        let Some(i) = self.features.iter().position(|f| f.fid == fid) else {
            return false;
        };
        self.undo_stack.push(self.features.clone());
        self.redo_stack.clear();
        let attributes = &mut self.features[i].attributes;
        for (field, value) in values {
            match value {
                Some(v) => attributes.insert(field.clone(), v.clone()),
                None => attributes.remove(field),
            };
        }
        true
    }

    /// Deletes a shape (`QgsVectorLayer::deleteFeature`, one undo step). `false` when there is no such fid.
    pub fn delete_feature(&mut self, fid: i64) -> bool {
        let Some(i) = self.features.iter().position(|f| f.fid == fid) else {
            return false;
        };
        self.undo_stack.push(self.features.clone());
        self.redo_stack.clear();
        self.features.remove(i);
        true
    }

    /// Several changes as one undo step (one QGIS edit command): new geometries for some shapes, new shapes
    /// with temporary ids, and shapes to delete. `None`, and nothing changed, when a shape to change or to
    /// delete is not there; else the ids of the new shapes.
    pub fn change_add_delete(&mut self, changes: &[(i64, String)], additions: Vec<(String, BTreeMap<String, String>)>, deletions: &[i64]) -> Option<Vec<i64>> {
        let there = |fid: &i64| self.features.iter().any(|f| f.fid == *fid);
        if !changes.iter().all(|(fid, _)| there(fid)) || !deletions.iter().all(there) {
            return None;
        }
        self.undo_stack.push(self.features.clone());
        self.redo_stack.clear();
        for f in &mut self.features {
            if let Some((_, wkt)) = changes.iter().find(|(fid, _)| *fid == f.fid) {
                f.wkt_geom.clone_from(wkt);
            }
        }
        let mut added = Vec::with_capacity(additions.len());
        for (wkt_geom, attributes) in additions {
            added.push(self.next_temp_id);
            self.features.push(FeatureRecord { fid: self.next_temp_id, wkt_geom, attributes });
            self.next_temp_id -= 1;
        }
        self.features.retain(|f| !deletions.contains(&f.fid));
        Some(added)
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.features, prev);
            self.redo_stack.push(current);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.features, next);
            self.undo_stack.push(current);
            true
        } else {
            false
        }
    }

    pub fn assign_positive_ids_on_commit(&mut self) {
        let mut next_pos = self
            .features
            .iter()
            .map(|f| f.fid)
            .filter(|&id| id > 0)
            .max()
            .unwrap_or(0)
            + 1;
        for f in &mut self.features {
            if f.fid < 0 {
                f.fid = next_pos;
                next_pos += 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "feature_snapshot_tests.rs"]
mod feature_snapshot_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo_feature_push_undo_to_zero_and_redo_restores_exact_geometry() {
        let mut buf = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
        let wkt =
            "POLYGON((219800 361500, 219810 361500, 219810 361510, 219800 361510, 219800 361500))"
                .to_owned();
        let mut attrs = BTreeMap::new();
        attrs.insert("FEAT_NO".to_owned(), "1호 주거지".to_owned());
        buf.add_unsaved_feature(wkt.clone(), attrs);
        assert_eq!(buf.features.len(), 1);

        assert!(buf.undo());
        assert_eq!(buf.features.len(), 0);

        assert!(buf.redo());
        assert_eq!(buf.features.len(), 1);
        assert_eq!(buf.features[0].wkt_geom, wkt);

        // TestKaLayerEditBuffer::testTempFidAndCommitPromotion:
        // 3 unsaved features get -1, -2, -3 and promote to 1, 2, 3 on commit
        let mut commit_buf = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
        let id1 = commit_buf.add_unsaved_feature(wkt.clone(), BTreeMap::new());
        let id2 = commit_buf.add_unsaved_feature(wkt.clone(), BTreeMap::new());
        let id3 = commit_buf.add_unsaved_feature(wkt, BTreeMap::new());
        assert_eq!([id1, id2, id3], [-1, -2, -3]);

        commit_buf.assign_positive_ids_on_commit();
        let promoted: Vec<i64> = commit_buf.features.iter().map(|f| f.fid).collect();
        assert_eq!(promoted, vec![1, 2, 3]);
    }
}

#[cfg(test)]
mod change_tests {
    use super::*;

    /// 도형 바꾸기는 되돌리기 한 단계 — 되돌리면 옛 도형, 다시 실행하면 새 도형. 없는 fid 는 아무것도 안 한다.
    #[test]
    fn change_geometry_is_one_undo_step() {
        let mut b = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
        let fid = b.add_unsaved_feature("POINT (1 1)".to_owned(), BTreeMap::new());
        assert!(b.change_geometry(fid, "POINT (2 2)".to_owned()));
        assert!(!b.change_geometry(99, "POINT (3 3)".to_owned()));
        assert_eq!(b.features[0].wkt_geom, "POINT (2 2)");
        assert!(b.undo());
        assert_eq!(b.features[0].wkt_geom, "POINT (1 1)");
        assert!(b.redo());
        assert_eq!(b.features[0].wkt_geom, "POINT (2 2)");
    }

    /// 「겹친 곳 지우기」 changes a shape, adds its loose pieces and deletes the cutter as ONE step
    /// (PolygonErase.cpp:318-333, one edit command): one undo brings all of it back. A shape that is not
    /// there stops everything.
    #[test]
    fn several_changes_are_one_undo_step() {
        let mut b = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
        let outer = b.add_unsaved_feature("POINT (1 1)".to_owned(), BTreeMap::from([("name".to_owned(), "바깥".to_owned())]));
        let cutter = b.add_unsaved_feature("POINT (2 2)".to_owned(), BTreeMap::new());
        let before = b.features.clone();
        assert_eq!(b.change_add_delete(&[(99, "POINT (0 0)".to_owned())], Vec::new(), &[cutter]), None);
        assert_eq!(b.change_add_delete(&[(outer, "POINT (0 0)".to_owned())], Vec::new(), &[99]), None);
        assert_eq!(b.features, before, "nothing changed");

        let piece = ("POINT (5 5)".to_owned(), BTreeMap::from([("name".to_owned(), "바깥".to_owned())]));
        let added = b.change_add_delete(&[(outer, "POINT (9 9)".to_owned())], vec![piece], &[cutter]).expect("one step");
        assert_eq!(added, [-3]);
        assert_eq!(b.features.iter().map(|f| (f.fid, f.wkt_geom.as_str())).collect::<Vec<_>>(), [(outer, "POINT (9 9)"), (-3, "POINT (5 5)")]);
        assert!(b.undo());
        assert_eq!(b.features, before);
        assert!(b.redo());
        assert_eq!(b.features.len(), 2);
    }

    /// Deleting a shape is one undo step: undo brings it back as it was, redo deletes it again; an fid that
    /// is not there changes nothing and makes no step.
    #[test]
    fn delete_feature_is_one_undo_step() {
        let mut b = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
        let first = b.add_unsaved_feature("POINT (1 1)".to_owned(), BTreeMap::from([("kind".to_owned(), "주거지".to_owned())]));
        let second = b.add_unsaved_feature("POINT (2 2)".to_owned(), BTreeMap::new());
        assert!(!b.delete_feature(99));
        assert!(b.delete_feature(first));
        assert_eq!(b.features.iter().map(|f| f.fid).collect::<Vec<_>>(), [second]);
        assert!(b.undo());
        assert_eq!(b.features.len(), 2);
        assert_eq!((b.features[0].fid, b.features[0].attributes.get("kind").map(String::as_str)), (first, Some("주거지")));
        assert!(b.redo());
        assert_eq!(b.features.len(), 1);
        // The step before the deletion is still the adding of the second shape.
        assert!(b.undo() && b.undo());
        assert_eq!(b.features.len(), 1);
    }

    /// 속성 바꾸기(C++ 「속성 편집」 한 명령)는 되돌리기 한 단계 — None 은 값을 지운다(NULL). 없는 fid 는 아무것도 안 한다.
    #[test]
    fn change_attributes_is_one_undo_step() {
        let mut b = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
        let attrs = BTreeMap::from([("kind".to_owned(), "주거지".to_owned()), ("note".to_owned(), "숯".to_owned())]);
        let fid = b.add_unsaved_feature("POINT (1 1)".to_owned(), attrs.clone());
        let changes = [("kind".to_owned(), Some("수혈".to_owned())), ("note".to_owned(), None), ("period".to_owned(), Some("삼국".to_owned()))];
        assert!(b.change_attributes(fid, &changes));
        assert!(!b.change_attributes(99, &changes));
        let changed = BTreeMap::from([("kind".to_owned(), "수혈".to_owned()), ("period".to_owned(), "삼국".to_owned())]);
        assert_eq!(b.features[0].attributes, changed);
        assert!(b.undo());
        assert_eq!(b.features.len(), 1);
        assert_eq!(b.features[0].attributes, attrs);
        assert!(b.redo());
        assert_eq!(b.features[0].attributes, changed);
    }
}
