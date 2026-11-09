//! `dialogs_download_print` 에서 옮긴 「NgiiTopoDownload」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self};

use crate::app_state::StrataAppState;
use crate::dlg_topographic_browser::render_topographic_browser;

pub(crate) fn render_ngii_topo_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    render_topographic_browser(app, ctx, open);
}
