//! 다운로드·인쇄 대화상자 모음 — 각 대화상자는 자기 파일에 있다.
//!
//! 원래 3,077줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 호출 경로 `crate::dialogs_download_print::render_download_and_print_dialogs` 는 그대로다.

use eframe::egui::{self, Color32};
use strata_net::heritage_intranet::{HeritagePipelineOutcome, HeritageStage};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

pub(crate) use crate::dlg_heritage_region_select::render_heritage_region_select;
pub(crate) use crate::dlg_ngii_topo_download::render_ngii_topo_download;
pub(crate) use crate::dlg_cadastral_download::render_cadastral_download;
pub(crate) use crate::dlg_trench_generator::render_trench_generator;
pub(crate) use crate::dlg_pdf_preparing::render_pdf_preparing;
pub(crate) use crate::dlg_drawing_print::render_drawing_print;
pub(crate) use crate::dlg_new_survey::render_new_survey;
pub(crate) use crate::dlg_survey_contour::render_survey_contour;
pub(crate) use crate::dlg_heritage_browser::render_heritage_browser;
pub(crate) use crate::dlg_topographic_import::render_topographic_import;
pub(crate) use crate::dlg_cad_import_notice::render_cad_import_notice;



/// C++ `KaDownloadUi::configure` 1:1 — 베이지 헤더 배너(`#EFECE4`)
pub(crate) fn render_ka_download_header(ui: &mut egui::Ui, title: &str, subtitle: &str, min_w: f32) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(239, 236, 228))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_min_width(min_w);
            ui.label(egui::RichText::new(title).size(19.0).strong());
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(subtitle)
                    .size(13.5)
                    .color(Color32::from_rgb(80, 79, 74)),
            );
        });
    ui.add_space(10.0);
}

/// 「받기」 한 번의 인트라넷 작업을 딴 실에서 돌린다. UI 를 멈추지 않는다.
struct HeritageFetchJob {
    rx: std::sync::mpsc::Receiver<Result<HeritagePipelineOutcome, String>>,
}

thread_local! {
    static HERITAGE_FETCH_JOB: std::cell::RefCell<Option<HeritageFetchJob>> =
        const { std::cell::RefCell::new(None) };
}

/// 받는 중인가. 화면을 다시 그리게 하는 데 쓴다.
pub(crate) fn heritage_fetch_in_progress() -> bool {
    HERITAGE_FETCH_JOB.with(|job| job.borrow().is_some())
}

/// 「받기」 — C++ `KaHeritageBrowser::start()` (`KaHeritageBrowser.cpp:571-629`) 와 같은 순서.
///
/// 막는 조건(계정·시군·조사폴더)은 요청 전에 C++ 문구로 끝낸다. 통과하면
/// 다운로드 화면 → 목록 → 폼 → 시·군 검색 → 자료별 ZIP 을 실제 HTTP 로 받는다.
pub(crate) fn heritage_start_fetch(app: &mut StrataAppState) {
    if heritage_fetch_in_progress() {
        return;
    }
    let sido = app.modals.heritage_sido.clone();
    let city = app.modals.heritage_city.clone();
    // ReferenceStorage.cpp:39-43 — derived from the survey path; no survey file -> None ->
    // MainWindow.cpp:3095-3099 text from heritage_preflight.
    let survey_dir = crate::survey_folder::survey_folder(app.survey_path.as_deref());
    let have_account = app.accounts.heritage_configured;

    if let Err(why) = strata_net::heritage_intranet::heritage_preflight(
        have_account,
        survey_dir.as_deref(),
        &sido,
        &city,
    ) {
        app.modals.heritage_stage_idx = HeritageStage::Failed.ui_index();
        app.modals.heritage_progress_pct = 0.0;
        app.modals.heritage_status_detail = why;
        return;
    }

    let datasets: Vec<(String, String)> = strata_net::heritage_intranet::HERITAGE_TAB_CODES
        .iter()
        .map(|(label, tab)| ((*label).to_owned(), (*tab).to_owned()))
        .collect();
    let (tx, rx) = std::sync::mpsc::channel();
    // heritage_preflight above already refused a missing survey folder.
    let Some(worker_dir) = survey_dir else {
        return;
    };
    let worker_sido = sido.clone();
    let worker_city = city.clone();
    std::thread::spawn(move || {
        let mut outcome = match strata_net::heritage_intranet::heritage_http_search(
            Some(worker_dir.as_path()),
            have_account,
            &worker_sido,
            &worker_city,
            None,
        ) {
            Ok(outcome) => outcome,
            Err(why) => {
                let _ = tx.send(Err(why));
                return;
            }
        };
        let _ = strata_net::heritage_intranet::heritage_http_download(
            &mut outcome,
            worker_dir.as_path(),
            &datasets,
            None,
        );
        let _ = tx.send(Ok(outcome));
    });

    HERITAGE_FETCH_JOB.with(|job| *job.borrow_mut() = Some(HeritageFetchJob { rx }));
    app.modals.heritage_stage_idx = HeritageStage::Search.ui_index();
    app.modals.heritage_progress_pct = 0.2;
    app.modals.heritage_status_detail =
        format!("{sido} {city} — 인트라넷에서 자료 목록과 검색 결과를 받는 중입니다.");
    app.modals.heritage_request_log.clear();
    app.home_map.draw_guide_message = Some(app.modals.heritage_status_detail.clone());
}

