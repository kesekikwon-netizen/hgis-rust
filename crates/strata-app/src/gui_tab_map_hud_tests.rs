//! Guide band title, `— ` hint, paper-look live sketch measure, icon, and layout tests
//! (`target/cpp-ref/src/app/KaDrawGuideBand.cpp:28-53,129-158`,
//! `target/cpp-ref/src/app/KaDrawSketchTools.cpp:173-237`,
//! `target/cpp-ref/src/app/MainWindowChrome.cpp:71-82`,
//! `target/cpp-ref/tests/test_sketch_measure.cpp:13-46`,
//! `target/cpp-ref/tests/test_shell_chrome.cpp:217-241`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Color32, FontFamily, Pos2, RawInput, Rect, Shape, Vec2};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

use super::MapToolFlags;

const ACCENT: Color32 = Color32::from_rgb(181, 87, 58);

fn draw_flags(tool: MapToolKind, cursor_world: Option<[f64; 2]>) -> MapToolFlags {
    MapToolFlags {
        is_trench: false,
        is_measure: false,
        is_draw_tool: true,
        is_select_tool: false,
        is_style: false,
        effective_tool: tool,
        cursor_world,
    }
}

fn paint_hud(app: &mut StrataAppState, input: MapToolFlags) -> (Rect, Vec<Shape>) {
    let ctx = egui::Context::default();
    let canvas = Rect::from_min_size(Pos2::new(100.0, 80.0), Vec2::new(1100.0, 600.0));
    let out = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let p = ui.painter_at(canvas);
            app.paint_map_hud(ctx, ui, &p, canvas, input);
        });
    });
    (canvas, out.shapes.into_iter().map(|c| c.shape).collect())
}

fn find_pill_rect(shapes: &[Shape]) -> Option<Rect> {
    shapes.iter().find_map(|s| match s {
        Shape::Rect(r) if r.stroke.color == ACCENT => Some(r.rect),
        _ => None,
    })
}

fn find_rect_of_width(shapes: &[Shape], width: f32) -> Option<Rect> {
    shapes.iter().find_map(|s| match s {
        Shape::Rect(r) if (r.rect.width() - width).abs() < 0.5 => Some(r.rect),
        _ => None,
    })
}

fn pill_texts(shapes: &[Shape], pill: Rect) -> Vec<(String, Pos2, f32, bool)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Text(t) if pill.contains(t.pos) => {
                let mono = t
                    .galley
                    .job
                    .sections
                    .first()
                    .is_some_and(|sec| sec.format.font_id.family == FontFamily::Monospace);
                Some((t.galley.text().to_owned(), t.pos, t.galley.size().x, mono))
            }
            _ => None,
        })
        .collect()
}

fn pill_strings(shapes: &[Shape]) -> Vec<String> {
    pill_texts(shapes, find_pill_rect(shapes).unwrap()).into_iter().map(|x| x.0).collect()
}

fn glyph_signature_in_hud(shapes: &[Shape], glyph_box: Rect) -> Vec<String> {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::LineSegment { points, stroke }
                if stroke.color == ACCENT && glyph_box.expand(2.0).contains(points[0]) =>
            {
                Some(format!("L:{:.1},{:.1}-{:.1},{:.1}", points[0].x, points[0].y, points[1].x, points[1].y))
            }
            Shape::Path(p)
                if p.stroke.color == eframe::epaint::ColorMode::Solid(ACCENT)
                    && p.points.first().is_some_and(|pt| glyph_box.expand(2.0).contains(*pt)) =>
            {
                Some(format!("P:{}:{}:{:.1},{:.1}", p.closed, p.points.len(), p.points[0].x, p.points[0].y))
            }
            Shape::Circle(c)
                if (c.stroke.color == ACCENT || c.fill == ACCENT) && glyph_box.expand(2.0).contains(c.center) =>
            {
                Some(format!("C:{:.1},{:.1}:{:.1}", c.center.x, c.center.y, c.radius))
            }
            Shape::Rect(r)
                if r.stroke.color == ACCENT && r.rect.width() < 24.0 && glyph_box.expand(2.0).contains(r.rect.center()) =>
            {
                Some(format!("R:{:.1},{:.1}:{:.1}x{:.1}", r.rect.min.x, r.rect.min.y, r.rect.width(), r.rect.height()))
            }
            _ => None,
        })
        .collect()
}

fn expected_svg_signature(canvas: Rect, icon_id: &str) -> Vec<String> {
    let ctx = egui::Context::default();
    let glyph_rect = Rect::from_min_size(Pos2::new(canvas.left() + 22.0, canvas.top() + 20.0), Vec2::splat(20.0));
    let out = ctx.run(RawInput::default(), |ctx| {
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("icon_ref")));
        assert!(crate::svg_path_paint::paint_svg_icon(&p, glyph_rect, icon_id, ACCENT));
    });
    let shapes: Vec<Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    glyph_signature_in_hud(&shapes, glyph_rect)
}

