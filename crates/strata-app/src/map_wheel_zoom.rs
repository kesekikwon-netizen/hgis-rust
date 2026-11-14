//! Map canvas wheel and `+` / `−` button zoom rules.
//!
//! C++ references:
//! - `target/cpp-ref/src/core/LayerOps.h:42-45` (`kWheelZoomFactor = 1.2`)
//! - `target/cpp-ref/src/core/LayerOps.cpp:2255-2258` (`setWheelFactor` and `qgis/zoom_factor`)
//! - `target/cpp-ref/src/app/KaApplication.cpp:877` (`qgis/zoom_factor` set at startup)
//! - `target/cpp-ref/src/app/KaDrawingTools.cpp:105-121` (`Ctrl` fine-zoom ratio `0.35`, cursor-anchored zoom)
//! - `target/cpp-ref/src/app/KaMapControls.cpp:78-88` (`mapZoomIn` / `mapZoomOut` call `QgsMapCanvas::zoomIn` / `zoomOut`)
//!
//! Map engine user documentation:
//! - QGIS 3.40 User Guide, *QGIS GUI — Map View* (`https://docs.qgis.org/3.40/en/docs/user_manual/introduction/qgis_gui.html#map-view`)
//!   and *QGIS Configuration — Map Tools* (`https://docs.qgis.org/3.40/en/docs/user_manual/introduction/qgis_configuration.html`):
//!   rolling the wheel zooms centered on the mouse cursor; holding `Ctrl` zooms in finer steps (no numeric constant
//!   is specified in the user guide, so `CTRL_WHEEL_FINE_RATIO` follows `KaDrawingTools.cpp:113-114`).
//!
//! Toolkit wheel event on Windows (`Cargo.lock`: `winit 0.30.13`, `egui-winit 0.31.1`, `egui 0.31.1`):
//! - `winit 0.30.13` (`src/platform_impl/windows/event_loop.rs:1727-1742`, `src/event.rs:953-959`) divides `WM_MOUSEWHEEL`
//!   by `WHEEL_DELTA` (`120`) into `MouseScrollDelta::LineDelta(0.0, 1.0)` per notch.
//! - `egui-winit 0.31.1` (`src/lib.rs:694-711`) pushes `egui::Event::MouseWheel { unit: MouseWheelUnit::Line, delta: vec2(0.0, 1.0), modifiers }`.
//! - `egui 0.31.1` (`src/data/input.rs:503-519, 1161-1170`, `src/input_state/mod.rs:321-399`, `src/memory/mod.rs:285-286, 315`)
//!   converts `Line` by `options.line_scroll_speed` (`40.0` pt), turns `Shift` + wheel into horizontal `raw_scroll_delta`
//!   (`src/input_state/mod.rs:327-331`), and smooths `smooth_scroll_delta` across multiple frames while routing `Ctrl` + wheel
//!   out of `smooth_scroll_delta`. C++ Strata defines no `Shift` + wheel rule on `m_canvas` (`MainWindow.cpp:510, 2258-2265`,
//!   `LayerOps.cpp:2255-2258`, `KaDrawingTools.cpp:102-122`), so reading `Event::MouseWheel` directly applies `1.2` once per notch
//!   with or without `Shift`.

use eframe::egui::{self, Event, MouseWheelUnit, Pos2, Rect};
use strata_canvas2d::viewport::ViewportTransform;

/// Scale factor per wheel notch and per `+` / `−` click (`target/cpp-ref/src/core/LayerOps.h:44`).
pub(crate) const WHEEL_ZOOM_FACTOR: f64 = 1.2;

/// Finer `Ctrl` + wheel ratio above `1.0` (`target/cpp-ref/src/app/KaDrawingTools.cpp:113-114`).
pub(crate) const CTRL_WHEEL_FINE_RATIO: f64 = 0.35;

/// `meters_per_pixel` multiplier for one `+` button click (`target/cpp-ref/src/app/KaMapControls.cpp:87`).
pub(crate) const ZOOM_IN_MPP_FACTOR: f64 = 1.0 / WHEEL_ZOOM_FACTOR;

