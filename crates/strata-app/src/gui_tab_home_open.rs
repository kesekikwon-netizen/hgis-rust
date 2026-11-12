//! 「열기」 — C++ `MainWindow::openProject`(`MainWindowSession.cpp:1634-1650`)와, 작업공간(.qgz·
//! 내장 프로젝트) 없이 조사 데이터만 여는 `openSurveyGpkg` 길(:242-420, `loadSurveyLayers`
//! :762-790). 작업공간 복원(2D)·지도에 도형 그리기(지시 3)·열기 전 저장 확인은 아직 없다.

use std::path::Path;

use strata_storage::survey_gpkg_read::{read_survey_tables, validate_survey_for_open};
use strata_types::schema::{DomainLayerKey, LegendGroup};

use crate::app_state::{MainTab, StrataAppState};
use crate::left_layer_panel::{LayerTreeEntry, LeftLayerPanel};

impl StrataAppState {
    /// C++ `MainWindowSession.cpp:1634-1650` openProject 의 파일 선택(시작 폴더 없음).
    pub fn open_survey_file_dialog(&mut self) {
        let picked = ::rfd::FileDialog::new()
            .set_title("열기")
            .add_filter("조사", &["gpkg", "qgz", "qgs"])
            .add_filter("GeoPackage", &["gpkg"])
            .add_filter("QGIS", &["qgz", "qgs"])
            .pick_file();
        let Some(path) = picked else {
            // 사용자가 취소했다 — C++ 은 아무 것도 하지 않는다.
            return;
        };
        self.open_survey_gpkg(&path);
    }

    /// C++ openSurveyGpkg 의 조사 데이터만 여는 길. 열 수 없으면 알리고 지금 조사는 그대로 둔다(false).
    pub fn open_survey_gpkg(&mut self, path: &Path) -> bool {
        // C++ :1646-1650 — .qgz/.qgs 를 고르면 옆의 같은 이름 .gpkg 를 연다.
        let mut target = path.to_path_buf();
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gpkg")) {
            let companion = path.with_extension("gpkg");
            if !companion.exists() {
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」은(는) GeoPackage 조사 파일이 아니고 같은 이름의 .gpkg 도 없습니다.",
                    path.display()
                ));
                return false;
            }
            target = companion;
        }
        // C++ :246-251 — 바꾸기 전에 검사하고, 실패하면 「조사 열기 실패」만 알린다.
        let read = validate_survey_for_open(&target)
            .and_then(|()| read_survey_tables(&target).map_err(|e| e.to_string()));
        let tables = match read {
            Ok(t) => t,
            Err(msg) => {
                self.home_map.draw_guide_message = Some(format!("조사 열기 실패: {msg}"));
                return false;
            }
        };
        // C++ :389 kaSafeClearQgisProject — 이전 조사를 비운다. 작업 좌표계는 바꾸지 않는다(:770).
        let title = target.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        self.reset_survey_workspace(&title);
        // C++ 은 이 길에서 상태줄에 아무 말도 띄우지 않는다(빈 글 = 기본 안내도 없음).
        self.home_map.draw_guide_message = Some(String::new());
        // C++ :775 addNonEmptyDomainLayers — 도형이 있는 표만 올린다.
        for t in &tables {
            if !t.layer.features.is_empty() {
                add_saved_layer_on_top(&mut self.left_layers, &t.table, t.layer.features.len());
            }
        }
        self.survey_tables = tables;
        self.left_layers.apply_default_basemaps();
        self.survey_path = Some(target.clone());
        // C++ :400-401 applyStartupMap -> ensureStartupViewReady — 데이터 범위가 아니라 한반도 전체.
        // 지도 칸 크기는 다음 지도 화면에서 정해지므로 그때 한 번 더 맞춘다.
        self.home_map.zoom_to_korea(self.work_crs);
        self.home_map.korea_view_pending = true;
        // C++ :412-419 rememberSurvey · showMapWorkspace · rememberSurveyDir · markSurveySaved.
        self.remember_survey(&target, &strata_storage::recent_surveys::complete_base_name(&target.to_string_lossy()));
        self.active_tab = MainTab::Map;
        self.remember_survey_dir(&target);
        self.is_dirty = false;
        true
    }
}

/// C++ `LayerOps.cpp:1749-1759` kaSavedGpkgLayerTitle — 범례 이름, 저장 실패 문구에도 쓴다.
pub(crate) fn saved_gpkg_layer_title(table: &str) -> String {
    match table {
        "survey_area" => "조사구역",
        "feature_poly" => "유구 (면)",
        "feature_line" => "유구 (선)",
        "section_line" => "단면선",
        "control_points" => "기준점",
        "artifact_point" => "유물",
        "trial_trench" => "시굴격자",
        t if t.starts_with("user_poly_") => "면",
        t => t,
    }
    .to_owned()
}

/// C++ `LayerOps.cpp:1885-1913` — 도메인 표는 `ensure_domain_layer` 로, 그 밖의 도형 표는 표 이름
/// 그대로 조사 데이터에 올린다. QGIS 는 새 레이어를 트리 맨 위에 넣으므로(:1727-1738) 범례는
/// 표 순서의 거꾸로 보인다.
fn add_saved_layer_on_top(panel: &mut LeftLayerPanel, table: &str, count: usize) {
    if let Some(key) = DomainLayerKey::from_id(table) {
        panel.ensure_domain_layer(key);
    } else if !panel.entries.iter().any(|e| e.layer_key == table) {
        panel.entries.push(LayerTreeEntry {
            layer_key: table.to_owned(),
            label_ko: String::new(),
            group: LegendGroup::SurveyData,
            visible: true,
            opacity: 1.0,
            read_only: false,
            snap_enabled: true,
            feature_count: 0,
        });
    }
    if let Some(i) = panel.entries.iter().position(|e| e.layer_key == table) {
        let mut entry = panel.entries.remove(i);
        entry.label_ko = saved_gpkg_layer_title(table);
        entry.feature_count = count;
        panel.entries.insert(0, entry);
    }
}

#[cfg(test)]
#[path = "gui_tab_home_open_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gui_tab_home_open_view_tests.rs"]
mod view_tests;
