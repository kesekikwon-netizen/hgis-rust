//! Actual temporary raster workflow, target/cpp-ref/tests/test_align_tool.cpp:80-127.
use crate::app_state::StrataAppState;
use crate::gui_ribbon_bar::AlignPair;
use std::path::Path;

fn app_for(source: &Path) -> (StrataAppState, String) {
    image::save_buffer(source, &vec![240; 100 * 50 * 3], 100, 50, image::ColorType::Rgb8).expect("owned raster fixture");
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(source.with_extension("gpkg"));
    app.add_raster_from_path(source).expect("actual raster load");
    let key = app.pictures.layers[0].key.clone();
    app.start_align_session(&key);
    app.align.pairs = vec![
        AlignPair { src: [0.0, 0.0], map: [200000.0, 450100.0] },
        AlignPair { src: [100.0, 0.0], map: [200200.0, 450100.0] },
        AlignPair { src: [0.0, 50.0], map: [200000.0, 450000.0] },
    ];
    app.is_dirty = false;
    (app, key)
}

fn settle(app: &mut StrataAppState, done: impl Fn(&StrataAppState) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let ctx = eframe::egui::Context::default();
    while !done(app) && std::time::Instant::now() < deadline {
        app.poll_align_task(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(done(app), "actual alignment did not finish: {}", app.status_left_message());
}

#[test]
fn apply_places_the_real_picture_and_restore_returns_its_original_file() {
    let temp = tempfile::tempdir().expect("temporary survey folder");
    let source = temp.path().join("scan.png");
    let (mut app, key) = app_for(&source);
    app.apply_align_move().expect("valid picture pairs");
    settle(&mut app, |app| app.align.applied);
    let layer = app.pictures.layers.iter().find(|l| l.key == key).expect("same layer key");
    assert_ne!(layer.file, source, "apply reported success without persisting a real picture fit");
    assert!(!layer.align_pending);
    let info = strata_engine::picture_info(&layer.file).expect("persisted real image");
    assert_eq!(info.transform, Some([200000.0, 2.0, 0.0, 450100.0, 0.0, -2.0]));
    assert_eq!(info.epsg, Some(5187));
    assert!(app.left_layers.entries.iter().find(|e| e.layer_key == key).expect("real legend row").visible);
    app.restore_align_originals();
    settle(&mut app, |app| app.pictures.layers.iter().any(|l| l.key == key && l.file == source));
    assert!(!app.align.applied);
    assert!(app.pictures.layers.iter().find(|l| l.key == key).expect("restored row").align_pending);
    assert!(!source.with_extension("pgw").exists(), "user source sidecars must remain untouched");
}

#[test]
fn completion_is_not_published_before_the_file_worker_finishes() {
    let temp = tempfile::tempdir().expect("temporary folder");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.apply_align_move().expect("queued valid work");
    assert!(!app.align.applied, "UI claimed completion before reading the file worker result");
    app.stop_align_session();
    app.poll_align_task(&eframe::egui::Context::default());
    assert!(app.align.target_layer.is_none() && !app.align.applied, "closed session accepted a late result");
}

#[test]
fn closing_a_pending_restore_cannot_delete_the_picture_still_shown_on_the_map() {
    let temp = tempfile::tempdir().expect("temporary folder");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.apply_align_move().expect("queued fit");
    settle(&mut app, |a| a.align.applied);
    let shown = app.pictures.layers[0].file.clone();
    app.restore_align_originals();
    let task = app.align.raster.as_mut().expect("real session").task.0.take().expect("queued restore");
    app.stop_align_session();
    task.receive.recv_timeout(std::time::Duration::from_secs(5)).expect("actual file worker completed").expect("restore preparation");
    assert!(shown.is_file(), "a closed restore deleted a file that the map still references");
}

#[test]
fn point_edits_during_restore_cannot_leave_the_layer_on_a_deleted_output() {
    let temp = tempfile::tempdir().expect("temporary folder");
    let source = temp.path().join("scan.png");
    let (mut app, _) = app_for(&source);
    app.apply_align_move().expect("queued fit");
    settle(&mut app, |a| a.align.applied);
    app.restore_align_originals();
    app.align.set_source([5.0, 5.0]);
    app.align.accept_map([200010.0, 450090.0]);
    settle(&mut app, |a| a.align.raster.as_ref().is_some_and(|s| s.task.0.is_none()));
    assert_eq!(app.pictures.layers[0].file, source, "restore was incorrectly rejected because the point set changed");
}

#[test]
fn actual_alignment_split_keeps_source_and_map_visible_on_small_monitors() {
    for [width, height] in [[640, 480], [1280, 720]] {
        let temp = tempfile::tempdir().expect("temporary pixels");
        let (mut app, _) = app_for(&temp.path().join("scan.png"));
        let rendered = crate::test_mods::ui_snapshot_support::render([width, height], |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| app.render_map_tab_ui(ctx, ui));
        });
        assert!(rendered.texts().iter().any(|s| s.contains("왼쪽 · 그림")), "actual alignment still has no source pane");
        for (text, rect) in rendered.text_rects() {
            assert!(rect.left() >= -0.5 && rect.right() <= width as f32 + 0.5, "{width}: {text} is outside the workspace: {rect:?}");
        }
        rendered.save_if_requested(&format!("alignment-split-{width}"));
    }
}

