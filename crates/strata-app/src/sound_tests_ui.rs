//! Headless egui tests for the central click hook, theme menu sound setting, and survey save hook (SND1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Sense, Vec2};
use strata_types::theme_display::SoundLevel;

use crate::app_state::{MainTab, StrataAppState};
use crate::sound::{
    attach_controller_for_test, frame_had_button_click, on_frame_end, RecordingBackend,
    SoundController, SoundKind,
};

fn press_events(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn release_events(pos: Pos2) -> Vec<Event> {
    vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

fn run_pass(ctx: &egui::Context, events: Vec<Event>, f: impl FnMut(&egui::Context)) -> FullOutput {
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0));
    ctx.run(
        RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        },
        f,
    )
}

fn simulate_primary_click<F: FnMut(&mut egui::Ui)>(pos: Pos2, mut build_ui: F) -> bool {
    let ctx = egui::Context::default();
    let _ = run_pass(&ctx, Vec::new(), |ctx| {
        egui::CentralPanel::default().show(ctx, &mut build_ui);
    });
    let _ = run_pass(&ctx, press_events(pos), |ctx| {
        egui::CentralPanel::default().show(ctx, &mut build_ui);
    });
    let mut clicked = false;
    let _ = run_pass(&ctx, release_events(pos), |ctx| {
        egui::CentralPanel::default().show(ctx, &mut build_ui);
        clicked = frame_had_button_click(ctx);
    });
    clicked
}

#[test]
fn central_hook_detects_button_checkbox_radio_and_custom_tab_or_menu_click() {
    let mut btn_pos = Pos2::ZERO;
    let _ = simulate_primary_click(Pos2::ZERO, |ui| {
        btn_pos = ui.button("확인").rect.center();
    });
    assert!(simulate_primary_click(btn_pos, |ui| {
        let _ = ui.button("확인");
    }));

    let mut checked = false;
    let mut cb_pos = Pos2::ZERO;
    let _ = simulate_primary_click(Pos2::ZERO, |ui| {
        cb_pos = ui.checkbox(&mut checked, "큰 글씨").rect.center();
    });
    assert!(simulate_primary_click(cb_pos, |ui| {
        let _ = ui.checkbox(&mut checked, "큰 글씨");
    }));

    let mut choice = SoundLevel::Quiet;
    let mut radio_pos = Pos2::ZERO;
    let _ = simulate_primary_click(Pos2::ZERO, |ui| {
        radio_pos = ui
            .radio_value(&mut choice, SoundLevel::Normal, "소리 보통")
            .rect
            .center();
    });
    assert!(simulate_primary_click(radio_pos, |ui| {
        let _ = ui.radio_value(&mut choice, SoundLevel::Normal, "소리 보통");
    }));

    // Custom Sense::click() tab / menu item (gui_ribbon_tabbar.rs, menu_popup.rs).
    let mut tab_pos = Pos2::ZERO;
    let _ = simulate_primary_click(Pos2::ZERO, |ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, 30.0), Sense::click());
        tab_pos = r.center();
    });
    assert!(simulate_primary_click(tab_pos, |ui| {
        let _ = ui.allocate_exact_size(Vec2::new(64.0, 30.0), Sense::click());
    }));
}

#[test]
fn central_hook_ignores_map_canvas_click_hover_typing_drag_and_scroll() {
    let map_clicked = simulate_primary_click(Pos2::new(200.0, 200.0), |ui| {
        let _ = ui.allocate_exact_size(Vec2::new(500.0, 400.0), Sense::click_and_drag());
    });
    assert!(!map_clicked, "map canvas click must not emit a button tick");

    let mut text = String::from("조사명");
    let mut edit_pos = Pos2::ZERO;
    let _ = simulate_primary_click(Pos2::ZERO, |ui| {
        edit_pos = ui.text_edit_singleline(&mut text).rect.center();
    });
    assert!(!simulate_primary_click(edit_pos, |ui| {
        let _ = ui.text_edit_singleline(&mut text);
    }));

    let ctx = egui::Context::default();
    let mut hover_pos = Pos2::ZERO;
    let _ = run_pass(&ctx, Vec::new(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            hover_pos = ui.button("저장").rect.center();
            let _ = ui.text_edit_singleline(&mut text);
        });
    });
    let mut hovered_tick = false;
    let _ = run_pass(
        &ctx,
        vec![
            Event::PointerMoved(hover_pos),
            Event::Text("가나다".to_owned()),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, -2.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("저장");
                let _ = ui.text_edit_singleline(&mut text);
            });
            hovered_tick = frame_had_button_click(ctx);
        },
    );
    assert!(!hovered_tick);
}

