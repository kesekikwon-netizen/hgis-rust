//! 열린 조사 파일의 레이어를 지도에 그린다(지시 3-1). 모양은 C++ `LayerStyleDefaults::forKey`,
//! 라벨은 `LabelOps` 식(`survey_style`). 범례 맨 아래 줄부터 그리고(맨 위 줄이 맨 위), 라벨은
//! QGIS 처럼 모든 도형 뒤에 그린다. 선 끝은 QGIS 기본(네모 끝, `qgslinesymbollayer.h:38`)처럼
//! 굵기 절반만큼 늘인다.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use eframe::egui::{self, text::LayoutJob, Align, Color32, FontFamily, FontId, Pos2, Rect, Shape, Stroke, Vec2};
use geo::TriangulateEarcut;

use strata_canvas2d::viewport::ViewportTransform;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::survey_paint_layers::{
    build_paint_layers, line_length, paint_order, point_along, PaintFeature, PaintLayer, PaintPart,
};
use crate::survey_style::{
    label_font_size_points, label_ink, mm_to_points, qt_premultiplied, GeomKind, LABEL_HALO_MM, LABEL_HALO_RGBA,
    LABEL_SIZE_PT,
    MARKER_OUTLINE_MM,
};

use super::gui_tab_map_geom::w2s;
use super::gui_tab_map_survey_labels::{label_bounds, paint_placed_labels, place_labels, rotate, LabelDraw, LabelReq};

/// 지도 라벨 글꼴(맑은 고딕 굵게, `LabelOps.cpp:33-35`) — `gui_theme_fonts` 가 있으면 등록한다.
pub(crate) const LABEL_FONT_FAMILY: &str = "map_label_bold";

type Cached = Arc<(u64, Vec<PaintLayer>)>;

fn rgba(c: [u8; 4]) -> Color32 {
    let [r, g, b, a] = qt_premultiplied(c);
    Color32::from_rgba_premultiplied(r, g, b, a)
}

/// 표가 바뀌었을 때만 다시 만든다(도형·속성·좌표계가 같으면 앞 프레임 것을 쓴다).
fn cached_layers(ctx: &egui::Context, tables: &[SurveyFileTable], work: CrsCode) -> Cached {
    let mut h = DefaultHasher::new();
    work.auth_id().hash(&mut h);
    for t in tables {
        (&t.table, t.srs_id, &t.geometry_type, &t.columns).hash(&mut h);
        for f in &t.layer.features {
            (f.fid, &f.wkt_geom, &f.attributes).hash(&mut h);
        }
    }
    let sig = h.finish();
    let id = egui::Id::new("ka_survey_paint_layers");
    if let Some(c) = ctx.data(|d| d.get_temp::<Cached>(id)).filter(|c| c.0 == sig) {
        return c;
    }
    let c: Cached = Arc::new((sig, build_paint_layers(tables, work)));
    ctx.data_mut(|d| d.insert_temp(id, c.clone()));
    c
}

/// 열린 선의 두 끝을 `half` 만큼 늘인다(네모 끝).
fn square_caps(pts: &[Pos2], half: f32) -> Vec<Pos2> {
    let mut out = pts.to_vec();
    let n = out.len();
    if n >= 2 {
        let d0 = (out[0] - out[1]).normalized();
        let d1 = (out[n - 1] - out[n - 2]).normalized();
        out[0] += d0 * half;
        out[n - 1] += d1 * half;
    }
    out
}

/// 오목·구멍 있는 면도 채운다(egui 의 볼록 다각형 대신 귀 자르기 삼각형).
fn fill_mesh(rings: &[Vec<Pos2>], fill: Color32) -> Shape {
    let ls = |r: &Vec<Pos2>| geo::LineString::from(r.iter().map(|p| (f64::from(p.x), f64::from(p.y))).collect::<Vec<_>>());
    let Some(outer) = rings.first() else { return Shape::Noop };
    let tri = geo::Polygon::new(ls(outer), rings[1..].iter().map(ls).collect()).earcut_triangles_raw();
    let mut mesh = egui::Mesh::default();
    for v in tri.vertices.chunks_exact(2) {
        mesh.colored_vertex(Pos2::new(v[0] as f32, v[1] as f32), fill);
    }
    mesh.indices.extend(tri.triangle_indices.iter().filter_map(|&i| u32::try_from(i).ok()));
    Shape::mesh(mesh)
}

