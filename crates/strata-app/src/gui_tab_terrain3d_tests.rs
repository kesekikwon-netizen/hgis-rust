//! F1g — `KaTerrain3dStudio` without a 3D scene (`target/cpp-ref/src/app/KaTerrain3dStudio.cpp:84-149`,
//! `:464-469`, `:541-546`; `target/cpp-ref/src/core/Terrain3dService.cpp:237-242`).
//! Ported expectations from `target/cpp-ref/tests/test_terrain_3d.cpp` (`studio_hidesManualChrome`,
//! `renderPerspective_paperWhiteAndBright`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Color32, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, Vec2};

use crate::app_state::{MainTab, StrataAppState};

#[derive(Default)]
struct FrameOut {
    texts: Vec<String>,
    nodes: Vec<(String, bool, Option<Rect>)>,
    shapes: Vec<egui::epaint::ClippedShape>,
}

fn collect_shape_texts(shapes: &[Shape], out: &mut Vec<String>) {
    for s in shapes {
        match s {
            Shape::Text(t) => out.push(t.galley.job.text.clone()),
            Shape::Vec(v) => collect_shape_texts(v, out),
            _ => {}
        }
    }
}

fn run_frame(ctx: &egui::Context, time: f64, events: Vec<Event>, mut paint: impl FnMut(&egui::Context)) -> FrameOut {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0))),
        time: Some(time),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, &mut paint);
    let mut texts = Vec::new();
    for cs in &out.shapes {
        collect_shape_texts(std::slice::from_ref(&cs.shape), &mut texts);
    }
    let nodes = out.platform_output.accesskit_update.map_or_else(Vec::new, |u| {
        u.nodes
            .iter()
            .filter_map(|(_, n)| {
                let bounds = n.bounds().map(|b| {
                    Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
                });
                n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
            })
            .collect()
    });
    FrameOut { texts, nodes, shapes: out.shapes }
}

fn click_button(app: &mut StrataAppState, ctx: &egui::Context, label: &str) -> FrameOut {
    let mut last = FrameOut::default();
    for step in 0..3 {
        last = run_frame(ctx, f64::from(step) * 0.1, vec![], |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }
    let (_, _, bounds) = last
        .nodes
        .iter()
        .find(|(l, _, _)| l == label)
        .cloned()
        .unwrap_or_else(|| panic!("missing button {label:?} in {:?}", last.nodes));
    let at = bounds.expect("button bounds").center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    for (t, ev) in [(0.4, Event::PointerMoved(at)), (0.5, press(true)), (0.6, press(false))] {
        let _ = run_frame(ctx, t, vec![ev], |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }
    run_frame(ctx, 0.7, vec![], |ctx| {
        let _ = app.render_workspace_egui(ctx);
    })
}

fn assert_plain_white_right_view(shapes: &[egui::epaint::ClippedShape]) {
    fn walk(shape: &Shape, white_rects: &mut usize) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, white_rects)),
            Shape::Rect(r) if r.rect.left() > 190.0 => {
                assert!(
                    r.fill == Color32::WHITE || r.fill == Color32::TRANSPARENT,
                    "non-white rect in right view: {:?} fill={:?}",
                    r.rect,
                    r.fill
                );
                if r.fill == Color32::WHITE && r.rect.width() > 400.0 {
                    *white_rects += 1;
                }
            }
            Shape::Text(t) => assert!(t.pos.x <= 270.0, "unexpected text in right view: {:?}", t.galley.job.text),
            Shape::LineSegment { points, .. } => {
                assert!(points[0].x <= 270.0 && points[1].x <= 270.0, "unexpected line in right view: {points:?}");
            }
            Shape::Mesh(m) => {
                assert!(m.vertices.iter().all(|v| v.pos.x <= 270.0), "unexpected mesh in right view");
            }
            Shape::Path(p) => {
                assert!(p.points.iter().all(|pt| pt.x <= 270.0), "unexpected path in right view");
            }
            _ => {}
        }
    }
    let mut white_rects = 0usize;
    shapes.iter().for_each(|cs| walk(&cs.shape, &mut white_rects));
    assert!(white_rects >= 1, "right view must paint a plain white rect");
}

