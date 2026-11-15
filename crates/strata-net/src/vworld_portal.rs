//! The session with the VWorld site that the county archives of the continuous cadastral map are received
//! in (`target/cpp-ref/src/core/CadastralPortal.cpp:26-105`, `Session::request`): the pages of one run share
//! their cookies, a page is given 40 s and an archive half an hour, and what comes back is told apart by
//! the C++ words. curl does the transfer; the address and the login form reach it on its standard input,
//! never on the command line (`curl_stdin.rs`), and every launch goes through this crate's gate
//! (`network.rs`), so the offline switch holds.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
#[path = "vworld_portal_tests.rs"]
mod tests;

/// `CadastralPortal.cpp:27-28, 48, 272`.
pub const SITE: &str = "https://www.vworld.kr";
pub const DATASET: &str = "/dtmk/dtmk_ntads_s002.do?dsId=30563";
pub const LOGIN: &str = "/v4po_usrlogin_a004.do";
pub const USER_AGENT: &str = "Mozilla/5.0 ka-hgis";
/// What curl writes after the body, so that the status of the answer can be read.
const STATUS_MARK: &str = "\n__KA_HTTP_STATUS__:";

/// Why a request brought nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalFailure {
    /// No byte for the idle time, or the whole time ran out (`:65-69`).
    Timeout,
    /// More than a page or an archive may be (`:75-77`).
    TooBig,
    /// The archive could not be written (`:79-81`).
    Disk,
    /// Not reached, refused, or another status than 200 (`:93-98`).
    Network,
    /// A page came in place of the archive: the session is not logged in (`:99-101`).
    NotZip,
    /// The caller gave the request up while it ran.
    GivenUp,
}

impl PortalFailure {
    /// The C++ words of the failure; none for a request that was given up.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::Timeout => "지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요.",
            Self::TooBig => "지적도 서버 응답이 허용 크기를 넘었습니다.",
            Self::Disk => "지적도 저장 공간을 확인하세요.",
            Self::Network => "VWorld 통신에 실패했습니다. 인터넷 연결과 계정 설정을 확인하세요.",
            Self::NotZip => "VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요.",
            Self::GivenUp => "",
        }
    }
}

