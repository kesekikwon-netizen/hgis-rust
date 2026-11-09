//! `dialogs_download_print` 에서 옮긴 「CadImportNotice」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self};

use crate::app_state::StrataAppState;

pub(crate) fn render_cad_import_notice(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    crate::dialogs_modal::render_cad_import_notice(app, ctx, open);
}
