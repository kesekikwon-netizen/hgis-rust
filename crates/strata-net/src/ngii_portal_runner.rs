//! 웹 화면 도우미(`ka-hgis-web`)를 띄워 국토정보플랫폼 자동 진행을 돌린다.
//!
//! C++ 은 QWebEngineView 안에서 0.4초 타이머로 `ngii-topographic.js` 를 실행한다
//! (`KaTopographicBrowser.cpp:728`). 여기서는 도우미에게 표준 입력으로 스크립트를 보내고
//! 표준 출력으로 답을 받는다. 계정 값은 로그인 화면에서 돌 스크립트 안에만 실려 가고,
//! 어디에도 적지 않는다.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::ngii_portal_flow::{PortalFlow, Stage, LOGIN_PATHS, PORTAL_PATH};

/// 자동 진행 간격. C++ `m_automationTimer` 400 ms.
pub const POLL_INTERVAL: Duration = Duration::from_millis(400);
pub const PORTAL_URL: &str = "https://map.ngii.go.kr/ms/map/NlipMap.do";
/// 믿는 사이트(국토지리정보원과 그 하위 도메인).
pub const PORTAL_HOST: &str = "ngii.go.kr";

/// 공식 로그인이 지나가는 정부 통합인증 주소(2026-11-11 실제 확인). 지나가기만 허용한다 —
/// 그 화면에서는 스크립트도 계정 입력도 하지 않는다.
pub const AUTH_PASS_THROUGH_HOSTS: [&str; 1] = ["sso.anyid.go.kr"];

/// 로그인 화면에만 넘기는 값.
#[derive(Clone)]
pub struct PortalLogin {
    pub id: String,
    pub password: String,
    pub attempt: u64,
}

impl std::fmt::Debug for PortalLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 계정 값은 디버그 출력에도 내지 않는다.
        f.debug_struct("PortalLogin").field("attempt", &self.attempt).finish_non_exhaustive()
    }
}

/// 자동 진행에 넣는 값들.
#[derive(Debug, Clone)]
pub struct PortalInputs {
    /// `data/providers/ngii-*.js` 의 내용.
    pub automation_js: String,
    pub login: PortalLogin,
    pub birth_date: String,
    pub purpose: String,
}

/// 화면에서 돌릴 스크립트: 로그인 화면이면 `login` 명령과 계정을, 아니면 지금 단계를 넘긴다.
/// 돌려주는 값은 `{command, result}`.
#[must_use]
pub fn wrap_script(automation_js: &str, input: &Value, login: &PortalLogin) -> String {
    let login_input = json!({
        "command": "login",
        "loginId": login.id,
        "loginPassword": login.password,
        "loginAttempt": login.attempt.to_string(),
    });
    format!(
        "(function(){{var input={input};var official=location.protocol==='https:'&&(location.hostname==={host}||location.hostname.slice(-{dot_len})==={dot_host});if(!official)return {{command:'',result:{{}},error:'origin'}};if({paths}.indexOf(location.pathname)>=0){{var l={login_input};for(var k in l)input[k]=l[k];}}\nvar fresh=false;if(location.pathname==={portal}&&!window.__kaPageMark){{window.__kaPageMark=1;return {{command:input.command,fresh:true,result:{{}}}};}}\ntry{{var r=\n{automation_js}\n(input);return {{command:input.command,fresh:fresh,result:r||{{}}}};}}catch(e){{return {{command:input.command,result:{{}},error:String(e&&e.message)}};}}}})()",
        paths = json!(LOGIN_PATHS),
        portal = json!(PORTAL_PATH),
        host = json!(PORTAL_HOST),
        dot_host = json!(format!(".{PORTAL_HOST}")),
        dot_len = PORTAL_HOST.len() + 1,
    )
}

/// 공개DEM 용 자동화: 도엽 고르기·신청서 열기는 `ngii-dem.js`, 로그인은 수치지형도 스크립트가 맡는다.
#[must_use]
pub fn dem_automation(topographic_js: &str, dem_js: &str) -> String {
    format!(
        "(function(input){{if(input.command==='login')return(
{topographic_js}
)(input);return(
{dem_js}
)(input);}})"
    )
}

