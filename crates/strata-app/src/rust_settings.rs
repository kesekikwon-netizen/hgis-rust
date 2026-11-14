//! Rust-only user settings file (`strata-rust.ini`) next to `ka-hgis.ini` (SND1b).
//!
//! `ka-hgis.ini` is shared with the C++ app and is never touched for Rust-only preferences.
//! Format: UTF-8 `key=value` lines after a first comment line; unknown keys are preserved.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use strata_types::theme_display::SoundLevel;

pub(crate) const HEADER_COMMENT: &str =
    "; Strata Rust app settings (UTF-8, key=value; separate from ka-hgis.ini)";
pub(crate) const KEY_SOUND_LEVEL: &str = "sound_level";

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn sound_level_to_value(level: SoundLevel) -> &'static str {
    match level {
        SoundLevel::Normal => "normal",
        SoundLevel::Quiet => "quiet",
        SoundLevel::Off => "off",
    }
}

fn sound_level_from_value(raw: &str) -> SoundLevel {
    match raw.trim() {
        "normal" => SoundLevel::Normal,
        "quiet" => SoundLevel::Quiet,
        "off" => SoundLevel::Off,
        _ => SoundLevel::Quiet,
    }
}

#[must_use]
pub(crate) fn load_sound_level(path: &Path) -> SoundLevel {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return SoundLevel::Quiet;
    };
    let text = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
    let mut level = SoundLevel::Quiet;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == KEY_SOUND_LEVEL {
                level = sound_level_from_value(v);
            }
        }
    }
    level
}

