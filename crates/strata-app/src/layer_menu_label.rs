//! 레이어 메뉴 「라벨·명칭 속성」이 보는 라벨 상태 — C++ `LayerLabelControls::describe`(core/LayerLabelControls.cpp:87-150)를
//! 우리 라벨 모양(`survey_style::LabelRule` 과 사용자가 고른 것) 위에서 그대로. QGIS 라벨 식 대신 그 식이 무엇인지만 든다.

use crate::survey_area_setup::is_survey_area_key;

/// 레이어에 걸린 라벨(QGIS labeling 이 하는 일).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LabelSetting {
    /// 라벨 없음(labeling == nullptr) — 아는 항목(knownField)을 찾는다.
    Unconfigured,
    /// 항목 하나(simple field).
    Field(String),
    /// 면적 식 — 항목 없으면 면적만(`format_number(area($geometry), n) || ' ㎡'`), 있으면 「이름\n(면적 ㎡)」.
    Area(Option<String>),
    /// 면적을 끈 빈 식(`''`, ka_hgis/label_show_area 있음).
    Empty,
}

/// `LayerLabelControls::Info`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LabelInfo {
    pub(crate) supported: bool,
    pub(crate) needs_field: bool,
    pub(crate) area_editable: bool,
    pub(crate) field_editable: bool,
    pub(crate) caption: String,
    pub(crate) field: String,
    pub(crate) reason: String,
}

/// knownField — 이름·번호처럼 보이는 항목만(PNU·ID·A1 같은 것은 일부러 뺀다).
fn known_field(fields: &[String]) -> String {
    const NAMES: [&str; 23] = [
        "JIBUN", "지번", "survey_name", "feature_no", "artifact_no", "point_no", "유적명", "유적명칭", "사업명", "조사명",
        "site_name", "yujuk_nm", "yujeok_nm", "유구번호", "riv_nm", "하천명", "기호", "buld_nm", "road_nm", "지명", "명칭",
        "name", "title",
    ];
    let lookup = |want: &str| fields.iter().find(|f| *f == want).or_else(|| fields.iter().find(|f| f.eq_ignore_ascii_case(want)));
    NAMES.iter().find_map(|n| lookup(n)).cloned().unwrap_or_default()
}

/// describe — `vector` 가 아니면 이미지(래스터), `special` 은 지적·수치지형 묶음(ka_hgis/cadastral, topographic_group).
pub(crate) fn describe_label(setting: &LabelSetting, fields: &[String], vector: bool, polygon: bool, key: &str, special_layer: bool) -> LabelInfo {
    let mut info = LabelInfo { caption: "글자".to_owned(), ..LabelInfo::default() };
    if !vector {
        info.reason = "이미지에 포함된 글자는 따로 숨길 수 없습니다. 색상 범례는 도면에서 설정합니다.".to_owned();
        return info;
    }
    let configured = *setting != LabelSetting::Unconfigured;
    let has = |f: &str| fields.iter().any(|x| x.eq_ignore_ascii_case(f));
    info.field = match setting {
        LabelSetting::Unconfigured => known_field(fields),
        LabelSetting::Field(f) if has(f) => f.clone(),
        LabelSetting::Area(Some(f)) => f.clone(),
        _ => String::new(),
    };
    let area = matches!(setting, LabelSetting::Area(_));
    let empty_managed = *setting == LabelSetting::Empty;
    let survey_area = is_survey_area_key(key) && polygon;
    let expression = match setting {
        LabelSetting::Area(None) => "format_number(area($geometry), 2) || ' ㎡'".to_owned(),
        LabelSetting::Area(Some(f)) => format!("coalesce(\"{f}\", '') || '\\n(' || format_number(area($geometry), 1) || ' ㎡)'"),
        LabelSetting::Empty => "''".to_owned(),
        LabelSetting::Field(f) => f.clone(),
        LabelSetting::Unconfigured => info.field.clone(),
    };
    let lower = expression.to_lowercase();
    let height_words = lower.contains("elevation") || lower.contains("z_min(") || lower.contains("등고수치") || lower.contains("\"수치\"");
    let special = special_layer
        || ["JIBUN", "PNU"].iter().any(|s| info.field.eq_ignore_ascii_case(s))
        || info.field == "지번"
        || info.field == "기호"
        || height_words
        || lower.contains("soil_type_geo")
        || lower.contains("riv_nm");
    info.area_editable = polygon && !special && ((!configured && survey_area) || area || empty_managed || !info.field.is_empty());
    info.supported = configured || !info.field.is_empty() || survey_area;
    info.needs_field = !info.supported;
    info.field_editable = !special && (info.needs_field || info.area_editable || !info.field.is_empty());
    if !info.supported {
        info.reason = "표시할 내용을 먼저 선택해 주세요.".to_owned();
        return info;
    }
    info.caption = if lower.contains("jibun") || lower.contains("지번") {
        "지번"
    } else if height_words || info.field == "수치" {
        "높이"
    } else if area || empty_managed || (!configured && survey_area) {
        if info.field.is_empty() { "면적" } else { "이름·면적" }
    } else if info.field == "기호" {
        "기호"
    } else if lower.contains("riv_nm") || info.field == "하천명" {
        "하천명"
    } else if lower.contains("soil_type_geo") {
        "지형명"
    } else if info.field.contains("_no") || info.field.contains("번호") {
        "번호"
    } else {
        "이름"
    }
    .to_owned();
    info
}

/// MainWindowContextMenus.cpp:96-108 fieldTitle — 별칭이 없으면 이 표, 그래도 없으면 항목 이름.
pub(crate) fn field_title(field: &str) -> String {
    match field {
        "survey_name" => "조사명",
        "feature_no" => "유구 번호",
        "kind" => "종류",
        "period" => "시대",
        "point_id" => "기준점 번호",
        "section_id" => "단면 번호",
        "name" => "이름",
        "note" => "비고",
        "area_m2" => "면적(㎡)",
        other => return other.to_owned(),
    }
    .to_owned()
}

#[cfg(test)]
#[path = "layer_menu_label_tests.rs"]
mod tests;
