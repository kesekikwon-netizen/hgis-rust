//! Which page is which, by its address (`trustedPage`, `transferPage` and the login paths of
//! `pollAutomation`, `KaTopographicBrowser.cpp:628-640, 743-755`).

/// The parts of an address the rules look at, as `QUrl` gives them: scheme and host in lower case, the port
/// only when it is written out, the path without query and fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlParts {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

impl UrlParts {
    /// Splits `scheme://[user@]host[:port][/path][?query][#fragment]`. Anything else is not a page address.
    #[must_use]
    pub fn parse(url: &str) -> Option<Self> {
        let (scheme, rest) = url.split_once("://")?;
        if scheme.is_empty() || !scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return None;
        }
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        let host_port = authority.rsplit('@').next().unwrap_or("");
        let (host, port) = match host_port.rsplit_once(':') {
            Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => (host, Some(port.parse::<u16>().ok()?)),
            Some((host, "")) => (host, None),
            _ => (host_port, None),
        };
        if host.is_empty() {
            return None;
        }
        let path_end = tail.find(['?', '#']).unwrap_or(tail.len());
        Some(Self { scheme: scheme.to_ascii_lowercase(), host: host.to_ascii_lowercase(), port, path: tail[..path_end].to_owned() })
    }
}

const PORTAL_MAP_PATH: &str = "/ms/map/NlipMap.do";
/// The login pages, in the order the C++ looks for them (`:743-746`); a later one wins.
const LOGIN_PATHS: [&str; 4] = ["/mn/loginPopup.do", "/anyid/common/login.do", "/anyid/common/anyidLogin.do", "/member/login.do"];

/// Same scheme, host and port as the portal (`trustedPage`, `:628-631`).
pub(super) fn trusted(portal: &UrlParts, url: &UrlParts) -> bool {
    url.scheme == portal.scheme && url.host == portal.host && url.port == portal.port
}

/// The portal's map page.
pub(super) fn portal_map(portal: &UrlParts, url: &UrlParts) -> bool {
    trusted(portal, url) && url.path == PORTAL_MAP_PATH
}

fn local_host(url: &UrlParts) -> bool {
    url.host == "127.0.0.1" || url.host == "localhost"
}

/// The page that hands over the files of an order (`transferPage`, `:633-639`).
pub(super) fn transfer_page(portal: &UrlParts, url: &UrlParts, test_mode: bool) -> bool {
    let local_test = test_mode && trusted(portal, url) && local_host(url);
    if local_test && url.path == "/pd/purchs/download.do" {
        return true;
    }
    url.path == "/nlippd/pd/innorix/innorixdownLoad.do"
        && (local_test || (url.scheme == "https" && url.host == "nlippd.ngii.go.kr" && url.port.is_none_or(|p| p == 443)))
}

