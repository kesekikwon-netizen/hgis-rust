//! 리본 단추 클릭 처리(앞부분) — 라벨별 동작.
//!
//! 원래 `handle_ribbon_click` 하나이던 match 를 둘로 나눴다. 라벨 집합이 서로
//! 겹치지 않으므로 두 match 를 차례로 실행해도 결과는 원문과 같다(각 match 는
//! 자기 라벨에서만 걸리고 나머지 팔은 지나간다). 팔 순서는 원문 그대로다.


use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_draw::DEM_LEGEND_KEY;

impl StrataAppState {
    /// 라벨별 동작 — 앞쪽 라벨이면 `_a` 만, 아니면 `_b` 만 실행한다.
    ///
    /// 원문은 match 하나였다. 둘로 나눴으므로 **한쪽만** 실행해야 한다:
    /// `_b` 의 마지막 팔 `_ => self.active_tab = MainTab::Map` 이 catch-all 이라
    /// 둘 다 부르면 `_a` 라벨에서도 지도 탭으로 되돌아가 버린다.
    pub(crate) fn handle_ribbon_click(&mut self, label: &str) {
        if matches!(
            label,
            "새 조사"
                | "열기"
                | "저장"
                | "다른 이름"
                | "선택"
                | "측거"
                | "그리기"
                | "시굴격자"
                | "버퍼"
                | "DEM"
                | "지적"
                | "수치지형"
                | "유산"
        ) {
            self.handle_ribbon_click_a(label);
        } else {
            self.handle_ribbon_click_b(label);
        }
    }

    /// 앞쪽 라벨들.
    fn handle_ribbon_click_a(&mut self, label: &str) {
        match label {
            "새 조사" => self.modals.active = Some(ActiveModalDialog::NewSurvey),
            // C++ MainWindowSession.cpp:1634 openProject — 파일 대화상자로 조사 파일을 고른다
            "열기" => self.open_survey_file_dialog(),
            // C++ MainWindowSession.cpp:1409 saveProject / :1420 saveProjectAs
            "저장" => {
                // C++ `MainWindowSession.cpp:1409-1418` saveProject —
                // 조사 파일이 없으면 `saveProjectAs()` 로 「다른 이름으로 저장」
                // 대화상자를 연다. 그 동작을 그대로 둔다(사용자가 경로를 고른다).
                let _ = self.save_survey_project();
            }
            "다른 이름" => {
                let _ = self.save_survey_project_as();
            }
            // C++ MainWindowEditing.cpp:182 startSelectTool — 같은 도구를 다시 누르면
            // 이동(Pan)으로 돌아가고 상태줄에 「도형 선택 종료」를 띄운다.
            "선택" => {
                self.active_tab = MainTab::Map;
                if self.active_tool == MapToolKind::Select {
                    self.active_tool = MapToolKind::Pan;
                    self.status_bar_hint = "도형 선택 종료".to_owned();
                } else {
                    self.active_tool = MapToolKind::Select;
                    self.status_bar_hint = "도형선택 — 도형을 클릭하면 점이 나옵니다. 점을 끌면 옮기고(Ctrl=자석), 점을 찍은 뒤 Delete로 지웁니다. 같은 자리를 다시 누르면 겹친 도형을 고릅니다.".to_owned();
                }
            }
            // C++ MainWindowEditing.cpp:242 startMeasureTool — 토글 + 두 상태줄 문구
            "측거" => {
                if self.active_tool == MapToolKind::Measure {
                    self.active_tool = MapToolKind::Pan;
                    self.status_bar_hint = "줄자 종료".to_owned();
                } else {
                    self.active_tool = MapToolKind::Measure;
                    self.active_tab = MainTab::Map;
                    self.status_bar_hint =
                        "줄자: 점을 찍고 우클릭에서 마침을 고르세요. 면적은 면적만 나옵니다."
                            .to_owned();
                }
            }
            // C++ MainWindowRibbon.cpp:345 — 다시 누르면 도구를 닫고 이동으로 돌아간다
            "그리기" => self.toggle_draw_sub_tools(),
            // C++ MainWindow.cpp:1834 startTrenchGrid — 조사가 없으면 먼저 안내하고 열지 않는다
            "시굴격자" => {
                if self.survey_path.is_none() {
                    self.home_map.draw_guide_message = Some(
                        "먼저 「새 조사」를 만들거나 「열기」로 조사를 여세요.".to_owned(),
                    );
                } else {
                    self.modals.active = Some(ActiveModalDialog::TrenchGenerator);
                }
            }
            "버퍼" => {
                self.active_tab = MainTab::Map;
                self.ribbon.toggle_sub_mode(SubToolsMode::Buffer);
            }
            // C++ MainWindowOverlay.cpp:303-348 toggleDemMap — DEM 겹침이 이미 있으면
            // 보이기/숨기기만 하고(:315-340), 없을 때만 받으러 간다(:344 startDemDownload).
            // C++ 은 :315 에서 이름 "DEM" 으로 묻지만 이것은 BasemapDsm.h:44-57 이
            // ka_hgis/reference_kind=dem 태그로 찾는 것과 같은 레이어이고, 그 범례 이름은
            // BasemapDsm.h:21 `지표모델(DSM) 30m · Copernicus` 다. Rust 범례는 이 이름을
            // `지표모델(DSM) 30m · Coperni...` 로 줄여 쓰므로 두 키를 같은 값으로 맞춘다.
            "DEM" => {
                self.active_tab = MainTab::Map;
                let present = self
                    .home_map
                    .real_survey
                    .layer_visible
                    .contains_key("DEM")
                    || self
                        .home_map
                        .real_survey
                        .layer_visible
                        .contains_key(DEM_LEGEND_KEY);
                if present {
                    let make_visible = !self.home_map.real_survey.is_layer_visible("DEM");
                    for key in ["DEM", DEM_LEGEND_KEY] {
                        self.home_map
                            .real_survey
                            .layer_visible
                            .insert(key.to_owned(), make_visible);
                    }
                    // MainWindowOverlay.cpp:330-332 — DEM 을 끄는데 지질도가 꺼져 있으면
                    // 같이 올라와 있던 「지형 음영」 도 내린다. C++ `LayerOps::isLayerVisible`
                    // 은 레이어가 없으면 false 이므로(BasemapOps.cpp:2358-2359) Rust 기본값
                    // (없으면 true)을 그대로 쓰지 않고 있는지까지 본다.
                    let geology_on = self
                        .home_map
                        .real_survey
                        .layer_visible
                        .contains_key("지질도(KIGAM 1:5만)")
                        && self
                            .home_map
                            .real_survey
                            .is_layer_visible("지질도(KIGAM 1:5만)");
                    if !make_visible && !geology_on {
                        self.home_map
                            .real_survey
                            .layer_visible
                            .insert("지형 음영".to_owned(), false);
                    }
                    // MainWindowOverlay.cpp:337-340 문구 그대로
                    self.status_bar_hint = if make_visible {
                        "DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요.".to_owned()
                    } else {
                        "DEM 지형을 숨겼습니다.".to_owned()
                    };
                } else {
                    // :344 startDemDownload
                    self.modals.active = Some(ActiveModalDialog::DemDownload);
                }
            }
            "지적" => {
                self.active_tab = MainTab::Map;
                self.left_layers.add_cadastral_wms_layer();
                self.modals.active = Some(ActiveModalDialog::CadastralDownload);
            }
            "수치지형" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
            "유산" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect);
            }
            _ => {}
        }
    }
}
