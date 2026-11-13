//! F1 — 「주변유적 받기」 창에 지어낸 유적이 보이지 않는다(화면 없는 egui 한 프레임, 그린 글을 모은다).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;

fn texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.text().to_owned()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// 받기 전 창의 글 전부(여러 프레임 — 자리 잡기).
fn window_texts(app: &mut StrataAppState) -> String {
    let ctx = egui::Context::default();
    let mut all = Vec::new();
    for _ in 0..3 {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))), ..RawInput::default() };
        let out = ctx.run(raw, |ctx| {
            let mut open = true;
            super::render_heritage_browser(app, ctx, &mut open);
        });
        all.clear();
        out.shapes.iter().for_each(|c| texts(&c.shape, &mut all));
    }
    all.join("\n")
}

/// 받은 자료가 없으면 C++ 처럼 단계 「대기」만 — 견본 유적 이름·지정번호·「검색 결과 N건」은 없다.
#[test]
fn no_invented_sites_before_any_download() {
    let mut app = StrataAppState::new_startup();
    let text = window_texts(&mut app);
    assert!(text.contains("주변유적 다운로드"), "{text}");
    assert!(text.contains("대기"), "{text}");
    for invented in ["경주", "월성", "첨성대", "황룡사지", "고분군", "인왕동", "사적 제", "국보 제", "기념물 제", "검색 결과", "위치 보기", "반경 1 km"] {
        assert!(!text.contains(invented), "{invented:?} is on screen:\n{text}");
    }
}
