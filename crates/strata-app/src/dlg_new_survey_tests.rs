//! Tests for the new-survey dialog origin suggestion, longitude ruler mark, and name error
//! (`target/cpp-ref/src/app/KaNewSurveyDialog.cpp:124-135,165-179,205-212`,
//! `target/cpp-ref/src/app/KaLongitudeRuler.cpp:14-16,99-107`,
//! `target/cpp-ref/tests/test_home.cpp:88-124`).

use eframe::egui::{self, Color32, Pos2, RawInput, Rect, Shape};
use strata_canvas2d::viewport::ViewportTransform;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;

const CLAY_MARK: Color32 = Color32::from_rgb(181, 87, 58);

struct FrameScan {
    texts: Vec<String>,
    buttons: Vec<(String, bool)>,
    clay_lines: usize,
    clay_polygons: usize,
}

fn collect_shapes(
    shape: &Shape,
    texts: &mut Vec<String>,
    clay_lines: &mut usize,
    clay_polygons: &mut usize,
) {
    match shape {
        Shape::Text(t) => texts.push(t.galley.job.text.clone()),
        Shape::LineSegment { stroke, .. } => {
            if stroke.color == CLAY_MARK {
                *clay_lines += 1;
            }
        }
        Shape::Path(path) => {
            if path.fill == CLAY_MARK && path.points.len() == 3 {
                *clay_polygons += 1;
            }
        }
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_shapes(s, texts, clay_lines, clay_polygons);
            }
        }
        _ => {}
    }
}

fn run_dialog(app: &mut StrataAppState) -> FrameScan {
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    let mut texts = Vec::new();
    let mut buttons = Vec::new();
    let mut clay_lines = 0;
    let mut clay_polygons = 0;
    for step in 0..3 {
        if step == 2 {
            ctx.enable_accesskit();
        }
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0))),
            time: Some(f64::from(step)),
            ..RawInput::default()
        };
        let out = ctx.run(raw, |ctx| {
            let mut open = true;
            super::render_new_survey(app, ctx, &mut open);
        });
        texts.clear();
        clay_lines = 0;
        clay_polygons = 0;
        for clipped in &out.shapes {
            collect_shapes(&clipped.shape, &mut texts, &mut clay_lines, &mut clay_polygons);
        }
        if let Some(update) = out.platform_output.accesskit_update {
            buttons = update
                .nodes
                .iter()
                .filter_map(|(_, n)| n.label().map(|l| (l.to_owned(), n.is_disabled())))
                .collect();
        }
    }
    FrameScan {
        texts,
        buttons,
        clay_lines,
        clay_polygons,
    }
}

fn next_button_disabled(scan: &FrameScan) -> Option<bool> {
    scan.buttons
        .iter()
        .find(|(label, _)| label == "다음")
        .map(|(_, disabled)| *disabled)
}

#[test]
fn suggested_work_crs_follows_tm_zones() {
    // Ported from target/cpp-ref/tests/test_home.cpp:88-96.
    assert_eq!(
        super::suggested_work_crs(Some(126.98)),
        Some(WorkCrs::Epsg5186)
    );
    assert_eq!(
        super::suggested_work_crs(Some(126.53)),
        Some(WorkCrs::Epsg5186)
    );
    assert_eq!(
        super::suggested_work_crs(Some(128.73)),
        Some(WorkCrs::Epsg5187)
    );
    assert_eq!(
        super::suggested_work_crs(Some(129.36)),
        Some(WorkCrs::Epsg5187)
    );
    assert_eq!(super::suggested_work_crs(Some(125.0)), None);
    assert_eq!(super::suggested_work_crs(Some(130.9)), None);
    assert_eq!(super::suggested_work_crs(Some(f64::NAN)), None);
    assert_eq!(super::suggested_work_crs(None), None);
}

#[test]
fn dialog_hides_recommend_sentence_and_ruler_mark_when_no_suggestion() {
    // target/cpp-ref/src/app/KaNewSurveyDialog.cpp:96,124-135,172 and
    // target/cpp-ref/src/app/KaLongitudeRuler.cpp:99-107,
    // target/cpp-ref/tests/test_home.cpp:133-135.
    let mut app = StrataAppState::new_startup();
    let scan = run_dialog(&mut app);

    assert!(
        !scan.texts.iter().any(|t| t.contains("추천")),
        "expected no recommendation badge when map longitude is unknown: {:?}",
        scan.texts
    );
    assert!(
        !scan.texts.iter().any(|t| t.starts_with("지금 지도 중심(")),
        "expected no suggestion sentence when map longitude is unknown: {:?}",
        scan.texts
    );
    assert_eq!(
        scan.texts
            .iter()
            .map(String::as_str)
            .filter(|t| t.ends_with('°'))
            .collect::<Vec<_>>(),
        vec!["126°", "127°", "128°", "129°", "130°"],
        "ruler must draw only whole-degree labels and no centre-mark degree text: {:?}",
        scan.texts
    );
    assert_eq!(
        (scan.clay_lines, scan.clay_polygons),
        (0, 0),
        "expected no centre line or triangle mark on ruler when map longitude is unknown"
    );

    // Off-ruler longitude (130.9°E in EPSG:5187) also draws no mark and no suggestion.
    app.work_crs = WorkCrs::Epsg5187;
    app.home_map.viewport =
        ViewportTransform::new([368_016.392_127, 546_200.375_630], 1.0, [1000.0, 600.0]);
    let off_ruler = run_dialog(&mut app);
    assert!(!off_ruler.texts.iter().any(|t| t.contains("추천")));
    assert!(!off_ruler.texts.iter().any(|t| t.starts_with("지금 지도 중심(")));
    assert_eq!((off_ruler.clay_lines, off_ruler.clay_polygons), (0, 0));
}

