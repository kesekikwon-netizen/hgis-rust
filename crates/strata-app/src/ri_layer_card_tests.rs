//! Regression tests for the right-inspector `스타일` tab (`KaInspectorStyle`).
//! Ported from `target/cpp-ref/tests/test_inspector_panel.cpp:149-234` and
//! `target/cpp-ref/tests/test_inspector_style.cpp:67-184`, plus `KaInspectorStyle.cpp:113-225`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, epaint::Shape, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_types::crs::CrsCode;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::right_inspector::InspectorCardTab;
use crate::survey_paint_layers::build_paint_layers_styled;
use crate::survey_style::DomainStyle;

struct Harness {
    app: StrataAppState,
    ctx: egui::Context,
    _dir: tempfile::TempDir,
}

impl Harness {
    fn open_demo() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("demo.gpkg");
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
        std::fs::copy(src, &path).expect("copy demo.gpkg");
        let mut app = StrataAppState::new_startup();
        assert!(app.open_survey_gpkg(&path));
        app.right_inspector.active_card = InspectorCardTab::LayerStyleCard;
        Self { app, ctx: egui::Context::default(), _dir: dir }
    }

    fn run(&mut self, events: Vec<Event>) -> Vec<egui::epaint::ClippedShape> {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 900.0))), events, ..RawInput::default() };
        self.ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| self.app.render_cpp_right_inspector(ui));
        }).shapes
    }

    fn settle(&mut self) -> Vec<egui::epaint::ClippedShape> {
        let _ = self.run(vec![]);
        self.run(vec![])
    }

    fn click_at(&mut self, at: Pos2) {
        let btn = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        let _ = self.run(vec![Event::PointerMoved(at)]);
        let _ = self.run(vec![btn(true)]);
        let _ = self.run(vec![btn(false)]);
        let _ = self.run(vec![]);
    }

    fn click_text(&mut self, needle: &str) {
        let shapes = self.settle();
        let pos = find_text_rect(&shapes, |s| s == needle).unwrap_or_else(|| panic!("missing {needle:?} in {:?}", all_texts(&shapes))).center();
        self.click_at(pos);
    }

    fn click_swatch(&mut self, idx: usize) {
        let swatches = swatch_rects(&self.settle());
        assert_eq!(swatches.len(), 8, "expected 8 colour swatches");
        self.click_at(swatches[idx].0.center());
    }

    fn set_spin_mm(&mut self, text: &str) {
        let shapes = self.settle();
        let rect = find_text_rect(&shapes, |s| s.ends_with(" mm")).unwrap_or_else(|| panic!("missing mm spin in {:?}", all_texts(&shapes)));
        self.click_at(rect.center());
        let sel = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
        let _ = self.run(vec![Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: sel }]);
        let _ = self.run(vec![Event::Text(text.to_owned())]);
        let _ = self.run(vec![Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
        let _ = self.run(vec![]);
    }

    fn texts(&mut self) -> String {
        all_texts(&self.settle()).join("\n")
    }

    fn style_of(&self, table: &str) -> DomainStyle {
        let rs = &self.app.home_map.real_survey;
        let layers = build_paint_layers_styled(&self.app.survey_tables, CrsCode::Epsg5187, &rs.label_views, &rs.layer_styles);
        layers.iter().find(|l| l.table == table).expect("paint layer").style
    }
}

fn walk_shapes(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk_shapes(s, f));
    }
}

fn all_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    let mut out = Vec::new();
    for c in shapes {
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Text(t) = s {
                out.push(t.galley.text().to_owned());
            }
        });
    }
    out
}

fn find_text_rect(shapes: &[egui::epaint::ClippedShape], pred: impl Fn(&str) -> bool) -> Option<Rect> {
    let mut found = None;
    for c in shapes {
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Text(t) = s {
                if found.is_none() && pred(t.galley.text()) {
                    found = Some(Rect::from_min_size(t.pos, t.galley.size()));
                }
            }
        });
    }
    found
}

fn swatch_rects(shapes: &[egui::epaint::ClippedShape]) -> Vec<(Rect, f32)> {
    let mut out: Vec<(Rect, f32)> = Vec::new();
    for c in shapes {
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Rect(r) = s {
                if (r.rect.width() - 44.0).abs() < 1.0 && (r.rect.height() - 24.0).abs() < 1.0 {
                    if let Some(prev) = out.iter_mut().find(|(p, _)| (p.center() - r.rect.center()).length() < 2.0) {
                        prev.1 = prev.1.max(r.stroke.width);
                    } else {
                        out.push((r.rect, r.stroke.width));
                    }
                }
            }
        });
    }
    out
}

