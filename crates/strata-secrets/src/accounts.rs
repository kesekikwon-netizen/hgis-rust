use crate::secret_store::{parse_qt_ini_file, serialize_qt_ini, SecretStore};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use strata_types::error::StrataResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    HeritageIntranet,
    NgiiTopographic,
    VworldCadastral,
}

impl AccountKind {
    pub const fn group_name(self) -> &'static str {
        match self {
            Self::HeritageIntranet => "heritage",
            Self::NgiiTopographic => "ngii",
            Self::VworldCadastral => "vworld_account",
        }
    }

    pub const fn alt_group_name(self) -> &'static str {
        match self {
            Self::HeritageIntranet => "heritage",
            Self::NgiiTopographic => "ngii",
            Self::VworldCadastral => "cadastral",
        }
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::HeritageIntranet => "heritage-account.ini",
            Self::NgiiTopographic => "ngii-account.ini",
            Self::VworldCadastral => "vworld-account.ini",
        }
    }

    pub const fn alt_file_name(self) -> &'static str {
        match self {
            Self::HeritageIntranet => "heritage-account.ini",
            Self::NgiiTopographic => "ngii-account.ini",
            Self::VworldCadastral => "cadastral-account.ini",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    pub fn is_ready(&self) -> bool {
        !self.username.trim().is_empty() && !self.password.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccountSettings;

impl AccountSettings {
    #[must_use]
    pub fn user_config_dir() -> PathBuf {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(&local).join("ka-hgis").join("ka-hgis");
            if p.is_dir() {
                return p;
            }
            return PathBuf::from(local).join("ka-hgis").join("ka-hgis");
        }
        let wsl_path = PathBuf::from("/mnt/c/Users/권을/AppData/Local/ka-hgis/ka-hgis");
        if wsl_path.is_dir() {
            return wsl_path;
        }
        std::env::temp_dir().join("ka-hgis")
    }

    pub fn save_to_file(
        path: &Path,
        kind: AccountKind,
        creds: &Credentials,
        portable: bool,
    ) -> StrataResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let group = kind.group_name();
        let mut map = BTreeMap::new();
        map.insert(format!("{group}/username"), creds.username.clone());
        SecretStore::write_password_to_map(&mut map, group, &creds.password, portable)?;
        let content = serialize_qt_ini(&map);
        let tmp_path = path.with_extension("ini.tmp");
        std::fs::write(&tmp_path, content.as_bytes())?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    pub fn read_ini(path: &Path, kind: AccountKind) -> StrataResult<Credentials> {
        if !path.is_file() {
            return Ok(Credentials::empty());
        }
        let map = parse_qt_ini_file(path)?;
        for group in [kind.group_name(), kind.alt_group_name()] {
            let username = map
                .get(&format!("{group}/username"))
                .cloned()
                .unwrap_or_default();
            let password = SecretStore::read_password_from_map(&map, group).unwrap_or_default();
            if !username.is_empty() || !password.is_empty() {
                return Ok(Credentials { username, password });
            }
        }
        Ok(Credentials::empty())
    }

    pub fn read_from_dir(config_dir: &Path, kind: AccountKind) -> Credentials {
        let primary = config_dir.join(kind.file_name());
        if let Ok(c) = Self::read_ini(&primary, kind) {
            if !c.username.is_empty() || !c.password.is_empty() {
                return c;
            }
        }
        let alt = config_dir.join(kind.alt_file_name());
        Self::read_ini(&alt, kind).unwrap_or_default()
    }

    pub fn read_from_files(
        personal: &Path,
        fallbacks: &[PathBuf],
        kind: AccountKind,
    ) -> StrataResult<Credentials> {
        if personal.is_file() {
            let creds = Self::read_ini(personal, kind)?;
            if !creds.password.is_empty() {
                return Ok(creds);
            }
            let map = parse_qt_ini_file(personal)?;
            let undec = SecretStore::has_undecryptable_password(&map, kind.group_name())
                || SecretStore::has_undecryptable_password(&map, kind.alt_group_name());
            if !undec {
                return Ok(creds);
            }
        }
        for fb in fallbacks {
            if fb.is_file() {
                return Self::read_ini(fb, kind);
            }
        }
        Ok(Credentials::empty())
    }

    pub fn describe_for_log(creds: &Credentials) -> &'static str {
        if creds.username.trim().is_empty() {
            "계정 없음"
        } else if creds.password.is_empty() {
            "계정 설정 미완료"
        } else {
            "저장된 계정 사용"
        }
    }
}