/// The rank of a login page among the login paths, or `None` when the page is not one (`:747-754`): a login
/// path on the portal itself, or the member login of the institute's own site over https without a port.
pub(super) fn login_rank(portal: &UrlParts, url: &UrlParts) -> Option<usize> {
    let rank = LOGIN_PATHS.iter().position(|p| *p == url.path)?;
    let member = url.path == "/member/login.do" && url.scheme == "https" && url.port.is_none() && (url.host == "www.ngii.go.kr" || url.host == "ngii.go.kr");
    (trusted(portal, url) || member).then_some(rank)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn parts(url: &str) -> UrlParts {
        UrlParts::parse(url).unwrap()
    }

    fn portal() -> UrlParts {
        parts(crate::ngii_auto::PORTAL_HOME)
    }

    #[test]
    fn an_address_is_split_as_qurl_splits_it() {
        let u = parts("HTTPS://Map.NGII.go.kr/ms/map/NlipMap.do?tabGb=total#x");
        assert_eq!((u.scheme.as_str(), u.host.as_str(), u.port, u.path.as_str()), ("https", "map.ngii.go.kr", None, "/ms/map/NlipMap.do"));
        assert_eq!(parts("http://127.0.0.1:8123/a").port, Some(8123));
        assert_eq!(parts("https://user@host.kr:444").host, "host.kr");
        assert_eq!(parts("https://host.kr").path, "");
        assert_eq!(parts("https://host.kr?x=/mn/loginPopup.do").path, "");
        for bad in ["", "about:blank", "mailto:a@b", "https://", "https://:80/x", "https://host:99999/x", "ht tp://host/x"] {
            assert_eq!(UrlParts::parse(bad), None, "{bad}");
        }
    }

    /// `:628-631` — scheme, host and port must all be the portal's.
    #[test]
    fn only_the_portals_own_origin_is_trusted() {
        let p = portal();
        assert!(portal_map(&p, &parts("https://map.ngii.go.kr/ms/map/NlipMap.do")));
        for other in [
            "http://map.ngii.go.kr/ms/map/NlipMap.do",
            "https://map.ngii.go.kr:8443/ms/map/NlipMap.do",
            "https://map.ngii.go.kr.evil.example/ms/map/NlipMap.do",
            "https://evil.example/ms/map/NlipMap.do",
            "https://map.ngii.go.kr/ms/map/NlipMap.do/x",
            "https://map.ngii.go.kr/other.do",
        ] {
            assert!(!portal_map(&p, &parts(other)), "{other}");
        }
    }

    /// `:633-639`
    #[test]
    fn the_transfer_page_is_the_institutes_own_or_the_local_test_one() {
        let p = portal();
        assert!(transfer_page(&p, &parts("https://nlippd.ngii.go.kr/nlippd/pd/innorix/innorixdownLoad.do?a=1"), false));
        assert!(transfer_page(&p, &parts("https://nlippd.ngii.go.kr:443/nlippd/pd/innorix/innorixdownLoad.do"), false));
        for other in [
            "http://nlippd.ngii.go.kr/nlippd/pd/innorix/innorixdownLoad.do",
            "https://nlippd.ngii.go.kr:8443/nlippd/pd/innorix/innorixdownLoad.do",
            "https://evil.example/nlippd/pd/innorix/innorixdownLoad.do",
            "https://map.ngii.go.kr/pd/purchs/download.do",
            "http://127.0.0.1:8123/pd/purchs/download.do",
        ] {
            assert!(!transfer_page(&p, &parts(other), false), "{other}");
            assert!(!transfer_page(&p, &parts(other), true), "{other} (test mode, real portal)");
        }
        let local = parts("http://127.0.0.1:8123/ms/map/NlipMap.do");
        assert!(transfer_page(&local, &parts("http://127.0.0.1:8123/pd/purchs/download.do"), true));
        assert!(transfer_page(&local, &parts("http://127.0.0.1:8123/nlippd/pd/innorix/innorixdownLoad.do"), true));
        assert!(!transfer_page(&local, &parts("http://127.0.0.1:8123/pd/purchs/download.do"), false), "not outside test mode");
        assert!(!transfer_page(&local, &parts("http://127.0.0.1:9999/pd/purchs/download.do"), true), "another port");
    }

    /// `:743-754`
    #[test]
    fn login_pages_are_the_portals_login_paths_and_the_member_login() {
        let p = portal();
        assert_eq!(login_rank(&p, &parts("https://map.ngii.go.kr/mn/loginPopup.do")), Some(0));
        assert_eq!(login_rank(&p, &parts("https://map.ngii.go.kr/anyid/common/login.do")), Some(1));
        assert_eq!(login_rank(&p, &parts("https://map.ngii.go.kr/anyid/common/anyidLogin.do?x=1")), Some(2));
        assert_eq!(login_rank(&p, &parts("https://www.ngii.go.kr/member/login.do")), Some(3));
        assert_eq!(login_rank(&p, &parts("https://ngii.go.kr/member/login.do")), Some(3));
        for other in [
            "https://www.ngii.go.kr:443/member/login.do",
            "http://www.ngii.go.kr/member/login.do",
            "https://www.ngii.go.kr/mn/loginPopup.do",
            "https://evil.example/member/login.do",
            "https://map.ngii.go.kr/ms/map/NlipMap.do",
        ] {
            assert_eq!(login_rank(&p, &parts(other)), None, "{other}");
        }
    }
}