#[test]
fn guide_band_shows_tool_chip_title_hint_and_cursor_measure_for_line_and_polygon() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.survey_path = Some(std::env::temp_dir().join("hud.gpkg"));
    app.display_options.paper_look = true;
    let hint = "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)";

    app.active_tool = MapToolKind::DrawLine;
    app.edit_layer_key = Some("feature_line".to_owned());
    let (_, s0) = paint_hud(&mut app, draw_flags(MapToolKind::DrawLine, Some([50.0, 50.0])));
    assert_eq!(pill_strings(&s0), vec!["그리기 · 유구 선", hint]);

    app.edit_layer_key = Some("section_line".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0]];
    let (_, s1) = paint_hud(&mut app, draw_flags(MapToolKind::DrawLine, None));
    assert_eq!(pill_strings(&s1), vec!["그리기 · 단면선 · 점 1", hint]);

    app.edit_layer_key = Some("feature_line".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0], [3.0, 4.0]];
    let (_, s2) = paint_hud(&mut app, draw_flags(MapToolKind::DrawLine, Some([3.0, 10.0])));
    assert_eq!(pill_strings(&s2), vec!["그리기 · 유구 선 · 점 2", hint, "변 6.00 m · 길이 11.00 m"]);

    app.active_tool = MapToolKind::DrawPolygon;
    app.edit_layer_key = Some("feature_poly".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0]];
    let (_, s3) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, Some([4.2, 0.0])));
    assert_eq!(pill_strings(&s3), vec!["그리기 · 유구 면 · 점 1", hint, "변 4.20 m"]);

    app.draw_sketch.points = vec![[0.0, 0.0], [10.0, 0.0]];
    let (_, s4) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, Some([10.0, 5.0])));
    assert_eq!(pill_strings(&s4), vec!["그리기 · 유구 면 · 점 2", hint, "변 5.00 m · 면적 25.0 ㎡"]);
}

// Ported from target/cpp-ref/tests/test_sketch_measure.cpp:25-46 (guideBandShowsTheFiguresOnlyInPaper).
#[test]
fn guide_band_shows_measure_only_in_paper_look_after_hint() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.survey_path = Some(std::env::temp_dir().join("hud_paper.gpkg"));
    app.active_tool = MapToolKind::DrawPolygon;
    app.edit_layer_key = Some("feature_poly".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0], [10.0, 0.0]];

    app.display_options.paper_look = false;
    let (_, stock_shapes) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, Some([10.0, 5.0])));
    assert_eq!(pill_strings(&stock_shapes), vec!["그리기 · 유구 면 · 점 2", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"]);

    app.display_options.paper_look = true;
    let (_, paper_shapes) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, Some([10.0, 5.0])));
    let paper_texts = pill_texts(&paper_shapes, find_pill_rect(&paper_shapes).unwrap());
    assert_eq!(paper_texts.len(), 3, "{paper_texts:?}");
    assert_eq!(paper_texts[0].0, "그리기 · 유구 면 · 점 2");
    assert_eq!(paper_texts[1].0, "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)");
    assert_eq!(paper_texts[2].0, "변 5.00 m · 면적 25.0 ㎡");
    assert!(paper_texts[2].3, "measure label must use monospace font");
    assert!(paper_texts[2].1.x >= paper_texts[1].1.x + paper_texts[1].2, "measure sits after hint");

    app.draw_sketch.points.clear();
    let (_, empty_shapes) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, None));
    assert_eq!(pill_strings(&empty_shapes), vec!["그리기 · 유구 면", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"]);
}

// Ported from target/cpp-ref/tests/test_shell_chrome.cpp:217-241 (band_toolPillPushesOpacityRailDown).
#[test]
fn guide_band_paints_em_dash_hint_label_at_12_12_height_36() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.active_tool = MapToolKind::DrawPolygon;
    app.edit_layer_key = Some("feature_poly".to_owned());

    let (canvas, shapes) = paint_hud(&mut app, draw_flags(MapToolKind::DrawPolygon, None));
    let pill = find_pill_rect(&shapes).expect("pill visible");
    assert_eq!(pill.min - canvas.min, Vec2::new(12.0, 12.0));
    assert_eq!(pill.height(), 36.0);
    assert_eq!(pill_strings(&shapes), vec!["그리기 · 유구 면", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"]);

    let glyph_box = Rect::from_min_size(Pos2::new(pill.left() + 10.0, pill.center().y - 10.0), Vec2::splat(20.0));
    assert!(!glyph_signature_in_hud(&shapes, glyph_box).is_empty(), "tool glyph painted");

    let undo_rect = find_rect_of_width(&shapes, 68.0).expect("undo/redo pair");
    assert!((undo_rect.left() - (pill.right() + 8.0)).abs() < 0.5);
    let rail_rect = find_rect_of_width(&shapes, 260.0).expect("opacity rail");
    assert_eq!(rail_rect.min - canvas.min, Vec2::new(12.0, 56.0));
    assert!(!pill.intersects(rail_rect), "opacity rail sits below the band");

    // Empty hint keeps the pill and title visible while hiding the `— ` hint label (:237-238).
    let ctx = egui::Context::default();
    let empty_hint_out = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let p = ui.painter_at(canvas);
            let spec = super::GuideBandSpec { icon_id: "gps", title: "그리기 · 기준점".to_owned(), hint: String::new(), measure: None };
            assert_eq!(app.paint_guide_band(ui, &p, canvas, Some(&spec)), 56.0);
        });
    });
    let eh_shapes: Vec<Shape> = empty_hint_out.shapes.into_iter().map(|c| c.shape).collect();
    assert_eq!(pill_strings(&eh_shapes), vec!["그리기 · 기준점"]);

    // Pan / no tool folds the pill and moves the rail back to the top (:239-241).
    app.active_tool = MapToolKind::Pan;
    let pan_flags = MapToolFlags { is_draw_tool: false, effective_tool: MapToolKind::Pan, ..draw_flags(MapToolKind::Pan, None) };
    let (canvas2, folded_shapes) = paint_hud(&mut app, pan_flags);
    assert!(find_pill_rect(&folded_shapes).is_none());
    assert_eq!(find_rect_of_width(&folded_shapes, 260.0).unwrap().min - canvas2.min, Vec2::new(12.0, 12.0));
}

