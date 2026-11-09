//! `gpkg` 프로젝트·일지 시험 — 자동 저장 복구·MRU·예약 이름·일지.
//!
//! `gpkg` 에서 분리했다.

use strata_types::crs::WorkCrs;
use strata_types::error::StrataResult;

use crate::gpkg::*;

#[test]
fn test_crash_autosave_recovery_trigger_and_lock_file_exclusion_and_mru_limit_10(
) -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let proj_path = dir.path().join("site_a.gpkg");
    GpkgRepository::create_new_survey(&proj_path, WorkCrs::Epsg5187)?;
    assert_eq!(
        check_project_open_preflight(&proj_path, false),
        ProjectOpenResult::Success
    );

    // 1. Lock file exclusion (.hgis.lock)
    let lock_path =
        std::path::PathBuf::from(format!("{}{HGIS_LOCK_FILE_EXT}", proj_path.display()));
    std::fs::write(&lock_path, "1234:1700000000:workstation")?;
    assert_eq!(
        check_project_open_preflight(&proj_path, false),
        ProjectOpenResult::LockedByAnotherProcess
    );
    std::fs::remove_file(&lock_path)?;

    // 2. Crash autosave recovery trigger (.hgis.autosave)
    let autosave_path =
        std::path::PathBuf::from(format!("{}{HGIS_AUTOSAVE_FILE_EXT}", proj_path.display()));
    std::fs::write(&autosave_path, b"snapshot")?;
    assert_eq!(
        check_project_open_preflight(&proj_path, false),
        ProjectOpenResult::CrashDetectedNeedsRecovery
    );
    std::fs::remove_file(&autosave_path)?;

    // 3. Recent projects MRU capped at 10
    let mut mru = Vec::new();
    for i in 1..=12 {
        push_recent_project_mru(&mut mru, &format!("/projects/p{i}.gpkg"));
    }
    assert_eq!(mru.len(), HGIS_MAX_RECENT_PROJECTS);
    assert_eq!(mru[0], "/projects/p12.gpkg");
    push_recent_project_mru(&mut mru, "/projects/p5.gpkg");
    assert_eq!(mru.len(), HGIS_MAX_RECENT_PROJECTS);
    assert_eq!(mru[0], "/projects/p5.gpkg");
    Ok(())
}

#[test]
fn test_progress_and_man_days_calculation_and_diary_roundtrip() -> StrataResult<()> {
    // TestKaExcavationProgressTracker::testProgressAndManDaysCalculation
    let history = vec![
        FieldDiaryEntry {
            diary_id: 1,
            work_date: "2026-11-05".to_string(),
            weather: "맑음".to_string(),
            worker_count: 5,
            active_trenches: "T-1".to_string(),
            discovered_features: "1호 수혈".to_string(),
            completed_area_m2: 1500.0,
            notes: String::new(),
        },
        FieldDiaryEntry {
            diary_id: 2,
            work_date: "2026-11-06".to_string(),
            weather: "흐림".to_string(),
            worker_count: 8,
            active_trenches: "T-1, T-2".to_string(),
            discovered_features: "2호 주거지".to_string(),
            completed_area_m2: 3500.0,
            notes: String::new(),
        },
        FieldDiaryEntry {
            diary_id: 3,
            work_date: "2026-11-07".to_string(),
            weather: "맑음".to_string(),
            worker_count: 7,
            active_trenches: "T-3".to_string(),
            discovered_features: "3호 구상유구".to_string(),
            completed_area_m2: 6000.0,
            notes: "안전 점검 완료".to_string(),
        },
    ];
    let metrics = calculate_excavation_progress_metrics(10_000.0, &history);
    assert_eq!(metrics.total_man_days, 20);
    assert!((metrics.progress_percentage - 60.0).abs() < 1e-6);

    // TestKaFieldDiaryManager::testRecordAndQueryRoundtrip
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("diary_test.gpkg");
    let row_id = record_diary_entry_sqlite(&db_path, &history[2])?;
    assert!(row_id >= 1);
    let loaded = query_diary_entries_sqlite(&db_path)?;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].work_date, "2026-11-07");
    assert_eq!(loaded[0].weather, "맑음");
    assert_eq!(loaded[0].worker_count, 7);
    assert_eq!(loaded[0].active_trenches, "T-3");
    assert!((loaded[0].completed_area_m2 - 6000.0).abs() < 1e-6);
    Ok(())
}

#[test]
fn test_windows_reserved_file_names_and_central_meridian_128_detection() {
    // TestSurveyProjectFactory::testWindowsReservedFileNames
    for invalid in [
        "CON",
        "prn",
        "aux",
        "com1",
        "lpt3",
        "경주_유적?조사",
        "",
        "   ",
    ] {
        assert!(
            !is_valid_project_name(invalid),
            "expected {invalid:?} to be rejected"
        );
    }
    assert!(is_valid_project_name("정상_조사_2026"));
    assert_eq!(
        validate_survey_name_ko("CON"),
        Err("Windows가 장치 이름으로 쓰는 예약 이름이라 조사명으로 쓸 수 없습니다.")
    );
    assert_eq!(
        validate_survey_name_ko("경주?유적"),
        Err("조사명에는 \\ / : * ? \" < > | 와 제어 문자를 쓸 수 없습니다.")
    );
    assert_eq!(
        validate_survey_name_ko("..."),
        Err("조사명은 마침표(.)로 끝날 수 없습니다. Windows가 파일 이름 끝의 점을 지웁니다.")
    );
    assert_eq!(validate_survey_name_ko("병산동"), Ok("병산동".to_string()));

    // TestKaLongitudeRuler::testCentralMeridian128Detection
    assert!(is_central_meridian_128(5186));
    assert!(!is_central_meridian_128(5187));
    assert_eq!(recommend_crs_for_longitude(127.38), WorkCrs::Epsg5186);
    assert_eq!(recommend_crs_for_longitude(128.90), WorkCrs::Epsg5187);
    let ticks = calculate_longitude_ticks(126.0, 130.0, 1.0);
    assert_eq!(ticks.len(), 5);
    assert!((ticks[2] - HGIS_CENTRAL_MERIDIAN_LON).abs() < 1e-9);
}
