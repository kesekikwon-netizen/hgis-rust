//! Actual acceptance, target/cpp-ref/src/app/KaPaperDialog.cpp:86-179,191-198.
use super::*;

fn numbers(h: &mut Harness) -> Vec<(f64, bool, Rect)> {
    let output = h.frame(vec![]);
    let mut result: Vec<_> = output
        .platform_output
        .accesskit_update
        .expect("accessibility")
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let value = n.numeric_value()?;
            let b = n.bounds()?;
            Some((value, n.is_disabled(), Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))))
        })
        .collect();
    result.sort_by(|a, b| a.2.top().total_cmp(&b.2.top()));
    result
}

fn type_mm(h: &mut Harness, index: usize, text: &str) {
    let fields = numbers(h);
    assert_eq!(fields.len(), 2, "two actual paper fields");
    h.press(fields[index].2.center(), PointerButton::Primary);
    key(
        h,
        Key::A,
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    );
    h.frame(vec![Event::Text(text.to_owned())]);
    key(h, Key::Enter, Modifiers::NONE);
}

#[test]
fn paper_setup_accepts_all_cpp_presets_without_writing_the_survey() {
    // Exact dimensions from KaPaperDialog.cpp:161-179; no spatial approximation.
    for (a3, landscape, expected) in [(false, false, (210.0, 297.0)), (false, true, (317.0, 220.0)), (true, false, (297.0, 420.0)), (true, true, (420.0, 297.0))] {
        let mut h = start([1680, 960]);
        let before = data(&h);
        open(&mut h);
        if a3 {
            click(&mut h, "A3");
        }
        if landscape {
            click(&mut h, "가로");
        }
        assert_eq!(data(&h), before);
        click(&mut h, "용지 만들기");
        assert!(h.app.modals.active.is_none());
        assert_eq!(h.app.active_tab, MainTab::Drawing);
        assert_eq!((h.app.drawing.sheet.width_mm, h.app.drawing.sheet.height_mm), expected);
        assert!(h.app.drawing.sheet.all_decorations_inside_sheet());
        assert_eq!(data(&h), before);
    }
}

#[test]
fn paper_setup_custom_dimensions_survive_orientation_and_accept() {
    let mut h = start([1680, 960]);
    let before = data(&h);
    open(&mut h);
    click(&mut h, "직접 입력");
    type_mm(&mut h, 0, "123.4");
    type_mm(&mut h, 1, "456.7");
    click(&mut h, "가로");
    let fields = numbers(&mut h);
    assert_eq!(fields.iter().map(|n| n.0).collect::<Vec<_>>(), [123.4, 456.7]);
    assert!(fields.iter().all(|n| !n.1));
    h.save("paper-custom-edited");
    assert_eq!(data(&h), before);
    click(&mut h, "용지 만들기");
    assert!(h.app.modals.active.is_none());
    // One decimal in the C++ editor; 1e-9 mm only allows floating-point representation error.
    assert!((h.app.drawing.sheet.width_mm - 123.4).abs() < 1e-9);
    assert!((h.app.drawing.sheet.height_mm - 456.7).abs() < 1e-9);
    assert_eq!(data(&h), before);
}

#[test]
fn paper_setup_custom_editor_retains_cpp_limits() {
    let mut h = start([1680, 960]);
    open(&mut h);
    click(&mut h, "직접 입력");
    type_mm(&mut h, 0, "1");
    type_mm(&mut h, 1, "3000");
    assert_eq!(numbers(&mut h).iter().map(|n| n.0).collect::<Vec<_>>(), [50.0, 2000.0]);
    click(&mut h, "용지 만들기");
    assert_eq!((h.app.drawing.sheet.width_mm, h.app.drawing.sheet.height_mm), (50.0, 2000.0));
}

#[test]
fn paper_setup_stock_presets_disable_only_the_numeric_fields() {
    let mut h = start([640, 480]);
    h.app.paper_look = false;
    open(&mut h);
    let fields = numbers(&mut h);
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().all(|n| n.1));
    assert_eq!(fields.iter().map(|n| n.0).collect::<Vec<_>>(), [210.0, 297.0]);
    click(&mut h, "직접 입력");
    assert!(numbers(&mut h).iter().all(|n| !n.1));
    h.save("paper-stock-custom");
}

#[test]
fn paper_setup_short_body_scroll_reaches_custom_fields_without_zooming_map() {
    let mut h = start([480, 360]);
    open(&mut h);
    click(&mut h, "직접 입력");
    let zoom = h.app.home_map.viewport.meters_per_pixel;
    let card = super::lifecycle::card(&h);
    h.frame(vec![Event::PointerMoved(card.center())]);
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -500.0),
        modifiers: Modifiers::NONE,
    }]);
    for _ in 0..20 {
        h.frame(vec![]);
    }
    let fields = numbers(&mut h);
    assert_eq!(fields.len(), 2);
    let footer = control(&mut h, "용지 만들기");
    assert!(
        h.ctx.screen_rect().contains_rect(fields[1].2) && fields[1].2.bottom() < footer.top(),
        "custom height field is not reachable above the fixed footer"
    );
    type_mm(&mut h, 1, "654.3");
    assert_eq!(numbers(&mut h)[1].0, 654.3);
    assert_eq!(h.app.home_map.viewport.meters_per_pixel, zoom);
    h.save("paper-short-custom-scrolled");
    click(&mut h, "취소");
    assert!(h.app.modals.active.is_none());
}
