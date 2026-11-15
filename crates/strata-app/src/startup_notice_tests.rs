//! The startup notice: the timeline of the section (ported from tests/test_startup_strata.cpp), the reading
//! time, and what a frame of the card holds.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, pos2, vec2, Color32, Rect};

use super::paint::{paint_card, BEDROCK, CLAY, PAPER};
use super::*;

/// ported from tests/test_startup_strata.cpp:strataTimelineRunsInOrderAndReducedMotionSkipsIt — four
/// boundaries bottom up, the pit, the clay of stratum Ⅲ, the 기준점, and the hairlines that fade.
#[test]
fn the_timeline_runs_in_order() {
    for index in 0..4_u8 {
        let start = 0.4 + 0.35 * f64::from(index);
        assert!(boundary_reveal(index, start) < 1e-12);
        assert!((boundary_reveal(index, start + 0.35) - 1.0).abs() < 1e-12);
    }
    assert!(pit_reveal(1.9) < 1e-12 && (pit_reveal(2.4) - 1.0).abs() < 1e-12);
    assert_eq!(clay_reveal(2.6), 0.0);
    assert!((clay_reveal(3.2) - 1.0).abs() < 1e-12);
    assert_eq!(marker_reveal(3.2), 0.0);
    assert!((marker_reveal(3.6) - 1.0).abs() < 1e-12);
    assert!(cross_visibility(3.6) > 0.9);
    assert_eq!(cross_visibility(3.0), 0.0);
    assert_eq!(cross_visibility(TIMELINE_SECONDS), 0.0);
    assert!(staff_reveal(0.4) < 1e-12 && (staff_reveal(1.8) - 1.0).abs() < 1e-12);
}

/// The notice is read for five seconds (`ReadingDurationMs`); the level string is the progress line and
/// reaches the staff a little before the notice closes (92 % of the time, `KaStartupSplash.cpp:201-203`); the
/// timeline of the section is stretched to end there too (`:249-250`).
#[test]
fn the_reading_time_is_five_seconds_and_the_string_arrives_just_before() {
    let mut notice = StartupNotice::new(false);
    assert_eq!(notice.status(0.5), "앱을 준비하고 있습니다");
    assert_eq!((notice.progress(0.5), notice.seconds(0.5)), (0.0, 0.0), "nothing moves before the app is ready");
    notice.mark_ready(0.0);
    assert_eq!(notice.status(1.0), "잠시 후 홈 화면이 열립니다");
    assert!((notice.progress(2.3) - 0.5).abs() < 1e-9, "2.3 s of 4.6 s");
    assert!((notice.seconds(4.6) - TIMELINE_SECONDS).abs() < 1e-9);
    assert_eq!(notice.progress(4.7), 1.0);
    assert_eq!(notice.status(4.7), "준비 완료");
    assert!(!notice.done(4.99) && notice.done(5.0));
}

/// ported from tests/test_startup_strata.cpp:motionGetsItsTimeWhenTheAppIsReadyLate — an app that is ready
/// only after the reading time still shows the drawing for three seconds (`kMinMotionMs`).
#[test]
fn the_motion_gets_its_time_when_the_app_is_ready_late() {
    let mut notice = StartupNotice::new(false);
    notice.mark_ready(5.4);
    assert!(!notice.done(6.4));
    assert!(!notice.done(8.39) && notice.done(8.4));
    assert!((notice.seconds(5.4 + 3.0 * 0.92) - TIMELINE_SECONDS).abs() < 1e-9);
}

/// Reduced motion (Windows 「애니메이션 효과 표시」 off): the finished picture at once, for the reading time only.
#[test]
fn reduced_motion_shows_the_finished_picture_for_the_reading_time() {
    let mut notice = StartupNotice::new(true);
    notice.mark_ready(4.0);
    assert_eq!((notice.frame(4.1).seconds, notice.frame(4.1).progress), (10.0, 1.0));
    assert!(notice.done(5.0), "no extra time for a drawing that does not move");
}

fn luminance(colour: Color32) -> f64 {
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(colour.r()) + 0.7152 * linear(colour.g()) + 0.0722 * linear(colour.b())
}

