use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use strata_types::error::{StrataError, StrataResult};

pub const ENTROPY: &[u8] = b"ka-hgis-account-v1";
pub const DPAPI_BLOB_PREFIX: &[u8] = b"DPAPI1:";

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

pub fn unprotect_bytes(b64: &str) -> StrataResult<Vec<u8>> {
    let trimmed = b64.trim();
    if trimmed.is_empty() {
        return Err(StrataError::Secret("빈 암호화 데이터".into()));
    }
    let decoded = BASE64
        .decode(trimmed)
        .map_err(|e| StrataError::Secret(format!("Base64 복호화 실패: {e}")))?;
    if decoded.is_empty() {
        return Err(StrataError::Secret("복호화된 바이트가 비어 있습니다".into()));
    }
    let payload = decoded
        .strip_prefix(DPAPI_BLOB_PREFIX)
        .unwrap_or(decoded.as_slice());
    if payload.is_empty() {
        return Err(StrataError::Secret("DPAPI 페이로드가 비어 있습니다".into()));
    }
    Ok(xor_entropy(payload.to_vec()))
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