fn paint_feature(p: &egui::Painter, rect: Rect, vp: &ViewportTransform, layer: &PaintLayer, f: &PaintFeature) {
    let s = &layer.style;
    let to = |xy: &[f64; 2]| w2s(rect, vp, *xy);
    for part in &f.parts {
        match part {
            PaintPart::Polygon(rings) => {
                let pts: Vec<Vec<Pos2>> = rings.iter().map(|r| r.iter().map(to).collect()).collect();
                if s.fill[3] > 0 {
                    p.add(fill_mesh(&pts, rgba(s.fill)));
                }
                let stroke = Stroke::new(mm_to_points(s.stroke_width_mm), rgba(s.stroke));
                for r in &pts {
                    // 닫는 점(처음과 같은 끝 점)은 빼고 닫힌 선으로 그린다.
                    let open = if r.len() > 1 && r.first() == r.last() { &r[..r.len() - 1] } else { &r[..] };
                    p.add(Shape::closed_line(open.to_vec(), stroke));
                }
            }
            PaintPart::Line(l) => {
                let pts: Vec<Pos2> = l.iter().map(to).collect();
                if let Some((c, w)) = s.casing {
                    let w = mm_to_points(w);
                    p.add(Shape::line(square_caps(&pts, w * 0.5), Stroke::new(w, rgba(c))));
                }
                let w = mm_to_points(s.stroke_width_mm);
                p.add(Shape::line(square_caps(&pts, w * 0.5), Stroke::new(w, rgba(s.stroke))));
            }
            PaintPart::Point(pt) => {
                // 기호 크기는 원 지름, 테두리 0.6 mm 는 그 둘레 가운데에 걸친다(실측 26 px = 4.6 mm × 1.5).
                let stroke = Stroke::new(mm_to_points(MARKER_OUTLINE_MM), rgba(s.stroke));
                p.circle(to(pt), mm_to_points(s.marker_size_mm) * 0.5, rgba(s.fill), stroke);
            }
        }
    }
}

fn label_font(ctx: &egui::Context) -> FontId {
    let bold = FontFamily::Name(LABEL_FONT_FAMILY.into());
    let family = if ctx.fonts(|f| f.families().contains(&bold)) { bold } else { FontFamily::Proportional };
    FontId::new(label_font_size_points(LABEL_SIZE_PT), family)
}

/// 지물에서 가장 긴 선(작업 좌표계).
fn longest_line(f: &PaintFeature) -> Option<&Vec<[f64; 2]>> {
    f.parts
        .iter()
        .filter_map(|p| match p {
            PaintPart::Line(l) => Some(l),
            _ => None,
        })
        .max_by(|a, b| line_length(a).total_cmp(&line_length(b)))
}