#[test]
fn dialog_keeps_default_origin_and_only_suggests() {
    // Ported from target/cpp-ref/tests/test_home.cpp:100-124.
    // Seoul (126.98°E, 37.57°N) in EPSG:5187 coordinates while 5187 remains selected.
    let mut app = StrataAppState::new_startup();
    app.work_crs = WorkCrs::Epsg5187;
    app.modals.new_survey_crs = WorkCrs::Epsg5187;
    app.home_map.viewport =
        ViewportTransform::new([21_537.904_716, 554_191.863_901], 1.0, [1000.0, 600.0]);

    let scan = run_dialog(&mut app);

    assert_eq!(
        app.modals.new_survey_crs,
        WorkCrs::Epsg5187,
        "selected origin must never be switched by the suggestion"
    );
    assert!(
        scan.texts
            .iter()
            .any(|t| t.starts_with("5186 중부원점") && t.contains("추천") && !t.contains("[추천]")),
        "expected unbracketed 추천 on 5186 button: {:?}",
        scan.texts
    );
    assert!(
        !scan
            .texts
            .iter()
            .any(|t| t.starts_with("5187 동부원점") && t.contains("추천")),
        "5187 button must not carry 추천 when 5186 is suggested: {:?}",
        scan.texts
    );
    assert!(
        scan.texts
            .iter()
            .any(|t| t == "지금 지도 중심(동경 127.0°)은 중부원점(5186) 구역입니다."),
        "expected formatted suggestion sentence: {:?}",
        scan.texts
    );
    assert_eq!(
        (scan.clay_lines, scan.clay_polygons),
        (1, 1),
        "expected one centre line and one triangle mark on ruler for 126.98°E"
    );
    assert_eq!(
        scan.texts
            .iter()
            .map(String::as_str)
            .filter(|t| t.ends_with('°'))
            .collect::<Vec<_>>(),
        vec!["126°", "127°", "128°", "129°", "130°"],
        "centre mark on ruler must carry no degree text: {:?}",
        scan.texts
    );
}

#[test]
fn dialog_name_validation_hides_empty_error_and_controls_next_button() {
    // Ported from target/cpp-ref/tests/test_home.cpp:111-124.
    let mut app = StrataAppState::new_startup();

    // Empty name: no error text, 「다음」 disabled.
    app.modals.new_survey_name.clear();
    let empty_scan = run_dialog(&mut app);
    assert!(
        !empty_scan.texts.iter().any(|t| t == "조사명을 입력하세요."),
        "empty field must hide name error text: {:?}",
        empty_scan.texts
    );
    assert_eq!(
        next_button_disabled(&empty_scan),
        Some(true),
        "expected 「다음」 disabled for empty name: {:?}",
        empty_scan.buttons
    );

    // Reserved device name 「NUL」: error contains 「예약」, 「다음」 disabled.
    app.modals.new_survey_name = "NUL".to_owned();
    let nul_scan = run_dialog(&mut app);
    assert!(
        nul_scan.texts.iter().any(|t| t.contains("예약")),
        "expected reserved-name error containing 예약: {:?}",
        nul_scan.texts
    );
    assert_eq!(
        next_button_disabled(&nul_scan),
        Some(true),
        "expected 「다음」 disabled for NUL: {:?}",
        nul_scan.buttons
    );

    // Usable name 「병산동」: no error text, 「다음」 enabled.
    app.modals.new_survey_name = "병산동".to_owned();
    let valid_scan = run_dialog(&mut app);
    assert!(
        !valid_scan
            .texts
            .iter()
            .any(|t| t == "조사명을 입력하세요." || t.contains("예약")),
        "expected no validation error for 병산동: {:?}",
        valid_scan.texts
    );
    assert_eq!(
        next_button_disabled(&valid_scan),
        Some(false),
        "expected 「다음」 enabled for 병산동: {:?}",
        valid_scan.buttons
    );
}
