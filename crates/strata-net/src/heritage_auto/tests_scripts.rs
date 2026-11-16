//! The scripts must find the frame that holds the download panel
//! (`target/cpp-ref/src/core/HeritageIntranetFlow.cpp:43-92`).
//!
//! The panel lives in a same-origin child frame, so a script that reads
//! `document.getElementById('searchForm')` on the top frame finds nothing and the run
//! stops without saying why. Every script therefore goes through `wrap()`, which walks
//! the frames, picks the one holding the panel, and rebinds `document` to it.

#[test]
fn every_script_wraps_with_the_frame_walker() {
    for (name, script) in [
        ("login_probe", super::login_probe_script()),
        ("dismiss_tutorial", super::dismiss_tutorial_script()),
        ("open_download_page", super::open_download_page_script()),
        ("agree_terms", super::agree_terms_script()),
        ("search", super::search_script()),
        ("download_all", super::download_all_script()),
        ("page_probe", super::download_page_probe_script()),
    ] {
        assert!(
            script.contains("function walk(w,out,depth)"),
            "{name}: no frame walker in:\n{script}"
        );
        assert!(
            script.contains("var W=pick();var document=W.document;"),
            "{name}: does not rebind document to the chosen frame:\n{script}"
        );
        assert!(
            script.ends_with("})()"),
            "{name}: not an invoked function expression:\n{script}"
        );
    }
}

/// `jsString` (`:125-133`) serialises a one-element JSON array and cuts the brackets, so the
/// escaping is JSON's. A password with a quote must not end the JavaScript string.
#[test]
fn values_are_escaped_like_the_cpp() {
    // A JavaScript string literal keeps its quotes: `user1` without them is an identifier.
    assert_eq!(super::js_string("plain"), r#""plain""#);
    assert_eq!(super::js_string(r#"a\b"#), r#""a\\b""#);
    assert_eq!(super::js_string("a\"b"), r#""a\"b""#);
    assert_eq!(super::js_string("a\nb"), r#""a\nb""#);
    assert_eq!(super::js_string("a\tb"), r#""a\tb""#);
    assert_eq!(super::js_string("가"), r#""가""#, "Korean passes through unescaped");
}

/// The password reaches the page escaped and nowhere else: the script is the only place it may
/// appear, and a raw quote in it would close the string and run whatever follows.
#[test]
fn a_quote_in_the_password_does_not_close_the_script_string() {
    let s = super::login_script("user1", r#"p"w"#);
    assert!(s.contains(r#"p\"w"#), "password not escaped:\n{s}");
    assert!(!s.contains(r#"="p"w""#), "raw password in script:\n{s}");
}

/// The login script calls the site's own `goLogin()`; a plain POST does not log in because the
/// page RSA-encrypts the password into hidden fields (`:135-148`).
#[test]
fn the_login_script_leaves_the_submit_to_the_page() {
    let s = super::login_script("u", "p");
    assert!(s.contains("getElementById('userId')"), "{s}");
    assert!(s.contains("getElementById('pwd')"), "{s}");
    assert!(s.contains("goLogin()"), "does not call the page's own submit:\n{s}");
    assert!(!s.contains("j_spring_security_check"), "posts the form itself:\n{s}");
}
