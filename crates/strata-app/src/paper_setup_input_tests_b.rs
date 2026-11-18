//! Actual lifecycle, target/cpp-ref/src/app/KaDrawingStudio.cpp:447-454 and KaPaperDialog.cpp:191-198.
use super::*;

pub(super) fn card(h: &Harness) -> Rect {
    h.ctx.read_response(egui::Id::new("ka_modal_paper_setup")).expect("actual paper surface").rect
}

#[test]
fn paper_setup_header_close_and_reopen_reset_the_draft() {
    let mut h = start([1680, 960]);
    let before = work(&h);
    open(&mut h);
    click(&mut h, "A3");
    click(&mut h, "가로");
    click(&mut h, "×");
    assert!(h.app.modals.active.is_none());
    assert_eq!(work(&h), before);
    key(&mut h, Key::L, Modifiers { ctrl: true, ..Default::default() });
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let output = h.frame(vec![]);
    for text in ["210", "297"] {
        assert!(visible_text(&output, text, h.ctx.screen_rect()).is_some());
    }
    // One logical point is the render rounding allowance, not a sheet/map tolerance.
    assert!(card(&h).center().distance(h.ctx.screen_rect().center()) <= 1.0, "keyboard entry reused a previous click anchor");
    h.save("paper-keyboard-reopened");
    click(&mut h, "취소");
    assert_eq!(work(&h), before);
}

#[test]
fn paper_setup_sheet_trigger_connects_upwards_and_preserves_existing_layout() {
    let mut h = start([1680, 960]);
    let before = data(&h);
    open(&mut h);
    click(&mut h, "용지 만들기");
    h.app.drawing.sheet.scale_denominator = 2500;
    h.app.drawing.sheet_center = Some([222_000.0, 360_000.0]); // Prepared EPSG:5187 metres.
    let source = control(&mut h, "용지/방향");
    h.press(source.center(), PointerButton::Primary);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(card(&h).bottom() < source.top());
    let output = h.frame(vec![]);
    assert!(
        output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, Shape::Path(p) if p.closed && p.points.len() == 3 && p.points.iter().any(|v| (v.x-source.center().x).abs()<1.0 && (v.y-(source.top()-1.0)).abs()<1.0))),
        "sheet paper control has no upward connection"
    );
    h.save("paper-sheet-trigger");
    click(&mut h, "가로");
    click(&mut h, "용지 만들기");
    assert!(h.app.modals.active.is_none());
    assert_eq!((h.app.drawing.sheet.width_mm, h.app.drawing.sheet.height_mm), (317.0, 220.0));
    assert_eq!(h.app.drawing.sheet.scale_denominator, 2500);
    assert_eq!(h.app.drawing.sheet_center, Some([222_000.0, 360_000.0]));
    assert_eq!(data(&h), before);
}

#[test]
fn paper_setup_resize_keeps_actions_and_custom_draft_reachable() {
    let mut h = start([1680, 960]);
    let before = work(&h);
    open(&mut h);
    click(&mut h, "직접 입력");
    h.size = [480, 360];
    for _ in 0..5 {
        h.frame(vec![]);
    }
    h.save("paper-resized-custom-short");
    for label in ["취소", "용지 만들기"] {
        assert!(h.ctx.screen_rect().expand(1.0).contains_rect(control(&mut h, label)));
    }
    click(&mut h, "취소");
    assert!(h.app.modals.active.is_none());
    assert_eq!(work(&h), before);
}

#[test]
fn paper_setup_native_double_dpi_has_visible_actions_in_short_client() {
    let mut h = start([480, 360]);
    h.app.modals.active = Some(ActiveModalDialog::PaperSetup);
    let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(480.0, 360.0));
    let mut textures = crate::gui_tab_drawing::sheet_raster::Textures::default();
    let mut output = egui::FullOutput::default();
    for frame in 0..10 {
        let mut input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(100.0 + f64::from(frame) * 0.1),
            ..Default::default()
        };
        input.viewports.get_mut(&egui::ViewportId::ROOT).expect("native viewport").native_pixels_per_point = Some(2.0);
        output = h.ctx.run(input, |ctx| {
            h.app.render_workspace_egui(ctx);
        });
        textures.apply(&output.textures_delta);
    }
    assert_eq!(output.pixels_per_point, 2.0);
    for label in ["A4", "A3", "직접 입력", "용지 만들기", "취소"] {
        assert!(visible_text(&output, label, screen).is_some(), "DPI clipped paper action: {label}");
    }
    crate::test_mods::ui_snapshot_support::save_frame_if_requested(&h.ctx, &output, &textures, [960, 720], "paper-native-dpi2-short");
}
