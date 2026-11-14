//! Headless egui tests for the bottom status bar download gauge and popup
//! (`docs/RUST_PLAN.md` §2.8; 4 px slim bar precedent in `target/cpp-ref/src/app/KaProgressPlate.cpp:15-24`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};

use crate::app_state::{MainTab, StrataAppState};
use crate::download_jobs::{DownloadJobState, StopCode};

struct BarHarness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    size: egui::Vec2,
}

impl BarHarness {
    fn new_1280() -> Self {
        Self {
            app: StrataAppState::new_startup(),
            ctx: egui::Context::default(),
            time: 1.0,
            size: egui::vec2(1280.0, 900.0),
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        })
    }

    fn click_at(&mut self, pos: Pos2) -> egui::FullOutput {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::default(),
        }]);
        let out = self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::default(),
        }]);
        let immediate_repaint = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|v| v.repaint_delay.is_zero());
        if immediate_repaint {
            self.frame(vec![])
        } else {
            out
        }
    }
}

fn find_text_rect(out: &egui::FullOutput, needle: &str) -> Option<Rect> {
    out.shapes.iter().find_map(|c| match &c.shape {
        Shape::Text(t) if t.galley.text() == needle => Some(
            Rect::from_min_size(t.pos, t.galley.size()),
        ),
        _ => None,
    })
}

fn all_texts(out: &egui::FullOutput) -> Vec<String> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn find_cancel_rects(out: &egui::FullOutput) -> Vec<Rect> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::LineSegment { points: [a, b], .. }
                if (b.x - a.x - 7.0).abs() < 0.2 && (b.y - a.y - 7.0).abs() < 0.2 =>
            {
                Some(Rect::from_two_pos(*a, *b))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn empty_list_matches_baseline_and_long_job_keeps_right_cluster_fixed_at_1280px() {
    let mut h = BarHarness::new_1280();
    let home_empty = h.frame(vec![]);
    let home_texts = all_texts(&home_empty);
    assert!(home_texts.iter().any(|t| t == "먼저 「새 조사」로 오늘 현장을 만드세요."));
    assert!(!home_texts.iter().any(|t| t.contains("받는 중")));
    assert!(find_cancel_rects(&home_empty).is_empty());

    // Switch to Map tab with unsaved flag so the right-hand cluster is maximally populated.
    h.app.active_tab = MainTab::Map;
    h.app.survey_path = Some(std::path::PathBuf::from("sample.gpkg"));
    h.app.is_dirty = true;
    let map_empty = h.frame(vec![]);
    let base_render = find_text_rect(&map_empty, "지도갱신 켜짐").expect("render toggle");
    let base_submit = find_text_rect(&map_empty, "→ 제출 5179").expect("submit chip");
    let base_unsaved = find_text_rect(&map_empty, "저장 안 됨").expect("unsaved chip");
    let base_central_h = h.app.home_map.viewport.screen_size_px[1];

    // Add a job with a very long title and a very long left status message at 1280 px width.
    h.app.status_bar_hint =
        "도형을 넣었습니다 (조사구역, 1개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다".to_owned();
    let _ = h.app.download_jobs.add_job(
        "국가유산청주변유적장거리공간자료전체목록내려받기",
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: Some("압축 풀고 레이어 확인하는 중".to_owned()),
        },
        h.time,
    );
    let map_with_gauge = h.frame(vec![]);
    let after_render = find_text_rect(&map_with_gauge, "지도갱신 켜짐").expect("render toggle");
    let after_submit = find_text_rect(&map_with_gauge, "→ 제출 5179").expect("submit chip");
    let after_unsaved = find_text_rect(&map_with_gauge, "저장 안 됨").expect("unsaved chip");
    let after_central_h = h.app.home_map.viewport.screen_size_px[1];

    // Bar height did not grow (central panel height unchanged) and right-hand cluster did not move.
    assert!((after_central_h - base_central_h).abs() < 0.1, "{after_central_h} vs {base_central_h}");
    assert!((after_render.min.x - base_render.min.x).abs() < 0.5, "{after_render:?} vs {base_render:?}");
    assert!((after_submit.min.x - base_submit.min.x).abs() < 0.5, "{after_submit:?} vs {base_submit:?}");
    assert!((after_unsaved.min.x - base_unsaved.min.x).abs() < 0.5, "{after_unsaved:?} vs {base_unsaved:?}");
    assert!(after_render.max.x <= 1280.0);

    let pct_rect = find_text_rect(&map_with_gauge, "62%").expect("gauge percent visible at 1280px");
    assert!(pct_rect.max.x < after_unsaved.min.x, "gauge stays left of right-hand cluster");
}

