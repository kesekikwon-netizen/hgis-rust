//! Clicks in the ribbon 「더보기」 menu. The catch-all at gui_ribbon_menus_b.rs sent every
//! unmatched row to AccountSettings (MainWindowRibbon.cpp:628-686).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::popup_input_harness::Harness;
use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{Event, Key, PointerButton, Pos2};

const DEFERRED_TIP: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 WU-07)";
const MAP_TIP: &str =
    "리본과 왼쪽 패널을 잠시 숨겨 지도를 넓게 봅니다. 다시 누르면 돌아옵니다 (Ctrl+F11)";
const PANEL_TIP: &str = "레이어·파일함 패널을 접거나 다시 폅니다 (F9)";
const WIDE_ON: &str = "지도 넓게 보기 — Ctrl+F11 키를 다시 누르면 리본과 왼쪽 패널이 돌아옵니다.";
const WIDE_OFF: &str = "지도 넓게 보기를 끝냈습니다.";
const PANEL_ON: &str = "왼쪽 패널을 접었습니다. F9 키를 누르면 다시 펼칩니다.";
const PANEL_OFF: &str = "왼쪽 패널을 펼쳤습니다.";

const DEFERRED: [&str; 5] = [
    "참조 벡터를 조사 파일 밖으로…",
    "조사 폴더 임시 자료 정리…",
    "토양도 SHP 불러오기",
    "CSV 기준점",
    "자료 준비 상태",
];

fn start() -> Harness {
    let directory = tempfile::tempdir().expect("temp");
    let mut app = StrataAppState::new_startup();
    app.active_tab = crate::app_state::MainTab::Map;
    let mut h = Harness::new(app, directory);
    h.ctx.all_styles_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

fn painted(h: &mut Harness) -> Vec<String> {
    let output = h.frame(vec![]);
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            eframe::egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect()
}

fn find_text(h: &mut Harness, title: &str) -> Option<Pos2> {
    let output = h.frame(vec![]);
    output.shapes.iter().find_map(|s| match &s.shape {
        eframe::egui::Shape::Text(t) if t.galley.text() == title => {
            let rect = t.galley.rect.translate(t.pos.to_vec2());
            (h.ctx.screen_rect().contains_rect(rect) && s.clip_rect.contains_rect(rect))
                .then_some(rect.center())
        }
        _ => None,
    })
}

fn press_text(h: &mut Harness, title: &str) {
    let at = find_text(h, title).unwrap_or_else(|| panic!("missing {title}"));
    h.press(at, PointerButton::Primary);
}

fn open_more(h: &mut Harness) {
    if find_text(h, "더보기").is_none() {
        press_text(h, "기타");
    }
    press_text(h, "더보기");
    assert!(h.app.ribbon.more_menu_open, "더보기 did not open its menu");
}

fn hover(h: &mut Harness, title: &str) {
    let at = find_text(h, title).unwrap_or_else(|| panic!("missing {title}"));
    for _ in 0..8 {
        h.frame(vec![Event::PointerMoved(at)]);
    }
}

fn key(h: &mut Harness, key: Key) {
    for pressed in [true, false] {
        h.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Default::default(),
        }]);
    }
}

#[test]
fn deferred_more_rows_stay_closed_and_do_not_open_account_settings() {
    let mut h = start();
    open_more(&mut h);
    for label in DEFERRED {
        hover(&mut h, label);
        let lines = painted(&mut h);
        assert!(
            lines.iter().any(|line| line.contains(DEFERRED_TIP)),
            "{label} tip missing: {lines:?}"
        );
        press_text(&mut h, label);
        assert_ne!(
            h.app.modals.active,
            Some(ActiveModalDialog::AccountSettings),
            "{label}"
        );
        assert!(
            h.app.modals.active.is_none(),
            "{label} opened {:?}",
            h.app.modals.active
        );
        assert!(!h.app.right_inspector.map_focused, "{label}");
        assert!(!h.app.right_inspector.left_panel_collapsed, "{label}");
        if !h.app.ribbon.more_menu_open {
            open_more(&mut h);
        }
    }
}

#[test]
fn api_key_row_is_the_only_more_row_that_opens_account_settings() {
    let mut h = start();
    open_more(&mut h);
    press_text(&mut h, "API 키 입력 · 계정 설정…");
    assert_eq!(
        h.app.modals.active,
        Some(ActiveModalDialog::AccountSettings)
    );
    assert!(!h.app.ribbon.more_menu_open);
}

#[test]
fn about_row_shows_the_gpl_notice_and_not_account_settings() {
    let mut h = start();
    open_more(&mut h);
    press_text(&mut h, "정보");
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::About));
    let lines = painted(&mut h);
    let body = lines.join("\n");
    for wanted in [
        "Strata · 필드고고학 GIS  v",
        "동국문화재연구원 · 만든이: 권영인 · 조유량 · 박종환",
        "작업 좌표계: EPSG:5186/5187 · 제출: EPSG:5179",
        "GDAL/OGR  © OSGeo 및 기여자 · MIT 계열",
        "GEOS  © GEOS contributors · LGPLv2.1",
        "본 소프트웨어는 GNU GPL v2 이상으로 배포됩니다.",
        "THIRD_PARTY_NOTICES.md",
    ] {
        assert!(body.contains(wanted), "about missing {wanted}: {body}");
    }
    assert!(!body.contains("qgis_core"), "this build does not link QGIS");
    assert!(
        !body.contains("LibreDWG"),
        "this build does not ship LibreDWG"
    );
    assert!(!h.app.right_inspector.left_panel_collapsed);
    key(&mut h, Key::F9);
    assert!(
        !h.app.right_inspector.left_panel_collapsed,
        "the about box let F9 through"
    );
    press_text(&mut h, "확인");
    assert!(
        h.app.modals.active.is_none(),
        "확인 left {:?}",
        h.app.modals.active
    );
}

#[test]
fn wide_map_row_toggles_the_same_focus_as_f11() {
    let mut h = start();
    open_more(&mut h);
    hover(&mut h, "지도 넓게 보기");
    let lines = painted(&mut h);
    assert!(lines.iter().any(|line| line.contains(MAP_TIP)), "{lines:?}");
    press_text(&mut h, "지도 넓게 보기");
    assert_ne!(
        h.app.modals.active,
        Some(ActiveModalDialog::AccountSettings)
    );
    assert!(h.app.modals.active.is_none());
    assert!(h.app.right_inspector.map_focused);
    assert_eq!(h.app.status_left_message(), WIDE_ON);
    assert!(!h.app.ribbon.more_menu_open);
    key(&mut h, Key::F11);
    assert!(!h.app.right_inspector.map_focused);
    assert_eq!(h.app.status_left_message(), WIDE_OFF);
}

#[test]
fn left_panel_row_toggles_the_same_panel_as_f9() {
    let mut h = start();
    open_more(&mut h);
    hover(&mut h, "왼쪽 패널 접기/펴기");
    let lines = painted(&mut h);
    assert!(
        lines.iter().any(|line| line.contains(PANEL_TIP)),
        "{lines:?}"
    );
    press_text(&mut h, "왼쪽 패널 접기/펴기");
    assert_ne!(
        h.app.modals.active,
        Some(ActiveModalDialog::AccountSettings)
    );
    assert!(h.app.right_inspector.left_panel_collapsed);
    assert_eq!(h.app.status_left_message(), PANEL_ON);
    open_more(&mut h);
    press_text(&mut h, "왼쪽 패널 접기/펴기");
    assert!(!h.app.right_inspector.left_panel_collapsed);
    assert_eq!(h.app.status_left_message(), PANEL_OFF);
}
