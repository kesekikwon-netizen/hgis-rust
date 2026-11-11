//! 검증용: 연속지적도 받기의 사이트 연결을 한 단계씩 실제로 해 본다.
//! 아이디·비밀번호·키 값은 찍지 않는다(있는지 없는지만 찍는다).
//!
//! `cargo run -p strata-net --example cadastral_probe`            → 계정·키가 있는지만 확인
//! `cargo run -p strata-net --example cadastral_probe -- login`   → 로그인 1회 + 자료 목록 1쪽

use strata_net::portal_session::PortalSession;
use strata_net::vworld_client::{self, VworldClient};
use strata_net::vworld_login::{vworld_portal_login, CADASTRAL_DATASET_URL};
use strata_secrets::accounts::{AccountKind, AccountSettings};

fn main() {
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let dir = AccountSettings::user_config_dir();
    let creds = AccountSettings::read_from_dir(&dir, AccountKind::VworldCadastral);
    println!("config_dir_exists={}", dir.is_dir());
    println!("api_key_present={}", !key.trim().is_empty());
    println!("cadastral_account_ready={}", creds.is_ready());
    for (name, kind) in [
        ("ngii", AccountKind::NgiiTopographic),
        ("heritage", AccountKind::HeritageIntranet),
    ] {
        let c = AccountSettings::read_from_dir(&dir, kind);
        println!("{name}_account_ready={}", c.is_ready());
    }
    if std::env::args().nth(1).as_deref() != Some("login") {
        return;
    }
    if !creds.is_ready() {
        println!("login=skipped (계정 없음)");
        return;
    }
    let session = PortalSession::new(CADASTRAL_DATASET_URL);
    match vworld_portal_login(&session, &creds.username, &creds.password) {
        Ok(()) => println!("login=success"),
        Err(f) => {
            println!("login=failed account_rejected={} message={}", f.account_rejected, f.message);
            return;
        }
    }
    // 자료 목록 전체(쪽마다 100줄, 100줄 미만이면 끝).
    let mut resources = Vec::new();
    for page in 1..=10 {
        match session.get_text(&VworldClient::cadastral_dataset_page_url(page, 100)) {
            Ok(html) => {
                let rows = vworld_client::parse_cadastral_resources(&html);
                println!("catalog_page{page}_bytes={} zip_rows={}", html.len(), rows.len());
                let short = rows.len() < 100;
                resources.extend(rows);
                if short {
                    break;
                }
            }
            Err(e) => {
                println!("catalog=failed {e}");
                return;
            }
        }
    }
    println!("catalog_total={}", resources.len());

    // `login <min_lon> <min_lat> <max_lon> <max_lat> <출력 폴더>` 면 시·군·구 → 고르기 → ZIP 까지.
    let args: Vec<String> = std::env::args().skip(2).collect();
    let (Some(out_dir), Ok(bbox)) = (
        args.get(4),
        args.iter().take(4).map(|a| a.parse::<f64>()).collect::<Result<Vec<_>, _>>(),
    ) else {
        return;
    };
    if bbox.len() != 4 {
        return;
    }
    let url = VworldClient::districts_url(key.trim(), [bbox[0], bbox[1], bbox[2], bbox[3]]);
    let Ok(text) = session.get_text(&url) else {
        println!("districts=failed");
        return;
    };
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    let status = json["response"]["status"].as_str().unwrap_or("").to_owned();
    let features = json["response"]["result"]["featureCollection"]["features"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let districts: Vec<vworld_client::CadastralDistrict> = features
        .iter()
        .map(|f| vworld_client::CadastralDistrict {
            code: f["properties"]["sig_cd"].as_str().unwrap_or("").to_owned(),
            name: f["properties"]["full_nm"].as_str().unwrap_or("").to_owned(),
        })
        .collect();
    println!("districts_status={status} count={}", districts.len());
    for d in &districts {
        println!("  {} {}", d.code, d.name);
    }
    let files = match vworld_client::select_cadastral_resources(&resources, &districts) {
        Ok(files) => files,
        Err(e) => {
            println!("select=failed {e}");
            return;
        }
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for f in &files {
        let dest = std::path::Path::new(out_dir).join(vworld_client::cadastral_zip_file_name(f));
        let started = std::time::Instant::now();
        let result = session.download_zip(
            &VworldClient::cadastral_zip_url(&f.file_no),
            &dest,
            vworld_client::CADASTRAL_MAX_ZIP_KB * 1024,
            &cancel,
        );
        let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        println!(
            "download {} listed_kb={} got_bytes={size} secs={:.1} result={result:?}",
            f.name,
            f.size_kb,
            started.elapsed().as_secs_f32()
        );
        if result.is_err() {
            for line in session.response_headers(&VworldClient::cadastral_zip_url(&f.file_no)) {
                println!("  header: {line}");
            }
            // 원인 확인용: 같은 주소의 응답 앞부분만 본다(쿠키·계정 값은 들어 있지 않다).
            match session.get_text(&VworldClient::cadastral_zip_url(&f.file_no)) {
                Ok(body) => println!("  body_head={:?}", body.chars().take(240).collect::<String>()),
                Err(e) => println!("  get_failed={e}"),
            }
            break;
        }
    }
}
