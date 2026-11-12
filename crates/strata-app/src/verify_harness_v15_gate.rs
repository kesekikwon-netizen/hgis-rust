//! 검증 게이트 V15 — 리본 시추 칩·`KaBoreholeDialog` 반경/카드/단추.
//!
//! `verify_harness.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 판정 조건·문구는 그대로다.

use crate::app_state::StrataAppState;
use super::verify_harness_types::GateResult;

/// V15 (`d7afcc8`): 리본 칩 최소폭 36px(10% 축소)·「자료 받기」 다섯째 단추 「시추」·`KaBoreholeDialog`.
/// C++ `KaTheme.h:97-98`, `MainWindowRibbon.cpp:532-547`, `KaBoreholeDialog.cpp:66-101` 1:1.
#[must_use]
pub fn v15_borehole_ribbon_and_dialog_gate() -> GateResult {
    use crate::dialogs_borehole::{BoreholeDialogAction, BoreholeDialogState};
    use crate::dialogs_borehole_spec::{BOREHOLE_MAP_CARDS, BOREHOLE_RADIUS_CHOICES};
    use crate::gui_ribbon_bar::UX_RIBBON_GROUPS;
    use strata_types::theme_tokens::ButtonMetrics;

    let metrics = ButtonMetrics::default();
    let chip_ok = metrics.ribbon_chip_width == 36 && metrics.ribbon_min_width == 36;

    // 「자료 받기」가 다섯 단추(DEM·지적·수치지형·유산·시추)이고 그 다섯째가 「시추」다.
    let fetch_group = UX_RIBBON_GROUPS
        .iter()
        .find(|(title, _)| *title == "자료 받기")
        .map(|(_, items)| *items);
    let fetch_ok = fetch_group
        .is_some_and(|items| items.len() == 5 && items[4] == "시추");

    // 시추 단추가 쓰는 Lucide glyph 가 등록돼 있고, tooltip 이 C++ 원문 그대로다.
    let icon_ok = crate::ribbon_icons::cpp_icon_id("시추") == Some("borehole")
        && crate::ribbon_icon_svg::svg_parts("borehole").is_some();
    let tooltip_ok = crate::ribbon_icons::cpp_ribbon_tooltip("시추")
        .starts_with("시추자료를 별도로 받아 5가지 지반·시추 GIS 도면");

    // 대화상자: 반경 세 선택지, 다섯 카드, 네 번째 카드의 EL.m 서브 체크, 세 단추.
    let d = BoreholeDialogState::default();
    let dialog_ok = BOREHOLE_RADIUS_CHOICES.len() == 3
        && d.radius_meters == 5000
        && d.radius_choice_index() == 1
        && BOREHOLE_MAP_CARDS.len() == 5
        && d.bedrock_depth_map
        && !d.bedrock_elev_map;

    // 조사 폴더가 없으면 창을 열지 않고 안내만 남긴다(`KaBoreholeDialog.cpp:66-70`).
    let mut app = StrataAppState::new_startup();
    let refused_ok = !app.open_borehole_dialog() && !app.borehole_dialog.open;
    app.survey_path = Some(std::env::temp_dir().join("strata-v15-gate.gpkg"));
    let opened_ok = app.open_borehole_dialog() && app.borehole_dialog.open;

    // 세 단추가 서로 다른 동작을 가리킨다(`KaBoreholeDialog.cpp:169-192`).
    let actions_ok = BoreholeDialogAction::FetchAndGenerate {
        radius_meters: 5000,
    } != BoreholeDialogAction::Regenerate
        && BoreholeDialogAction::Regenerate != BoreholeDialogAction::OpenLayout;

    GateResult {
        gate_id: "V15",
        title_ko: "시추 버튼 36px 칩 · KaBoreholeDialog 반경/5카드/3단추",
        passed: chip_ok
            && fetch_ok
            && icon_ok
            && tooltip_ok
            && dialog_ok
            && refused_ok
            && opened_ok
            && actions_ok,
        detail: format!(
            "chip={}px fetch={:?} cards={} radius={}m",
            metrics.ribbon_chip_width,
            fetch_group.map(<[&str]>::len),
            BOREHOLE_MAP_CARDS.len(),
            d.radius_meters
        ),
    }
}
