//! F1d-1 — the GPS control-point dialog starts with empty id/x/y/z and an empty origin ComboBox selection,
//! while keeping datum, ellipsoid, projection, accuracy, pdop and fix
//! (target/cpp-ref/src/app/KaControlPointDialog.cpp:16-19,60-71).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Pos2, RawInput, Rect, Shape};

use crate::app_state::StrataAppState;

fn collect_texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.job.text.clone()),
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_texts(s, out);
            }
        }
        _ => {}
    }
}

fn painted_texts(app: &mut StrataAppState) -> Vec<String> {
    let ctx = egui::Context::default();
    let mut texts = Vec::new();
    for _ in 0..3 {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..RawInput::default()
        };
        let out = ctx.run(raw, |ctx| {
            let mut open = true;
            super::render_control_point(app, ctx, &mut open);
        });
        texts.clear();
        for clipped in &out.shapes {
            collect_texts(&clipped.shape, &mut texts);
        }
    }
    texts
}

#[test]
fn origin_combo_starts_with_empty_selection() {
    let mut app = StrataAppState::new_startup();
    let texts = painted_texts(&mut app);

    for origin in ["서부원점", "중부원점", "동부원점", "동해원점", "단일원점(UTM-K)"] {
        assert!(
            !texts.iter().any(|t| t == origin),
            "origin ComboBox must start empty, found {origin:?} in {texts:?}"
        );
    }
    for kept in ["세계측지계", "GRS80", "TM/UTM-K", "1.0", "1.5", "RTK"] {
        assert!(
            texts.iter().any(|t| t == kept),
            "expected default {kept:?} to remain painted in {texts:?}"
        );
    }
}
