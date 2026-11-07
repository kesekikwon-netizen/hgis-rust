use crate::dpapi::{protect_bytes, unprotect_bytes, xor_entropy};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::collections::BTreeMap;
use std::path::Path;
use strata_types::error::{StrataError, StrataResult};

#[derive(Debug, Clone, Copy)]
pub struct SecretStore;

impl SecretStore {
    /// C++ `SecretStore::saveSecret(const QString &key, const QString &plainText) -> bool`
    pub fn save_secret(map: &mut BTreeMap<String, String>, key: &str, plain_text: &str) -> bool {
        if key.trim().is_empty() || plain_text.is_empty() {
            return false;
        }
        let encoded = Self::wrap_portable(plain_text);
        if encoded.is_empty() {
            return false;
        }
        map.insert(key.trim().to_owned(), encoded);
        true
    }

    /// C++ `SecretStore::loadSecret(const QString &key) -> std::optional<QString>`
    #[must_use]
    pub fn load_secret(map: &BTreeMap<String, String>, key: &str) -> Option<String> {
        let raw = map.get(key.trim())?;
        if raw.trim().is_empty() {
            return None;
        }
        Self::unwrap_portable(raw).ok()
    }

    /// C++ DPAPI / 자격 증명 오류 사용자 친화적 한글 메시지 매핑
    #[must_use]
    pub fn format_credential_error_ko(raw_err: &str) -> String {
        if raw_err.trim().is_empty() {
            "인증 키를 안전하게 저장할 수 없습니다 (Windows 자격 증명 오류)".to_owned()
        } else {
            format!("인증 키를 안전하게 저장할 수 없습니다 (Windows 자격 증명 오류: {raw_err})")
        }
    }

    pub fn wrap_portable(password: &str) -> String {
        if password.is_empty() {
            return String::new();
        }
        let wrapped = xor_entropy(password.as_bytes().to_vec());
        BASE64.encode(wrapped)
    }

    pub fn unwrap_portable(stored: &str) -> StrataResult<String> {
        let trimmed = stored.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }
        let raw = BASE64
            .decode(trimmed)
            .map_err(|e| StrataError::Secret(format!("포터블 비밀번호 Base64 오류: {e}")))?;
        let plain = xor_entropy(raw);
        String::from_utf8(plain)
            .map_err(|e| StrataError::Secret(format!("포터블 비밀번호 UTF-8 오류: {e}")))
    }

    pub fn write_password_to_map(
        map: &mut BTreeMap<String, String>,
        group: &str,
        password: &str,
        portable: bool,
    ) -> StrataResult<()> {
        let plain_k = format!("{group}/password");
        let dpapi_k = format!("{group}/password_dpapi");
        let port_k = format!("{group}/password_portable");
        map.remove(&plain_k);
        map.remove(&dpapi_k);
        map.remove(&port_k);
        if password.is_empty() {
            return Ok(());
        }
        if portable {
            map.insert(port_k, Self::wrap_portable(password));
        } else {
            let blob = protect_bytes(password.as_bytes())?;
            map.insert(dpapi_k, blob);
        }
        Ok(())
    }

    pub fn read_password_from_map(
        map: &BTreeMap<String, String>,
        group: &str,
    ) -> StrataResult<String> {
        let port_k = format!("{group}/password_portable");
        if let Some(v) = map.get(&port_k) {
            if !v.trim().is_empty() {
                return Self::unwrap_portable(v);
            }
        }
        let dpapi_k = format!("{group}/password_dpapi");
        if let Some(v) = map.get(&dpapi_k) {
            if !v.trim().is_empty() {
                let bytes = unprotect_bytes(v)?;
                return String::from_utf8(bytes)
                    .map_err(|e| StrataError::Secret(format!("비밀번호 UTF-8 오류: {e}")));
            }
        }
        let plain_k = format!("{group}/password");
        if let Some(v) = map.get(&plain_k) {
            return Ok(v.clone());
        }
        Ok(String::new())
    }

    pub fn has_undecryptable_password(map: &BTreeMap<String, String>, group: &str) -> bool {
        let port_k = format!("{group}/password_portable");
        if let Some(v) = map.get(&port_k) {
            if !v.trim().is_empty() && Self::unwrap_portable(v).is_ok() {
                return false;
            }
        }
        let plain_k = format!("{group}/password");
        if map.get(&plain_k).is_some_and(|v| !v.is_empty()) {
            return false;
        }
        let dpapi_k = format!("{group}/password_dpapi");
        if let Some(v) = map.get(&dpapi_k) {
            if !v.trim().is_empty() {
                return unprotect_bytes(v).is_err();
            }
        }
        false
    }
}

pub fn parse_qt_ini_file(path: &Path) -> StrataResult<BTreeMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_qt_ini_str(&content))
}

pub fn parse_qt_ini_str(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut current_section = String::new();
    for raw_line in content.lines() {
        let line = raw_line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').to_string();
            let full_key =
                if current_section.is_empty() || current_section.eq_ignore_ascii_case("General") {
                    key.to_string()
                } else {
                    format!("{current_section}/{key}")
                };
            map.insert(full_key, val);
        }
    }
    map
}

pub fn serialize_qt_ini(map: &BTreeMap<String, String>) -> String {
    let mut sections: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (full_key, val) in map {
        if let Some((sec, k)) = full_key.split_once('/') {
            sections
                .entry(sec.to_string())
                .or_default()
                .push((k.to_string(), val.clone()));
        } else {
            sections
                .entry("General".to_string())
                .or_default()
                .push((full_key.clone(), val.clone()));
        }
    }
    let mut out = String::new();
    for (sec, items) in sections {
        out.push_str(&format!("[{sec}]\n"));
        for (k, v) in items {
            out.push_str(&format!("{k}={v}\n"));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_xor_entropy_matches_cpp_ka_secret_store_byte_for_byte() -> StrataResult<()> {
        let wrapped = SecretStore::wrap_portable("MyFieldPass!2026");
        assert!(!wrapped.is_empty());
        let recovered = SecretStore::unwrap_portable(&wrapped)?;
        assert_eq!(recovered, "MyFieldPass!2026");
        Ok(())
    }

    #[test]
    fn test_secret_store_save_and_load_roundtrip_stores_base64_ciphertext_not_plaintext() {
        let mut store = BTreeMap::new();
        let ok = SecretStore::save_secret(&mut store, "VWORLD_API_KEY", "VWORLD_KEY_12345!@#");
        assert!(ok);
        let raw_stored = store
            .get("VWORLD_API_KEY")
            .expect("stored key must exist in map");
        assert_ne!(raw_stored, "VWORLD_KEY_12345!@#");
        assert_eq!(
            SecretStore::load_secret(&store, "VWORLD_API_KEY").as_deref(),
            Some("VWORLD_KEY_12345!@#")
        );
        assert_eq!(SecretStore::load_secret(&store, "MISSING_KEY"), None);
    }
}
