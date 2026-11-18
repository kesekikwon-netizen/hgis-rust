//! Actual paper setup input, target/cpp-ref/src/app/KaPaperDialog.cpp:43-198 and MainWindowExport.cpp:204-255.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Shape};

#[path = "paper_setup_input_tests_a.rs"]
mod acceptance;
#[path = "paper_setup_input_tests_b.rs"]
mod lifecycle;

#[derive(PartialEq)]
struct OriginalBytes(Vec<u8>);
impl std::fmt::Debug for OriginalBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalBytes").field("length", &self.0.len()).finish_non_exhaustive()
    }
}

fn start(size: [u32; 2]) -> Harness {
    let directory = tempfile::tempdir().expect("owned temporary survey");
    let file = directory.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = String::new;
    app.paper_look = true;
    assert!(app.open_survey_gpkg(&file));
    app.active_tab = MainTab::Map;
    let mut h = Harness::new(app, directory);
    h.size = size;
    h.ctx.enable_accesskit();
    for _ in 0..4 {
        h.frame(vec![]);
    }
    h
}

fn visible_text(output: &egui::FullOutput, text: &str, screen: Rect) -> Option<Pos2> {
    output.shapes.iter().find_map(|s| match &s.shape {
        Shape::Text(t) if t.galley.text() == text => {
            let rect = t.galley.rect.translate(t.pos.to_vec2());
            (screen.contains_rect(rect) && s.clip_rect.contains_rect(rect)).then_some(rect.center())
        }
        _ => None,
    })
}

fn open(h: &mut Harness) -> Pos2 {
    let output = h.frame(vec![]);
    let source = if let Some(at) = visible_text(&output, "도면", h.ctx.screen_rect()) {
        h.press(at, PointerButton::Primary);
        at
    } else {
        let group = h.point_at("내보내기");
        h.press(group, PointerButton::Primary);
        let action = h.point_at("도면");
        h.press(action, PointerButton::Primary);
        group
    };
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::PaperSetup), "actual drawing action did not open paper setup");
    let output = h.frame(vec![]);
    assert!(visible_text(&output, "용지 설정", h.ctx.screen_rect()).is_some(), "actual paper heading missing");
    source
}

fn control(h: &mut Harness, label: &str) -> Rect {
    let output = h.frame(vec![]);
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("actual accessibility tree")
        .nodes
        .iter()
        .find_map(|(_, n)| {
            if n.label() != Some(label) {
                return None;
            }
            n.bounds().map(|b| Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32)))
        })
        .unwrap_or_else(|| panic!("actual paper control missing: {label}"))
}

fn click(h: &mut Harness, label: &str) {
    let at = control(h, label).center();
    h.press(at, PointerButton::Primary);
}

fn data(h: &Harness) -> impl PartialEq + std::fmt::Debug {
    (
        super::vertex_crs_input_tests::records(h),
        format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables),
        OriginalBytes(std::fs::read(h.app.survey_path.as_ref().expect("owned survey")).expect("survey bytes")),
    )
}

fn work(h: &Harness) -> impl PartialEq + std::fmt::Debug {
    (
        data(h),
        h.app.is_dirty,
        h.app.drawing.sheet.clone(),
        h.app.drawing.sheet_center,
        h.app.open_studio_tabs.clone(),
        format!("{:?}|{:?}|{}", h.app.active_tool, h.app.ribbon.sub_mode, h.app.ribbon.sub_toolbar_visible),
    )
}

fn key(h: &mut Harness, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        h.frame_with_modifiers(
            vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }],
            modifiers,
        );
    }
}

#[test]
fn paper_setup_narrow_controls_are_inside_the_actual_client() {
    for size in [[480, 480], [640, 480]] {
        let mut h = start(size);
        open(&mut h);
        h.save(&format!("paper-narrow-{}", size[0]));
        // One logical point covers edge/stroke rounding; no millimetre or map-CRS tolerance is inferred.
        for label in ["A4", "A3", "직접 입력", "세로", "가로", "용지 만들기", "취소"] {
            let rect = control(&mut h, label);
            assert!(h.ctx.screen_rect().expand(1.0).contains_rect(rect), "paper control is outside the client: {label} {rect:?}");
        }
    }
}

