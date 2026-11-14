//! Sheet-check coordinate tests (`target/cpp-ref/src/core/ChecklistEngine.h:8-26`, `target/cpp-ref/src/core/ChecklistEngine.cpp:23,92,150`).

use std::sync::atomic::AtomicBool;

use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

use crate::checklist_engine::{ChecklistEngine, ChecklistSeverity};

#[test]
fn evaluate_sheet_findings_carry_no_coordinate() {
    let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    for has_site_boundary in [true, false] {
        let findings =
            ChecklistEngine::evaluate_sheet(&sheet, SubmitCrs::Epsg5179, has_site_boundary);
        assert_eq!(findings.len(), 27);
        for finding in &findings {
            assert_eq!(
                finding.target_world_xy, None,
                "rule {} carried invented coordinate {:?}",
                finding.rule_id, finding.target_world_xy
            );
        }
    }
}

#[test]
fn evaluate_sheet_with_topology_carries_coordinate_only_when_protruding() {
    let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    let cancel = AtomicBool::new(false);
    let survey_bbox = Some([200_000.0, 350_000.0, 200_100.0, 350_100.0]);
    let inside_features = [[200_010.0, 350_010.0, 200_050.0, 350_050.0]];

    let clean = ChecklistEngine::evaluate_sheet_with_topology(
        &sheet,
        SubmitCrs::Epsg5179,
        survey_bbox,
        &inside_features,
        Some(&cancel),
    )
    .expect("inside topology check");
    assert_eq!(clean.len(), 26);
    for finding in &clean {
        assert_eq!(
            finding.target_world_xy, None,
            "non-protruding rule {} carried coordinate {:?}",
            finding.rule_id, finding.target_world_xy
        );
    }

    let missing_boundary = ChecklistEngine::evaluate_sheet_with_topology(
        &sheet,
        SubmitCrs::Epsg5179,
        None,
        &[],
        Some(&cancel),
    )
    .expect("missing boundary topology check");
    assert_eq!(missing_boundary.len(), 26);
    for finding in &missing_boundary {
        assert_eq!(
            finding.target_world_xy, None,
            "missing-boundary rule {} carried coordinate {:?}",
            finding.rule_id, finding.target_world_xy
        );
    }

    let protruding_features = [[200_050.0, 350_050.0, 200_101.0, 350_080.0]];
    let protruding = ChecklistEngine::evaluate_sheet_with_topology(
        &sheet,
        SubmitCrs::Epsg5179,
        survey_bbox,
        &protruding_features,
        Some(&cancel),
    )
    .expect("protruding topology check");
    let ext = protruding
        .iter()
        .find(|f| f.rule_id == "EXTENT_MATCH")
        .expect("EXTENT_MATCH finding");
    assert!(!ext.passed);
    assert_eq!(ext.severity, ChecklistSeverity::Error);
    assert_eq!(ext.target_world_xy, Some([200_075.5, 350_065.0]));
}
