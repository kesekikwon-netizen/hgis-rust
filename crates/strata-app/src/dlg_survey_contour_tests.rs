//! F1d-1 — with no file chosen, accepting the survey-contour dialog only shows 「좌표 파일을 고르세요.」
//! in the preview text and creates nothing (target/cpp-ref/src/app/KaSurveyContourDialog.cpp:126-131).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};
use gdal::vector::LayerAccess;

use std::sync::Mutex;

use crate::app_state::StrataAppState;

static PRESS: Mutex<()> = Mutex::new(());
use crate::dialogs_modal::ActiveModalDialog;

fn collect_texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.job.text.clone()),
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_texts(s, out);
            }
        }
        _ => {}
    }
}

type AccessNode = (String, bool, Option<Rect>);

fn frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> (Vec<AccessNode>, Vec<String>) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        let mut open = app.modals.active == Some(ActiveModalDialog::SurveyContour);
        if open {
            super::render_survey_contour(app, ctx, &mut open);
            if !open {
                app.modals.active = None;
            }
        }
    });
    let mut texts = Vec::new();
    for clipped in &out.shapes {
        collect_texts(&clipped.shape, &mut texts);
    }
    let nodes = out
        .platform_output
        .accesskit_update
        .map_or_else(Vec::new, |update| {
            update
                .nodes
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
        });
    (nodes, texts)
}

#[test]
fn empty_path_accept_shows_choose_file_prompt_and_creates_nothing() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.modals.contour_file_path, "");
    app.modals.active = Some(ActiveModalDialog::SurveyContour);

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let (mut nodes, mut before_texts) = (Vec::new(), Vec::new());
    for _ in 0..3 {
        (nodes, before_texts) = frame(&mut app, &ctx, vec![]);
    }
    assert!(before_texts
        .iter()
        .any(|t| t == "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다."));
    assert!(!before_texts.iter().any(|t| t.contains("점 248개")));
    assert!(!before_texts.iter().any(|t| t == "좌표 파일을 고르세요."));

    let (disabled, at) = nodes
        .iter()
        .find(|(l, _, _)| l == "등고선 만들기")
        .map(|(_, disabled, r)| (*disabled, r.expect("accept button bounds").center()))
        .unwrap_or_else(|| panic!("accept button missing: {nodes:?}"));
    assert!(!disabled, "an empty path can still be refused");
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);
    let (_, after_texts) = frame(&mut app, &ctx, vec![]);

    assert!(
        after_texts.iter().any(|t| t == "좌표 파일을 고르세요."),
        "preview must show choose-file prompt: {after_texts:?}"
    );
    assert!(!after_texts.iter().any(|t| t.contains("점 248개")));
    assert_eq!(app.modals.active, Some(ActiveModalDialog::SurveyContour));
    assert!(app.left_layers.entries.is_empty());
    assert_eq!(app.home_map.draw_guide_message, None);
    assert!(!app.is_dirty);
}

fn fixed_count_phrases(texts: &[String]) -> Vec<String> {
    const NEEDLES: [&str; 4] = ["점 248개", "24.5", "48.2", "표고점 248개"];
    texts
        .iter()
        .filter(|t| NEEDLES.iter().any(|n| t.contains(n)))
        .cloned()
        .collect()
}

fn open_contour(path: &str) -> (StrataAppState, egui::Context, Vec<String>) {
    let mut app = StrataAppState::new_startup();
    app.modals.contour_file_path = path.to_owned();
    app.modals.active = Some(ActiveModalDialog::SurveyContour);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut texts = Vec::new();
    for _ in 0..3 {
        (_, texts) = frame(&mut app, &ctx, vec![]);
    }
    for _ in 0..100 {
        if !texts
            .iter()
            .any(|text| text == super::survey_contour_sheet::READING)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        (_, texts) = frame(&mut app, &ctx, vec![]);
    }
    (app, ctx, texts)
}

/// Any typed path must not paint the invented preview, and accepting it must not
/// add an empty section line (`dlg_survey_contour.rs` used to do both).
#[test]
fn arbitrary_path_never_shows_fixed_counts_or_writes_section_line() {
    let (mut app, ctx, texts) = open_contour("not-a-survey-248.csv");
    assert!(
        fixed_count_phrases(&texts).is_empty(),
        "invented counts in {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요."),
        "missing file must say so: {texts:?}"
    );

    let (nodes, _) = frame(&mut app, &ctx, vec![]);
    let (disabled, at) = nodes
        .iter()
        .find(|(l, _, _)| l == "등고선 만들기")
        .map(|(_, disabled, r)| (*disabled, r.expect("accept button bounds").center()))
        .unwrap_or_else(|| panic!("accept button missing: {nodes:?}"));
    assert!(!disabled, "a chosen path can be built");
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);
    let (_, after) = frame(&mut app, &ctx, vec![]);

    assert!(
        fixed_count_phrases(&after).is_empty(),
        "invented counts after click: {after:?}"
    );
    assert!(
        app.left_layers.entries.is_empty(),
        "no domain layer: {:?}",
        app.left_layers.entries
    );
    assert_eq!(app.home_map.draw_guide_message, None);
    assert!(!app.is_dirty);
}

