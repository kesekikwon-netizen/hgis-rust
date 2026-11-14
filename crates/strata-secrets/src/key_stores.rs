//! VWorld·역사지리정보DB 키를 찾는 곳과 순서 — C++ `VworldSettings::loadApiKey`(VworldSettings.cpp:111-170)·
//! `loadHistoryGisApiKey`(:196-209)의 설치판. C++ 은 뒤에서 찾은 키를 앞 저장소에 다시 써 두지만 Rust 는 읽기만 한다.
//! 레지스트리는 C++ `QSettings("ka-hgis", "ka-hgis")` = `HKCU\Software\ka-hgis\ka-hgis`, 앞에서 못 찾았을 때만 연다.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use crate::accounts::AccountSettings;
use crate::secret_store::parse_qt_ini_file;

/// 키 저장소들 — 시험은 가짜 레지스트리·환경 변수를 넣는다.
pub struct KeyStores<'a> {
    /// 사용자 설정 폴더의 ka-hgis-vworld.ini(C++ makeSettings).
    pub ini: PathBuf,
    /// `HKCU\Software\ka-hgis\ka-hgis\<하위 키>` 의 `ApiKey`.
    pub registry: &'a dyn Fn(&str) -> String,
    pub env: &'a dyn Fn(&str) -> String,
    /// C++ readRepoSecretsIni 후보들.
    pub repo_secrets: Vec<PathBuf>,
    /// 앱 기본 설정 파일(C++ `QSettings bare` = QGIS 프로필 ini, 최근 조사와 같은 파일) — 격리 실행이면 None.
    pub bare_ini: Option<PathBuf>,
}

impl std::fmt::Debug for KeyStores<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyStores").field("ini", &self.ini).finish_non_exhaustive()
    }
}

impl KeyStores<'static> {
    /// 이 PC 의 저장소들.
    #[must_use]
    pub fn user() -> Self {
        Self::in_dir(&AccountSettings::user_config_dir())
    }

    /// 사용자 설정 폴더가 `config_dir` 인 저장소들(레지스트리·환경 변수·저장소 secrets.ini 는 이 PC 것).
    #[must_use]
    pub fn in_dir(config_dir: &std::path::Path) -> Self {
        let app = std::env::current_exe().ok().and_then(|p| p.parent().map(std::path::Path::to_path_buf)).unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_default();
        Self {
            ini: config_dir.join("ka-hgis-vworld.ini"),
            registry: &registry_api_key,
            env: &env_value,
            repo_secrets: vec![
                app.join("../config/secrets.ini"),
                app.join("../../config/secrets.ini"),
                app.join("config/secrets.ini"),
                cwd.join("config/secrets.ini"),
            ],
            bare_ini: bare_user_ini(),
        }
    }
}

/// `%APPDATA%\ka-hgis\ka-hgis\profiles\default\ka-hgis\ka-hgis.ini`(strata-app `user_settings_ini` 와 같은 곳).
/// `KA_HGIS_SETTINGS_DIR` 로 격리한 실행(C++ isolateUserState)이면 C++ 처럼 읽지 않는다.
fn bare_user_ini() -> Option<PathBuf> {
    if std::env::var("KA_HGIS_SETTINGS_DIR").is_ok_and(|s| !s.trim().is_empty()) {
        return None;
    }
    let root = PathBuf::from(std::env::var_os("APPDATA")?).join("ka-hgis").join("ka-hgis");
    Some(["profiles", "default", "ka-hgis", "ka-hgis.ini"].iter().fold(root, |p, s| p.join(s)))
}

impl KeyStores<'_> {
    /// loadApiKey — ini → 레지스트리 → 환경 변수 VWORLD_API_KEY → 앱 기본 설정 파일의 옛 키 →
    /// 저장소 secrets.ini(readKeyFromIni: VWorld/ApiKey·vworld/apiKey·apiKey).
    /// (ini 의 옛 키 vworld/apiKey 는 Windows 에서 대소문자를 가리지 않아 VWorld/ApiKey 와 같은 키다.)
    #[must_use]
    pub fn api_key(&self) -> String {
        let steps: [&dyn Fn() -> String; 5] = [
            &|| ini_value(&self.ini, "VWorld/ApiKey"),
            &|| (self.registry)("VWorld"),
            &|| (self.env)("VWORLD_API_KEY"),
            &|| self.bare_ini.as_deref().map(|p| ini_value(p, "vworld/apiKey")).unwrap_or_default(),
            &|| {
                let read = |p: &PathBuf| [ini_value(p, "VWorld/ApiKey"), ini_value(p, "apiKey")].into_iter().find(|k| !k.trim().is_empty());
                self.repo_secrets.iter().find_map(read).unwrap_or_default()
            },
        ];
        first_key(&steps)
    }

    /// loadHistoryGisApiKey — ini → 레지스트리 → 환경 변수 HISTORY_GIS_API_KEY.
    #[must_use]
    pub fn history_key(&self) -> String {
        let steps: [&dyn Fn() -> String; 3] = [
            &|| ini_value(&self.ini, "HistoryGis/ApiKey"),
            &|| (self.registry)("HistoryGis"),
            &|| (self.env)("HISTORY_GIS_API_KEY"),
        ];
        first_key(&steps)
    }
}

fn first_key(steps: &[&dyn Fn() -> String]) -> String {
    steps.iter().map(|step| step().trim().to_owned()).find(|k| !k.is_empty()).unwrap_or_default()
}

/// QSettings ini 값 — 키 대소문자를 가리지 않는다(Windows).
fn ini_value(path: &std::path::Path, key: &str) -> String {
    if !path.is_file() {
        return String::new();
    }
    parse_qt_ini_file(path).unwrap_or_default().into_iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v).unwrap_or_default()
}

fn env_value(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

/// `reg.exe query HKCU\Software\ka-hgis\ka-hgis\<하위 키> /v ApiKey` 의 값(없으면 빈 글). 값은 표준출력으로만 받는다.
/// 지도 그리기가 매 프레임 키를 찾으므로 한 번 읽은 값을 기억한다(Rust 는 레지스트리에 쓰지 않는다).
fn registry_api_key(subkey: &str) -> String {
    static SEEN: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();
    if !cfg!(windows) {
        return String::new();
    }
    let seen = SEEN.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(v) = seen.lock().ok().and_then(|g| g.get(subkey).cloned()) {
        return v;
    }
    let key = format!("HKCU\\Software\\ka-hgis\\ka-hgis\\{subkey}");
    let mut cmd = Command::new("reg.exe");
    strata_types::process::no_window(&mut cmd)
        .args(["query", key.as_str(), "/v", "ApiKey"])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    let value = cmd
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| reg_sz_value(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    if let Ok(mut g) = seen.lock() {
        g.insert(subkey.to_owned(), value.clone());
    }
    value
}

/// reg.exe 출력의 `<이름>    REG_SZ    <값>` 줄에서 값.
fn reg_sz_value(out: &str) -> String {
    out.lines()
        .find_map(|line| line.split_once("REG_SZ").map(|(_, v)| v.trim().to_owned()))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "key_stores_tests.rs"]
mod tests;
