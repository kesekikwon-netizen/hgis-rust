//! Headless egui tests for order DL1b: `NONE-1` informational phrasing and colour,
//! stopped-count visibility alongside running jobs in the status bar, and single `@gauge=` route.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Color32, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};

use crate::app_state::StrataAppState;
use crate::download_gauge::tok;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::snap_shot::{SnapRequest, SnapStep};
use crate::status_bar_text::gauge_job_text;

struct Dl1bHarness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
}

impl Dl1bHarness {
    fn new() -> Self {
        Self {
            app: StrataAppState::new_startup(),
            ctx: egui::Context::default(),
            time: 1.0,
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
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
        let immediate = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|v| v.repaint_delay.is_zero());
        if immediate { self.frame(vec![]) } else { out }
    }
}

fn find_text_shape<'a>(out: &'a egui::FullOutput, needle: &str) -> Option<&'a egui::epaint::TextShape> {
    out.shapes.iter().find_map(|c| match &c.shape {
        Shape::Text(t) if t.galley.text() == needle => Some(t),
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

fn effective_section_colors(t: &egui::epaint::TextShape) -> Vec<Color32> {
    if let Some(over) = t.override_text_color {
        return vec![over];
    }
    t.galley
        .job
        .sections
        .iter()
        .map(|s| {
            if s.format.color == Color32::PLACEHOLDER {
                t.fallback_color
            } else {
                s.format.color
            }
        })
        .collect()
}

#[test]
fn none1_shows_no_data_info_in_normal_colour_without_failure_wording_in_bar_and_popup() {
    let mut h = Dl1bHarness::new();
    let warn_col = tok(h.app.theme.warn);
    // Verbatim C++ empty-area text (`target/cpp-ref/src/core/DemDownloadService.cpp:207`).
    let cpp_reason = "이 범위에는 제공되는 DEM 자료가 없습니다. 바다를 제외한 조사구역으로 이동하고 다시 내려받으세요.";
    let (id, _guard) = h.app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Stopped {
            code: StopCode::None1,
            reason: cpp_reason.to_owned(),
        },
        h.time,
    );

    let jt = gauge_job_text(h.app.download_jobs.get(id).unwrap(), h.time, 12);
    assert_eq!(jt.bar_label, "DEM · 이 범위에는 자료가 없습니다 · NONE-1");
    assert_eq!(jt.full_label, "DEM · 이 범위에는 자료가 없습니다 · NONE-1");
    assert!(!jt.warning);
    assert_eq!(jt.tooltip.as_deref(), Some(cpp_reason));

    let bar_out = h.frame(vec![]);
    let bar_texts = all_texts(&bar_out);
    assert!(!bar_texts.iter().any(|t| t.contains("받지 못함")), "{bar_texts:?}");
    let bar_shape = find_text_shape(&bar_out, "DEM · 이 범위에는 자료가 없습니다 · NONE-1")
        .expect("NONE-1 informational text in status bar");
    for c in effective_section_colors(bar_shape) {
        assert_ne!(c, warn_col, "NONE-1 bar text must not use warning colour");
    }
    let bar_rect = Rect::from_min_size(bar_shape.pos, bar_shape.galley.size());

    let popup_out = h.click_at(bar_rect.center());
    assert!(h.app.download_jobs.popup_open);
    let popup_texts = all_texts(&popup_out);
    assert!(!popup_texts.iter().any(|t| t.contains("받지 못함")), "{popup_texts:?}");
    assert!(popup_texts.iter().any(|t| t == cpp_reason), "{popup_texts:?}");

    let popup_headers: Vec<&egui::epaint::TextShape> = popup_out
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Text(t) if t.galley.text() == "DEM · 이 범위에는 자료가 없습니다 · NONE-1" => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(popup_headers.len(), 2, "both bar and popup show the NONE-1 header");
    for hdr in popup_headers {
        for c in effective_section_colors(hdr) {
            assert_ne!(c, warn_col, "NONE-1 popup header must not use warning colour");
        }
    }
}

#[test]
fn several_jobs_surface_stopped_count_in_warning_colour_with_and_without_running_jobs() {
    let mut h = Dl1bHarness::new();
    let warn_col = tok(h.app.theme.warn);

    let (run_id, run_h) = h.app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: None,
        },
        h.time,
    );
    let (_stop_id, _stop_h) = h.app.download_jobs.add_job(
        "지적도",
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            reason: "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.".to_owned(),
        },
        h.time,
    );

    // 1) 1 running + 1 stopped (non-NONE-1), without opening the popup:
    let out_both = h.frame(vec![]);
    assert!(!h.app.download_jobs.popup_open);
    let summary = find_text_shape(&out_both, "받는 중 1건 · 멈춤 1건")
        .expect("bar must show running count and stopped count together");
    let cols = effective_section_colors(summary);
    assert_eq!(cols.len(), 2, "two colour sections expected: {cols:?}");
    assert_ne!(cols[0], warn_col, "first part uses normal text colour");
    assert_eq!(cols[1], warn_col, "second part 「멈춤 1건」 uses warning colour");
    assert!(find_text_shape(&out_both, "62%").is_some(), "running job percent stays visible");

    // 2) Nothing left running (DEM finishes, lingering alongside stopped 지적도) -> 「멈춤 1건」 in warn colour.
    run_h.report_done("dem.tif");
    let out_stopped_only = h.frame(vec![]);
    assert_eq!(h.app.download_jobs.len(), 2);
    let stopped_summary = find_text_shape(&out_stopped_only, "멈춤 1건")
        .expect("with nothing left running, bar shows 멈춤 1건");
    assert_eq!(effective_section_colors(stopped_summary), vec![warn_col]);

    // 3) Dismiss Done job so only 1 stopped job remains -> keeps single-job text.
    assert!(h.app.download_jobs.dismiss(run_id));
    let out_single = h.frame(vec![]);
    assert!(find_text_shape(&out_single, "지적도 받지 못함 · ACC-2").is_some());

    // 4) NONE-1 does NOT count as 멈춤 when paired with a running job.
    let mut h2 = Dl1bHarness::new();
    let (_r2, _rh2) = h2.app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Running {
            fraction: Some(0.40),
            stage: None,
        },
        h2.time,
    );
    let _ = h2.app.download_jobs.add_job(
        "토양도",
        DownloadJobState::Stopped {
            code: StopCode::None1,
            reason: "이 범위에는 토양도 데이터가 없습니다.".to_owned(),
        },
        h2.time,
    );
    let out_none1 = h2.frame(vec![]);
    assert!(find_text_shape(&out_none1, "받는 중 1건").is_some());
    assert!(!all_texts(&out_none1).iter().any(|t| t.contains("멈춤")));
}

#[test]
fn snap_apply_action_has_no_duplicate_gauge_route_while_snap_tick_runs_presets() {
    let mut h = Dl1bHarness::new();
    h.app.snap_apply_action("@gauge=running");
    assert!(
        h.app.download_jobs.is_empty(),
        "snap_apply_action must not hold the second unreachable @gauge= route"
    );

    h.app.snap = Some(SnapRequest {
        prefix: std::env::temp_dir().join("strata_dl1b_unit_unused"),
        steps: vec![SnapStep {
            action: Some("@gauge=running".to_owned()),
            name: "g".to_owned(),
        }],
        settle: 0,
        waiting: false,
    });
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
        time: Some(2.0),
        ..Default::default()
    };
    let app = &mut h.app;
    let _ = h.ctx.run(raw, |ctx| {
        app.snap_tick(ctx);
    });
    assert_eq!(h.app.download_jobs.len(), 1);
}
