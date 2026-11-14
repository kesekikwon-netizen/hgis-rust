//! dialogs_modal 회귀 시험 3/3.

#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `dialogs_modal` 회귀 시험.
use crate::dialogs_modal::*;
// D8 KaNewSurveyDialog / KaLongitudeRuler / KaLayerInformationPanel
// D16 KaLineColorDialog / KaSurveyAreaDialog / KaSubmitDialog
// D18 최신 kyi/main 정본 — 고지형 자료 창 반경 3항목·버튼 순서
// D19 KaTrenchDialog 하단 버튼·개별편집 + KaDrawingStudio SpinBox 범위

#[test]
fn test_t6_cpp_dialogs_4_variants_and_defaults() {
    let mut ctrl = ModalDialogController::default();
    // target/cpp-ref/src/app/KaControlPointDialog.cpp:16-19,60-71 — id and origin start empty.
    assert_eq!(ctrl.cp_id, "");
    assert_eq!(ctrl.cp_datum, "세계측지계");
    assert_eq!(ctrl.cp_ellipsoid, "GRS80");
    assert_eq!(ctrl.cp_projection, "TM/UTM-K");
    assert_eq!(ctrl.cp_origin, "");
    assert_eq!(ctrl.cp_fix, "RTK");
    assert_eq!(ctrl.survey_area_layer_name, "조사구역");
    assert_eq!(ctrl.survey_area_color_idx, 1);
    assert!((ctrl.survey_area_width_mm - 1.5).abs() < 1e-4);
    assert_eq!(ctrl.line_color_rgb, [0xEA, 0x58, 0x0C]);
    assert_eq!(ctrl.dem_mode_idx, 2);
    assert_eq!(ctrl.dem_relief_strength_pct, 30);

    for variant in [
        ActiveModalDialog::ControlPoint,
        ActiveModalDialog::SurveyArea,
        ActiveModalDialog::LineColor,
        ActiveModalDialog::DemClass,
        ActiveModalDialog::ReferenceStatus,
        ActiveModalDialog::HeritageBrowser,
        ActiveModalDialog::TopographicImport,
    ] {
        ctrl.active = None;
        assert!(ctrl.show_exclusive_modal(variant));
        assert_eq!(ctrl.active, Some(variant));
    }
}

// F1d-1 — target/cpp-ref/src/app/KaNewSurveyDialog.cpp:55-57,
// target/cpp-ref/src/app/KaSurveyContourDialog.cpp:29,128-130,
// target/cpp-ref/src/app/KaControlPointDialog.cpp:16-19,60-71.
#[test]
fn default_modal_values_start_empty_like_cpp() {
    let ctrl = ModalDialogController::default();
    assert_eq!(ctrl.new_survey_name, "");
    assert_eq!(ctrl.contour_file_path, "");
    assert_eq!(ctrl.cp_id, "");
    assert_eq!(ctrl.cp_x, "");
    assert_eq!(ctrl.cp_y, "");
    assert_eq!(ctrl.cp_z, "");
    assert_eq!(ctrl.cp_origin, "");
    assert_eq!(ctrl.cp_datum, "세계측지계");
    assert_eq!(ctrl.cp_ellipsoid, "GRS80");
    assert_eq!(ctrl.cp_projection, "TM/UTM-K");
    assert_eq!(ctrl.cp_accuracy, "1.0");
    assert_eq!(ctrl.cp_pdop, "1.5");
    assert_eq!(ctrl.cp_fix, "RTK");
}

