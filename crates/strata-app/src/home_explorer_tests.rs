//! Real folder behavior; target/cpp-ref/src/app/KaFileBrowserPanel.cpp:283-387, owner explorer request.
use super::*;
fn settle(state: &mut ExplorerState) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while state.busy() && std::time::Instant::now() < deadline {
        state.poll();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(!state.busy(), "folder IO failed to settle");
}
#[test]
fn navigation_lists_actual_metadata_folders_first_and_filters_case_insensitively() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    std::fs::create_dir(temp.path().join("z-folder")).expect("folder");
    std::fs::write(temp.path().join("A-scan.tif"), b"actual five").expect("file");
    let mut state = ExplorerState::default();
    state.navigate(Location::Folder(temp.path().to_path_buf()));
    settle(&mut state);
    assert_eq!(state.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["z-folder", "A-scan.tif"]);
    assert_eq!(state.entries[1].bytes, Some(11));
    assert_eq!(state.entries[0].bytes, None);
    assert_eq!(state.entries[1].modified, std::fs::metadata(temp.path().join("A-scan.tif")).expect("metadata").modified().ok());
    state.set_filter("SCAN".to_owned());
    assert_eq!(state.visible().iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["A-scan.tif"]);
}
#[test]
fn fast_navigation_discards_old_results_and_back_forward_up_preserve_locations() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    std::fs::create_dir(&a).expect("a");
    std::fs::create_dir(&b).expect("b");
    std::fs::write(a.join("old.txt"), "a").expect("old");
    std::fs::write(b.join("current.txt"), "b").expect("current");
    let mut state = ExplorerState::default();
    state.navigate(Location::Folder(a.clone()));
    state.navigate(Location::Folder(b.clone()));
    settle(&mut state);
    assert_eq!(state.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["current.txt"]);
    state.travel(-1);
    settle(&mut state);
    assert_eq!(state.location, Location::Folder(a));
    state.travel(1);
    settle(&mut state);
    assert_eq!(state.location, Location::Folder(b));
    state.up();
    settle(&mut state);
    assert_eq!(state.location, Location::Folder(temp.path().to_path_buf()));
}
#[test]
fn multiple_pins_survive_restart_and_removal_without_touching_shared_settings() {
    let temp = tempfile::tempdir().expect("temporary settings");
    let shared = temp.path().join("ka-hgis.ini");
    std::fs::write(&shared, b"owner-settings").expect("shared");
    let prefs = temp.path().join("strata-explorer.json");
    let a = temp.path().join("survey A");
    let b = temp.path().join("survey B");
    let mut state = ExplorerState::default();
    state.configure(prefs.clone());
    settle(&mut state);
    state.pin(a.clone());
    state.pin(b.clone());
    state.pin(a.clone());
    settle(&mut state);
    let mut again = ExplorerState::default();
    again.configure(prefs.clone());
    settle(&mut again);
    assert_eq!(again.pins, [a.clone(), b.clone()], "chosen folders were not restored exactly once");
    again.unpin(&a);
    settle(&mut again);
    let mut last = ExplorerState::default();
    last.configure(prefs);
    settle(&mut last);
    assert_eq!(last.pins, [b]);
    assert_eq!(std::fs::read(shared).expect("shared unchanged"), b"owner-settings");
}

#[test]
fn a_render_snapshot_cannot_replay_old_pending_preference_writes() {
    let temp = tempfile::tempdir().expect("temporary preferences");
    let file = temp.path().join("strata-explorer.json");
    let mut owner = ExplorerState::default();
    owner.configure(file.clone());
    settle(&mut owner);
    let old = temp.path().join("old");
    let current = temp.path().join("current");
    owner.pin(old.clone());
    let mut snapshot = owner.clone();
    owner.unpin(&old);
    owner.pin(current.clone());
    settle(&mut owner);
    snapshot.poll();
    settle(&mut snapshot);
    let mut again = ExplorerState::default();
    again.configure(file);
    settle(&mut again);
    assert_eq!(again.pins, [current], "a copied render state overwrote newer preferences");
}
#[test]
fn actual_home_at_narrow_width_never_paints_overlapping_dashboard_columns() {
    let mut app = crate::app_state::StrataAppState::new_startup();
    let rendered = crate::test_mods::ui_snapshot_support::render([640, 480], |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| app.render_home_tab_ui(ui));
    });
    for (text, rect) in rendered.text_rects() {
        assert!(rect.right() <= 640.5, "home text overflows640px: {text} at{rect:?}");
    }
    assert!(rendered.texts().iter().any(|s| s == "파일함"), "home is still a dashboard without folder navigation");
    rendered.save_if_requested("home-explorer-640");
}

