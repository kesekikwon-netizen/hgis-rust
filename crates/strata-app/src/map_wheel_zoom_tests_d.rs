//! Floating window occlusion, orphan-ComboBox unsticking, and `d4` popup closing tests (`MAP1e` `b1`, `b1(b)`, `b1(c)`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, Event, Key, Modifiers, Order, PointerButton, Pos2, Rect};

use crate::dialogs_modal::ActiveModalDialog;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::ribbon::SubToolsMode;

use super::tests::{new_map_app, run_frame, windows_wheel_notch};
use super::tests_c::{click_at, find_text_center, OVER_UNCOVERED_MAP};

fn walk_shapes(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk_shapes(s, f));
    }
}

fn collect_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
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

fn find_text_pred(shapes: &[egui::epaint::ClippedShape], pred: impl Fn(&str, Pos2) -> bool) -> Option<Pos2> {
    for c in shapes {
        let mut hit = None;
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Text(t) = s {
                let rect = Rect::from_min_size(t.pos, t.galley.size());
                if pred(t.galley.text(), rect.center()) {
                    hit = Some(rect.center());
                }
            }
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

#[test]
fn wheel_notch_over_floating_non_modal_window_never_zooms_map_behind_it() {
    for &(dialog, over_window) in &[
        (ActiveModalDialog::HeritageBrowser, Pos2::new(840.0, 480.0)),
        (ActiveModalDialog::DemClass, Pos2::new(840.0, 480.0)),
        (ActiveModalDialog::TrenchGenerator, Pos2::new(900.0, 500.0)),
    ] {
        let (mut app, ctx) = new_map_app();
        app.modals.active = Some(dialog);
        app.home_map.viewport.meters_per_pixel = 12.0;
        let center0 = app.home_map.viewport.center_xy;

        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_window)]);
        assert_eq!(
            ctx.layer_id_at(over_window).map(|l| l.order),
            Some(Order::Middle),
            "{dialog:?}: pointer {over_window:?} must hit floating window layer"
        );
        let _ = run_frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(over_window), windows_wheel_notch(1.0, Modifiers::NONE)],
        );
        assert_eq!(
            app.home_map.viewport.meters_per_pixel, 12.0,
            "{dialog:?}: wheel over floating window must not zoom map behind it"
        );
        assert_eq!(app.home_map.viewport.center_xy, center0);
    }
}

/// `b1 (c)`: opening a ComboBox inside a window and closing the window directly leaves
/// `ctx.memory(|m| m.any_popup_open())` true; wheel zoom on the map must still work.
#[test]
fn closing_window_with_open_combo_box_does_not_leave_wheel_ignored() {
    // 1) Non-modal DemClass + ka_dem_preset_combo, window closed directly without closing combo box.
    let (mut app, ctx) = new_map_app();
    app.modals.active = Some(ActiveModalDialog::DemClass);
    let _ = run_frame(&mut app, &ctx, vec![]);
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let combo_pos = find_text_pred(&shapes, |t, _| t.contains("전국") || t.contains("2000m"))
        .unwrap_or_else(|| panic!("DEM preset combo missing; texts={:?}", collect_texts(&shapes)));
    let _ = click_at(&mut app, &ctx, combo_pos, PointerButton::Primary);
    assert!(ctx.memory(|m| m.any_popup_open()), "DEM preset combo must be open");

    app.modals.active = None;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(OVER_UNCOVERED_MAP)]);
    assert!(
        ctx.memory(|m| m.any_popup_open()),
        "egui popup_id stays in memory after parent window closes without closing the combo box"
    );

    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(OVER_UNCOVERED_MAP)]);
    let _ = run_frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(OVER_UNCOVERED_MAP), windows_wheel_notch(1.0, Modifiers::NONE)],
    );
    assert!(
        (12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9,
        "map wheel zoom must work after closing window with open combo (got {})",
        app.home_map.viewport.meters_per_pixel
    );

    // 2) Modal PaleoSourceDownload + ka_paleo_radius_combo, closed directly.
    let (mut app, ctx) = new_map_app();
    app.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
    let _ = run_frame(&mut app, &ctx, vec![]);
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let combo_pos = find_text_pred(&shapes, |t, _| t.contains("5,000 m") || t.contains("5 km"))
        .unwrap_or_else(|| panic!("paleo radius combo missing; texts={:?}", collect_texts(&shapes)));
    let _ = click_at(&mut app, &ctx, combo_pos, PointerButton::Primary);
    assert!(ctx.memory(|m| m.any_popup_open()), "paleo radius combo must be open");
    app.modals.active = None;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(OVER_UNCOVERED_MAP)]);
    assert!(ctx.memory(|m| m.any_popup_open()), "egui popup_id still in memory after modal closes");

    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(OVER_UNCOVERED_MAP), windows_wheel_notch(1.0, Modifiers::NONE)],
    );
    assert!(
        (12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9,
        "map wheel zoom must work after modal closes with open combo"
    );
}

