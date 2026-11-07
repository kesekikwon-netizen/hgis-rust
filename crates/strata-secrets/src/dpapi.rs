use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use strata_types::error::{StrataError, StrataResult};

pub const ENTROPY: &[u8] = b"ka-hgis-account-v1";
pub const DPAPI_BLOB_PREFIX: &[u8] = b"DPAPI1:";
const WIN_DPAPI_HEADER: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0xD0, 0x8C, 0x9D, 0xDF];

fn dpapi_cache() -> &'static Mutex<BTreeMap<String, Vec<u8>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub fn xor_entropy(mut data: Vec<u8>) -> Vec<u8> {
    let n = ENTROPY.len();
    for (i, b) in data.iter_mut().enumerate() {
        *b ^= ENTROPY[i % n];
    }
    data
}

pub fn protect_bytes(plain: &[u8]) -> StrataResult<String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    let mut framed = Vec::with_capacity(DPAPI_BLOB_PREFIX.len() + plain.len());
    framed.extend_from_slice(DPAPI_BLOB_PREFIX);
    framed.extend_from_slice(&xor_entropy(plain.to_vec()));
    Ok(BASE64.encode(framed))
}

fn unprotect_windows_dpapi_b64(trimmed_b64: &str) -> StrataResult<Vec<u8>> {
    if let Ok(guard) = dpapi_cache().lock() {
        if let Some(cached) = guard.get(trimmed_b64) {
            return Ok(cached.clone());
        }
    }
    let script = format!(
        "Add-Type -AssemblyName System.Security; \
         $b = [Convert]::FromBase64String('{trimmed_b64}'); \
         $e = [System.Text.Encoding]::ASCII.GetBytes('ka-hgis-account-v1'); \
         $p = [System.Security.Cryptography.ProtectedData]::Unprotect($b, $e, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
         [Convert]::ToBase64String($p)"
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| StrataError::Secret(format!("Windows DPAPI 호출 실패: {e}")))?;
    if !output.status.success() {
        return Err(StrataError::Secret("Windows DPAPI 복호화 거부".into()));
    }
    let out_b64 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let plain = BASE64
        .decode(&out_b64)
        .map_err(|e| StrataError::Secret(format!("DPAPI 평문 Base64 오류: {e}")))?;
    if let Ok(mut guard) = dpapi_cache().lock() {
        guard.insert(trimmed_b64.to_string(), plain.clone());
    }
    Ok(plain)
}

pub fn unprotect_bytes(b64: &str) -> StrataResult<Vec<u8>> {
    let trimmed = b64.trim().trim_matches('"');
    if trimmed.is_empty() {
        return Err(StrataError::Secret("빈 암호화 데이터".into()));
    }
    let decoded = BASE64
        .decode(trimmed)
        .map_err(|e| StrataError::Secret(format!("Base64 복호화 실패: {e}")))?;
    if decoded.is_empty() {
        return Err(StrataError::Secret(
            "복호화된 바이트가 비어 있습니다".into(),
        ));
    }
    if let Some(payload) = decoded.strip_prefix(DPAPI_BLOB_PREFIX) {
        if payload.is_empty() {
            return Err(StrataError::Secret("DPAPI 페이로드가 비어 있습니다".into()));
        }
        return Ok(xor_entropy(payload.to_vec()));
    }
    if decoded.starts_with(WIN_DPAPI_HEADER) {
        return unprotect_windows_dpapi_b64(trimmed);
    }
    Ok(xor_entropy(decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_framed_blob_and_legacy_raw_blob_both_unprotect_cleanly() -> StrataResult<()> {
        let secret = b"HeritagePass#2026";
        let framed_b64 = protect_bytes(secret)?;
        let recovered = unprotect_bytes(&framed_b64)?;
        assert_eq!(recovered, secret);

        let legacy_b64 = BASE64.encode(xor_entropy(secret.to_vec()));
        let recovered_legacy = unprotect_bytes(&legacy_b64)?;
        assert_eq!(recovered_legacy, secret);
        Ok(())
    }
}
