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
}
