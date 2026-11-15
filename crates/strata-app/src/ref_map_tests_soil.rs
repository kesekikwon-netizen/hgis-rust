//! The 토양 button from the press to the legend row, on answers made here (no request leaves the test):
//! `target/cpp-ref/src/app/MainWindowOverlay.cpp:438-490` (`downloadSoilTerrain`), `:604-680`, and the look
//! of the layer in `target/cpp-ref/src/core/SoilMapService.cpp:68-78`, `:200-236`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::Ordering;

use serde_json::json;
use strata_engine::Transformer;
use strata_refmap::transfer::{Request, TransferFailure};
use strata_types::schema::LegendGroup;

use super::tests::{bench, CENTRE};
use crate::download_jobs::DownloadJobState;
use crate::ref_vector::data::{LabelLook, Look, PolygonFill};

/// The service: every table counts one unit, and holds one of 300 m beside the survey (EPSG:5186).
fn naas(request: &Request) -> Result<Vec<u8>, TransferFailure> {
    assert!(request.url.starts_with("https://gis.naas.go.kr/geoserver/soilmap/ows?") && request.user_agent == "ka-hgis/0.3", "{}", request.url);
    if request.url.contains("&resultType=hits&") {
        return Ok(br#"<FeatureCollection numberMatched="1"/>"#.to_vec());
    }
    let table: f64 = request.url.split('&').find_map(|p| p.strip_prefix("typeNames=soilmap:SOIL_")).and_then(|n| n.parse().ok()).unwrap();
    let at = Transformer::new(5187, 5186).unwrap().transform(CENTRE).unwrap();
    let (x, y) = (at[0] + table * 400.0, at[1]);
    let ring = vec![[x, y], [x + 300.0, y], [x + 300.0, y + 300.0], [x, y + 300.0], [x, y]];
    let unit = json!({"type": "Feature", "properties": {"soil_type_geo": "04"}, "geometry": {"type": "Polygon", "coordinates": [ring]}});
    Ok(json!({"type": "FeatureCollection", "crs": {"type": "name", "properties": {"name": "EPSG:5186"}}, "features": [unit]}).to_string().into_bytes())
}

/// A press of 토양 fetches the three tables for the 10 km around the survey area, stores the map in the
/// survey folder and puts 「토양도(흙토람)」 into the legend, in the official colours with the C++ labels. A
/// second press hides the row, a third shows it; nothing is fetched again.
#[test]
fn the_soil_button_fetches_the_map_and_then_hides_and_shows_it() {
    let mut b = bench(naas, || true);
    b.app.handle_ribbon_click("토양");
    assert_eq!(b.status(), "토양도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.");
    b.frame(1.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 6, "a count and the features of each of the three tables");

    let row = b.app.left_layers.entries.iter().find(|e| e.layer_key == "ref_soil").expect("row");
    assert_eq!((row.label_ko.as_str(), row.group), ("토양도(흙토람)", LegendGroup::ReferenceMap));
    assert!(b.app.home_map.real_survey.is_layer_visible("토양도(흙토람)"));
    let jobs = b.app.download_jobs.jobs();
    assert_eq!((jobs.len(), jobs[0].title.as_str()), (1, "토양도"));
    assert_eq!(jobs[0].state, DownloadJobState::Done { result: "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned() });
    assert_eq!(b.status(), "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.");

    let stored: Vec<_> = std::fs::read_dir(b.dir.path()).unwrap().flatten().map(|e| e.path().join("토양도_흙토람.gpkg")).filter(|f| f.is_file()).collect();
    assert_eq!(stored.len(), 1);
    assert!(stored[0].parent().unwrap().join("ka-hgis-scope.json").is_file());
    let spec = b.app.ref_vectors.spec_of("ref_soil").expect("layer");
    assert_eq!((spec.file.as_path(), spec.table.as_str(), spec.work_epsg), (stored[0].as_path(), "soil_map", 5187));
    // Fill at alpha 175 in the colour of the terrain class, outline 40/44/52 at alpha 180, 0.28 mm
    // (`terrainFillSymbol`, SoilMapService.cpp:68-78); a unit without a colour is the grey 미분류.
    assert_eq!(spec.look, Look::Polygons { stroke: [40, 44, 52, 180], width_mm: 0.28, fill: PolygonFill::Column { name: "ka_color", alpha: 175, missing: [200, 200, 200] } });
    // Malgun Gothic 8 pt bold in 28/32/38 with a white edge of 0.7 at alpha 235, only where it fits into its
    // unit and only at 1:40 000 or closer (`applyTerrainLabels`, :200-236).
    assert_eq!(spec.label, Some(LabelLook { column: "ka_label", size_pt: 8.0, ink: [28, 32, 38], halo_mm: 0.7, halo: [255, 255, 255, 235], bold: true, closest_scale: Some(40_000.0), inside_only: true }));

    b.app.handle_ribbon_click("토양");
    assert!(!b.app.home_map.real_survey.is_layer_visible("토양도(흙토람)"));
    assert_eq!(b.status(), "토양도(흙토람)를 껐습니다.");
    b.app.handle_ribbon_click("토양");
    assert!(b.app.home_map.real_survey.is_layer_visible("토양도(흙토람)"));
    assert_eq!(b.status(), "토양도(흙토람)를 다시 켰습니다.");
    b.frame(2.0);
    assert_eq!((b.requests.load(Ordering::SeqCst), b.app.download_jobs.len()), (6, 1), "nothing is fetched again");
}

fn no_answer(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    panic!("no request may be sent here")
}

/// Without a survey area, or with the network off, the press says why and adds no row.
#[test]
fn the_soil_button_says_why_it_cannot_start() {
    let mut b = bench(no_answer, || true);
    b.app.survey_tables.clear();
    b.app.handle_ribbon_click("토양");
    assert_eq!(b.status(), "조사구역을 먼저 그려 주세요. 조사구역에서 반경 10km 안의 토양도를 받습니다.");

    let mut b = bench(no_answer, || false);
    b.app.handle_ribbon_click("토양");
    assert_eq!(b.status(), "인터넷 접속이 꺼져 있어 토양도 서버에 연결할 수 없습니다.");
    b.frame(1.0);
    assert!(!b.app.left_layers.entries.iter().any(|e| e.layer_key == "ref_soil"));
}
