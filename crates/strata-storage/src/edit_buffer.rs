use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strata_types::schema::DomainLayerKey;

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
    pub features: Vec<FeatureRecord>,
    next_temp_id: i64,
    undo_stack: Vec<Vec<FeatureRecord>>,
    redo_stack: Vec<Vec<FeatureRecord>>,
}

impl LayerEditBuffer {
    pub fn new(key: DomainLayerKey) -> Self {
        Self {
            key: Some(key),
            visible: true,
            features: Vec::new(),
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
}
