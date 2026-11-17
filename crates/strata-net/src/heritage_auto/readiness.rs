//! DOM evidence before downloads (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:949-1147`).

use super::{HeritageAutomation, HeritageEvent, HeritageStage, LOGIN_POLL_MS};

/// Constructed only from the positive pre-search count (`KaHeritageBrowser.cpp:1261-1268`).
#[derive(Debug, Clone, Copy)]
pub(super) struct Baseline(i64);

#[derive(serde::Deserialize)]
struct ResultInfo {
    rows: u64,
    total: i64,
    #[serde(rename = "searchReady")]
    search_ready: bool,
}

/// The C++ navigation bound is 75 polls of 700 ms (`KaHeritageBrowser.cpp:103,135,871-875`).
pub(super) const NAVIGATION_LIMIT_MS: u64 = 75 * LOGIN_POLL_MS;

impl HeritageAutomation {
    pub(super) fn on_download_page_ready(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            "ready" => {
                self.go(HeritageStage::Terms);
            }
            "page-error" => {
                return self
                    .stop("국가유산 사이트가 찾을 수 없는 페이지를 표시했습니다.".to_owned())
            }
            _ => {}
        }
        None
    }

    pub(super) fn on_terms_ready(&mut self, value: &str) -> Option<HeritageEvent> {
        if value == "codedeta" {
            self.go(HeritageStage::Kind);
        }
        None
    }

    pub(super) fn on_kind_ready(&mut self, value: &str) -> Option<HeritageEvent> {
        if value == "ready" {
            self.wait_for(HeritageStage::KindBaseline);
        }
        None
    }

    pub(super) fn on_baseline(&mut self, value: &str) -> Option<HeritageEvent> {
        if let Ok(info) = serde_json::from_str::<ResultInfo>(value) {
            if info.total > 0 {
                self.baseline = Some(Baseline(info.total));
                self.go(HeritageStage::Region);
            }
        }
        None
    }

    pub(super) fn on_region_ready(&mut self, value: &str) -> Option<HeritageEvent> {
        let selected = value
            .strip_prefix("ok:")
            .and_then(|values| values.split_once('|'));
        if selected.is_some_and(|(sido, city)| !sido.is_empty() && !city.is_empty()) {
            self.go(HeritageStage::Search);
        } else {
            self.wait_for(HeritageStage::Region);
        }
        None
    }

    pub(super) fn on_results(&mut self, value: &str) -> Option<HeritageEvent> {
        let Ok(info) = serde_json::from_str::<ResultInfo>(value) else {
            return self.stop("검색 건수를 읽지 못해 전국 받기를 하지 않습니다".to_owned());
        };
        self.quiet_ticks = self.quiet_ticks.saturating_add(1);
        if !info.search_ready {
            return None;
        }
        let Some(Baseline(before)) = self.baseline else {
            return self.stop("검색 건수를 읽지 못해 전국 받기를 하지 않습니다".to_owned());
        };
        if info.total == 0 {
            let kind = self.current_kind();
            if matches!(self.advance_kind(), Some(HeritageEvent::Done)) {
                self.queued_events.push(HeritageEvent::Done);
            }
            return Some(HeritageEvent::KindAbsent { kind });
        }
        if info.rows == 0 {
            return None;
        }
        if info.total < 0 || info.total == before {
            if self.quiet_ticks <= 20 {
                return None;
            }
            return self.stop(if info.total < 0 {
                format!("검색 건수를 읽지 못해 전국 받기를 하지 않습니다(검색 전 {before}건 · 검색 후 {}건).", info.total)
            } else {
                format!("시·군 조건이 걸리지 않았습니다(검색결과가 {before}건 그대로입니다). 전국 자료를 받지 않기 위해 멈춥니다.")
            });
        }
        self.go(HeritageStage::DownloadAll);
        None
    }
}
