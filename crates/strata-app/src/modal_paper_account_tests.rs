//! `modal_paper_account` tests — `KaPaperDialog` (`용지 설정`) behaviour in headless egui frames
//! (`target/cpp-ref/src/app/KaPaperDialog.cpp:43-198`, `target/cpp-ref/src/app/MainWindowExport.cpp:214-255`;
//! expected values ported from `target/cpp-ref/tests/test_drawing_polish.cpp:125-169`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};

#[derive(Debug, Clone)]
struct NodeSnap {
    label: Option<String>,
    num: Option<f64>,
    disabled: bool,
    bounds: Option<Rect>,
}

fn collect_texts(shape: &Shape, out: &mut Vec<(String, egui::Color32)>) {
    match shape {
        Shape::Text(t) => {
            let col = t
                .galley
                .job
                .sections
                .first()
                .map_or(t.fallback_color, |s| s.format.color);
            out.push((t.galley.job.text.clone(), col));
        }
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_texts(s, out);
            }
        }
        _ => {}
    }
}

fn run_frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> (Vec<NodeSnap>, Vec<(String, egui::Color32)>) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        render_active_modal_dialogs(app, ctx);
    });
    let mut texts = Vec::new();
    for clipped in &out.shapes {
        collect_texts(&clipped.shape, &mut texts);
    }
    let nodes = out.platform_output.accesskit_update.map_or_else(Vec::new, |u| {
        u.nodes
            .iter()
            .map(|(_, n)| NodeSnap {
                label: n.label().map(str::to_owned),
                num: n.numeric_value(),
                disabled: n.is_disabled(),
                bounds: n.bounds().map(|b| {
                    Rect::from_min_max(
                        Pos2::new(b.x0 as f32, b.y0 as f32),
                        Pos2::new(b.x1 as f32, b.y1 as f32),
                    )
                }),
            })
            .collect()
    });
    (nodes, texts)
}

fn click_label(app: &mut StrataAppState, ctx: &egui::Context, label: &str) {
    let (nodes, _) = run_frame(app, ctx, vec![]);
    let at = nodes
        .iter()
        .find(|n| n.label.as_deref() == Some(label))
        .and_then(|n| n.bounds)
        .unwrap_or_else(|| panic!("missing button {label:?} in {nodes:?}"))
        .center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run_frame(app, ctx, vec![Event::PointerMoved(at)]);
    run_frame(app, ctx, vec![press(true)]);
    run_frame(app, ctx, vec![press(false)]);
}

