//! curl 에 주소를 명령줄이 아니라 표준입력의 설정 글(`--config -`, `url = "…"`)로 준다(SEC1).
//! VWorld 키 등 주소에 든 값이 프로세스 명령줄(작업 관리자·다른 프로그램)에 보이지 않게 — 받는 일은 그대로 curl 이 한다.

use std::process::{Command, Output};

/// curl 설정 파일 한 줄 `url = "…"` — 따옴표 안의 `\` 와 `"` 는 `\` 로 막는다.
#[must_use]
pub fn curl_url_config(url: &str) -> String {
    format!("url = \"{}\"\n", url.replace('\\', "\\\\").replace('"', "\\\""))
}

/// 옵션을 단 curl 명령에 `--config -` 를 붙이고, 표준입력으로 줄 설정 글을 함께 돌려준다.
#[must_use]
pub fn with_url_on_stdin(mut cmd: Command, url: &str) -> (Command, String) {
    cmd.args(["--config", "-"]);
    (cmd, curl_url_config(url))
}

/// 주소를 표준입력으로 넘겨 curl 을 돌리고 끝까지 기다린다(표준출력·오류는 모은다).
///
/// # Errors
/// curl 을 시작하지 못하거나 표준입력에 쓰지 못함.
pub fn output_with_url_on_stdin(cmd: Command, url: &str) -> std::io::Result<Output> {
    let (cmd, config) = with_url_on_stdin(cmd, url);
    crate::network::run_output_with_stdin(cmd, config.as_bytes())
}

#[cfg(test)]
#[path = "curl_stdin_tests.rs"]
mod tests;