fn press_escape(app: &mut crate::app_state::StrataAppState, ctx: &egui::Context) {
    let _ = run_frame(
        app,
        ctx,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
}

#[test]
fn d4_popups_close_on_outside_click_and_escape() {
    // 1) ka_snap_assist_menu: click inside keeps open; outside click closes & unblocks wheel; Escape closes.
    let (mut app, ctx) = new_map_app();
    app.ribbon.sub_toolbar_visible = true;
    app.ribbon.sub_mode = SubToolsMode::Draw;
    app.ribbon.snap_assist_menu_open = true;
    let _ = run_frame(&mut app, &ctx, vec![]);
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let trace_pos = find_text_pred(&shapes, |t, _| t.contains("자동채움"))
        .unwrap_or_else(|| panic!("snap trace checkbox missing; texts={:?}", collect_texts(&shapes)));
    let _ = click_at(&mut app, &ctx, trace_pos, PointerButton::Primary);
    assert!(app.ribbon.snap_assist_menu_open, "clicking checkbox inside snap menu must keep it open");

    let _ = click_at(&mut app, &ctx, OVER_UNCOVERED_MAP, PointerButton::Primary);
    assert!(!app.ribbon.snap_assist_menu_open, "clicking outside ka_snap_assist_menu must close it");
    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(OVER_UNCOVERED_MAP), windows_wheel_notch(1.0, Modifiers::NONE)],
    );
    assert!((12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9);

    app.ribbon.snap_assist_menu_open = true;
    let _ = run_frame(&mut app, &ctx, vec![]);
    press_escape(&mut app, &ctx);
    assert!(
        !app.draw_row_open() || !app.ribbon.snap_assist_menu_open,
        "Escape must dismiss ka_snap_assist_menu"
    );
    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(OVER_UNCOVERED_MAP), windows_wheel_notch(1.0, Modifiers::NONE)],
    );
    assert!((12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9);

    // 2) ka_theme_menu_popup: button toggles; click inside keeps open; outside click closes; Escape closes.
    let (mut app, ctx) = new_map_app();
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let theme_btn = find_text_center(&shapes, "테마 ▼").expect("theme trigger");
    let _ = click_at(&mut app, &ctx, theme_btn, PointerButton::Primary);
    assert!(app.theme_menu_open, "clicking theme button must open popup");
    let _ = run_frame(&mut app, &ctx, vec![]);
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let inside = find_text_pred(&shapes, |t, _| t.contains("고대비"))
        .unwrap_or_else(|| panic!("theme item missing; texts={:?}", collect_texts(&shapes)));
    let _ = click_at(&mut app, &ctx, inside, PointerButton::Primary);
    assert!(app.theme_menu_open, "clicking option inside ka_theme_menu_popup must keep it open");
    assert!(app.display_options.high_contrast, "theme option click must take effect");
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let theme_btn = find_text_center(&shapes, "테마 ▼").expect("theme trigger after option click");
    let _ = click_at(&mut app, &ctx, theme_btn, PointerButton::Primary);
    assert!(!app.theme_menu_open, "clicking theme button again must toggle popup closed (btn={theme_btn:?})");

    let _ = click_at(&mut app, &ctx, theme_btn, PointerButton::Primary);
    assert!(app.theme_menu_open);
    let _ = click_at(&mut app, &ctx, OVER_UNCOVERED_MAP, PointerButton::Primary);
    assert!(!app.theme_menu_open, "clicking outside ka_theme_menu_popup must close it");

    let _ = click_at(&mut app, &ctx, theme_btn, PointerButton::Primary);
    assert!(app.theme_menu_open);
    press_escape(&mut app, &ctx);
    assert!(!app.theme_menu_open, "Escape must close ka_theme_menu_popup");

    // 3) ka_status_scale_popup: button toggles; click inside keeps open; outside click closes; Escape closes.
    let (mut app, ctx) = new_map_app();
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let scale_btn = find_text_pred(&shapes, |t, p| t.starts_with("1:") && p.y > 920.0).expect("status scale btn");
    let _ = click_at(&mut app, &ctx, scale_btn, PointerButton::Primary);
    assert!(app.scale_menu_open, "clicking scale button must open popup");
    let _ = run_frame(&mut app, &ctx, vec![]);
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let p100 = find_text_pred(&shapes, |t, _| t == "1:100")
        .unwrap_or_else(|| panic!("scale preset 1:100 missing; texts={:?}", collect_texts(&shapes)));
    let edit_pos = Pos2::new(p100.x, p100.y - 24.0);
    let _ = click_at(&mut app, &ctx, edit_pos, PointerButton::Primary);
    assert!(app.scale_menu_open, "clicking TextEdit inside scale popup must keep it open");
    let _ = click_at(&mut app, &ctx, OVER_UNCOVERED_MAP, PointerButton::Primary);
    assert!(!app.scale_menu_open, "clicking outside ka_status_scale_popup must close it");

    let _ = click_at(&mut app, &ctx, scale_btn, PointerButton::Primary);
    assert!(app.scale_menu_open);
    press_escape(&mut app, &ctx);
    assert!(!app.scale_menu_open, "Escape must close ka_status_scale_popup");

    // 4) ka_download_gauge_popup: outside secondary click and Escape also close it.
    let (mut app, ctx) = new_map_app();
    let (_id, _guard) = app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Stopped { code: StopCode::None1, reason: "안내".to_owned() },
        1.0,
    );
    app.download_jobs.popup_open = true;
    let _ = run_frame(&mut app, &ctx, vec![]);
    let _ = click_at(&mut app, &ctx, OVER_UNCOVERED_MAP, PointerButton::Secondary);
    assert!(!app.download_jobs.popup_open, "secondary click outside must close gauge popup");

    app.download_jobs.popup_open = true;
    let _ = run_frame(&mut app, &ctx, vec![]);
    press_escape(&mut app, &ctx);
    assert!(!app.download_jobs.popup_open, "Escape must close gauge popup");
}
