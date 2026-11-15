//! The 지질 button from the press to the legend row, on answers made here (no request leaves the test):
//! `target/cpp-ref/src/app/MainWindowOverlay.cpp:221-236` (hide and show), `:405-472`, `:529-680` (fetch),
//! and the owner's rules of 2026-11-15 (the survey area decides the area; a kept map is not fetched again).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui;
use serde_json::json;
use strata_engine::Transformer;
use strata_refmap::transfer::{Request, TransferFailure};
use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::{DomainLayerKey, LegendGroup};

use super::run::RefMapJobs;
use super::*;
use crate::download_jobs::{DownloadJobState, StopCode};

/// A survey area of 100 m near Gyeongju, EPSG:5187 (the work CRS of the tests).
pub(super) const CENTRE: [f64; 2] = [220_000.0, 370_000.0];

pub(super) fn draw_survey_area(app: &mut StrataAppState, centre: [f64; 2], srs_id: i32) {
    let (x, y) = (centre[0], centre[1]);
    let wkt = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", x - 50.0, y - 50.0, x + 50.0, y - 50.0, x + 50.0, y + 50.0, x - 50.0, y + 50.0, x - 50.0, y - 50.0);
    let mut layer = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
    layer.features.push(FeatureRecord { fid: 1, wkt_geom: wkt, attributes: Default::default() });
    app.survey_tables.retain(|t| t.table != "survey_area");
    app.survey_tables.push(SurveyFileTable { table: "survey_area".to_owned(), srs_id, geometry_type: "POLYGON".to_owned(), columns: Vec::new(), baseline: layer.features.clone(), layer });
    // As in the app: a drawn survey area has its row in the legend, above everything else.
    app.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
}

/// An answer of the service: one unit of 200 m at the survey (coordinates in EPSG:5186), or the picture.
pub(super) fn kigam(request: &Request) -> Result<Vec<u8>, TransferFailure> {
    if request.url.contains("request=GetMap") {
        let side = |name: &str| request.url.split('&').find_map(|p| p.strip_prefix(name)).and_then(|v| v.parse::<u32>().ok()).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(side("width="), side("height="), image::Rgba([250, 244, 180, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        return Ok(bytes.into_inner());
    }
    let at = Transformer::new(5187, 5186).unwrap().transform(CENTRE).unwrap();
    let ring = vec![[at[0] - 100.0, at[1] - 100.0], [at[0] + 100.0, at[1] - 100.0], [at[0] + 100.0, at[1] + 100.0], [at[0] - 100.0, at[1] + 100.0], [at[0] - 100.0, at[1] - 100.0]];
    let unit = json!({"type": "Feature", "id": "litho.1", "properties": {"기호": "Qa", "지층": "충적층", "시대": "현생누대 신생대 제4기"}, "geometry": {"type": "Polygon", "coordinates": [ring]}});
    Ok(json!({"type": "FeatureCollection", "crs": {"type": "name", "properties": {"name": "EPSG:5186"}}, "features": [unit]}).to_string().into_bytes())
}

pub(super) struct Bench {
    pub(super) app: StrataAppState,
    pub(super) dir: tempfile::TempDir,
    pub(super) requests: Arc<AtomicUsize>,
    ctx: egui::Context,
}

/// An app with a saved survey in a temp folder and a drawn survey area; answers come from `answer`.
pub(super) fn bench(answer: fn(&Request) -> Result<Vec<u8>, TransferFailure>, online: fn() -> bool) -> Bench {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    draw_survey_area(&mut app, CENTRE, 5187);
    let requests = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&requests);
    app.ref_maps = RefMapJobs::for_test(
        Arc::new(move |request, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            answer(request)
        }),
        online,
    );
    app.ref_vectors.run_at_once();
    Bench { app, dir, requests, ctx: egui::Context::default() }
}

impl Bench {
    pub(super) fn frame(&mut self, now: f64) {
        self.app.drive_reference_maps(&self.ctx, now);
        self.app.download_jobs.tick(now);
    }
    pub(super) fn status(&self) -> &str {
        self.app.home_map.draw_guide_message.as_deref().unwrap_or_default()
    }
    pub(super) fn has_row(&self) -> bool {
        self.app.left_layers.entries.iter().any(|e| e.layer_key == GEOLOGY.key)
    }
    pub(super) fn stored_maps(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(self.dir.path()).unwrap().flatten() {
            let file = entry.path().join(GEOLOGY.file_name);
            if file.is_file() {
                found.push(file);
            }
        }
        found
    }
}

/// Work that was asked for and not run yet.
pub(super) type Held = Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>;

fn no_answer(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    panic!("no request may be sent here")
}

#[test]
fn without_a_saved_survey_or_a_survey_area_nothing_starts() {
    let mut b = bench(no_answer, || true);
    b.app.survey_path = None;
    b.app.press_reference_map(&GEOLOGY);
    assert_eq!(b.status(), "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.");

    b.app.survey_path = Some(b.dir.path().join("현장.gpkg"));
    b.app.survey_tables.clear();
    b.app.press_reference_map(&GEOLOGY);
    assert_eq!(b.status(), "조사구역을 먼저 그려 주세요. 조사구역에서 반경 10km 안의 지질도를 받습니다.");
    b.frame(1.0);
    assert!(b.app.download_jobs.is_empty() && !b.has_row() && !b.app.ref_maps.is_busy());
    assert_eq!(b.app.active_tab, crate::app_state::MainTab::Map);
}

#[test]
fn with_the_network_off_the_gauge_says_why() {
    let mut b = bench(no_answer, || false);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    let jobs = b.app.download_jobs.jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "지질도");
    assert_eq!(jobs[0].state, DownloadJobState::Stopped { code: StopCode::Site1, reason: "인터넷 접속이 꺼져 있어 지질도 서버에 연결할 수 없습니다.".to_owned() });
    assert!(!b.has_row());
}

