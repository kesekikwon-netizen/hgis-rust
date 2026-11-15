//! The list of received DEM files and what the 「DEM」 button does with a finished run.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;

use serde_json::json;

use super::*;
use crate::download_jobs::DownloadJobState;
use crate::ref_vector::data::Look;
use crate::topo_download::{topo_request, OrderOf, TopoFacts};

fn entry(num: &str, year: &str, file: &str) -> DemEntry {
    DemEntry { num: num.to_owned(), name: "경주".to_owned(), year: year.to_owned(), file: file.into() }
}

fn record(num: &str, year: &str) -> Map<String, Value> {
    json!({"source": "NGII demQuery.do", "num": num, "name": "경주", "year": year, "minx": 1_135_268.9, "miny": 1_751_751.6, "maxx": 1_158_300.5, "maxy": 1_779_858.7}).as_object().cloned().unwrap()
}

/// A sheet's DEM as the portal sends it: `(B080)공개DEM_<도엽>_img_<연도>.zip` with `<도엽>/<도엽>.img` inside
/// (seen on 2026-11-15 for sheet 35901).
fn portal_zip(folder: &Path, num: &str) -> PathBuf {
    std::fs::create_dir_all(folder).unwrap();
    let path = folder.join(format!("(B080)공개DEM_{num}_img_2025.zip"));
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(format!("{num}/{num}.img"), options).unwrap();
    writer.write_all(b"raster").unwrap();
    writer.finish().unwrap();
    path
}

/// What is written is read back; a sheet is listed once (the newer entry takes the place of the older);
/// a sheet counts as received while its file is there; a list that cannot be read is an empty one.
#[test]
fn the_list_of_received_dem_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("DEM");
    assert!(read_list(&root).is_empty() && received_numbers(&root).is_empty() && files_of(&root).is_empty());
    std::fs::create_dir_all(root.join("원본/a")).unwrap();
    std::fs::write(root.join("원본/a/35901.img"), b"x").unwrap();
    add_to_list(&root, &[entry("35901", "2024", "원본/old/35901.img"), entry("35902", "2025", "원본/gone/35902.img")]).unwrap();
    add_to_list(&root, &[entry("35901", "2025", "원본/a/35901.img")]).unwrap();
    assert_eq!(read_list(&root), [entry("35901", "2025", "원본/a/35901.img"), entry("35902", "2025", "원본/gone/35902.img")]);
    assert_eq!(received_numbers(&root), ["35901"]);
    assert_eq!(files_of(&root), [root.join("원본/a/35901.img")]);

    std::fs::write(root.join(LIST_FILE), "not json").unwrap();
    assert!(read_list(&root).is_empty());
}

/// The portal sends a sheet's DEM as a ZIP. It is unpacked inside the DEM folder (the ZIP itself is left as
/// it came) and the raster inside becomes the entry of the sheet. A ZIP that cannot be unpacked is said by
/// the number of its sheet; a file the portal has no DEM record for is left out.
#[test]
fn a_received_zip_is_unpacked_and_its_raster_listed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("DEM");
    let zip = portal_zip(&root.join("원본/a"), "35901");
    let broken = root.join("원본/b/(B080)공개DEM_35902_img_2025.zip");
    std::fs::create_dir_all(broken.parent().unwrap()).unwrap();
    std::fs::write(&broken, b"this is no archive").unwrap();
    let stray = (root.join("원본/c/unknown.img"), json!({"name": "x"}).as_object().cloned().unwrap());
    let received = vec![(zip.clone(), record("35901", "2025")), (broken, record("35902", "2025")), stray];

    let (entries, problems) = entries_of(&root, &received);
    assert_eq!(entries.len(), 1, "{entries:?}");
    let listed = &entries[0];
    assert_eq!((listed.num.as_str(), listed.name.as_str(), listed.year.as_str()), ("35901", "경주", "2025"));
    assert!(listed.file.starts_with("풀림") && listed.file.ends_with("payload/35901/35901.img"), "{:?}", listed.file);
    assert_eq!(std::fs::read(root.join(&listed.file)).unwrap(), b"raster");
    assert!(zip.is_file(), "the received ZIP stays");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].starts_with("35902 · "), "{problems:?}");
}

