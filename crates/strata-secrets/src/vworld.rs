use crate::secret_store::{parse_qt_ini_file, serialize_qt_ini};
use std::collections::BTreeMap;
use std::path::Path;
use strata_types::error::StrataResult;

#[derive(Debug, Clone, Copy)]
pub struct VworldSettings;

impl VworldSettings {
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

    /// C++ `VworldSettings::loadApiKey` 의 순서(ini → 레지스트리 → 환경 변수 → 저장소 secrets.ini, `KeyStores`).
    #[must_use]
    pub fn load_api_key() -> String {
        crate::key_stores::KeyStores::user().api_key()
    }

    /// C++ `VworldSettings::loadHistoryGisApiKey` 의 순서(ini → 레지스트리 → 환경 변수).
    #[must_use]
    pub fn load_history_gis_api_key() -> String {
        crate::key_stores::KeyStores::user().history_key()
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
