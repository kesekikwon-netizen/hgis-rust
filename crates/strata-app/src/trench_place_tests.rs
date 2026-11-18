//! 「구역에 깔기」 writes `trial_trench` in EPSG:5187 metres and stays quiet when the plan is empty.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use geo::{Coord, Geometry, LineString, Polygon};
use strata_geo::trench_plan::build_for_target_ratio;
use strata_storage::edit_buffer::FeatureRecord;
use strata_storage::gpkg::GpkgRepository;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};

#[path = "trench_place_tests_check.rs"]
mod check;
use check::assert_placed;

const X0: f64 = 220_000.0;
const Y0: f64 = 460_000.0;

fn square_wkt(size: f64) -> String {
    let (x1, y1) = (X0 + size, Y0 + size);
    format!("POLYGON (({X0} {Y0}, {x1} {Y0}, {x1} {y1}, {X0} {y1}, {X0} {Y0}))")
}

fn square_geom(size: f64) -> Geometry<f64> {
    let (x1, y1) = (X0 + size, Y0 + size);
    Geometry::Polygon(Polygon::new(
        LineString::from(vec![
            Coord { x: X0, y: Y0 },
            Coord { x: x1, y: Y0 },
            Coord { x: x1, y: y1 },
            Coord { x: X0, y: y1 },
            Coord { x: X0, y: Y0 },
        ]),
        vec![],
    ))
}

fn opened() -> (StrataAppState, PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("survey.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).expect("new survey");
    let mut app = StrataAppState::new_startup();
    assert!(
        app.open_survey_gpkg(&path),
        "open {}",
        app.home_map.draw_guide_message.as_deref().unwrap_or("")
    );
    app.modals.trench_auto_place = true;
    app.modals.trench_survey_kind_idx = 0;
    app.modals.trench_azimuth_deg = 0.0;
    app.modals.trench_shape = strata_geo::trench_grid::TrenchShapeKind::Straight;
    app.modals.active = Some(ActiveModalDialog::TrenchGenerator);
    (app, path, dir)
}

fn add_square(app: &mut StrataAppState, size: f64) {
    let area = app
        .survey_tables
        .iter_mut()
        .find(|t| t.table == "survey_area")
        .expect("survey_area");
    area.layer.features.push(FeatureRecord {
        fid: 1,
        wkt_geom: square_wkt(size),
        attributes: BTreeMap::new(),
    });
}

fn trench_count(app: &StrataAppState) -> usize {
    app.survey_tables
        .iter()
        .find(|t| t.table == "trial_trench")
        .map(|t| t.layer.features.len())
        .unwrap_or(0)
}

fn click_place(app: &mut StrataAppState) {
    let ctx = egui::Context::default();
    let mut nodes = Vec::new();
    for step in 0..3 {
        nodes = frame(&ctx, f64::from(step) * 0.1, vec![], app);
    }
    let bounds = nodes
        .iter()
        .find(|(label, disabled, _)| label.contains("구역에 깔기") && !disabled)
        .and_then(|(_, _, b)| *b)
        .unwrap_or_else(|| panic!("enabled 구역에 깔기 not in {nodes:?}"));
    let at = bounds.center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    for (t, ev) in [
        (0.4, Event::PointerMoved(at)),
        (0.5, press(true)),
        (0.6, press(false)),
    ] {
        let _ = frame(&ctx, t, vec![ev], app);
    }
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(30) {
        let _ = frame(&ctx, 0.7, vec![], app);
        let pending = app.trench_place_job.is_some();
        let calculating = app
            .home_map
            .draw_guide_message
            .as_deref()
            .unwrap_or("")
            .contains("계산하는 중");
        if !pending && !calculating {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn frame(
    ctx: &egui::Context,
    time: f64,
    events: Vec<Event>,
    app: &mut StrataAppState,
) -> Vec<(String, bool, Option<Rect>)> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0))),
        time: Some(time),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| render_active_modal_dialogs(app, ctx));
    out.platform_output
        .accesskit_update
        .map_or_else(Vec::new, |u| {
            u.nodes
                .iter()
                .filter_map(|(_, n)| {
                    let bounds = n.bounds().map(|b| {
                        Rect::from_min_max(
                            Pos2::new(b.x0 as f32, b.y0 as f32),
                            Pos2::new(b.x1 as f32, b.y1 as f32),
                        )
                    });
                    n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
                })
                .collect()
        })
}