/// C++ `KaAccountModal::ApiServiceType` — 5대 외부 공공 서비스 구분
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiServiceType {
    VWorld,
    KoreanHistoryDb,
    NationalSpatialCadastral,
    NgiiTopographic,
    EHeritage,
}

impl ApiServiceType {
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::VWorld => "VWorld",
            Self::KoreanHistoryDb => "역사지리DB",
            Self::NationalSpatialCadastral => "국가공간정보(지적도)",
            Self::NgiiTopographic => "국토정보플랫폼(수치지형도)",
            Self::EHeritage => "국가유산청",
        }
    }

    #[must_use]
    pub const fn min_key_len(self) -> usize {
        match self {
            Self::VWorld => 32,
            Self::KoreanHistoryDb
            | Self::NationalSpatialCadastral
            | Self::NgiiTopographic
            | Self::EHeritage => 16,
        }
    }
}

/// C++ `KaAccountModal::AccountValidationResult`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountValidationResult {
    Ok,
    InvalidKeyLength,
}

/// C++ `KaAccountModal::validateAndSaveCredentials` — 5대 공공 서비스 API 키 공백 제거 및 최소 길이(VWorld 32자, 기타 16자) 검증
pub fn validate_service_credential_key(
    service: ApiServiceType,
    raw_key: &str,
) -> Result<String, (AccountValidationResult, String)> {
    let trimmed = raw_key.trim();
    let cur_len = trimmed.chars().count();
    let min_len = service.min_key_len();
    if cur_len < min_len {
        return Err((
            AccountValidationResult::InvalidKeyLength,
            format!(
                "{} API 키는 최소 {}자 이상이어야 합니다 (현재: {}자)",
                service.label_ko(),
                min_len,
                cur_len
            ),
        ));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_personal_account_file_disables_default_login_without_falling_back_to_bundled(
    ) -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let personal = tmp.path().join("heritage-account.ini");
        let bundled = tmp.path().join("heritage-local.ini");
        AccountSettings::save_to_file(
            &personal,
            AccountKind::HeritageIntranet,
            &Credentials::empty(),
            true,
        )?;
        AccountSettings::save_to_file(
            &bundled,
            AccountKind::HeritageIntranet,
            &Credentials::new("bundled_id", "bundled_pw"),
            true,
        )?;
        let loaded =
            AccountSettings::read_from_files(&personal, &[bundled], AccountKind::HeritageIntranet)?;
        assert!(loaded.username.is_empty());
        assert!(loaded.password.is_empty());
        Ok(())
    }

    #[test]
    fn test_validate_service_credential_key_vworld_32_and_ngii_16() {
        let short_vw = validate_service_credential_key(ApiServiceType::VWorld, "1234567890");
        assert!(short_vw.is_err());
        let (code, msg) = short_vw.expect_err("short vworld key must fail");
        assert_eq!(code, AccountValidationResult::InvalidKeyLength);
        assert!(msg.contains("최소 32자"));
        assert!(msg.contains("현재: 10자"));

        let valid_vw = validate_service_credential_key(
            ApiServiceType::VWorld,
            "  0123456789abcdef0123456789abcdef  ",
        )
        .expect("32-char vworld key");
        assert_eq!(valid_vw.len(), 32);

        let valid_ngii =
            validate_service_credential_key(ApiServiceType::NgiiTopographic, "0123456789abcdef")
                .expect("16-char ngii key");
        assert_eq!(valid_ngii.len(), 16);
    }
}
