//! Page dialogs. A hidden view must never put a window on the screen, so `alert`, `confirm` and `prompt` are
//! replaced before any page script runs and answered as the C++ page answers them while it suppresses dialogs:
//! nothing for an alert, "no" for a confirm, "cancel" for a prompt
//! (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:61-69`). The page tells the host that a dialog was asked
//! for. That message comes from the page and is treated as such: it can only say "a dialog of this kind with
//! this text", it carries no secret and nothing is done on its strength but showing the user a notice.

/// Longest dialog text that is passed on (characters).
pub const MAX_DIALOG_TEXT: usize = 500;

/// Runs in every frame when its document is created.
pub const QUIET_DIALOGS_SCRIPT: &str = r#"(function () {
  if (window.__strataQuietDialogs) { return; }
  window.__strataQuietDialogs = true;
  var tell = function (kind, text) {
    try {
      window.ipc.postMessage(JSON.stringify({ strata: 'dialog', kind: kind, text: String(text === undefined ? '' : text).slice(0, 500) }));
    } catch (e) {}
  };
  var quiet = function (target) {
    target.alert = function (text) { tell('alert', text); };
    target.confirm = function (text) { tell('confirm', text); return false; };
    target.prompt = function (text) { tell('prompt', text); return null; };
  };
  quiet(window);
  // A window a page opens starts with an empty document this script has not run in, and the page can call
  // a dialog of it at once (review of 2026-11-15, S1). The page that opened it makes it quiet.
  var open = window.open;
  window.open = function () {
    var opened = open.apply(window, arguments);
    try { if (opened) { quiet(opened); } } catch (e) {}
    return opened;
  };
})();"#;

/// Kind of dialog the page asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    Alert,
    Confirm,
    Prompt,
}

/// What a page said about a dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogNotice {
    pub kind: DialogKind,
    pub text: String,
}

/// Reads a message a page posted to the host. Anything that is not exactly a dialog notice is ignored.
#[must_use]
pub fn parse_page_message(body: &str) -> Option<DialogNotice> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let object = value.as_object()?;
    if object.get("strata")?.as_str()? != "dialog" {
        return None;
    }
    let kind = match object.get("kind")?.as_str()? {
        "alert" => DialogKind::Alert,
        "confirm" => DialogKind::Confirm,
        "prompt" => DialogKind::Prompt,
        _ => return None,
    };
    let text: String = object.get("text")?.as_str()?.chars().take(MAX_DIALOG_TEXT).collect();
    Some(DialogNotice { kind, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dialog_notice_is_read() {
        assert_eq!(
            parse_page_message(r#"{"strata":"dialog","kind":"alert","text":"아이디 또는 비밀번호를 확인하세요."}"#),
            Some(DialogNotice { kind: DialogKind::Alert, text: "아이디 또는 비밀번호를 확인하세요.".to_owned() })
        );
        assert_eq!(parse_page_message(r#"{"strata":"dialog","kind":"confirm","text":""}"#).map(|n| n.kind), Some(DialogKind::Confirm));
        assert_eq!(parse_page_message(r#"{"strata":"dialog","kind":"prompt","text":"x"}"#).map(|n| n.kind), Some(DialogKind::Prompt));
    }

    /// A page can post anything: only the exact shape counts, and a long text is cut.
    #[test]
    fn anything_else_a_page_posts_is_ignored() {
        for body in [
            "",
            "not json",
            "[]",
            r#"{"strata":"dialog"}"#,
            r#"{"strata":"dialog","kind":"alert"}"#,
            r#"{"strata":"dialog","kind":"alert","text":5}"#,
            r#"{"strata":"dialog","kind":"run","text":"x"}"#,
            r#"{"strata":"script","kind":"alert","text":"x"}"#,
            r#"{"kind":"alert","text":"x"}"#,
        ] {
            assert_eq!(parse_page_message(body), None, "{body}");
        }
        let long = format!(r#"{{"strata":"dialog","kind":"alert","text":"{}"}}"#, "가".repeat(2_000));
        assert_eq!(parse_page_message(&long).map(|n| n.text.chars().count()), Some(MAX_DIALOG_TEXT));
    }

    /// The script answers as the C++ page does while it suppresses dialogs (:61-69) and replaces all three.
    #[test]
    fn the_script_replaces_all_three_dialogs_with_the_cpp_answers() {
        let s = QUIET_DIALOGS_SCRIPT;
        assert!(s.contains("target.alert = function (text) { tell('alert', text); };"));
        assert!(s.contains("target.confirm = function (text) { tell('confirm', text); return false; };"));
        assert!(s.contains("target.prompt = function (text) { tell('prompt', text); return null; };"));
        assert!(s.contains("quiet(window);"), "the page's own window");
        assert!(s.contains("if (opened) { quiet(opened); }"), "and a window it opens (the real engine is asked in the probe)");
        assert!(s.contains(".slice(0, 500)"));
    }
}
