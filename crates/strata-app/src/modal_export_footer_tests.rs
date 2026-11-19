//! 「제출 꾸러미 만들기…」는 실물 꾸러미로 간다(WU-06). 「레이어 5179 변환…」은 아직 흐림.
//! 화면 없는 egui 한 프레임의 accesskit 나무에서 단추 이름과 흐림 여부를 읽는다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;

fn buttons(app: &mut StrataAppState) -> Vec<(String, bool)> {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut last = Vec::new();
    for _ in 0..3 {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))), ..RawInput::default() };
        let out = ctx.run(raw, |ctx| {
            let mut open = true;
            crate::modal_export::render_export_submit_package(app, ctx, &mut open);
        });
        if let Some(update) = out.platform_output.accesskit_update {
            last = update.nodes.iter().filter_map(|(_, n)| n.label().map(|l| (l.to_owned(), n.is_disabled()))).collect();
        }
    }
    last
}

#[test]
fn submit_button_is_enabled_and_convert_stays_off() {
    let mut app = StrataAppState::new_startup();
    let found = buttons(&mut app);
    let state = |name: &str| found.iter().find(|(l, _)| l == name).map(|(_, d)| *d);
    assert_eq!(state("제출 꾸러미 만들기…"), Some(false), "{found:?}");
    assert_eq!(state("레이어 5179 변환…"), Some(true), "{found:?}");
    assert_eq!(state("닫기"), Some(false), "{found:?}");
}