/// `meters_per_pixel` multiplier for one `−` button click (`target/cpp-ref/src/app/KaMapControls.cpp:88`).
pub(crate) const ZOOM_OUT_MPP_FACTOR: f64 = WHEEL_ZOOM_FACTOR;

/// Pure `(notches, ctrl) -> meters_per_pixel` multiplier (`notches > 0` zooms in, `< 0` zooms out).
/// Uses `base.powf(-notches)` so fractional notches compose multiplicatively across frames and multiple notches multiply.
#[must_use]
pub(crate) fn wheel_zoom_mpp_factor(notches: f64, ctrl: bool) -> f64 {
    if !notches.is_finite() || notches == 0.0 {
        return 1.0;
    }
    let base = if ctrl {
        1.0 + (WHEEL_ZOOM_FACTOR - 1.0) * CTRL_WHEEL_FINE_RATIO
    } else {
        WHEEL_ZOOM_FACTOR
    };
    base.powf(-notches)
}

/// Applies all `Event::MouseWheel` events of this frame around the rectangle-local cursor position in `rect`.
///
/// Modal dialogs in `strata-app` use centered `egui::Window` layers (`Order::Middle`, `egui-0.31.1/src/containers/area.rs:212`,
/// `src/layers.rs:10-27`) without a full-screen backdrop, leaving the `CentralPanel` (`Order::Background`,
/// `src/containers/panel.rs:384-390`) hovered beside the dialog. Ignore background wheel zoom whenever any modal,
/// popup, or `Middle`/`Foreground` area is open (`src/memory/mod.rs:972-974, 1077-1079, 1229-1235, 1306-1318`).
pub(crate) fn apply_map_wheel_zoom(
    ui: &egui::Ui,
    rect: Rect,
    hover_pos: Option<Pos2>,
    vp: &mut ViewportTransform,
) {
    if ui.layer_id().order == egui::Order::Background
        && ui.ctx().memory(|m| {
            m.top_modal_layer().is_some()
                || m.any_popup_open()
                || m.areas().visible_layer_ids().iter().any(|l| {
                    matches!(l.order, egui::Order::Middle | egui::Order::Foreground)
                })
        })
    {
        return;
    }
    let line_speed = f64::from(ui.ctx().options(|o| o.line_scroll_speed));
    let (factor, any_wheel, ptr_pos) = ui.input(|i| {
        let mut combined = 1.0_f64;
        let mut any = false;
        let page_h = f64::from(i.screen_rect.height());
        for ev in &i.events {
            if let Event::MouseWheel { unit, delta, modifiers } = ev {
                let notches = match unit {
                    MouseWheelUnit::Line => f64::from(delta.y),
                    MouseWheelUnit::Point if line_speed > 0.0 => f64::from(delta.y) / line_speed,
                    MouseWheelUnit::Page if line_speed > 0.0 => f64::from(delta.y) * page_h / line_speed,
                    _ => 0.0,
                };
                if notches.is_finite() && notches != 0.0 {
                    let ctrl = modifiers.ctrl || modifiers.command;
                    combined *= wheel_zoom_mpp_factor(notches, ctrl);
                    any = true;
                }
            }
        }
        (combined, any, i.pointer.hover_pos())
    });
    if !any_wheel {
        return;
    }
    // `ViewportTransform::zoom_at` (`crates/strata-canvas2d/src/viewport.rs:48-54`) takes map-rectangle-local pixels.
    let anchor = match hover_pos.or(ptr_pos) {
        Some(hp) => [f64::from(hp.x - rect.left()), f64::from(hp.y - rect.top())],
        None => [f64::from(rect.width()) * 0.5, f64::from(rect.height()) * 0.5],
    };
    vp.zoom_at(factor, anchor);
}

#[cfg(test)]
#[path = "map_wheel_zoom_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "map_wheel_zoom_tests_b.rs"]
mod tests_b;