#[test]
fn panel_shows_cpp_texts_in_cpp_order_and_plain_white_view() {
    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Terrain3d);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();

    let mut out = FrameOut::default();
    for step in 0..3 {
        out = run_frame(&ctx, f64::from(step) * 0.1, vec![], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_terrain3d_tab_ui(ctx, ui));
        });
    }

    let expected_order = [
        "입체지형",
        "지금 지도에 보이는 화면을 고해상 입체로 만듭니다. 탭을 열면 자동으로 시작합니다.",
        "화면을 입체로",
        "이미지 저장",
        "입체지형 도면출력",
        "높이 과장",
        "2.0 배",
        "아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)",
    ];
    let mut prev_idx: Option<usize> = None;
    for expected in expected_order {
        let start = prev_idx.map_or(0, |i| i + 1);
        let rel = out.texts[start..]
            .iter()
            .position(|t| t == expected)
            .unwrap_or_else(|| panic!("expected {expected:?} after index {start} in {:?}", out.texts));
        prev_idx = Some(start + rel);
    }
    assert_eq!(out.texts.iter().filter(|t| t.as_str() == "2.0 배").count(), 1);

    for forbidden in [
        "표면에 입힐 그림",
        "시점 프리셋",
        "사선 조감",
        "수직 조감",
        "현장 시점",
        "DEM 160×160",
        "굴착 토량",
        "드레이핑",
        "500 m",
        "KaTerrain3dView",
        "DEM 열기",
        "파일함",
    ] {
        assert!(!out.texts.iter().any(|t| t.contains(forbidden)), "unexpected {forbidden:?}");
    }
    assert_plain_white_right_view(&out.shapes);

    // Dragging and wheel over the right view change nothing visible.
    let view_pos = Pos2::new(760.0, 450.0);
    let _ = run_frame(
        &ctx,
        0.5,
        vec![
            Event::PointerMoved(view_pos),
            Event::PointerButton { pos: view_pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() },
        ],
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_terrain3d_tab_ui(ctx, ui));
        },
    );
    let after_drag = run_frame(
        &ctx,
        0.6,
        vec![
            Event::PointerMoved(Pos2::new(820.0, 490.0)),
            Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: Vec2::new(0.0, 3.0), modifiers: Modifiers::default() },
        ],
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_terrain3d_tab_ui(ctx, ui));
        },
    );
    assert_plain_white_right_view(&after_drag.shapes);
}

#[test]
fn autofill_button_is_disabled_and_carries_w56_tooltip() {
    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Terrain3d);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();

    let deferred = "아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)";
    let mut base = FrameOut::default();
    for step in 0..3 {
        base = run_frame(&ctx, f64::from(step) * 0.1, vec![], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_terrain3d_tab_ui(ctx, ui));
        });
    }
    let (_, disabled, bounds) = base
        .nodes
        .iter()
        .find(|(l, _, _)| l == "화면을 입체로")
        .cloned()
        .unwrap_or_else(|| panic!("missing 화면을 입체로 in {:?}", base.nodes));
    assert!(disabled, "화면을 입체로 must be disabled until W-56");
    assert_eq!(base.texts.iter().filter(|t| t.as_str() == deferred).count(), 1);

    let at = bounds.expect("bounds").center();
    let mut hovered = base;
    for step in 1..=3 {
        hovered = run_frame(&ctx, f64::from(step), vec![Event::PointerMoved(at)], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_terrain3d_tab_ui(ctx, ui));
        });
    }
    assert_eq!(hovered.texts.iter().filter(|t| t.as_str() == deferred).count(), 2);
}

#[test]
fn save_image_click_sets_cpp_text_and_writes_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("demo_copy.gpkg");
    std::fs::write(&gpkg, b"").expect("seed path");

    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(gpkg);
    app.open_studio_tab(MainTab::Terrain3d);
    let before_entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).map(|e| e.file_name()).collect();

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let after = click_button(&mut app, &ctx, "이미지 저장");

    let cpp_msg = "먼저 지도 화면을 입체로 만드세요.";
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(cpp_msg));
    assert!(after.texts.iter().any(|t| t.contains(cpp_msg)));
    assert!(!after.texts.iter().any(|t| t.contains("저장함")));
    assert!(app.modals.active.is_none());

    let after_entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().filter_map(Result::ok).map(|e| e.file_name()).collect();
    assert_eq!(after_entries, before_entries, "no file may be created");
    assert!(!std::path::Path::new("입체지형.png").exists());
}

#[test]
fn export_sheet_click_sets_cpp_text_and_keeps_tab_and_layout_flag() {
    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Terrain3d);
    assert!(!app.drawing.terrain3d_layout_active);

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let after = click_button(&mut app, &ctx, "입체지형 도면출력");

    let cpp_msg = "먼저 지도 화면을 입체로 만드세요.";
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(cpp_msg));
    assert!(after.texts.iter().any(|t| t.contains(cpp_msg)));
    assert_eq!(app.active_tab, MainTab::Terrain3d);
    assert!(!app.drawing.terrain3d_layout_active);
}