/// Preview counts come from the file, in the C++ sentence
/// (`KaSurveyContourDialog.cpp` `showPreview`: 점 N개, 표고 min–max m, 등고선 약 L줄).
#[test]
fn sample_csv_preview_matches_point_count_and_elevation_range() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("points.csv");
    std::fs::write(
        &path,
        "점번호,X,Y,표고\n1,200000,500000,10\n2,200010,500000,12.5\n3,200000,500010,11\n4,200010,500010,15.25\n",
    )
    .unwrap();
    let (_, _, texts) = open_contour(&path.display().to_string());
    assert!(
        fixed_count_phrases(&texts).is_empty(),
        "invented counts for a real file: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "점 4개, 표고 10.0–15.3 m. 등고선 약 10줄."),
        "preview must match the four rows: {texts:?}"
    );
}

fn press_make(app: &mut StrataAppState, ctx: &egui::Context) {
    // One build at a time: the worker slot is process-wide.
    let _guard = PRESS.lock().unwrap_or_else(|poison| poison.into_inner());
    let (nodes, _) = frame(app, ctx, vec![]);
    let at = nodes
        .iter()
        .find(|(label, _, _)| label == "등고선 만들기")
        .and_then(|(_, _, bounds)| *bounds)
        .unwrap()
        .center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    frame(app, ctx, vec![Event::PointerMoved(at)]);
    frame(app, ctx, vec![press(true)]);
    frame(app, ctx, vec![press(false)]);
    for _ in 0..200 {
        frame(app, ctx, vec![]);
        let settled = !super::survey_contour_job::running()
            && (app.modals.active.is_none() || app.modals.contour_build_note.is_some());
        if settled {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// A four-corner slope becomes four contour lines in the survey folder and a reference row.
#[test]
fn sample_csv_writes_the_package_and_shows_the_contour_layer() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"survey-file").unwrap();
    let csv = dir.path().join("slope.csv");
    std::fs::write(&csv, "X,Y,Z\n0,0,10\n100,0,10\n0,100,20\n100,100,20\n").unwrap();
    let (mut app, ctx, _) = open_contour(&csv.display().to_string());
    app.survey_path = Some(survey);
    app.modals.contour_interval_m = 2.0;
    app.modals.contour_cell_size_m = 10.0;
    press_make(&mut app, &ctx);

    assert_eq!(app.modals.active, None);
    assert!(app.is_dirty);
    assert_eq!(app.status_bar_hint, "등고선 4줄, 색 구간 0개를 올렸습니다.");
    let row = app
        .left_layers
        .entries
        .iter()
        .find(|entry| entry.label_ko == "등고선")
        .unwrap();
    assert_eq!(row.feature_count, 4);
    assert_eq!(row.group, strata_types::schema::LegendGroup::ReferenceMap);
    assert!(row.read_only);
    assert!(app.ref_vectors.spec_of("contour:slope:lines").is_some());
    assert!(app
        .left_layers
        .entries
        .iter()
        .all(|entry| entry.layer_key != "section_line"));
    let gpkg = dir
        .path()
        .join("측량등고선")
        .join("slope")
        .join("contours.gpkg");
    assert!(
        gpkg.is_file()
            && dir
                .path()
                .join("측량등고선")
                .join("slope")
                .join("surface.tif")
                .is_file()
    );
    let dataset = gdal::Dataset::open(&gpkg).unwrap();
    let mut elevs = Vec::new();
    for feature in dataset.layer_by_name("contour_lines").unwrap().features() {
        if let Some(gdal::vector::FieldValue::RealValue(z)) = feature.field(0).unwrap() {
            elevs.push((z * 100.0).round() as i32);
        }
    }
    elevs.sort();
    assert_eq!(elevs, vec![1200, 1400, 1600, 1800]);
}

/// Two points are refused and the survey folder gains no contour package.
#[test]
fn too_few_points_write_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"survey-file").unwrap();
    let csv = dir.path().join("two.csv");
    std::fs::write(&csv, "X,Y,Z\n0,0,1\n1,1,2\n").unwrap();
    let (mut app, ctx, _) = open_contour(&csv.display().to_string());
    app.survey_path = Some(survey);
    press_make(&mut app, &ctx);
    let (_, after) = frame(&mut app, &ctx, vec![]);
    assert!(after.iter().any(|text| text.contains("3개")), "{after:?}");
    assert!(!dir.path().join("측량등고선").exists());
    assert!(app.left_layers.entries.is_empty());
    assert!(!app.is_dirty);
}
