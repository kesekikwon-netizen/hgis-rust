//! 국토정보플랫폼 실제 HTTP 두 단계 (C++ `data/providers/ngii-topographic.js:34-121`).
//!
//! `ngii_topographic` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use std::process::Command;

use super::ngii_protocol::NgiiTopographicClient;
use super::ngii_sheets::SheetQuery;

/// 공식 포털의 두 POST 를 실제로 보내 1:25,000 DXF 도엽 목록을 받는다.
///
/// C++ `ngii-topographic.js:43-64` 와 같은 순서·같은 본문이다. `x`/`y` 는 EPSG:5179 미터,
/// `buffer_m` 은 조사 반경(`MainWindowDownloads.cpp:187` = 5000).
pub fn fetch_official_sheets_25k(
    x: f64,
    y: f64,
    buffer_m: f64,
    wanted: &[String],
) -> Result<SheetQuery, String> {
    let scope_body = NgiiTopographicClient::combine_buffer_body(x, y, buffer_m);
    let scope_bytes = http_post_form(&NgiiTopographicClient::combine_buffer_query_url(), &scope_body)?;
    let area = NgiiTopographicClient::parse_combine_buffer(&scope_bytes, x, y, buffer_m)?;
    let suchi_bytes =
        http_post_form(&NgiiTopographicClient::suchi_query_url(), &NgiiTopographicClient::suchi_body(&area))?;
    NgiiTopographicClient::parse_suchi(&suchi_bytes, &area, wanted)
}

/// curl 로 `application/x-www-form-urlencoded` POST (C++ 은 브라우저 fetch 로 같은 본문을 보낸다)
fn http_post_form(url: &str, body: &str) -> Result<Vec<u8>, String> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };
    let mut cmd = Command::new(curl_bin);
    cmd.args([
        "-sS",
        "--max-time",
        "30",
        "-A",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3",
        "-H",
        "Content-Type: application/x-www-form-urlencoded; charset=UTF-8",
        "-H",
        "Referer: https://map.ngii.go.kr/ms/map/NlipMap.do",
        "-X",
        "POST",
        "--data-binary",
        body,
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd
        .output()
        .map_err(|_| NgiiTopographicClient::portal_timeout_message().to_owned())?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(NgiiTopographicClient::portal_timeout_message().to_owned());
    }
    Ok(output.stdout)
}
