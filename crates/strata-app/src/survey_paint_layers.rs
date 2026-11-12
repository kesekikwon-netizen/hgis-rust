//! 조사 표 → 지도에 그릴 레이어(작업 좌표계 도형, 라벨 글·자리).
//!
//! C++ 은 QGIS 가 조사 파일의 레이어를 작업 좌표계로 바로 바꿔(on-the-fly) 그린다. 여기서는
//! strata-geo 의 TM 변환으로 바꾼다(PROJ 와 demo 모서리 차이 7.9 mm, `crs_transform_tests.rs`).
//! 라벨 면적은 QGIS `area($geometry)` 처럼 표의 좌표계에서 잰 평면 면적이다.

use strata_geo::crs_transform::CrsTransformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;

use crate::left_layer_panel::LayerTreeEntry;
use crate::survey_style::{domain_style_for_key, label_rule, label_text, DomainStyle, GeomKind, LabelRule};

#[derive(Debug, Clone)]
pub(crate) enum PaintPart {
    Point([f64; 2]),
    Line(Vec<[f64; 2]>),
    Polygon(Vec<Vec<[f64; 2]>>),
}

/// 라벨 자리(작업 좌표계). 선 라벨은 그 자리의 선 방향(단위 벡터)도 갖는다.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LabelAnchor {
    pub(crate) xy: [f64; 2],
    pub(crate) dir: Option<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub(crate) struct PaintFeature {
    pub(crate) parts: Vec<PaintPart>,
    pub(crate) label: Option<String>,
    pub(crate) anchor: Option<LabelAnchor>,
    /// 표의 좌표계에서 잰 면 면적(라벨 겹침 차례에 쓴다).
    pub(crate) area: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct PaintLayer {
    pub(crate) table: String,
    /// 범례 이름 — 표시·라벨 체크가 이 이름으로 저장된다.
    pub(crate) title: String,
    pub(crate) kind: GeomKind,
    pub(crate) style: DomainStyle,
    pub(crate) rule: LabelRule,
    pub(crate) features: Vec<PaintFeature>,
}

/// 바꿀 수 있는 조사 파일 좌표계. 5174(Bessel)는 strata-geo 에 측지계 변환이 없어 그리지 않는다.
fn srs_code(srs_id: i32) -> Option<CrsCode> {
    match srs_id {
        5186 => Some(CrsCode::Epsg5186),
        5187 => Some(CrsCode::Epsg5187),
        5179 => Some(CrsCode::Epsg5179),
        4326 => Some(CrsCode::Epsg4326),
        3857 => Some(CrsCode::Epsg3857),
        _ => None,
    }
}

fn ring_signed_area(ring: &[[f64; 2]]) -> f64 {
    ring.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() * 0.5
}

fn polygon_area(rings: &[Vec<[f64; 2]>]) -> f64 {
    let mut it = rings.iter();
    let outer = it.next().map_or(0.0, |r| ring_signed_area(r).abs());
    outer - it.map(|r| ring_signed_area(r).abs()).sum::<f64>()
}

/// QGIS `area($geometry)` — 평면 면적, 구멍은 뺀다. 면이 아니면 0.
pub(crate) fn planar_area(parts: &[WkbGeometry]) -> f64 {
    parts
        .iter()
        .map(|p| match p {
            WkbGeometry::Polygon(rings) => polygon_area(rings),
            _ => 0.0,
        })
        .sum()
}

/// 면 하나의 무게중심(구멍 반영).
fn polygon_centroid(rings: &[Vec<[f64; 2]>]) -> Option<[f64; 2]> {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for (i, ring) in rings.iter().enumerate() {
        let s = ring_signed_area(ring);
        let sign = if (i == 0) == (s >= 0.0) { 1.0 } else { -1.0 };
        for w in ring.windows(2) {
            let cross = w[0][0] * w[1][1] - w[1][0] * w[0][1];
            cx += sign * (w[0][0] + w[1][0]) * cross;
            cy += sign * (w[0][1] + w[1][1]) * cross;
        }
        a += sign * s;
    }
    (a.abs() > f64::EPSILON).then(|| [cx / (6.0 * a), cy / (6.0 * a)])
}

pub(crate) fn line_length(pts: &[[f64; 2]]) -> f64 {
    pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// 선을 따라 `d` 만큼 간 점 — 선 밖이면 가까운 끝.
pub(crate) fn point_along(pts: &[[f64; 2]], d: f64) -> Option<[f64; 2]> {
    let (first, last) = (pts.first()?, pts.last()?);
    if d <= 0.0 {
        return Some(*first);
    }
    let mut left = d;
    for w in pts.windows(2) {
        let len = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if len > 0.0 && left <= len {
            let t = left / len;
            return Some([w[0][0] + t * (w[1][0] - w[0][0]), w[0][1] + t * (w[1][1] - w[0][1])]);
        }
        left -= len;
    }
    Some(*last)
}

/// 선 길이의 가운데 점과 그 자리의 방향.
fn line_middle(pts: &[[f64; 2]]) -> Option<([f64; 2], [f64; 2])> {
    let mut left = line_length(pts) * 0.5;
    for w in pts.windows(2) {
        let len = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if len > 0.0 && left <= len {
            let t = left / len;
            let dir = [(w[1][0] - w[0][0]) / len, (w[1][1] - w[0][1]) / len];
            return Some(([w[0][0] + t * (w[1][0] - w[0][0]), w[0][1] + t * (w[1][1] - w[0][1])], dir));
        }
        left -= len;
    }
    None
}

/// 라벨 자리(표의 좌표계). 여러 부분이면 QGIS 처럼 가장 큰 부분(면적·길이)에 붙인다.
fn label_anchor_source(parts: &[WkbGeometry]) -> Option<([f64; 2], Option<[f64; 2]>)> {
    let size = |p: &WkbGeometry| match p {
        WkbGeometry::Polygon(r) => polygon_area(r),
        WkbGeometry::LineString(l) => line_length(l),
        WkbGeometry::Point(_) => 0.0,
    };
    let biggest = parts.iter().max_by(|a, b| size(a).total_cmp(&size(b)))?;
    match biggest {
        WkbGeometry::Point(p) => Some((*p, None)),
        WkbGeometry::LineString(l) => line_middle(l).map(|(xy, d)| (xy, Some(d))),
        WkbGeometry::Polygon(r) => polygon_centroid(r).map(|xy| (xy, None)),
    }
}

fn build_feature(parts: &[WkbGeometry], tr: &CrsTransformer, rule: &LabelRule, attrs: &std::collections::BTreeMap<String, String>) -> PaintFeature {
    let t = |p: &[f64; 2]| tr.transform_xy(p[0], p[1]);
    let area = planar_area(parts);
    let anchor = label_anchor_source(parts).map(|(xy, dir)| {
        let a = t(&xy);
        // 방향은 작은 걸음을 같이 바꿔 작업 좌표계에서 다시 잰다.
        let dir = dir.and_then(|d| {
            let b = t(&[xy[0] + d[0], xy[1] + d[1]]);
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            (len > 0.0).then(|| [(b[0] - a[0]) / len, (b[1] - a[1]) / len])
        });
        LabelAnchor { xy: a, dir }
    });
    let parts = parts
        .iter()
        .map(|p| match p {
            WkbGeometry::Point(pt) => PaintPart::Point(t(pt)),
            WkbGeometry::LineString(l) => PaintPart::Line(l.iter().map(t).collect()),
            WkbGeometry::Polygon(r) => PaintPart::Polygon(r.iter().map(|ring| ring.iter().map(t).collect()).collect()),
        })
        .collect();
    PaintFeature { parts, label: label_text(rule, attrs, area), anchor, area }
}

/// 조사 표들을 작업 좌표계 `work` 의 그릴 레이어로. 형식·좌표계를 모르는 표는 뺀다.
pub(crate) fn build_paint_layers(tables: &[SurveyFileTable], work: CrsCode) -> Vec<PaintLayer> {
    let mut out = Vec::new();
    for t in tables {
        let (Some(kind), Some(src)) = (GeomKind::from_type_name(&t.geometry_type), srs_code(t.srs_id)) else {
            continue;
        };
        let tr = CrsTransformer::new(src, work);
        let rule = label_rule(&t.table, kind, &t.columns);
        let features = t
            .layer
            .features
            .iter()
            .filter_map(|f| {
                let parts = wkt_to_parts(&f.wkt_geom).ok().filter(|p| !p.is_empty())?;
                Some(build_feature(&parts, &tr, &rule, &f.attributes))
            })
            .collect();
        out.push(PaintLayer {
            table: t.table.clone(),
            title: crate::gui_tab_home::saved_gpkg_layer_title(&t.table),
            kind,
            style: domain_style_for_key(&t.table),
            rule,
            features,
        });
    }
    out
}

/// 그리는 차례 — 범례 맨 아래 줄부터(맨 위 줄이 맨 위에). 범례에 없는 표는 그리지 않는다.
pub(crate) fn paint_order(entries: &[LayerTreeEntry], layers: &[PaintLayer]) -> Vec<usize> {
    entries
        .iter()
        .rev()
        .filter_map(|e| layers.iter().position(|l| l.table == e.layer_key))
        .collect()
}

#[cfg(test)]
#[path = "survey_paint_layers_tests.rs"]
mod tests;
