//! 쿠키를 유지하는 HTTP 세션 — VWorld 누리집 로그인이 필요한 요청에 쓴다.
//!
//! C++ `CadastralPortal.cpp:31-105` 의 `Session::request` 는 `QNetworkAccessManager` 하나로
//! 쿠키를 이어 가며 요청한다. 여기서는 `curl` 의 쿠키 항아리(`-b`/`-c`)로 같은 일을 한다.
//! - 쿠키 항아리는 임시 폴더의 고유한 파일이고 세션이 끝나면 지운다(세션 쿠키는 비밀이다).
//! - 로그인 폼 본문은 명령줄이 아니라 표준 입력으로 넘긴다(작업 관리자에 보이지 않게).
//! - 큰 파일은 `-o` 로 파일에 직접 받고, 받는 동안 취소할 수 있다.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// C++ `CadastralPortal.cpp:47` 의 User-Agent.
const USER_AGENT: &str = "Mozilla/5.0 ka-hgis";
/// 일반 요청 시간 제한(초) — C++ `:45` 40초.
const REQUEST_TIMEOUT_SECS: &str = "40";
/// ZIP: 120초 동안 바이트가 오지 않으면 중단, 전체 30분 — C++ `:45-46`.
const ZIP_IDLE_SECS: &str = "120";
const ZIP_TOTAL_SECS: &str = "1800";

static SERIAL: AtomicU64 = AtomicU64::new(0);

fn unique_temp_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let n = SERIAL.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("ka-hgis-{tag}-{}-{nanos}-{n}", std::process::id()))
}

/// 폼 값 하나를 퍼센트 인코딩한다(영숫자와 `-_.~` 만 그대로).
#[must_use]
pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 3);
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `이름=값&…` 폼 본문.
#[must_use]
pub fn form_body(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// 쿠키를 이어 가는 세션.
#[derive(Debug)]
pub struct PortalSession {
    jar: PathBuf,
    /// 누리집 요청에 붙일 Referer.
    site_referer: String,
}

impl Drop for PortalSession {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.jar);
    }
}

impl PortalSession {
    /// 새 세션. `site_referer` 는 `api.vworld.kr` 이 아닌 주소에 붙일 Referer 다.
    #[must_use]
    pub fn new(site_referer: &str) -> Self {
        Self {
            jar: unique_temp_path("cookies"),
            site_referer: site_referer.to_owned(),
        }
    }

    /// C++ `:48-49` — API 는 `https://localhost`, 누리집은 자료 화면 주소.
    fn referer_for(&self, url: &str) -> &str {
        if url.starts_with("https://api.vworld.kr/") {
            "https://localhost"
        } else {
            &self.site_referer
        }
    }

    fn base_args(&self, url: &str, out_file: &Path) -> Vec<String> {
        let jar = self.jar.to_string_lossy().into_owned();
        vec![
            "-sS".into(),
            "-g".into(),
            // C++ 은 같은 출처의 리다이렉트를 따라간다(`SameOriginRedirectPolicy`). curl 에는
            // 그 옵션이 없어 https 로만, 5번까지 따라가게 한다.
            "-L".into(),
            "--max-redirs".into(),
            "5".into(),
            "--proto-redir".into(),
            "=https".into(),
            "-A".into(),
            USER_AGENT.into(),
            "-e".into(),
            self.referer_for(url).to_owned(),
            "-b".into(),
            jar.clone(),
            "-c".into(),
            jar,
            "-w".into(),
            "%{http_code}".into(),
            "-o".into(),
            out_file.to_string_lossy().into_owned(),
        ]
    }

    /// 요청을 보내고 본문을 돌려준다. HTTP 상태가 200 이 아니면 실패다(C++ `:93-98`).
    fn run(&self, mut args: Vec<String>, url: &str, stdin_body: Option<&str>) -> Result<Vec<u8>, String> {
        let out_file = unique_temp_path("body");
        let mut full = self.base_args(url, &out_file);
        full.append(&mut args);
        full.push(url.to_owned());
        let mut child = Command::new("curl")
            .args(&full)
            .stdin(if stdin_body.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("curl 을 실행하지 못했습니다: {e}"))?;
        if let (Some(body), Some(mut stdin)) = (stdin_body, child.stdin.take()) {
            let _ = stdin.write_all(body.as_bytes());
        }
        let output = child.wait_with_output().map_err(|e| e.to_string());
        let body = std::fs::read(&out_file).unwrap_or_default();
        let _ = std::fs::remove_file(&out_file);
        let output = output?;
        let status = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !output.status.success() || status != "200" {
            return Err(format!("HTTP {status} (curl {})", output.status.code().unwrap_or(-1)));
        }
        Ok(body)
    }

