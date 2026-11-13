//! 검수 상태 — C++ `ProjectStateBuilder::empty`·`fromProject`(core/ProjectStateBuilder.cpp)와 `Offenders`
//! (ChecklistStateGeometry.cpp:61-90). 규칙 판정은 같은 규칙 파일을 읽는 `ChecklistEngine` 이 한다.
//!
//! 「프로젝트의 조사 레이어」는 범례에 있는 조사 표(그 표 이름 = layer_key)이고, 도형·속성은 열어 둔 표의 편집 버퍼
//! (저장 안 한 것 포함 — QGIS 도 편집 버퍼를 읽는다)다. 도면 판정은 조판한 용지(user_sheet 등)를 보는데, Rust 에는
//! 아직 조판 용지가 없어 C++ 에서 용지가 없을 때와 같다(유적위치도·유구배치도·개별유구실측도 없음, 용지 검사 통과).

use std::collections::HashMap;

use serde_json::{json, Map, Value};
use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::app_state::StrataAppState;

/// `LayerOps::domainLayerKeys` 차례.
pub(crate) const DOMAIN_KEYS: [&str; 7] =
    ["survey_area", "feature_poly", "feature_line", "section_line", "control_points", "artifact_point", "trial_trench"];
/// `kMaxOffendersPerKey`.
const MAX_OFFENDERS: usize = 200;

/// 범례에 있는 조사 표 하나(C++ 프로젝트 레이어).
pub(crate) struct DomainLayer<'a> {
    pub(crate) table: &'a SurveyFileTable,
    pub(crate) name: String,
}

/// `domainLayersForKey(project, key)`.
pub(crate) fn layers_for<'a>(layers: &'a [DomainLayer<'a>], key: &str) -> impl Iterator<Item = &'a DomainLayer<'a>> {
    let key = key.to_owned();
    layers.iter().filter(move |l| l.table.table == key)
}

/// C++ `Offenders` — 키마다 걸린 도형(최대 200개)과 설명.
#[derive(Default)]
pub(crate) struct Offenders {
    items: HashMap<String, Vec<Value>>,
    counts: HashMap<String, usize>,
    notes: HashMap<String, String>,
}

impl Offenders {
    pub(crate) fn add(&mut self, key: &str, layer: &DomainLayer<'_>, fid: i64, label: &str) {
        let n = self.counts.entry(key.to_owned()).or_insert(0);
        *n += 1;
        if *n > MAX_OFFENDERS {
            return;
        }
        let item = json!({"layer_id": layer.table.table, "layer_name": layer.name, "fid": fid, "label": label});
        self.items.entry(key.to_owned()).or_default().push(item);
    }

    pub(crate) fn note(&mut self, key: &str, text: String) {
        self.notes.insert(key.to_owned(), text);
    }

    fn write_to(self, st: &mut Map<String, Value>) {
        let mut notes: Map<String, Value> = self.notes.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
        for (k, n) in &self.counts {
            if *n > MAX_OFFENDERS && !notes.contains_key(k) {
                notes.insert(k.clone(), Value::String(format!("{n}건 중 앞 {MAX_OFFENDERS}건만 목록에 보입니다.")));
            }
        }
        st.insert("offenders".to_owned(), Value::Object(self.items.into_iter().map(|(k, v)| (k, Value::Array(v))).collect()));
        st.insert("notes".to_owned(), Value::Object(notes));
    }
}

/// `featureLabel` — point_id·feature_no·section_id·artifact_no·survey_name·name 가운데 처음 찬 값, 없으면 「#fid」.
pub(crate) fn feature_label(f: &strata_storage::edit_buffer::FeatureRecord) -> String {
    ["point_id", "feature_no", "section_id", "artifact_no", "survey_name", "name"]
        .iter()
        .find_map(|k| attr(f, k).filter(|v| !v.is_empty()))
        .unwrap_or_else(|| format!("#{}", f.fid))
}

