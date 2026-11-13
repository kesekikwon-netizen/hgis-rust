//! F1c-1 — the borehole window's rebuild button stays disabled until P1: it rebuilt GIS maps from the
//! synthetic paleo boreholes into the survey folder (cpp KaBoreholeDialog.cpp:170-181 rebuilds from
//! real loaded logs only). Headless egui frames; the button is found and clicked through the accesskit tree.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;
use crate::dialogs_borehole_spec::BOREHOLE_BTN_REBUILD;

type Node = (String, bool, Option<Rect>);

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) -> Vec<Node> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| app.render_borehole_dialog_frame(ctx));
    out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
        update
            .nodes
            .iter()
            .filter_map(|(_, n)| {
                let bounds = n.bounds().map(|b| {
                    Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
                });
                n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
            })
            .collect()
    })
}

/// A survey is open and the paleo state holds loaded boreholes — the conditions under which the
/// button used to be enabled.
fn open_window(survey_dir: &std::path::Path) -> (StrataAppState, egui::Context, Node) {
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey_dir.join("현장.gpkg"));
    assert!(app.paleo_3d.declustered_borehole_count > 0, "precondition: boreholes loaded");
    assert!(
        app.paleo_3d.last_paleo_output.as_ref().is_some_and(|o| o.parsed_boreholes.len() >= 2),
        "precondition: at least two parsed boreholes"
    );
    assert!(app.open_borehole_dialog());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes = frame(&mut app, &ctx, vec![]);
    }
    let rebuild = nodes.iter().find(|(l, ..)| l == BOREHOLE_BTN_REBUILD).cloned();
    (app, ctx, rebuild.unwrap_or_else(|| panic!("rebuild button missing: {nodes:?}")))
}

#[test]
fn rebuild_button_is_disabled_with_survey_and_loaded_boreholes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (_app, _ctx, rebuild) = open_window(dir.path());
    assert!(rebuild.1, "rebuild button must be disabled: {rebuild:?}");
}

#[test]
fn clicking_rebuild_writes_nothing_into_the_survey_folder() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, ctx, rebuild) = open_window(dir.path());
    let at = rebuild.2.expect("rebuild button bounds").center();
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);
    let listing: Vec<_> = std::fs::read_dir(dir.path()).expect("list").map(|e| e.expect("entry").file_name()).collect();
    assert!(listing.is_empty(), "survey folder must stay empty: {listing:?}");
}
