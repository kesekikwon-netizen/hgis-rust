//! The keys of the main window, in the whole window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Event, Key, Modifiers, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};

/// MORE0 M4-39: the arrow keys moved every trench of the survey by 0.1 m (1 m with Shift) whatever tool was
/// on, with no undo. In the C++ they move a trench only in the trench move tool (KaTrenchMoveToolKeys.cpp:
/// 45-72), which this app does not have yet: on the map an arrow key moves no trench and changes nothing.
#[test]
fn an_arrow_key_on_the_map_moves_no_trench() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    let ring = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0], [0.0, 0.0]];
    app.home_map.real_survey.trenches.push(crate::real_survey::RealTrenchPoly { name: "Tr-1".to_owned(), ring: ring.clone(), area_m2: 50.0, center: [5.0, 2.5] });
    let ctx = egui::Context::default();
    for (key, modifiers) in [(Key::ArrowRight, Modifiers::default()), (Key::ArrowUp, Modifiers::SHIFT)] {
        let press = Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), events: vec![press], modifiers, ..Default::default() };
        let _ = ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }
    assert_eq!(app.home_map.real_survey.trenches[0].ring, ring);
    assert!(!app.is_dirty);
}
