//! Local recorded/fake printer messages only; no OS enumeration or printing from tests.
use super::*;

fn recorded() -> Vec<u8> {
    br#"{"version":1,"printers":[{"name":"FAKE-CATALOG-PRINTER","is_default":true,"file_output":false,"papers":[{"name":"A3","width_mm":297.0,"height_mm":420.0,"portrait":[5.0,5.0,287.0,410.0],"landscape":[5.0,5.0,410.0,287.0]}]}]}"#.to_vec()
}

#[test]
fn recorded_catalog_preserves_real_names_default_and_physical_paper() {
    let printers = parse_catalog(&recorded()).expect("recorded OS response");
    assert_eq!(printers.len(), 1);
    assert_eq!(printers[0].name, "FAKE-CATALOG-PRINTER");
    assert!(printers[0].is_default);
    assert_eq!(printers[0].papers[0].portrait, [5.0, 5.0, 287.0, 410.0]);
    assert!(parse_catalog(br#"{"version":1,"printers":[]}"#).expect("empty catalog is real").is_empty());
}

#[test]
fn catalog_boundary_rejects_bad_version_and_invalid_physical_dimensions() {
    let wrong_version = String::from_utf8(recorded()).expect("ASCII fixture").replace("\"version\":1", "\"version\":2");
    assert!(parse_catalog(wrong_version.as_bytes()).is_err());
    let invalid = String::from_utf8(recorded()).expect("ASCII fixture").replace("\"width_mm\":297.0", "\"width_mm\":-1.0");
    assert!(parse_catalog(invalid.as_bytes()).is_err());
    assert!(parse_catalog(b"not JSON").is_err());
}

#[test]
fn dispatch_receipt_checks_actual_device_paper_printer_and_page_count() {
    let printer = parse_catalog(&recorded()).expect("recorded catalog").remove(0);
    let paper = printer.papers[0].clone();
    let plan = super::super::print_plan::build_plan([297.0, 420.0], &paper, super::super::print_plan::PrintMode::Single).expect("known physical paper");
    let request = PrintRequest { printer: printer.name, title: "FAKE-SHEET".to_owned(), paper, plan, drawing_mm: [297.0, 420.0], dpi: 300, output: None };
    let good = br#"{"version":1,"printer":"FAKE-CATALOG-PRINTER","pages":1,"paper_mm":[296.926,420.116]}"#;
    assert_eq!(validate_receipt(&request, good).expect("actual OS rounding within C++ 2mm tolerance").pages, 1);
    let text = std::str::from_utf8(good).expect("ASCII fixture");
    for bad in [text.replace("296.926,420.116", "210,297"), text.replace("\"pages\":1", "\"pages\":0"), text.replace("FAKE-CATALOG-PRINTER", "WRONG-PRINTER")] {
        assert!(validate_receipt(&request, bad.as_bytes()).is_err(), "false dispatch success: {bad}");
    }
}
