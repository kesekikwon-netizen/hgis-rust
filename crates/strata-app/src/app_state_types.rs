//! `app_state` 의 공용 타입 — 단축키 레지스트리·프로젝트 템플릿·탭 열거형.
//!
//! `app_state.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! `StrataAppState` 구조체 자체는 창구(`app_state.rs`)에 그대로 남아 있다.

use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

/// C++ `ShortcutManager::registerShortcut` 단축키 중복 충돌 방어 레지스트리 1:1
#[derive(Debug, Clone, Default)]
pub struct ShortcutManager {
    pub bindings: std::collections::BTreeMap<String, String>,
}

impl ShortcutManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ `ShortcutManager::registerShortcut(actionId, seq) -> bool` — 이미 등록된 키 시퀀스(`seq`)와 충돌 시 `false` 반환 1:1
    pub fn register_shortcut(&mut self, action_id: &str, key_seq: &str) -> bool {
        let normalized_seq = key_seq.trim().to_ascii_uppercase();
        if normalized_seq.is_empty() || action_id.trim().is_empty() {
            return false;
        }
        if self
            .bindings
            .values()
            .any(|existing| existing == &normalized_seq)
        {
            return false;
        }
        self.bindings
            .insert(action_id.trim().to_owned(), normalized_seq);
        true
    }
}

/// C++ `KaHomeScreen::ProjectTemplateConfig` 신규 프로젝트 생성 템플릿 설정 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTemplateConfig {
    pub survey_title: String,
    pub work_crs: WorkCrs,
    pub investigator: String,
    pub default_layers: Vec<DomainLayerKey>,
}

impl Default for ProjectTemplateConfig {
    fn default() -> Self {
        Self {
            survey_title: "새 고고학 지표·시굴조사".to_owned(),
            work_crs: WorkCrs::Epsg5186,
            investigator: "국가유산청 지표·시굴조사단".to_owned(),
            default_layers: vec![
                DomainLayerKey::SurveyArea,
                DomainLayerKey::FeaturePoly,
                DomainLayerKey::FeatureLine,
                DomainLayerKey::TrialTrench,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Home,
    Map,
    Paleo,
    Drawing,
    Terrain3d,
}

impl MainTab {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Home,
            Self::Map,
            Self::Paleo,
            Self::Drawing,
            Self::Terrain3d,
        ]
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Home => "홈",
            Self::Map => "지도",
            Self::Paleo => "고지형분석",
            Self::Drawing => "도면",
            Self::Terrain3d => "입체지형",
        }
    }
}
