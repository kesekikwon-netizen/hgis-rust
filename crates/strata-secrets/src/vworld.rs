use crate::accounts::AccountSettings;
use crate::secret_store::{parse_qt_ini_file, serialize_qt_ini};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use strata_types::error::StrataResult;

#[derive(Debug, Clone, Copy)]
pub struct VworldSettings;

impl VworldSettings {
    pub fn default_ini_path() -> Option<PathBuf> {
        let dir = AccountSettings::user_config_dir();
        let candidate = dir.join("ka-hgis-vworld.ini");
        if candidate.is_file() {
            return Some(candidate);
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let legacy = PathBuf::from(&local_app_data)
                .join("ka-hgis")
                .join("ka-hgis-vworld.ini");
            if legacy.is_file() {
                return Some(legacy);
            }
            return Some(
                PathBuf::from(local_app_data)
                    .join("ka-hgis")
                    .join("ka-hgis")
                    .join("ka-hgis-vworld.ini"),
            );
        }
        Some(candidate)
    }

    pub fn load_from_ini(path: &Path) -> StrataResult<String> {
        if !path.is_file() {
            return Ok(String::new());
        }
        let map = parse_qt_ini_file(path)?;
        for candidate in ["VWorld/ApiKey", "vworld/apiKey", "apiKey"] {
            if let Some(val) = map.get(candidate) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return Ok(trimmed.to_string());
                }
            }
        }
        Ok(String::new())
    }

    pub fn load_api_key() -> String {
        if let Some(ini) = Self::default_ini_path() {
            if let Ok(k) = Self::load_from_ini(&ini) {
                if !k.is_empty() {
                    return k;
                }
            }
        }
        if let Ok(env_k) = std::env::var("VWORLD_API_KEY") {
            let trimmed = env_k.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        let repo_secrets = Path::new("config/secrets.ini");
        if let Ok(k) = Self::load_from_ini(repo_secrets) {
            if !k.is_empty() {
                return k;
            }
        }
        String::new()
    }

    pub fn load_history_gis_api_key() -> String {
        if let Some(ini) = Self::default_ini_path() {
            if let Ok(k) = Self::load_history_gis_from_ini(&ini) {
                if !k.is_empty() {
                    return k;
                }
            }
        }
        if let Ok(env_k) = std::env::var("HISTORY_GIS_API_KEY") {
            let trimmed = env_k.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        String::new()
    }

    pub fn save_to_ini(path: &Path, key: &str) -> StrataResult<()> {
        let mut map = if path.is_file() {
            parse_qt_ini_file(path).unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        map.remove("vworld/apiKey");
        if key.trim().is_empty() {
            map.remove("VWorld/ApiKey");
        } else {
            map.insert("VWorld/ApiKey".to_string(), key.trim().to_string());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serialize_qt_ini(&map))?;
        Ok(())
    }

    pub fn save_history_gis_to_ini(path: &Path, key: &str) -> StrataResult<()> {
        let mut map = if path.is_file() {
            parse_qt_ini_file(path).unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        if key.trim().is_empty() {
            map.remove("HistoryGis/ApiKey");
        } else {
            map.insert("HistoryGis/ApiKey".to_string(), key.trim().to_string());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serialize_qt_ini(&map))?;
        Ok(())
    }

    pub fn load_history_gis_from_ini(path: &Path) -> StrataResult<String> {
        if !path.is_file() {
            return Ok(String::new());
        }
        let map = parse_qt_ini_file(path)?;
        Ok(map
            .get("HistoryGis/ApiKey")
            .map(|s| s.trim().to_string())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::SecretMask;
    use tempfile::TempDir;

    #[test]
    fn vworld_key_load_priority_and_secret_masking() -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let ini_path = tmp.path().join("ka-hgis-vworld.ini");
        std::fs::write(&ini_path, "[VWorld]\nApiKey=TEST-VWORLD-KEY-999\n")?;
        let key = VworldSettings::load_from_ini(&ini_path)?;
        assert_eq!(key, "TEST-VWORLD-KEY-999");
        let masked = SecretMask::redact(
            "GET https://api.vworld.kr/req/wmts?key=TEST-VWORLD-KEY-999",
            &[&key],
        );
        assert!(!masked.contains("TEST-VWORLD-KEY-999"));
        assert!(masked.contains("***"));
        Ok(())
    }
}
