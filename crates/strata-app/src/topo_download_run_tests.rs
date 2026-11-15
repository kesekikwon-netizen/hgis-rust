//! What becomes of a pressed order when the survey it was pressed for is closed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_net::ngii_auto::Scope;
use strata_secrets::accounts::Credentials;

use crate::app_state::StrataAppState;
use crate::download_jobs::StopCode;
use crate::topo_download::{topo_request, OrderOf, TopoFacts};

/// An order belongs to the survey it was pressed for: its files go to that survey's folder and its result
/// to that survey's map. When another survey takes its place, a press that still waits for its first frame
/// is forgotten, and nothing of it is left that would put one survey's map on another (found in review,
/// 2026-11-15: the result of a run that ended after the change went into the new survey's folder).
#[test]
fn an_order_is_let_go_when_another_survey_takes_its_place() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let facts = TopoFacts { survey_path: Some(&survey), centre_work: Some([220_000.0, 370_000.0]), work_crs: strata_types::crs::WorkCrs::Epsg5187, online: true };
    let request = topo_request(OrderOf::Dem, facts, || Credentials::new("tester", "pw-for-test")).unwrap();
    assert_eq!(request.scope, Scope { numbers: Vec::new(), cached_numbers: Vec::new(), ..request.scope.clone() });

    let mut app = StrataAppState::new_startup();
    app.dem.pending = Some(Ok(request));
    app.topo.pending = Some(Err((StopCode::Dir2, "조사 폴더에 지형도 폴더를 만들지 못했습니다.".to_owned())));
    assert!(app.topographic_download_busy());

    app.drop_topographic_post();
    assert!(app.dem.pending.is_none() && app.topo.pending.is_none());
    assert!(!app.topographic_download_busy() && app.download_jobs.is_empty());
}
