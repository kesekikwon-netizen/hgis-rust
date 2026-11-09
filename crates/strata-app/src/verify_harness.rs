//! `verify_harness` 는 검증 게이트의 재수출 창구 겸 오케스트레이터다.
//!
//! 원래 689줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::verify_harness::<이름>` 경로는 그대로 동작한다.
//!
//! **게이트 순서와 개수는 절대 바꾸지 않는다.** `--verify-all` 출력과
//! `gates.len() == 17` 시험이 여기에 걸려 있다. V1~V9, V10~V14, V15, V16~V17 은
//! 각자 파일에 있지만 `run_v1_to_v10_verification_harness` 가 **원래 순서대로** 부른다.

#[path = "verify_harness_borehole_gates.rs"]
mod verify_harness_borehole_gates;
#[path = "verify_harness_gates_v1_v9.rs"]
mod verify_harness_gates_v1_v9;
#[path = "verify_harness_gates_v10_v14.rs"]
mod verify_harness_gates_v10_v14;
#[path = "verify_harness_types.rs"]
mod verify_harness_types;
#[path = "verify_harness_v15_gate.rs"]
mod verify_harness_v15_gate;

pub use verify_harness_borehole_gates::{
    v16_borehole_maps_generation_gate, v17_borehole_maps_are_painted_gate,
};
pub use verify_harness_types::*;
pub use verify_harness_v15_gate::*;

use strata_storage::gpkg::GpkgRepository;
use strata_storage::wkb_codec::{decode_gpkg_wkb, encode_gpkg_wkb, WkbGeometry};
use strata_types::crs::WorkCrs;

use verify_harness_gates_v1_v9::push_v1_to_v9_gates;
use verify_harness_gates_v10_v14::push_v10_to_v14_gates;

/// V1~V17 검증 하네스. 게이트 순서·개수는 C++/기존 구현 그대로다.
#[must_use]
pub fn run_v1_to_v10_verification_harness() -> Vec<GateResult> {
    let mut gates = Vec::with_capacity(17);

    push_v1_to_v9_gates(&mut gates);
    push_v10_to_v14_gates(&mut gates);

    gates.push(v15_borehole_ribbon_and_dialog_gate());
    gates.push(v16_borehole_maps_generation_gate());
    gates.push(v17_borehole_maps_are_painted_gate());

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
