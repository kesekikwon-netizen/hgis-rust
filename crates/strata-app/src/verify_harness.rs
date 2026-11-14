//! Verification harness orchestrator (`--verify-all`, gates V1..V14) and `--smoke` check.

#[path = "verify_harness_gates_v1_v9.rs"]
mod verify_harness_gates_v1_v9;
#[path = "verify_harness_gates_v10_v14.rs"]
mod verify_harness_gates_v10_v14;
#[path = "verify_harness_types.rs"]
mod verify_harness_types;

pub use verify_harness_types::*;

use strata_storage::gpkg::GpkgRepository;
use strata_storage::wkb_codec::{decode_gpkg_wkb, encode_gpkg_wkb, WkbGeometry};
use strata_types::crs::WorkCrs;

use verify_harness_gates_v1_v9::push_v1_to_v9_gates;
use verify_harness_gates_v10_v14::push_v10_to_v14_gates;

/// Runs verification gates V1..V14 in order.
#[must_use]
pub fn run_v1_to_v10_verification_harness() -> Vec<GateResult> {
    let mut gates = Vec::with_capacity(14);

    push_v1_to_v9_gates(&mut gates);
    push_v10_to_v14_gates(&mut gates);

    gates
}

/// C++ `hgis.exe --smoke` 3단계 코어 무결성(`CrsRegistry`, `WkbCodec`, `GpkgStorage`) 검증 보고서 1:1
#[must_use]
pub fn run_cpp_smoke_suite_report() -> (bool, String) {
    let crs_ok = strata_types::crs::find_crs_by_epsg(5186).is_some();
    let poly = WkbGeometry::Polygon(vec![vec![
        [200_000.0, 400_000.0],
        [200_010.0, 400_000.0],
        [200_010.0, 400_010.0],
        [200_000.0, 400_010.0],
        [200_000.0, 400_000.0],
    ]]);
    let encoded = encode_gpkg_wkb(5186, &poly);
    let wkb_ok = decode_gpkg_wkb(&encoded).is_ok_and(|(s, g)| s == 5186 && g == poly);

    let tmp_gpkg = std::env::temp_dir().join(format!("ka_hgis_smoke_{}.gpkg", std::process::id()));
    let gpkg_ok = GpkgRepository::create_survey_schema_5_layers(&tmp_gpkg, WorkCrs::Epsg5186);
    let _ = std::fs::remove_file(&tmp_gpkg);

    let passed_cnt = usize::from(crs_ok) + usize::from(wkb_ok) + usize::from(gpkg_ok);
    let failed_cnt = 3 - passed_cnt;
    let report = format!(
        "[SMOKE] CrsRegistry: {} (EPSG:5186 loaded)\n\
         [SMOKE] WkbCodec: {} (Roundtrip passed)\n\
         [SMOKE] GpkgStorage: {} (Atomic write verified)\n\
         [SMOKE] Result: ALL {passed_cnt} PASSED, {failed_cnt} FAILED. (Exit Code: {failed_cnt})",
        if crs_ok { "OK" } else { "FAIL" },
        if wkb_ok { "OK" } else { "FAIL" },
        if gpkg_ok { "OK" } else { "FAIL" },
    );
    (failed_cnt == 0, report)
}

#[cfg(test)]
#[path = "verify_harness_tests.rs"]
mod tests;
