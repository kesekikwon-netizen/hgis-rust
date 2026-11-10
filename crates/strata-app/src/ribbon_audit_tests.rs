//! 리본 버튼 전수 감사 — 29개 버튼이 실제로 상태를 바꾸는지 확인한다.
//!
//! ## 왜 필요한가
//!
//! 2026-11-10 사용자 지적: "이제 기능을 하나하나 구현하라. 현재 아무것도 안 된다."
//! `gui_ribbon_click_a/b.rs` 에 29개 라벨의 match 팔은 다 있었지만, 그 팔이
//! **실제로 상태를 바꾸는지**는 아무도 확인하지 않았다.
//!
//! 셸에서 `--snap` 으로 누르는 방법은 인코딩(한글이 `??` 로 바뀜) 때문에 불안정해서,
//! 여기서 Rust 로 직접 눌러 본다. 창을 띄우지 않고 확인하므로 빠르고 결정적이다.
//!
//! ## 판정 기준
//!
//! 버튼을 눌렀을 때 **다음 중 하나라도 바뀌면 동작하는 것**으로 본다:
//! - 탭(`active_tab`)
//! - 대화상자(`modals.active`)
//! - 활성 도구(`active_tool`)
//! - 안내 문구(`draw_guide_message`) — "안내만" 하는 버튼도 있다
//! - 열린 스튜디오 탭(`open_studio_tabs`)
//! - 부속 도구 줄(`sub_tools`)
//!
//! 아무 것도 안 바뀌면 **빈 버튼**이다. 이 시험이 그 목록을 보고한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::{MainTab, StrataAppState};

/// 리본 라벨 전체 (C++ `UX_RIBBON_GROUPS` 7그룹).
const ALL_LABELS: [&str; 27] = [
    // 조사
    "새 조사",
    "열기",
    "저장",
    "다른 이름",
    // 기록
    "선택",
    "측거",
    "그리기",
    "시굴격자",
    "버퍼",
    // 자료 받기
    "DEM",
    "지적",
    "수치지형",
    "유산",
    "시추",
    // 배경 지도
    "지형",
    "등고선",
    "토양",
    "고지형",
    "지질",
    "수계",
    "옛 지도",
    // 정합
    "정합",
    // 내보내기
    "도면",
    "인쇄",
    "GeoTIFF",
    "검수·제출",
    // 기타
    "더보기",
];

/// 앱 상태에서 "눈에 보이는 변화"를 한 문자열로 요약한다.
fn state_fingerprint(a: &StrataAppState) -> String {
    let r = &a.ribbon;
    format!(
        "tab={:?}|modal={:?}|tool={:?}|msg={:?}|studio={:?}|\
         menus={}{}{}{}{}{}{}|sub={:?}{:?}{:?}",
        a.active_tab,
        a.modals.active,
        a.active_tool,
        a.home_map.draw_guide_message,
        a.open_studio_tabs,
        r.dem_menu_open as u8,
        r.soil_menu_open as u8,
        r.cadastral_menu_open as u8,
        r.topo_menu_open as u8,
        r.geology_menu_open as u8,
        r.old_map_menu_open as u8,
        r.more_menu_open as u8,
        r.sub_toolbar_visible,
        r.sub_mode,
        r.draw_sub_target
    )
}

/// **핵심 감사**: 모든 리본 버튼을 눌러 보고 아무 것도 안 바뀌는 것을 찾는다.
///
/// `열기`·`저장`·`다른 이름` 은 네이티브 파일 대화상자를 연다. 헤드리스에서는
/// 대화상자가 안 뜨므로 상태가 안 바뀌는 게 정상이다. 대신 환경변수로 경로를
/// 주입해 **실제로 열리고 저장되는지**를 따로 확인한다
/// (`open_reads_gpkg_from_env`, `save_writes_gpkg_from_env`).
#[test]
fn every_ribbon_button_changes_something() {
    // 파일 대화상자를 쓰는 버튼 — 경로 주입으로 따로 검증한다.
    const NEEDS_FILE_DIALOG: [&str; 3] = ["열기", "저장", "다른 이름"];

    let mut dead: Vec<&str> = Vec::new();
    for label in ALL_LABELS {
        if NEEDS_FILE_DIALOG.contains(&label) {
            continue;
        }
        let mut a = StrataAppState::new_startup();
        let before = state_fingerprint(&a);
        a.handle_ribbon_click(label);
        let after = state_fingerprint(&a);
        if before == after {
            dead.push(label);
        }
    }
    assert!(
        dead.is_empty(),
        "아무 것도 하지 않는 리본 버튼 {}개: {:?}",
        dead.len(),
        dead
    );
}

