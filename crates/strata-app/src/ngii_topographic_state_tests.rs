//! NET0b — 「도엽 확인」 in 수치지형도 내려받기 is disabled with the WB2 tooltip until the embedded browser view
//! lands (target/cpp-ref/src/app/KaTopographicBrowser.cpp:509-537, data/providers/ngii-topographic.js);
//! clicking it sends no request and changes no window state.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_net::ngii_topographic::NgiiTopographicClient;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

fn collect_texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.text().to_owned()),
        Shape::Vec(v) => v.iter().for_each(|s| collect_texts(s, out)),
        _ => {}
    }
}

fn window_frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
    events: Vec<Event>,
) -> (Vec<(String, bool, Option<Rect>)>, String) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        crate::dlg_ngii_topo_download::render_ngii_topo_download(app, ctx, open);
    });
    let mut texts = Vec::new();
    out.shapes.iter().for_each(|c| collect_texts(&c.shape, &mut texts));
    let nodes = out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
        update
            .nodes
            .iter()
            .filter_map(|(_, n)| {
                let bounds = n.bounds().map(|b| {
                    Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
                });
                n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
            })
            .collect()
    });
    (nodes, texts.join("\n"))
}

#[test]
fn sheet_lookup_button_is_deferred_wb2() {
    let mut app = StrataAppState::new_startup();
    app.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
    let before_stage = app.modals.topo_browser_stage;
    let before_log = app.modals.topo_browser_log.clone();

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.show_tooltips_only_when_still = false;
    });
    let mut open = true;
    let mut nodes = Vec::new();
    let mut painted = String::new();
    for _ in 0..3 {
        (nodes, painted) = window_frame(&mut app, &ctx, &mut open, vec![]);
    }
    let (_, disabled, bounds) = nodes
        .iter()
        .find(|(l, _, _)| l == "도엽 확인")
        .cloned()
        .unwrap_or_else(|| panic!("sheet lookup button missing: {nodes:?}"));
    assert!(disabled, "도엽 확인 must be disabled until WB2: {nodes:?}");
    for enabled_btn in ["상세 보기", "다음 단계", "취소"] {
        assert_eq!(
            nodes.iter().find(|(l, _, _)| l == enabled_btn).map(|(_, d, _)| *d),
            Some(false),
            "{enabled_btn} must stay enabled"
        );
    }
    assert!(
        painted.contains("「도엽 확인」을 누르면 조사 주변 5km의 1:25,000 DXF 도엽을 국토정보플랫폼에 묻습니다."),
        "idle status text must remain:\n{painted}"
    );
    assert!(
        painted.contains(NgiiTopographicClient::login_stage_blocked_message()),
        "browser notice must remain:\n{painted}"
    );

    let at = bounds.expect("bounds").center();
    let (_, hover_text) = window_frame(&mut app, &ctx, &mut open, vec![Event::PointerMoved(at)]);
    let (_, hover_text2) = window_frame(&mut app, &ctx, &mut open, vec![Event::PointerMoved(at)]);
    let combined = format!("{hover_text}\n{hover_text2}");
    assert!(
        combined.contains("아직 Rust 판에서 연결하지 않았습니다 (지시 WB2)"),
        "expected WB2 tooltip on hover:\n{combined}"
    );

    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    window_frame(&mut app, &ctx, &mut open, vec![press(true)]);
    let (_, after_text) = window_frame(&mut app, &ctx, &mut open, vec![press(false)]);

    assert!(open);
    assert_eq!(app.modals.active, Some(ActiveModalDialog::NgiiTopoDownload));
    assert_eq!(app.modals.topo_browser_stage, before_stage);
    assert_eq!(app.modals.topo_browser_log, before_log);
    assert!(
        after_text.contains("「도엽 확인」을 누르면 조사 주변 5km의 1:25,000 DXF 도엽을 국토정보플랫폼에 묻습니다."),
        "status text must stay unchanged after click:\n{after_text}"
    );
    assert!(
        !after_text.contains("국토정보플랫폼에 1:25,000 DXF 도엽을 묻고 있습니다."),
        "must not enter running lookup state:\n{after_text}"
    );
}
