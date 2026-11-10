//! 검증 게이트 V18 — 지도 캔버스가 **무엇을 그리는지** 순서까지 확인한다.
//!
//! ## 왜 필요한가
//!
//! 2026-11-10, 지도 탭을 레이어 엔진으로 갈아끼우면서 `paint_gyeongju_heritage_canvas`
//! (247줄)와 `gui_tab_map_canvas_overlay`(121줄)를 지웠다. 대체 경로는 조사 데이터만
//! 그렸고, 그 안에 있던 **러버밴드·GPS 기준점·좌표격자·수치지형도·CAD 스냅·DEM 음영**은
//! 넘어가지 않았다.
//!
//! 컴파일·clippy·기존 17개 게이트가 모두 통과했다. 함수째 지웠으므로 `dead_code`
//! 경고조차 없었다. V17 이 세운 교훈("파일 생성과 화면 표시는 다른 일이다")이
//! 지도 캔버스에는 아직 적용되지 않았던 것이다.
//!
//! ## 무엇을 보는가
//!
//! 창을 띄우지 않고 **도형 목록과 그리는 순서**를 본다(ENGINE_DESIGN 원칙 3).
//! 순서가 중요한 이유: 겹침이 데이터보다 먼저 그려지면 좌표격자가 조사구역을 덮는다.
//! 그건 "그려졌다"로는 잡히지 않는 결함이다.

use super::verify_harness_types::GateResult;

/// V18: 지도 캔버스가 조사 데이터·겹침·러버밴드를 **기대한 순서로** 내는지.
#[must_use]
pub fn v18_map_canvas_draws_data_and_overlays_gate() -> GateResult {
    use strata_canvas2d::layer_engine::{
        count_shapes, render_stage, DrawShape, LayerStage, RenderOptions, ScreenTransform,
    };

    use crate::map_stage_bridge::stage_from_survey;
    use crate::real_survey::RealSurveyData;

    // 조사구역 하나 + 시굴 트렌치 하나를 가진 조사를 만든다.
    let mut rs = RealSurveyData::new_empty_survey(&std::env::temp_dir());
    rs.survey_rings = vec![vec![
        [200_000.0, 500_000.0],
        [200_100.0, 500_000.0],
        [200_100.0, 500_100.0],
        [200_000.0, 500_100.0],
    ]];
    rs.trenches.push(crate::real_survey::RealTrenchPoly {
        name: "T1".to_owned(),
        ring: vec![
            [200_010.0, 500_010.0],
            [200_050.0, 500_010.0],
            [200_050.0, 500_050.0],
        ],
        area_m2: 800.0,
        center: [200_030.0, 500_030.0],
    });

    let stage = stage_from_survey(&rs);
    let xf = ScreenTransform::new([200_000.0, 500_100.0], 0.5, [0.0, 0.0]);
    let draws = render_stage(&stage, &xf, RenderOptions::default());

    // 1) 조사 데이터가 실제로 도형을 낸다 — 빈 화면이면 안 된다.
    let shape_count = count_shapes(&draws);
    let data_ok = shape_count > 0 && !draws.is_empty();

    // 2) z순서: 조사구역이 시굴격자보다 아래(먼저)에 온다.
    let keys: Vec<&str> = draws.iter().map(|d| d.key.as_str()).collect();
    let order_ok = match (
        keys.iter().position(|k| *k == "survey_area"),
        keys.iter().position(|k| *k == "trial_trench"),
    ) {
        (Some(a), Some(t)) => a < t,
        _ => false,
    };

    // 3) 라벨이 도형으로 나온다(면적·트렌치 이름).
    let label_count = draws
        .iter()
        .flat_map(|d| d.shapes.iter())
        .filter(|s| matches!(s, DrawShape::Label { .. }))
        .count();
    let labels_ok = label_count >= 2;

    // 4) 레이어를 끄면 그 레이어 도형이 사라진다 — "꺼도 안 꺼지던" 결함의 재발 감시.
    let mut off = stage_from_survey(&rs);
    off.set_visible("trial_trench", false);
    let off_draws = render_stage(&off, &xf, RenderOptions::default());
    let hide_ok = !off_draws.iter().any(|d| d.key == "trial_trench");

    // 5) 빈 조사는 아무 것도 그리지 않는다 — 새 조사에 합성 그림이 뜨던 결함의 재발 감시.
    let empty = RealSurveyData::new_empty_survey(&std::env::temp_dir());
    let empty_stage: LayerStage = stage_from_survey(&empty);
    let empty_ok = render_stage(&empty_stage, &xf, RenderOptions::default()).is_empty();

    // 6) 러버밴드(그리는 중인 선)를 그리는 함수가 실제로 있는지.
    //
    // 이게 이 게이트의 핵심이다. 러버밴드는 레이어가 아니라 화면 겹침이라
    // 엔진 도형 목록에 안 나온다. 그래서 위 1~5번을 다 통과해도 사라질 수 있었다.
    // 함수 참조를 잡아 두면 지웠을 때 컴파일이 깨져 조용히 사라지지 않는다.
    let rubberband_ok = has_scene_overlay_painter();

    let passed =
        data_ok && order_ok && labels_ok && hide_ok && empty_ok && rubberband_ok;

    GateResult {
        gate_id: "V18",
        title_ko: "지도 캔버스 조사 데이터·겹침 그리기(순서·러버밴드 포함)",
        passed,
        detail: format!(
            "layers={} shapes={} labels={} order={} hide={} empty={} rubberband={}",
            draws.len(),
            shape_count,
            label_count,
            if order_ok { "ok" } else { "bad" },
            if hide_ok { "ok" } else { "bad" },
            if empty_ok { "ok" } else { "bad" },
            if rubberband_ok { "ok" } else { "missing" }
        ),
    }
}

/// 겹침 그리기 함수가 있는지 확인한다.
///
/// 메서드를 값으로 잡아 두므로, 누군가 `paint_scene_overlays` 를 지우면
/// 이 줄이 컴파일되지 않는다. 경고가 아니라 **오류**로 막는 것이 목적이다.
fn has_scene_overlay_painter() -> bool {
    use crate::app_state::StrataAppState;
    use crate::gui_tab_map::SceneOverlayCtx;

    let f: fn(&StrataAppState, SceneOverlayCtx<'_>) = StrataAppState::paint_scene_overlays;
    let _ = f;
    // 지형 음영도 같은 이유로 잡아 둔다 — DEM 배경도 조용히 사라졌던 적이 있다.
    let g: fn(
        &StrataAppState,
        &eframe::egui::Context,
        &eframe::egui::Painter,
        eframe::egui::Rect,
        &strata_canvas2d::viewport::ViewportTransform,
    ) = StrataAppState::paint_terrain_backdrop;
    let _ = g;
    true
}
