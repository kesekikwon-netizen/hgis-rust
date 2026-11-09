//! `cad_csv_io` 시험 — DXF·CSV·GGA·인코딩.
//!
//! `cad_csv_io` 에서 분리했다.

use crate::cad_csv_io::*;

#[test]
fn test_dxf_block_insert_parsing_extracts_5_point_benchmark_blocks() {
    let mut dxf_sample = String::new();
    for idx in 0..5 {
        dxf_sample.push_str(&format!(
            "0\nINSERT\n2\nPOINT_BENCHMARK\n10\n{:.1}\n20\n{:.1}\n50\n{:.1}\n",
            200_000.0 + f64::from(idx) * 10.0,
            350_000.0 + f64::from(idx) * 20.0,
            f64::from(idx) * 15.0
        ));
    }
    let parsed = parse_dxf_block_inserts(dxf_sample.as_bytes(), "POINT_BENCHMARK");
    assert_eq!(parsed.len(), 5);
    assert_eq!(parsed[0].block_name, "POINT_BENCHMARK");
    assert!((parsed[4].x - 200_040.0).abs() < 1e-6);
    assert!((parsed[4].rotation_deg - 60.0).abs() < 1e-6);
}

#[test]
fn test_csv_benchmark_importer_bom_and_skip_malformed_row_extracts_2_valid_points_and_1_warning(
) {
    let mut csv_bytes = UTF8_BOM_BYTES.to_vec();
    csv_bytes.extend_from_slice(
        "ID,X,Y,Name\nCP01,200123.45,543210.12,기준점A\nCP02,BAD_NUMBER,543215.00,불량행\nCP03,200150.00,543220.00,기준점B\n".as_bytes(),
    );
    let result = import_control_points_csv_checked(&csv_bytes);
    assert_eq!(result.points.len(), 2);
    assert_eq!(result.skipped_rows, 1);
    assert_eq!(result.warnings.len(), 1);
    assert!(result.warnings[0].contains("[Line 3]"));
    assert_eq!(result.points[0].point_id, "CP01");
    assert!((result.points[0].x - 200_123.45).abs() < 1e-6);
    assert_eq!(result.points[1].point_id, "CP03");

    // TestKaDxfSpotImporter::testTextElevationFallback:
    // POINT (100.0, 200.0, 0.0) + TEXT at 0.5m distance with "78.45" -> elevated to 78.45m
    let dxf_2d_with_text =
        "0\nPOINT\n10\n100.0\n20\n200.0\n30\n0.0\n0\nTEXT\n10\n100.3\n20\n200.4\n1\n78.45\n";
    let spots = extract_dxf_spot_elevations_with_text_fallback(dxf_2d_with_text.as_bytes());
    assert_eq!(spots.len(), 1);
    assert!((spots[0][0] - 100.0).abs() < 1e-6);
    assert!((spots[0][1] - 200.0).abs() < 1e-6);
    assert!((spots[0][2] - 78.45).abs() < 1e-6);
}

#[test]
fn test_gga_checksum_and_rtk_coord_conversion_and_exif_dms_and_view_cone_apex_and_radius() {
    let raw_body = "GNGGA,082530.00,3549.9542,N,12913.1432,E,4,14,0.8,45.2,M,18.5,M,,";
    let cs = raw_body.bytes().fold(0_u8, |acc, b| acc ^ b);
    let sentence = format!("${raw_body}*{cs:02X}");
    let rec = parse_gga_sentence(&sentence).expect("valid GNGGA sentence");
    assert!(rec.is_valid);
    assert_eq!(rec.quality, GnssQuality::RtkFixed);
    assert_eq!(rec.satellites_used, 14);
    assert!((rec.hdop - 0.8).abs() < 1e-9);
    assert!((rec.latitude - (35.0 + 49.9542 / 60.0)).abs() < 1e-7);
    assert!((rec.longitude - (129.0 + 13.1432 / 60.0)).abs() < 1e-7);
    assert!(filter_rtk_quality(&rec, HGIS_RTK_MAX_ALLOWED_HDOP));

    let lat = exif_rational_dms_to_decimal((35, 1), (50, 1), (4500, 100), 'N')
        .expect("valid EXIF DMS");
    assert!((lat - 35.845_833_333).abs() < 1e-6);

    let cone = build_photo_view_cone_ring([100.0, 100.0], 90.0, 60.0, 15.0, 4);
    assert_eq!(cone.first().copied(), Some([100.0, 100.0]));
    assert_eq!(cone.last().copied(), Some([100.0, 100.0]));
    // Midpoint of 4 steps is index 3 (at 90 deg due east -> (115.0, 100.0))
    assert!((cone[3][0] - 115.0).abs() < 1e-6);
    assert!((cone[3][1] - 100.0).abs() < 1e-6);
}

