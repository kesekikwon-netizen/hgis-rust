//! 조사 레이어 라벨의 자리 고르기와 그리기 — C++ LabelOps 의 배치: 면은 무게중심(OverPoint),
//! 선은 선 위 가운데(Line, 선 위쪽), 점은 둘레(AroundPoint).
//!
//! QGIS 라벨 엔진(PAL)은 겹치는 라벨을 전체 비용으로 고른다. 그것을 다시 만들지 않고 C++ 화면에
//! 맞춘 가까운 규칙으로 고른다(`gui_tab_map_survey_labels_tests.rs`): 자리가 하나뿐인 라벨(면·선)
//! 먼저, 그중 면 안에 드는 라벨 먼저, 큰 면 먼저. 점 라벨은 그 뒤에 후보 자리 중 첫 빈 곳.
//! 빈 곳이 없으면 그리지 않는다.

use std::sync::Arc;

use eframe::egui::{epaint::TextShape, Color32, Galley, Painter, Pos2, Rect, Vec2};

/// 라벨 하나의 후보 자리(화면 글 상자 — QGIS 처럼 흰 테는 겹침에 넣지 않는다)와 고르는 차례에 쓰는 값.
#[derive(Debug)]
pub(crate) struct LabelReq {
    pub(crate) candidates: Vec<Rect>,
    pub(crate) fits_inside: bool,
    pub(crate) area: f64,
}

/// 라벨마다 고른 후보 번호(None = 그리지 않음).
pub(crate) fn place_labels(reqs: &[LabelReq]) -> Vec<Option<usize>> {
    let mut order: Vec<usize> = (0..reqs.len()).collect();
    // 안정 정렬 — 같으면 받은 차례(범례 아래 줄부터)를 지킨다.
    order.sort_by(|&a, &b| {
        let (ra, rb) = (&reqs[a], &reqs[b]);
        (ra.candidates.len() > 1)
            .cmp(&(rb.candidates.len() > 1))
            .then(rb.fits_inside.cmp(&ra.fits_inside))
            .then(rb.area.total_cmp(&ra.area))
    });
    let mut placed: Vec<Rect> = Vec::new();
    let mut out = vec![None; reqs.len()];
    for i in order {
        let free = reqs[i].candidates.iter().position(|c| !placed.iter().any(|p| p.intersects(*c)));
        if let Some(c) = free {
            placed.push(reqs[i].candidates[c]);
            out[i] = Some(c);
        }
    }
    out
}

/// 그릴 라벨 하나: 글(여러 줄은 가운데 맞춤), 각도(라디안), 후보마다 글 상자의 왼쪽 위(돌리기 전).
#[derive(Debug)]
pub(crate) struct LabelDraw {
    pub(crate) galley: Arc<Galley>,
    pub(crate) ink: Color32,
    pub(crate) angle: f32,
    pub(crate) origins: Vec<Pos2>,
}

pub(crate) fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// 글 상자(크기 `size`)를 `origin` 에서 `angle` 만큼 돌려 둘 때의 화면 바깥 상자.
pub(crate) fn label_bounds(origin: Pos2, size: Vec2, angle: f32) -> Rect {
    let corners = [Vec2::ZERO, Vec2::new(size.x, 0.0), size, Vec2::new(0.0, size.y)].map(|c| origin + rotate(c, angle));
    Rect::from_points(&corners)
}

/// 고른 자리에 흰 테(`LabelOps.cpp:40-43` buffer 0.8 mm)를 먼저, 그 위에 글을 그린다.
/// 흰 테는 글을 둘레로 조금씩 옮겨 그려 만든다.
pub(crate) fn paint_placed_labels(p: &Painter, draws: &[LabelDraw], chosen: &[Option<usize>], halo: f32, halo_color: Color32) {
    let mut offsets = Vec::new();
    for (n, r) in [(16, halo), (8, halo * 0.5)] {
        for i in 0..n {
            let a = std::f32::consts::TAU * (i as f32) / (n as f32);
            offsets.push(Vec2::new(a.cos() * r, a.sin() * r));
        }
    }
    for (d, c) in draws.iter().zip(chosen) {
        let Some(ci) = c else { continue };
        let Some(origin) = d.origins.get(*ci) else { continue };
        let pos = *origin - rotate(d.galley.rect.min.to_vec2(), d.angle);
        for off in &offsets {
            p.add(TextShape::new(pos + *off, d.galley.clone(), halo_color).with_override_text_color(halo_color).with_angle(d.angle));
        }
        p.add(TextShape::new(pos, d.galley.clone(), d.ink).with_override_text_color(d.ink).with_angle(d.angle));
    }
}

#[cfg(test)]
#[path = "gui_tab_map_survey_labels_tests.rs"]
mod tests;