/// ported from tests/test_startup_strata.cpp:strataNoticeShowsBedrockAndReadableNoticesFromTheFirstFrame —
/// the ink of the copyright line (paper at 96 %) reads on the bedrock: at least 4.5 to 1.
#[test]
fn the_notice_ink_reads_on_the_bedrock() {
    let mix = |ink: u8, under: u8| (f64::from(ink) * 0.96 + f64::from(under) * 0.04).round() as u8;
    let ink = Color32::from_rgb(mix(PAPER.r(), BEDROCK.r()), mix(PAPER.g(), BEDROCK.g()), mix(PAPER.b(), BEDROCK.b()));
    let (light, dark) = (luminance(ink), luminance(BEDROCK));
    assert!((light + 0.05) / (dark + 0.05) >= 4.5);
}

/// What one frame of the card paints, as egui shapes.
fn shapes_of(frame: &CardFrame) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let card = Rect::from_min_size(pos2(16.0, 10.0), vec2(700.0, 400.0));
    let out = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(732.0, 434.0))), ..Default::default() }, |ctx| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| paint_card(ui.painter(), card, None, frame));
    });
    out.shapes.into_iter().map(|clipped| clipped.shape).collect()
}

fn texts(shapes: &[Shape]) -> Vec<String> {
    let mut found = Vec::new();
    for shape in shapes {
        if let Shape::Text(text) = shape {
            found.push(text.galley.text().to_owned());
        }
    }
    found
}

fn has_mesh_of(shapes: &[Shape], colour: Color32) -> bool {
    shapes.iter().any(|shape| matches!(shape, Shape::Mesh(mesh) if mesh.vertices.iter().any(|v| v.color == colour)))
}

/// From the first frame the card shows the name, the status, the bedrock with the two notice lines and the
/// dashed level string; the strata have not stacked yet (no clay, no 기준점). The finished picture has them,
/// and 「EL. 154.00 m」 where the string reached the staff.
#[test]
fn the_first_frame_has_the_name_and_the_notices_and_the_finished_one_the_section() {
    let first = shapes_of(&CardFrame { seconds: 0.0, progress: 0.0, status: "앱을 준비하고 있습니다", version: "2.1", clock: None });
    let words = texts(&first);
    for wanted in ["Strata", "필드고고학 GIS · v2.1", "앱을 준비하고 있습니다", "© 2026 동국문화재연구원 · 권영인 · GNU GPL v2 이상으로 배포"] {
        assert!(words.iter().any(|w| w == wanted), "{wanted} in {words:?}");
    }
    assert!(words.iter().any(|w| w.starts_with("지도·자료  VWorld · 국토정보플랫폼")), "{words:?}");
    assert!(first.iter().any(|shape| matches!(shape, Shape::Rect(rect) if rect.fill == BEDROCK)), "the bedrock band");
    assert!(!has_mesh_of(&first, CLAY) && !words.iter().any(|w| w == "기준점" || w == "EL. 154.00 m" || w == "Ⅲ"));

    let finished = shapes_of(&CardFrame { seconds: 10.0, progress: 1.0, status: "준비 완료", version: "2.1", clock: None });
    let words = texts(&finished);
    assert!(has_mesh_of(&finished, CLAY), "stratum Ⅲ is filled with clay");
    for wanted in ["기준점", "EL. 154.00 m", "Ⅰ", "Ⅴ", "154", "152", "준비 완료"] {
        assert!(words.iter().any(|w| w == wanted), "{wanted} in {words:?}");
    }
}

/// Each frame carries the seconds since the card appeared, so that 「rust」 swings; without motion it has none
/// and the word stands still.
#[test]
fn a_frame_carries_the_clock_of_the_swing_unless_motion_is_off() {
    let mut moving = StartupNotice::new(false);
    moving.mark_ready(0.0);
    assert_eq!(moving.frame(1.25).clock, Some(1.25));
    let mut still = StartupNotice::new(true);
    still.mark_ready(0.0);
    assert_eq!(still.frame(1.25).clock, None);
}
