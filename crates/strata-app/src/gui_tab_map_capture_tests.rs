//! 실제 입력 경로(화면 없는 egui) — 지도 칸을 왼쪽으로 세 번 누른 뒤 Enter·우클릭·더블클릭으로 마치면 유구 면이
//! 하나 들어가고(꼭짓점 셋 + 닫는 점), Esc 는 그리던 점만 지우고, 그리는 동안 왼쪽 끌기는 지도를 옮기지 않는다.
//! demo.gpkg 는 임시 사본, 위성·지적 타일은 끄고 돌린다(시험이 타일을 내려받지 않게).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};

struct Harness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("demo.gpkg");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
        let mut app = StrataAppState::new_startup();
        app.open_survey_gpkg(&path);
        app.active_tab = MainTab::Map;
        for e in app.left_layers.entries.clone() {
            if !matches!(e.group, strata_types::schema::LegendGroup::SurveyData) {
                app.home_map.real_survey.layer_visible.insert(e.label_ko, false);
            }
        }
        app.handle_ribbon_click("그리기");
        app.start_edit_feature_poly();
        let mut h = Self { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let _ = self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }

    /// 보통 빠르기의 누름(앞 누름에서 0.5 초 뒤 — 더블클릭이 아니다).
    fn press(&mut self, at: Pos2, which: PointerButton) {
        self.time += 0.5;
        self.quick_press(at, which);
    }

    /// 앞 누름 바로 뒤(0.15 초 안)의 누름.
    fn quick_press(&mut self, at: Pos2, which: PointerButton) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button: which, pressed: true, modifiers: Modifiers::default() }]);
        self.frame(vec![Event::PointerButton { pos: at, button: which, pressed: false, modifiers: Modifiers::default() }]);
    }

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::default() }]);
    }

    fn three_points(&mut self) {
        for at in [Pos2::new(700.0, 450.0), Pos2::new(820.0, 450.0), Pos2::new(820.0, 570.0)] {
            self.press(at, PointerButton::Primary);
        }
        assert_eq!(self.app.draw_sketch.points.len(), 3, "three presses on the map add three vertices");
    }

    /// 유구 면 수와 마지막 도형의 바깥 고리 점 수.
    fn last_ring_len(&self) -> (usize, usize) {
        let t = self.app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
        let parts = wkt_to_parts(&t.layer.features.last().unwrap().wkt_geom).expect("wkt");
        let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
        (t.layer.features.len(), rings[0].len())
    }
}

#[test]
fn enter_finishes_the_clicked_polygon() {
    let mut h = Harness::new();
    h.three_points();
    h.key(Key::Enter);
    assert_eq!(h.last_ring_len(), (3, 4));
    assert!(h.app.is_dirty);
}

#[test]
fn right_click_finishes_without_opening_the_map_menu() {
    let mut h = Harness::new();
    h.three_points();
    h.press(Pos2::new(760.0, 520.0), PointerButton::Secondary);
    assert_eq!(h.last_ring_len(), (3, 4));
    assert!(!h.app.map_context_menu_open, "right-click finishes the shape; no menu while drawing");
}

#[test]
fn double_click_finishes_without_a_duplicate_vertex() {
    let mut h = Harness::new();
    h.press(Pos2::new(700.0, 450.0), PointerButton::Primary);
    h.press(Pos2::new(820.0, 450.0), PointerButton::Primary);
    h.press(Pos2::new(820.0, 570.0), PointerButton::Primary);
    h.quick_press(Pos2::new(820.0, 570.0), PointerButton::Primary);
    assert_eq!(h.last_ring_len(), (3, 4));
}

/// 빠르게 두 자리를 찍어도(egui 는 더블클릭으로 본다) 같은 자리가 아니면 마치지 않는다 — Qt 처럼 4 px 안일 때만.
#[test]
fn quick_presses_at_different_spots_do_not_finish() {
    let mut h = Harness::new();
    h.three_points();
    h.quick_press(Pos2::new(700.0, 570.0), PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 4, "the fourth vertex is added, the shape stays open");
    assert_eq!(h.last_ring_len().0, 2, "nothing finished yet");
}

#[test]
fn escape_drops_only_the_sketch() {
    let mut h = Harness::new();
    h.press(Pos2::new(700.0, 450.0), PointerButton::Primary);
    h.press(Pos2::new(820.0, 450.0), PointerButton::Primary);
    h.key(Key::Escape);
    assert!(h.app.draw_sketch.points.is_empty());
    assert_eq!(h.app.active_tool, MapToolKind::DrawPolygon, "Esc keeps the drawing tool");
    assert!(h.app.draw_row_open(), "Esc keeps the drawing row");
    assert_eq!(h.app.status_left_message(), "그리던 도형을 지웠습니다.");
}

#[test]
fn left_drag_while_drawing_does_not_pan() {
    let mut h = Harness::new();
    let center = h.app.home_map.viewport.center_xy;
    let (a, b) = (Pos2::new(700.0, 450.0), Pos2::new(780.0, 520.0));
    h.frame(vec![Event::PointerMoved(a)]);
    h.frame(vec![Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }]);
    h.frame(vec![Event::PointerMoved(Pos2::new(740.0, 480.0))]);
    h.frame(vec![Event::PointerMoved(b)]);
    h.frame(vec![Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }]);
    assert_eq!(h.app.home_map.viewport.center_xy, center);
    assert_eq!(h.app.draw_sketch.points.len(), 1, "only the press adds a vertex");
}

/// Enter 로 마친 그 누름이 「이름·번호」 창의 「저장」까지 누르지 않는다 — 창은 열려 있고, 다음 Enter 가 저장한다.
#[test]
fn enter_that_finishes_does_not_also_save_the_form() {
    let mut h = Harness::new();
    h.three_points();
    h.key(Key::Enter);
    assert!(h.app.home_map.feature_form.is_some(), "the form stays open after the finishing Enter");
    h.frame(vec![]);
    h.key(Key::Enter);
    assert!(h.app.home_map.feature_form.is_none());
    let t = h.app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    assert_eq!(t.layer.features.last().unwrap().attributes.get("feature_no").map(String::as_str), Some("1호"));
}

/// 창의 Esc 는 「건너뛰기」 — 그린 도형은 남고 그리기 도구도 그대로다.
#[test]
fn escape_skips_the_form_and_keeps_the_tool() {
    let mut h = Harness::new();
    h.three_points();
    h.key(Key::Enter);
    h.frame(vec![]);
    h.key(Key::Escape);
    assert!(h.app.home_map.feature_form.is_none());
    assert_eq!(h.last_ring_len(), (3, 4));
    assert_eq!(h.app.active_tool, MapToolKind::DrawPolygon);
}