#[test]
fn test_cp949_detection_and_lossless_conversion_and_korean_alias_resolution() {
    // TestKaCharsetAutoDetector::testCp949DetectionAndLosslessConversion
    let cp949_bytes: &[u8] = b"\xB1\xE8\xC1\xDF\xBC\xAE\x82\xA1";
    let detected = detect_korean_encoding(cp949_bytes);
    assert_eq!(detected, DetectedEncoding::Cp949);
    let utf8_text = convert_to_utf8_lossless(cp949_bytes, detected);
    assert_eq!(utf8_text, "김중석똠");

    // EUC-KR standard range only
    let euckr_only: &[u8] = b"\xB1\xE8\xC1\xDF\xBC\xAE";
    assert_eq!(detect_korean_encoding(euckr_only), DetectedEncoding::EucKr);

    // TestKaFieldMappingTemplate::testKoreanAliasResolution
    let fields = ["유구명", "유구종류", "시대", "해발고도", "비고"];
    let mapped = resolve_field_schema_mapping(&fields);
    assert!(mapped.is_all_required_mapped);
    assert_eq!(
        mapped
            .source_to_target_map
            .get("유구명")
            .map(String::as_str),
        Some("FEAT_NO")
    );
    assert_eq!(
        mapped
            .source_to_target_map
            .get("유구종류")
            .map(String::as_str),
        Some("FEAT_TYPE")
    );
    assert_eq!(
        mapped.source_to_target_map.get("시대").map(String::as_str),
        Some("PERIOD")
    );
    assert_eq!(
        mapped
            .source_to_target_map
            .get("해발고도")
            .map(String::as_str),
        Some("ELEVATION")
    );
    assert_eq!(mapped.unmapped_source_fields, vec!["비고".to_string()]);

    // TestCadLayerClassifier::testLayerRoleClassification
    assert_eq!(
        classify_cad_layer_name("TR-01_BOUNDARY"),
        CadLayerRole::TrenchBoundary
    );
    assert_eq!(
        classify_cad_layer_name("유구_주거지_선"),
        CadLayerRole::FeatureBoundary
    );
    assert_eq!(
        classify_cad_layer_name("CONT_1M"),
        CadLayerRole::ContourLine
    );
    assert_eq!(
        classify_cad_layer_name("TBM_POINT"),
        CadLayerRole::SurveyPoint
    );
    assert_eq!(
        classify_cad_layer_name("SECTION_A-B"),
        CadLayerRole::SectionProfile
    );

    // TestKaCadImport::testLocalOriginWarningDetection: (0,0) ~ (500,300) -> requires_crs_shift_warning = true
    assert!(detect_cad_local_origin_or_mm_warning([
        0.0, 0.0, 500.0, 300.0
    ]));
    assert!(!detect_cad_local_origin_or_mm_warning([
        200_000.0, 500_000.0, 200_500.0, 500_300.0
    ]));

    // TestKaControlCsvBox::testCoordinateAxisSwapping: (542100.25, 201300.50) -> swapped to (201300.50, 542100.25)
    let (x_norm, y_norm, swapped) =
        normalize_csv_control_point_xy_order(542_100.25, 201_300.50);
    assert!(swapped);
    assert!((x_norm - 201_300.50).abs() < 1e-6);
    assert!((y_norm - 542_100.25).abs() < 1e-6);

    // TestKoreaRegionCatalog::testGyeongjuCodeResolution & LocationSearch PNU validation
    let dist = resolve_korea_sigungu_code("47110").expect("Gyeongju code 47110");
    assert_eq!(dist.sido_name, "경상북도");
    assert_eq!(dist.sigungu_name, "경주시");
    assert!(dist.bbox[2] > dist.bbox[0] && dist.bbox[3] > dist.bbox[1]);
    assert!(validate_vworld_parcel_pnu_19(
        "4713010300100760000",
        false,
        76,
        0
    ));

    // TestSurveyPointSchema::testSchemaInferenceEastNorth
    let header = ["No", "Val1", "Val2", "EL"];
    let row1 = ["1", "195420.5", "542100.8", "45.2"];
    assert_eq!(
        infer_survey_point_schema(&header, &[&row1]),
        SurveyPointColumnOrder::NameXYZDesc
    );

    // TestSurveyNameSanitizer::testInvalidCharactersSubstitution
    assert_eq!(
        sanitize_project_name("  경주:황남동*발굴?조사/2026.  "),
        "경주_황남동_발굴_조사_2026"
    );
}