/// Ported from `target/cpp-ref/tests/test_inspector_panel.cpp:149-194` (`styleTab_editsTheChosenLayerInPlace`).
#[test]
fn style_tab_edits_the_chosen_layer_in_place() {
    let mut h = Harness::open_demo();
    let empty = h.texts();
    assert!(empty.contains("지도에서 도형을 고르거나 왼쪽 목록에서 레이어를 고르면 그 레이어의 색과 굵기를 여기서 바로 바꿉니다."), "{empty}");
    for legacy in ["저장 (Enter)", "되돌리기", "도형 삭제 (Del)", "면·선 색상…", "기준점 번호"] {
        assert!(!empty.contains(legacy), "legacy control {legacy:?} must not appear:\n{empty}");
    }

    h.app.right_inspector.selected_layer_key = Some("유구 (면)".to_owned());
    assert!(!h.texts().contains("지도에서 도형을 고르거나"));
    assert!(!h.app.is_dirty);

    h.click_swatch(4); // 파랑 (#2563eb), KaStylePalette.h:12-13
    let s = h.style_of("feature_poly");
    assert_eq!(s.stroke, [0x25, 0x63, 0xEB, 255]);
    assert_eq!(s.fill, [0x25, 0x63, 0xEB, 50]);
    assert!(h.app.is_dirty, "styleApplied marks the survey unsaved");

    h.app.is_dirty = false;
    h.set_spin_mm("2.4");
    assert!((h.style_of("feature_poly").stroke_width_mm - 2.4).abs() < 1e-4);
    assert!(h.app.is_dirty);

    // A colour set elsewhere is not undone by a later width change here (:173-176).
    let red_style = DomainStyle { fill: [0xDC, 0x26, 0x26, 255], stroke: [0xDC, 0x26, 0x26, 255], stroke_width_mm: 2.4, marker_size_mm: 3.5, casing: None };
    h.app.home_map.real_survey.layer_styles.insert("feature_poly".to_owned(), red_style);
    h.set_spin_mm("3.0");
    let s2 = h.style_of("feature_poly");
    assert_eq!(s2.stroke, [0xDC, 0x26, 0x26, 255]);
    assert!((s2.stroke_width_mm - 3.0).abs() < 1e-4);

    h.app.right_inspector.selected_layer_key = None;
    assert!(h.texts().contains("지도에서 도형을 고르거나"));

    // Reference / cadastral vector layer points to right-click 「면·외곽선 색」 (:188-190).
    let ref_entry = LayerTreeEntry { layer_key: "ref_topo".to_owned(), label_ko: "수치지형도".to_owned(), group: LegendGroup::ReferenceMap, visible: true, opacity: 1.0, read_only: true, snap_enabled: false, feature_count: 1 };
    h.app.left_layers.entries.push(ref_entry);
    h.app.right_inspector.selected_layer_key = Some("수치지형도".to_owned());
    assert!(h.texts().contains("선으로 된 지도의 선 색은 왼쪽 목록에서 그 레이어를 오른쪽 클릭해 「면·외곽선 색」으로 바꿉니다."));

    // Raster basemap falls back to the empty sentence (`MainWindowChrome.cpp:103`).
    h.app.right_inspector.selected_layer_key = Some("vworld_satellite".to_owned());
    assert!(h.texts().contains("지도에서 도형을 고르거나"));
}

/// Ported from `target/cpp-ref/tests/test_inspector_panel.cpp:217-234` (`styleTab_fillAndOutlineCanBeTurnedOff`).
#[test]
fn style_tab_fill_and_outline_can_be_turned_off() {
    let mut h = Harness::open_demo();
    h.app.right_inspector.selected_layer_key = Some("유구 (면)".to_owned());
    h.click_text("채우기 없음 (선만)");
    let s1 = h.style_of("feature_poly");
    assert_eq!(s1.fill[3], 0);
    assert!(s1.stroke[3] > 0);
    h.click_text("채우기 없음 (선만)");
    h.click_text("선 없음");
    let s2 = h.style_of("feature_poly");
    assert!(s2.fill[3] > 0);
    assert_eq!(s2.stroke[3], 0);
}

