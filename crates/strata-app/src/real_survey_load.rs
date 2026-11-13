//! 빈 조사 상태 — 시작·열기·새 조사가 이전 조사의 그림 자료를 비울 때 쓴다.
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 예전의 경주시 폴더 자동 읽기
//! (`load_gyeongju_or_fallback`·`load_from_dir`)는 지시 3-2 에서 없앴다 — C++ 은 시작할 때 홈만 띄운다.

use std::collections::BTreeMap;

use super::real_survey_types::RealSurveyData;

impl RealSurveyData {
    /// C++ `MainWindow::newSurvey` — 새 조사는 빈 프로젝트에서 시작한다.
    /// 경계·시굴·지적·유산 어느 것도 이전 조사에서 남기지 않는다.
    #[must_use]
    pub fn new_empty_survey() -> Self {
        Self {
            survey_name: String::new(),
            site_name: String::new(),
            survey_rings: Vec::new(),
            survey_area_m2: 0.0,
            trenches: Vec::new(),
            distance_rings: BTreeMap::new(),
            layer_styles: BTreeMap::new(),
            label_views: BTreeMap::new(),
            cadastral_parcels: Vec::new(),
            heritage_polys: Vec::new(),
            layer_visible: BTreeMap::new(),
            label_visible: BTreeMap::new(),
            layer_opacity: 1.0,
            layer_brightness: 0.5,
            file_browser_dir: std::env::temp_dir(),
            file_browser_entries: Vec::new(),
            measure_area_tab: false,
        }
    }
}
