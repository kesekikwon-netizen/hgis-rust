//! What the pages themselves report: a popup opening or loading, the map page loading anew, a dialog, a
//! link to another program (`KaTopographicBrowser.cpp:521-525, 536-563, 591-596, 612-616`).

use super::machine::{Asked, NgiiAutomation};
use super::url::{portal_map, transfer_page, UrlParts};
use super::{PageHost, PageId, Stage};

impl NgiiAutomation {
    /// A page was opened or began to load (`:536-550`). Named transfer windows can be reused: only a fresh
    /// load from the page that is submitting may supply the current order's file list.
    pub fn page_load_started(&mut self, host: &dyn PageHost, page: PageId) {
        let loading = host.pages().into_iter().find(|p| p.id == page);
        let opener = loading.as_ref().and_then(|p| p.opener);
        if opener.is_some() && opener == self.order_page && matches!(self.stage, Stage::Order | Stage::Files) {
            self.order_popups.insert(page, (self.generation, self.current_number()));
        }
        // After a login the map page loads anew, and its address is the map's long before its content is
        // there (seconds on the real site). Asked to open the form then, the page script finds the member
        // logged in and no sheet list, and stops the run. The page is not asked until it has loaded; then
        // the selection starts again (`page_loaded`).
        let map_again = self.portal.as_ref().zip(loading).is_some_and(|(portal, p)| UrlParts::parse(&p.url).is_some_and(|url| portal_map(portal, &url)));
        if map_again && matches!(self.stage, Stage::OpenForm | Stage::Form | Stage::Submit) && !self.reloading.contains(&page) {
            self.reloading.push(page);
        }
    }

    /// A page finished loading (`:551-563, 591-596`).
    pub fn page_loaded(&mut self, host: &mut dyn PageHost, page: PageId, _now: u64) {
        let Some(portal) = self.portal.clone() else {
            return;
        };
        self.reloading.retain(|p| *p != page);
        let Some(loaded) = host.pages().into_iter().find(|p| p.id == page) else {
            return;
        };
        let Some(url) = UrlParts::parse(&loaded.url) else {
            return;
        };
        let number = self.current_number();
        let fresh = self.order_popups.get(&page).is_some_and(|(generation, sheet)| *generation == self.generation && *sheet == number);
        if loaded.opener == self.order_page && transfer_page(&portal, &url, self.config.test_mode) && fresh {
            if let Some(token) = host.run_script(page, "window.name === 'downForm'") {
                self.asked.insert(token, Asked::DownForm { generation: self.generation, number, page });
            }
        }
        // The map page loading anew before the order is out starts the selection again (`:591-596`).
        if portal_map(&portal, &url) && matches!(self.stage, Stage::OpenForm | Stage::Form | Stage::Submit) {
            self.generation += 1;
            self.script_in_flight = false;
            self.stage = Stage::Select;
        }
    }

    /// A page asked for a dialog; it was answered without showing anything (`:521-525`).
    pub fn dialog_raised(&mut self) {
        self.message("자동 받기에 추가 확인이 필요합니다. 로그인 설정과 연결 상태를 확인해 주세요.");
    }

    /// A page tried to hand over to another program (`:612-616`): a hidden run cannot follow.
    pub fn external_program_asked(&mut self, host: &mut dyn PageHost) {
        self.fail(host, "자동 전송을 계속할 수 없습니다. 전송 프로그램 연결이 필요합니다. 지도 작업은 계속할 수 있습니다.");
    }
}
