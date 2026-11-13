//! 레이어 목록 우클릭 메뉴의 줄 — C++ `MainWindow::showLayerTreeContextMenu`(MainWindowContextMenus.cpp:176-448)와 가지
//! `menuKind`(:44-67). 줄·C++ 객체 이름·구분선(Qt 처럼 겹치거나 앞뒤에 남은 것은 숨김)·하위 메뉴·체크·흐림 이유를 만든다.
//! 아직 잇지 않은 줄은 C++ 자리에 C++ 글로 흐리게 두고 툴팁에 지시 번호(CM2a-1, hgis-2e). 그리기는 gui_layer_panel_context_menu.

use crate::layer_menu_label::{field_title, LabelInfo};
use crate::survey_area_setup::is_survey_area_key;

/// 동작이 있는 줄의 C++ 객체 이름(스냅 단계가 이름을 &'static str 로 찾는다).
pub(crate) const MENU_IDS: [&str; 13] = [
    "layer.style", "layer.draw", "layer.area", "layer.zoom", "layer.fullExtent", "layer.labelsVisible", "layer.rename",
    "layer.labelFieldValue", "layer.labelArea", "layer.labelSizeValue", "layer.opacityValue", "layer.bottom", "layer.remove",
];

/// menuKind 의 가지.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerMenuKind {
    Area,
    Feature,
    Control,
    Section,
    Trench,
    Reference,
    ExternalVector,
    ExternalRaster,
}

/// :44-63 — 조사 레이어는 layer_key 로(한글 제목이 아니다), 그 밖은 참조·지도 바탕이면 Reference, 아니면 바깥 자료.
pub(crate) fn menu_kind(layer_key: &str, vector: bool, reference: bool) -> LayerMenuKind {
    match layer_key {
        k if vector && is_survey_area_key(k) => LayerMenuKind::Area,
        "control_points" if vector => LayerMenuKind::Control,
        "section_line" if vector => LayerMenuKind::Section,
        "trial_trench" if vector => LayerMenuKind::Trench,
        "feature_poly" | "feature_line" | "artifact_point" if vector => LayerMenuKind::Feature,
        _ if reference => LayerMenuKind::Reference,
        _ if vector => LayerMenuKind::ExternalVector,
        _ => LayerMenuKind::ExternalRaster,
    }
}

/// 도형 형식(래스터는 None).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuGeom {
    Point,
    Line,
    Polygon,
}

/// 메뉴 한 줄(하위 메뉴면 `submenu`). `value` 는 고른 항목(라벨 항목 이름·글자 크기·투명도 %).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MenuRow {
    pub(crate) id: &'static str,
    pub(crate) text: String,
    pub(crate) enabled: bool,
    pub(crate) tooltip: String,
    pub(crate) checked: Option<bool>,
    pub(crate) value: String,
    pub(crate) submenu: Option<Vec<MenuEntry>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MenuEntry {
    Row(MenuRow),
    Separator,
}

/// 메뉴를 만들 때 보는 레이어 상태.
#[derive(Debug, Clone)]
pub(crate) struct LayerMenuInput {
    pub(crate) kind: LayerMenuKind,
    pub(crate) vector: bool,
    pub(crate) valid: bool,
    pub(crate) feature_count: usize,
    pub(crate) geom: Option<MenuGeom>,
    /// m_isOpeningSurvey.
    pub(crate) busy: bool,
    pub(crate) survey_open: bool,
    /// 조사 레이어이고 원본이 지금 조사 파일.
    pub(crate) own_survey: bool,
    pub(crate) read_only: bool,
    /// LayerOps::isReferenceLayer || isCadastralLayer — 편집 줄을 숨긴다.
    pub(crate) reference_or_cadastral: bool,
    /// 원본이 .shp.
    pub(crate) shp: bool,
    /// 공급자 wms + type=xyz.
    pub(crate) xyz: bool,
    pub(crate) at_group_bottom: bool,
    /// layer->opacity().
    pub(crate) opacity: f64,
    pub(crate) label: LabelInfo,
    pub(crate) has_labeling: bool,
    /// vector->labelsEnabled() — 「숨기기」/「보이기」.
    pub(crate) labels_visible: bool,
    /// QGIS 가 보이는 항목(GPKG 는 fid 가 맨 앞).
    pub(crate) fields: Vec<String>,
    pub(crate) current_label_field: String,
    pub(crate) show_area: bool,
    pub(crate) label_size_pt: f64,
}

