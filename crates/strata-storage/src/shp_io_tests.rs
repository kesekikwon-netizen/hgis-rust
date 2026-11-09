use crate::shp_io::{
    calculate_ngii_sheet_number, cp949_byte_len, infer_topographic_source_crs_epsg,
    is_safe_archive_relative_path, resolve_topographic_sheet_25k,
    truncate_dbf_field_name_cp949, validate_topographic_zip_bytes, TopoSheetZone,
    TopographicArchiveError, DBF_MAX_FIELD_NAME_LEN,
};

#[test]
fn test_shapefile_export_cp949_truncates_16byte_korean_field_to_10bytes_on_syllable_boundary() {
    let col = "발굴조사기관명칭";
    assert_eq!(cp949_byte_len(col), 16);
    let truncated = truncate_dbf_field_name_cp949(col, 1);
    assert_eq!(truncated, "발굴조_1");
    assert!(cp949_byte_len(&truncated) <= DBF_MAX_FIELD_NAME_LEN);
    assert_eq!(cp949_byte_len(&truncated), 8);

    // 10바이트 이하 컬럼명은 충돌 없을 때 그대로 유지
    let short_col = "유구번호";
    assert_eq!(cp949_byte_len(short_col), 8);
    assert_eq!(truncate_dbf_field_name_cp949(short_col, 0), "유구번호");

    // TestTopographicSheets::testSheetNumberCalculation:
    // Bulguksa area: 129°19'00"E (129.3166667), 35°47'30"N (35.7916667), scale 5000 -> "35809094"
    let lon = 129.0 + 19.0 / 60.0;
    let lat = 35.0 + 47.0 / 60.0 + 30.0 / 3600.0;
    let sheet_5k = calculate_ngii_sheet_number(lon, lat, 5000);
    assert_eq!(sheet_5k, "35809094");

    // C++ TopographicSheets::resolve 6-digit 1:25,000 sheet resolution
    let resolved = resolve_topographic_sheet_25k("359121").expect("valid 1:25k sheet");
    assert_eq!(resolved.zone, TopoSheetZone::East);
    assert_eq!(resolved.crs_auth_id, "EPSG:5187");

    // TestTopographicSourceCrs::testCrsInference:
    // BBOX X in 295430.0..=297500.0 -> 5187 (East belt)
    assert_eq!(
        infer_topographic_source_crs_epsg(295_430.0, 297_500.0),
        5187
    );
    assert_eq!(
        infer_topographic_source_crs_epsg(198_000.0, 205_000.0),
        5186
    );

    // TestTopographicArchive::testCorruptedZipDetection
    let truncated_zip = b"PK\x03\x04\x14\x00\x00\x00truncated_stream_without_eocd";
    assert_eq!(
        validate_topographic_zip_bytes(truncated_zip),
        Err(TopographicArchiveError::CorruptedZipStream)
    );
    assert!(is_safe_archive_relative_path("35809094/contour_5k.shp"));
    assert!(!is_safe_archive_relative_path("../evil.shp"));
    assert!(!is_safe_archive_relative_path("CON.shp"));
}
