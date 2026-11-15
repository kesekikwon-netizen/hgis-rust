//! The same run for the portal's public DEM (공개DEM, product 4 of its integrated search): 1:50,000 sheets
//! with five-digit numbers, other kinds of file, and words that name the DEM. The site's side of it is
//! `data/providers/ngii-dem.js`; what the portal answers is described in docs/NETWORK_ACCESS.md.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::tests_support::{config, needs_input, round, FakeHost, TRANSFER};
use super::{AutoConfig, AutoEvent, NgiiAutomation, PageId, Product, Scope, Stage};

fn dem_config(downloads: &Path) -> AutoConfig {
    AutoConfig { product: Product::Dem, ..config(downloads) }
}

/// The circle of a survey; the app names no sheets for a DEM (the portal's list decides), only those it has.
fn dem_scope(cached: &[&str]) -> Scope {
    Scope { x: 1_154_436.0, y: 1_761_737.0, radius: 5000.0, numbers: Vec::new(), cached_numbers: cached.iter().map(|n| (*n).to_owned()).collect() }
}

fn started(downloads: &Path, cached: &[&str]) -> (NgiiAutomation, FakeHost) {
    let mut host = FakeHost::with_portal();
    let mut machine = NgiiAutomation::new(dem_config(downloads));
    machine.prepare_sheets(&mut host, dem_scope(cached), 0);
    (machine, host)
}

const SELECTED: &str = r#"{"state":"selected","message":"","items":[{"num":"35901","name":"경주","year":"2025"},{"num":"35902","name":"불국사","year":"2025"}],
  "downloadItems":[{"num":"35901","name":"경주","year":"2025","source":"NGII demQuery.do"}],"cached":["35902"],"unavailable":[]}"#;

/// The numbers of a DEM run are those of 1:50,000 sheets; the numbers of a sheet run stay six digits.
#[test]
fn a_dem_run_takes_five_digit_sheet_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, _host) = started(dir.path(), &["35902"]);
    assert!(needs_input(&machine.take_events()).is_empty());
    assert_ne!(machine.stage(), Stage::Stopped);

    // A six-digit number is no DEM sheet.
    let mut host = FakeHost::with_portal();
    let mut machine = NgiiAutomation::new(dem_config(dir.path()));
    machine.prepare_sheets(&mut host, dem_scope(&["359012"]), 0);
    assert_eq!(needs_input(&machine.take_events()), vec!["자동 선택할 공개DEM 도엽 목록과 반경을 확인하지 못했습니다."]);

    // What is handed on as a DEM is what the app reads: the portal's ZIP and the image inside it (seen
    // 2026-11-15: `(B080)공개DEM_35901_img_2025.zip` holding `35901/35901.img`). Other kinds are notes.
    let handed = |name: &str| Product::Dem.is_data_file(Path::new(name));
    assert!(handed("(B080)공개DEM_35901_img_2025.zip") && handed("35901.IMG"));
    assert!(!handed("35901.tif") && !handed("35901.txt") && !handed("35901.xml") && !handed("35901.asc"));

    // And a five-digit one is no 1:25,000 sheet.
    let mut machine = NgiiAutomation::new(config(dir.path()));
    machine.prepare_sheets(&mut host, dem_scope(&["35902"]), 0);
    assert_eq!(needs_input(&machine.take_events()), vec!["자동 선택할 1:25,000 도엽 목록과 반경을 확인하지 못했습니다."]);
}

/// The raster of a DEM (whatever kind the portal sends: an archive, an image, a grid) is handed on with
/// the portal's record of its sheet; the notes beside it are not.
#[test]
fn the_raster_of_a_dem_is_handed_on_and_the_notes_beside_it_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path(), &["35902"]);
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"submitted"}"#);
    host.open(2, "about:blank", Some(1));
    machine.page_load_started(&host, PageId(2));
    host.open(2, TRANSFER, Some(1));
    machine.page_loaded(&mut host, PageId(2), now);
    let token = host.token_of("window.name === 'downForm'");
    machine.script_answered(&mut host, token, "true", now);
    round(&mut machine, &mut host, &mut now, r#"{"state":"orderReady"}"#);
    let files = r#"{"state":"ready","files":3,"manifest":[
      {"name":"35901.img","id":"a1","number":"35901","bytes":52000000},
      {"name":"35901.xml","id":"a2","number":"35901","bytes":900},
      {"name":"35901.prj","id":"a3","number":"35901","bytes":400}]}"#;
    round(&mut machine, &mut host, &mut now, files);
    round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    assert_eq!(machine.stage(), Stage::Transfer);
    machine.take_events();

    let mut taken = Vec::new();
    for name in ["35901.img", "35901.xml", "35901.prj"] {
        let target = machine.download_offered(&mut host, PageId(2), name, now).expect("the file is taken");
        let path = target.directory.join(&target.file_name);
        std::fs::write(&path, b"data").unwrap();
        machine.download_finished(&mut host, &path, now);
        taken.push(path);
        if name != "35901.prj" {
            round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
        }
    }
    let events = machine.take_events();
    let handed: Vec<(&Path, &str, &str, &str)> =
        events.iter().filter_map(|e| if let AutoEvent::SheetDownloaded { path, record } = e { Some((path.as_path(), record["num"].as_str().unwrap(), record["year"].as_str().unwrap(), record["source"].as_str().unwrap())) } else { None }).collect();
    // The record names the portal's DEM list as where it is from, not the list of the topographic sheets:
    // the app takes only such a record for a DEM (dem_post.rs).
    assert_eq!(handed, vec![(taken[0].as_path(), "35901", "2025", "NGII demQuery.do")]);
    assert_eq!((machine.stage(), machine.status()), (Stage::Complete, "선택한 DEM 파일을 모두 받았습니다. 지도에 올릴 준비를 합니다."));
}

/// Nothing offered for the circle, and everything already there: said in the DEM's words.
#[test]
fn the_words_of_a_dem_run_name_the_dem() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path(), &[]);
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, r#"{"state":"unavailable","message":"","items":[],"downloadItems":[],"cached":[],"unavailable":[]}"#);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["이 범위에서 제공 중인 공개DEM 이 없습니다."]);

    let (mut machine, mut host) = started(dir.path(), &["35901", "35902"]);
    let mut now = 0;
    let cached = r#"{"state":"cached","message":"","items":[{"num":"35901","name":"경주","year":"2025"},{"num":"35902","name":"불국사","year":"2025"}],"downloadItems":[],"cached":["35901","35902"],"unavailable":[]}"#;
    round(&mut machine, &mut host, &mut now, cached);
    assert_eq!((machine.stage(), machine.status()), (Stage::Complete, "필요한 DEM 을 모두 보관하고 있습니다. 다시 신청하지 않습니다."));
}
