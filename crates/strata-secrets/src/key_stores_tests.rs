//! 키 찾는 순서 — 임시 ini·가짜 레지스트리·가짜 환경 변수만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;

use super::*;

fn none(_: &str) -> String {
    String::new()
}

/// loadApiKey: 사용자 ini → 레지스트리 → 환경 변수 → 저장소 secrets.ini. 앞에서 찾으면 뒤는 열지 않는다.
#[test]
fn api_key_follows_the_cpp_order() {
    let dir = tempfile::tempdir().unwrap();
    let ini = dir.path().join("ka-hgis-vworld.ini");
    let secrets = dir.path().join("secrets.ini");
    let asked = Cell::new(0);
    let registry = |sub: &str| {
        asked.set(asked.get() + 1);
        if sub == "VWorld" { "FAKE-REG".to_owned() } else { String::new() }
    };
    let env = |name: &str| if name == "VWORLD_API_KEY" { "FAKE-ENV".to_owned() } else { String::new() };
    std::fs::write(&ini, "[VWorld]\nApiKey=FAKE-INI\n").unwrap();
    let stores = KeyStores { ini: ini.clone(), registry: &registry, env: &env, repo_secrets: vec![secrets.clone()] };
    assert_eq!(stores.api_key(), "FAKE-INI");
    assert_eq!(asked.get(), 0, "the registry is not opened when the ini has the key");
    std::fs::write(&ini, "[General]\napiKey=FAKE-PLAIN\n").unwrap();
    assert_eq!(stores.api_key(), "FAKE-REG", "a bare apiKey in the user ini is not the cpp key");
    let stores = KeyStores { ini: ini.clone(), registry: &none, env: &env, repo_secrets: vec![secrets.clone()] };
    assert_eq!(stores.api_key(), "FAKE-ENV");
    std::fs::write(&secrets, "apiKey=FAKE-SECRETS\n").unwrap();
    let stores = KeyStores { ini, registry: &none, env: &none, repo_secrets: vec![dir.path().join("없음.ini"), secrets] };
    assert_eq!(stores.api_key(), "FAKE-SECRETS");
}

/// Windows 의 QSettings ini 는 키 대소문자를 가리지 않는다 — 옛 키 vworld/apiKey 도 같은 키.
#[test]
fn ini_key_case_does_not_matter() {
    let dir = tempfile::tempdir().unwrap();
    let ini = dir.path().join("ka-hgis-vworld.ini");
    std::fs::write(&ini, "[vworld]\napiKey= FAKE-OLD \n").unwrap();
    let stores = KeyStores { ini, registry: &none, env: &none, repo_secrets: Vec::new() };
    assert_eq!(stores.api_key(), "FAKE-OLD");
}

/// loadHistoryGisApiKey: 사용자 ini → 레지스트리 HistoryGis → 환경 변수(저장소 secrets.ini 는 보지 않는다).
#[test]
fn history_key_follows_the_cpp_order() {
    let dir = tempfile::tempdir().unwrap();
    let ini = dir.path().join("ka-hgis-vworld.ini");
    let secrets = dir.path().join("secrets.ini");
    std::fs::write(&secrets, "[HistoryGis]\nApiKey=FAKE-SECRETS\n").unwrap();
    let registry = |sub: &str| if sub == "HistoryGis" { "FAKE-REG".to_owned() } else { String::new() };
    let env = |name: &str| if name == "HISTORY_GIS_API_KEY" { "FAKE-ENV".to_owned() } else { String::new() };
    let stores = KeyStores { ini: ini.clone(), registry: &registry, env: &env, repo_secrets: vec![secrets.clone()] };
    assert_eq!(stores.history_key(), "FAKE-REG");
    std::fs::write(&ini, "[HistoryGis]\nApiKey=FAKE-INI\n").unwrap();
    assert_eq!(stores.history_key(), "FAKE-INI");
    let stores = KeyStores { ini: dir.path().join("없음.ini"), registry: &none, env: &env, repo_secrets: vec![secrets.clone()] };
    assert_eq!(stores.history_key(), "FAKE-ENV");
    let stores = KeyStores { ini: dir.path().join("없음.ini"), registry: &none, env: &none, repo_secrets: vec![secrets] };
    assert_eq!(stores.history_key(), "");
}

/// reg.exe 출력에서 REG_SZ 값만.
#[test]
fn registry_output_value() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\ka-hgis\\ka-hgis\\VWorld\r\n    ApiKey    REG_SZ    FAKE-REG-1\r\n\r\n";
    assert_eq!(reg_sz_value(out), "FAKE-REG-1");
    assert_eq!(reg_sz_value("ERROR: not found"), "");
}