#[test]
fn alignment_escape_cancels_only_the_pending_source_and_keeps_the_session() {
    let temp = tempfile::tempdir().expect("temporary pixels");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.align.set_source([10.0, 10.0]);
    let ctx = eframe::egui::Context::default();
    let _ = ctx.run(
        eframe::egui::RawInput {
            events: vec![eframe::egui::Event::Key { key: eframe::egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }],
            ..Default::default()
        },
        |ctx| app.handle_cpp_keyboard_shortcuts(ctx).apply(&mut app, ctx),
    );
    assert!(app.align.pending_source.is_none(), "Escape failed to cancel the pending left point");
    assert_eq!(app.active_tool, strata_canvas2d::map_tools::MapToolKind::GeorefAlign, "Escape incorrectly ended alignment");
}

#[test]
fn alignment_undo_removes_the_last_pair_without_touching_other_edit_buffers() {
    let temp = tempfile::tempdir().expect("temporary pixels");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.undo_map_action();
    assert_eq!(app.align.pairs.len(), 2, "Ctrl+Z route did not reach alignment pairs");
}

#[test]
fn actual_alignment_pointer_clicks_pair_original_pixels_with_work_crs_map_points() {
    use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
    let temp = tempfile::tempdir().expect("temporary pixels");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.align.pairs.clear();
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &Default::default());
    let frame = |app: &mut StrataAppState, events: Vec<Event>| {
        ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 700.0))), events, ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_map_tab_ui(ctx, ui));
        })
    };
    for _ in 0..3 {
        frame(&mut app, vec![]);
    }
    let session = app.align.raster.as_ref().expect("actual raster session");
    let source_rect = session.source_rect.expect("visible source pane");
    let map_rect = session.map_rect.expect("visible paired map pane");
    let source_at = session.view.screen(source_rect, [10.0, 10.0]);
    let click = |app: &mut StrataAppState, at: Pos2| {
        frame(app, vec![Event::PointerMoved(at), Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }]);
        frame(app, vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }])
    };
    app.home_map.viewport.center_xy = [200000.0, 450000.0];
    app.home_map.viewport.meters_per_pixel = 1.0;
    let output = click(&mut app, source_at);
    assert!(
        output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "1" && text.pos.x < source_rect.right())),
        "source click marker is missing from its release frame"
    );
    let pending = app.align.pending_source.expect("source click must precede map click");
    // Screen values below1200 have f32 ULP<=0.000122px; the source fit is>1px/pixel.
    assert!((pending[0] - 10.0).abs() < 0.0002 && (pending[1] - 10.0).abs() < 0.0002);
    click(&mut app, map_rect.center() + egui::vec2(30.0, -20.0));
    assert_eq!(app.align.pairs.len(), 1, "actual right pane did not consume the pending pixel");
    assert_eq!(app.align.pairs[0].map, [200030.0, 450020.0], "right map must use current work EPSG5187 at1m/pixel");
}

