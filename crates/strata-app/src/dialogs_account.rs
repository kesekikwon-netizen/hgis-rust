use std::path::{Path, PathBuf};

use strata_secrets::accounts::{AccountKind, AccountSettings, Credentials};
use strata_secrets::mask::SecretMask;
use strata_secrets::secret_store::parse_qt_ini_file;
use strata_secrets::vworld::VworldSettings;
use strata_types::error::StrataError;

#[derive(Debug, Clone, Default)]
pub struct AccountDialogState {
    pub config_dir: PathBuf,
    pub vworld_key: String,
    pub history_gis_key: String,
    pub cadastral_creds: Credentials,
    pub ngii_creds: Credentials,
    pub heritage_creds: Credentials,
    pub vworld_configured: bool,
    pub history_gis_configured: bool,
    pub cadastral_configured: bool,
    pub ngii_configured: bool,
    pub heritage_configured: bool,
    pub status_summary_ko: String,
    pub show_passwords: bool,
    /// Set by `validate_credentials` when a site account has only one of ID/PW.
    pub validation_error: Option<String>,
}

impl AccountDialogState {
    #[must_use]
    pub fn probe_user_accounts() -> Self {
        Self::probe_local_accounts(&AccountSettings::user_config_dir())
    }

    #[must_use]
    pub fn probe_local_accounts(config_dir: &Path) -> Self {
        // SEC2 — 진짜 DPAPI 꾸러미는 PowerShell 로 푼다: 계정 파일들의 값을 한 번에(시작이 느려지지 않게).
        let blobs: Vec<String> = [AccountKind::VworldCadastral, AccountKind::NgiiTopographic, AccountKind::HeritageIntranet]
            .iter()
            .flat_map(|k| [k.file_name(), k.alt_file_name()])
            .filter_map(|name| parse_qt_ini_file(&config_dir.join(name)).ok())
            .flat_map(|map| map.into_iter().filter(|(k, _)| k.ends_with("/password_dpapi")).map(|(_, v)| v))
            .collect();
        strata_secrets::dpapi::prefetch_unprotect(&blobs);
        let vworld_ini = config_dir.join("ka-hgis-vworld.ini");
        let vworld_key = VworldSettings::load_from_ini(&vworld_ini)
            .ok()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(VworldSettings::load_api_key);
        let history_gis_key = VworldSettings::load_history_gis_from_ini(&vworld_ini)
            .ok()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(VworldSettings::load_history_gis_api_key);

        let cadastral_creds =
            AccountSettings::read_from_dir(config_dir, AccountKind::VworldCadastral);
        let ngii_creds = AccountSettings::read_from_dir(config_dir, AccountKind::NgiiTopographic);
        let heritage_creds =
            AccountSettings::read_from_dir(config_dir, AccountKind::HeritageIntranet);

        let mut st = Self {
            config_dir: config_dir.to_path_buf(),
            vworld_key,
            history_gis_key,
            cadastral_creds,
            ngii_creds,
            heritage_creds,
            vworld_configured: false,
            history_gis_configured: false,
            cadastral_configured: false,
            ngii_configured: false,
            heritage_configured: false,
            status_summary_ko: String::new(),
            show_passwords: false,
            validation_error: None,
        };
        st.recompute_flags();
        st
    }

    /// C++ KaTopographicAccountDialog.cpp:60-61 — a site account must have both
    /// ID and password, or neither. Only one filled in is a user mistake worth
    /// stopping on, because storing it would look "configured" while failing
    /// to log in later.
    pub fn validate_credentials(&mut self) -> bool {
        for creds in [
            &self.cadastral_creds,
            &self.ngii_creds,
            &self.heritage_creds,
        ] {
            if creds.username.trim().is_empty() != creds.password.is_empty() {
                self.validation_error = Some(
                    "아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요.".to_owned(),
                );
                return false;
            }
        }
        self.validation_error = None;
        true
    }

    pub fn recompute_flags(&mut self) {
        self.vworld_configured = !self.vworld_key.trim().is_empty();
        self.history_gis_configured = !self.history_gis_key.trim().is_empty();
        self.cadastral_configured = !self.cadastral_creds.username.trim().is_empty()
            && !self.cadastral_creds.password.is_empty();
        self.ngii_configured =
            !self.ngii_creds.username.trim().is_empty() && !self.ngii_creds.password.is_empty();
        self.heritage_configured = !self.heritage_creds.username.trim().is_empty()
            && !self.heritage_creds.password.is_empty();

        let ready_cnt = [
            self.vworld_configured,
            self.history_gis_configured,
            self.cadastral_configured,
            self.ngii_configured,
            self.heritage_configured,
        ]
        .into_iter()
        .filter(|&b| b)
        .count();

        let raw_summary = format!(
            "자료 연결 5곳 중 {ready_cnt}곳 설정됨 (VWorld: {}, 역사지리DB: {}, 지적도: {}, 수치지형도: {}, 국가유산청: {})",
            if self.vworld_configured { "설정됨" } else { "미설정" },
            if self.history_gis_configured { "설정됨" } else { "미설정" },
            if self.cadastral_configured { "설정됨" } else { "미설정" },
            if self.ngii_configured { "설정됨" } else { "미설정" },
            if self.heritage_configured { "설정됨" } else { "미설정" },
        );
        self.status_summary_ko = SecretMask::redact(&raw_summary, &[]);
    }

    /// 저장 — 계정 비밀번호를 암호화하지 못하면 그 파일은 쓰지 않고 C++ 의 알림 글을 돌려준다
    /// (KaSecretStore.cpp:156-158, TopographicSettings.cpp:83).
    ///
    /// # Errors
    /// 처음 실패한 계정의 알림 글.
    pub fn save_all_to_disk(&mut self) -> Result<(), String> {
        let dir = if self.config_dir.as_os_str().is_empty() {
            AccountSettings::user_config_dir()
        } else {
            self.config_dir.clone()
        };
        let vworld_ini = dir.join("ka-hgis-vworld.ini");
        let _ = VworldSettings::save_to_ini(&vworld_ini, &self.vworld_key);
        let _ = VworldSettings::save_history_gis_to_ini(&vworld_ini, &self.history_gis_key);
        let mut first_error = None;
        for (kind, creds) in [
            (AccountKind::VworldCadastral, &self.cadastral_creds),
            (AccountKind::NgiiTopographic, &self.ngii_creds),
            (AccountKind::HeritageIntranet, &self.heritage_creds),
        ] {
            if let Err(e) = AccountSettings::save_to_file(&dir.join(kind.file_name()), kind, creds, false) {
                first_error.get_or_insert(match e {
                    StrataError::Secret(message) => message,
                    _ => "계정 설정을 기록하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요.".to_owned(),
                });
            }
        }
        self.recompute_flags();
        first_error.map_or(Ok(()), Err)
    }
}