/// `열기` 가 실제로 GPKG 를 읽는다 — 파일 대화상자 대신 환경변수로 경로를 준다.
///
/// **가드로 감싼다.** 환경변수가 새지 않으면 「열기」가 실제 파일 창을 띄워
/// 사용자 화면을 덮는다(2026-11-10 실제 사고).
#[test]
fn open_reads_gpkg_from_env() {
    use crate::test_env_guard::{EnvGuard, TempDirGuard};

    let dir = TempDirGuard::new("ka_open");
    let path = dir.path.join("열기검증.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(
        &path,
        strata_types::crs::WorkCrs::Epsg5187,
    )
    .expect("GPKG 생성");

    let mut a = StrataAppState::new_startup();
    {
        let _env = EnvGuard::set("KA_HGIS_OPEN_PATH", &path);
        a.handle_ribbon_click("열기");
    }

    assert_eq!(
        a.survey_path.as_deref(),
        Some(path.as_path()),
        "「열기」 가 준 경로를 열어야 한다"
    );
}

/// `다른 이름` 이 실제로 GPKG 를 쓴다.
#[test]
fn save_writes_gpkg_from_env() {
    use crate::test_env_guard::{EnvGuard, TempDirGuard};

    let dir = TempDirGuard::new("ka_save");
    let path = dir.path.join("저장검증.gpkg");

    let mut a = StrataAppState::new_startup();
    let ok = {
        let _env = EnvGuard::set("KA_HGIS_SAVE_PATH", &path);
        a.save_survey_project_as()
    };

    assert!(ok, "「다른 이름」 이 저장에 성공해야 한다");
    assert!(path.exists(), "파일이 실제로 생겨야 한다: {}", path.display());
}

/// 각 버튼이 **무엇을** 바꾸는지 표로 남긴다(회귀 감시 + 문서).
///
/// **파일 대화상자를 여는 버튼은 누르지 않는다.** `cargo test` 는 창을 띄우는
/// 프로세스라, 「열기」를 누르면 네이티브 열기 창이 실제로 떠서 작업을 막는다
/// (2026-11-10 실제로 그랬다 — 시험 12분간 화면을 덮었다).
///
/// 이 시험은 실패시키지 않고 출력만 남긴다. 실패 판정은 위 시험이 한다.
#[test]
fn report_what_each_button_does() {
    const NEEDS_FILE_DIALOG: [&str; 3] = ["열기", "저장", "다른 이름"];
    for label in ALL_LABELS {
        if NEEDS_FILE_DIALOG.contains(&label) {
            eprintln!("[RIBBON] {label:10} (파일 대화상자 — 건너뜀)");
            continue;
        }
        let mut a = StrataAppState::new_startup();
        let b_tab = a.active_tab;
        let b_modal = format!("{:?}", a.modals.active);
        let b_tool = format!("{:?}", a.active_tool);
        let b_msg = a.home_map.draw_guide_message.clone();
        let b_studio = a.open_studio_tabs.clone();
        a.handle_ribbon_click(label);

        let mut changes: Vec<String> = Vec::new();
        if a.active_tab != b_tab {
            changes.push(format!("tab→{:?}", a.active_tab));
        }
        if format!("{:?}", a.modals.active) != b_modal {
            changes.push(format!("modal→{:?}", a.modals.active));
        }
        if format!("{:?}", a.active_tool) != b_tool {
            changes.push(format!("tool→{:?}", a.active_tool));
        }
        if a.home_map.draw_guide_message != b_msg {
            let m = a.home_map.draw_guide_message.clone().unwrap_or_default();
            let short: String = m.chars().take(28).collect();
            changes.push(format!("msg「{short}」"));
        }
        if a.open_studio_tabs != b_studio {
            changes.push(format!("studio→{:?}", a.open_studio_tabs));
        }
        if changes.is_empty() {
            eprintln!("[RIBBON] {label:10} !! 아무 것도 안 바뀜");
        } else {
            eprintln!("[RIBBON] {label:10} {}", changes.join(" · "));
        }
    }
}