fn row(id: &'static str, text: impl Into<String>, reason: &str, help: &str) -> MenuRow {
    let enabled = reason.is_empty();
    MenuRow { id, text: text.into(), enabled, tooltip: if enabled { help } else { reason }.to_owned(), checked: None, value: String::new(), submenu: None }
}

/// 아직 잇지 않은 줄 — C++ 이 흐리는 이유가 있으면 그것, 없으면 지시 번호.
fn deferred(mut r: MenuRow, order: &str) -> MenuRow {
    if r.enabled {
        r.tooltip = format!("아직 Rust 판에서 연결하지 않았습니다 (지시 {order})");
    }
    r.enabled = false;
    r
}

fn checkable(mut r: MenuRow, on: bool, value: impl Into<String>) -> MenuRow {
    r.checked = Some(on);
    r.value = value.into();
    r
}

fn submenu(id: &'static str, text: &str, reason: &str, help: &str, items: Vec<MenuEntry>) -> MenuRow {
    let mut r = row(id, text, reason, help);
    r.submenu = Some(items);
    r
}

/// Qt QMenu(separatorsCollapsible) — 겹친 구분선은 하나로, 앞뒤에 남은 것은 숨긴다.
pub(crate) fn collapse_separators(entries: Vec<MenuEntry>) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = Vec::new();
    for e in entries {
        match e {
            MenuEntry::Separator if out.is_empty() || matches!(out.last(), Some(MenuEntry::Separator)) => {}
            MenuEntry::Row(mut r) => {
                r.submenu = r.submenu.map(collapse_separators);
                out.push(MenuEntry::Row(r));
            }
            sep => out.push(sep),
        }
    }
    while matches!(out.last(), Some(MenuEntry::Separator)) {
        out.pop();
    }
    out
}

/// 「표시할 항목」 — 항목이 10개를 넘으면 「항목 1–10」 묶음, 90개마다 「다음 항목…」(:311-325).
fn field_rows(m: &LayerMenuInput) -> Vec<MenuEntry> {
    let n = m.fields.len();
    let value = |f: &String| MenuEntry::Row(checkable(row("layer.labelFieldValue", field_title(f), "", ""), *f == m.current_label_field, f.clone()));
    if n <= 10 {
        return m.fields.iter().map(value).collect();
    }
    let groups: Vec<MenuEntry> = m
        .fields
        .chunks(10)
        .enumerate()
        .map(|(g, chunk)| MenuEntry::Row(submenu("layer.labelFields", &format!("항목 {}–{}", g * 10 + 1, (g * 10 + 10).min(n)), "", "", chunk.iter().map(value).collect())))
        .collect();
    // 90 개(9 묶음)마다 그 아래 「다음 항목…」 안으로 들어간다.
    let mut pages = groups.chunks(9).rev();
    let mut tail: Option<Vec<MenuEntry>> = None;
    for page in pages.by_ref() {
        let mut items = page.to_vec();
        if let Some(rest) = tail.take() {
            items.push(MenuEntry::Row(submenu("layer.labelFieldsNext", "다음 항목…", "", "", rest)));
        }
        tail = Some(items);
    }
    tail.unwrap_or_default()
}

