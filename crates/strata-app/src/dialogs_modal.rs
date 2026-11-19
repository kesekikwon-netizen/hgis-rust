//! 모달 대화상자 모음 — 각 대화상자는 자기 파일에 있다.
//!
//! 원래 3,431줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 경로 `crate::dialogs_modal::<이름>` 은 그대로 동작한다.

use crate::app_state::StrataAppState;
use crate::dialogs_download_print::render_download_and_print_dialogs;
use crate::ribbon_icons::paint_strata_logo;
use eframe::egui::{self, Color32, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModalDialog {
    NewSurvey,
    SurveyContour,
    AccountSettings,
    TrenchGenerator,
    ExportSubmitPackage,
    PaperSetup,
    DemDownload,
    HeritageRegionSelect,
    NgiiTopoDownload,
    CadastralDownload,
    CadastralStyle,
    PaleoSourceDownload,
    PdfPreparing,
    ControlPoint,
    SurveyArea,
    LineColor,
    DemClass,
    ReferenceStatus,
    HeritageBrowser,
    TopographicImport,
    CadImportNotice,
    /// `QMessageBox::about` titled 「정보」 (MainWindow.cpp:3048).
    About,
}

pub use crate::modal_types::{topo_crs_label_ko, topo_default_crs_idx, topo_record_key, TOPO_CRS_CANDIDATES, TOPO_CRS_UNKNOWN};
pub use crate::modal_types::{ModalDialogController, TopoTableRow};

pub fn render_strata_modal_header(ui: &mut egui::Ui, title: &str, open: &mut bool) {
    ui.horizontal(|ui| {
        let (logo_r, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::hover());
        paint_strata_logo(&ui.painter_at(logo_r), logo_r);
        ui.add_space(4.0);
        ui.label(egui::RichText::new(title).size(13.5).color(Color32::from_rgb(30, 30, 28)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(egui::RichText::new("×").size(18.0).color(Color32::from_rgb(55, 54, 50))).fill(Color32::TRANSPARENT).stroke(Stroke::NONE);
            if ui.add(close_btn).clicked() {
                *open = false;
            }
        });
    });
    ui.add_space(8.0);
}

#[path = "dlg_about.rs"]
mod dlg_about;

pub(crate) use crate::modal_control::render_control_point;
pub(crate) use crate::modal_downloads::render_dem_download;
pub(crate) use crate::modal_downloads::render_paleo_source_download;
pub(crate) use crate::modal_export::render_export_submit_package;
pub(crate) use crate::modal_paper_account::render_account_settings;
pub(crate) use crate::modal_paper_account::render_paper_setup;
pub(crate) use crate::modal_reference::render_reference_status;
pub(crate) use crate::modal_style::render_line_color;
pub(crate) use crate::modal_style_demclass::render_dem_class;
pub(crate) use crate::modal_survey::render_survey_area;

pub fn render_active_modal_dialogs(app: &mut StrataAppState, ctx: &egui::Context) {
    let Some(kind) = app.modals.active else {
        return;
    };
    let mut open = true;
    match kind {
        ActiveModalDialog::PaperSetup => render_paper_setup(app, ctx, &mut open),
        ActiveModalDialog::DemDownload => render_dem_download(app, ctx, &mut open),
        ActiveModalDialog::AccountSettings => render_account_settings(app, ctx, &mut open),
        ActiveModalDialog::About => dlg_about::render_about(ctx, &mut open),
        ActiveModalDialog::PaleoSourceDownload => render_paleo_source_download(app, ctx, &mut open),
        ActiveModalDialog::ExportSubmitPackage => render_export_submit_package(app, ctx, &mut open),
        ActiveModalDialog::ControlPoint => render_control_point(app, ctx, &mut open),
        ActiveModalDialog::SurveyArea => render_survey_area(app, ctx, &mut open),
        ActiveModalDialog::LineColor => render_line_color(app, ctx, &mut open),
        ActiveModalDialog::DemClass => render_dem_class(app, ctx, &mut open),
        ActiveModalDialog::CadastralStyle => crate::ref_vector::cadastral::layer::render_cadastral_style(app, ctx, &mut open),
        ActiveModalDialog::ReferenceStatus => render_reference_status(app, ctx, &mut open),
        other => render_download_and_print_dialogs(app, ctx, other, &mut open),
    }
    if !open {
        if matches!(kind, ActiveModalDialog::ExportSubmitPackage) {
            app.submit_phase = None;
        }
        app.modals.active = None;
    }
}