/// 도구 버튼이 각각 다른 도구를 켠다.
///
/// `시굴격자` 는 조사가 없으면 안내만 하고 도구를 안 켠다(합리적 — 배치할 곳이 없다).
/// 그래서 조사가 있을 때만 도구가 켜지는지 본다.
#[test]
fn tool_buttons_select_distinct_tools() {
    use strata_canvas2d::map_tools::MapToolKind;
    let cases: [(&str, MapToolKind); 3] = [
        ("선택", MapToolKind::Select),
        ("측거", MapToolKind::Measure),
        ("그리기", MapToolKind::DrawPolygon),
    ];
    for (label, want) in cases {
        let mut a = StrataAppState::new_startup();
        a.handle_ribbon_click(label);
        assert_eq!(a.active_tool, want, "「{label}」 은 {want:?} 를 켜야 한다");
        assert_eq!(a.active_tab, MainTab::Map, "「{label}」 은 지도 탭으로 가야 한다");
    }
}

/// `시굴격자` 는 조사가 있으면 시굴 대화상자를 열고, 없으면 안내한다.
#[test]
fn trench_button_needs_a_survey() {
    // 조사 없음 → 안내만.
    let mut a = StrataAppState::new_startup();
    a.handle_ribbon_click("시굴격자");
    assert!(
        a.modals.active.is_none(),
        "조사가 없으면 대화상자를 열지 않는다"
    );
    assert!(
        a.home_map
            .draw_guide_message
            .as_deref()
            .is_some_and(|m| m.contains("새 조사")),
        "대신 무엇을 해야 하는지 안내해야 한다"
    );

    // 조사 있음 → 시굴 대화상자.
    let dir = std::env::temp_dir().join(format!("ka_trench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("임시 폴더");
    let path = dir.join("시굴검증.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(
        &path,
        strata_types::crs::WorkCrs::Epsg5187,
    )
    .expect("GPKG 생성");
    let mut b = StrataAppState::new_startup();
    b.survey_path = Some(path.clone());
    b.handle_ribbon_click("시굴격자");
    assert!(
        matches!(
            b.modals.active,
            Some(crate::dialogs_modal::ActiveModalDialog::TrenchGenerator)
        ),
        "조사가 있으면 시굴 대화상자를 연다"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 대화상자 버튼들이 서로 다른 대화상자를 연다.
///
/// `시추` 는 조사 폴더가 있어야 열린다(C++ `KaBoreholeDialog.cpp:66-70`).
/// 그래서 조사를 준비한 뒤 확인한다.
#[test]
fn dialog_buttons_open_distinct_dialogs() {
    let dir = std::env::temp_dir().join(format!("ka_dlg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("임시 폴더");
    let path = dir.join("대화검증.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(
        &path,
        strata_types::crs::WorkCrs::Epsg5187,
    )
    .expect("GPKG 생성");

    for label in ["지적", "수치지형", "유산", "시추", "인쇄", "검수·제출", "더보기"] {
        let mut a = StrataAppState::new_startup();
        // 시추는 조사 폴더가 있어야 창이 열린다.
        a.survey_path = Some(path.clone());
        a.handle_ribbon_click(label);
        // `modals.active` 를 쓰지 않는 창(시추)도 있으므로 둘 다 본다.
        let opened = a.modals.active.is_some() || a.borehole_dialog.open;
        assert!(opened, "「{label}」 이 어떤 창도 열지 않았다");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// 배경 지도 7버튼이 각각 안내나 상태를 남긴다(조용히 아무 것도 안 하면 안 된다).
#[test]
fn background_map_buttons_are_not_silent() {
    for label in ["지형", "등고선", "토양", "고지형", "지질", "수계", "옛 지도"] {
        let mut a = StrataAppState::new_startup();
        let before = state_fingerprint(&a);
        a.handle_ribbon_click(label);
        assert_ne!(
            state_fingerprint(&a),
            before,
            "「{label}」 이 아무 반응도 없다 — 눌러도 모르는 상태면 안 된다"
        );
    }
}