#[test]
fn saving_an_inflight_fit_waits_for_the_actual_output_before_closing() {
    let temp = tempfile::tempdir().expect("temporary picture");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.apply_align_move().expect("queued apply");
    app.save_align_result();
    settle(&mut app, |a| a.align.raster.as_ref().is_none_or(|s| s.task.0.is_none()));
    assert!(app.align.target_layer.is_none(), "save did not wait for and close the actual job");
    assert!(app.pictures.layers[0].file.with_extension("pgw").is_file());
    assert!(app.status_left_message().starts_with("맞춤 저장: "));
}

#[test]
fn source_corner_pair_numbers_remain_inside_the_actual_painter_clip() {
    let temp = tempfile::tempdir().expect("temporary pixels");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    let rendered = crate::test_mods::ui_snapshot_support::render([640, 480], |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| app.render_map_tab_ui(ctx, ui));
    });
    let source = app.align.raster.as_ref().and_then(|s| s.source_rect).expect("real source pane");
    for (text, bounds, clip) in rendered.text_clips() {
        if ["1", "2", "3"].contains(&text.as_str()) && bounds.center().x <= source.right() {
            assert!(clip.contains_rect(bounds), "source corner label {text} is clipped: {bounds:?} by {clip:?}");
        }
    }
}

#[test]
fn a_referenced_picture_waits_outside_the_right_map_during_pair_selection() {
    let temporary = tempfile::tempdir().expect("temporary picture");
    let source = temporary.path().join("scan.png");
    std::fs::write(source.with_extension("pgw"), "2\n0\n0\n-2\n200001\n450099\n").expect("recorded existing coordinates");
    let (mut app, _) = app_for(&source);
    assert!(!app.align.raster.as_ref().expect("session").original.pending, "fixture must start referenced");
    assert!(app.pictures.layers[0].align_pending, "begin failed to exclude the referenced source from the right map");
    app.stop_align_session();
    assert!(!app.pictures.layers[0].align_pending, "closing without applying must keep the original picture placement");
}

#[test]
fn applied_picture_is_brought_into_view_after_the_split_closes() {
    let temp = tempfile::tempdir().expect("temporary picture");
    let (mut app, _) = app_for(&temp.path().join("scan.png"));
    app.apply_align_move().expect("real fit");
    settle(&mut app, |a| a.align.applied);
    let rendered = crate::test_mods::ui_snapshot_support::render([1280, 720], |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| app.render_map_tab_ui(ctx, ui));
    });
    // Fixed C++200x100m extent in EPSG5187;1e-6m is the C++ affine test tolerance.
    let centre = app.home_map.viewport.center_xy;
    assert!((centre[0] - 200100.0).abs() < 1e-6 && (centre[1] - 450050.0).abs() < 1e-6, "actual fit remains outside the map view: {centre:?}");
    let extent = app.home_map.viewport.world_bbox();
    assert!(extent[2] - extent[0] >= 249.999999 && extent[3] - extent[1] >= 124.999999, "C++25% map margin is missing");
    rendered.save_if_requested("alignment-applied");
}

#[test]
fn restored_referenced_source_stays_out_of_the_pair_target_until_close() {
    let temporary = tempfile::tempdir().expect("temporary picture");
    let source = temporary.path().join("scan.png");
    std::fs::write(source.with_extension("pgw"), "2\n0\n0\n-2\n200001\n450099\n").expect("existing EPSG5187 coordinates");
    let (mut app, _) = app_for(&source);
    app.apply_align_move().expect("actual fit");
    settle(&mut app, |a| a.align.applied);
    app.restore_align_originals();
    settle(&mut app, |a| !a.align.applied);
    assert!(app.pictures.layers[0].align_pending, "restored source covers its right-hand alignment target");
    app.work_crs = strata_types::crs::WorkCrs::Epsg5186;
    app.stop_align_session();
    assert!(!app.pictures.layers[0].align_pending);
    assert_eq!(app.pictures.layers[0].source_epsg, Some(5187), "restored coordinates were relabelled as the new work CRS");
}
