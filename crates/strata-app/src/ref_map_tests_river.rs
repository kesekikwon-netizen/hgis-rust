//! The 수계 button from the press to the legend row, on answers made here (no request leaves the test and
//! no stored key is read: `TEST_KEY` is the key of the C++ tests):
//! `target/cpp-ref/src/app/MainWindowOverlay.cpp:550-596` (`downloadRiverMap`), `:604-680`, and the look of
//! the layer in `target/cpp-ref/src/core/RiverMapService.cpp:67-80`, `:189-260`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::Ordering;

use serde_json::json;
use strata_engine::Transformer;
use strata_refmap::transfer::{Request, TransferFailure};
use strata_types::schema::LegendGroup;

use super::tests::{bench, CENTRE};
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::ref_vector::data::{LabelLook, Look, PolygonFill};

/// VWorld: one national river of 0.001 degrees at the survey, in longitude and latitude. The request must
/// carry the key, the address the key is registered for, and the program's own name.
fn vworld(request: &Request) -> Result<Vec<u8>, TransferFailure> {
    assert!(request.url.starts_with("https://api.vworld.kr/req/wfs?") && request.url.ends_with("&KEY=TEST_KEY"), "the request of the river network");
    assert_eq!((request.user_agent, request.referer), ("ka-hgis/0.3", Some("https://localhost")));
    let [lon, lat] = Transformer::new(5187, 4326).unwrap().transform(CENTRE).unwrap();
    let ring = [[lon, lat], [lon + 0.001, lat], [lon + 0.001, lat + 0.001], [lon, lat + 0.001], [lon, lat]];
    let river = json!({"type": "Feature", "id": "lt_c_wkmstrm.1", "properties": {"riv_nm": "시험하천", "riv_level": "국가하천"}, "geometry": {"type": "Polygon", "coordinates": [ring]}});
    Ok(json!({"type": "FeatureCollection", "features": [river]}).to_string().into_bytes())
}