#[test]
fn paper_setup_short_client_keeps_create_and_cancel_reachable() {
    let mut h = start([480, 360]);
    open(&mut h);
    h.save("paper-short");
    for label in ["용지 만들기", "취소"] {
        let rect = control(&mut h, label);
        assert!(h.ctx.screen_rect().expand(1.0).contains_rect(rect), "paper footer action is outside the short client: {label} {rect:?}");
    }
    let before = work(&h);
    click(&mut h, "취소");
    assert!(h.app.modals.active.is_none());
    assert_eq!(work(&h), before);
}

#[test]
fn paper_setup_choices_and_actions_have_consistent_hit_heights() {
    let mut h = start([1680, 960]);
    open(&mut h);
    h.save("paper-wide-before-actions");
    let heights: Vec<_> = ["A4", "A3", "직접 입력", "세로", "가로", "용지 만들기", "취소"].into_iter().map(|label| control(&mut h, label).height()).collect();
    let (low, high) = heights.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), &v| (low.min(v), high.max(v)));
    assert!(low >= 34.0 && high - low <= 1.0, "paper buttons have inconsistent or small hit areas: {heights:?}");
}

#[test]
fn paper_setup_has_a_visible_connection_to_its_real_ribbon_source() {
    let mut h = start([1680, 960]);
    let source = open(&mut h);
    let output = h.frame(vec![]);
    let connected = output.shapes.iter().any(|s| match &s.shape {
        Shape::Path(p) if p.closed && p.points.len() == 3 => p.points.iter().any(|point| (point.x - source.x).abs() <= 16.0 && point.y >= source.y + 6.0 && point.y <= source.y + 48.0),
        _ => false,
    });
    assert!(connected, "paper form has no visible connection to its initiating ribbon control");
}

#[test]
fn paper_setup_owns_background_ribbon_input_without_changing_work() {
    let mut h = start([1680, 960]);
    let measure = h.point_at("측거");
    open(&mut h);
    let before = work(&h);
    h.press(measure, PointerButton::Primary);
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::PaperSetup), "background action replaced the paper dialog");
    assert_eq!(work(&h), before, "paper dialog leaked a background ribbon click");
}

#[test]
fn paper_setup_escape_discards_the_draft_and_preserves_the_sheet() {
    let mut h = start([1680, 960]);
    let before = work(&h);
    open(&mut h);
    click(&mut h, "A3");
    click(&mut h, "가로");
    h.escape();
    assert!(h.app.modals.active.is_none(), "Escape left the paper draft open");
    assert_eq!(work(&h), before, "Escape changed the existing sheet or survey");
}

#[test]
fn paper_setup_ctrl_l_returns_to_an_existing_drawing_without_a_new_draft() {
    let mut h = start([1680, 960]);
    let before = data(&h);
    open(&mut h);
    click(&mut h, "용지 만들기");
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    let map = h.point_at("지도");
    h.press(map, PointerButton::Primary);
    h.app.drawing.sheet.legend_title = "MENU19 retained layout".to_owned();
    h.app.drawing.sheet.scale_denominator = 2500;
    h.app.drawing.sheet_center = Some([222_000.0, 360_000.0]); // Prepared work CRS EPSG:5187, metres.
    let sheet = h.app.drawing.sheet.clone();
    let center = h.app.drawing.sheet_center;
    key(&mut h, Key::L, Modifiers { ctrl: true, ..Default::default() });
    assert!(h.app.modals.active.is_none(), "Ctrl+L asked for a new paper although the drawing tab already exists");
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    assert_eq!(h.app.drawing.sheet, sheet);
    assert_eq!(h.app.drawing.sheet_center, center);
    assert_eq!(data(&h), before);
}

#[test]
fn paper_setup_owns_background_ctrl_d_without_changing_work() {
    let mut h = start([1680, 960]);
    open(&mut h);
    let before = work(&h);
    key(&mut h, Key::D, Modifiers { ctrl: true, ..Default::default() });
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::PaperSetup));
    assert_eq!(work(&h), before, "paper dialog leaked a background application shortcut");
}

#[test]
fn paper_setup_escape_preserves_the_existing_map_tool() {
    let mut h = start([1680, 960]);
    let measure = h.point_at("측거");
    h.press(measure, PointerButton::Primary);
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::Measure);
    let before = work(&h);
    open(&mut h);
    h.escape();
    assert!(h.app.modals.active.is_none());
    assert_eq!(work(&h), before, "paper Escape changed the underlying map tool");
}
