//! The word 「rust」 beside 「Strata」: how it swings, how its depth is seen, and where its letters lie on the card.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::epaint::Mesh;
use eframe::egui::{self, pos2, vec2, LayerId, RawInput, Rect, TextureId};

use super::super::paint::{paint_card, Card};
use super::super::CardFrame;
use super::{pose, project, word_left, word_meshes, Pose, DEPTH, MAX_YAW, PERIOD, SIDE_BACK, SLIDE, STILL_YAW};

/// From the front it faces the eye; a quarter of a swing later it has turned and slid fully one way, three
/// quarters later fully the other way, and after a whole swing it faces the eye again. It never goes past.
#[test]
fn the_word_swings_left_and_right_and_comes_back() {
    let at = |part: f64| pose(Some(PERIOD * part));
    assert_eq!(at(0.0), Pose { yaw: 0.0, slide: 0.0 });
    assert!((at(0.25).yaw - MAX_YAW).abs() < 1e-5 && (at(0.25).slide - SLIDE).abs() < 1e-4);
    assert!((at(0.75).yaw + MAX_YAW).abs() < 1e-5 && (at(0.75).slide + SLIDE).abs() < 1e-4);
    assert!(at(1.0).yaw.abs() < 1e-5 && at(1.0).slide.abs() < 1e-4);
    for step in 0..=120 {
        let now = pose(Some(f64::from(step) * 0.05));
        assert!(now.yaw.abs() <= MAX_YAW + 1e-6 && now.slide.abs() <= SLIDE + 1e-4, "{now:?}");
    }
    // Without motion (Windows asks for it, or KA_HGIS_REDUCED_MOTION=1) the word stands still, a little turned.
    assert_eq!(pose(None), Pose { yaw: STILL_YAW, slide: 0.0 });
}

/// Turned so that its right end goes back, the word shows its left sides: the back of a letter is seen left of
/// its face; turned the other way, right of it. The face is narrower when turned, and a slide moves it all.
#[test]
fn a_turned_word_shows_its_sides_and_a_slide_moves_it() {
    let front = Pose { yaw: 0.0, slide: 0.0 };
    assert_eq!(project([0.0, 0.0, 0.0], front), [0.0, 0.0]);
    let turned = Pose { yaw: 0.4, slide: 0.0 };
    assert!(project([0.0, 0.0, DEPTH], turned)[0] < project([0.0, 0.0, 0.0], turned)[0]);
    let back = Pose { yaw: -0.4, slide: 0.0 };
    assert!(project([0.0, 0.0, DEPTH], back)[0] > project([0.0, 0.0, 0.0], back)[0]);
    let width = |pose: Pose| project([60.0, 0.0, 0.0], pose)[0] - project([-60.0, 0.0, 0.0], pose)[0];
    assert!((width(front) - 120.0).abs() < 1e-3 && width(turned) < 115.0 && width(back) < 115.0);
    let slid = Pose { yaw: 0.0, slide: 12.0 };
    assert_eq!(project([5.0, -7.0, 0.0], slid), [17.0, -7.0]);
}

/// The layers of the word on a 700 x 400 card at `pose`, and where 「Strata」 ends.
fn layers(unit: f32, pose: Pose) -> (Vec<Mesh>, f32, Rect) {
    let ctx = egui::Context::default();
    let card_rect = Rect::from_min_size(pos2(20.0, 30.0), vec2(700.0, 400.0) * unit);
    let mut out = (Vec::new(), 0.0, card_rect);
    let _ = ctx.run(RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(LayerId::background());
        let card = Card { painter: &painter, rect: card_rect, unit };
        let strata = painter.layout_no_wrap("Strata".to_owned(), card.serif(60.0), egui::Color32::BLACK);
        out = (word_meshes(&card, word_left(&card), 140.0, pose), card.at(29.0, 0.0).x + strata.size().x, card_rect);
    });
    out
}

/// At every moment of a swing, and on a small and a large card, the letters stay right of 「Strata」, inside
/// the card, and above the levelling staff (its top is at 196 of 400).
#[test]
fn the_word_stays_beside_strata_and_above_the_staff() {
    for unit in [0.8_f32, 1.0, 1.5] {
        for step in 0..=36 {
            let (meshes, strata_right, card) = layers(unit, pose(Some(PERIOD * f64::from(step) / 36.0)));
            let points: Vec<_> = meshes.iter().flat_map(|mesh| mesh.vertices.iter().map(|v| v.pos)).collect();
            assert!(!points.is_empty());
            let (left, right) = points.iter().fold((f32::MAX, f32::MIN), |(l, r), p| (l.min(p.x), r.max(p.x)));
            let (top, bottom) = points.iter().fold((f32::MAX, f32::MIN), |(t, b), p| (t.min(p.y), b.max(p.y)));
            assert!(left >= strata_right + 8.0 * unit, "unit {unit} step {step}: {left} against {strata_right}");
            assert!(right <= card.right() - 20.0 * unit && top >= card.top() + 50.0 * unit, "unit {unit} step {step}");
            assert!(bottom <= card.top() + 196.0 * unit, "unit {unit} step {step}: {bottom}");
        }
    }
}

/// The sides are many thin copies drawn first, from the back, darker than the face; the face comes last,
/// lighter at the top than at the foot. Every layer is the font's own letters.
#[test]
fn the_sides_come_first_and_the_face_last() {
    let (meshes, _, _) = layers(1.0, Pose { yaw: 0.4, slide: 0.0 });
    assert!(meshes.len() > 10, "{} layers", meshes.len());
    let luma = |c: egui::Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
    let face = meshes.last().unwrap();
    let side_max = meshes[..meshes.len() - 1].iter().flat_map(|m| m.vertices.iter().map(|v| luma(v.color))).max().unwrap();
    let face_min = face.vertices.iter().map(|v| luma(v.color)).min().unwrap();
    assert!(side_max < face_min, "sides {side_max}, face {face_min}");
    assert!(luma(meshes[0].vertices[0].color) < luma(meshes[meshes.len() - 2].vertices[0].color), "darker at the back");
    let top = face.vertices.iter().min_by(|a, b| a.pos.y.total_cmp(&b.pos.y)).unwrap();
    let foot = face.vertices.iter().max_by(|a, b| a.pos.y.total_cmp(&b.pos.y)).unwrap();
    assert!(luma(top.color) > luma(foot.color));
    for mesh in &meshes {
        assert_eq!(mesh.texture_id, TextureId::default());
        assert!(mesh.vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv.x) && (0.0..=1.0).contains(&v.uv.y)));
    }
}

/// The card paints the word in every frame, moving or still: its back copy is there in the side colour.
#[test]
fn the_card_paints_the_word() {
    for clock in [None, Some(0.0), Some(PERIOD * 0.3)] {
        let ctx = egui::Context::default();
        let frame = CardFrame { seconds: 0.0, progress: 0.0, status: "앱을 준비하고 있습니다", version: "2.1", clock };
        let out = ctx.run(RawInput::default(), |ctx| {
            paint_card(&ctx.layer_painter(LayerId::background()), Rect::from_min_size(pos2(16.0, 10.0), vec2(700.0, 400.0)), None, &frame);
        });
        let back = out.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Mesh(mesh) if !mesh.vertices.is_empty() && mesh.vertices.iter().all(|v| v.color == SIDE_BACK)));
        assert!(back, "{clock:?}");
    }
}
