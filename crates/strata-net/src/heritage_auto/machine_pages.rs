//! The stages between the login and the first file: open the download panel, agree to the
//! pledge, pick a kind, pick the city, press 검색.
//!
//! Stage order is the C++ one (`KaHeritageBrowser.cpp:820-826`):
//! `Login → DismissTutorial → OpenDownloadPage → AgreeTerms → SelectDataset → SelectRegion →
//! Search`. Each stage's answer words come from the script that was run
//! (`HeritageIntranetFlow.cpp`).

use super::machine::{HeritageAutomation, HeritageEvent, HeritageStage};

impl HeritageAutomation {
    /// The script for the current stage, if that stage is one of these.
    pub(super) fn page_script(&self) -> Option<String> {
        match self.stage {
            // The tutorial hides the top menu; `none` means there was nothing to dismiss.
            HeritageStage::DismissTutorial => Some(super::scripts::dismiss_tutorial_script()),
            HeritageStage::DownloadPage => Some(super::scripts::open_download_page_script()),
            HeritageStage::DownloadPageReady => Some(super::scripts::download_page_probe_script()),
            HeritageStage::Terms => Some(super::scripts::agree_terms_script()),
            HeritageStage::TermsReady => Some(super::scripts::form_hint_script()),
            HeritageStage::Kind => Some(super::scripts::select_dataset_script(
                super::scripts::KINDS[self.kind_index.min(super::scripts::KINDS.len() - 1)].0,
            )),
            HeritageStage::KindReady => Some(super::scripts::dataset_ready_script(
                super::scripts::KINDS[self.kind_index].0,
            )),
            HeritageStage::KindBaseline | HeritageStage::Results => {
                Some(super::scripts::result_page_info_script())
            }
            HeritageStage::Region => Some(super::scripts::select_region_script(
                &self.config.sido,
                &self.config.city,
            )),
            HeritageStage::RegionReady => Some(super::scripts::verify_region_script()),
            HeritageStage::Search => Some(super::scripts::search_script()),
            _ => None,
        }
    }

    /// One answer to a page stage. Each arm is one C++ callback's body.
    pub(super) fn on_page_answer(&mut self, value: &str) -> Option<HeritageEvent> {
        match self.stage.clone() {
            HeritageStage::DismissTutorial => self.on_tutorial(value),
            HeritageStage::DownloadPage => self.on_download_page(value),
            HeritageStage::DownloadPageReady => self.on_download_page_ready(value),
            HeritageStage::Terms => self.on_terms(value),
            HeritageStage::TermsReady => self.on_terms_ready(value),
            HeritageStage::Kind => self.on_kind(value),
            HeritageStage::KindReady => self.on_kind_ready(value),
            HeritageStage::KindBaseline => self.on_baseline(value),
            HeritageStage::Region => self.on_region(value),
            HeritageStage::RegionReady => self.on_region_ready(value),
            HeritageStage::Search => self.on_search(value),
            HeritageStage::Results => self.on_results(value),
            _ => None,
        }
    }

    /// Only an absent tutorial permits the next action (`KaHeritageBrowser.cpp:935-944`).
    fn on_tutorial(&mut self, value: &str) -> Option<HeritageEvent> {
        if value == "none" {
            self.go(HeritageStage::DownloadPage);
        } else {
            self.wait_for(HeritageStage::TutorialProbe);
        }
        None
    }

