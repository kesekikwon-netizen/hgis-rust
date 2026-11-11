//! 국토정보플랫폼 자동 진행을 실제 사이트로 확인한다 — **신청서가 열리는 데까지만**.
//! 신청 제출(되돌릴 수 없음)은 하지 않는다. 계정 값은 출력하지 않는다.
//!
//! `cargo run -p strata-net --example ngii_portal_probe -- <ka-hgis-web.exe> <받을 폴더> <x5179> <y5179> <반경m>`

use std::path::PathBuf;

use serde_json::{json, Map};
use strata_net::ngii_portal_flow::{PortalFlow, Stage, PORTAL_PATH};
use strata_net::ngii_portal_runner::{PortalHelper, PortalInputs, PortalLogin, PORTAL_URL};
use strata_secrets::accounts::{AccountKind, AccountSettings};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [helper, downloads, x, y, radius] = args.as_slice() else {
        return Err("인자: <도우미> <받을 폴더> <x> <y> <반경>".to_owned());
    };
    let number = |s: &String| s.parse::<f64>().map_err(|_| "숫자가 아닙니다.".to_owned());
    let creds = AccountSettings::read_from_dir(&AccountSettings::user_config_dir(), AccountKind::NgiiTopographic);
    println!("저장된 계정: 아이디 {} · 비밀번호 {}", !creds.username.is_empty(), !creds.password.is_empty());
    let login = PortalLogin { id: creds.username, password: creds.password, attempt: 1 };

    let js_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/providers/ngii-topographic.js");
    let automation = std::fs::read_to_string(js_path).map_err(|e| format!("자동화 스크립트: {e}"))?;

    let mut scope = Map::new();
    scope.insert("x".to_owned(), json!(number(x)?));
    scope.insert("y".to_owned(), json!(number(y)?));
    scope.insert("radius".to_owned(), json!(number(radius)?));
    scope.insert("numbers".to_owned(), json!([]));
    scope.insert("cachedNumbers".to_owned(), json!([]));
    let mut flow = PortalFlow::new(scope, 1);

    let mut helper = PortalHelper::start(&PathBuf::from(helper), &PathBuf::from(downloads), std::env::var_os("KA_HGIS_WEB_SHOW").is_none())?;
    helper.open(PORTAL_URL)?;
    let started = std::time::Instant::now();
    let inputs = PortalInputs { automation_js: automation, login, birth_date: String::new(), purpose: String::new() };
    // 최대 60초만 돌린다(멈춘 화면을 오래 붙잡지 않는다).
    let mut last = String::new();
    while started.elapsed().as_secs() < 60 && !matches!(flow.stage, Stage::Form | Stage::Stopped | Stage::Complete) {
        helper.step(&mut flow, &inputs)?;
        let now = format!("단계={:?} 표시={} 안내={} | {}", flow.stage, flow.phase, flow.message, helper.last_note);
        if now != last {
            println!("{:>5.1}s {now}", started.elapsed().as_secs_f32());
            last = now;
        }
        std::thread::sleep(strata_net::ngii_portal_runner::POLL_INTERVAL);
    }
    if flow.stage == Stage::OpenForm {
        // 멈춘 로그인 화면의 모양만 읽는다(값·토큰은 읽지 않는다).
        let script = "(function(){var host=function(u){try{return new URL(u,location.href).origin+new URL(u,location.href).pathname;}catch(e){return '?';}};            return {ready:document.readyState,title:document.title,text:(document.body?document.body.innerText:'').split('\\n').join(' ').slice(0,200),            forms:Array.from(document.forms).map(function(f){return {action:host(f.action),method:f.method,target:f.target,fields:f.elements.length};}),            frames:Array.from(document.querySelectorAll('iframe')).map(function(f){return host(f.src);}),            scripts:Array.from(document.scripts).map(function(x){return x.src?host(x.src):'inline:'+x.textContent.slice(0,900);})};})()";
        for path in strata_net::ngii_portal_flow::LOGIN_PATHS {
            if let Some(answer) = helper.eval(&[path.to_owned()], script)? {
                if answer["found"] == true {
                    println!("멈춘 화면 {path}: {}", answer["result"]);
                }
            }
        }
    }
    println!("끝: 단계={:?} 중단={:?} 신청할 자료={}", flow.stage, flow.failed, flow.records.len());
    for r in &flow.records {
        println!("  도엽 {} {} (이력 {})", r["num"], r["name"], r["historyNo"]);
    }
    if flow.stage == Stage::Form {
        // 신청서의 항목 이름만 읽는다(값은 읽지 않는다).
        let script = "(function(){var f=document.querySelector('.popup.mapApplication.online');if(!f)return null;\
            return Array.from(f.querySelectorAll('input,select,button')).map(function(e){return e.tagName+':'+(e.name||e.id||'')+':'+(e.type||'')+':'+((e.tagName==='BUTTON'?e.textContent:'')||'').trim();});})()";
        let answer = helper.eval(&[PORTAL_PATH.to_owned()], script)?;
        println!("신청서 항목: {}", answer.map(|a| a["result"].to_string()).unwrap_or_default());
    }
    Ok(())
}