#[test]
fn running_job_shows_title_4px_bar_62_percent_and_cancel_sets_flag_only() {
    let mut h = BarHarness::new_1280();
    let (id, _) = h.app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: None,
        },
        h.time,
    );
    let out = h.frame(vec![]);
    let home_msg = find_text_rect(&out, "먼저 「새 조사」로 오늘 현장을 만드세요.").expect("home msg");
    let title_rect = find_text_rect(&out, "DEM 받는 중").expect("running title on home tab");
    let pct_rect = find_text_rect(&out, "62%").expect("62% label");
    let cancel_rect = *find_cancel_rects(&out).first().expect("cancel button");
    assert!(title_rect.min.x > home_msg.max.x, "gauge painted directly after left message");

    // Verify 4 px high, ~120 px wide track rect is painted in the bottom bar.
    let has_slim_track = out.shapes.iter().any(|c| match &c.shape {
        Shape::Rect(r) => {
            r.rect.top() >= 860.0
                && (r.rect.height() - 4.0).abs() < 0.5
                && (r.rect.width() - 120.0).abs() < 2.0
        }
        _ => false,
    });
    assert!(has_slim_track, "4 px x 120 px track rect expected");
    assert!(pct_rect.min.x > title_rect.max.x);
    assert!(cancel_rect.min.x > pct_rect.max.x);

    // Clicking ✕ sets only the cancel flag; state remains Running until worker reports Cancelled.
    let _ = h.click_at(cancel_rect.center());
    assert!(h.app.download_jobs.get(id).unwrap().is_cancel_requested());
    assert!(matches!(
        h.app.download_jobs.get(id).unwrap().state,
        DownloadJobState::Running { .. }
    ));

    // Without a fraction: moving segment is painted and no percent text appears.
    h.app.download_jobs.set_state(
        id,
        DownloadJobState::Running {
            fraction: None,
            stage: None,
        },
        h.time,
    );
    let out_indet = h.frame(vec![]);
    assert!(!all_texts(&out_indet).iter().any(|t| t.ends_with('%')));
}

#[test]
fn stopped_job_shows_code_opens_popup_with_full_reason_and_close_dismisses() {
    let mut h = BarHarness::new_1280();
    let full_reason = "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.";
    h.app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            reason: full_reason.to_owned(),
        },
        h.time,
    );
    let out = h.frame(vec![]);
    let stop_rect = find_text_rect(&out, "DEM 받지 못함 · ACC-2").expect("stopped code shown");
    assert!(find_cancel_rects(&out).is_empty(), "Stopped job has no cancel ✕");

    // Clicking the Stopped gauge opens the popup above the bar with the full reason and 「닫기」.
    let popup_out = h.click_at(stop_rect.center());
    assert!(h.app.download_jobs.popup_open);
    let texts = all_texts(&popup_out);
    assert!(texts.iter().any(|t| t.contains(full_reason)), "popup texts: {texts:?}");
    let close_rect = find_text_rect(&popup_out, "닫기").expect("닫기 button in popup");
    assert!(close_rect.max.y < stop_rect.min.y, "popup sits above the status bar");

    // Clicking 「닫기」 dismisses the Stopped job.
    let after_close = h.click_at(close_rect.center());
    assert!(h.app.download_jobs.is_empty());
    assert!(!all_texts(&after_close).iter().any(|t| t.contains("ACC-2")));
}

#[test]
fn two_jobs_show_count_and_popup_lists_each_job_with_cancel() {
    let mut h = BarHarness::new_1280();
    h.app.download_jobs.apply_snap_preset("two", h.time);
    assert_eq!(h.app.download_jobs.len(), 2);
    let second_id = h.app.download_jobs.jobs()[1].id;

    let out = h.frame(vec![]);
    let summary_rect = find_text_rect(&out, "받는 중 2건").expect("two jobs count shown");
    assert!(find_text_rect(&out, "62%").is_some(), "first unfinished job fraction shown");

    // Clicking the gauge opens the popup listing every job with its own text, bar, and ✕.
    let popup_out = h.click_at(summary_rect.center());
    let texts = all_texts(&popup_out);
    assert!(texts.iter().any(|t| t == "DEM 받는 중"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "지적도 기다리는 중"), "{texts:?}");

    let cancel_buttons: Vec<Rect> = find_cancel_rects(&popup_out)
        .into_iter()
        .filter(|r| r.top() < 860.0)
        .collect();
    assert_eq!(cancel_buttons.len(), 2, "each unfinished job in popup has its own ✕");

    // Clicking the second job's ✕ cancels only the second job.
    let _ = h.click_at(cancel_buttons[1].center());
    assert!(!h.app.download_jobs.jobs()[0].is_cancel_requested());
    assert!(h.app.download_jobs.get(second_id).unwrap().is_cancel_requested());
}