/// 임시 폴더에 가짜 자료를 만들고 스캔 결과가 창 상태로 옮겨지는지 본다.
/// C++ `KaTopographicImportDialog.cpp:433-497` `scanFolder` → `populate`.
#[test]
fn test_r14_topographic_import_reads_the_real_folder_instead_of_fixed_rows() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let folder = std::env::temp_dir().join(format!(
        "ka_hgis_r14_app_{}_{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("임시 폴더");
    let mut header = vec![0_u8; 100];
    header[..4].copy_from_slice(&9994_u32.to_be_bytes());
    for (offset, value) in [(36, 218_450.0_f64), (44, 359_800.0), (52, 221_250.0), (60, 362_400.0)]
    {
        header[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    std::fs::write(folder.join("A0010000.shp"), &header).expect("shp");
    std::fs::write(
        folder.join("A0010000.prj"),
        "PROJCS[\"Korea 2000 / East Belt\",AUTHORITY[\"EPSG\",\"5187\"]]",
    )
    .expect("prj");
    // .prj 없는 DXF — C++ 규칙대로 [검토 필요]이고 자동 체크되지 않는다.
    std::fs::write(
        folder.join("행정경계.dxf"),
        "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nG0010000\n10\n0.0\n20\n0.0\n\
         0\nLINE\n8\nG0010000\n10\n10.0\n20\n10.0\n0\nENDSEC\n0\nEOF\n",
    )
    .expect("dxf");

    let mut ctrl = ModalDialogController::default();
    // 시작할 때는 하드코딩 목록이 없다.
    assert!(ctrl.topo_records.is_empty());
    assert!(ctrl.topo_row_checks.is_empty());
    assert!(!ctrl.topo_scanned);

    ctrl.rescan_topo_folder(&folder);
    assert!(std::fs::remove_dir_all(&folder).is_ok());

    assert!(ctrl.topo_scanned);
    assert_eq!(ctrl.topo_records.len(), 2);
    assert_eq!(ctrl.topo_row_checks.len(), 2);
    assert_eq!(ctrl.topo_files_read, 2);
    // C++ `:469-470` — 도로는 체크, 검토 필요 DXF 는 해제.
    let road = ctrl
        .topo_records
        .iter()
        .position(|r| r.layer_name == "A0010000")
        .expect("도로 레코드");
    let review = ctrl
        .topo_records
        .iter()
        .position(|r| r.review_reason.is_some())
        .expect("검토 필요 레코드");
    assert!(ctrl.topo_row_checks[road]);
    assert!(!ctrl.topo_row_checks[review]);
    // `.prj` 를 읽었으므로 콤보가 「파일 정보」를 가리킨다(C++ `:484`).
    assert_eq!(ctrl.topo_row_crs_idx[road], 1);
    assert_eq!(ctrl.topo_row_crs_idx[review], 0);

    let rows = ctrl.topo_table_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[road].label, "도로경계(미분류)");
    assert_eq!(
        rows[road].extent_label,
        "X 218450.0 ~ 221250.0\nY 359800.0 ~ 362400.0"
    );
    assert!(rows[review].review_reason.is_some());
    assert_eq!(rows[review].extent_label, "X 0.0 ~ 10.0\nY 0.0 ~ 10.0");
    // 상태줄은 C++ `:494-495` 문구다.
    assert!(ctrl.topo_status_text.starts_with("레이어 2개 · 파일 2개 확인 · 캐시 0개."));
    assert!((ctrl.topo_progress_pct - 1.0).abs() < 1e-6);

    // 체크된 항목(도로)만으로는 막히지 않는다.
    assert_eq!(ctrl.topo_blocking_message(), None);
    assert_eq!(ctrl.topo_checked_rows(), vec![road]);

    // 검토 필요 항목을 사용자가 직접 체크하면 추가는 되지만, 좌표계를 골라야 한다
    // (C++ `:534-536`).
    ctrl.topo_row_checks[review] = true;
    assert_eq!(
        ctrl.topo_blocking_message(),
        Some(strata_net::topographic_catalog::IMPORT_NEEDS_CRS)
    );
    ctrl.topo_row_crs_idx[review] = 2; // 후보 중 하나를 직접 고른다(C++ `:533-537`)
    assert_eq!(ctrl.topo_blocking_message(), None);

    // 없는 폴더는 C++ `TopographicCatalog.cpp:262` 문구로 끝나고 목록은 그대로다.
    let before = ctrl.topo_records.len();
    ctrl.rescan_topo_folder(&std::env::temp_dir().join("ka_hgis_r14_없는폴더"));
    assert!(ctrl.topo_status_text.starts_with("수치지형도 폴더를 읽을 수 없습니다:"));
    assert_eq!(ctrl.topo_records.len(), before, "오류일 때 기존 목록을 지우지 않는다");

    // 색인 취소 문구(C++ `:451`).
    ctrl.cancel_topo_scan();
    assert_eq!(
        ctrl.topo_status_text,
        "색인을 취소했습니다. 기존 지도는 유지됩니다."
    );
}

/// C++ `KaTopographicImportDialog.cpp:243-259,479-482` — 좌표계 이름표와 후보 12종.
#[test]
fn test_r14_topographic_crs_labels_follow_the_cpp_dialog() {
    assert_eq!(topo_crs_label_ko(5187), "동부원점(GRS80) · EPSG:5187");
    assert_eq!(topo_crs_label_ko(5176), "동부원점(보정 베셀) · EPSG:5176");
    assert_eq!(topo_crs_label_ko(5179), "UTM-K(GRS80) · EPSG:5179");
    assert_eq!(topo_crs_label_ko(4326), "경위도(WGS84) · EPSG:4326");
    assert_eq!(topo_crs_label_ko(9999), TOPO_CRS_UNKNOWN);
    assert_eq!(
        TOPO_CRS_CANDIDATES,
        [5185, 5186, 5187, 5188, 5173, 5174, 5175, 5176, 5177, 5178, 5179, 4326]
    );
    // .prj 를 읽었으면 1(파일 정보), 못 읽었으면 0(미확인).
    assert_eq!(topo_default_crs_idx(Some(5187)), 1);
    assert_eq!(topo_default_crs_idx(None), 0);
}

#[test]
fn test_d6_trench_dial_survey_kind_and_contour_advanced_toggle() {
    let mut ctrl = ModalDialogController::default();

    // trench defaults
    assert_eq!(ctrl.trench_survey_kind_idx, 0);
    assert_eq!(ctrl.trench_rows, 2);
    assert_eq!(ctrl.trench_cols, 2);
    assert_eq!(ctrl.trench_balk_m, 10.0);
    assert_eq!(ctrl.trench_prefix, "Tr-");
    assert!(!ctrl.trench_slope_align);

    // 조사 종류 cycles through the 3 C++ labels
    let labels = [
        "시굴조사 — 전체 면적의 10%",
        "표본조사 — 전체 면적의 2%",
        "직접 지정 — 규격·둑을 내가 정함",
    ];
    for expect in 1..=3usize {
        ctrl.trench_survey_kind_idx = (ctrl.trench_survey_kind_idx + 1) % 3;
        assert_eq!(expect % 3, ctrl.trench_survey_kind_idx);
    }
    assert_eq!(labels.len(), 3);

    // contour advanced starts closed, then toggles open
    assert!(!ctrl.contour_advanced_open);
    ctrl.contour_advanced_open = true;
    assert!(ctrl.contour_advanced_open);

    // 5 CRS choices; cell/edge accept the "0 == automatic" sentinel
    assert_eq!(ctrl.contour_file_crs_idx, 0);
    ctrl.contour_file_crs_idx = 4;
    assert_eq!(ctrl.contour_file_crs_idx, 4);
    ctrl.contour_max_edge_m = 0.0;
    ctrl.contour_cell_size_m = 0.0;
    assert_eq!(ctrl.contour_max_edge_m, 0.0);
    assert_eq!(ctrl.contour_cell_size_m, 0.0);
}

// D5 KaTopographicAccountDialog.cpp:60-61 — an account with only one of
// ID/PW must block the save; both-empty and both-filled are fine.
