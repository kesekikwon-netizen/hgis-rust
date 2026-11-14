//! 다운로드·인쇄 대화상자 모음 — 각 대화상자는 자기 파일에 있다.
//!
//! 원래 3,077줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 호출 경로 `crate::dialogs_download_print::render_download_and_print_dialogs` 는 그대로다.

use eframe::egui::{self, Color32};

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

/// C++ 자료 받기·시굴격자·도면 인쇄 대화상자 (`KaDownloadUi.cpp`, `KaTrenchDialog.cpp`, `KaPrintDialogPlate.cpp`) 1:1 재현
pub fn render_download_and_print_dialogs(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    kind: ActiveModalDialog,
    open: &mut bool,
) {
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
