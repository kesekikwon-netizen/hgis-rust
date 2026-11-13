//! 「도형 속성」 창의 실제 입력 경로(화면 없는 egui) — Enter·Esc·비고 줄바꿈·주 창 단축키. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};
use crate::checklist_state::attr;

struct Harness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl Harness {
    /// demo 를 열고 유구 1의 「도형 속성」 창을 연 뒤 한 프레임(연 프레임의 키는 받지 않는다).
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("demo.gpkg");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
        let mut app = StrataAppState::new_startup();
        app.open_survey_gpkg(&path);
        app.active_tab = MainTab::Map;
        app.open_feature_record("feature_poly", 1);
        let mut h = Self { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
        h.frame(vec![], Modifiers::default());
        h
    }

    fn frame(&mut self, events: Vec<Event>, modifiers: Modifiers) {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
            time: Some(self.time),
            events,
            modifiers,
            ..Default::default()
        };
        let app = &mut self.app;
        let _ = self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        for pressed in [true, false] {
            self.frame(vec![Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }], modifiers);
        }
    }
}

/// 비고(여러 줄)에서 Enter 는 줄바꿈 — 창은 열려 있고 저장하지 않는다(QPlainTextEdit 이 Enter 를 먹는다).
#[test]
fn enter_in_the_note_adds_a_line() {
    let mut h = Harness::new();
    h.ctx.memory_mut(|m| m.request_focus(super::record_box_id("note")));
    h.frame(vec![], Modifiers::default());
    h.key(Key::Enter, Modifiers::default());
    let form = h.app.home_map.feature_record.as_ref().expect("still open");
    assert!(form.rows.iter().find(|r| r.field == "note").unwrap().text.contains('\n'));
    assert!(!h.app.is_dirty);
}

/// 다른 칸에서 Enter 는 「저장」(기본 단추) — 편집 버퍼에 쓰고 C++ 상태 글.
#[test]
fn enter_elsewhere_saves() {
    let mut h = Harness::new();
    h.app.home_map.feature_record.as_mut().unwrap().rows.iter_mut().find(|r| r.field == "feature_no").unwrap().text = "7호".to_owned();
    h.key(Key::Enter, Modifiers::default());
    assert!(h.app.home_map.feature_record.is_none());
    let t = h.app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    assert_eq!(attr(t.layer.features.iter().find(|f| f.fid == 1).unwrap(), "feature_no").as_deref(), Some("7호"));
    assert!(h.app.status_left_message().starts_with("속성을 바꿨습니다: 유구 (면) (#1)."));
}

/// Esc 는 「취소」 — 아무것도 바꾸지 않는다.
#[test]
fn escape_cancels() {
    let mut h = Harness::new();
    h.app.home_map.feature_record.as_mut().unwrap().rows[0].text = "수혈".to_owned();
    h.key(Key::Escape, Modifiers::default());
    assert!(h.app.home_map.feature_record.is_none());
    assert!(!h.app.is_dirty);
}

/// 창이 열린 동안 Ctrl+Z·Ctrl+S 는 듣지 않는다(C++ 모달).
#[test]
fn main_shortcuts_wait_while_the_window_is_open() {
    let mut h = Harness::new();
    let before = h.app.status_left_message().to_owned();
    for k in [Key::Z, Key::S] {
        h.key(k, Modifiers::CTRL);
    }
    assert!(h.app.home_map.feature_record.is_some());
    assert_eq!(h.app.status_left_message(), before);
}