fn sql_count(path: &Path) -> i64 {
    rusqlite::Connection::open(path)
        .expect("db")
        .query_row("SELECT count(*) FROM trial_trench", [], |r| r.get(0))
        .expect("count")
}

/// A plan with no cell must not say the grid was placed, and must not write a row.
#[test]
fn click_with_empty_plan_does_not_report_success() {
    let (mut app, path, _dir) = opened();
    add_square(&mut app, 3.0);
    let plan = build_for_target_ratio(Some(&square_geom(3.0)), 10.0, 2.0, 0.0);
    assert!(
        plan.cells.is_empty(),
        "3 m square in EPSG:5187 cannot hold a trench"
    );
    click_place(&mut app);
    let msg = app.home_map.draw_guide_message.as_deref().unwrap_or("");
    assert_eq!(msg, plan.error, "empty plan message");
    assert!(!msg.contains("배치했습니다"), "{msg}");
    assert!(!msg.contains("끌어 옮기세요"), "{msg}");
    assert_eq!(trench_count(&app), 0);
    assert!(!app.is_dirty);
    assert_eq!(sql_count(&path), 0);
}

/// No survey polygon: the button does not invent a placed count.
#[test]
fn click_without_survey_area_does_not_report_success() {
    let (mut app, path, _dir) = opened();
    click_place(&mut app);
    let msg = app.home_map.draw_guide_message.as_deref().unwrap_or("");
    assert_eq!(msg, "시굴격자: 놓을 자리(원점)를 지도에서 누르세요.");
    assert!(!msg.contains("배치했습니다"));
    assert_eq!(trench_count(&app), 0);
    assert!(!app.is_dirty);
    assert_eq!(sql_count(&path), 0);
}

/// Placing writes one `trial_trench` row per computed cell, in EPSG:5187, and save/reopen keeps them.
#[test]
fn click_place_writes_trial_trench_and_reopens() {
    let (mut app, path, _dir) = opened();
    add_square(&mut app, 80.0);
    let plan = build_for_target_ratio(Some(&square_geom(80.0)), 10.0, 2.0, 0.0);
    assert!(
        !plan.cells.is_empty() && plan.error.is_empty(),
        "{}",
        plan.error
    );
    click_place(&mut app);
    assert_placed(&app, &plan.cells);
    let msg = app.home_map.draw_guide_message.as_deref().unwrap_or("");
    assert!(
        msg.contains(&format!("시굴격자 {}개", plan.cells.len())),
        "{msg}"
    );
    assert!(msg.contains("끌어 옮기세요"), "{msg}");
    assert!(!msg.contains("배치했습니다"), "{msg}");
    assert!(app.is_dirty);
    click_place(&mut app);
    assert_eq!(
        trench_count(&app),
        plan.cells.len(),
        "a second click must not stack another grid"
    );
    assert!(
        app.save_survey_project(),
        "save {}",
        app.home_map.draw_guide_message.as_deref().unwrap_or("")
    );
    assert_eq!(sql_count(&path), i64::try_from(plan.cells.len()).unwrap());
    let srs: i32 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = 'trial_trench'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let area_srs: i32 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = 'survey_area'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((srs, area_srs), (5187, 5187), "both layers stay EPSG:5187");
    let mut again = StrataAppState::new_startup();
    assert!(again.open_survey_gpkg(&path));
    assert_placed(&again, &plan.cells);
}
