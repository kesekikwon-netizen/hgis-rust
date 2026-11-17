//! Actual import and receipt writes (`target/cpp-ref/tests/test_heritage_browser.cpp:130-155,294-315`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, Geometry, LayerAccess, LayerOptions, OGRwkbGeometryType};
use gdal::DriverManager;

use super::*;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::heritage_run::lifecycle_tests::ready;
use strata_net::heritage_auto::HeritageEvent;
use strata_refmap::heritage::DATASETS;

fn drain(app: &mut StrataAppState) -> DownloadJobState {
    let limit = Instant::now() + Duration::from_secs(15);
    loop {
        let settled = app
            .heritage
            .download
            .as_ref()
            .is_none_or(|download| download.phase == Phase::Settled || download.complete());
        if settled {
            app.drive_heritage_download(&eframe::egui::Context::default());
            app.download_jobs.tick(1.0);
            return app.download_jobs.jobs()[0].state.clone();
        }
        let reply = app
            .heritage
            .download
            .as_ref()
            .unwrap()
            .worker
            .replies
            .recv_timeout(limit.saturating_duration_since(Instant::now()))
            .expect("real disk worker did not settle");
        // Component-only engine acceptance; actual handle ACK is checked by the native example
        // (target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1217).
        app.take_heritage_import_with(reply, |_, _| true);
    }
}

fn archive(dir: &Path) -> PathBuf {
    archive_named(dir, "fixture")
}

pub(super) fn archive_named(dir: &Path, name: &str) -> PathBuf {
    // One synthetic 10 m square, work CRS EPSG:5187, wholly inside the synthetic survey.
    let shp = dir.join(format!("{name}.shp"));
    let mut file = DriverManager::get_driver_by_name("ESRI Shapefile")
        .unwrap()
        .create_vector_only(&shp)
        .unwrap();
    let crs = SpatialRef::from_epsg(5187).unwrap();
    let layer = file
        .create_layer(LayerOptions {
            name,
            srs: Some(&crs),
            ty: OGRwkbGeometryType::wkbPolygon,
            options: None,
        })
        .unwrap();
    let mut feature = Feature::new(layer.defn()).unwrap();
    feature
        .set_geometry(
            Geometry::from_wkt(
                "POLYGON ((220000 370000,220010 370000,220010 370010,220000 370010,220000 370000))",
            )
            .unwrap(),
        )
        .unwrap();
    feature.create(&layer).unwrap();
    drop(feature);
    drop(file);
    let path = dir.join(format!("{name}.zip"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    for suffix in ["shp", "shx", "dbf", "prj"] {
        zip.start_file(
            format!("{name}.{suffix}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(&std::fs::read(shp.with_extension(suffix)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
    path
}

#[test]
fn six_actual_imports_keep_every_layer_and_one_gauge() {
    let (mut app, dir) = ready();
    let zip = archive(dir.path());
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    for dataset in &DATASETS {
        app.take_heritage_event(HeritageEvent::DatasetReady {
            kind: dataset.name,
            files: vec![zip.clone()],
        });
    }
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(drain(&mut app), DownloadJobState::Done { .. }));
    let rows: Vec<_> = app
        .left_layers
        .entries
        .iter()
        .filter(|row| row.layer_key.starts_with(super::super::KEY_PREFIX))
        .collect();
    assert_eq!(rows.len(), 6);
    for row in rows {
        let spec = app.ref_vectors.spec_of(&row.layer_key).unwrap();
        assert_eq!(spec.work_epsg, 5187);
        assert_eq!(crate::ref_vector::data::load(spec).unwrap().items.len(), 1);
    }
    assert_eq!(app.download_jobs.len(), 1);
    let region =
        strata_refmap::heritage_recent::region_folder(dir.path(), "경상북도", "경주시").unwrap();
    for dataset in &DATASETS {
        let found = strata_refmap::heritage_recent::find(
            &region,
            dataset,
            chrono::Local::now().naive_local(),
        );
        assert_eq!(found.as_slice(), std::slice::from_ref(&zip));
    }
}

#[test]
fn invalid_zip_retries_without_publishing_recent_record() {
    let (mut app, dir) = ready();
    let path = dir.path().join("broken.zip");
    std::fs::write(&path, b"PK\x03\x04truncated").unwrap();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    app.take_heritage_event(HeritageEvent::DatasetReady {
        kind: DATASETS[0].name,
        files: vec![path],
    });
    let reply = app
        .heritage
        .download
        .as_ref()
        .unwrap()
        .worker
        .replies
        .recv_timeout(Duration::from_secs(15))
        .unwrap();
    let mut outcomes = Vec::new();
    app.take_heritage_import_with(reply, |_, outcome| {
        outcomes.push(outcome);
        true
    });
    assert!(
        matches!(&outcomes[..], [strata_net::heritage_auto::DatasetOutcome::RetryableFailure { reason }]
        if reason.contains("broken.zip"))
    );
    app.download_jobs.tick(1.0);
    assert!(matches!(
        app.download_jobs.jobs()[0].state,
        DownloadJobState::Running { .. }
    ));
    assert!(app.heritage.download.as_ref().unwrap().kinds[0] == Kind::Pending);
    let region =
        strata_refmap::heritage_recent::region_folder(dir.path(), "경상북도", "경주시").unwrap();
    assert!(!region.join("recent.json").exists());
}

#[test]
fn pledge_event_writes_exact_terms_to_the_captured_survey() {
    let (mut app, dir) = ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    let terms = "actual \"quoted\" terms\nsecond line\n";
    app.take_heritage_event(HeritageEvent::PledgeAgreed {
        terms: terms.into(),
    });
    for dataset in &DATASETS {
        app.take_heritage_event(HeritageEvent::KindAbsent { kind: dataset.name });
    }
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(drain(&mut app), DownloadJobState::Done { .. }));
    let receipts: Vec<_> = std::fs::read_dir(dir.path().join("주변유적/receipts"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(receipts.len(), 1);
    let text = std::fs::read_to_string(receipts[0].path()).unwrap();
    assert!(text.ends_with(&format!("----\n{terms}\n")), "{text}");
    assert!(!text.contains("fixture-user"));
}

#[test]
fn rejected_agreement_receipt_does_not_advance_to_success() {
    let (mut app, dir) = ready();
    std::fs::write(dir.path().join("주변유적"), b"blocks receipt directory").unwrap();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    app.take_heritage_event(HeritageEvent::PledgeAgreed {
        terms: "actual terms".into(),
    });
    for dataset in &DATASETS {
        app.take_heritage_event(HeritageEvent::KindAbsent { kind: dataset.name });
    }
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(
        drain(&mut app),
        DownloadJobState::Stopped {
            code: StopCode::Dir2,
            ..
        }
    ));
}

#[test]
fn a_received_kind_refreshes_the_reuse_choice_when_setup_reopens() {
    let (mut app, dir) = ready();
    let zip = archive(dir.path());
    assert_eq!(app.heritage_reuse_when("경상북도", "경주시"), None);
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    app.take_heritage_event(HeritageEvent::DatasetReady {
        kind: DATASETS[0].name,
        files: vec![zip],
    });
    for dataset in &DATASETS[1..] {
        app.take_heritage_event(HeritageEvent::KindAbsent { kind: dataset.name });
    }
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(drain(&mut app), DownloadJobState::Done { .. }));
    assert!(
        app.heritage_reuse_when("경상북도", "경주시").is_some(),
        "newly imported data was hidden by the cached setup answer"
    );
}
