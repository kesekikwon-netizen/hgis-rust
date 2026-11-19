//! CRS candidates on the CAD notice (`KaCadImportNotice.cpp:39-68`).

use std::path::Path;

use crate::app_state::StrataAppState;
use crate::cad_crs_stated::{crs_label, stated_auth_ids, CANDIDATE_AUTH_IDS};
use crate::dialogs_modal::ActiveModalDialog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CadNoticeKind {
    OtherCrs { auth_id: String },
    NoCoordinates,
    Align,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CadNoticeLine {
    pub kind: CadNoticeKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CadCrsNotice {
    pub menu: Vec<CadNoticeLine>,
    pub align_label: &'static str,
}

/// `used_auth_id` is left out (`KaCadImportNotice.cpp:54`).
/// `recent` is `CadCrsGuess.cpp:181-184`: stated ids share the low base, then the recent index.
#[must_use]
pub(crate) fn notice_for(path: &Path, used_auth_id: &str, recent: &[String]) -> CadCrsNotice {
    let header = header_text(path);
    let texts = header.as_slice();
    let stated = stated_auth_ids(path, texts);
    let mut ids: Vec<&str> = CANDIDATE_AUTH_IDS.to_vec();
    ids.sort_by_key(|id| crs_rank(id, &stated, recent));
    let mut menu = Vec::new();
    for id in ids.into_iter().filter(|id| *id != used_auth_id) {
        menu.push(CadNoticeLine {
            kind: CadNoticeKind::OtherCrs {
                auth_id: id.to_owned(),
            },
            label: format!("{} ({id})", crs_label(id)),
        });
    }
    menu.push(CadNoticeLine {
        kind: CadNoticeKind::NoCoordinates,
        label: "좌표 없는 도면으로 보기".to_owned(),
    });
    CadCrsNotice {
        menu,
        align_label: "직접 맞추기",
    }
}

#[must_use]
pub(crate) fn notice_choice_status(file_name: &str, kind: &CadNoticeKind) -> String {
    match kind {
        // `KaCadImportNotice.cpp:64-65` — the drawing group is gone, so align cannot start.
        CadNoticeKind::Align => "이 도면이 지도에 없습니다. 도면을 다시 불러와 주세요.".to_owned(),
        CadNoticeKind::OtherCrs { .. } | CadNoticeKind::NoCoordinates => {
            format!("CAD 도면(DXF·DWG)은 아직 지도에 올리지 못합니다: {file_name}")
        }
    }
}

impl StrataAppState {
    pub(crate) fn open_cad_crs_notice(&mut self, path: &Path) {
        self.modals.cad_import_path = path.display().to_string();
        self.modals.active = Some(ActiveModalDialog::CadImportNotice);
    }

    pub(crate) fn apply_cad_notice(&mut self, kind: &CadNoticeKind) {
        let name = Path::new(&self.modals.cad_import_path)
            .file_name()
            .map(|file_name| file_name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.show_status_message(notice_choice_status(&name, kind));
        self.modals.active = None;
    }
}

fn header_text(path: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let end = bytes.len().min(65_536);
    vec![String::from_utf8_lossy(&bytes[..end]).into_owned()]
}

fn crs_rank(id: &str, stated: &[String], recent: &[String]) -> i32 {
    let base = if stated.iter().any(|item| item == id) {
        0
    } else {
        1000
    };
    let place = recent
        .iter()
        .position(|item| item == id)
        .map_or(100, |index| i32::try_from(index).unwrap_or(100));
    base + place
}

#[cfg(test)]
#[path = "cad_crs_notice_tests.rs"]
mod tests;
