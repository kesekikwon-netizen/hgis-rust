//! Windows DPAPI 를 PowerShell 로 — C++ `protectBytes`/`unprotectBytes`(KaSecretStore.cpp:47-93)와 같은 호출
//! (CurrentUser, 엔트로피 ka-hgis-account-v1, CRYPTPROTECT_UI_FORBIDDEN, 암호화 설명 "ka-hgis"). SEC1: 스크립트와 값은
//! 표준입력으로만(`-Command -`, 명령줄에는 옵션뿐). 표준입력은 한 줄씩 실행되므로 줄마다 끝나는 문장으로 쓴다.
//! 값은 Base64 로만 넣는다(비밀번호의 따옴표가 스크립트를 깨지 않게).

use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use strata_types::error::{StrataError, StrataResult};

use crate::dpapi::{wipe, WIN_DPAPI_HEADER};

fn cache() -> &'static Mutex<BTreeMap<String, Vec<u8>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// 스크립트를 표준입력에서 읽는 PowerShell(명령줄에는 옵션만).
pub(crate) fn dpapi_powershell() -> Command {
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", "-"]);
    cmd
}

/// CryptProtectData 를 설명 "ka-hgis"·플래그 1(UI 금지)로 부르는 형 — .NET `ProtectedData.Protect` 는 설명이 비어 C++ 과 다르다.
const PROTECT_TYPE: &str = "Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; \
public static class KaDpapi { [StructLayout(LayoutKind.Sequential)] public struct Blob { public int Cb; public IntPtr Pb; } \
[DllImport(\"crypt32.dll\", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool CryptProtectData(ref Blob input, string description, ref Blob entropy, IntPtr reserved, IntPtr prompt, int flags, ref Blob output); \
[DllImport(\"kernel32.dll\")] static extern IntPtr LocalFree(IntPtr h); \
public static byte[] Protect(byte[] plain, byte[] ent) { Blob i = new Blob(), e = new Blob(), o = new Blob(); \
i.Cb = plain.Length; i.Pb = Marshal.AllocHGlobal(Math.Max(1, plain.Length)); Marshal.Copy(plain, 0, i.Pb, plain.Length); \
e.Cb = ent.Length; e.Pb = Marshal.AllocHGlobal(ent.Length); Marshal.Copy(ent, 0, e.Pb, ent.Length); \
try { if (!CryptProtectData(ref i, \"ka-hgis\", ref e, IntPtr.Zero, IntPtr.Zero, 1, ref o) || o.Cb == 0) throw new Exception(\"protect\"); \
byte[] r = new byte[o.Cb]; Marshal.Copy(o.Pb, r, 0, o.Cb); LocalFree(o.Pb); return r; } \
finally { Marshal.Copy(new byte[plain.Length], 0, i.Pb, plain.Length); Marshal.FreeHGlobal(i.Pb); Marshal.FreeHGlobal(e.Pb); } } }'";

const ENTROPY_LINE: &str = "$e = [System.Text.Encoding]::ASCII.GetBytes('ka-hgis-account-v1')";

fn quoted_list(items: &[String]) -> String {
    items.iter().map(|s| format!("'{s}'")).collect::<Vec<_>>().join(",")
}

/// 값(Base64)마다 암호화 → 바로 풀어 같으면 「+꾸러미」, 아니면 「-」를 한 줄씩.
fn protect_script(plain_b64: &[String]) -> String {
    format!(
        "Add-Type -AssemblyName System.Security\n{PROTECT_TYPE}\n{ENTROPY_LINE}\n\
         foreach ($x in @({})) {{ try {{ $p = [Convert]::FromBase64String($x); $b = [KaDpapi]::Protect($p, $e); \
         $q = [System.Security.Cryptography.ProtectedData]::Unprotect($b, $e, 'CurrentUser'); \
         if ([Convert]::ToBase64String($q) -ceq $x) {{ '+' + [Convert]::ToBase64String($b) }} else {{ '-' }}; \
         [Array]::Clear($p, 0, $p.Length); [Array]::Clear($q, 0, $q.Length) }} catch {{ '-' }} }}\n",
        quoted_list(plain_b64)
    )
}

