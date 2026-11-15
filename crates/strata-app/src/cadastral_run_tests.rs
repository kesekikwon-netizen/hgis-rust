//! The 지적 button from the press to the legend row, on a run made here (nothing is asked of any site):
//! `target/cpp-ref/src/app/MainWindowCadastral.cpp:82-175`, and the owner's rules of 2026-11-15 (no window,
//! the bottom gauge, only the reason on a failure).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use eframe::egui;
use strata_refmap::cadastral::portal::{PortalStop, Why};
use strata_refmap::cadastral::Ready;
use strata_types::schema::LegendGroup;

use super::*;
use crate::ref_vector::data::{Look, PolygonFill};
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::map::tests::{draw_survey_area, CENTRE};

pub(super) struct Bench {
    pub(super) app: StrataAppState,
    dir: tempfile::TempDir,
    runs: Arc<AtomicUsize>,
    ctx: egui::Context,
}

/// An app with a saved survey in a temp folder and a drawn survey area, a stored key and account, and a
/// run that ends as `outcome` says.
pub(super) fn bench(outcome: fn(&Wanted) -> Result<Ready, PortalStop>) -> Bench {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    draw_survey_area(&mut app, CENTRE, 5187);
    let runs = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&runs);
    app.cadastral = CadastralJobs::for_test(Arc::new(move |wanted, _, stage| {
        counted.fetch_add(1, Ordering::SeqCst);
        stage(8, "VWorld 계정을 확인하고 있습니다…");
        outcome(wanted)
    }));
    Bench { app, dir, runs, ctx: egui::Context::default() }
}

impl Bench {
    pub(super) fn frame(&mut self, now: f64) {
        self.app.drive_cadastral(&self.ctx, now);
        self.app.download_jobs.tick(now);
    }
    pub(super) fn status(&self) -> &str {
        self.app.home_map.draw_guide_message.as_deref().unwrap_or_default()
    }
    fn row(&self) -> Option<usize> {
        self.app.left_layers.entries.iter().position(|e| e.layer_key == CADASTRAL_KEY)
    }
    fn job(&self) -> Option<&DownloadJobState> {
        self.app.download_jobs.jobs().last().map(|job| &job.state)
    }
}

pub(super) fn table(wanted: &Wanted) -> Result<Ready, PortalStop> {
    let file = wanted.directory.join("표시").join("made.gpkg");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"table").unwrap();
    Ok(Ready { file, parcels: 3, reused: false })
}

/// A press starts the run for the drawn survey areas, 5 km around them, into `<survey folder>/지적도`, with
/// the stored key and account; no window opens; when the table is there its row stands at the top of the
/// legend, outside 주변·참조, with outlines of 0.2 mm and lot numbers from 1:10,000 in
/// (`MainWindowCadastral.cpp:82-175`, `CadastralImport.cpp:244-294`, `LayerOps.cpp:1091-1101`).
#[test]
fn a_press_receives_the_parcels_and_puts_their_row_on_top() {
    let mut bench = bench(|wanted| {
        assert_eq!((wanted.work_epsg, wanted.key.as_str(), wanted.username.as_str(), wanted.password.as_str()), (5187, "KEY", "field", "secret"));
        assert_eq!(wanted.areas.len(), 1);
        assert!(wanted.directory.ends_with("지적도"));
        table(wanted)
    });
    bench.app.press_cadastral();
    assert_eq!(bench.status(), "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.");
    assert!(bench.app.modals.active.is_none(), "no window");
    bench.frame(1.0);
    bench.frame(1.1);

    assert_eq!(bench.runs.load(Ordering::SeqCst), 1);
    assert_eq!(bench.row(), Some(0));
    let row = &bench.app.left_layers.entries[0];
    assert_eq!((row.label_ko.as_str(), row.group, row.read_only), ("지적도 · 조사 주변 5km", LegendGroup::Cadastral, true));
    assert!(bench.app.home_map.real_survey.is_layer_visible("지적도 · 조사 주변 5km"));
    let spec = bench.app.ref_vectors.spec_of(CADASTRAL_KEY).unwrap();
    assert_eq!(spec.table, "cadastral");
    assert_eq!(spec.look, Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.2, fill: PolygonFill::None });
    let label = spec.label.unwrap();
    assert_eq!((label.column, label.size_pt, label.ink, label.bold, label.closest_scale), ("JIBUN", 8.0, [0, 0, 0], false, Some(10_000.0)));
    let said = "조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다.";
    assert_eq!(bench.status(), said);
    assert!(matches!(bench.job(), Some(DownloadJobState::Done { result }) if result == said));
    assert_eq!(bench.app.download_jobs.jobs().last().unwrap().title, "조사 주변 지적도");
}

