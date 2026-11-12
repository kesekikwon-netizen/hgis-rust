//! 정합 동작 — 이동 적용·원본 복원·저장·세션 시작/종료·보조툴바 숨김·겹침 토글.


use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_align::AlignSessionState;

impl StrataAppState {
    /// C++ `MainWindow::applyAlignMove()` — `MainWindowAlign.cpp:627-703`.
    /// 점이 2개 미만이면 C++ `KaAlignMapTool::applyMove` 문구(`KaAlignMapTool.cpp:346-348`)로 거절한다.
    pub(crate) fn apply_align_move(&mut self) -> Result<(), String> {
        if self.align.pairs.len() < 2 {
            return Err("점을 2개 이상 찍은 뒤 「이동」을 누르세요".to_owned());
        }
        let src: Vec<[f64; 2]> = self.align.pairs.iter().map(|p| p.src).collect();
        let dst: Vec<[f64; 2]> = self.align.pairs.iter().map(|p| p.map).collect();
        let transform = strata_geo::polygon_ops::solve_helmert_georef(&src, &dst)
            .map_err(|e| e.to_string())?;
        self.align.transform = Some(transform);
        self.align.applied = true;
        self.is_dirty = true;
        // :695-701 — 좌표 파일을 새로 썼다는 사실과 백업 위치를 함께 알린다.
        // Rust 는 아직 세계파일을 쓰지 않으므로(:694 `backup` 이 빈 경우의 C++ 분기와 같다)
        // 뒷문장 없이 앞문장만 남긴다.
        Ok(())
    }

    /// C++ `KaAlignMapTool::restoreOriginals()` — `KaAlignMapToolEdit.cpp:124-168`.
    /// 돌려주는 값은 (성공 여부, C++ 원문 메시지).
    pub(crate) fn restore_align_originals(&mut self) -> (bool, String) {
        // C++ 은 세션 레이어가 없으면 `되돌릴 맞추기가 없습니다`(:132-133)
        if self.align.target_layer.is_none() {
            return (true, "되돌릴 맞추기가 없습니다".to_owned());
        }
        let applied = self.align.applied;
        self.align.pairs.clear();
        self.align.pending_source = None;
        self.align.transform = None;
        self.align.applied = false;
        if !applied {
            // :147-148 — 아직 세계파일에 쓴 것이 없으면 점만 지운다
            return (
                true,
                "찍은 점을 지웠습니다. 그림 파일은 아직 바뀌지 않았습니다".to_owned(),
            );
        }
        self.is_dirty = true;
        (true, "도면을 맞추기 전 자리로 되돌렸습니다".to_owned())
    }

    /// C++ `MainWindowAlign.cpp:745-792` 「맞추기 저장」.
    /// 돌려주는 값은 (알림 문구, 상태줄 문구).
    pub(crate) fn save_align_result(&mut self) -> (String, String) {
        // :748-752 — 점이 2개 미만이면 저장하지 않고 안내만 한다
        if self.align.pairs.len() < 2 && !self.align.applied {
            return (
                "맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요.".to_owned(),
                "점을 2곳 이상 찍은 뒤 저장하세요.".to_owned(),
            );
        }
        if !self.align.applied && self.apply_align_move().is_err() {
            return (
                "맞추기 — 도면을 옮기지 못했습니다".to_owned(),
                self.align.status_text(),
            );
        }
        // :763 — 저장하며 도면 세션은 끝난다
        self.stop_align_session();
        self.is_dirty = true;
        let path = self.align.saved_path.clone();
        (
            "맞춰 두었습니다. 이 도면은 참고용이니 제출할 구역은 그리기로 직접 그리세요.".to_owned(),
            match path {
                // :791 `맞춤 저장: %1`
                Some(p) => format!("맞춤 저장: {p}"),
                None => self.align.status_text(),
            },
        )
    }