/// 칸 값(앞뒤 공백 뺌). 칸이 없거나 NULL 이면 None — QgsFields::lookupField 처럼 대소문자는 가리지 않는다.
pub(crate) fn attr(f: &strata_storage::edit_buffer::FeatureRecord, field: &str) -> Option<String> {
    f.attributes.iter().find(|(k, _)| k.eq_ignore_ascii_case(field)).map(|(_, v)| v.trim().to_owned())
}

/// `ProjectStateBuilder::empty`.
fn empty() -> Map<String, Value> {
    let v = json!({
        "survey_area_count": 0, "control_points_count": 0, "feature_poly_count": 0, "feature_line_count": 0,
        "project_crs_set": false, "project_crs_authid": "",
        "has_datum": false, "has_ellipsoid": false, "has_projection": false, "has_origin": false, "has_accuracy": false,
        "has_kind_period": true, "has_abstract_marker": false, "survey_is_polygon": false,
        "features_within_survey": true, "features_real_geometry": true, "geometries_valid": true,
        "geometries_nonempty": true, "geometries_nonzero_area": true, "export_crs_valid": true,
        "required_fields_filled": true, "control_points_point_id_unique": true, "feature_poly_feature_no_unique": true,
        "sheet_covers_targets": true, "sheet_has_elements": true, "HERITAGE_PLEDGE_SCOPE": true,
        "layout_exists:site_location": false, "layout_exists:feature_plan": false,
        "layout_exists:feature_detail": false, "layout_exists:survey_area_map": false,
    });
    v.as_object().cloned().unwrap_or_default()
}

/// `ProjectStateBuilder::fromProject` — 조사 표(범례에 있는 것), 작업 좌표계, 국가유산 인트라넷 자료 여부로.
pub(crate) fn build_checklist_state(layers: &[DomainLayer<'_>], work_epsg: u32, heritage_ok: bool) -> Value {
    let mut st = empty();
    let mut off = Offenders::default();
    st.insert("HERITAGE_PLEDGE_SCOPE".to_owned(), json!(heritage_ok));
    st.insert("project_crs_set".to_owned(), json!(true));
    st.insert("project_crs_authid".to_owned(), json!(format!("EPSG:{work_epsg}")));
    off.note("project_crs_authid", format!("지금 EPSG:{work_epsg}"));
    let total = |key: &str| layers_for(layers, key).map(|l| l.table.layer.features.len()).sum::<usize>();
    for key in ["survey_area", "feature_poly", "feature_line", "control_points"] {
        st.insert(format!("{key}_count"), json!(total(key)));
    }
    crate::checklist_state_geometry::scan_geometry(layers, work_epsg, &mut st, &mut off);
    crate::checklist_state_fields::scan_fields(layers, &mut st, &mut off);
    let mut unique_numbers = true;
    for layer in layers_for(layers, "feature_poly") {
        for (fid, label) in crate::checklist_state_fields::feature_number_duplicates(layer.table) {
            unique_numbers = false;
            off.add("feature_poly_feature_no_unique", layer, fid, &label);
        }
    }
    st.insert("feature_poly_feature_no_unique".to_owned(), json!(unique_numbers));
    // 도면 판정(layout_exists:*·sheet_*)은 조판한 용지가 있을 때만 바뀐다 — 아직 없으므로 empty() 값 그대로.
    off.write_to(&mut st);
    Value::Object(st)
}

impl StrataAppState {
    /// 「검수·제출」이 판정할 상태(C++ ProjectStateBuilder::fromProject(QgsProject::instance())).
    pub(crate) fn checklist_state(&self) -> Value {
        let layers: Vec<DomainLayer<'_>> = DOMAIN_KEYS
            .iter()
            .flat_map(|key| {
                self.left_layers.entries.iter().filter(move |e| e.layer_key == *key).filter_map(|e| {
                    let table = self.survey_tables.iter().find(|t| t.table == e.layer_key)?;
                    Some(DomainLayer { table, name: e.label_ko.clone() })
                })
            })
            .collect();
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        build_checklist_state(&layers, work, self.home_map.real_survey.heritage_polys.is_empty())
    }
}

#[cfg(test)]
#[path = "checklist_state_tests.rs"]
mod tests;