/// A press of 수계 fetches the river network for the 10 km around the survey area with the user's VWorld
/// key, stores it in the survey folder and puts 「수계도(하천망)」 into the legend, in the water colours of
/// the C++ with the river names. A second press hides the row, a third shows it; nothing is fetched again.
#[test]
fn the_river_button_fetches_the_map_and_then_hides_and_shows_it() {
    let mut b = bench(vworld, || true);
    b.app.ref_maps.set_key(|| "TEST_KEY".to_owned());
    b.app.handle_ribbon_click("수계");
    assert_eq!(b.status(), "수계도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.");
    b.frame(1.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 1, "one page for the box of the scope");

    let row = b.app.left_layers.entries.iter().find(|e| e.layer_key == "ref_river").expect("row");
    assert_eq!((row.label_ko.as_str(), row.group), ("수계도(하천망)", LegendGroup::ReferenceMap));
    assert!(b.app.home_map.real_survey.is_layer_visible("수계도(하천망)"));
    let jobs = b.app.download_jobs.jobs();
    assert_eq!((jobs.len(), jobs[0].title.as_str()), (1, "수계도"));
    assert_eq!(jobs[0].state, DownloadJobState::Done { result: "수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned() });
    assert_eq!(b.status(), "수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.");

    let stored: Vec<_> = std::fs::read_dir(b.dir.path()).unwrap().flatten().map(|e| e.path().join("수계도_VWorld.gpkg")).filter(|f| f.is_file()).collect();
    assert_eq!(stored.len(), 1);
    // Neither the note beside the map nor its folder's name holds the key.
    let folder = stored[0].parent().unwrap();
    assert!(!std::fs::read_to_string(folder.join("ka-hgis-scope.json")).unwrap().contains("TEST_KEY") && !folder.to_string_lossy().contains("TEST_KEY"));
    let spec = b.app.ref_vectors.spec_of("ref_river").expect("layer");
    assert_eq!((spec.file.as_path(), spec.table.as_str(), spec.work_epsg), (stored[0].as_path(), "river_map", 5187));
    // Fill at alpha 230 in the colour of the river's class, outline that colour made darker, opaque,
    // 0.32 mm (`waterFillSymbol`, RiverMapService.cpp:67-80); a river without a class is the light blue
    // of 「기타 수계」 (:218).
    assert_eq!(spec.look, Look::Polygons { stroke: [80, 131, 157, 255], width_mm: 0.32, fill: PolygonFill::Columns { fill: "ka_color", line: "ka_line", alpha: 230, missing: [120, 196, 236] } });
    // The river's name: Malgun Gothic 8 pt bold in 0/72/186 with a white edge of 0.7 at alpha 225 (:224-252).
    assert_eq!(spec.label, Some(LabelLook { column: "riv_nm", size_pt: 8.0, ink: [0, 72, 186], halo_mm: 0.7, halo: [255, 255, 255, 225], bold: true, closest_scale: None, inside_only: false }));

    b.app.handle_ribbon_click("수계");
    assert!(!b.app.home_map.real_survey.is_layer_visible("수계도(하천망)"));
    assert_eq!(b.status(), "수계도(하천망)를 껐습니다.");
    b.app.handle_ribbon_click("수계");
    assert!(b.app.home_map.real_survey.is_layer_visible("수계도(하천망)"));
    assert_eq!(b.status(), "수계도(하천망)를 다시 켰습니다.");
    b.frame(2.0);
    assert_eq!((b.requests.load(Ordering::SeqCst), b.app.download_jobs.len()), (1, 1), "nothing is fetched again");
}

fn no_answer(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    panic!("no request may be sent here")
}

/// `:563-567`: without a stored key nothing is asked; the status line and the gauge (KEY-1) say what to do.
#[test]
fn without_a_stored_key_the_river_button_says_where_to_put_one() {
    let mut b = bench(no_answer, || true);
    b.app.handle_ribbon_click("수계");
    assert_eq!(b.status(), "VWorld 인증키가 없습니다. 배경지도 설정에서 키를 먼저 등록하세요.");
    b.frame(1.0);
    let jobs = b.app.download_jobs.jobs();
    assert_eq!((jobs.len(), jobs[0].title.as_str()), (1, "수계도"));
    assert_eq!(jobs[0].state, DownloadJobState::Stopped { code: StopCode::Key1, reason: "VWorld 인증키가 없습니다. 배경지도 설정에서 키를 먼저 등록하세요.".to_owned() });
    assert!(!b.app.left_layers.entries.iter().any(|e| e.layer_key == "ref_river"));
}

/// VWorld refuses the key: the gauge says so with its code (KEY-2), and no sentence holds the key.
#[test]
fn a_refused_key_goes_to_the_gauge_with_its_code() {
    fn refused(_: &Request) -> Result<Vec<u8>, TransferFailure> {
        Ok(br#"<ServiceExceptionReport><ServiceException code="INCORRECT_KEY">INCORRECT_KEY : TEST_KEY</ServiceException></ServiceExceptionReport>"#.to_vec())
    }
    let mut b = bench(refused, || true);
    b.app.ref_maps.set_key(|| "TEST_KEY".to_owned());
    b.app.handle_ribbon_click("수계");
    b.frame(1.0);
    let jobs = b.app.download_jobs.jobs();
    let DownloadJobState::Stopped { code, reason } = &jobs[0].state else {
        panic!("{:?}", jobs[0].state);
    };
    assert_eq!((*code, reason.as_str()), (StopCode::Key2, "VWorld 서버 연결 실패: VWorld 인증키가 거부되었습니다. 배경지도 설정에서 키를 확인한 뒤 다시 내려받으세요."));
    assert!(!b.status().contains("TEST_KEY"));
    assert!(!b.app.left_layers.entries.iter().any(|e| e.layer_key == "ref_river"));
}