    /// C++ `MainWindow::stopAlignSession()` — `MainWindowAlign.cpp:705-714`.
    /// 도구를 놓고 이동으로 돌아간 뒤 세션을 끝낸다.
    pub(crate) fn stop_align_session(&mut self) {
        self.align.end_session();
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
        if self.active_tool == MapToolKind::GeorefAlign {
            self.active_tool = MapToolKind::Pan;
        }
    }

    /// C++ `GeorefService::isAlignableLayer` — `GeorefService.cpp:502-512`.
    /// 도메인 조사 레이어(`:495-500`)는 맞출 수 없고, GDAL 래스터와 OGR·메모리 벡터만 된다.
    /// 레이어 목록에서 그런 첫 레이어의 표시 이름을 고른다(C++ 은 현재 레이어를 쓴다).
    pub(crate) fn alignable_layer_name(&self) -> Option<String> {
        let current = self.right_inspector.selected_layer_key.as_deref();
        let is_alignable = |key: &str| {
            !matches!(
                key,
                "survey_area"
                    | "feature_poly"
                    | "feature_line"
                    | "section_line"
                    | "control_points"
                    | "artifact_point"
                    | "trial_trench"
            ) && !key.starts_with("vworld_")
        };
        if let Some(key) = current.filter(|k| is_alignable(k)) {
            return self
                .left_layers
                .entries
                .iter()
                .find(|e| e.layer_key == key)
                .map(|e| e.label_ko.clone())
                .or_else(|| Some(key.to_owned()));
        }
        self.left_layers
            .entries
            .iter()
            .find(|e| is_alignable(&e.layer_key))
            .map(|e| e.label_ko.clone())
    }

    /// C++ `MainWindow::startAlignSession(QgsMapLayer*)` — `MainWindowAlign.cpp:799-838`.
    pub(crate) fn start_align_session(&mut self, target_layer: &str) {
        self.align = AlignSessionState {
            target_layer: Some(target_layer.to_owned()),
            ..AlignSessionState::default()
        };
        // :831-832 showAlignSplit + showSubToolsAlign
        self.ribbon.toggle_sub_mode(SubToolsMode::Align);
        self.active_tool = MapToolKind::GeorefAlign;
        // :838 — 세션을 열면 상태줄에 statusText() 를 띄운다
        self.status_bar_hint = self.align.status_text();
    }

    /// C++ `MainWindow::hideSubTools()` — `MainWindowRibbon.cpp:769-777`.
    /// 정합 모드에서 도구 줄을 닫으면 정합 세션도 함께 끝난다. 그리기 줄이면 stopCaptureTool(:773-774).
    pub(crate) fn hide_sub_tools(&mut self) {
        if self.ribbon.sub_mode == SubToolsMode::Align {
            self.stop_align_session();
            return;
        }
        if self.ribbon.sub_mode == SubToolsMode::Draw {
            self.stop_capture_tool();
        }
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
    }

    /// C++ `MainWindowOverlay.cpp:225-240` `toggleExistingOverlay` 그대로.
    /// 레이어가 이미 있으면 켜고 끄기만 하며 `…를 껐습니다.` / `…를 다시 켰습니다.` 를
    /// 남기고 `true`. 없으면 `false` 라 호출한 쪽이 처음 받아 오는 길로 간다.
    /// (`layer_visible` 에 키가 있는지가 C++ 의 `layersMatchingBaseName` 비어 있지 않음에 해당)
    pub(crate) fn toggle_existing_overlay(&mut self, title: &str) -> bool {
        if !self
            .home_map
            .real_survey
            .layer_visible
            .contains_key(title)
        {
            return false;
        }
        let now_on = !self.home_map.real_survey.is_layer_visible(title);
        self.home_map
            .real_survey
            .layer_visible
            .insert(title.to_owned(), now_on);
        self.status_bar_hint = if now_on {
            format!("{title}를 다시 켰습니다.")
        } else {
            format!("{title}를 껐습니다.")
        };
        true
    }
}
