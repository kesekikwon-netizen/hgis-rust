//! `SurveyDoc`(조사 파일의 7개 도메인 레이어)와 화면 상태를 잇는다.
//!
//! C++ 은 `QgsVectorLayer` 하나가 파일·편집 버퍼·화면을 모두 맡는다. Rust 판은
//! 화면이 아직 `RealSurveyData`(조사구역 링·시굴격자)를 직접 그리므로, 열 때는
//! 문서 → 화면, 저장할 때는 화면 → 문서로 그 둘만 맞춘다. 유구·단면선·기준점·유물은
//! 문서가 유일한 원본이고 `gui_tab_map_domain` 이 문서에서 바로 그린다.

use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::survey_doc::{FieldValue, SurveyDoc, SurveyFeature};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::real_survey::{ring_area_m2, ring_centroid, RealTrenchPoly};

/// 열린 고리(첫 점 ≠ 끝 점)를 닫는다. GPKG 폴리곤 고리는 닫혀 있어야 한다.
fn closed_ring(mut ring: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if let (Some(first), Some(last)) = (ring.first().copied(), ring.last().copied()) {
        if first != last {
            ring.push(first);
        }
    }
    ring
}

fn exterior(feature: &SurveyFeature) -> Option<&Vec<[f64; 2]>> {
    match feature.geom.as_ref()? {
        WkbGeometry::Polygon(rings) => rings.first(),
        _ => None,
    }
}

/// 화면의 고리 목록을 문서 레이어에 맞춘다. 순서가 같은 도형은 fid·속성을 지키고,
/// 바깥 고리가 달라졌을 때만 지오메트리를 바꾼다(안 고친 도형은 원본 바이트 그대로 저장된다).
fn reconcile_polygons(doc: &mut SurveyDoc, key: DomainLayerKey, rings: Vec<Vec<[f64; 2]>>) {
    let layer = doc.layer_mut(key);
    layer.features.truncate(rings.len());
    for (i, ring) in rings.into_iter().enumerate() {
        let ring = closed_ring(ring);
        match layer.features.get_mut(i) {
            Some(feature) => {
                if exterior(feature) != Some(&ring) {
                    feature.set_geom(WkbGeometry::Polygon(vec![ring]));
                }
            }
            None => layer
                .features
                .push(SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![ring]))),
        }
    }
}

impl StrataAppState {
    /// 문서를 통째로 바꿔 끼우고 화면(조사구역·시굴격자·제목·중심)을 그에 맞춘다.
    pub(crate) fn install_survey_doc(&mut self, doc: SurveyDoc, path: &std::path::Path) {
        self.work_crs = if doc.srs_id == 5186 {
            WorkCrs::Epsg5186
        } else {
            WorkCrs::Epsg5187
        };
        let rs = &mut self.home_map.real_survey;
        rs.gpkg_path = path.to_path_buf();
        if let Some(dir) = path.parent() {
            rs.survey_dir = dir.to_path_buf();
        }
        self.survey_doc = doc;
        self.refresh_view_from_doc();
    }

    /// 문서가 바뀐 뒤: 화면이 쓰는 조사구역 고리·시굴격자·넓이·중심을 문서에서 다시 만든다.
    pub(crate) fn refresh_view_from_doc(&mut self) {
        let doc = &self.survey_doc;
        let rs = &mut self.home_map.real_survey;
        rs.survey_rings = doc
            .layer(DomainLayerKey::SurveyArea)
            .features
            .iter()
            .filter_map(|f| exterior(f).cloned())
            .collect();
        rs.trenches = doc
            .layer(DomainLayerKey::TrialTrench)
            .features
            .iter()
            .filter_map(|f| {
                let ring = exterior(f)?.clone();
                Some(RealTrenchPoly {
                    name: f.text("name"),
                    area_m2: ring_area_m2(&ring),
                    center: ring_centroid(&ring),
                    ring,
                })
            })
            .collect();
        if let Some(first) = doc.layer(DomainLayerKey::SurveyArea).features.first() {
            rs.survey_name = first.text("survey_name");
            rs.site_name = first.text("site_name");
        }
        rs.survey_area_m2 = rs.survey_rings.iter().map(|r| ring_area_m2(r)).sum();
        if let Some(ring) = rs.survey_rings.first() {
            rs.survey_center = ring_centroid(ring);
        } else if let Some([min_x, max_x, min_y, max_y]) = doc.extent() {
            rs.survey_center = [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5];
        }
    }

