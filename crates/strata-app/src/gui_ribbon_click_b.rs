//! 리본 단추 클릭 처리(뒷부분) — 라벨별 동작.



use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 뒤쪽 라벨들.
    pub(crate) fn handle_ribbon_click_b(&mut self, label: &str) {
        match label {
            // C++ `MainWindowRibbon.cpp:532-547` btnBoreholeFetch (`d7afcc8`) —
            // 조사 폴더가 없으면 「먼저 조사 폴더를 준비하세요」 안내만 남기고 창을 열지 않는다
            // (`KaBoreholeDialog.cpp:66-70`).
            "시추" => {
                self.active_tab = MainTab::Map;
                let _ = self.open_borehole_dialog();
            }
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
                self.status_bar_hint =
                    "토양도(흙토람) 받기는 아직 Rust 판에 옮기지 못했습니다. 지도에는 아무것도 더해지지 않았습니다.".to_owned();
            }
            // C++ MainWindowOverlay.cpp:405-472 downloadGeologyMap — :409 toggleExistingOverlay
            // (`지질도(KIGAM 1:5만)`, m_actGeology)로 껐다 켰다 하고, 켤 때는 지형 음영
            // 겹침(GeologyMapService::reliefLayerTitle = `지질 음영`, GeologyMapService.cpp:60)
            // 도 같은 상태로 맞춘다(:422-423). 없을 때만 내려받는다(:467 → :536,622).
            "지질" => {
                self.active_tab = MainTab::Map;
                // 있으면 켜고 끄고, 없으면 KIGAM 공개 서비스에서 받아 올린다.
                self.toggle_or_download_geology();
            }
            // C++ MainWindowOverlay.cpp:476-527 downloadRiverMap — :479 toggleExistingOverlay
            // (`수계도(하천망)`, m_actRiver)로 껐다 켰다 하고, 없을 때만 내려받는다
            // (:522 → :536,622 `수계도를 추가했습니다. …`).
            "수계" => {
                self.active_tab = MainTab::Map;
                self.status_bar_hint =
                    "수계도(하천망) 받기는 아직 Rust 판에 옮기지 못했습니다. 지도에는 아무것도 더해지지 않았습니다.".to_owned();
            }
            "옛 지도" => {
                self.active_tab = MainTab::Map;
                self.home_map.draw_guide_message = Some(
                    "옛 지도(대동여지도·1919 조선지형도) 배경은 아직 Rust 판에 옮기지 못했습니다. 지도는 바뀌지 않았습니다."
                        .to_owned(),
                );
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
            // MainWindowRibbon.cpp:566-573 + KaShellUi.cpp:48-50 + MainWindow.cpp:1086 —
            // GeoTIFF 는 「지도」 탭에서만 쓴다. 다른 탭이면 사유만 알리고 끝낸다.
            "GeoTIFF" => {
                if self.active_tab != MainTab::Map {
                    self.home_map.draw_guide_message =
                        Some(crate::dialogs_download_print::MAP_TAB_ONLY_REASON.to_owned());
                    return;
                }
                // MainWindowExport.cpp:63-67 — 저장할 레이어가 없으면 안내만 한다.
                if self.left_layers.entries.is_empty() {
                    self.home_map.draw_guide_message =
                        Some("지도에 저장할 레이어가 없습니다.".to_owned());
                    return;
                }
                // MainWindowExport.cpp:68-75 — 저장 대화상자: 기본 이름 지도_EPSG_5187.tif,
                // 필터 "GeoTIFF (*.tif *.tiff)", 기본 확장자 tif, 시작 폴더는 조사 폴더.
                let default_name =
                    crate::dialogs_download_print::geotiff_default_file_name(self.work_crs.auth_id());
                let Some(path) = ::rfd::FileDialog::new()
                    .set_title("현재 지도 GeoTIFF 저장")
                    .add_filter("GeoTIFF", &["tif", "tiff"])
                    .set_file_name(&default_name)
                    .set_directory(&self.home_map.real_survey.survey_dir)
                    .save_file()
                else {
                    return;
                };
                // MapGeoTiffExport.cpp:94-103 — 화면 캔버스 출력 크기 그대로, 상한만 검사한다.
                let size = match crate::dialogs_download_print::geotiff_canvas_size_px(
                    self.home_map.viewport.screen_size_px,
                ) {
                    Ok(sz) => sz,
                    Err(reason) => {
                        self.home_map.draw_guide_message = Some(reason.to_owned());
                        return;
                    }
                };
                // MapGeoTiffExport.cpp:187-190 — 좌표계는 지도 좌표계 그대로 기록한다.
                let auth_id = self.work_crs.auth_id();
                if strata_layout::pdf_geotiff::export_geotiff_with_cancel(
                    &path,
                    size.0,
                    size.1,
                    self.work_crs.epsg_code(),
                    None,
                )
                .is_ok()
                {
                    // MainWindowExport.cpp:90-91 — "GeoTIFF 저장 완료 · EPSG:5187 · 경로"
                    self.home_map.draw_guide_message =
                        Some(format!("GeoTIFF 저장 완료 · {} · {}", auth_id, path.display()));
                } else {
                    self.home_map.draw_guide_message =
                        Some("GeoTIFF를 저장하지 못했습니다".to_owned());
                }
            }
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.modals.active = Some(ActiveModalDialog::AccountSettings),
            _ => self.active_tab = MainTab::Map,
        }
    }
}
