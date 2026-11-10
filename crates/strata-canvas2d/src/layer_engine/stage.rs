//! 레이어 창고 — C++ `QgsProject` + `QgsLayerTree` 의 Rust 대응.
//!
//! C++ 은 프로젝트에 레이어를 넣고(`project->addMapLayer`) 순서를 관리한다.
//! Rust 는 지금까지 레이어를 **그리는 코드의 if 문**으로 흉내 냈다. 그래서
//! 레이어를 꺼도 안 꺼지고, 순서를 바꿀 수 없고, 추가하려면 코드를 고쳐야 했다.
//!
//! 여기서는 레이어를 데이터로 들고 있는다:
//! - 무엇을(지오메트리) — [`LayerGeometry`]
//! - 어떻게(스타일) — [`super::style::DomainStyle`]
//! - 언제(순서) — `z_order`
//! - 보이는지 — `visible`

use super::style::DomainStyle;

/// 레이어가 담는 지오메트리 종류. C++ `Qgis::GeometryType` 대응.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryKind {
    /// 면(폴리곤).
    Polygon,
    /// 선.
    Line,
    /// 점.
    Point,
}

/// 지도 좌표계(EPSG:5186/5187)의 한 피처.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// 이 피처를 그릴 좌표들. 면은 닫힌 링, 선은 열린 경로, 점은 1개.
    pub pts: Vec<[f64; 2]>,
    /// 라벨 글자(없으면 빈 문자열). 예: 면적 "1,234㎡", 이름, 번호.
    pub label: String,
}

impl Feature {
    /// 점 하나.
    #[must_use]
    pub fn point(x: f64, y: f64) -> Self {
        Self {
            pts: vec![[x, y]],
            label: String::new(),
        }
    }

    /// 선.
    #[must_use]
    pub fn line(pts: Vec<[f64; 2]>) -> Self {
        Self {
            pts,
            label: String::new(),
        }
    }

    /// 면(닫힌 링).
    #[must_use]
    pub fn polygon(pts: Vec<[f64; 2]>) -> Self {
        Self {
            pts,
            label: String::new(),
        }
    }

    /// 라벨을 붙인 사본.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// 무게중심(라벨 위치). 점이 없으면 `None`.
    #[must_use]
    pub fn center(&self) -> Option<[f64; 2]> {
        if self.pts.is_empty() {
            return None;
        }
        let n = self.pts.len() as f64;
        let sx: f64 = self.pts.iter().map(|p| p[0]).sum();
        let sy: f64 = self.pts.iter().map(|p| p[1]).sum();
        Some([sx / n, sy / n])
    }
}

/// 한 레이어. C++ `QgsVectorLayer` + 그 심볼.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLayer {
    /// 도메인 레이어 키(`survey_area` 등) 또는 배경지도 키.
    pub key: String,
    /// 화면에 보이는 이름.
    pub title: &'static str,
    /// 지오메트리 종류.
    pub geometry: GeometryKind,
    /// 그리기 순서. **작을수록 아래**(먼저 그려짐). C++ 레이어 트리 순서와 같다.
    pub z_order: i32,
    /// 표시 여부. 끄면 그리지 않는다.
    pub visible: bool,
    /// 이 레이어의 스타일.
    pub style: DomainStyle,
    /// 피처들.
    pub features: Vec<Feature>,
    /// 파일에서 온 레이어인지(조사 데이터) 아니면 참조 지도인지.
    pub is_survey_data: bool,
}

impl MapLayer {
    /// 빈 레이어를 만든다.
    #[must_use]
    pub fn new(key: &str, title: &'static str, geometry: GeometryKind, z_order: i32) -> Self {
        Self {
            key: key.to_owned(),
            title,
            geometry,
            z_order,
            visible: true,
            style: DomainStyle::for_key(key),
            features: Vec::new(),
            is_survey_data: true,
        }
    }

    /// 참조 지도(배경지도·유산 등)로 표시한다.
    #[must_use]
    pub fn as_reference(mut self) -> Self {
        self.is_survey_data = false;
        self
    }

    /// 그릴 것이 있는지 — 보이고, 피처가 하나 이상.
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.visible && !self.features.is_empty()
    }

    /// 피처 수(범례 배지용).
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}

/// 레이어 창고. C++ `QgsProject` 의 레이어 목록에 해당한다.
#[derive(Debug, Clone, Default)]
pub struct LayerStage {
    layers: Vec<MapLayer>,
}

impl LayerStage {
    /// 빈 창고.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 레이어를 넣는다. 같은 키가 있으면 **교체**한다(C++ `addMapLayer` 와 같은 동작).
    pub fn upsert(&mut self, layer: MapLayer) -> &mut Self {
        if let Some(slot) = self.layers.iter_mut().find(|l| l.key == layer.key) {
            *slot = layer;
        } else {
            self.layers.push(layer);
        }
        self
    }

    /// 레이어 하나를 가져온다.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&MapLayer> {
        self.layers.iter().find(|l| l.key == key)
    }

    /// 레이어 하나를 고칠 수 있게 가져온다.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut MapLayer> {
        self.layers.iter_mut().find(|l| l.key == key)
    }

    /// 레이어를 지운다.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.layers.len();
        self.layers.retain(|l| l.key != key);
        self.layers.len() < before
    }

    /// 표시 여부를 바꾼다.
    pub fn set_visible(&mut self, key: &str, visible: bool) -> bool {
        match self.get_mut(key) {
            Some(l) => {
                l.visible = visible;
                true
            }
            None => false,
        }
    }

    /// 표시 여부를 뒤집는다.
    pub fn toggle_visible(&mut self, key: &str) -> bool {
        match self.get_mut(key) {
            Some(l) => {
                l.visible = !l.visible;
                true
            }
            None => false,
        }
    }

    /// 스타일을 바꾼다(C++ `ka_hgis/style_*` 사용자 선택 대응).
    pub fn set_style(&mut self, key: &str, style: DomainStyle) -> bool {
        match self.get_mut(key) {
            Some(l) => {
                l.style = style;
                true
            }
            None => false,
        }
    }

    /// 레이어 수.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// 비었는지.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// 그릴 순서대로(작은 z 먼저) 정렬한 목록. **보이는 레이어만**.
    ///
    /// C++ 은 레이어 트리를 아래에서 위로 그린다. 같은 순서를 만든다.
    #[must_use]
    pub fn paint_order(&self) -> Vec<&MapLayer> {
        let mut out: Vec<&MapLayer> = self.layers.iter().filter(|l| l.has_content()).collect();
        out.sort_by_key(|l| l.z_order);
        out
    }

    /// 조사 데이터 레이어만(범례·체크리스트용).
    #[must_use]
    pub fn survey_layers(&self) -> Vec<&MapLayer> {
        let mut out: Vec<&MapLayer> = self.layers.iter().filter(|l| l.is_survey_data).collect();
        out.sort_by_key(|l| l.z_order);
        out
    }

    /// 모든 레이어(표시 여부 무관) — 레이어 패널용.
    #[must_use]
    pub fn all(&self) -> &[MapLayer] {
        &self.layers
    }

    /// 전체를 비운다(새 조사).
    pub fn clear(&mut self) {
        self.layers.clear();
    }
}
