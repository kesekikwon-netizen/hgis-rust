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
        // KaSecretStore::writePassword (KaSecretStore.cpp:138-164) — 암호화하지 못하면 원래 칸들을 그대로 두고 오류.
        let plain_k = format!("{group}/password");
        let dpapi_k = format!("{group}/password_dpapi");
        let port_k = format!("{group}/password_portable");
        let (keep_k, value) = if password.is_empty() {
            (None, String::new())
        } else if portable {
            (Some(port_k.clone()), Self::wrap_portable(password))
        } else {
            let blob = protect_bytes(password.as_bytes())
                .ok()
                .filter(|b| !b.is_empty())
                .ok_or_else(|| StrataError::Secret("비밀번호를 암호화하지 못했습니다. 평문으로는 저장하지 않습니다.".into()))?;
            (Some(dpapi_k.clone()), blob)
        };
        map.remove(&plain_k);
        map.remove(&dpapi_k);
        map.remove(&port_k);
        if let Some(k) = keep_k {
            map.insert(k, value);
        }
        Ok(())
    }

    pub fn read_password_from_map(
        map: &BTreeMap<String, String>,
        group: &str,
    ) -> StrataResult<String> {
        // KaSecretStore::readPassword (KaSecretStore.cpp:166-190) — 포터블 → DPAPI(풀리면) → 평문 칸.
        let port_k = format!("{group}/password_portable");
        let portable = map.get(&port_k).map(|v| Self::unwrap_portable(v).unwrap_or_default()).unwrap_or_default();
        if !portable.is_empty() {
            return Ok(portable);
        }
        let dpapi_k = format!("{group}/password_dpapi");
        if let Some(v) = map.get(&dpapi_k).filter(|v| !v.trim().is_empty()) {
            if let Ok(bytes) = unprotect_bytes(v) {
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
        }
        Ok(map.get(&format!("{group}/password")).cloned().unwrap_or_default())
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

    /// KaSecretStore.cpp:170-184 — DPAPI 칸을 풀 수 없으면 평문 칸으로 넘어간다(앞에서 멈추지 않는다).
    #[cfg(windows)]
    #[test]
    fn read_falls_back_to_the_plain_key_when_dpapi_cannot_be_read() -> StrataResult<()> {
        let mut framed = b"DPAPI1:".to_vec();
        framed.extend(xor_entropy(b"FAKE-old".to_vec()));
        let mut map = BTreeMap::new();
        map.insert("ngii/password_dpapi".to_owned(), BASE64.encode(framed));
        assert_eq!(SecretStore::read_password_from_map(&map, "ngii")?, "");
        map.insert("ngii/password".to_owned(), "FAKE-plain".to_owned());
        assert_eq!(SecretStore::read_password_from_map(&map, "ngii")?, "FAKE-plain");
        SecretStore::write_password_to_map(&mut map, "ngii", "FAKE-new", false)?;
        assert!(!map.contains_key("ngii/password"), "writePassword clears the other two keys");
        assert_eq!(SecretStore::read_password_from_map(&map, "ngii")?, "FAKE-new");
        Ok(())
    }
}
