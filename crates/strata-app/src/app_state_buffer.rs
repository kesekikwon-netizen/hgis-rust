//! 주변유적경계 500m·1000m — C++ `runSiteBuffer500/1000`(MainWindowRibbon.cpp:816-839), 쓸 면 레이어 고르기
//! `NearbyHeritageExport::resolveSourceLayer`(NearbyHeritageCollect.cpp:127-142), 경계 만들기
//! `BufferAnalysis::addDistanceRing`(BufferAnalysis.cpp). 도형 셈은 strata-engine(GEOS), 좌표계는 PROJ.
//! 면은 열어 둔 조사 표의 편집 버퍼(저장 안 한 것 포함)에서 읽는다. 저장할 때 조사 파일에 굽는 것은 아직 없다.

use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::real_survey::DistanceRing;

fn is_polygon_table(t: &SurveyFileTable) -> bool {
    t.geometry_type.to_ascii_uppercase().contains("POLYGON")
}

impl StrataAppState {
    /// resolveSourceLayer — 고른 범례 레이어가 쓸 수 있는 면 레이어면 그것, 아니면 보이는 조사구역, 아니면 처음 쓸 수 있는 조사구역.
    fn buffer_source(&self) -> Option<&SurveyFileTable> {
        let usable = |e: &LayerTreeEntry| -> Option<&SurveyFileTable> {
            if e.group != LegendGroup::SurveyData || e.layer_key.starts_with("user:buffer_") {
                return None;
            }
            self.survey_tables.iter().find(|t| t.table == e.layer_key && is_polygon_table(t) && !t.layer.features.is_empty())
        };
        let current = self.right_inspector.selected_layer_key.as_deref();
        if let Some(t) = self.left_layers.entries.iter().filter(|e| Some(e.label_ko.as_str()) == current).find_map(usable) {
            return Some(t);
        }
        let areas: Vec<&LayerTreeEntry> = self.left_layers.entries.iter().filter(|e| e.layer_key == "survey_area").collect();
        areas
            .iter()
            .filter(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko))
            .find_map(|e| usable(e))
            .or_else(|| areas.iter().find_map(|e| usable(e)))
    }

    /// 면들을 작업 좌표계 WKT 로(unionInCrs 의 옮기기). 옮기지 못한 도형은 뺀다.
    fn work_wkts(&self, t: &SurveyFileTable) -> Vec<String> {
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let tr = u32::try_from(t.srs_id).ok().filter(|s| *s > 0 && *s != work).and_then(|s| strata_engine::Transformer::new(s, work).ok());
        t.layer
            .features
            .iter()
            .filter_map(|f| {
                let parts = wkt_to_parts(&f.wkt_geom).ok().filter(|p| !p.is_empty())?;
                let parts = match &tr {
                    Some(tr) => crate::checklist_state_geometry::moved(&parts, tr)?,
                    None => parts,
                };
                Some(crate::checklist_state_geometry::parts_wkt(&parts))
            })
            .collect()
    }

    /// `addDistanceRing` — 경계를 만들고 범례 맨 위에 「주변 Nm」·「주변 Nm 거리」를 (다시) 올린다. 실패하면 까닭.
    pub(crate) fn add_distance_ring(&mut self, meters: u32) -> Result<(), &'static str> {
        let Some(src) = self.buffer_source() else { return Err("면 레이어를 먼저 선택하세요") };
        let wkts = self.work_wkts(src);
        if wkts.is_empty() {
            return Err("선택한 레이어에 도형이 없습니다");
        }
        let lines = strata_engine::distance_rings(&wkts, f64::from(meters)).map_err(|_| "주변 범위를 만들지 못했습니다")?;
        if lines.is_empty() {
            return Err("범위가 너무 작거나 외곽선을 얻지 못했습니다");
        }
        let labels: Vec<[f64; 2]> = lines.iter().filter_map(|l| strata_engine::ring_label_points(l).ok()).flatten().collect();
        if labels.is_empty() {
            return Err("동서남북 거리 글자를 넣지 못했습니다");
        }
        self.home_map.real_survey.distance_rings.insert(meters, DistanceRing { lines, labels });
        let title = format!("주변 {meters}m");
        for (key, label) in [(format!("user:buffer_ring_{meters}"), title.clone()), (format!("user:buffer_label_{meters}"), format!("{title} 거리"))] {
            self.left_layers.entries.retain(|e| e.layer_key != key);
            self.home_map.real_survey.layer_visible.insert(label.clone(), true);
            self.left_layers.entries.insert(
                0,
                LayerTreeEntry {
                    layer_key: key,
                    label_ko: label,
                    group: LegendGroup::SurveyData,
                    visible: true,
                    opacity: 1.0,
                    read_only: false,
                    snap_enabled: true,
                    feature_count: 0,
                },
            );
        }
        Ok(())
    }

    /// 버퍼 줄 「500m」·「1000m」 — runSiteBuffer500/1000. 주변유적 수는 유산 자료가 진짜가 되면 붙인다(C++ 은 그때만 센다).
    pub(crate) fn run_site_buffer(&mut self, meters: u32) {
        match self.add_distance_ring(meters) {
            Ok(()) => self.show_status_message(format!("주변 {meters}m 경계를 그렸습니다")),
            // C++ 은 알림 창(「주변유적경계」)이다 — 창이 생기기 전까지 상태줄에.
            Err(why) => self.show_status_message(format!("{meters}m 경계를 그리지 못했습니다. {why}")),
        }
    }
}

#[cfg(test)]
#[path = "app_state_buffer_tests.rs"]
mod tests;