pub(crate) fn save_sound_level(path: &Path, level: SoundLevel) {
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let val = sound_level_to_value(level);
    let mut entries: Vec<String> = Vec::new();
    let mut wrote_sound = false;
    if let Ok(existing) = std::fs::read_to_string(path) {
        let text = existing.strip_prefix('\u{feff}').unwrap_or(&existing);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }
            if let Some((raw_k, raw_v)) = line.split_once('=') {
                let key = raw_k.trim();
                if key.is_empty() {
                    continue;
                }
                if key == KEY_SOUND_LEVEL {
                    if !wrote_sound {
                        entries.push(format!("{KEY_SOUND_LEVEL}={val}"));
                        wrote_sound = true;
                    }
                } else {
                    entries.push(format!("{key}={raw_v}"));
                }
            }
        }
    }
    if !wrote_sound {
        entries.push(format!("{KEY_SOUND_LEVEL}={val}"));
    }
    let mut out = String::with_capacity(HEADER_COMMENT.len() + entries.len() * 24 + 2);
    out.push_str(HEADER_COMMENT);
    out.push('\n');
    for entry in &entries {
        out.push_str(entry);
        out.push('\n');
    }
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "strata-rust.ini".to_owned());
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_file_name(format!("{name}.{}.{seq}.tmp", std::process::id()));
    if std::fs::write(&tmp, out.as_bytes()).and_then(|()| std::fs::rename(&tmp, path)).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use eframe::egui::{self, Event, PointerButton, Pos2, RawInput, Rect, Vec2};
    use strata_types::theme_display::SoundLevel;

    use super::{load_sound_level, save_sound_level, HEADER_COMMENT, KEY_SOUND_LEVEL};
    use crate::app_state::{MainTab, StrataAppState};
    use crate::user_settings::rust_settings_ini_from_shared;

    fn fnv1a_64(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    fn run_tabbar_pass(ctx: &egui::Context, app: &mut StrataAppState, events: Vec<Event>) -> egui::FullOutput {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0));
        ctx.run(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ctx| {
            egui::TopBottomPanel::top("tabbar_rust_settings_test").show(ctx, |ui| {
                app.paint_ribbon_tab_bar(ctx, ui);
            });
        })
    }

    fn find_popup_label_pos(out: &egui::FullOutput, label: &str) -> Pos2 {
        fn walk(shape: &egui::epaint::Shape, label: &str, found: &mut Option<Pos2>) {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    *found = Some(t.pos + Vec2::new(4.0, 4.0));
                }
                egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, label, found)),
                _ => {}
            }
        }
        let mut found = None;
        out.shapes.iter().for_each(|c| walk(&c.shape, label, &mut found));
        found.unwrap_or_else(|| panic!("missing popup label {label}"))
    }

    fn click_theme_radio(ctx: &egui::Context, app: &mut StrataAppState, label: &str) {
        app.active_tab = MainTab::Home;
        app.theme_menu_open = true;
        let _ = run_tabbar_pass(ctx, app, Vec::new());
        let _ = run_tabbar_pass(ctx, app, Vec::new());
        let at = find_popup_label_pos(&run_tabbar_pass(ctx, app, Vec::new()), label);
        let btn = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = run_tabbar_pass(ctx, app, vec![Event::PointerMoved(at), btn(true)]);
        let _ = run_tabbar_pass(ctx, app, vec![btn(false)]);
    }

    #[test]
    fn sound_level_written_on_change_and_read_on_next_start_leaving_shared_ini_byte_identical() {
        let dir = tempfile::tempdir().unwrap();
        let shared_ini = dir.path().join("ka-hgis.ini");
        let shared_bytes_before = b"[RecentSurveys]\r\nitems=\"C:/demo/a.gpkg\\ta\\t1700000000000\"\r\n";
        std::fs::write(&shared_ini, shared_bytes_before).unwrap();
        let shared_hash_before = fnv1a_64(shared_bytes_before);
        let rust_ini = rust_settings_ini_from_shared(&shared_ini).unwrap();
        assert_eq!(rust_ini, dir.path().join("strata-rust.ini"));

        let ctx = egui::Context::default();
        let mut app1 = StrataAppState::new_startup();
        app1.attach_user_settings(Some(shared_ini.clone()));
        app1.load_rust_startup_settings();
        assert_eq!(app1.display_options.sound_level, SoundLevel::Quiet);
        assert!(!rust_ini.exists(), "startup read must not create strata-rust.ini");

        click_theme_radio(&ctx, &mut app1, "소리 끔");
        assert_eq!(app1.display_options.sound_level, SoundLevel::Off);
        assert!(rust_ini.is_file(), "changing sound level must write strata-rust.ini");

        let written_text = std::fs::read_to_string(&rust_ini).unwrap();
        let first_line = written_text.lines().next().unwrap_or_default();
        assert!(first_line.starts_with(';') && first_line.contains("Strata Rust"));
        assert!(written_text.contains(&format!("{KEY_SOUND_LEVEL}=off")), "{written_text}");

        // Rendering idle frames without changing the choice must not rewrite the file.
        std::fs::write(&rust_ini, format!("{HEADER_COMMENT}\nprobe_keep=1\n{KEY_SOUND_LEVEL}=off\n")).unwrap();
        let _ = run_tabbar_pass(&ctx, &mut app1, Vec::new());
        assert!(std::fs::read_to_string(&rust_ini).unwrap().contains("probe_keep=1"));

        // Next start reads the saved choice once at startup.
        let mut app2 = StrataAppState::new_startup();
        app2.attach_user_settings(Some(shared_ini.clone()));
        app2.load_rust_startup_settings();
        assert_eq!(app2.display_options.sound_level, SoundLevel::Off);

        // Shared C++ ka-hgis.ini is byte-for-byte untouched.
        let shared_bytes_after = std::fs::read(&shared_ini).unwrap();
        assert_eq!(fnv1a_64(&shared_bytes_after), shared_hash_before);
        assert_eq!(shared_bytes_after.as_slice(), shared_bytes_before);
    }

    #[test]
    fn missing_empty_unreadable_and_unknown_values_default_to_quiet_without_error() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_sound_level(&dir.path().join("missing.ini")), SoundLevel::Quiet);

        let valid = dir.path().join("valid.ini");
        std::fs::write(&valid, format!("{HEADER_COMMENT}\n{KEY_SOUND_LEVEL}=normal\n")).unwrap();
        assert_eq!(load_sound_level(&valid), SoundLevel::Normal);

        let empty = dir.path().join("empty.ini");
        std::fs::write(&empty, "").unwrap();
        assert_eq!(load_sound_level(&empty), SoundLevel::Quiet);

        let unknown = dir.path().join("unknown.ini");
        std::fs::write(&unknown, format!("{HEADER_COMMENT}\n{KEY_SOUND_LEVEL}=loudest\n")).unwrap();
        assert_eq!(load_sound_level(&unknown), SoundLevel::Quiet);

        let bad_utf8 = dir.path().join("bad_utf8.ini");
        std::fs::write(&bad_utf8, [0xFF_u8, 0xFE, 0x00, 0x81]).unwrap();
        assert_eq!(load_sound_level(&bad_utf8), SoundLevel::Quiet);

        let unreadable_dir = dir.path().join("unreadable_as_dir.ini");
        std::fs::create_dir(&unreadable_dir).unwrap();
        assert_eq!(load_sound_level(&unreadable_dir), SoundLevel::Quiet);
    }

    #[test]
    fn rewriting_sound_level_keeps_unknown_keys_and_header_comment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("strata-rust.ini");
        let initial = format!("{HEADER_COMMENT}\nfuture_alpha=42\n{KEY_SOUND_LEVEL}=normal\nfuture_beta=hello world\n");
        std::fs::write(&path, initial).unwrap();

        save_sound_level(&path, SoundLevel::Off);
        assert_eq!(load_sound_level(&path), SoundLevel::Off);

        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert!(rewritten.lines().next().unwrap_or_default().starts_with(';'));
        assert!(rewritten.contains("future_alpha=42"), "{rewritten}");
        assert!(rewritten.contains("future_beta=hello world"), "{rewritten}");
        assert!(rewritten.contains(&format!("{KEY_SOUND_LEVEL}=off")), "{rewritten}");
        assert!(!rewritten.contains(&format!("{KEY_SOUND_LEVEL}=normal")), "{rewritten}");
    }

    #[test]
    fn snap_run_neither_reads_nor_writes_rust_settings() {
        let real_dir = tempfile::tempdir().unwrap();
        let snap_temp = tempfile::tempdir().unwrap();
        let real_shared = real_dir.path().join("ka-hgis.ini");
        let real_rust = real_dir.path().join("strata-rust.ini");
        std::fs::write(&real_shared, "[RecentSurveys]\r\nitems=@Invalid()\r\n").unwrap();
        let seeded = format!("{HEADER_COMMENT}\n{KEY_SOUND_LEVEL}=off\n");
        std::fs::write(&real_rust, &seeded).unwrap();

        let mut normal = StrataAppState::new_startup();
        normal.attach_user_settings(Some(real_shared.clone()));
        normal.load_rust_startup_settings();
        assert_eq!(normal.display_options.sound_level, SoundLevel::Off);

        let ctx = egui::Context::default();
        let mut snap_app = StrataAppState::new_startup();
        snap_app.snap = Some(crate::snap_shot::SnapRequest::parse(&snap_temp.path().to_string_lossy(), ""));
        snap_app.attach_user_settings_for_snap(Some(real_shared), snap_temp.path());
        snap_app.load_rust_startup_settings();
        assert_eq!(snap_app.display_options.sound_level, SoundLevel::Quiet);

        click_theme_radio(&ctx, &mut snap_app, "소리 보통");
        assert_eq!(snap_app.display_options.sound_level, SoundLevel::Normal);
        assert_eq!(std::fs::read_to_string(&real_rust).unwrap(), seeded);
        assert!(!snap_temp.path().join("strata-rust.ini").exists());
    }
}