#[test]
fn locked_paper_size_is_a_reading_line_and_presets_follow_cpp_paper_dialog() {
    // Ported from target/cpp-ref/tests/test_drawing_polish.cpp:125-158
    // (TestDrawingPolish::lockedPaperSizeIsAReadingLine) and KaPaperDialog.cpp:45, 143-186.
    let mut app = StrataAppState::new_startup();
    app.ribbon.sub_toolbar_visible = true;
    app.modals.active = Some(ActiveModalDialog::PaperSetup);
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    ctx.enable_accesskit();

    let (mut nodes, mut texts) = (Vec::new(), Vec::new());
    for _ in 0..3 {
        (nodes, texts) = run_frame(&mut app, &ctx, vec![]);
    }

    // Window title is verbatim C++ "용지 설정" (KaPaperDialog.cpp:45).
    assert!(
        texts.iter().any(|(t, _)| t == "용지 설정"),
        "expected exact title '용지 설정', got {texts:?}"
    );
    assert!(!texts.iter().any(|(t, _)| t == "용지 설정 - Strata"));
    for lbl in ["A4", "A3", "직접 입력", "세로", "가로", "용지 만들기", "취소"] {
        assert!(
            nodes.iter().any(|n| n.label.as_deref() == Some(lbl)),
            "missing control {lbl:?} in {nodes:?}"
        );
    }

    // Initial A4 세로: reading lines "210" and "297" in dark ink, no numeric spin widgets.
    assert!(nodes.iter().all(|n| n.num.is_none()), "spin visible on A4: {nodes:?}");
    let (_, w_col) = texts
        .iter()
        .find(|(t, _)| t == "210")
        .expect("reading '210' must be visible");
    assert!(
        u16::from(w_col.r()) + u16::from(w_col.g()) + u16::from(w_col.b()) < 300
            && w_col.a() > 200,
        "reading must be in dark ink, got {w_col:?}"
    );
    assert!(texts.iter().any(|(t, _)| t == "297"));
    assert!(texts
        .iter()
        .any(|(t, _)| t == "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."));

    // Click "가로": A4 가로 is the 317x220 field sheet with the scale warning note.
    click_label(&mut app, &ctx, "가로");
    let (_, texts) = run_frame(&mut app, &ctx, vec![]);
    assert!(texts.iter().any(|(t, _)| t == "317"), "texts={texts:?}");
    assert!(texts.iter().any(|(t, _)| t == "220"), "texts={texts:?}");
    assert!(texts
        .iter()
        .any(|(t, _)| t.starts_with("A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다.")));

    // Click "A3" (while "가로" stays selected): 420 x 297, default note.
    click_label(&mut app, &ctx, "A3");
    let (_, texts) = run_frame(&mut app, &ctx, vec![]);
    assert!(texts.iter().any(|(t, _)| t == "420"), "texts={texts:?}");
    assert!(texts.iter().any(|(t, _)| t == "297"), "texts={texts:?}");
    assert!(texts
        .iter()
        .any(|(t, _)| t == "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."));

    // Switch to A4 가로 then click "직접 입력": note resets to default (p != 0), spins appear enabled.
    click_label(&mut app, &ctx, "A4");
    click_label(&mut app, &ctx, "직접 입력");
    let (nodes, texts) = run_frame(&mut app, &ctx, vec![]);
    assert!(
        texts
            .iter()
            .any(|(t, _)| t == "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."),
        "custom mode must show default note (KaPaperDialog.cpp:149): {texts:?}"
    );
    let mut spins: Vec<_> = nodes.iter().filter(|n| n.num.is_some()).collect();
    spins.sort_by(|a, b| {
        a.bounds
            .expect("bounds")
            .min
            .y
            .total_cmp(&b.bounds.expect("bounds").min.y)
    });
    assert_eq!(spins.len(), 2, "expected 2 enabled spin fields: {nodes:?}");
    assert!(spins.iter().all(|s| !s.disabled));

    // Type 123.4 into the width spin (test_drawing_polish.cpp:156) and accept ("용지 만들기").
    let w_at = spins[0].bounds.expect("width spin bounds").center();
    let press = |pressed| Event::PointerButton {
        pos: w_at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run_frame(&mut app, &ctx, vec![Event::PointerMoved(w_at)]);
    run_frame(&mut app, &ctx, vec![press(true)]);
    run_frame(&mut app, &ctx, vec![press(false)]);
    run_frame(&mut app, &ctx, vec![]);
    run_frame(&mut app, &ctx, vec![Event::Text("123.4".to_owned())]);
    run_frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        }],
    );
    click_label(&mut app, &ctx, "용지 만들기");

    assert_eq!(app.modals.active, None);
    assert_eq!(app.active_tab, MainTab::Drawing);
    assert!(!app.ribbon.sub_toolbar_visible);
    assert!((app.drawing.sheet.width_mm - 123.4).abs() < 1e-6);
    assert!((app.drawing.sheet.height_mm - 220.0).abs() < 1e-6);
    assert!((app.drawing.sheet.map_frame.width_mm - 103.4).abs() < 1e-6);
    assert!(app.drawing.sheet.all_decorations_inside_sheet());
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some(StrataAppState::DRAWING_STUDIO_WELCOME)
    );
}

#[test]
fn stock_paper_dialog_keeps_its_disabled_fields() {
    // Ported from target/cpp-ref/tests/test_drawing_polish.cpp:160-169
    // (TestDrawingPolish::stockPaperDialogKeepsItsDisabledFields).
    let mut app = StrataAppState::new_startup();
    app.paper_look = false;
    app.modals.active = Some(ActiveModalDialog::PaperSetup);
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    ctx.enable_accesskit();

    let mut nodes = Vec::new();
    for _ in 0..3 {
        (nodes, _) = run_frame(&mut app, &ctx, vec![]);
    }
    let mut spins: Vec<_> = nodes.iter().filter(|n| n.num.is_some()).collect();
    spins.sort_by(|a, b| {
        a.bounds
            .expect("bounds")
            .min
            .y
            .total_cmp(&b.bounds.expect("bounds").min.y)
    });
    assert_eq!(spins.len(), 2, "stock look must show both spin boxes: {nodes:?}");
    assert!(spins.iter().all(|s| s.disabled), "stock preset spins must be disabled: {spins:?}");
    assert_eq!(spins[0].num, Some(210.0));
    assert_eq!(spins[1].num, Some(297.0));
}

#[test]
fn cancelling_paper_dialog_keeps_existing_sheet_and_tabs_unchanged() {
    // KaPaperDialog.cpp:191-198 + MainWindowExport.cpp:216-217 + KaDrawingStudio.cpp:447-451.
    let mut app = StrataAppState::new_startup();
    app.drawing
        .set_paper_and_orientation(SheetPaperSize::A3, SheetOrientation::Landscape);
    assert_eq!((app.drawing.sheet.width_mm, app.drawing.sheet.height_mm), (420.0, 297.0));
    app.modals.active = Some(ActiveModalDialog::PaperSetup);

    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    ctx.enable_accesskit();
    for _ in 0..3 {
        run_frame(&mut app, &ctx, vec![]);
    }
    click_label(&mut app, &ctx, "가로");
    click_label(&mut app, &ctx, "취소");

    assert_eq!(app.modals.active, None);
    assert!(!app.open_studio_tabs.contains(&MainTab::Drawing));
    assert_eq!(app.drawing.sheet.paper_size, SheetPaperSize::A3);
    assert_eq!(app.drawing.sheet.orientation, SheetOrientation::Landscape);
    assert_eq!((app.drawing.sheet.width_mm, app.drawing.sheet.height_mm), (420.0, 297.0));
}
