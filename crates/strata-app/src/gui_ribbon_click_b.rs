//! 리본 단추 클릭 처리(뒷부분) — 라벨별 동작.



use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 뒤쪽 라벨들.
    pub(crate) fn handle_ribbon_click_b(&mut self, label: &str) {
        match label {
            // C++ MainWindowOverlay.cpp:243-271 toggleTerrainMap. 이미 있는 「지형맵」 은
            // :247 toggleExistingOverlay(:225-240)가 껐다 켰다 하고 `지형맵를 껐습니다.` /
            // `지형맵를 다시 켰습니다.` 를 내며, 처음 올릴 때만 :267 의
            // `지형맵을 올렸습니다. 다시 누르면 숨깁니다.` 가 나간다.
            "지형" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("지형맵") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("지형맵".to_owned(), true);
                    self.status_bar_hint =
                        "지형맵을 올렸습니다. 다시 누르면 숨깁니다.".to_owned();
                }
            }
            // C++ MainWindowRibbon.cpp:429-436 — `contour` 단추에는 setCheckable 이 없어
            // 토글이 아니고, 누르면 MainWindowSurveyContour.cpp:104 createSurveyContours 만
            // 돈다. 그 함수는 단면선 같은 도메인 레이어를 만들지 않는다(계산이 끝나면
            // :165 SurveyContourStyle::apply 가 결과 레이어를 올린다).
            "등고선" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::SurveyContour);
            }
            // C++ MainWindowOverlay.cpp:352-401 downloadSoilTerrain — 본체 클릭은
            // :357 toggleExistingOverlay(`토양도(흙토람)`)로 껐다 켰다 하고, 없을 때만
            // 내려받는다(:396 startReferenceDownload → :536,622 `토양도를 추가했습니다. …`).
            "토양" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("토양도(흙토람)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("토양도(흙토람)".to_owned(), true);
                    self.status_bar_hint =
                        "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
            }
            // C++ MainWindowOverlay.cpp:405-472 downloadGeologyMap — :409 toggleExistingOverlay
            // (`지질도(KIGAM 1:5만)`, m_actGeology)로 껐다 켰다 하고, 켤 때는 지형 음영
            // 겹침(GeologyMapService::reliefLayerTitle = `지질 음영`, GeologyMapService.cpp:60)
            // 도 같은 상태로 맞춘다(:422-423). 없을 때만 내려받는다(:467 → :536,622).
            "지질" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("지질도(KIGAM 1:5만)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("지질도(KIGAM 1:5만)".to_owned(), true);
                    self.status_bar_hint =
                        "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
                let now_on = self
                    .home_map
                    .real_survey
                    .is_layer_visible("지질도(KIGAM 1:5만)");
                self.home_map
                    .real_survey
                    .layer_visible
                    .insert("지질 음영".to_owned(), now_on);
            }
            // C++ MainWindowOverlay.cpp:476-527 downloadRiverMap — :479 toggleExistingOverlay
            // (`수계도(하천망)`, m_actRiver)로 껐다 켰다 하고, 없을 때만 내려받는다
            // (:522 → :536,622 `수계도를 추가했습니다. …`).
            "수계" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("수계도(하천망)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("수계도(하천망)".to_owned(), true);
                    self.status_bar_hint =
                        "수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
            }
            "옛 지도" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000,
                    0.8,
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "고지형" => self.open_studio_tab(MainTab::Paleo),
            // C++ MainWindowRibbon.cpp:522-523 — 「정합」 단추는 setCheckable 없이
            // `georefAssistant` 에 연결된다. 다시 눌러도 세션을 끄지 않고 맞출 도면을 고르는
            // 흐름이 다시 돈다. 끄는 길은 서브툴바 「닫기」(:793) → hideSubTools(:769-777)
            // → stopAlignSession(:705) 하나뿐이다.
            "정합" => {
                self.active_tab = MainTab::Map;
                // :846-875 — 맞출 수 있는 도면(도메인 조사 레이어가 아닌 OGR·메모리 벡터와
                // GDAL 래스터, GeorefService.cpp:502-512)이 있는지 본다.
                if let Some(target) = self.alignable_layer_name() {
                    self.start_align_session(&target);
                } else {
                    // :871-874 QMessageBox::information 원문
                    self.home_map.draw_guide_message = Some(
                        "맞추기 — JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요."
                            .to_owned(),
                    );
                }
            }
            // MainWindowRibbon.cpp:552-557 「도면」 → MainWindow::openLayoutDesigner (Ctrl+L).
            // MainWindowExport.cpp:204-213 — the studio tab already exists: go there without
            // asking for the paper again (re-opening would move the user's sheet view).
            // MainWindowExport.cpp:216 — only the first open asks, through KaPaperDialog.
            "도면" => {
                if self.open_studio_tabs.contains(&MainTab::Drawing) {
                    self.open_studio_tab(MainTab::Drawing);
                    // MainWindowExport.cpp:206 `hideSubTools()` — 도면 탭으로 가면 도구 줄을 닫는다.
                    self.hide_sub_tools();
                    self.home_map.draw_guide_message =
                        Some(StrataAppState::DRAWING_STUDIO_WELCOME.to_owned());
                } else {
                    self.modals.active = Some(ActiveModalDialog::PaperSetup);
                }
            }
            "인쇄" => {
                // MainWindowExport.cpp:186-194 — 도면 화면이 앞에 없으면 도면 탭을 열고
                // 인쇄 창을 띄운다. 용지 기본값은 KaPrintDialog.cpp:124 대로 프린터가
                // A3 를 지원할 때만 A3 다.
                self.open_studio_tab(MainTab::Drawing);
                self.modals.print_paper =
                    crate::dialogs_modal::default_print_paper_for_printer();
                self.modals.active = Some(ActiveModalDialog::DrawingPrint);
            }
            // Deferred until R7 (gui_ribbon_draw::deferred_ribbon_tooltip): cpp writes a real map
            // GeoTIFF (MainWindowExport.cpp:58-96 + MapGeoTiffExport.cpp); the Rust writer made a
            // header-only TIFF, so the click opens no dialog, writes nothing and says nothing.
            "GeoTIFF" => {}
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.modals.active = Some(ActiveModalDialog::AccountSettings),
            _ => self.active_tab = MainTab::Map,
        }
    }
}

#[cfg(test)]
#[path = "gui_ribbon_click_b_tests.rs"]
mod tests;
