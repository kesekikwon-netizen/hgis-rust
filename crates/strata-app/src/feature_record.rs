//! 「도형 속성」 기록 — C++ `MainWindow::editFeatureAttributes`(MainWindowEditing.cpp:616-776)의 칸 고르기·편집기·값 검사,
//! 새 모양의 구역 배치 `KaRecordPlate::arrange`(KaRecordPlate.cpp:9-21, 180-266), 낱말 `attributeFieldLabelKo`
//! (MainWindowEditing.cpp:287-311) + `SurveySchema::labelKo`, 표고 그림 글 `KaElevationFigure`(KaElevationFigure.cpp:12-48).
//! 창 모양은 2쪽(dlg), 오른쪽 카드는 3쪽.

use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::checklist_state::attr;
use crate::feature_form::{canonical_value, choices};

/// KaRecordPlate.cpp:10 kSections.
pub(crate) const RECORD_SECTIONS: [&str; 5] = ["식별", "층위", "표고", "조사", "사진 · 비고"];
/// 표고 구역(상면·바닥 칸 옆에 단면 그림).
pub(crate) const ELEVATION_SECTION: usize = 2;
/// MainWindowEditing.cpp:647-648.
pub(crate) const RECORD_TIP: &str = "그린 도형의 속성입니다. 종류·시대 등을 입력한 뒤 저장하세요.";
/// MainWindowEditing.cpp:689-690.
pub(crate) const NO_FIELDS: &str = "이 레이어에 편집할 속성 필드가 없습니다.";
/// 「저장」(setText)·「취소」(새 모양 KaRecordPlate.cpp:256 setText, qtbase_ko 도 「취소」).
pub(crate) const RECORD_BUTTONS: [&str; 2] = ["저장", "취소"];

/// KaRecordPlate.cpp:13-21 kPlaces — (칸, 구역, 한 줄 전체). 여기 없는 칸은 식별 끝에.
const PLACES: [(&str, usize, bool); 16] = [
    ("kind", 0, false),
    ("period", 0, false),
    ("feature_no", 0, false),
    ("survey_name", 0, false),
    ("site_name", 0, false),
    ("layer_no", 1, false),
    ("relation", 1, true),
    ("top_el", 2, false),
    ("bottom_el", 2, false),
    ("depth_m", 2, false),
    ("z", 2, false),
    ("status", 3, false),
    ("surv_date", 3, false),
    ("surveyor", 3, false),
    ("photo", 4, true),
    ("note", 4, true),
];

/// attributeFieldLabelKo 의 표, 없으면 SurveySchema::labelKo(레이어 차례로 처음 것 — kind 는 「유구종류」), 그래도 없으면 칸 이름.
pub(crate) fn field_label_ko(field: &str) -> String {
    const LABELS: [(&str, &str); 38] = [
        ("survey_name", "조사명"),
        ("site_name", "유적명"),
        ("kind", "유구종류"),
        ("period", "시대"),
        ("feature_no", "유구번호"),
        ("artifact_no", "유물번호"),
        ("note", "비고"),
        ("section_id", "단면번호"),
        ("point_id", "점ID"),
        ("x", "X"),
        ("y", "Y"),
        ("z", "표고 Z"),
        ("datum", "측지기준계"),
        ("ellipsoid", "타원체"),
        ("projection", "투영"),
        ("origin", "원점"),
        ("accuracy", "정확도 메모"),
        ("accuracy_m", "정확도(m)"),
        ("pdop", "PDOP"),
        ("fix_type", "수신상태"),
        ("pixel_x", "픽셀 X"),
        ("pixel_y", "픽셀 Y"),
        // SurveySchema.cpp:28-93 의 기록 칸.
        ("layer_no", "층위·문맥 번호"),
        ("depth_m", "깊이(m)"),
        ("top_el", "상면 표고(m)"),
        ("bottom_el", "바닥 표고(m)"),
        ("relation", "선후관계"),
        ("status", "조사 상태"),
        ("photo", "사진"),
        ("surveyor", "조사자"),
        ("surv_date", "조사일"),
        ("in_feature", "소속 유구"),
        ("length", "길이"),
        ("name", "이름"),
        ("width", "폭"),
        ("uid", "고유번호"),
        ("created_at", "만든 시각"),
        ("updated_at", "고친 시각"),
    ];
    LABELS.iter().find(|(f, _)| *f == field).map_or_else(|| field.to_owned(), |(_, l)| (*l).to_owned())
}