/// The DEM of a sheet the survey has is named to the portal as one it has: it is not ordered again. The
/// app names no sheets itself (the portal's list says which 1:50,000 sheets hold the circle), and the
/// files go to <survey folder>/DEM/원본.
#[test]
fn a_dem_press_asks_for_the_circle_and_names_what_the_survey_has() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let root = dir.path().join("DEM");
    std::fs::create_dir_all(root.join("원본/a")).unwrap();
    std::fs::write(root.join("원본/a/35901.img"), b"x").unwrap();
    add_to_list(&root, &[entry("35901", "2025", "원본/a/35901.img")]).unwrap();

    let facts = TopoFacts { survey_path: Some(&survey), centre_work: Some([220_000.0, 370_000.0]), work_crs: strata_types::crs::WorkCrs::Epsg5187, online: true };
    let request = topo_request(OrderOf::Dem, facts, || strata_secrets::accounts::Credentials::new("tester", "pw-for-test")).unwrap();
    assert_eq!((request.scope.numbers.len(), request.scope.cached_numbers.clone(), request.scope.radius), (0, vec!["35901".to_owned()], 5000.0));
    assert_eq!(request.downloads, root.join("원본"));

    // Without a survey the DEM's own words say where it would go.
    let none = TopoFacts { survey_path: None, centre_work: None, work_crs: strata_types::crs::WorkCrs::Epsg5187, online: true };
    let refused = topo_request(OrderOf::Dem, none, || panic!("no account is read")).unwrap_err();
    assert_eq!(refused.1, "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다.");
}

fn app_with_survey(dir: &Path) -> StrataAppState {
    let survey = dir.join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    app
}

fn rows(app: &StrataAppState) -> Vec<(String, String, f32, bool)> {
    let visible = |name: &str| app.home_map.real_survey.is_layer_visible(name);
    app.left_layers.entries.iter().filter(|e| e.layer_key.starts_with("ref_dem")).map(|e| (e.layer_key.clone(), e.label_ko.clone(), e.opacity, visible(&e.label_ko))).collect()
}

/// A finished DEM run: its sheet is in the list, and the legend has 「지형 음영」 (30 %, as
/// `BasemapOps.cpp:2084`) directly over 「DEM」, both checked and both read-only reference rows; the
/// gauge and the status line say so.
#[test]
fn a_finished_dem_run_is_listed_and_put_on_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("DEM");
    let zip = portal_zip(&root.join("원본/a"), "35901");
    let mut app = app_with_survey(dir.path());
    assert!(!app.dem_shown(), "the button is no switch before there is a DEM (`m_btnDem->setChecked`)");
    let (_, job) = app.download_jobs.add_job("DEM", DownloadJobState::Running { fraction: None, stage: None }, 0.0);
    assert!(app.begin_dem_post(vec![(zip, record("35901", "2025"))], job).is_none());
    assert!(app.dem_shown());
    app.download_jobs.tick(1.0);
    assert_eq!(app.download_jobs.jobs()[0].state, DownloadJobState::Done { result: "DEM을 준비했습니다.".to_owned() });
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("DEM을 준비했습니다. 도엽 1장을 지도에 올렸습니다."));
    assert_eq!(received_numbers(&root), ["35901"]);
    assert_eq!(rows(&app), [("ref_dem_relief".to_owned(), "지형 음영".to_owned(), 0.30, true), ("ref_dem".to_owned(), "DEM".to_owned(), 1.0, true)]);
    let at = |key: &str| app.left_layers.entries.iter().position(|e| e.layer_key == key).unwrap();
    assert_eq!(at("ref_dem"), at("ref_dem_relief") + 1, "the relief lies directly over the DEM");
    assert_eq!((app.ref_vectors.look_of("ref_dem"), app.ref_vectors.look_of("ref_dem_relief")), (Some(&Look::Heights { ramp: crate::ref_vector::data::HeightRamp::National }), Some(&Look::Relief { z_factor: 1.0 })));
    assert!(app.left_layers.entries.iter().filter(|e| e.layer_key.starts_with("ref_dem")).all(|e| e.read_only && e.group == strata_types::schema::LegendGroup::ReferenceMap));

    // The button now hides and shows both (`MainWindowOverlay.cpp:315-340`).
    app.handle_ribbon_click("DEM");
    assert_eq!(rows(&app).iter().map(|row| row.3).collect::<Vec<_>>(), [false, false]);
    assert_eq!(app.status_bar_hint, "DEM 지형을 숨겼습니다.");
    assert!(!app.dem_shown());
    app.handle_ribbon_click("DEM");
    assert_eq!(rows(&app).iter().map(|row| row.3).collect::<Vec<_>>(), [true, true]);
    assert!(!app.topographic_download_busy(), "a DEM on the map is not ordered again");

    // The rows are found by what they are, not by their names: the user may have renamed them.
    for (key, name) in [("ref_dem_relief", "내 음영"), ("ref_dem", "내 표고")] {
        app.left_layers.entries.iter_mut().find(|e| e.layer_key == key).unwrap().label_ko = name.to_owned();
        app.home_map.real_survey.layer_visible.insert(name.to_owned(), true);
    }
    app.handle_ribbon_click("DEM");
    assert_eq!(rows(&app).iter().map(|row| row.3).collect::<Vec<_>>(), [false, false]);
    app.handle_ribbon_click("DEM");
    assert_eq!(rows(&app).iter().map(|row| row.3).collect::<Vec<_>>(), [true, true]);
}

