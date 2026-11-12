//! 레이어 카드에 그리는 행 명세와 그 순서.
//!
//! C++ `KaLayerInformationPanel` 은 QGIS 레이어 트리를 **실제 레이어만** 그린다.
//! 예전 Rust 판은 14행을 고정으로 그려서, 빈 새 조사에도 「시굴격자 232」 같은
//! 데모 행이 남아 있었다(사용자 지적, 2026-11-09). 이제 `LeftLayerPanel.entries`
//! 를 그대로 옮겨 그린다 — 레이어가 없으면 행도 없다.

use eframe::egui::Color32;
use strata_types::schema::LegendGroup;

pub(crate) use super::gui_layer_panel_paint::SwatchKind;
use crate::left_layer_panel::{LayerTreeEntry, LeftLayerPanel};

/// 한 행을 그리는 데 필요한 값.
pub(crate) struct LayerRowSpec {
    /// 이 행 위에 붙는 구역 띠 제목(빈 문자열이면 띠 없음). 띠에는 윗줄이 있다.
    pub section: &'static str,
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
    /// 그 레이어의 도형 수(이름이 같은 레이어가 있어도 행마다 제 수).
    pub count: usize,
    /// 오른쪽 칸 이름.
    pub right_label: Option<&'static str>,
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
    // C++ `LayerStyleDefaults.cpp:42-86` forKey — 조사 레이어 기본 색(다른 조사 표는 기본 파랑).
    let rgba = Color32::from_rgba_unmultiplied;
    let rgb = Color32::from_rgb;
    match entry.layer_key.as_str() {
        "survey_area" => SwatchKind::DomainFill { fill: rgba(180, 83, 9, 70), stroke: rgb(146, 64, 14), stroke_mm: 1.6 },
        "feature_poly" => SwatchKind::DomainFill { fill: rgba(22, 163, 74, 90), stroke: rgb(17, 94, 44), stroke_mm: 1.8 },
        "feature_line" => SwatchKind::DomainLine { stroke: rgb(202, 138, 4), width_mm: 1.8, casing: false },
        "section_line" => SwatchKind::DomainLine { stroke: rgb(190, 24, 93), width_mm: 1.8, casing: true },
        "control_points" => SwatchKind::DomainMarker { fill: rgb(234, 179, 8), stroke: rgb(161, 98, 7), size_mm: 4.0 },
        "artifact_point" => SwatchKind::DomainMarker { fill: rgb(185, 28, 28), stroke: rgb(127, 29, 29), size_mm: 3.6 },
        "trial_trench" => SwatchKind::DomainFill { fill: rgba(220, 38, 38, 18), stroke: rgb(220, 38, 38), stroke_mm: 0.5 },
        // 버퍼 반경 레이어(C++ 「주변 500m」)는 띠 견본을 쓴다.
        k if k.contains("buffer") => SwatchKind::BufferBand,
        // 지정유산 하위 항목은 흰 견본(C++ 트리의 하위 노드).
        k if k.contains("heritage") => SwatchKind::HeritageSub,
        _ if entry.group == LegendGroup::SurveyData => {
            SwatchKind::DomainFill { fill: rgba(37, 99, 235, 90), stroke: rgb(37, 99, 235), stroke_mm: 1.2 }
        }
        _ => SwatchKind::RasterChecker,
    }
}

/// 조사 레이어의 「글자」 칸 이름 — C++ `LayerLabelControls.cpp:140-145` 가 새 레이어 기본
/// 라벨(`LayerOps.cpp:227-233` applyDomainDrawStyle)에 붙이는 이름: 시굴격자는 이름+면적,
/// 다른 면은 면적, 선·점은 이름.
fn label_caption(entry: &LayerTreeEntry) -> Option<&'static str> {
    if entry.group != LegendGroup::SurveyData {
        return None;
    }
    Some(match entry.layer_key.as_str() {
        "trial_trench" => "이름·면적",
        "survey_area" | "feature_poly" => "면적",
        k if k.starts_with("user_poly_") => "면적",
        _ => "이름",
    })
}

/// 패널의 실제 레이어 목록에서 행 목록을 만든다.
///
/// 순서는 `LeftLayerPanel.entries` 순서 그대로다(= C++ 트리 순서).
/// 레이어가 하나도 없으면 빈 목록을 돌려주고, 화면에는 안내문만 남는다.
pub fn layer_rows(panel: &LeftLayerPanel, dark: Color32, purple_leaf: Color32) -> Vec<LayerRowSpec> {
    let mut rows: Vec<LayerRowSpec> = Vec::with_capacity(panel.entries.len());
    let mut last_group: Option<LegendGroup> = None;

    for entry in &panel.entries {
        let group = entry.group;
        // C++ `KaLayerSectionDelegate.cpp:46-69` — 구역이 바뀌는 첫 행 위에 띠 제목을 붙인다
        // (따로 한 줄을 만들지 않는다).
        let section = if last_group == Some(group) {
            ""
        } else {
            last_group = Some(group);
            match group {
                LegendGroup::SurveyData => "조사 데이터",
                LegendGroup::Cadastral => "지적도",
                LegendGroup::ReferenceMap => "주변·참조",
                LegendGroup::Basemap => "배경 지도",
            }
        };
        let ink = if group == LegendGroup::Cadastral {
            purple_leaf
        } else {
            dark
        };
        rows.push(LayerRowSpec {
            section,
            indent: 0.0,
            arrow: None,
            swatch: swatch_for(entry),
            name: entry.label_ko.clone(),
            show_count: entry.feature_count > 0,
            count: entry.feature_count,
            right_label: label_caption(entry),
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