/// KaFeatureFormDialog.cpp:35-40 placeholderFor.
pub(crate) fn placeholder_for(field: &str) -> &'static str {
    match field {
        "kind" => "예: 주거지, 수혈 — 목록에 없으면 직접 적으세요",
        "period" => "예: 청동기, 원삼국",
        "feature_no" => "예: 1호",
        _ => "",
    }
}

/// QString::toDouble(C 로캘) — inf·nan 은 받고 infinity·쉼표·빈칸 묶음은 받지 않는다(Qt 로 확인).
pub(crate) fn qt_to_double(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.to_ascii_lowercase().contains("infinity") {
        return None;
    }
    t.parse::<f64>().ok()
}

/// QString::toLongLong(10진).
pub(crate) fn qt_to_long_long(text: &str) -> Option<i64> {
    text.trim().parse::<i64>().ok()
}

/// 칸의 편집기 — createValueEditor(종류·시대 목록, 새 모양 비고 여러 줄, 한 줄) 와 숫자 칸(MainWindowEditing.cpp:673-677).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordEditor {
    Real,
    Integer,
    Choice,
    Lines,
    Line,
}

/// 칸 하나 — 화면 차례대로 `FeatureRecordForm::rows` 에 들어 있다.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordRow {
    pub(crate) field: String,
    pub(crate) label: String,
    pub(crate) editor: RecordEditor,
    pub(crate) section: usize,
    pub(crate) across: bool,
    pub(crate) text: String,
}

/// KaUserError::warn 의 네 글(제목·무엇·왜·어떻게).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordWarning {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) detail: String,
    pub(crate) action: String,
}

/// 열려 있는 「도형 속성」 창의 값.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeatureRecordForm {
    pub(crate) table: String,
    pub(crate) fid: i64,
    pub(crate) layer_name: String,
    pub(crate) rows: Vec<RecordRow>,
    pub(crate) kind_choices: Vec<String>,
    pub(crate) period_choices: Vec<String>,
    /// 연 프레임의 키·누름은 창의 것이 아니다(3-3b 와 같은 규칙).
    pub(crate) keys_armed: bool,
}

fn editor_for(field: &str, declared: &str) -> RecordEditor {
    let ty = declared.to_ascii_uppercase();
    if ty.contains("INT") {
        RecordEditor::Integer
    } else if ty.contains("REAL") || ty.contains("DOUB") || ty.contains("FLOA") {
        RecordEditor::Real
    } else if field == "kind" || field == "period" {
        RecordEditor::Choice
    } else if field == "note" {
        RecordEditor::Lines
    } else {
        RecordEditor::Line
    }
}

fn is_auto_field(field: &str) -> bool {
    ["uid", "created_at", "updated_at"].iter().any(|a| a.eq_ignore_ascii_case(field))
}