/// A run that ends with nothing new shows the DEM the survey has; one that ends with nothing at all, or
/// with an archive that cannot be unpacked, says so and adds no row.
#[test]
fn a_run_without_a_new_file_shows_what_the_survey_has_or_says_there_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("DEM");
    let mut app = app_with_survey(dir.path());
    let (_, job) = app.download_jobs.add_job("DEM", DownloadJobState::Running { fraction: None, stage: None }, 0.0);
    assert!(app.begin_dem_post(Vec::new(), job).is_none());
    app.download_jobs.tick(1.0);
    assert_eq!(app.download_jobs.jobs()[0].state, DownloadJobState::Done { result: "지도에 올릴 DEM 이 없습니다.".to_owned() });
    assert!(rows(&app).is_empty());

    let broken = root.join("원본/b/(B080)공개DEM_35902_img_2025.zip");
    std::fs::create_dir_all(broken.parent().unwrap()).unwrap();
    std::fs::write(&broken, b"this is no archive").unwrap();
    let (_, job) = app.download_jobs.add_job("DEM", DownloadJobState::Running { fraction: None, stage: None }, 2.0);
    assert!(app.begin_dem_post(vec![(broken, record("35902", "2025"))], job).is_none());
    app.download_jobs.tick(3.0);
    let stopped = app.download_jobs.jobs().iter().find_map(|job| if let DownloadJobState::Stopped { code, reason } = &job.state { Some((*code, reason.clone())) } else { None }).unwrap();
    assert!(stopped.0 == StopCode::Data1 && stopped.1.starts_with("35902 · "), "{stopped:?}");
    assert!(rows(&app).is_empty());

    // The survey has a sheet from an earlier run: the portal sent nothing new, the sheet goes on the map.
    std::fs::create_dir_all(root.join("원본/a")).unwrap();
    std::fs::write(root.join("원본/a/35901.img"), b"x").unwrap();
    add_to_list(&root, &[entry("35901", "2025", "원본/a/35901.img")]).unwrap();
    let (_, job) = app.download_jobs.add_job("DEM", DownloadJobState::Running { fraction: None, stage: None }, 4.0);
    assert!(app.begin_dem_post(Vec::new(), job).is_none());
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("DEM을 준비했습니다. 도엽 1장을 지도에 올렸습니다."));
    assert_eq!(rows(&app).len(), 2);
}
