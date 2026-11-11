//! `SurveyDoc` — 조사 파일(.gpkg)의 7개 도메인 레이어를 메모리에 올린 것.
//!
//! C++ 은 `QgsVectorLayer` 가 도형·속성·fid 를 들고 있다. Rust 에는 그 계층이 없어
//! 이 구조체가 그 자리를 맡는다: 도형마다 fid·지오메트리·속성을 따로 가진다.
//! 읽기는 `survey_gpkg_read`, 쓰기는 `survey_gpkg_write` 에 있다.

use std::collections::BTreeMap;

use strata_types::schema::DomainLayerKey;

use crate::wkb_codec::WkbGeometry;

/// 속성 값 하나. GPKG(SQLite) 열의 실제 저장 값 그대로다.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Null,
    Text(String),
    Real(f64),
    Int(i64),
    Blob(Vec<u8>),
}

impl FieldValue {
    /// 글자로 읽는다(없으면 빈 글자).
    #[must_use]
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Real(v) => v.to_string(),
            Self::Int(v) => v.to_string(),
            Self::Null | Self::Blob(_) => String::new(),
        }
    }
}

/// 도형 하나: fid + 지오메트리 + 속성.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurveyFeature {
    /// 파일에 이미 있는 도형이면 그 fid. 아직 저장하지 않은 새 도형은 `None`.
    pub fid: Option<i64>,
    /// 풀어 읽은 지오메트리. 지원하지 않는 종류면 `None` 이고 `source_blob` 만 남는다.
    pub geom: Option<WkbGeometry>,
    /// 파일에서 읽은 원래 지오메트리 바이트. 도형을 고치지 않았으면 저장할 때
    /// 이 바이트를 그대로 다시 쓴다(멀티폴리곤 등 원본을 망가뜨리지 않는다).
    pub source_blob: Option<Vec<u8>>,
    /// 열 이름 → 값.
    pub attrs: BTreeMap<String, FieldValue>,
}

impl SurveyFeature {
    /// 새로 그린 도형(아직 fid 없음).
    #[must_use]
    pub fn new_drawn(geom: WkbGeometry) -> Self {
        Self {
            fid: None,
            geom: Some(geom),
            source_blob: None,
            attrs: BTreeMap::new(),
        }
    }

    /// 지오메트리를 바꾼다. 원본 바이트는 더 이상 맞지 않으므로 버린다.
    pub fn set_geom(&mut self, geom: WkbGeometry) {
        self.geom = Some(geom);
        self.source_blob = None;
    }

    /// 글자 속성을 넣는다.
    pub fn set_text(&mut self, name: &str, value: &str) {
        self.attrs
            .insert(name.to_owned(), FieldValue::Text(value.to_owned()));
    }

    /// 글자 속성을 읽는다(없으면 빈 글자).
    #[must_use]
    pub fn text(&self, name: &str) -> String {
        self.attrs.get(name).map(FieldValue::as_text).unwrap_or_default()
    }
}

/// 도메인 레이어 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyLayer {
    pub key: DomainLayerKey,
    pub features: Vec<SurveyFeature>,
}

/// 조사 파일 전체(7개 도메인 레이어). 레이어는 항상 `DomainLayerKey::ALL` 순서로 7개 있다.
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyDoc {
    /// 작업 좌표계 EPSG 번호(5186 또는 5187).
    pub srs_id: i32,
    pub layers: Vec<SurveyLayer>,
}

impl SurveyDoc {
    /// 빈 조사.
    #[must_use]
    pub fn new(srs_id: i32) -> Self {
        Self {
            srs_id,
            layers: DomainLayerKey::ALL
                .into_iter()
                .map(|key| SurveyLayer {
                    key,
                    features: Vec::new(),
                })
                .collect(),
        }
    }

    fn index_of(key: DomainLayerKey) -> usize {
        DomainLayerKey::ALL
            .iter()
            .position(|k| *k == key)
            .unwrap_or(0)
    }

    #[must_use]
    pub fn layer(&self, key: DomainLayerKey) -> &SurveyLayer {
        &self.layers[Self::index_of(key)]
    }

    pub fn layer_mut(&mut self, key: DomainLayerKey) -> &mut SurveyLayer {
        &mut self.layers[Self::index_of(key)]
    }

    /// 도형을 더하고 그 레이어 안의 순번을 돌려준다.
    pub fn add_feature(&mut self, key: DomainLayerKey, feature: SurveyFeature) -> usize {
        let layer = self.layer_mut(key);
        layer.features.push(feature);
        layer.features.len() - 1
    }

    /// 도형 수 합계.
    #[must_use]
    pub fn total_features(&self) -> usize {
        self.layers.iter().map(|l| l.features.len()).sum()
    }

    /// 도형이 하나라도 있는 레이어의 키(C++ `addNonEmptyDomainLayers` 가 범례에 올리는 것).
    #[must_use]
    pub fn non_empty_keys(&self) -> Vec<DomainLayerKey> {
        self.layers
            .iter()
            .filter(|l| !l.features.is_empty())
            .map(|l| l.key)
            .collect()
    }

    /// 모든 도형을 감싸는 범위 `[min_x, max_x, min_y, max_y]`. 도형이 없으면 `None`.
    #[must_use]
    pub fn extent(&self) -> Option<[f64; 4]> {
        let mut out: Option<[f64; 4]> = None;
        for feature in self.layers.iter().flat_map(|l| l.features.iter()) {
            let Some(geom) = feature.geom.as_ref() else {
                continue;
            };
            let e = geom.envelope_xy();
            out = Some(match out {
                None => e,
                Some(o) => [o[0].min(e[0]), o[1].max(e[1]), o[2].min(e[2]), o[3].max(e[3])],
            });
        }
        out
    }
}
