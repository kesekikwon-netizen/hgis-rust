//! 레이어 카드에 그리는 행 명세와 그 순서.
//!
//! C++ `KaLayerInformationPanel` 은 QGIS 레이어 트리를 **실제 레이어만** 그린다.
//! 예전 Rust 판은 14행을 고정으로 그려서, 빈 새 조사에도 「시굴격자 232」 같은
//! 데모 행이 남아 있었다(사용자 지적, 2026-11-09). 이제 `LeftLayerPanel.entries`
//! 를 그대로 옮겨 그린다 — 레이어가 없으면 행도 없다.

use eframe::egui::Color32;
use strata_types::schema::LegendGroup;

use super::gui_layer_panel_paint::SwatchKind;
use crate::left_layer_panel::{LayerTreeEntry, LeftLayerPanel};

/// 한 행을 그리는 데 필요한 값.
pub(crate) struct LayerRowSpec {
    /// 이 행이 속한 묶음 머리글(빈 문자열이면 없음).
    pub section: &'static str,
    /// 묶음 머리글 위에 구분선을 그릴지.
    pub top_rule: bool,
    /// 왼쪽 들여쓰기(px).
    pub indent: f32,
    /// 접이식 화살표(없으면 None).
    pub arrow: Option<&'static str>,
    /// 기호 견본 종류.
    pub swatch: SwatchKind,
    /// 화면에 보이는 이름(한국어).
    pub name: String,
    /// 오른쪽에 셀 개수 배지를 그릴지.
    pub show_count: bool,
    /// 오른쪽 칸 이름.
    pub right_label: Option<&'static str>,
    /// 행 배경을 살짝 다르게(교차 줄무늬).
    pub alt_bg: bool,
    /// 글자색.
    pub ink: Color32,
}

fn swatch_for(entry: &LayerTreeEntry) -> SwatchKind {
    if entry.group == LegendGroup::Cadastral {
        return SwatchKind::CadastralOutline;
    }
    if entry.group == LegendGroup::Basemap {
        return SwatchKind::RasterChecker;
    }
    match entry.layer_key.as_str() {
        "survey_area" => SwatchKind::SurveyOrange,
        "trial_trench" => SwatchKind::TrenchBeige,
        "control_points" => SwatchKind::DistanceBox,
        // 버퍼 반경 레이어(C++ 「주변 500m」)는 띠 견본을 쓴다.
        k if k.contains("buffer") => SwatchKind::BufferBand,
        // 지정유산 하위 항목은 흰 견본(C++ 트리의 하위 노드).
        k if k.contains("heritage") => SwatchKind::HeritageSub,
        _ => SwatchKind::RasterChecker,
    }
}

/// 패널의 실제 레이어 목록에서 행 목록을 만든다.
///
/// 순서는 `LeftLayerPanel.entries` 순서 그대로다(= C++ 트리 순서).
/// 레이어가 하나도 없으면 빈 목록을 돌려주고, 화면에는 안내문만 남는다.
pub fn layer_rows(
    panel: &LeftLayerPanel,
    dark: Color32,
    blue: Color32,
    purple_grp: Color32,
    purple_leaf: Color32,
) -> Vec<LayerRowSpec> {
    let mut rows: Vec<LayerRowSpec> = Vec::with_capacity(panel.entries.len() + 4);
    let mut last_group: Option<LegendGroup> = None;

    for entry in &panel.entries {
        let group = entry.group;
        // 묶음이 바뀌면 머리글 행을 하나 넣는다(C++ 트리의 그룹 노드).
        if last_group != Some(group) {
            let (title, ink) = match group {
                LegendGroup::SurveyData => ("조사 데이터", blue),
                LegendGroup::Cadastral => ("지적도", dark),
                LegendGroup::ReferenceMap => ("주변·참조", purple_grp),
                LegendGroup::Basemap => ("배경 지도", blue),
            };
            // 첫 묶음 앞에는 구분선을 그리지 않는다.
            let top_rule = !rows.is_empty();
            rows.push(LayerRowSpec {
                section: title,
                top_rule,
                indent: 0.0,
                arrow: Some("▼"),
                swatch: SwatchKind::GroupFolder,
                name: String::new(),
                show_count: false,
                right_label: None,
                alt_bg: false,
                ink,
            });
            last_group = Some(group);
        }

        let ink = if group == LegendGroup::Cadastral {
            purple_leaf
        } else {
            dark
        };
        rows.push(LayerRowSpec {
            section: "",
            top_rule: false,
            indent: 0.0,
            arrow: None,
            swatch: swatch_for(entry),
            name: entry.label_ko.clone(),
            show_count: entry.feature_count > 0,
            right_label: None,
            alt_bg: false,
            ink,
        });
    }

    rows
}

/// 패널이 비었을 때 보여 줄 안내 문구(C++ `refreshLayerEmptyState`).
#[must_use]
pub fn layer_panel_empty_hint() -> &'static str {
    "아직 레이어가 없습니다. 리본 「자료 받기」에서 위성·지적을 켜세요."
}