// Ported from target/cpp-ref/src/app/KaDrawSketchTools.cpp:178-199,216-236,
// target/cpp-ref/src/app/MainWindowChrome.cpp:71-76, and target/cpp-ref/src/app/KaIconsOutlineA.cpp:113-122.
#[test]
fn guide_band_icon_and_point_title_follow_active_tool_and_edit_layer() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    let cases = [
        (MapToolKind::DrawLine, Some("feature_line"), "draw_line", "그리기 · 유구 선", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
        (MapToolKind::DrawLine, Some("section_line"), "draw_line", "그리기 · 단면선", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
        (MapToolKind::DrawPolygon, Some("feature_poly"), "draw_poly", "그리기 · 유구 면", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
        (MapToolKind::DrawPolygon, Some("survey_area"), "draw_poly", "그리기 · 조사구역", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
        (MapToolKind::PlacePoint, Some("control_points"), "gps", "그리기 · 기준점", "— 클릭한 자리에 점을 놓습니다"),
        (MapToolKind::PlacePoint, Some("artifact_point"), "artifact", "그리기 · 유물 위치", "— 클릭한 자리에 점을 놓습니다"),
        (MapToolKind::PlacePoint, None, "draw_poly", "그리기", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
        (MapToolKind::PlacePoint, Some("custom_point"), "draw_poly", "그리기", "— 클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"),
    ];
    for (tool, key, want_icon, want_title, want_hint) in cases {
        app.active_tool = tool;
        app.edit_layer_key = key.map(str::to_owned);
        let (canvas, shapes) = paint_hud(&mut app, draw_flags(tool, None));
        assert_eq!(pill_strings(&shapes), vec![want_title, want_hint], "{tool:?} / {key:?}");
        let glyph_box = Rect::from_min_size(Pos2::new(canvas.left() + 22.0, canvas.top() + 20.0), Vec2::splat(20.0));
        let want = expected_svg_signature(canvas, want_icon);
        assert!(!want.is_empty());
        assert_eq!(glyph_signature_in_hud(&shapes, glyph_box), want, "{tool:?} / {key:?} ({want_icon})");
        if want_icon == "gps" {
            assert_ne!(want, expected_svg_signature(canvas, "region"), "gps must use oGps, not region");
            assert_ne!(want, expected_svg_signature(canvas, "artifact"));
        }
    }

    let other_cases = [
        (MapToolFlags { is_draw_tool: false, is_select_tool: true, effective_tool: MapToolKind::Select, ..draw_flags(MapToolKind::Select, None) }, "select"),
        (MapToolFlags { is_draw_tool: false, is_measure: true, effective_tool: MapToolKind::Measure, ..draw_flags(MapToolKind::Measure, None) }, "measure"),
        (MapToolFlags { is_draw_tool: false, is_trench: true, effective_tool: MapToolKind::TrenchPlace, ..draw_flags(MapToolKind::TrenchPlace, None) }, "trench_grid"),
    ];
    for (flags, want_icon) in other_cases {
        app.active_tool = flags.effective_tool;
        let (canvas, shapes) = paint_hud(&mut app, flags);
        let glyph_box = Rect::from_min_size(Pos2::new(canvas.left() + 22.0, canvas.top() + 20.0), Vec2::splat(20.0));
        assert_eq!(glyph_signature_in_hud(&shapes, glyph_box), expected_svg_signature(canvas, want_icon), "{want_icon}");
    }
}