#[test]
fn requested_size_order_keeps_folders_first_with_a_stable_name_tie_break() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    std::fs::create_dir(temp.path().join("folder")).expect("folder");
    for (name, data) in [("a.txt", "four"), ("b.txt", "x"), ("c.txt", "also")] {
        std::fs::write(temp.path().join(name), data).expect("actual sizes");
    }
    let mut state = ExplorerState::default();
    state.navigate(Location::Folder(temp.path().into()));
    settle(&mut state);
    state.set_sort(Sort::Size, true);
    settle(&mut state);
    assert_eq!(state.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["folder", "a.txt", "c.txt", "b.txt"]);
    state.set_sort(Sort::Size, false);
    settle(&mut state);
    assert_eq!(state.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["folder", "b.txt", "a.txt", "c.txt"]);
}

#[test]
fn pending_pin_writes_finish_even_after_leaving_the_home_tab() {
    let temp = tempfile::tempdir().expect("temporary preferences");
    let file = temp.path().join("strata-explorer.json");
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.explorer.configure(file.clone());
    settle(&mut app.explorer);
    app.explorer.pin(temp.path().join("first"));
    app.explorer.poll();
    app.explorer.pin(temp.path().join("last"));
    app.active_tab = crate::app_state::MainTab::Map;
    let ctx = eframe::egui::Context::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while app.explorer.busy() && std::time::Instant::now() < deadline {
        let _ = ctx.run(Default::default(), |ctx| {
            app.render_workspace_egui(ctx);
        });
        std::thread::sleep(std::time::Duration::from_millis(3));
    }
    let mut again = ExplorerState::default();
    again.configure(file);
    settle(&mut again);
    assert_eq!(again.pins, [temp.path().join("first"), temp.path().join("last")], "leaving home stranded the latest pin write");
}

#[test]
fn actual_folder_rows_and_long_pins_fit_each_monitor_without_text_overlap() {
    let temp = tempfile::tempdir().expect("temporary content");
    for index in 0..80 {
        std::fs::write(temp.path().join(format!("survey-plan-with-a-long-file-name-{index:03}.png")), b"actual bytes").expect("file");
    }
    for [width, height] in [[640, 480], [900, 600], [1920, 1080]] {
        let mut app = crate::app_state::StrataAppState::new_startup();
        app.explorer.navigate(Location::Folder(temp.path().into()));
        settle(&mut app.explorer);
        app.explorer.pin(temp.path().join("a-long-frequent-survey-folder-name-that-must-elide"));
        let rendered = crate::test_mods::ui_snapshot_support::render([width, height], |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| app.render_home_tab_ui(ui));
        });
        let glyphs = rendered.text_clips().into_iter().filter(|(_, r, c)| r.intersects(*c)).collect::<Vec<_>>();
        for (text, rect, clip) in &glyphs {
            assert!(rect.left() >= -0.5 && rect.right() <= width as f32 + 0.5, "{width}: {text} overflows: {rect:?} clip{clip:?}");
        }
        for (i, (a, ar, _)) in glyphs.iter().enumerate() {
            for (b, br, _) in &glyphs[i + 1..] {
                assert!(!ar.shrink(1.0).intersects(br.shrink(1.0)), "{width}: overlapping {a} {ar:?} and {b} {br:?}");
            }
        }
        assert!(glyphs.iter().filter(|(text, _, _)| text.starts_with("survey-plan")).count() < 30, "offscreen rows were painted");
        rendered.save_if_requested(&format!("home-folder-{width}"));
    }
}
