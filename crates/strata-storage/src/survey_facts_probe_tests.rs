//! 홈 개수 세기 — 임시 GPKG·임시 ini 만, 드라이브 종류는 가짜.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::survey_facts::lookup;

fn fixed(_: char) -> Option<String> {
    Some("Fixed".to_owned())
}

/// demo 사본: 구역 1, 유구(면+선)를 세어 기록하고, 열린 조사·다른 드라이브·없는 파일은 건드리지 않는다.
#[test]
fn probe_counts_unknown_local_surveys_only() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = dir.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &gpkg).expect("copy");
    let bytes = std::fs::read(&gpkg).expect("read");
    let path = gpkg.to_string_lossy().into_owned();
    let (areas, features) = count_layers(&path).expect("gpkg");
    assert!(areas >= 1 && features >= 2, "{areas} {features}");
    let ini = dir.path().join("ka-hgis.ini");
    assert_eq!(probe_counts_if_unknown(&ini, std::slice::from_ref(&path), std::slice::from_ref(&path), 12, &fixed).unwrap(), 0, "open in the app");
    let usb = |_: char| Some("Removable".to_owned());
    assert_eq!(probe_counts_if_unknown(&ini, std::slice::from_ref(&path), &[], 12, &usb).unwrap(), 0, "not a fixed drive");
    assert_eq!(probe_counts_if_unknown(&ini, &[path.clone(), "C:/없는/조사.gpkg".to_owned()], &[], 12, &fixed).unwrap(), 1);
    let f = lookup(&ini, &path);
    assert_eq!((f.areas, f.features, f.work_crs.as_str()), (areas, features, ""), "the probe never sets a work CRS");
    assert_eq!(probe_counts_if_unknown(&ini, std::slice::from_ref(&path), &[], 12, &fixed).unwrap(), 0, "already known");
    assert_eq!(std::fs::read(&gpkg).expect("read"), bytes, "the survey file is only read");
}

/// GPKG 가 아니면 모름, UNC 는 원격.
#[test]
fn non_gpkg_and_remote() {
    let dir = tempfile::tempdir().expect("temp dir");
    let txt = dir.path().join("x.gpkg");
    std::fs::write(&txt, b"not sqlite").expect("file");
    assert_eq!(count_layers(&txt.to_string_lossy()), None);
    assert_eq!(probe_block_for("//server/share/a.gpkg", &[], &fixed), ProbeBlock::Remote);
}
