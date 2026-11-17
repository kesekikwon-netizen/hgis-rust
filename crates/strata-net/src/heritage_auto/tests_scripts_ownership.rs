use super::*;

#[test]
fn browser_json_preserves_only_typed_document_facts() {
    let raw = r#"{"form":true,"agreement":false,"login":false,"menu":true,"authenticated":true}"#;
    let browser = r#""{\"form\":true,\"agreement\":false,\"login\":false,\"menu\":true,\"authenticated\":true}""#;
    let expected = DocumentFacts {
        form: true,
        agreement: false,
        login: false,
        menu: true,
        authenticated: true,
    };
    assert_eq!(parse_document_probe(raw), Some(expected));
    assert_eq!(parse_document_probe(browser), Some(expected));
}

#[test]
fn missing_or_wrongly_typed_document_facts_are_unavailable() {
    for reply in [
        "null",
        "undefined",
        r#"{"form":true}"#,
        r#"{"form":"true","agreement":false,"login":false,"menu":false,"authenticated":false}"#,
    ] {
        assert_eq!(parse_document_probe(reply), None, "{reply}");
    }
}