/// Ported from `target/cpp-ref/tests/test_inspector_style.cpp:67-96` (`plateTabHasASwatchAndTheWidthBeside`)
/// and `KaInspectorStyle.cpp:131-134, 158, 201` + `LayerOps.cpp:504`.
#[test]
fn plate_tab_swatch_width_row_and_geometry_labels() {
    let mut h = Harness::open_demo();
    h.app.right_inspector.selected_layer_key = Some("유구 (면)".to_owned());
    let shapes = h.settle();
    let mut has_head_swatch = false;
    shapes.iter().for_each(|c| walk_shapes(&c.shape, &mut |s| {
        if let Shape::Rect(r) = s {
            has_head_swatch |= (r.rect.width() - 20.0).abs() < 1.0 && (r.rect.height() - 14.0).abs() < 1.0;
        }
    }));
    assert!(has_head_swatch, "plate style tab must show 20x14 layer swatch");
    let lbl = find_text_rect(&shapes, |s| s == "외곽선 굵기").expect("외곽선 굵기");
    let spin = find_text_rect(&shapes, |s| s.ends_with(" mm")).expect("mm spin");
    assert!((lbl.center().y - spin.center().y).abs() < 6.0, "label and spin on one row");

    // Line layer hides fill/stroke checkboxes, shows 「선 굵기」, and preserves section_line white casing.
    h.app.right_inspector.selected_layer_key = Some("단면선".to_owned());
    let line_txt = h.texts();
    assert!(line_txt.contains("선 굵기") && !line_txt.contains("채우기 없음 (선만)") && !line_txt.contains("선 없음"), "{line_txt}");
    h.click_swatch(4);
    let sec = h.style_of("section_line");
    assert_eq!(sec.stroke, [0x25, 0x63, 0xEB, 255]);
    assert_eq!(sec.casing, Some(([255, 255, 255, 255], 3.0)));

    // Point layer shows both checkboxes and 「점 크기」, editing marker_size_mm.
    h.app.right_inspector.selected_layer_key = Some("유물".to_owned());
    let pt_txt = h.texts();
    assert!(pt_txt.contains("점 크기") && pt_txt.contains("채우기 없음 (선만)") && pt_txt.contains("선 없음"), "{pt_txt}");
    h.set_spin_mm("5.2");
    assert!((h.style_of("artifact_point").marker_size_mm - 5.2).abs() < 1e-4);
}

/// Ported from `target/cpp-ref/tests/test_inspector_style.cpp:121-174`.
#[test]
fn checkbox_and_point_colour_invariants_match_cpp() {
    let mut h = Harness::open_demo();
    h.app.right_inspector.selected_layer_key = Some("유구 (면)".to_owned());
    h.click_swatch(4);
    h.click_text("채우기 없음 (선만)");
    h.click_text("선 없음");
    let look = h.style_of("feature_poly");
    assert_eq!(look.stroke[3], 0);
    assert_eq!(look.fill, [0x25, 0x63, 0xEB, 50], "not a grey hairline");
    assert!(h.texts().contains("1.8 mm"));

    // polygonColourKeepsNoOutline (:136-145)
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    h.click_text("선 없음");
    h.click_swatch(4);
    let area = h.style_of("survey_area");
    assert_eq!((area.stroke[3], area.fill), (0, [0x25, 0x63, 0xEB, 50]));
    assert!(swatch_rects(&h.settle())[4].1 >= 2.5, "swatch 4 marked active via fill when no_stroke");

    // pointColourTurnsTheFillOn (:148-157)
    let empty_pt = DomainStyle { fill: [0, 0, 0, 0], stroke: [0, 0, 0, 255], stroke_width_mm: 0.2, marker_size_mm: 3.5, casing: None };
    h.app.home_map.real_survey.layer_styles.insert("artifact_point".to_owned(), empty_pt);
    h.app.right_inspector.selected_layer_key = Some("유물".to_owned());
    h.click_swatch(4);
    assert_eq!(h.style_of("artifact_point").fill, [0x25, 0x63, 0xEB, 255]);

    // pointComesBackSolidWithAVisibleOutline (:160-174)
    h.app.home_map.real_survey.layer_styles.remove("artifact_point");
    h.click_text("채우기 없음 (선만)");
    h.click_text("채우기 없음 (선만)");
    assert_eq!(h.style_of("artifact_point").fill[3], 255);
    h.click_text("선 없음");
    h.click_text("선 없음");
    let pt = h.style_of("artifact_point");
    assert_eq!(pt.stroke[3], 255);
    assert_ne!([pt.stroke[0], pt.stroke[1], pt.stroke[2]], [pt.fill[0], pt.fill[1], pt.fill[2]]);
}

/// Ported from `target/cpp-ref/tests/test_inspector_style.cpp:177-184` and `MainWindowChrome.cpp:100-111`.
#[test]
fn boxes_follow_external_change_and_most_recent_layer_choice() {
    let mut h = Harness::open_demo();
    h.app.set_card_target("feature_poly", 1, true);
    assert!(h.texts().contains("유구 (면)"));
    h.app.right_inspector.selected_layer_key = Some("시굴격자".to_owned());
    assert!(h.texts().contains("시굴격자"));
    let ext = DomainStyle { fill: [0, 0, 0, 0], stroke: [0xDC, 0x26, 0x26, 255], stroke_width_mm: 1.0, marker_size_mm: 3.5, casing: None };
    h.app.home_map.real_survey.layer_styles.insert("trial_trench".to_owned(), ext);
    let shapes = h.settle();
    assert!(all_texts(&shapes).iter().any(|t| t == "1.0 mm"));
    assert!(swatch_rects(&shapes)[0].1 >= 2.5, "red swatch follows external change");
    h.click_text("채우기 없음 (선만)");
    assert!(h.style_of("trial_trench").fill[3] > 0);
}