/// 끝난 작업이 있으면 창에 반영한다. 반영했으면 true.
pub(crate) fn heritage_poll_fetch(app: &mut StrataAppState) -> bool {
    let finished = HERITAGE_FETCH_JOB.with(|job| {
        let mut slot = job.borrow_mut();
        let running = slot.as_ref()?;
        match running.rx.try_recv() {
            Ok(result) => {
                *slot = None;
                Some(result)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                *slot = None;
                Some(Err("인트라넷 작업이 중단되었습니다.".to_owned()))
            }
        }
    });
    let Some(result) = finished else {
        return false;
    };
    match result {
        Ok(outcome) => {
            let failed: Vec<String> = outcome.files.iter().filter_map(|f| f.error.clone()).collect();
            let saved = outcome.files.iter().filter(|f| f.saved_path.is_some()).count();
            app.modals.heritage_request_log = outcome.request_log.clone();
            app.modals.heritage_progress_pct = 1.0;
            if saved > 0 {
                app.modals.heritage_stage_idx = HeritageStage::Done.ui_index();
                let mut detail = outcome.detail_ko();
                if !failed.is_empty() {
                    detail.push_str(&format!(
                        " · 받지 못한 자료 {}종: {}",
                        failed.len(),
                        failed.join(" / ")
                    ));
                }
                app.modals.heritage_status_detail = detail;
            } else {
                // 파일이 하나도 안 왔다: 성공으로 넘기지 않는다(C++ `fail` 과 같은 태도).
                app.modals.heritage_stage_idx = HeritageStage::Failed.ui_index();
                app.modals.heritage_progress_pct = 0.0;
                app.modals.heritage_status_detail = if failed.is_empty() {
                    outcome.detail_ko()
                } else {
                    failed.join("\n")
                };
            }
            app.home_map.draw_guide_message = Some(app.modals.heritage_status_detail.clone());
        }
        Err(why) => {
            app.modals.heritage_stage_idx = HeritageStage::Failed.ui_index();
            app.modals.heritage_progress_pct = 0.0;
            app.modals.heritage_status_detail = why.clone();
            app.home_map.draw_guide_message = Some(why);
        }
    }
    true
}

/// C++ 자료 받기·시굴격자·도면 인쇄 대화상자 (`KaDownloadUi.cpp`, `KaTrenchDialog.cpp`, `KaPrintDialogPlate.cpp`) 1:1 재현
pub fn render_download_and_print_dialogs(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    kind: ActiveModalDialog,
    open: &mut bool,
) {
    // 받는 중이면 끝난 결과를 반영하고, 끝날 때까지 화면을 계속 다시 그린다.
    if matches!(
        kind,
        ActiveModalDialog::HeritageBrowser | ActiveModalDialog::HeritageRegionSelect
    ) && heritage_poll_fetch(app)
    {
        ctx.request_repaint();
    }
    if heritage_fetch_in_progress() {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    match kind {
        ActiveModalDialog::HeritageRegionSelect => render_heritage_region_select(app, ctx, open),
        ActiveModalDialog::NgiiTopoDownload => render_ngii_topo_download(app, ctx, open),
        ActiveModalDialog::CadastralDownload => render_cadastral_download(app, ctx, open),
        ActiveModalDialog::TrenchGenerator => render_trench_generator(app, ctx, open),
        ActiveModalDialog::PdfPreparing => render_pdf_preparing(app, ctx, open),
        ActiveModalDialog::DrawingPrint => render_drawing_print(app, ctx, open),
        ActiveModalDialog::NewSurvey => render_new_survey(app, ctx, open),
        ActiveModalDialog::SurveyContour => render_survey_contour(app, ctx, open),
        ActiveModalDialog::HeritageBrowser => render_heritage_browser(app, ctx, open),
        ActiveModalDialog::TopographicImport => render_topographic_import(app, ctx, open),
        ActiveModalDialog::CadImportNotice => render_cad_import_notice(app, ctx, open),
        _ => {}
    }
}