#[test]
fn on_frame_end_records_button_tick_and_survey_saved_and_stays_silent_for_snap() {
    let ctx = egui::Context::default();
    let rec = RecordingBackend::new();
    attach_controller_for_test(&ctx, SoundController::new(Box::new(rec.clone())));

    let mut app = StrataAppState::new_startup();
    assert_eq!(app.display_options.sound_level, SoundLevel::Quiet);

    let mut btn_center = Pos2::ZERO;
    let _ = run_pass(&ctx, Vec::new(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            btn_center = ui.button("단추").rect.center();
        });
        on_frame_end(ctx, &app);
    });
    assert!(rec.events().is_empty());

    for ev in [press_events(btn_center), release_events(btn_center)] {
        let _ = run_pass(&ctx, ev, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("단추");
            });
            on_frame_end(ctx, &app);
        });
    }
    assert_eq!(rec.events(), vec![(SoundKind::ButtonPress, SoundLevel::Quiet)]);

    let dir = tempfile::tempdir().unwrap();
    let gpkg = dir.path().join("snd1_save_test.gpkg");
    assert!(app.write_survey_to(&gpkg));
    let _ = run_pass(&ctx, Vec::new(), |ctx| on_frame_end(ctx, &app));
    assert_eq!(rec.events().last().copied(), Some((SoundKind::SurveySaved, SoundLevel::Quiet)));

    let count_after_save = rec.events().len();
    let _ = run_pass(&ctx, Vec::new(), |ctx| on_frame_end(ctx, &app));
    assert_eq!(rec.events().len(), count_after_save);

    assert!(app.write_survey_to(&gpkg));
    let _ = run_pass(&ctx, Vec::new(), |ctx| on_frame_end(ctx, &app));
    assert_eq!(rec.events().len(), count_after_save + 1);
    let count_after_save = rec.events().len();

    app.snap = Some(crate::snap_shot::SnapRequest::parse(
        &dir.path().to_string_lossy(),
        "",
    ));
    for ev in [press_events(btn_center), release_events(btn_center)] {
        let _ = run_pass(&ctx, ev, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("단추");
            });
            on_frame_end(ctx, &app);
        });
    }
    assert_eq!(rec.events().len(), count_after_save);
}

#[test]
fn theme_menu_popup_exposes_three_sound_choices_next_to_paper_look() {
    let ctx = egui::Context::default();
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Home;
    app.theme_menu_open = true;

    for _ in 0..2 {
        let _ = run_pass(&ctx, Vec::new(), |ctx| {
            egui::TopBottomPanel::top("tabbar_test").show(ctx, |ui| {
                app.paint_ribbon_tab_bar(ctx, ui);
            });
        });
    }
    let out = run_pass(&ctx, Vec::new(), |ctx| {
        egui::TopBottomPanel::top("tabbar_test").show(ctx, |ui| {
            app.paint_ribbon_tab_bar(ctx, ui);
        });
    });

    let mut labels = Vec::new();
    let mut off_pos: Option<Pos2> = None;
    fn collect_text(shape: &egui::epaint::Shape, labels: &mut Vec<String>, off_pos: &mut Option<Pos2>) {
        match shape {
            egui::epaint::Shape::Text(t) => {
                labels.push(t.galley.text().to_owned());
                if t.galley.text() == "소리 끔" {
                    *off_pos = Some(t.pos + Vec2::new(4.0, 4.0));
                }
            }
            egui::epaint::Shape::Vec(v) => {
                for s in v {
                    collect_text(s, labels, off_pos);
                }
            }
            _ => {}
        }
    }
    for clipped in &out.shapes {
        collect_text(&clipped.shape, &mut labels, &mut off_pos);
    }
    let joined = labels.join(" | ");
    assert!(joined.contains("새 모양 (종이·흙색)"), "got: {joined}");
    for expected in ["소리 보통", "소리 작게", "소리 끔"] {
        assert!(joined.contains(expected), "missing {expected} in {joined}");
    }

    let click_at = off_pos.expect("소리 끔 position in popup");
    for ev in [press_events(click_at), release_events(click_at)] {
        let _ = run_pass(&ctx, ev, |ctx| {
            egui::TopBottomPanel::top("tabbar_test").show(ctx, |ui| {
                app.paint_ribbon_tab_bar(ctx, ui);
            });
        });
    }
    assert_eq!(app.display_options.sound_level, SoundLevel::Off);
}