/// 꾸러미마다 「+평문 Base64」, 못 풀면 「-」를 한 줄씩.
pub(crate) fn unprotect_script(blobs: &[String]) -> String {
    format!(
        "Add-Type -AssemblyName System.Security\n{ENTROPY_LINE}\n\
         foreach ($x in @({})) {{ try {{ $p = [System.Security.Cryptography.ProtectedData]::Unprotect([Convert]::FromBase64String($x), $e, 'CurrentUser'); \
         '+' + [Convert]::ToBase64String($p); [Array]::Clear($p, 0, $p.Length) }} catch {{ '-' }} }}\n",
        quoted_list(blobs)
    )
}

/// 스크립트를 돌려 줄마다 「+값」은 Some(바이트), 그 밖은 None — 끝나지 못했거나 줄 수가 다르면 모두 None.
fn run_lines(script: &[u8], n: usize) -> Vec<Option<Vec<u8>>> {
    let output = (|| {
        let mut child = dpapi_powershell().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(script)?;
        }
        child.wait_with_output()
    })();
    let Ok(mut out) = output else { return vec![None; n] };
    let parsed: Vec<Option<Vec<u8>>> = {
        let text = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if out.status.success() && lines.len() == n {
            lines.iter().map(|l| l.strip_prefix('+').and_then(|b| BASE64.decode(b).ok())).collect()
        } else {
            vec![None; n]
        }
    };
    wipe(&mut out.stdout);
    parsed
}

/// 암호화 후 같은 프로세스에서 풀어 확인한 꾸러미(DPAPI 머리가 있는 것만).
pub(crate) fn protect_verified_many(plains: &[&[u8]]) -> Vec<Option<String>> {
    if plains.is_empty() {
        return Vec::new();
    }
    let items: Vec<String> = plains.iter().map(|p| BASE64.encode(p)).collect();
    let mut script = protect_script(&items).into_bytes();
    let out = run_lines(&script, plains.len());
    wipe(&mut script);
    for item in items {
        wipe(&mut item.into_bytes());
    }
    out.into_iter().map(|b| b.filter(|b| b.starts_with(WIN_DPAPI_HEADER)).map(|b| BASE64.encode(b))).collect()
}

/// 한 꾸러미 풀기(이미 푼 것은 기억에서).
pub(crate) fn unprotect_cached(trimmed: &str) -> StrataResult<Vec<u8>> {
    if let Some(hit) = cache().lock().ok().and_then(|g| g.get(trimmed).cloned()) {
        return Ok(hit);
    }
    let plain = run_lines(unprotect_script(&[trimmed.to_owned()]).as_bytes(), 1)
        .pop()
        .flatten()
        .ok_or_else(|| StrataError::Secret("Windows DPAPI 복호화 거부".into()))?;
    if let Ok(mut g) = cache().lock() {
        g.insert(trimmed.to_owned(), plain.clone());
    }
    Ok(plain)
}

/// 아직 풀지 않은 진짜 DPAPI 꾸러미들을 PowerShell 한 번으로 풀어 기억해 둔다.
pub(crate) fn prefetch(values: &[String]) {
    let mut todo: Vec<String> = values
        .iter()
        .map(|v| v.trim().trim_matches('"').to_owned())
        .filter(|v| BASE64.decode(v).is_ok_and(|b| b.starts_with(WIN_DPAPI_HEADER)))
        .collect();
    todo.sort();
    todo.dedup();
    if let Ok(g) = cache().lock() {
        todo.retain(|v| !g.contains_key(v));
    }
    if todo.is_empty() {
        return;
    }
    let plains = run_lines(unprotect_script(&todo).as_bytes(), todo.len());
    if let Ok(mut g) = cache().lock() {
        for (k, p) in todo.into_iter().zip(plains) {
            if let Some(p) = p {
                g.insert(k, p);
            }
        }
    }
}

#[cfg(test)]
#[path = "dpapi_win_tests.rs"]
mod tests;