    /// 방금 열거나 저장한 조사를 최근 목록 맨 위에 올리고 홈 표를 다시 만든다.
    pub(crate) fn remember_recent_survey(&mut self, path: &std::path::Path) {
        crate::recent_surveys::remember_survey(&mut self.home_map.recent_list, path);
        self.home_map.recent_rows =
            crate::recent_surveys::build_recent_rows(&self.home_map.recent_list);
    }

    /// 홈 표의 한 줄을 눌렀을 때: 파일이 있으면 열고, 없으면 그렇다고 알린다.
    pub(crate) fn open_recent_survey_row(&mut self, idx: usize) {
        let Some(row) = self.home_map.recent_rows.get(idx).cloned() else {
            return;
        };
        if row.exists {
            self.open_survey_gpkg(&row.path);
        } else {
            self.home_map.draw_guide_message = Some(format!(
                "「{}」 조사 파일을 찾을 수 없습니다: {}",
                row.name,
                row.path.display()
            ));
        }
    }

    /// 저장 직전: 화면에서 고친 조사구역·시굴격자를 문서에 되적는다.
    pub(crate) fn sync_view_into_survey_doc(&mut self) {
        // 카드에서 고치던 값이 있으면 먼저 그 도형에 적는다.
        self.sync_card_into_selected_feature();
        self.survey_doc.srs_id = self.work_crs.epsg_code() as i32;
        let rs = &self.home_map.real_survey;
        let area_rings = rs.survey_rings.clone();
        let trench_rings: Vec<Vec<[f64; 2]>> = rs.trenches.iter().map(|t| t.ring.clone()).collect();
        let trench_names: Vec<String> = rs.trenches.iter().map(|t| t.name.clone()).collect();
        reconcile_polygons(&mut self.survey_doc, DomainLayerKey::SurveyArea, area_rings);
        reconcile_polygons(&mut self.survey_doc, DomainLayerKey::TrialTrench, trench_rings);
        let trenches = &mut self.survey_doc.layer_mut(DomainLayerKey::TrialTrench).features;
        for (feature, name) in trenches.iter_mut().zip(trench_names) {
            if feature.text("name") != name {
                feature.set_text("name", &name);
            }
        }
        let title = self.project_title.clone();
        for feature in &mut self.survey_doc.layer_mut(DomainLayerKey::SurveyArea).features {
            if feature.text("survey_name").is_empty() {
                feature.set_text("survey_name", &title);
            }
        }
    }

    /// 그리기 도구가 마친 도형을 지금 고른 대상 레이어에 넣는다. 넣었으면 그 레이어 이름을 돌려준다.
    ///
    /// C++ `MainWindowEditing.cpp:782` `onGeometryCaptured` → `addFeature`: 대상 레이어는
    /// 누른 그리기 단추(조사구역 / 유구 / 유구 선 / 유물)가 정한다.
    pub(crate) fn commit_drawn_shape(&mut self, tool: MapToolKind, points: Vec<[f64; 2]>) -> Option<&'static str> {
        let (key, geom) = match tool {
            MapToolKind::DrawPolygon if points.len() >= 3 => {
                let key = if self.draw_target == DomainLayerKey::SurveyArea {
                    DomainLayerKey::SurveyArea
                } else {
                    DomainLayerKey::FeaturePoly
                };
                (key, WkbGeometry::Polygon(vec![closed_ring(points)]))
            }
            MapToolKind::DrawLine if points.len() >= 2 => {
                let key = if self.draw_target == DomainLayerKey::SectionLine {
                    DomainLayerKey::SectionLine
                } else {
                    DomainLayerKey::FeatureLine
                };
                (key, WkbGeometry::LineString(points))
            }
            MapToolKind::PlacePoint if !points.is_empty() => {
                (DomainLayerKey::ArtifactPoint, WkbGeometry::Point(points[0]))
            }
            _ => return None,
        };
        // 그리기도 되돌릴 수 있게 그리기 전 문서를 기록에 쌓는다.
        self.push_undo();
        let mut feature = SurveyFeature::new_drawn(geom);
        if key == DomainLayerKey::ArtifactPoint {
            let n = self.survey_doc.layer(key).features.len() + 1;
            feature
                .attrs
                .insert("artifact_no".to_owned(), FieldValue::Text(n.to_string()));
        }
        self.survey_doc.add_feature(key, feature);
        // 화면이 쓰는 조사구역·시굴격자 고리를 문서에서 다시 만든다.
        self.refresh_view_from_doc();
        self.left_layers.ensure_domain_layer(key);
        self.is_dirty = true;
        Some(key.name_ko())
    }
}

#[cfg(test)]
#[path = "survey_doc_bridge_tests.rs"]
mod tests;
