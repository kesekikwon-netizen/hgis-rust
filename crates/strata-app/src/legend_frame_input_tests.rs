//! Owner rounded-frame decision; preserve target/cpp-ref/src/app/KaDrawingStudio.cpp:2364-2390,3370-3407.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use crate::tab_drawing::SheetItem;
use eframe::egui::{self, epaint::Primitive, Color32, Event, PointerButton, Pos2, Rect, Shape, Vec2};

#[path = "legend_frame_input_tests_a.rs"]
mod print;

fn start() -> Harness {
    let directory = tempfile::tempdir().expect("owned temporary survey");
    let file = directory.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = String::new;
    assert!(app.open_survey_gpkg(&file));
    app.active_tab = MainTab::Map;
    let mut h = Harness::new(app, directory);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    for label in ["도면", "용지 만들기"] {
        let at = h.point_at(label);
        h.press(at, PointerButton::Primary);
    }
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

fn place(h: &Harness) -> Rect {
    let (_, placed, moved) = h.app.drawing.item_rects.iter().find(|(item, ..)| *item == SheetItem::Legend).expect("actual legend item");
    placed.translate(*moved)
}

fn frame_shapes(output: &egui::FullOutput) -> (Rect, Vec<egui::epaint::ClippedShape>) {
    let bounds = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::Rect(r) if r.fill == Color32::from_rgba_unmultiplied(255, 255, 255, 245) => Some(r.rect),
            _ => None,
        })
        .expect("production legend fill");
    let shapes: Vec<_> = output
        .shapes
        .iter()
        .filter(|clipped| matches!(&clipped.shape, Shape::Rect(r) if r.rect == bounds && (r.fill.a() == 245 || (r.stroke.width-0.9).abs() < 0.001)))
        .cloned()
        .collect();
    assert_eq!(shapes.len(), 2, "one real fill and one real border");
    (bounds, shapes)
}

fn rounded_geometry(ctx: &egui::Context, output: &egui::FullOutput) {
    let (bounds, shapes) = frame_shapes(output);
    for shape in shapes {
        assert!(shape.clip_rect.contains_rect(bounds), "legend frame is not wholly visible in this fixture");
        let primitives = ctx.tessellate(vec![shape], output.pixels_per_point);
        let vertices: Vec<_> = primitives
            .iter()
            .flat_map(|p| match &p.primitive {
                Primitive::Mesh(m) => m.vertices.as_slice(),
                Primitive::Callback(_) => &[],
            })
            .filter(|v| v.color.a() > 200)
            .map(|v| v.pos)
            .collect();
        assert!(!vertices.is_empty(), "real frame has no painted geometry");
        // A radius-six quarter circle removes (2-sqrt(2))*6=3.51 logical points at the diagonal.
        // Two points leave >1.5 points for stroke/pixel rounding; this tests visible contour, not a stored radius.
        for (x, y) in [(bounds.left(), bounds.top()), (bounds.right(), bounds.top()), (bounds.left(), bounds.bottom()), (bounds.right(), bounds.bottom())] {
            let gap = vertices.iter().map(|p| (p.x - x).abs() + (p.y - y).abs()).fold(f32::INFINITY, f32::min);
            assert!(gap >= 2.0, "legend contour still occupies a square corner: gap={gap} at{x},{y}");
        }
    }
}

fn snapshot(h: &Harness) -> impl PartialEq {
    (
        super::vertex_crs_input_tests::records(h),
        format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables),
        std::fs::read(h.app.survey_path.as_ref().expect("owned survey")).expect("original bytes"),
    )
}

fn texts(output: &egui::FullOutput) -> Vec<String> {
    let (bounds, _) = frame_shapes(output);
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Text(t) if bounds.contains(t.pos + t.galley.size() * 0.5) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn drag(h: &mut Harness, from: Pos2, by: Vec2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    h.frame(vec![Event::PointerMoved(from + by * 0.5)]);
    h.frame(vec![Event::PointerMoved(from + by)]);
    h.frame(vec![Event::PointerButton {
        pos: from + by,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }]);
    for _ in 0..2 {
        h.frame(vec![]);
    }
}

#[test]
fn legend_frame_has_clear_corners_in_real_wide_and_compact_drawing_views() {
    let mut h = start();
    let before = snapshot(&h);
    for size in [[1680, 960], [800, 600]] {
        h.size = size;
        for _ in 0..4 {
            h.frame(vec![]);
        }
        h.save(&format!("legend-frame-{}", size[0]));
        let output = h.frame(vec![]);
        assert_eq!(frame_shapes(&output).0, place(&h), "painted frame differs from existing hit geometry");
        assert!(texts(&output).iter().any(|text| text == "범례"));
        rounded_geometry(&h.ctx, &output);
        assert!(snapshot(&h) == before, "rendering changed the original survey or edit history");
    }
}

#[test]
fn legend_frame_real_drag_and_resize_preserve_content_and_original_data() {
    let mut h = start();
    let before = snapshot(&h);
    let output = h.frame(vec![]);
    let content = texts(&output);
    let original = place(&h);
    drag(&mut h, original.center(), egui::vec2(-40.0, 20.0));
    let moved = place(&h);
    // One logical point allows the existing f32 screen/mm round trip, not a map-CRS error.
    assert!(moved.min.distance(original.min + egui::vec2(-40.0, 20.0)) < 1.0);
    assert_eq!(h.app.drawing.selected_item, Some(SheetItem::Legend));
    drag(&mut h, moved.right_bottom() + egui::vec2(3.0, 3.0), egui::vec2(30.0, 20.0));
    let bigger = place(&h);
    assert!(bigger.min.distance(moved.min) < 1.0 && bigger.max.distance(moved.max + egui::vec2(30.0, 20.0)) < 1.0);
    drag(&mut h, bigger.right_bottom() + egui::vec2(3.0, 3.0), egui::vec2(-15.0, -10.0));
    let smaller = place(&h);
    assert!(smaller.max.distance(bigger.max - egui::vec2(15.0, 10.0)) < 1.0);
    h.frame(vec![Event::PointerMoved(egui::pos2(390.0, 700.0))]);
    let output = h.frame(vec![]);
    assert_eq!(texts(&output), content, "frame edit changed legend content");
    assert!(snapshot(&h) == before, "legend interaction changed original records/history/bytes");
    h.save("legend-frame-after-move-resize");
    rounded_geometry(&h.ctx, &output);
}
