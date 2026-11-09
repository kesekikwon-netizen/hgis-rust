//! 리본 정합 시험 — 원래 `mod tests_align` 을 그대로 옮겼다.
//! facade 에서 같은 이름으로 등록하므로 시험 경로가 그대로다.

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::gui_ribbon_bar::*;
use crate::ribbon::SubToolsMode;

/// R9 — 서브툴바 다섯 항목과 순서가 `MainWindowAlign.cpp:724,726,730,745,793` 과 같은지.
#[test]
fn test_r9_align_subtoolbar_has_cpp_five_items_in_cpp_order() {
    assert_eq!(
        ALIGN_SUBTOOLBAR_ITEMS,
        ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"]
    );
    assert_eq!(ALIGN_SUBTOOLBAR_ITEMS.len(), 5);
    assert_eq!(ALIGN_SUBTOOLBAR_CAPTION, "  맞추기 › ");
}

/// R9 — `KaAlignMapTool::statusText()` `KaAlignMapTool.cpp:453-462` 네 갈래.
#[test]
fn test_r9_align_status_text_matches_kaalignmaptool_status_text() {
    let mut s = AlignSessionState::default();
    assert_eq!(s.status_text(), "맞출 도면을 고르세요");

    s.target_layer = Some("사진".to_owned());
    assert_eq!(s.status_text(), "왼쪽 도면 모서리 → 오른쪽 지적 모서리");

    s.pending_source = Some([0.0, 0.0]);
    assert_eq!(s.status_text(), "오른쪽 지도에서 같은 모서리를 찍으세요");
}

/// R9 — `GeorefQuality::summary()` `GeorefQuality.cpp:88-89` 두 점·세 점 안내문.
#[test]
fn test_r9_align_pair_summary_matches_georefquality_summary() {
    let mut s = AlignSessionState {
        target_layer: Some("사진".to_owned()),
        ..AlignSessionState::default()
    };
    s.pairs.push(AlignPair {
        src: [0.0, 0.0],
        map: [100.0, 100.0],
    });
    assert_eq!(s.status_text(), "왼쪽 → 오른쪽 · 1점");

    s.pairs.push(AlignPair {
        src: [100.0, 0.0],
        map: [200.0, 100.0],
    });
    assert_eq!(
        s.status_text(),
        "왼쪽 → 오른쪽 · 2점 · 한 점 더 찍으면 기울기도 맞습니다"
    );

    s.pairs.push(AlignPair {
        src: [0.0, 100.0],
        map: [100.0, 200.0],
    });
    assert_eq!(
        s.status_text(),
        "왼쪽 → 오른쪽 · 3점 · 한 점 더 찍으면 어긋남을 잽니다"
    );
}

/// R9 — 「이동」을 점 2개 미만에서 누르면 `KaAlignMapTool.cpp:346-348` 문구로 거절.
#[test]
fn test_r9_align_move_needs_two_pairs_like_kaalignmaptool() {
    let mut app = StrataAppState::new_startup();
    app.align.target_layer = Some("사진".to_owned());
    assert_eq!(
        app.apply_align_move(),
        Err("점을 2개 이상 찍은 뒤 「이동」을 누르세요".to_owned())
    );
    assert!(!app.align.applied);

    app.align.pairs = vec![
        AlignPair {
            src: [0.0, 0.0],
            map: [100.0, 100.0],
        },
        AlignPair {
            src: [100.0, 0.0],
            map: [200.0, 100.0],
        },
    ];
    assert_eq!(app.apply_align_move(), Ok(()));
    assert!(app.align.applied);
    assert!(app.align.transform.is_some());
}

/// R9 — 「점 지우기」는 `KaAlignMapTool.cpp:319-329` 대로 왼쪽 점만 있으면 그것부터 지운다.
#[test]
fn test_r9_align_delete_point_removes_pending_then_last_pair() {
    let mut s = AlignSessionState {
        target_layer: Some("사진".to_owned()),
        ..AlignSessionState::default()
    };
    s.pairs.push(AlignPair {
        src: [0.0, 0.0],
        map: [100.0, 100.0],
    });
    s.pending_source = Some([5.0, 5.0]);
    assert!(s.remove_last_pair());
    assert!(s.pending_source.is_none());
    assert_eq!(s.pairs.len(), 1, "왼쪽 점만 찍힌 상태는 쌍을 지우지 않는다");

    assert!(s.remove_last_pair());
    assert!(s.pairs.is_empty());
    assert!(!s.remove_last_pair(), "지울 것이 없으면 false");
}

/// R9 — 「맞추기 저장」 점 2개 미만 안내는 `MainWindowAlign.cpp:750` 원문.
#[test]
fn test_r9_align_save_refuses_under_two_points_with_cpp_text() {
    let mut app = StrataAppState::new_startup();
    app.align.target_layer = Some("사진".to_owned());
    let (notice, status) = app.save_align_result();
    assert_eq!(
        notice,
        "맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요."
    );
    assert_eq!(status, "점을 2곳 이상 찍은 뒤 저장하세요.");
    assert!(!app.ribbon.sub_toolbar_visible);
}

/// R9 — 「닫기」는 `MainWindowAlign.cpp:705-714` 대로 세션을 끝내고 이동으로 돌아간다.
#[test]
fn test_r9_align_close_ends_session_and_returns_to_pan() {
    let mut app = StrataAppState::new_startup();
    app.start_align_session("사진");
    assert!(app.ribbon.sub_toolbar_visible);
    assert_eq!(app.ribbon.sub_mode, SubToolsMode::Align);
    assert_eq!(app.active_tool, MapToolKind::GeorefAlign);
    assert_eq!(app.status_bar_hint, "왼쪽 도면 모서리 → 오른쪽 지적 모서리");

    app.stop_align_session();
    assert!(!app.ribbon.sub_toolbar_visible);
    assert_eq!(app.ribbon.sub_mode, SubToolsMode::None);
    assert_eq!(app.active_tool, MapToolKind::Pan);
    assert!(app.align.target_layer.is_none());
}

/// R9 — 「정합」은 맞출 도면이 없으면 `MainWindowAlign.cpp:872-874` 원문으로 안내만 한다.
#[test]
fn test_r9_align_button_guides_when_no_alignable_layer() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("정합");
    assert!(!app.ribbon.sub_toolbar_visible);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some(
            "맞추기 — JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요."
        )
    );
}

/// R9 — 맞출 수 있는 도면이 있으면 세션을 열고 상태줄에 statusText() 를 띄운다
/// (`MainWindowAlign.cpp:831-838`). 도메인 조사 레이어는 맞출 수 없다(`GeorefService.cpp:504`).
#[test]
fn test_r9_align_button_opens_session_for_reference_layer_only() {
    let mut app = StrataAppState::new_startup();
    app.left_layers
        .ensure_domain_layer(strata_types::schema::DomainLayerKey::SurveyArea);
    app.handle_ribbon_click("정합");
    assert!(
        !app.ribbon.sub_toolbar_visible,
        "도메인 조사 레이어는 맞출 수 없으므로 세션이 열리면 안 된다"
    );

    app.left_layers.add_cadastral_wms_layer();
    app.right_inspector.selected_layer_key = Some("vworld_cadastral_wms".to_owned());
    app.handle_ribbon_click("정합");
    assert!(
        !app.ribbon.sub_toolbar_visible,
        "지적 그림(vworld_)도 맞출 대상이 아니다"
    );
}
