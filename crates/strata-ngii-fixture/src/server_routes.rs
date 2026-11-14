//! Route table branches 4..18 ported from `LocalDownloadServer`
//! (`target/cpp-ref/tests/test_topographic_browser.cpp:126-185`).

use crate::server::{graceful_close, start_transfer};
use crate::{NgiiFixtureState, COMBINE_BUFFER_QUERY_JSON, TRANSFER_SIZE};
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:126-184.
pub(crate) fn dispatch_standard_routes(
    mut stream: TcpStream,
    state: &Arc<Mutex<NgiiFixtureState>>,
    target: &str,
) {
    let mut content_type = "text/html; charset=utf-8";
    let mut extra = Vec::<u8>::new();
    let body: Vec<u8>;

    // Pre-handle `innorixDownLoad2.do` observer outside lock if registered (`test_topographic_browser.cpp:159-161`).
    let sdk_file_name = if target.starts_with("/nlippd/pd/innorix/innorixDownLoad2.do") {
        let name = query_item_value(target, "fileName");
        let observer = if let Ok(mut guard) = state.lock() {
            guard.sdk_requests.push(name.clone());
            guard.sdk_request_observed.clone()
        } else {
            return;
        };
        if let Some(cb) = observer {
            cb(&name);
        }
        Some(name)
    } else {
        None
    };

    let Ok(mut guard) = state.lock() else {
        return;
    };

    // Branch 4: `/ms/map/NlipMap.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:129-132`).
    if target.starts_with("/ms/map/NlipMap.do") {
        if guard.fail_portal_connection {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
        body = guard.portal_fixture.clone();
    // Branch 5: `/ms/map/combineBufferQuery.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:132-135`).
    } else if target.starts_with("/ms/map/combineBufferQuery.do") {
        body = COMBINE_BUFFER_QUERY_JSON.as_bytes().to_vec();
        content_type = "application/json";
    // Branch 6: `/ms/map/suchiQuery.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:135-137`).
    } else if target.starts_with("/ms/map/suchiQuery.do") {
        guard.search_count += 1;
        body = guard.search_fixture.clone();
        content_type = "application/json";
    // Branch 7: `/mn/loginCheck.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:137-140`).
    } else if target == "/mn/loginCheck.do" {
        guard.login_check_count += 1;
        body = if guard.logged_in {
            br#"{"isLogin":true}"#.to_vec()
        } else {
            br#"{"isLogin":false}"#.to_vec()
        };
        content_type = "application/json";
    // Branch 8: `/member/login.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:140-142`).
    } else if target.starts_with("/member/login.do") {
        body = guard.login_fixture.clone();
    // Branch 9: `/held-login-script.js` (`target/cpp-ref/tests/test_topographic_browser.cpp:142-144`).
    } else if target == "/held-login-script.js" {
        let _ = stream.set_read_timeout(None);
        guard.pending_login_script = Some(stream);
        return;
    // Branch 10: `/anyid/common/anyidLogin.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:144-146`).
    } else if target == "/anyid/common/anyidLogin.do" {
        body = guard.any_id_fixture.clone();
    // Branch 11: `/member/process.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:146-150`).
    } else if target == "/member/process.do" {
        guard.login_submission_count += 1;
        if guard.hold_login_response {
            let _ = stream.set_read_timeout(None);
            guard.held_sockets.push(stream);
            return;
        }
        if guard.accept_login {
            guard.logged_in = true;
            body = b"<html><script>parent.close();</script></html>".to_vec();
        } else {
            body = b"<html>Fixture login response</html>".to_vec();
        }
    // Branch 12: `/pd/purchs/download.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:150-152`).
    } else if target == "/pd/purchs/download.do" {
        body = if guard.redirect_transfer {
            b"<html><script>location.replace('/nlippd/pd/innorix/innorixdownLoad.do');</script></html>".to_vec()
        } else {
            guard.download_fixture.clone()
        };
    // Branch 13: `/nlippd/pd/innorix/innorixdownLoad.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:152-154`).
    } else if target == "/nlippd/pd/innorix/innorixdownLoad.do" {
        body = guard.download_fixture.clone();
    // Branch 14: `/fixture-*.zip` | `/fixture-*.xml` (`target/cpp-ref/tests/test_topographic_browser.cpp:154-159`).
    } else if target.starts_with("/fixture-") && (target.ends_with(".zip") || target.ends_with(".xml")) {
        let file_name = format!("terrain_{}", &target[9..]).into_bytes();
        if guard.stream_fixture_archives && target.ends_with(".zip") {
            start_transfer(&mut guard, stream, &file_name, false);
            return;
        }
        if guard.stream_fixture_archives && target.ends_with(".xml") && !guard.fixture_archive_finished {
            let _ = stream.set_read_timeout(None);
            guard.pending_xml_socket = Some(stream);
            guard.pending_xml_name = file_name;
            return;
        }
        body = b"PK-public-sheet-fixture".to_vec();
        content_type = "application/zip";
        extra.extend_from_slice(b"Content-Disposition: attachment; filename=");
        extra.extend_from_slice(&file_name);
        extra.extend_from_slice(b"\r\n");
    // Branch 15: `/nlippd/pd/innorix/innorixDownLoad2.do` (`target/cpp-ref/tests/test_topographic_browser.cpp:159-172`).
    } else if let Some(name) = sdk_file_name {
        if guard.hold_sdk_transfers && name.ends_with(".dxf") {
            guard.active_sdk_file = name.clone();
            start_transfer(&mut guard, stream, name.as_bytes(), false);
            return;
        }
        if guard.hold_sdk_transfers && name.ends_with(".xml") {
            let _ = stream.set_read_timeout(None);
            guard.sdk_xml_socket = Some(stream);
            guard.sdk_xml_name = name;
            return;
        }
        body = if name.ends_with(".dxf") {
            vec![b'x'; TRANSFER_SIZE]
        } else if name.ends_with(".xml") {
            b"fixture metadata".to_vec()
        } else {
            b"PK-fixture-complete".to_vec()
        };
        content_type = "application/octet-stream";
        extra = format!("Content-Disposition: attachment; filename={name}\r\n").into_bytes();
    // Branch 16: `/file.zip` (`target/cpp-ref/tests/test_topographic_browser.cpp:172-176`).
    } else if target == "/file.zip" {
        body = b"PK-fixture-complete".to_vec();
        content_type = "application/zip";
        extra = b"Content-Disposition: attachment; filename=terrain.zip\r\n".to_vec();
    // Branch 17: `/popup` (`target/cpp-ref/tests/test_topographic_browser.cpp:176-178`).
    } else if target == "/popup" {
        body = b"<html><head><title>popup</title></head><body>popup</body></html>".to_vec();
    // Branch 18: fallback (`target/cpp-ref/tests/test_topographic_browser.cpp:178-181`).
    } else {
        body = b"<html><head><title>local fixture</title></head><body>fixture</body></html>".to_vec();
        if target == "/" {
            extra = b"Set-Cookie: fixture=shared; Path=/; SameSite=Lax\r\n".to_vec();
        }
    }
    drop(guard);

    // Response trailer (`target/cpp-ref/tests/test_topographic_browser.cpp:182-184`).
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n");
    let _ = stream.write_all(&extra);
    let _ = write!(
        stream,
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(&body);
    graceful_close(stream);
}

// RFC 3986 percent-decoding matching `QUrlQuery(QUrl::fromEncoded(target)).queryItemValue(key)` (`:160`).
fn query_item_value(target: &str, key: &str) -> String {
    let without_frag = target.split('#').next().unwrap_or(target);
    let Some((_, query)) = without_frag.split_once('?') else {
        return String::new();
    };
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(k) == key {
            return percent_decode(v);
        }
    }
    String::new()
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