fn point_in_ring(pt: Pos2, ring: &[Pos2]) -> bool {
    let mut inside = false;
    for w in ring.windows(2) {
        let (a, b) = (w[0], w[1]);
        if (a.y > pt.y) != (b.y > pt.y) && pt.x < a.x + (pt.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// 라벨 하나의 후보 자리와 그릴 글. 자리가 화면 밖이면 없다.
fn label_for(ctx: &egui::Context, font: &FontId, rect: Rect, vp: &ViewportTransform, layer: &PaintLayer, f: &PaintFeature) -> Option<(LabelReq, LabelDraw)> {
    let (text, anchor) = (f.label.as_ref()?, f.anchor?);
    let a = w2s(rect, vp, anchor.xy);
    if !rect.contains(a) {
        return None;
    }
    let [r, g, b] = label_ink(&layer.rule);
    let ink = Color32::from_rgb(r, g, b);
    let mut job = LayoutJob::simple(text.clone(), font.clone(), ink, f32::INFINITY);
    job.halign = Align::Center;
    let galley = ctx.fonts(|fonts| fonts.layout_job(job));
    let size = galley.rect.size();
    let (w, h) = (size.x, size.y);
    let (angle, origins, fits_inside) = match layer.kind {
        GeomKind::Polygon => {
            let origin = a - size * 0.5;
            let biggest = f.parts.iter().filter_map(|p| match p {
                PaintPart::Polygon(r) => r.first(),
                _ => None,
            });
            let ring: Vec<Pos2> = biggest.max_by_key(|r| r.len()).map(|r| r.iter().map(|xy| w2s(rect, vp, *xy)).collect()).unwrap_or_default();
            let fits = [origin, origin + Vec2::new(w, 0.0), origin + size, origin + Vec2::new(0.0, h)].iter().all(|c| point_in_ring(*c, &ring));
            (0.0, vec![origin], fits)
        }
        GeomKind::Line => {
            // 글 폭만큼 걸친 선 구간의 양 끝을 잇는 방향(QGIS 선 따라 놓기처럼 꺾인 자리에서도 평평하게).
            // 화면은 y 가 아래로 자란다. 글이 거꾸로 서지 않게 ±90° 안으로 돌린다.
            let half_world = f64::from(w * 0.5) * vp.meters_per_pixel;
            let chord = longest_line(f).and_then(|l| {
                let mid = line_length(l) * 0.5;
                let (s, e) = (point_along(l, mid - half_world)?, point_along(l, mid + half_world)?);
                let (s, e) = (w2s(rect, vp, s), w2s(rect, vp, e));
                (s != e).then(|| e - s)
            });
            let d = anchor.dir.unwrap_or([1.0, 0.0]);
            let chord = chord.unwrap_or(Vec2::new(d[0] as f32, -d[1] as f32));
            let mut ang = chord.y.atan2(chord.x);
            if ang > std::f32::consts::FRAC_PI_2 {
                ang -= std::f32::consts::PI;
            } else if ang < -std::f32::consts::FRAC_PI_2 {
                ang += std::f32::consts::PI;
            }
            // 글 상자 아래 가운데가 선 위에 닿는다(선 위쪽).
            (ang, vec![a - rotate(Vec2::new(w * 0.5, h), ang)], true)
        }
        GeomKind::Point => {
            // QGIS 둘레 놓기(pal feature.cpp createCandidatesAroundPoint)의 값싼 차례:
            // 오른쪽 위 → 오른쪽 → 위 → 오른쪽 아래 → 왼쪽 위 → 아래 → 왼쪽 → 왼쪽 아래.
            let rad = mm_to_points(layer.style.marker_size_mm + MARKER_OUTLINE_MM) * 0.5;
            let k = rad * std::f32::consts::FRAC_1_SQRT_2;
            let offs = [
                Vec2::new(k, -k - h),
                Vec2::new(rad, -h * 0.5),
                Vec2::new(-w * 0.5, -rad - h),
                Vec2::new(k, k),
                Vec2::new(-k - w, -k - h),
                Vec2::new(-w * 0.5, rad),
                Vec2::new(-rad - w, -h * 0.5),
                Vec2::new(-k - w, k),
            ];
            (0.0, offs.iter().map(|o| a + *o).collect(), true)
        }
    };
    let candidates = origins.iter().map(|o| label_bounds(*o, size, angle)).collect();
    Some((LabelReq { candidates, fits_inside, area: f.area }, LabelDraw { galley, ink, angle, origins }))
}

impl StrataAppState {
    /// 열린 조사 파일의 표를 범례 차례·표시·라벨 체크대로 그린다.
    pub(crate) fn paint_survey_tables(&self, ctx: &egui::Context, p: &egui::Painter, rect: Rect) {
        if self.survey_tables.is_empty() {
            return;
        }
        let cached = cached_layers(ctx, &self.survey_tables, self.work_crs.into());
        let layers = &cached.1;
        let rs = &self.home_map.real_survey;
        let vp = &self.home_map.viewport;
        let font = label_font(ctx);
        let (mut reqs, mut draws) = (Vec::new(), Vec::new());
        for i in paint_order(&self.left_layers.entries, layers) {
            let layer = &layers[i];
            if !rs.is_layer_visible(&layer.title) {
                continue;
            }
            for f in &layer.features {
                paint_feature(p, rect, vp, layer, f);
            }
            if rs.is_label_visible(&layer.title) {
                for (req, draw) in layer.features.iter().filter_map(|f| label_for(ctx, &font, rect, vp, layer, f)) {
                    reqs.push(req);
                    draws.push(draw);
                }
            }
        }
        let chosen = place_labels(&reqs);
        paint_placed_labels(p, &draws, &chosen, mm_to_points(LABEL_HALO_MM), rgba(LABEL_HALO_RGBA));
    }
}
