//! 홈의 조사 사실 — C++ `KaStartPage::probeSurveyFacts`(KaStartPage.cpp:321-334: 홈에 올 때 모르는 조사를 세어 기록)와
//! `MainWindow::recordSurveyFacts`(MainWindowChrome.cpp:251-261: 저장 뒤 구역·유구 수와 작업 좌표계). 사용자 ini 는
//! 앱이 붙인 것만(스냅은 사본, 시험은 붙이지 않음).

use std::collections::HashMap;
use std::sync::OnceLock;

use strata_storage::survey_facts::{lookup_all, remember_counts, Facts, MAX_PROBES};
use strata_storage::survey_facts_probe::{drive_kinds, probe_counts_if_unknown};

use crate::app_state::{MainTab, StrataAppState};

/// 이 PC 의 드라이브 종류 — 처음 셀 조사가 있을 때 한 번만 읽는다.
fn drive_kind(letter: char) -> Option<String> {
    static KINDS: OnceLock<HashMap<char, String>> = OnceLock::new();
    KINDS.get_or_init(drive_kinds).get(&letter).cloned()
}

impl StrataAppState {
    /// 매 프레임 — 홈 탭에 들어올 때 한 번 센다(probeSurveyFacts), 다른 탭으로 가면 다음 방문에 다시.
    pub(crate) fn home_facts_on_visit(&mut self) {
        if self.active_tab != MainTab::Home {
            self.home_map.facts_probed = false;
            return;
        }
        if self.home_map.facts_probed {
            return;
        }
        self.home_map.facts_probed = true;
        if let Some(ini) = self.settings_ini.clone() {
            // probeCandidates — 지금 있는 로컬 조사만.
            let paths: Vec<String> = self.recent_surveys.iter().filter(|it| it.available && !it.path.starts_with("//")).map(|it| it.path.clone()).collect();
            let open: Vec<String> = self.survey_path.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            if let Err(e) = probe_counts_if_unknown(&ini, &paths, &open, MAX_PROBES, &drive_kind) {
                eprintln!("[facts] 조사 개수를 기록하지 못했습니다: {e}");
            }
        }
        self.refresh_home_facts();
    }

    /// 홈 목록 조사들의 사실을 다시 읽는다.
    pub(crate) fn refresh_home_facts(&mut self) {
        let paths: Vec<String> = self.recent_surveys.iter().map(|it| it.path.clone()).collect();
        let facts = self.settings_ini.as_deref().map_or_else(|| vec![Facts::default(); paths.len()], |ini| lookup_all(ini, &paths));
        self.home_map.home_facts = paths.into_iter().zip(facts).collect();
    }

    /// 그 조사의 사실(모르면 모름).
    pub(crate) fn facts_of(&self, path: &str) -> Facts {
        self.home_map.home_facts.get(path).cloned().unwrap_or_default()
    }

    /// recordSurveyFacts — 저장한 조사의 구역(survey_area 표들)·유구(feature_poly + feature_line) 수와 작업 좌표계.
    pub(crate) fn record_survey_facts(&mut self, path: &std::path::Path) {
        let Some(ini) = self.settings_ini.clone() else { return };
        let count = |pred: &dyn Fn(&str) -> bool| self.survey_tables.iter().filter(|t| pred(&t.table)).map(|t| t.layer.features.len()).sum::<usize>();
        let areas = count(&crate::survey_area_setup::is_survey_area_key);
        let features = count(&|k: &str| k == "feature_poly" || k == "feature_line");
        let as_i32 = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
        if let Err(e) = remember_counts(&ini, &path.to_string_lossy(), as_i32(areas), as_i32(features), self.work_crs.auth_id()) {
            eprintln!("[facts] 조사 개수를 기록하지 못했습니다: {e}");
        }
        self.refresh_home_facts();
    }
}

#[cfg(test)]
#[path = "app_state_home_facts_tests.rs"]
mod tests;
