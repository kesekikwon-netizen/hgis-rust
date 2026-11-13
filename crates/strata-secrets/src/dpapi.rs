use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
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

/// 스크립트를 표준입력에서 읽는 PowerShell(명령줄에는 옵션만).
fn dpapi_powershell() -> Command {
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", "-"]);
    cmd
}

/// DPAPI(CurrentUser, 엔트로피 ka-hgis-account-v1)로 꾸러미를 풀어 Base64 로 내는 한 줄.
fn unprotect_script(trimmed_b64: &str) -> String {
    format!(
        "Add-Type -AssemblyName System.Security; \
         $b = [Convert]::FromBase64String('{trimmed_b64}'); \
         $e = [System.Text.Encoding]::ASCII.GetBytes('ka-hgis-account-v1'); \
         $p = [System.Security.Cryptography.ProtectedData]::Unprotect($b, $e, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
         [Convert]::ToBase64String($p)\n"
    )
}

fn unprotect_windows_dpapi_b64(trimmed_b64: &str) -> StrataResult<Vec<u8>> {
    if let Ok(guard) = dpapi_cache().lock() {
        if let Some(cached) = guard.get(trimmed_b64) {
            return Ok(cached.clone());
        }
    }
    // SEC1 — 암호화된 계정 값이 든 스크립트는 명령줄이 아니라 표준입력으로(`-Command -`).
    let output = (|| {
        let mut child = dpapi_powershell().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(unprotect_script(trimmed_b64).as_bytes())?;
        }
        child.wait_with_output()
    })()
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

    /// SEC1 — C++ 앱이 쓴 진짜 Windows DPAPI 꾸러미(가짜 비밀번호)를 풀고, PowerShell 명령줄에는 그 꾸러미가 없다.
    #[cfg(windows)]
    #[test]
    fn windows_dpapi_blob_unprotects_without_the_blob_on_the_command_line() -> StrataResult<()> {
        let protect = "Add-Type -AssemblyName System.Security; \
             $e = [System.Text.Encoding]::ASCII.GetBytes('ka-hgis-account-v1'); \
             $p = [System.Text.Encoding]::UTF8.GetBytes('FAKE-PW-1234'); \
             [Convert]::ToBase64String([System.Security.Cryptography.ProtectedData]::Protect($p, $e, 'CurrentUser'))";
        let out = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", protect]).output().expect("powershell");
        let blob = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        assert!(BASE64.decode(&blob).expect("b64").starts_with(WIN_DPAPI_HEADER));
        assert_eq!(unprotect_bytes(&blob)?, b"FAKE-PW-1234");
        let args: Vec<String> = dpapi_powershell().get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["-NoProfile", "-NonInteractive", "-Command", "-"]);
        assert!(!unprotect_script(&blob).is_empty());
        Ok(())
    }
}
