//! The scale box of the sheet: what is typed waits for 「적용」, a preset number applies at once.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::StrataAppState;

/// ported from the C++ behaviour (KaDrawingStudio.cpp:1061-1082, 3121-3137, `keyboardTracking` off): a number
/// typed or dragged in the box changes nothing until it is applied (「적용」, Enter or leaving the box); then
/// the sheet takes it, 10 at least, and the status line says 「축척을 1 : N 로 맞췄습니다.」. Afterwards the box
/// shows the sheet's scale again.
#[test]
fn a_typed_scale_waits_for_apply() {
    let mut app = StrataAppState::new_startup();
    let before = app.drawing.sheet.scale_denominator;
    app.edit_sheet_scale(750);
    assert_eq!((app.drawing.sheet.scale_denominator, app.sheet_scale_entry()), (before, 750), "typing alone changes nothing");
    app.apply_sheet_scale_entry();
    assert_eq!(app.drawing.sheet.scale_denominator, 750);
    assert_eq!(app.status_left_message(), "축척을 1 : 750 로 맞췄습니다.");
    assert_eq!(app.sheet_scale_entry(), 750);
    app.edit_sheet_scale(3);
    app.apply_sheet_scale_entry();
    assert_eq!((app.drawing.sheet.scale_denominator, app.status_left_message()), (10, "축척을 1 : 10 로 맞췄습니다."));
}

/// A preset number applies at once (`:1100-1103`), with the same words.
#[test]
fn a_preset_scale_applies_at_once() {
    let mut app = StrataAppState::new_startup();
    app.pick_sheet_scale(2000);
    assert_eq!((app.drawing.sheet.scale_denominator, app.status_left_message()), (2000, "축척을 1 : 2000 로 맞췄습니다."));
}

/// A number typed and not applied is dropped once the scale changes some other way (the wheel of 「지도 조정」,
/// 「화면 맞춤」, a preset): the box then shows the sheet's new scale, as the C++ box follows the map.
#[test]
fn a_typed_scale_gives_way_to_a_scale_changed_otherwise() {
    let mut app = StrataAppState::new_startup();
    app.edit_sheet_scale(750);
    app.drawing.sheet.scale_denominator = 2500;
    assert_eq!(app.sheet_scale_entry(), 2500);
    app.apply_sheet_scale_entry();
    assert_eq!(app.drawing.sheet.scale_denominator, 2500);
}