/// 띄워 둔 도우미.
#[derive(Debug)]
pub struct PortalHelper {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<Value>,
    next_id: u64,
    /// 마지막 답의 요약(어느 화면·명령·상태·오류) — 원인 찾기용. 계정 값은 없다.
    pub last_note: String,
}

impl PortalHelper {
    /// 도우미를 띄우고 준비될 때까지 기다린다.
    pub fn start(helper_exe: &Path, downloads: &Path, hidden: bool) -> Result<Self, String> {
        let mut command = Command::new(helper_exe);
        command.args(["--allow", PORTAL_HOST]);
        for host in AUTH_PASS_THROUGH_HOSTS {
            command.args(["--allow-nav", host]);
        }
        command.arg("--downloads").arg(downloads);
        if hidden {
            command.arg("--hidden");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW — 검은 창을 띄우지 않는다
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // 원인 찾기: `KA_HGIS_WEB_STDERR=1` 이면 도우미의 오류 출력을 그대로 보인다.
            .stderr(if std::env::var_os("KA_HGIS_WEB_STDERR").is_some() { Stdio::inherit() } else { Stdio::null() })
            .spawn()
            .map_err(|e| format!("웹 화면 도우미를 시작하지 못했습니다: {e}"))?;
        let stdin = child.stdin.take().ok_or("도우미 입력을 열지 못했습니다.")?;
        let stdout = child.stdout.take().ok_or("도우미 출력을 열지 못했습니다.")?;
        let (sender, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    // 원인 찾기: 도우미가 알린 오류(막은 이동 등)를 보인다.
                    if value["event"] == "error" && std::env::var_os("KA_HGIS_WEB_STDERR").is_some() {
                        eprintln!("도우미: {}", value["message"]);
                    }
                    if sender.send(value).is_err() {
                        break;
                    }
                }
            }
        });
        let mut helper = Self { child, stdin, events, next_id: 0, last_note: String::new() };
        match helper.wait_event(Duration::from_secs(30), |e| e["event"] == "ready" || e["event"] == "error") {
            Some(e) if e["event"] == "ready" => Ok(helper),
            _ => Err("웹 화면 도우미가 준비되지 않았습니다(WebView2 를 확인하세요).".to_owned()),
        }
    }

    fn send(&mut self, command: &Value) -> Result<(), String> {
        writeln!(self.stdin, "{command}")
            .and_then(|()| self.stdin.flush())
            .map_err(|_| "웹 화면 도우미가 닫혔습니다.".to_owned())
    }

    fn wait_event(&mut self, timeout: Duration, want: impl Fn(&Value) -> bool) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.checked_duration_since(Instant::now())?;
            let event = self.events.recv_timeout(left).ok()?;
            if want(&event) {
                return Some(event);
            }
        }
    }

    pub fn open(&mut self, url: &str) -> Result<(), String> {
        self.send(&json!({"cmd": "open", "url": url}))
    }

    /// 스크립트를 돌리고 답(`found`, `result`)을 받는다. 답이 없으면 `None`.
    pub fn eval(&mut self, paths: &[String], script: &str) -> Result<Option<Value>, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"cmd": "eval", "id": id, "paths": paths, "script": script}))?;
        Ok(self.wait_event(Duration::from_secs(20), |e| e["event"] == "eval" && e["id"] == id))
    }

    /// 한 차례: 지금 단계를 화면에 시키고 답으로 단계를 옮긴다.
    pub fn step(&mut self, flow: &mut PortalFlow, inputs: &PortalInputs) -> Result<(), String> {
        let Some(request) = flow.poll(&inputs.birth_date, &inputs.purpose) else {
            return Ok(());
        };
        // 스크립트 실행은 도우미가 믿는 사이트(국토지리정보원)의 화면에서만 한다.
        let script = wrap_script(&inputs.automation_js, &request.input, &inputs.login);
        match self.eval(&request.paths, &script)? {
            Some(answer) if answer["found"] == true => {
                let command = answer["result"]["command"].as_str().unwrap_or("").to_owned();
                self.last_note = format!(
                    "{} 명령={command} 상태={} 오류={}",
                    answer["url"].as_str().unwrap_or("").split('?').next().unwrap_or(""),
                    answer["result"]["result"]["state"].as_str().unwrap_or("-"),
                    answer["result"]["error"].as_str().or(answer["result"]["result"]["detail"].as_str()).unwrap_or("-"),
                );
                // 포털 화면이 새로 열렸으면 그 답은 버리고 도엽 확인부터 다시 한다.
                if !(answer["result"]["fresh"] == true && flow.on_portal_reloaded()) {
                    flow.on_result(&command, &answer["result"]["result"]);
                }
            }
            Some(_) => {
                self.last_note = "시킬 화면 없음".to_owned();
                flow.on_missing_page();
            }
            // 답을 잃었다: 미리 넘긴 단계는 그대로 둔다(같은 신청을 두 번 내지 않는다).
            None => {
                self.last_note = "답 없음".to_owned();
                flow.on_result("", &Value::Null);
            }
        }
        Ok(())
    }

    /// `until` 단계에 닿거나 멈출 때까지 돌린다. 그 단계의 일은 시키지 않는다.
    pub fn run_until(
        &mut self,
        flow: &mut PortalFlow,
        until: Stage,
        inputs: &PortalInputs,
        mut on_change: impl FnMut(&PortalFlow, &str),
    ) -> Result<(), String> {
        let mut last = (flow.stage, flow.phase.clone(), flow.message.clone(), String::new());
        while flow.stage != until && !matches!(flow.stage, Stage::Stopped | Stage::Complete | Stage::Transfer) {
            self.step(flow, inputs)?;
            let now = (flow.stage, flow.phase.clone(), flow.message.clone(), self.last_note.clone());
            if now != last {
                on_change(flow, &self.last_note);
                last = now;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        Ok(())
    }
}

impl Drop for PortalHelper {
    fn drop(&mut self) {
        let _ = self.send(&json!({"cmd": "quit"}));
        std::thread::sleep(Duration::from_millis(300));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wrapped_script_gives_the_account_only_to_login_pages() {
        let login = PortalLogin { id: "someone".to_owned(), password: "p\"w'd</script>".to_owned(), attempt: 2 };
        let script = wrap_script("// c\n(function (input) { return {state:input.command}; })", &json!({"command": "select", "x": 1}), &login);
        // 화면 안에서도 공식 사이트(https, ngii.go.kr 와 그 하위)인지 먼저 확인한다 — 아니면 계정을 넣기 전에 끝낸다.
        let origin = script.find(r#"location.protocol==='https:'&&(location.hostname==="ngii.go.kr"||location.hostname.slice(-11)===".ngii.go.kr");if(!official)return"#).expect("주소 확인");
        assert!(origin < script.find("loginPassword").expect("계정"));
        // 계정은 로그인 경로일 때만 입력에 합쳐진다.
        let guard = script.find("indexOf(location.pathname)>=0){").expect("경로 확인");
        let account = script.find("loginPassword").expect("계정");
        assert!(guard < account && script.contains("\"/member/login.do\""));
        assert!(script.contains(r#""loginAttempt":"2""#));
        // 따옴표가 든 비밀번호도 문자열 밖으로 새지 않는다(JSON 으로 적는다).
        assert!(script.contains(r#"p\"w'd</script>"#));
        assert!(script.contains("(input);return {command:input.command,fresh:fresh,result:r||{}};}catch(e){"));
        // 포털 화면이 새로 열렸는지 표시를 남겨 알아내고, 그 차례에는 아무것도 시키지 않는다
        // (옛 단계의 명령이 새 화면에서 신청서를 열어 버리지 않게).
        assert!(script.contains("{window.__kaPageMark=1;return {command:input.command,fresh:true,result:{}};}"));
        assert!(script.contains(r#"location.pathname==="/ms/map/NlipMap.do"&&!window.__kaPageMark"#));
        // 디버그 출력에 계정이 없다.
        let shown = format!("{login:?}");
        assert!(!shown.contains("someone") && !shown.contains("p\\\"w"), "{shown}");
    }
}
