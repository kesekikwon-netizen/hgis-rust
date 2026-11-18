//! Resized and DPI layouts, target/cpp-ref/src/app/KaBeginnerRibbon.cpp:255-326.
use super::{connected, open, start, visible_texts};
use eframe::egui::{self, PointerButton, Shape};

#[test]
fn context_tools_draw_actions_fit_multiple_sizes_and_dpi() {
    for (size, dpi) in [([480, 480], 1.0), ([640, 480], 1.0), ([800, 600], 2.0), ([1280, 720], 1.0), ([1680, 960], 1.0), ([2560, 1440], 2.0)] {
        let mut h = start(false);
        h.size = size;
        let origin = open(&mut h, "그리기", "기록");
        connected(&mut h, origin);
        let rendered = crate::test_mods::ui_snapshot_support::render_at_dpi(size, dpi, |ctx| {
            h.app.render_workspace_egui(ctx);
        });
        rendered.save_if_requested(&format!("context-draw-{}-{}-dpi{dpi}", size[0], size[1]));
        let clips = rendered.text_clips();
        for action in ["자석", "공유 경계", "허용치", "모든 조사 레이어", "연속 그리기", "폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기", "닫기"] {
            let (_, rect, clip) = clips.iter().find(|(s, _, _)| s == action).expect("rendered action");
            assert!(clip.contains_rect(*rect), "clipped {action} at {size:?} DPI{dpi}");
        }
        let at = h.point_at("닫기");
        h.press(at, PointerButton::Primary);
        assert!(!h.app.draw_row_open());
    }
}

#[test]
fn context_tools_short_window_scroll_reaches_close_without_zooming_map() {
    let mut h = start(false);
    h.size = [480, 360];
    open(&mut h, "그리기", "기록");
    let card = super::flows::card(&mut h, "그리기 ›");
    let zoom = h.app.home_map.viewport.meters_per_pixel;
    h.frame(vec![egui::Event::PointerMoved(card.center())]);
    h.frame(vec![egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -500.0),
        modifiers: Default::default(),
    }]);
    for _ in 0..20 {
        h.frame(vec![]);
    }
    h.frame(vec![egui::Event::PointerMoved(egui::pos2(475.0, 180.0))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h.save("context-draw-short-scrolled");
    let at = h.point_at("닫기");
    h.press(at, PointerButton::Primary);
    assert!(!h.app.draw_row_open());
    assert_eq!(h.app.home_map.viewport.meters_per_pixel, zoom);
}

#[test]
fn context_tools_resize_reanchors_align_to_the_visible_group() {
    let mut h = start(true);
    open(&mut h, "정합", "정합");
    for size in [[800, 600], [1280, 720], [2560, 1440], [640, 480]] {
        h.size = size;
        for _ in 0..3 {
            h.frame(vec![]);
        }
        let output = h.frame(vec![]);
        let origin = visible_texts(&output).into_iter().rev().find(|(s, r)| s == "정합" && r.top() < 100.0).expect("current initiating control").1;
        connected(&mut h, origin);
        let card = super::flows::card(&mut h, "맞추기 ›");
        assert!(h.ctx.screen_rect().contains_rect(card));
        let buttons: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Rect(r) if card.contains_rect(r.rect) && r.rect.height() > 25.0 && r.rect.height() < 45.0 => Some(r.rect),
                _ => None,
            })
            .collect();
        assert!(buttons.len() >= 5);
        // All five action hitboxes must keep the same height after reflow; no pixel/CRS tolerance applies.
        assert!(buttons.iter().all(|r| r.height() == buttons[0].height()));
        h.save(&format!("context-align-{}-{}", size[0], size[1]));
        for action in ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"] {
            h.point_at(action);
        }
    }
    let at = h.point_at("닫기");
    h.press(at, PointerButton::Primary);
    assert!(!h.app.ribbon.sub_toolbar_visible);
}
