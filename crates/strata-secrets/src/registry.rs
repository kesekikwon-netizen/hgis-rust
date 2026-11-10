//! Windows 레지스트리에서 API 키를 읽는다. C++ `VworldSettings::loadApiKey` 대응.
//!
//! ## 왜 필요한가
//!
//! C++ 은 `QSettings("ka-hgis","ka-hgis")` 로 **레지스트리**를 먼저 읽는다
//! (`VworldSettings.cpp` `openNativeStore`). 실제 키도 거기에 있다:
//! `HKCU\Software\ka-hgis\ka-hgis\vworld\ApiKey`.
//!
//! 그런데 Rust 는 ini 파일만 봤다. 그래서 C++ 앱에서는 위성지도가 나오는데
//! Rust 앱에서는 **타일이 안 나오는** 상태였다(2026-11-10 사용자 지적 "데이터가 안 온다").
//!
//! ## 읽는 순서 (C++ 과 같게)
//!
//! 1. `VWorld/ApiKey` (새 형식)
//! 2. `vworld/ApiKey` (옛 형식 — Qt 는 키 이름의 대소문자를 구분하지 않는다)
//! 레지스트리는 **읽기만** 한다. 쓰기는 ini 로 한다(기존 동작 유지).

/// VWorld 키를 레지스트리에서 읽는다. 없으면 빈 문자열.
///
/// Windows 가 아니면 언제나 빈 문자열(레지스트리가 없다).
#[must_use]
pub fn read_vworld_key() -> String {
    read_value("vworld", "ApiKey")
        .or_else(|| read_value("VWorld", "ApiKey"))
        .unwrap_or_default()
}

/// 이력지리정보(역사지리DB) 키를 레지스트리에서 읽는다.
#[must_use]
pub fn read_history_gis_key() -> String {
    read_value("HistoryGis", "ApiKey")
        .or_else(|| read_value("historygis", "ApiKey"))
        .unwrap_or_default()
}

/// `HKCU\Software\ka-hgis\ka-hgis\<sub>\<name>` 을 읽는다.
#[cfg(windows)]
fn read_value(sub: &str, name: &str) -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // C++ 은 `Software\ka-hgis\ka-hgis` 를 앱 이름으로 쓴다.
    let path = format!(r"Software\ka-hgis\ka-hgis\{sub}");
    let key = hkcu.open_subkey_with_flags(&path, KEY_READ).ok()?;
    let v: String = key.get_value(name).ok()?;
    let v = v.trim().to_owned();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Windows 가 아닌 곳에서는 레지스트리가 없다.
#[cfg(not(windows))]
fn read_value(_sub: &str, _name: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 레지스트리를 읽는다 — 이 PC 에 C++ 앱이 키를 넣어 뒀다면 값이 나와야 한다.
    ///
    /// 값이 없는 PC(CI 등)에서는 빈 문자열이 정상이므로 **패닉하지 않는 것**만 본다.
    #[test]
    fn reading_never_panics_and_trims() {
        let k = read_vworld_key();
        assert_eq!(k, k.trim(), "앞뒤 공백이 없어야 한다");
        let h = read_history_gis_key();
        assert_eq!(h, h.trim());
    }

    /// 이 PC 에 C++ 키가 있으면 VWorld 키는 UUID 형식(36자)이어야 한다.
    ///
    /// 이 시험이 통과한다는 것은 Rust 가 레지스트리를 **실제로 읽는다**는 뜻이다.
    /// (키가 없는 PC 에서는 건너뛴다.)
    #[test]
    fn vworld_key_has_expected_shape_when_present() {
        let k = read_vworld_key();
        if k.is_empty() {
            return; // 이 PC 에 키가 없다
        }
        assert_eq!(k.len(), 36, "VWorld 키는 36자 UUID 형식: {k}");
        assert_eq!(
            k.matches('-').count(),
            4,
            "UUID 형식(하이픈 4개): {k}"
        );
    }
}
