//! 계정 비밀번호 암호화 — C++ `KaSecretStore`(core/KaSecretStore.cpp:36-93)와 같은 꼴. Windows 는 진짜 DPAPI
//! (CryptProtectData: CurrentUser, 엔트로피 ka-hgis-account-v1, 설명 "ka-hgis")만 쓰고 읽는다 — C++ 이 못 읽는 꼴은
//! Rust 도 못 읽는다(같은 「설정 필요」). 다른 OS 는 C++ `#else` 처럼 XOR. Windows 호출은 `dpapi_win`.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use strata_types::error::{StrataError, StrataResult};

/// KaSecretStore.cpp:37 kEntropy.
pub const ENTROPY: &[u8] = b"ka-hgis-account-v1";
/// CryptProtectData 꾸러미의 앞 8바이트(dwVersion 1 + 공급자 GUID 앞).
pub const WIN_DPAPI_HEADER: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0xD0, 0x8C, 0x9D, 0xDF];

/// xorEntropy (:39-45).
pub fn xor_entropy(mut data: Vec<u8>) -> Vec<u8> {
    let n = ENTROPY.len();
    for (i, b) in data.iter_mut().enumerate() {
        *b ^= ENTROPY[i % n];
    }
    data
}

/// 다 쓴 비밀 바이트를 0 으로(C++ secureWipe).
pub fn wipe(buf: &mut [u8]) {
    buf.fill(0);
    std::hint::black_box(&*buf);
}

/// protectBytes (:47-68) — 빈 값은 빈 글, 암호화하지 못하면 오류(평문으로 두지 않는다).
///
/// # Errors
/// DPAPI 암호화 실패.
pub fn protect_bytes(plain: &[u8]) -> StrataResult<String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    protect_verified_many(&[plain])
        .pop()
        .flatten()
        .ok_or_else(|| StrataError::Secret("비밀번호를 암호화하지 못했습니다.".into()))
}

/// 여러 값을 한 번에 암호화하고, 바로 풀어 같은지 확인한 꾸러미(Base64). 못 한 칸은 None.
#[cfg(windows)]
#[must_use]
pub fn protect_verified_many(plains: &[&[u8]]) -> Vec<Option<String>> {
    crate::dpapi_win::protect_verified_many(plains)
}

/// 다른 OS — C++ `#else` 의 XOR.
#[cfg(not(windows))]
#[must_use]
pub fn protect_verified_many(plains: &[&[u8]]) -> Vec<Option<String>> {
    plains.iter().map(|p| Some(BASE64.encode(xor_entropy(p.to_vec())))).collect()
}

/// unprotectBytes (:70-93).
///
/// # Errors
/// Base64 가 아니거나, 비었거나, (Windows) 이 사용자의 DPAPI 로 풀 수 없음.
pub fn unprotect_bytes(b64: &str) -> StrataResult<Vec<u8>> {
    let trimmed = b64.trim().trim_matches('"');
    let decoded = BASE64
        .decode(trimmed)
        .map_err(|e| StrataError::Secret(format!("Base64 복호화 실패: {e}")))?;
    if decoded.is_empty() {
        return Err(StrataError::Secret("복호화된 바이트가 비어 있습니다".into()));
    }
    unprotect_decoded(trimmed, decoded)
}

/// Windows — CryptUnprotectData 만. 진짜 DPAPI 꾸러미가 아니면 C++ 처럼 못 읽는다.
#[cfg(windows)]
fn unprotect_decoded(trimmed: &str, decoded: Vec<u8>) -> StrataResult<Vec<u8>> {
    if !decoded.starts_with(WIN_DPAPI_HEADER) {
        return Err(StrataError::Secret("Windows DPAPI 꾸러미가 아닙니다".into()));
    }
    crate::dpapi_win::unprotect_cached(trimmed)
}

#[cfg(not(windows))]
#[allow(clippy::unnecessary_wraps)]
fn unprotect_decoded(_trimmed: &str, decoded: Vec<u8>) -> StrataResult<Vec<u8>> {
    Ok(xor_entropy(decoded))
}

/// 여러 꾸러미를 한 번에 미리 풀어 둔다(앱 시작에 계정 파일 셋을 읽을 때 PowerShell 한 번).
pub fn prefetch_unprotect(values: &[String]) {
    #[cfg(windows)]
    crate::dpapi_win::prefetch(values);
    #[cfg(not(windows))]
    let _ = values;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 예전 Rust 꼴("DPAPI1:" + XOR)과 머리 없는 XOR 는 Windows 에서 C++ 처럼 못 읽는다.
    #[cfg(windows)]
    #[test]
    fn windows_reader_refuses_legacy_rust_and_raw_xor_forms() {
        let mut framed = b"DPAPI1:".to_vec();
        framed.extend(xor_entropy(b"FAKE-pw".to_vec()));
        assert!(unprotect_bytes(&BASE64.encode(framed)).is_err());
        assert!(unprotect_bytes(&BASE64.encode(xor_entropy(b"FAKE-pw".to_vec()))).is_err());
    }

    /// 다른 OS 는 C++ `#else` 처럼 XOR 로 쓰고 읽는다.
    #[cfg(not(windows))]
    #[test]
    fn xor_form_round_trips_off_windows() -> StrataResult<()> {
        let blob = protect_bytes(b"FAKE-pw")?;
        assert_eq!(BASE64.decode(&blob).unwrap_or_default(), xor_entropy(b"FAKE-pw".to_vec()));
        assert_eq!(unprotect_bytes(&blob)?, b"FAKE-pw");
        Ok(())
    }
}
