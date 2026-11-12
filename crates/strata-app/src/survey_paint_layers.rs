//! 조사 표 → 지도에 그릴 레이어(작업 좌표계 도형, 라벨 글·자리).
//!
//! C++ 은 QGIS 가 조사 파일의 레이어를 작업 좌표계로 바로 바꿔(on-the-fly) 그린다. 여기서도 같은
//! PROJ(strata-engine)로 바꾼다 — gdaltransform 과 1 mm 안(`strata-engine` crs_tests). PROJ 가 아는
//! 좌표계면 모두(옛 측지계 5174 포함) 그린다. 라벨 면적은 QGIS `area($geometry)` 처럼 표의 좌표계에서
//! 잰 평면 면적이다.

use strata_engine::Transformer;
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

/// 표 좌표계(gpkg srs_id) → 작업 좌표계 변환기. PROJ 가 모르는 좌표계면 None(그 표는 그리지 않는다).
fn transformer(srs_id: i32, work_epsg: u32) -> Option<Transformer> {
    let src = u32::try_from(srs_id).ok().filter(|s| *s > 0)?;
    Transformer::new(src, work_epsg).ok()
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

/// 도형 하나를 작업 좌표계로. PROJ 가 어느 점을 바꾸지 못하면 그 도형은 그리지 않는다(QGIS 도 건너뛴다).
fn build_feature(
    parts: &[WkbGeometry],
    tr: &Transformer,
    rule: &LabelRule,
    attrs: &std::collections::BTreeMap<String, String>,
) -> Option<PaintFeature> {
    let t = |p: &[f64; 2]| tr.transform(*p).ok();
    let all = |pts: &[[f64; 2]]| pts.iter().map(t).collect::<Option<Vec<_>>>();
    let area = planar_area(parts);
    let anchor = match label_anchor_source(parts) {
        Some((xy, dir)) => {
            let a = t(&xy)?;
            // 방향은 작은 걸음을 같이 바꿔 작업 좌표계에서 다시 잰다.
            let dir = dir.and_then(|d| {
                let b = t(&[xy[0] + d[0], xy[1] + d[1]])?;
                let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                (len > 0.0).then(|| [(b[0] - a[0]) / len, (b[1] - a[1]) / len])
            });
            Some(LabelAnchor { xy: a, dir })
        }
        None => None,
    };
    let parts = parts
        .iter()
        .map(|p| match p {
            WkbGeometry::Point(pt) => t(pt).map(PaintPart::Point),
            WkbGeometry::LineString(l) => all(l).map(PaintPart::Line),
            WkbGeometry::Polygon(r) => r.iter().map(|ring| all(ring)).collect::<Option<Vec<_>>>().map(PaintPart::Polygon),
        })
        .collect::<Option<Vec<_>>>()?;
    Some(PaintFeature { parts, label: label_text(rule, attrs, area), anchor, area })
}

/// 조사 표들을 작업 좌표계 `work` 의 그릴 레이어로. 형식·좌표계를 모르는 표는 뺀다.
pub(crate) fn build_paint_layers(tables: &[SurveyFileTable], work: CrsCode) -> Vec<PaintLayer> {
    let mut out = Vec::new();
    for t in tables {
        let (Some(kind), Some(tr)) = (GeomKind::from_type_name(&t.geometry_type), transformer(t.srs_id, work.epsg_number()))
        else {
            continue;
        };
        let rule = label_rule(&t.table, kind, &t.columns);
        let features = t
            .layer
            .features
            .iter()
            .filter_map(|f| {
                let parts = wkt_to_parts(&f.wkt_geom).ok().filter(|p| !p.is_empty())?;
                build_feature(&parts, &tr, &rule, &f.attributes)
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

/// 직사각형 [xmin, ymin, xmax, ymax] 둘레를 `f` 로 옮긴 점들의 바깥 상자 — QGIS transformBoundingBox
/// (PROJ 9.7+ 는 proj_trans_bounds 로 변마다 촘촘히 옮긴다).
pub(crate) fn outer_box_of(mut f: impl FnMut(f64, f64) -> [f64; 2], r: [f64; 4]) -> [f64; 4] {
    const STEPS: usize = 22;
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for i in 0..=STEPS {
        let t = i as f64 / STEPS as f64;
        let (x, y) = (r[0] + (r[2] - r[0]) * t, r[1] + (r[3] - r[1]) * t);
        for q in [f(x, r[1]), f(x, r[3]), f(r[0], y), f(r[2], y)] {
            b = [b[0].min(q[0]), b[1].min(q[1]), b[2].max(q[0]), b[3].max(q[1])];
        }
    }
    b
}

/// C++ `zoomToLayerMax` 의 레이어 범위 — 표 좌표계에서 도형들의 바깥 상자(QGIS `layer->extent()`)를
/// 작업 좌표계로 옮긴 바깥 상자. 도형이 없거나 좌표계를 모르면 None.
pub(crate) fn table_extent_in(t: &SurveyFileTable, work: strata_types::crs::WorkCrs) -> Option<[f64; 4]> {
    let work_epsg = CrsCode::from(work).epsg_number();
    let tr = transformer(t.srs_id, work_epsg)?;
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut add = |p: &[f64; 2]| b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
    for f in &t.layer.features {
        for part in wkt_to_parts(&f.wkt_geom).unwrap_or_default() {
            match part {
                WkbGeometry::Point(p) => add(&p),
                WkbGeometry::LineString(l) => l.iter().for_each(&mut add),
                WkbGeometry::Polygon(rings) => rings.iter().flatten().for_each(&mut add),
            }
        }
    }
    if !(b[0] <= b[2] && b[1] <= b[3]) {
        return None;
    }
    if u32::try_from(t.srs_id).ok() == Some(work_epsg) {
        return Some(b);
    }
    if b[2] - b[0] <= 0.0 || b[3] - b[1] <= 0.0 {
        // 너비나 높이가 0 인 상자(수평·수직 선, 점 하나)는 QGIS(PROJ 9.8 proj_trans_bounds)가 왼쪽 아래 한
        // 점으로 옮긴다 — qgis-dev 4.3.0 으로 확인. 그래서 C++ 은 그런 선의 시작점 둘레를 보인다.
        let p = tr.transform([b[0], b[1]]).ok()?;
        return Some([p[0], p[1], p[0], p[1]]);
    }
    let mut ok = true;
    let ext = outer_box_of(
        |x, y| match tr.transform([x, y]) {
            Ok(p) => p,
            Err(_) => {
                ok = false;
                [x, y]
            }
        },
        b,
    );
    ok.then_some(ext)
}

#[cfg(test)]
#[path = "survey_paint_layers_tests.rs"]
mod tests;