/// 레이어 줄의 메뉴.
pub(crate) fn layer_menu_entries(m: &LayerMenuInput) -> Vec<MenuEntry> {
    let domain = matches!(m.kind, LayerMenuKind::Area | LayerMenuKind::Feature | LayerMenuKind::Control | LayerMenuKind::Section | LayerMenuKind::Trench);
    let invalid = if m.valid { "" } else { "레이어 파일을 찾을 수 없습니다. 자료를 다시 열어 주세요." };
    let empty = if !m.valid { invalid } else if m.vector && m.feature_count == 0 { "이 레이어에는 도형이 없습니다." } else { "" };
    let busy = if m.busy { "조사 파일 처리가 끝난 뒤 다시 실행하세요." } else { "" };
    let not_own = "이 자료가 저장된 조사 파일을 먼저 열어 주세요.";
    let edit_unavailable = if !m.valid { "레이어 파일을 다시 열어 주세요." } else if m.read_only { "읽기 전용 자료입니다. 편집 가능한 조사 자료를 선택하세요." } else { "" };
    let draw_reason = if m.busy { busy } else if domain && !m.own_survey { not_own } else { edit_unavailable };
    let vertex_reason = if m.busy { busy } else if domain && !m.own_survey { not_own } else if !empty.is_empty() { empty } else { edit_unavailable };
    let mut out = vec![MenuEntry::Row(deferred(row("layer.import", "레이어 불러오기 (SHP·DXF·GPKG)", busy, ""), "W-A12")), MenuEntry::Separator];
    // appearance — 이름 바꾸기, 면·외곽선 색, 도형 속성, 라벨·명칭 속성 ▸, (깨진 한글 바로잡기 ▸).
    out.push(MenuEntry::Row(row("layer.rename", "이름 바꾸기 (두 번 클릭)", "", "")));
    out.push(MenuEntry::Row(row("layer.style", "면·외곽선 색", if m.vector { invalid } else { "영상 지도는 투명도로 표시를 조절하세요." }, "")));
    let attr_reason = if m.busy { busy } else if domain && !m.own_survey { not_own } else if !empty.is_empty() { empty } else { edit_unavailable };
    out.push(MenuEntry::Row(deferred(row("layer.attributes", "도형 속성", if m.vector { attr_reason } else { "영상 지도에는 편집할 도형 속성이 없습니다." }, ""), "CM2a-3")));
    out.push(MenuEntry::Row(label_menu(m)));
    if m.vector && m.shp {
        let items = ["CP949", "EUC-KR", "UTF-8", "System"]
            .iter()
            .map(|v| {
                let mut r = row("layer.encodingValue", if *v == "System" { "이 컴퓨터 설정" } else { v }, invalid, "");
                r.value = (*v).to_owned();
                MenuEntry::Row(r)
            })
            .collect();
        out.push(MenuEntry::Row(submenu("layer.encoding", "깨진 한글 바로잡기", "", "", items)));
    }
    // 투명도 ▸ — 보이기만(자료·색은 그대로). 체크 = 지금 불투명도.
    let opacity_reason = if m.valid { "" } else { "레이어 파일을 찾을 수 없습니다. 자료를 다시 열어 주세요." };
    let opacity_items = [0, 20, 40, 50, 60, 80]
        .iter()
        .map(|t| MenuEntry::Row(checkable(row("layer.opacityValue", format!("{t}% 투명하게"), opacity_reason, ""), (m.opacity * 100.0 - f64::from(100 - t)).abs() < 1.0, t.to_string())))
        .collect();
    out.push(MenuEntry::Row(submenu("layer.opacity", "투명도", opacity_reason, "지도에서만 흐리게 보입니다. 자료와 색은 바뀌지 않습니다.", opacity_items)));
    out.push(MenuEntry::Separator);
    out.push(MenuEntry::Row(row("layer.zoom", "이 레이어로 이동", if m.vector { empty } else { invalid }, "")));
    out.push(MenuEntry::Row(row("layer.fullExtent", "전체 보기", "", "")));
    out.push(MenuEntry::Row(row("layer.bottom", "같은 묶음의 맨 아래로", if m.at_group_bottom { "이 묶음에서 이미 맨 아래에 있습니다." } else { "" }, "")));
    // 도형 묶음 — 이 레이어에 맞는 줄만.
    let polygon = m.vector && m.geom == Some(MenuGeom::Polygon);
    let editable = m.vector && m.kind != LayerMenuKind::Reference && !m.reference_or_cadastral;
    if editable || polygon || m.kind == LayerMenuKind::Area {
        out.push(MenuEntry::Separator);
    }
    if editable {
        out.push(MenuEntry::Row(row("layer.draw", "그리기·편집 시작", draw_reason, "")));
        if m.geom != Some(MenuGeom::Point) {
            out.push(MenuEntry::Row(deferred(row("layer.vertices", "꼭짓점 수정", vertex_reason, ""), "CM2a-2")));
        }
    }
    if editable && polygon {
        let r = row("layer.merge", "폴리곤 묶기", vertex_reason, "지도에서 고른 면 2개 이상을 하나로 묶습니다. Ctrl+Z로 되돌릴 수 있습니다.");
        out.push(MenuEntry::Row(deferred(r, "CM2a-2")));
    }
    if polygon {
        out.push(MenuEntry::Row(row("layer.area", if m.kind == LayerMenuKind::Trench { "면적·조사 비율 확인" } else { "면적 확인" }, empty, "")));
    }
    if m.kind == LayerMenuKind::Area {
        let reason = if draw_reason.is_empty() { empty } else { draw_reason };
        let items = [("layer.trench10", "시굴 · 조사구역의 10%"), ("layer.trench2", "표본 · 조사구역의 2%"), ("layer.trenchCustom", "규격 지정·격자 만들기…")];
        let items = items.iter().map(|(id, t)| MenuEntry::Row(deferred(row(id, *t, reason, ""), "W-T1"))).collect();
        out.push(MenuEntry::Row(deferred(submenu("layer.trenchCreate", "시굴격자", reason, "", items), "W-T1")));
    }
    if m.xyz {
        out.push(MenuEntry::Row(deferred(row("layer.offline", "오프라인 저장 — 지금 화면 범위", invalid, ""), "R4b")));
    }
    if domain {
        out.push(MenuEntry::Separator);
        let reason = if m.busy { busy } else if !m.survey_open { "먼저 조사 파일을 열어 주세요." } else { "" };
        let help = "현재 조사 전체를 검수한 뒤 5179 SHP·조사도면.pdf·MANIFEST 제출 꾸러미를 만듭니다. 검수 오류가 남아 있으면 한 번 묻고 README에 적습니다.";
        out.push(MenuEntry::Row(deferred(row("layer.export", "제출 꾸러미 만들기…", reason, help), "R2")));
    }
    out.push(MenuEntry::Separator);
    if domain {
        let reason = if m.busy { busy } else if !m.own_survey { not_own } else { empty };
        out.push(MenuEntry::Row(deferred(row("layer.clear", "그린 도형 모두 지우기…", reason, ""), "CM2a-2")));
    }
    out.push(MenuEntry::Row(row("layer.remove", "레이어 삭제", busy, "목록에서 제거합니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다.")));
    collapse_separators(out)
}