/// The requests of one run. A test hands in its own answers.
pub trait PortalHttp {
    /// The body of a page or of an answer of the API.
    ///
    /// # Errors
    /// See [`PortalFailure`].
    fn get(&mut self, url: &str, give_up: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure>;

    /// The answer to a form (`application/x-www-form-urlencoded`), sent as the site's own page sends it.
    ///
    /// # Errors
    /// See [`PortalFailure`].
    fn post_form(&mut self, url: &str, form: &str, give_up: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure>;

    /// Receives an archive into `to`; `watch` is told the bytes received so far while it grows. `to` is
    /// there only when the archive is whole.
    ///
    /// # Errors
    /// See [`PortalFailure`].
    fn download(&mut self, url: &str, to: &Path, give_up: &dyn Fn() -> bool, watch: &mut dyn FnMut(u64)) -> Result<(), PortalFailure>;
}

/// curl ends with 28 when a time limit ran out, 63 when the file is larger than allowed, 23 when it could
/// not write.
fn failure_of(exit_code: Option<i32>) -> PortalFailure {
    match exit_code {
        Some(28) => PortalFailure::Timeout,
        Some(63) => PortalFailure::TooBig,
        Some(23) => PortalFailure::Disk,
        _ => PortalFailure::Network,
    }
}

fn part_of(file: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", file.display()))
}

/// The curl command of one request and the config text for its standard input.
fn request(jar: &Path, url: &str, form: Option<&str>, file: Option<&Path>) -> (Command, String) {
    // Idle time, whole time (seconds) and the most bytes: a page, or an archive (`:45-46, 75`).
    let (idle, whole, most) = if file.is_some() { (120_u32, 30 * 60_u32, 512_u64 * 1024 * 1024) } else { (40, 40, 32 * 1024 * 1024) };
    // The key of the API is registered for this referer; the site's pages are asked from its own page (`:49`).
    let referer = if url.starts_with("https://api.vworld.kr/") { "https://localhost".to_owned() } else { format!("{SITE}{DATASET}") };
    let mut command = Command::new(if cfg!(target_os = "windows") { "curl.exe" } else { "curl" });
    strata_types::process::no_window(&mut command);
    command.args(["-sS", "--fail", "--location", "--max-redirs", "5", "-A", USER_AGENT, "-e", &referer]);
    command.arg("--connect-timeout").arg(idle.to_string());
    command.args(["--speed-limit", "1", "--speed-time"]).arg(idle.to_string());
    command.arg("--max-time").arg(whole.to_string());
    command.arg("--max-filesize").arg(most.to_string());
    command.arg("-b").arg(jar).arg("-c").arg(jar);
    if form.is_some() {
        command.args(["-H", "X-Requested-With: XMLHttpRequest"]);
    }
    command.args(["-w", &format!("{STATUS_MARK}%{{http_code}}")]);
    if let Some(file) = file {
        command.arg("-o").arg(part_of(file));
    }
    command.args(["--config", "-"]);
    let mut config = crate::curl_stdin::curl_url_config(url);
    if let Some(form) = form {
        config.push_str(&format!("data = \"{}\"\n", form.replace('\\', "\\\\").replace('"', "\\\"")));
    }
    (command, config)
}

/// The body of an answer whose status was 200 (`:93-98`).
fn body_of(mut answer: Vec<u8>) -> Result<Vec<u8>, PortalFailure> {
    let mark = STATUS_MARK.as_bytes();
    let at = answer.windows(mark.len()).rposition(|window| window == mark).ok_or(PortalFailure::Network)?;
    if &answer[at + mark.len()..] != b"200" {
        return Err(PortalFailure::Network);
    }
    answer.truncate(at);
    Ok(answer)
}

/// What was received takes its place when it begins as an archive does (`:99-101`); else it is taken away.
fn keep_archive(part: &Path, to: &Path) -> Result<(), PortalFailure> {
    let begins = std::fs::read(part).map(|bytes| bytes.starts_with(b"PK\x03\x04")).unwrap_or(false);
    if !begins {
        let _ = std::fs::remove_file(part);
        return Err(PortalFailure::NotZip);
    }
    let _ = std::fs::remove_file(to);
    std::fs::rename(part, to).map_err(|_| PortalFailure::Disk)
}

/// The session of one run: its cookies live in a folder of its own, which goes with the session.
#[derive(Debug)]
pub struct CurlPortal {
    folder: PathBuf,
}

impl CurlPortal {
    /// # Errors
    /// The folder for the cookies cannot be made.
    pub fn open() -> std::io::Result<Self> {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let folder = std::env::temp_dir().join(format!("ka-hgis-vworld-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        Ok(Self { folder })
    }

    fn run(&self, url: &str, form: Option<&str>, file: Option<&Path>, give_up: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure> {
        let (command, config) = request(&self.folder.join("cookies.txt"), url, form, file);
        match crate::network::run_curl_url_with_stdin_until(command, url, config.as_bytes(), give_up) {
            Ok(None) => Err(PortalFailure::GivenUp),
            Ok(Some(output)) if output.status.success() => body_of(output.stdout),
            Ok(Some(output)) => Err(failure_of(output.status.code())),
            Err(_) => Err(PortalFailure::Network),
        }
    }
}

impl Drop for CurlPortal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

impl PortalHttp for CurlPortal {
    fn get(&mut self, url: &str, give_up: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure> {
        self.run(url, None, None, give_up)
    }

    fn post_form(&mut self, url: &str, form: &str, give_up: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure> {
        self.run(url, Some(form), None, give_up)
    }

    fn download(&mut self, url: &str, to: &Path, give_up: &dyn Fn() -> bool, watch: &mut dyn FnMut(u64)) -> Result<(), PortalFailure> {
        let (part, watch) = (part_of(to), RefCell::new(watch));
        // The asking for a stop is also the tick at which the growing file is looked at.
        let outcome = self.run(url, None, Some(to), &|| {
            if let Ok(file) = std::fs::metadata(&part) {
                (watch.borrow_mut())(file.len());
            }
            give_up()
        });
        match outcome {
            Ok(_) => keep_archive(&part, to),
            Err(failure) => {
                let _ = std::fs::remove_file(&part);
                Err(failure)
            }
        }
    }
}