    /// `openDownloadPageScript`: `already-open`, `opening`, `not-found`, `error:…`.
    /// The panel is opened through the site's own `showAgreePopup()`, never by navigating to
    /// `heritageDownload.do` (`:217-221`).
    fn on_download_page(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            "already-open" | "opening" => {
                self.wait_for(HeritageStage::DownloadPageReady);
                None
            }
            "not-found" => None,
            other => self.stop(format!(
                "자료 받기 화면을 여는 중 문제가 생겼습니다({other})."
            )),
        }
    }

    /// `agreeTermsScript`: `already-open`, `not-found`, `checked-no-confirm`, or a JSON object
    /// `{status, text}` where `text` is the pledge the page showed.
    fn on_terms(&mut self, value: &str) -> Option<HeritageEvent> {
        if value == "already-open" {
            self.go(HeritageStage::Kind);
            return None;
        }
        if let Some(text) = pledge_text(value) {
            self.pledge = Some(text.clone());
            self.wait_for(HeritageStage::TermsReady);
            return Some(HeritageEvent::PledgeAgreed { terms: text });
        }
        match value {
            "not-found" => self.stop(
                "서약서 동의 상자를 찾지 못했습니다. 인트라넷 화면이 바뀌었는지 확인해 주세요."
                    .to_owned(),
            ),
            "checked-no-confirm" => {
                self.stop("서약서에 동의는 했으나 「확인」 단추를 찾지 못했습니다.".to_owned())
            }
            other => self.stop(format!("서약서 동의 중 문제가 생겼습니다({other}).")),
        }
    }

    /// `selectDatasetScript`: `selected`, `not-found`, `no-changeTab`, `error:…`.
    /// The new kind is confirmed by the form node changing (`datasetReadyScript`, `:263-270`).
    fn on_kind(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            "selected" => {
                self.wait_for(HeritageStage::KindReady);
                None
            }
            "not-found" => self.stop(
                "유산 종류 탭을 찾지 못했습니다. 인트라넷 화면이 바뀌었는지 확인해 주세요."
                    .to_owned(),
            ),
            "no-changeTab" => self.stop(
                "인트라넷 화면에 종류를 바꾸는 기능이 없습니다. 화면이 바뀌었는지 확인해 주세요."
                    .to_owned(),
            ),
            other => self.stop(format!("유산 종류를 고르는 중 문제가 생겼습니다({other}).")),
        }
    }

    /// `selectRegionScript`: `selected`, `no-sido`, `not-found`, `sigungu-pending`,
    /// `sigungu:<n>` (`:272-328`).
    ///
    /// `no-sido` means the 시·도 was not among the options — the survey's city is not on the
    /// site, which is a fact about the survey, not a fault of the run.
    fn on_region(&mut self, value: &str) -> Option<HeritageEvent> {
        if value == "selected" {
            self.wait_for(HeritageStage::RegionReady);
            return None;
        }
        if let Some(count) = value.strip_prefix("sigungu:") {
            self.quiet_ticks = self.quiet_ticks.saturating_add(1);
            if self.quiet_ticks <= 12 || count == "0" {
                return None;
            }
            let city = self.config.city.clone();
            let sido = self.config.sido.clone();
            return self.stop(format!(
                "「{sido} {city}」 가 인트라넷 목록에 없습니다(시·군 {count}곳 중). \
                 조사 위치의 시·군을 확인해 주세요."
            ));
        }
        match value {
            "no-sido" => {
                let sido = self.config.sido.clone();
                self.stop(format!("「{sido}」 가 인트라넷 시·도 목록에 없습니다."))
            }
            "sigungu-pending" | "not-found" => None,
            other => self.stop(format!("지역을 고르는 중 문제가 생겼습니다({other}).")),
        }
    }

    /// `searchScript`: `searching` or `not-found`. `searching` only means the button was
    /// pressed; `resultPageInfoScript` says whether rows came back (`:396-413`).
    fn on_search(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            "searching" => {
                self.go(HeritageStage::Results);
                None
            }
            "not-found" => {
                self.quiet_ticks = self.quiet_ticks.saturating_add(1);
                if self.quiet_ticks > 15 {
                    self.stop("다운로드 폼에서 「검색」을 찾지 못했습니다.".to_owned())
                } else {
                    None
                }
            }
            other => self.stop(format!("검색 중 문제가 생겼습니다({other}).")),
        }
    }
}

/// The pledge text out of `{"status":"agreed","text":"…"}`, when that is what came back.
fn pledge_text(value: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(value).ok()?;
    (parsed.get("status")?.as_str()? == "agreed")
        .then(|| parsed.get("text")?.as_str().map(str::to_owned))
        .flatten()
}