    /// 진단용: 리다이렉트를 따라가지 않고 응답 머리만 받는다. 쿠키 줄은 뺀다.
    pub fn response_headers(&self, url: &str) -> Vec<String> {
        let jar = self.jar.to_string_lossy().into_owned();
        let out = Command::new("curl")
            .args(["-sS", "-g", "-A", USER_AGENT, "-e", self.referer_for(url), "-b", &jar, "-c", &jar])
            .args(["--max-time", REQUEST_TIMEOUT_SECS, "-D", "-", "-o"])
            .arg(if cfg!(windows) { "NUL" } else { "/dev/null" })
            .arg(url)
            .output();
        out.map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.to_ascii_lowercase().starts_with("set-cookie"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
    }

    /// GET — 본문을 글자로.
    pub fn get_text(&self, url: &str) -> Result<String, String> {
        let args = vec!["--max-time".into(), REQUEST_TIMEOUT_SECS.into()];
        self.run(args, url, None)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    /// POST(폼) — C++ `:52-55` 처럼 `X-Requested-With: XMLHttpRequest` 를 붙인다.
    /// 본문은 표준 입력으로 보낸다.
    pub fn post_form(&self, url: &str, fields: &[(&str, &str)]) -> Result<String, String> {
        let args = vec![
            "--max-time".into(),
            REQUEST_TIMEOUT_SECS.into(),
            "-H".into(),
            "X-Requested-With: XMLHttpRequest".into(),
            "--data-binary".into(),
            "@-".into(),
        ];
        self.run(args, url, Some(&form_body(fields)))
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    /// 큰 파일을 `dest` 에 직접 받는다. `cancel` 이 서면 멈춘다. 받은 것이 ZIP(`PK\x03\x04`)이
    /// 아니면 실패다(C++ `:99-101`). `max_bytes` 를 넘으면 curl 이 스스로 멈춘다.
    pub fn download_zip(
        &self,
        url: &str,
        dest: &Path,
        max_bytes: u64,
        cancel: &AtomicBool,
    ) -> Result<(), ZipDownloadError> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|_| ZipDownloadError::Disk)?;
        }
        // 받는 중인 파일은 대상과 같은 폴더(같은 디스크)에 둔다. 시스템 임시 폴더(C:)는 공간이
        // 모자랄 수 있다 — 실제로 C: 가 가득 차 curl 이 쓰기 오류(23)로 실패한 적이 있다.
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let part = dest.with_file_name(format!("ka-hgis-part-{}-{serial}.zip.part", std::process::id()));
        let _ = std::fs::remove_file(&part);
        let mut args = self.base_args(url, &part);
        args.extend(
            [
                "--speed-time",
                ZIP_IDLE_SECS,
                "--speed-limit",
                "1",
                "--max-time",
                ZIP_TOTAL_SECS,
                "--max-filesize",
            ]
            .map(String::from),
        );
        args.push(max_bytes.to_string());
        args.push(url.to_owned());
        let mut child = Command::new("curl")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ZipDownloadError::Network)?;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if cancel.load(Ordering::Relaxed) {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = std::fs::remove_file(&part);
                        return Err(ZipDownloadError::Canceled);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&part);
                    return Err(ZipDownloadError::Network);
                }
            }
        };
        let mut code = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = std::io::Read::read_to_string(&mut out, &mut code);
        }
        let fail = |e: ZipDownloadError| {
            let _ = std::fs::remove_file(&part);
            Err(e)
        };
        if !status.success() || code.trim() != "200" {
            // 진단 줄(비밀 없음): curl 종료 코드와 HTTP 상태.
            let mut why = String::new();
            if let Some(mut err) = child.stderr.take() {
                let _ = std::io::Read::read_to_string(&mut err, &mut why);
            }
            eprintln!("[portal_session] zip 받기 실패: curl={:?} http={} {}", status.code(), code.trim(), why.trim());
            // curl 28 = 시간 초과, 63 = 크기 초과.
            return fail(match status.code() {
                Some(28) => ZipDownloadError::Timeout,
                Some(63) => ZipDownloadError::TooLarge,
                Some(23) => ZipDownloadError::Disk, // 디스크에 쓰지 못함(공간 부족 등)
                _ => ZipDownloadError::Network,
            });
        }
        let mut head = [0_u8; 4];
        let is_zip = std::fs::File::open(&part)
            .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
            .is_ok()
            && &head == b"PK\x03\x04";
        if !is_zip {
            return fail(ZipDownloadError::NotZip);
        }
        std::fs::rename(&part, dest).or_else(|_| fail(ZipDownloadError::Disk))
    }
}

/// ZIP 받기가 실패한 까닭.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipDownloadError {
    Canceled,
    Timeout,
    TooLarge,
    Network,
    /// ZIP 대신 로그인·오류 화면이 왔다.
    NotZip,
    Disk,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_values_are_percent_encoded_so_base64_survives() {
        // Base64 의 + / = 는 폼에서 다른 뜻이 되므로 모두 인코딩한다(C++ `:272` 는 + 를 %2B 로 바꾼다).
        assert_eq!(percent_encode("a+b/c=="), "a%2Bb%2Fc%3D%3D");
        assert_eq!(percent_encode("/dtmk/x.do?dsId=30563"), "%2Fdtmk%2Fx.do%3FdsId%3D30563");
        assert_eq!(
            form_body(&[("usrIdeE", "aWQ="), ("nextUrl", "/a b")]),
            "usrIdeE=aWQ%3D&nextUrl=%2Fa%20b"
        );
    }

    #[test]
    fn the_referer_depends_on_the_host_like_cpp() {
        let s = PortalSession::new("https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563");
        assert_eq!(s.referer_for("https://api.vworld.kr/req/data?x=1"), "https://localhost");
        assert_eq!(
            s.referer_for("https://www.vworld.kr/v4po_usrlogin_a004.do"),
            "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"
        );
    }

    #[test]
    fn each_session_has_its_own_cookie_jar_and_removes_it_when_done() {
        let a = PortalSession::new("r");
        let b = PortalSession::new("r");
        assert_ne!(a.jar, b.jar);
        let jar = a.jar.clone();
        let _ = std::fs::write(&jar, "cookie");
        drop(a);
        assert!(!jar.exists(), "세션이 끝나면 쿠키 파일을 지운다");
    }
}
