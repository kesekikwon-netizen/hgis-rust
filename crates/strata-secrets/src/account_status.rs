//! 자료 연결 다섯 곳이 이 PC 에 넣어져 있는지 — C++ `AccountStatus`(core/AccountStatus.cpp:13-61). 값은 읽어도
//! 내보이지 않고 「있다/없다」만. 계정 셋은 C++ `CadastralPortal::hasCredentials`(CadastralPortal.cpp:129-133),
//! `TopographicSettings::hasCredentials`(TopographicSettings.cpp:38-41), `HeritageIntranetSettings::hasCredentials`
//! (HeritageIntranetSettings.cpp:59-62)와 같은 파일·같은 칸을 본다.

use std::path::{Path, PathBuf};

use crate::accounts::{AccountKind, AccountSettings};
use crate::key_stores::KeyStores;
use crate::secret_store::{parse_qt_ini_file, SecretStore};

/// AccountStatus::Source — 더보기 메뉴 순서.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    VworldKey,
    HistoryGisKey,
    CadastralAccount,
    TopographicAccount,
    HeritageAccount,
}

/// AccountStatus::Entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub source: Source,
    pub label: &'static str,
    pub used_for: &'static str,
    pub menu_path: &'static str,
    pub ready: bool,
}

/// kSources (:20-31).
const SOURCES: [(Source, &str, &str, &str); 5] = [
    (Source::VworldKey, "VWorld API 키", "배경 지도 · 지적 그림 · 시·군 판정", "더보기 → API 키 입력"),
    (Source::HistoryGisKey, "역사지리정보DB 키", "1919 조선지형도", "더보기 → API 키 입력"),
    (Source::CadastralAccount, "VWorld 지적도 계정", "지적도 받기", "더보기 → VWorld 지적도 아이디·비밀번호"),
    (Source::TopographicAccount, "수치지형도 계정", "수치지형도 받기", "더보기 → 수치지형도 아이디·비밀번호"),
    (Source::HeritageAccount, "국가유산 인트라넷 계정", "주변유적 받기", "더보기 → 국가유산 인트라넷 아이디·비밀번호"),
];

/// AccountStatus::snapshot(probe).
#[must_use]
pub fn snapshot(probe: &dyn Fn(Source) -> bool) -> Vec<Entry> {
    SOURCES
        .iter()
        .map(|&(source, label, used_for, menu_path)| Entry { source, label, used_for, menu_path, ready: probe(source) })
        .collect()
}

/// realProbe 가 보는 저장소들.
#[derive(Debug)]
pub struct Stores<'a> {
    pub keys: KeyStores<'a>,
    /// KaPortableRuntime::userConfigDir — 계정 파일 폴더.
    pub config_dir: PathBuf,
    /// 앱 실행 파일 폴더 — 번들 계정(config/*-local.ini).
    pub app_dir: PathBuf,
}

impl Stores<'static> {
    /// 사용자 설정 폴더가 `config_dir` 인 이 PC 의 저장소들.
    #[must_use]
    pub fn in_dir(config_dir: &Path) -> Self {
        let app_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
        Self { keys: KeyStores::in_dir(config_dir), config_dir: config_dir.to_path_buf(), app_dir }
    }
}

impl Stores<'_> {
    /// realProbe (:33-47).
    #[must_use]
    pub fn ready(&self, source: Source) -> bool {
        match source {
            Source::VworldKey => !self.keys.api_key().is_empty(),
            Source::HistoryGisKey => !self.keys.history_key().is_empty(),
            Source::CadastralAccount => self.cadastral_ready(),
            Source::TopographicAccount => self.site_ready(AccountKind::NgiiTopographic, "ngii-local.ini"),
            Source::HeritageAccount => self.site_ready(AccountKind::HeritageIntranet, "heritage-local.ini"),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<Entry> {
        snapshot(&|s| self.ready(s))
    }

    /// vworld-account.ini 의 아이디와 읽히는 비밀번호(KaSecretStore::hasReadablePassword).
    fn cadastral_ready(&self) -> bool {
        let Ok(map) = parse_qt_ini_file(&self.config_dir.join("vworld-account.ini")) else { return false };
        let named = map.get("vworld_account/username").is_some_and(|u| !u.trim().is_empty());
        named && SecretStore::read_password_from_map(&map, "vworld_account").is_ok_and(|p| !p.is_empty())
    }

    /// readFromFiles(개인 파일, 번들 파일들, migrate=false) 의 아이디·비밀번호가 둘 다 있으면.
    fn site_ready(&self, kind: AccountKind, bundled: &str) -> bool {
        let fallbacks = [self.app_dir.join("config").join(bundled), self.app_dir.join("../../config").join(bundled)];
        AccountSettings::read_from_files(&self.config_dir.join(kind.file_name()), &fallbacks, kind)
            .is_ok_and(|c| !c.username.trim().is_empty() && !c.password.is_empty())
    }
}

#[cfg(test)]
#[path = "account_status_tests.rs"]
mod tests;