/// The press, one frame, and the map is there: its row in the reference part of the legend, the file in a
/// folder of its own in the survey folder, the gauge done with the C++ words.
#[test]
fn a_press_fetches_the_map_and_puts_its_row_in_the_legend() {
    let mut b = bench(kigam, || true);
    let survey_row = b.app.left_layers.entries[0].label_ko.clone();
    b.app.press_reference_map(&GEOLOGY);
    assert!(b.app.home_map.real_survey.is_layer_visible(&survey_row), "the press does not touch another row");
    assert_eq!(b.status(), "지질도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.");
    assert!(b.app.ref_maps.is_busy());
    b.frame(1.0);
    assert!(!b.app.ref_maps.is_busy());
    assert_eq!(b.requests.load(Ordering::SeqCst), 2, "one request for the units, one for the picture");

    let row = b.app.left_layers.entries.iter().find(|e| e.layer_key == "ref_geology").expect("row");
    assert_eq!((row.label_ko.as_str(), row.group), ("지질도(KIGAM 1:5만)", LegendGroup::ReferenceMap));
    assert!(b.app.home_map.real_survey.is_layer_visible("지질도(KIGAM 1:5만)"));
    let jobs = b.app.download_jobs.jobs();
    assert_eq!((jobs.len(), jobs[0].title.as_str()), (1, "지질도"));
    assert_eq!(jobs[0].state, DownloadJobState::Done { result: "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned() });
    assert_eq!(b.status(), "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.");

    let maps = b.stored_maps();
    assert_eq!(maps.len(), 1);
    let folder = maps[0].parent().unwrap();
    assert!(folder.file_name().unwrap().to_str().unwrap().starts_with("ka-hgis-reference-"));
    assert!(folder.join("ka-hgis-scope.json").is_file());
    assert!(!b.dir.path().join(GEOLOGY.file_name).exists());
    // The layer draws that file, in the work CRS, coloured by the stored colour.
    let spec = b.app.ref_vectors.spec_of("ref_geology").expect("layer");
    assert_eq!((spec.file.as_path(), spec.table.as_str(), spec.work_epsg), (maps[0].as_path(), "geology_map", 5187));
    assert_eq!(spec.look, GEOLOGY.look);
    // The symbol of every unit is written over it: Malgun Gothic 8 pt in 40/40/40 with a white edge of
    // 0.6 mm at alpha 220 (`GeologyMapService.cpp:426-455`).
    assert_eq!(spec.label, Some(crate::ref_vector::data::LabelLook { column: "기호", size_pt: 8.0, ink: [40, 40, 40], halo_mm: 0.6, halo: [255, 255, 255, 220] }));
}

#[test]
fn a_second_press_hides_the_row_and_a_third_shows_it_again() {
    let mut b = bench(kigam, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    b.app.press_reference_map(&GEOLOGY);
    assert!(!b.app.home_map.real_survey.is_layer_visible("지질도(KIGAM 1:5만)"));
    assert_eq!(b.status(), "지질도(KIGAM 1:5만)를 껐습니다.");
    b.app.press_reference_map(&GEOLOGY);
    assert!(b.app.home_map.real_survey.is_layer_visible("지질도(KIGAM 1:5만)"));
    assert_eq!(b.status(), "지질도(KIGAM 1:5만)를 다시 켰습니다.");
    b.frame(2.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 2, "nothing is fetched again");
    assert_eq!(b.app.download_jobs.len(), 1);
}

/// A map kept for this very area is put on the map without a request; after the survey area has moved the
/// kept map is no longer the map of this area and a new one is fetched.
#[test]
fn a_map_kept_for_the_same_area_is_not_fetched_again() {
    let mut b = bench(kigam, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    // The survey is opened again later: the rows of reference maps are gone, the files are not.
    b.app.remove_reference_vector_rows();
    assert!(!b.has_row());
    b.app.press_reference_map(&GEOLOGY);
    assert!(b.has_row());
    assert_eq!(b.status(), "보관해 둔 지질도를 올렸습니다. 다시 누르면 숨깁니다.");
    b.frame(2.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 2, "no request for a kept map");
    assert_eq!(b.app.download_jobs.len(), 1, "and no new line in the gauge");

    b.app.remove_reference_vector_rows();
    draw_survey_area(&mut b.app, [CENTRE[0] + 3_000.0, CENTRE[1]], 5187);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(3.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 4, "another area: fetched");
    assert_eq!(b.stored_maps().len(), 2, "the older map stays where it is");
}

fn error_page(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    Ok(b"<ServiceExceptionReport>unavailable</ServiceExceptionReport>".to_vec())
}

fn nothing_here(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    Ok(br#"{"type":"FeatureCollection","features":[]}"#.to_vec())
}

/// A fetch that fails ends in the gauge with the code of its kind and the C++ words; no row, no file.
#[test]
fn a_failure_goes_to_the_gauge_with_its_code_and_adds_no_row() {
    let mut b = bench(error_page, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    assert_eq!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Stopped { code: StopCode::Site1, reason: "KIGAM 서버 연결 실패: 서버가 GeoJSON 대신 다른 응답을 보냈습니다.".to_owned() });
    assert!(!b.has_row() && b.stored_maps().is_empty() && !b.app.ref_maps.is_busy());

    let mut b = bench(nothing_here, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    assert_eq!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Stopped { code: StopCode::None1, reason: "이 범위에는 지질도 데이터가 없습니다.".to_owned() });
    assert!(!b.has_row());
}

/// While a map is being fetched a second press says so; the gauge's own cancel stops the fetch, and the
/// status line says it with the C++ words.
#[test]
fn a_press_during_a_fetch_says_busy_and_a_cancel_stops_it() {
    let mut b = bench(kigam, || true);
    let held: Held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    b.app.ref_maps.set_runner(Arc::new(move |work| keep.lock().unwrap().push(work)));
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    assert!(b.app.ref_maps.is_busy());
    assert!(matches!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Running { .. }));
    b.app.press_reference_map(&GEOLOGY);
    assert_eq!(b.status(), "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요.");
    assert_eq!(held.lock().unwrap().len(), 1, "no second fetch");

    let id = b.app.download_jobs.jobs()[0].id;
    assert!(b.app.download_jobs.request_cancel(id));
    let work = held.lock().unwrap().pop().unwrap();
    work();
    b.frame(2.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 0, "a fetch stopped before it began asks nothing");
    assert_eq!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Cancelled);
    assert_eq!(b.status(), "지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다.");
    assert!(!b.has_row() && b.stored_maps().is_empty());
}