/// What a run cannot start without is said on the status line, and what the user cannot mend on the map
/// also goes to the gauge with its code (`:83-120`); a missing account opens the account entry.
#[test]
fn a_press_that_cannot_start_says_why() {
    let mut none = bench(table);
    none.app.survey_path = None;
    none.app.press_cadastral();
    assert_eq!(none.status(), "먼저 조사를 열거나 저장하고 조사구역을 그려 주세요.");

    let mut bare = bench(table);
    bare.app.survey_tables.clear();
    bare.app.press_cadastral();
    assert_eq!(bare.status(), "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다.");

    let mut keyless = bench(table);
    keyless.app.cadastral.set_key(String::new);
    keyless.app.press_cadastral();
    keyless.frame(1.0);
    let words = "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요.";
    assert_eq!(keyless.status(), words);
    assert!(matches!(keyless.job(), Some(DownloadJobState::Stopped { code: StopCode::Key1, reason }) if reason == words));

    let mut nameless = bench(table);
    nameless.app.cadastral.set_account(|| (String::new(), String::new()));
    nameless.app.press_cadastral();
    assert_eq!(nameless.status(), "지적도 메뉴에서 VWorld 계정을 설정하세요.");
    assert_eq!(nameless.app.modals.active, Some(ActiveModalDialog::AccountSettings));

    let mut offline = bench(table);
    offline.app.cadastral.set_online(|| false);
    offline.app.press_cadastral();
    offline.frame(1.0);
    assert!(matches!(offline.job(), Some(DownloadJobState::Stopped { code: StopCode::Site1, reason }) if reason == "인터넷 접속이 꺼져 있어 VWorld에 연결할 수 없습니다."));
    for bench in [&none, &bare, &keyless, &nameless, &offline] {
        assert_eq!((bench.runs.load(Ordering::SeqCst), bench.row()), (0, None));
    }
}

/// A run that ends without a table: the gauge shows the reason with its code and no row appears; an
/// account the site did not take opens the account entry (`:142-155`); a stop is a stop (`:139`).
#[test]
fn a_run_that_fails_shows_only_its_reason() {
    let mut refused = bench(|_| Err(PortalStop::Failed { why: Why::AccountRefused, words: "VWorld 로그인에 실패했습니다.".to_owned() }));
    refused.app.press_cadastral();
    refused.frame(1.0);
    refused.frame(1.1);
    assert!(matches!(refused.job(), Some(DownloadJobState::Stopped { code: StopCode::Acc2, reason }) if reason == "VWorld 로그인에 실패했습니다."));
    assert_eq!((refused.row(), refused.app.modals.active), (None, Some(ActiveModalDialog::AccountSettings)));

    let mut empty = bench(|_| Err(PortalStop::Failed { why: Why::NoData, words: "조사 주변 범위와 겹치는 필지가 없습니다.".to_owned() }));
    empty.app.press_cadastral();
    empty.frame(1.0);
    empty.frame(1.1);
    assert!(matches!(empty.job(), Some(DownloadJobState::Stopped { code: StopCode::None1, .. })));
    assert!(empty.app.modals.active.is_none());

    let mut stopped = bench(|_| Err(PortalStop::Cancelled));
    stopped.app.press_cadastral();
    stopped.frame(1.0);
    stopped.frame(1.1);
    assert!(matches!(stopped.job(), Some(DownloadJobState::Cancelled)));
    assert_eq!(stopped.status(), "지적도 받기를 취소했습니다.");
}

/// The table of an earlier run for these very survey areas comes back by a press without asking the site
/// (the C++ keeps the layer in its project file); with the row on the map a press runs again, as the C++
/// does (`:82-133`), and so finds a newer archive of the month. Other survey areas are another table.
#[test]
fn a_table_kept_for_these_areas_comes_back_without_a_run() {
    let mut bench = bench(table);
    bench.app.press_cadastral();
    bench.frame(1.0);
    bench.frame(1.1);
    assert_eq!((bench.runs.load(Ordering::SeqCst), bench.row()), (1, Some(0)));

    // The survey is opened again: the row is gone, the table is in the folder.
    bench.app.left_layers.entries.retain(|e| e.layer_key != CADASTRAL_KEY);
    bench.app.press_cadastral();
    bench.frame(2.0);
    assert_eq!((bench.runs.load(Ordering::SeqCst), bench.row()), (1, Some(0)));
    assert_eq!(bench.status(), "저장된 조사 주변 지적도를 사용합니다.");

    bench.app.press_cadastral();
    bench.frame(3.0);
    bench.frame(3.1);
    assert_eq!(bench.runs.load(Ordering::SeqCst), 2, "with the row on the map a press runs again");

    bench.app.left_layers.entries.retain(|e| e.layer_key != CADASTRAL_KEY);
    draw_survey_area(&mut bench.app, [CENTRE[0] + 500.0, CENTRE[1]], 5187);
    bench.app.press_cadastral();
    bench.frame(4.0);
    bench.frame(4.1);
    assert_eq!(bench.runs.load(Ordering::SeqCst), 3, "another survey area is another table");
    assert!(bench.dir.path().join("지적도").join("strata-kept.json").is_file());
}

/// While a run works, a second press and a press on another map button are told to wait (`:84`); a run
/// belongs to its survey and is stopped when another survey takes its place.
#[test]
fn one_run_at_a_time_and_only_for_its_survey() {
    let mut bench = bench(table);
    bench.app.cadastral.hold_runs();
    bench.app.press_cadastral();
    bench.frame(1.0);
    bench.app.press_cadastral();
    assert_eq!(bench.status(), "지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요.");
    assert!(bench.app.cadastral.is_busy());
    bench.app.drop_cadastral_run();
    bench.frame(1.1);
    assert!(!bench.app.cadastral.is_busy());
    assert!(matches!(bench.job(), Some(DownloadJobState::Cancelled)));
    assert_eq!(bench.row(), None);
}
