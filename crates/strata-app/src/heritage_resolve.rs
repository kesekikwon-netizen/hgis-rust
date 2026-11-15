//! Which city the survey area lies in, for the 「유산」 dialog (`target/cpp-ref/src/core/
//! HeritageRegionResolver.cpp:292-353`, `app/MainWindow.cpp:3101-3166`): one point inside the survey area is
//! asked of VWorld's address service with the user's key, off the UI thread; the dialog opens with the city
//! it named, or says why none could be given and asks to pick one.

use std::sync::mpsc::TryRecvError;

use strata_engine::{point_on_surface, Transformer};
use strata_net::heritage_region::{address_url, parse_address, HeritageRegion, NOT_FOUND, NOT_REACHED, NO_KEY, TIMEOUT_SECS};
use strata_refmap::river_request::{REFERER, USER_AGENT};
use strata_refmap::transfer::{Request, TransferFailure};
use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::map::run::NotAsked;

#[cfg(test)]
#[path = "heritage_resolve_tests.rs"]
mod tests;

/// `MainWindow.cpp:3101-3106`.
const FINDING: &str = "조사구역이 속한 시·군을 찾는 중입니다…";
/// `HeritageRegionResolver.cpp:296`.
const STILL_FINDING: &str = "이전 판정이 아직 끝나지 않았습니다. 잠시 뒤 다시 누르세요.";
/// `:82`.
const TIMED_OUT: &str = "행정구역 서버의 응답 시간이 초과되었습니다. 시·군을 직접 고르세요.";
/// What the dialog says above its lists (`KaHeritageSetupDialogs.cpp:91-94`).
const CONFIRM: &str = "조사구역이 속한 시·군을 확인하세요.\n자료는 선택한 시·군 단위로만 받습니다.";
const PICK_BELOW: &str = "조사구역이 속한 시·군을 아래에서 직접 선택하면 계속 받을 수 있습니다.";

impl StrataAppState {
    /// The point the city is asked for: one inside the survey area, also when the area is hollow or bent
    /// (`lookupPoint`, `:221-229`), in degrees, longitude first.
    fn heritage_lookup_point(&self) -> Option<[f64; 2]> {
        let rings = crate::ref_vector::map::survey_area_rings(self).ok()?;
        let inside = point_on_surface(&rings).ok()??;
        Transformer::new(CrsCode::from(self.work_crs).epsg_number(), 4326).ok()?.transform(inside).ok()
    }

    /// After a press that may go on: the city is asked for, and the dialog opens when the answer is there.
    pub(crate) fn begin_heritage_region(&mut self) {
        if self.heritage.resolving.is_some() {
            return self.show_status_message(STILL_FINDING);
        }
        let Some([lon, lat]) = self.heritage_lookup_point() else {
            return self.open_heritage_dialog(Err(NOT_FOUND.to_owned()));
        };
        let question = move |key: &str| Request { url: address_url(key, lon, lat), user_agent: USER_AGENT, referer: Some(REFERER), idle_secs: TIMEOUT_SECS, total_secs: TIMEOUT_SECS };
        match self.ref_maps.ask_vworld(question) {
            Ok(answer) => {
                self.show_status_message(FINDING);
                self.heritage.resolving = Some(answer);
            }
            Err(NotAsked::NoKey) => self.open_heritage_dialog(Err(NO_KEY.to_owned())),
            Err(NotAsked::Offline) => self.open_heritage_dialog(Err(NOT_REACHED.to_owned())),
        }
    }

    /// Every frame: takes the answer about the city when it is there.
    pub(crate) fn drive_heritage_region(&mut self, ctx: &eframe::egui::Context) {
        let Some(answer) = self.heritage.resolving.as_ref() else {
            return;
        };
        let found = match answer.try_recv() {
            Err(TryRecvError::Empty) => {
                // Nothing wakes the window while the question is open.
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
                return;
            }
            Ok(Ok(body)) => parse_address(&body).and_then(|region| region.ok_or_else(|| NOT_FOUND.to_owned())),
            Ok(Err(TransferFailure::Timeout)) => Err(TIMED_OUT.to_owned()),
            Ok(Err(TransferFailure::Network | TransferFailure::Cancelled)) | Err(TryRecvError::Disconnected) => Err(NOT_REACHED.to_owned()),
        };
        self.heritage.resolving = None;
        self.open_heritage_dialog(found);
    }

    /// Opens the dialog: with the city that was found, or with the reason none was (what was picked before
    /// stays, and the user picks, `MainWindow.cpp:3110-3116`).
    fn open_heritage_dialog(&mut self, found: Result<HeritageRegion, String>) {
        match found {
            Ok(region) => {
                (self.modals.heritage_sido, self.modals.heritage_city) = (region.sido, region.city);
                self.heritage.region_note = None;
            }
            Err(reason) => self.heritage.region_note = Some(reason),
        }
        // The search is over: the status line no longer says that it runs.
        if self.home_map.draw_guide_message.as_deref() == Some(FINDING) {
            self.home_map.draw_guide_message = None;
        }
        self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect);
    }

    /// What the dialog says above the two lists.
    pub(crate) fn heritage_region_words(&self) -> String {
        match &self.heritage.region_note {
            Some(reason) => format!("{reason}\n{PICK_BELOW}"),
            None => CONFIRM.to_owned(),
        }
    }
}
