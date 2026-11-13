//! 자료 연결 다섯 곳 — 임시 저장소·가짜 값만(사용자 설정·레지스트리·계정 파일은 읽지 않는다).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::accounts::Credentials;

fn none(_: &str) -> String {
    String::new()
}

/// ported from tests/test_account_status.cpp:listsEverySourceInMenuOrder.
#[test]
fn lists_every_source_in_menu_order() {
    let entries = snapshot(&|_| false);
    let sources: Vec<Source> = entries.iter().map(|e| e.source).collect();
    assert_eq!(sources, [Source::VworldKey, Source::HistoryGisKey, Source::CadastralAccount, Source::TopographicAccount, Source::HeritageAccount]);
    for e in &entries {
        assert!(!e.label.is_empty() && !e.used_for.is_empty());
        assert!(e.menu_path.starts_with("더보기 → "), "{}", e.menu_path);
        assert!(!e.ready);
    }
    assert_eq!(entries[3].label, "수치지형도 계정");
    assert_eq!(entries[3].menu_path, "더보기 → 수치지형도 아이디·비밀번호");
}

fn stores<'a>(dir: &std::path::Path, app: &std::path::Path, registry: &'a dyn Fn(&str) -> String) -> Stores<'a> {
    Stores {
        keys: KeyStores { ini: dir.join("ka-hgis-vworld.ini"), registry, env: &none, repo_secrets: Vec::new() },
        config_dir: dir.to_path_buf(),
        app_dir: app.to_path_buf(),
    }
}

/// 빈 저장소는 다섯 곳 모두 「설정 필요」, 키는 ini·레지스트리 어느 쪽이든 「설정됨」.
#[test]
fn keys_are_ready_from_any_cpp_store() {
    let dir = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    let s = stores(dir.path(), app.path(), &none);
    assert!(s.snapshot().iter().all(|e| !e.ready));
    std::fs::write(dir.path().join("ka-hgis-vworld.ini"), "[VWorld]\nApiKey=FAKE\n").unwrap();
    let reg = |sub: &str| if sub == "HistoryGis" { "FAKE".to_owned() } else { String::new() };
    let s = stores(dir.path(), app.path(), &reg);
    let ready: Vec<bool> = s.snapshot().iter().map(|e| e.ready).collect();
    assert_eq!(ready, [true, true, false, false, false]);
}

/// CadastralPortal::hasCredentials — vworld-account.ini 의 아이디(빈칸 뺌)와 읽히는 비밀번호가 둘 다 있어야.
#[test]
fn cadastral_needs_a_name_and_a_readable_password() {
    let dir = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    let path = dir.path().join("vworld-account.ini");
    let s = stores(dir.path(), app.path(), &none);
    std::fs::write(&path, "[vworld_account]\nusername=  \npassword=FAKE\n").unwrap();
    assert!(!s.ready(Source::CadastralAccount), "blank name");
    std::fs::write(&path, "[vworld_account]\nusername=fake\n").unwrap();
    assert!(!s.ready(Source::CadastralAccount), "no password");
    AccountSettings::save_to_file(&path, AccountKind::VworldCadastral, &Credentials::new("fake", "FAKE-pw"), true).unwrap();
    assert!(s.ready(Source::CadastralAccount));
    std::fs::write(dir.path().join("cadastral-account.ini"), "[cadastral]\nusername=fake\npassword=FAKE\n").unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(!s.ready(Source::CadastralAccount), "cpp reads only vworld-account.ini");
}

/// Topographic/HeritageIntranetSettings::hasCredentials — 개인 파일이 있으면 그것(빈 값 = 끔), 없으면 앱 옆 config 의 번들 계정.
#[test]
fn site_accounts_use_the_personal_file_then_the_bundled_one() {
    let dir = tempfile::tempdir().unwrap();
    let app = tempfile::tempdir().unwrap();
    let s = stores(dir.path(), app.path(), &none);
    std::fs::create_dir_all(app.path().join("config")).unwrap();
    std::fs::write(app.path().join("config").join("ngii-local.ini"), "[ngii]\nusername=bundled\npassword=FAKE\n").unwrap();
    assert!(s.ready(Source::TopographicAccount), "bundled account");
    std::fs::write(dir.path().join("ngii-account.ini"), "[ngii]\nusername=\n").unwrap();
    assert!(!s.ready(Source::TopographicAccount), "an empty personal file turns the default login off");
    AccountSettings::save_to_file(&dir.path().join("heritage-account.ini"), AccountKind::HeritageIntranet, &Credentials::new("fake", "FAKE-pw"), true).unwrap();
    assert!(s.ready(Source::HeritageAccount));
}