impl FeatureRecordForm {
    /// 그 표의 도형 `fid` 의 창. 고칠 칸이 없으면 `NO_FIELDS`, 도형이 없으면 「고른 도형을 다시 읽지 못했습니다.」.
    pub(crate) fn new(t: &SurveyFileTable, fid: i64, layer_name: &str) -> Result<Self, &'static str> {
        let feature = t.layer.features.iter().find(|f| f.fid == fid).ok_or("고른 도형을 다시 읽지 못했습니다.")?;
        let mut found: Vec<RecordRow> = t
            .columns
            .iter()
            .filter(|(c, _)| !c.eq_ignore_ascii_case("fid") && !c.to_ascii_lowercase().starts_with("ogc_") && !is_auto_field(c))
            .map(|(c, ty)| RecordRow {
                field: c.clone(),
                label: field_label_ko(c),
                editor: editor_for(c, ty),
                section: 0,
                across: false,
                text: attr(feature, c).unwrap_or_default(),
            })
            .collect();
        if found.is_empty() {
            return Err(NO_FIELDS);
        }
        // KaRecordPlate::arrange — 구역마다 보드 차례, 여기 없는 칸은 식별 끝(파일 차례).
        let mut rows = Vec::with_capacity(found.len());
        for section in 0..RECORD_SECTIONS.len() {
            for (field, s, across) in PLACES.iter().filter(|p| p.1 == section) {
                if let Some(i) = found.iter().position(|r| r.field == *field) {
                    let mut row = found.remove(i);
                    (row.section, row.across) = (*s, *across);
                    rows.push(row);
                }
            }
            if section == 0 {
                let (unplaced, rest): (Vec<RecordRow>, Vec<RecordRow>) =
                    found.into_iter().partition(|r| !PLACES.iter().any(|p| p.0 == r.field));
                rows.extend(unplaced);
                found = rest;
            }
        }
        let has = |f: &str| rows.iter().any(|r| r.field == f && r.editor == RecordEditor::Choice);
        Ok(Self {
            table: t.table.clone(),
            fid,
            layer_name: layer_name.to_owned(),
            kind_choices: if has("kind") { choices(t, "kind") } else { Vec::new() },
            period_choices: if has("period") { choices(t, "period") } else { Vec::new() },
            rows,
            keys_armed: false,
        })
    }

    /// 창 제목(MainWindowEditing.cpp:643).
    pub(crate) fn title(&self) -> String {
        format!("도형 속성 — {}", self.layer_name)
    }

    /// 칸이 있는 구역의 낱말(위에서 아래로).
    pub(crate) fn section_words(&self) -> Vec<&'static str> {
        (0..RECORD_SECTIONS.len()).filter(|s| self.rows.iter().any(|r| r.section == *s)).map(|s| RECORD_SECTIONS[s]).collect()
    }

    /// 저장할 값 — 모든 칸을 먼저 검사하고(나쁜 숫자 하나면 아무것도 바꾸지 않는다), 빈 칸은 NULL(None),
    /// 종류·시대는 조사 맞춤법(valueEditorText), 숫자는 숫자 글로(MainWindowEditing.cpp:700-731).
    pub(crate) fn values(&self, t: &SurveyFileTable) -> Result<Vec<(String, Option<String>)>, RecordWarning> {
        let mut out = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let text = row.text.trim();
            let value = if text.is_empty() {
                None
            } else {
                match row.editor {
                    RecordEditor::Real => Some(qt_to_double(text).ok_or_else(|| bad_number(row, "숫자", text))?.to_string()),
                    RecordEditor::Integer => Some(qt_to_long_long(text).ok_or_else(|| bad_number(row, "정수", text))?.to_string()),
                    RecordEditor::Choice => Some(canonical_value(t, &row.field, text)),
                    RecordEditor::Lines | RecordEditor::Line => Some(text.to_owned()),
                }
            };
            out.push((row.field.clone(), value));
        }
        Ok(out)
    }
}

fn bad_number(row: &RecordRow, kind: &str, text: &str) -> RecordWarning {
    RecordWarning {
        title: "속성".to_owned(),
        text: format!("「{}」에 넣은 값이 {kind}가 아닙니다: {text}", row.label),
        detail: format!("이 칸은 {kind}만 받습니다. 아무 값도 바꾸지 않았습니다."),
        action: "숫자만 넣고(소수점은 .) 다시 저장하세요.".to_owned(),
    }
}

/// KaElevationFigure.cpp:18-20 metres — 소수 둘째 자리, 모르면 「—」.
fn metres(value: Option<f64>) -> String {
    match value {
        Some(v) if v.is_nan() => "nan".to_owned(),
        Some(v) => format!("{v:.2}"),
        None => "—".to_owned(),
    }
}

/// depthOf — 적은 깊이, 없으면 상면 − 바닥.
pub(crate) fn elevation_depth(top: &str, bottom: &str, depth: &str) -> Option<f64> {
    qt_to_double(depth).or_else(|| Some(qt_to_double(top)? - qt_to_double(bottom)?))
}

/// 표고 그림의 읽기 글(setLevels → accessibleName).
pub(crate) fn elevation_accessible_name(top: &str, bottom: &str, depth: &str) -> String {
    format!(
        "상면 표고 {}, 바닥 표고 {}, 깊이 {} 미터",
        metres(qt_to_double(top)),
        metres(qt_to_double(bottom)),
        metres(elevation_depth(top, bottom, depth))
    )
}

/// KaElevationFigure::level — 「EL. 152.40」.
pub(crate) fn elevation_level(text: &str) -> String {
    format!("EL. {}", metres(qt_to_double(text)))
}

#[cfg(test)]
#[path = "feature_record_tests.rs"]
pub(crate) mod tests;