/// 「라벨·명칭 속성」 ▸(:288-345).
fn label_menu(m: &LayerMenuInput) -> MenuRow {
    if !m.vector {
        return submenu("layer.labels", "라벨·명칭 속성", "영상에 포함된 글자는 별도로 크기를 바꿀 수 없습니다.", "", Vec::new());
    }
    let labelable = m.valid && m.label.supported;
    let reason = if labelable { "" } else { "표시할 이름이나 번호 항목이 없습니다." };
    let on = m.labels_visible;
    let mut items = vec![MenuEntry::Row(row("layer.labelsVisible", format!("{} {}", m.label.caption, if on { "숨기기" } else { "보이기" }), reason, ""))];
    let mut fields = submenu("layer.labelField", "표시할 항목", "", "", field_rows(m));
    fields.enabled = m.label.field_editable;
    items.push(MenuEntry::Row(fields));
    if m.label.area_editable {
        items.push(MenuEntry::Row(checkable(row("layer.labelArea", "면적(㎡) 함께 표시", reason, ""), m.show_area, "")));
    }
    let sizes = [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0]
        .iter()
        .map(|s: &f64| MenuEntry::Row(checkable(row("layer.labelSizeValue", format!("{s} pt"), reason, ""), (s - m.label_size_pt).abs() < 1e-9, s.to_string())))
        .collect();
    let mut size_menu = submenu("layer.labelSize", "글자 크기", "", "", sizes);
    size_menu.enabled = m.has_labeling;
    items.push(MenuEntry::Row(size_menu));
    submenu("layer.labels", "라벨·명칭 속성", reason, "", items)
}

/// 레이어가 아닌 줄(빈 곳·묶음 머리, :195-213).
pub(crate) fn blank_menu_entries(removable: bool, group: bool) -> Vec<MenuEntry> {
    let mut out = Vec::new();
    if removable || group {
        let help = if group {
            "이 묶음과 안의 레이어를 목록과 지도에서 뺍니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다."
        } else {
            "목록에서 제거합니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다."
        };
        out.push(MenuEntry::Row(row("layer.remove", "레이어 삭제", "", help)));
    }
    out.push(MenuEntry::Row(deferred(row("layer.import", "참고 자료 불러오기…", "", ""), "W-A12")));
    out
}

#[cfg(test)]
#[path = "layer_menu_model_tests.rs"]
mod tests;
