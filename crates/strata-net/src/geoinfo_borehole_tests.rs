//! `BoreholeRecord::water_level_m` 하위 호환·파싱 시험.
//!
//! `geoinfo_borehole.rs` 본문을 300줄 이하로 유지하기 위해 분리했다.

use strata_types::error::StrataResult;

use crate::geoinfo_borehole::{BoreholeRecord, GeoInfoBoreholeService};

#[test]
fn geoinfo_water_level_parses_number_or_string_and_defaults_to_zero() -> StrataResult<()> {
    // C++ `GeoInfoBoreholeParser.cpp:82-88` — `WATER` 를 숫자/문자열 모두 허용.
    for (sample, expected) in [
        (
            r#"[{"FROM_DEPTH":0.0,"TO_DEPTH":2.0,"LAYER_NAME":"충적층","USCS":"SM","WATER":2.4}]"#,
            2.4,
        ),
        (
            r#"[{"FROM_DEPTH":0.0,"TO_DEPTH":2.0,"LAYER_NAME":"충적층","USCS":"SM","WATER":" 3.1 "}]"#,
            3.1,
        ),
        // 속성 미출현 / 숫자로 파싱 불가 → 0.0 (C++ `hasWaterLevel` 미설정에 해당)
        (
            r#"[{"FROM_DEPTH":0.0,"TO_DEPTH":2.0,"LAYER_NAME":"충적층","USCS":"CL"}]"#,
            0.0,
        ),
        (
            r#"[{"FROM_DEPTH":0.0,"TO_DEPTH":2.0,"LAYER_NAME":"충적층","USCS":"CL","WATER":"N/A"}]"#,
            0.0,
        ),
    ] {
        let rec = GeoInfoBoreholeService::parse_sichudan_json(sample.as_bytes())?;
        assert!(
            (rec.water_level_m - expected).abs() < 1e-9,
            "sample={sample} expected={expected} got={}",
            rec.water_level_m
        );
    }
    Ok(())
}

#[test]
fn borehole_record_deserializes_without_water_level_field_for_backward_compat() {
    // `#[serde(default)]` 덕분에 기존 저장 파일(water_level_m 없음)도 읽혀야 한다.
    let json = r#"{"hole_code":"BH-1","tm_x":200000.0,"tm_y":400000.0,"altitude_m":35.0,
        "fill_depth_m":1.0,"alluvial_from_m":1.0,"alluvial_to_m":5.0,
        "alluvial_thickness_m":4.0,"weathered_depth_m":6.0,
        "dominant_uscs":"SM","paleo_hint":"충적층 두께 4.0m (SM)","layers":[]}"#;
    let rec = serde_json::from_str::<BoreholeRecord>(json);
    assert!(
        rec.is_ok(),
        "legacy record without water_level_m must deserialize"
    );
    if let Ok(rec) = rec {
        assert_eq!(rec.water_level_m, 0.0);
        assert_eq!(rec.hole_code, "BH-1");
    }
}
