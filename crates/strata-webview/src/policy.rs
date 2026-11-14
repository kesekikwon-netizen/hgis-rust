//! Which addresses a hidden view may open.
//!
//! The C++ page accepts http, https, about, blob and data and hands every other scheme to the system or
//! drops it (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:70-78`). A hidden view has nobody to ask: other
//! schemes are refused and reported. On top of that the app's network switch is asked for every web address
//! (`strata_net::network::is_allowed`, passed in as a predicate so that this crate does not depend on it).

use std::sync::Arc;

/// Verdict on one navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    Allow,
    /// A web address while the app's network switch is off.
    RefusedOffline,
    /// Not a web address (mailto:, file:, javascript:, a program link …).
    RefusedScheme,
}

/// The rule, fixed for a session.
#[derive(Clone)]
pub struct NavigationPolicy {
    network_allowed: Arc<dyn Fn() -> bool + Send + Sync>,
    local_origin: Option<String>,
}

impl std::fmt::Debug for NavigationPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationPolicy").field("local_origin", &self.local_origin).finish_non_exhaustive()
    }
}

impl NavigationPolicy {
    /// `local_origin` is the origin of a test site on this PC (`http://127.0.0.1:<port>`); it is reachable
    /// with the network switch off, and only that exact origin is.
    #[must_use]
    pub fn new(network_allowed: Arc<dyn Fn() -> bool + Send + Sync>, local_origin: Option<&str>) -> Self {
        Self { network_allowed, local_origin: local_origin.map(|o| o.trim_end_matches('/').to_ascii_lowercase()) }
    }

    #[must_use]
    pub fn judge(&self, url: &str) -> Navigation {
        let url = url.trim();
        let Some((scheme, _)) = url.split_once(':') else {
            return Navigation::RefusedScheme;
        };
        match scheme.to_ascii_lowercase().as_str() {
            "about" | "blob" | "data" => Navigation::Allow,
            "http" | "https" => {
                if self.is_local(url) || (self.network_allowed)() {
                    Navigation::Allow
                } else {
                    Navigation::RefusedOffline
                }
            }
            _ => Navigation::RefusedScheme,
        }
    }

    /// The address is on the test site: the origin matches to its end, then the address ends or a path,
    /// a query or a fragment begins. (`http://127.0.0.1:80.evil.example` is not `http://127.0.0.1:80`.)
    fn is_local(&self, url: &str) -> bool {
        let Some(origin) = &self.local_origin else {
            return false;
        };
        let lower = url.to_ascii_lowercase();
        lower.strip_prefix(origin.as_str()).is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(online: bool, local: Option<&str>) -> NavigationPolicy {
        NavigationPolicy::new(Arc::new(move || online), local)
    }

    /// KaTopographicBrowser.cpp:70-78 — the five schemes that stay inside the engine.
    #[test]
    fn web_and_in_engine_schemes_are_allowed_when_online() {
        let p = policy(true, None);
        for url in [
            "https://map.ngii.go.kr/ms/map/NlipMap.do?tabGb=total",
            "http://www.ngii.go.kr/member/login.do",
            "HTTPS://MAP.NGII.GO.KR/",
            "about:blank",
            "blob:https://nlippd.ngii.go.kr/1b2c",
            "data:text/html,hello",
        ] {
            assert_eq!(p.judge(url), Navigation::Allow, "{url}");
        }
    }

    /// A hidden view never starts another program and never opens a local file.
    #[test]
    fn other_schemes_are_refused_also_when_online() {
        let p = policy(true, None);
        for url in ["mailto:someone@example.org", "file:///C:/Windows/win.ini", "javascript:alert(1)", "ms-settings:display", "hwp://x", "no-scheme-at-all", ""] {
            assert_eq!(p.judge(url), Navigation::RefusedScheme, "{url}");
        }
    }

    #[test]
    fn the_network_switch_stops_every_web_address_but_not_the_in_engine_schemes() {
        let p = policy(false, None);
        assert_eq!(p.judge("https://map.ngii.go.kr/ms/map/NlipMap.do"), Navigation::RefusedOffline);
        assert_eq!(p.judge("http://127.0.0.1:8080/"), Navigation::RefusedOffline, "no test site was named");
        assert_eq!(p.judge("about:blank"), Navigation::Allow);
        assert_eq!(p.judge("mailto:x@example.org"), Navigation::RefusedScheme);
    }

    /// Only the exact origin of the test site passes with the switch off; a longer host that merely starts
    /// with it does not.
    #[test]
    fn the_local_test_site_is_matched_by_its_exact_origin() {
        let p = policy(false, Some("http://127.0.0.1:5123/"));
        for url in ["http://127.0.0.1:5123", "http://127.0.0.1:5123/", "http://127.0.0.1:5123/ms/map/NlipMap.do?x=1", "HTTP://127.0.0.1:5123#top", "http://127.0.0.1:5123?q"] {
            assert_eq!(p.judge(url), Navigation::Allow, "{url}");
        }
        for url in [
            "http://127.0.0.1:51234/",
            "http://127.0.0.1:5123.evil.example/",
            "http://127.0.0.1:5123@evil.example/",
            "https://127.0.0.1:5123/",
            "http://127.0.0.1/",
            "http://localhost:5123/",
        ] {
            assert_eq!(p.judge(url), Navigation::RefusedOffline, "{url}");
        }
    }
}
