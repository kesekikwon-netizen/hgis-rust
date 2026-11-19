//! 리본 단추 클릭 처리(뒷부분) — 라벨별 동작.

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 뒤쪽 라벨들.
    pub(crate) fn handle_ribbon_click_b(&mut self, label: &str) {
        match label {
            // MainWindowOverlay.cpp:239-267 toggleTerrainMap — the first click adds 「지형맵」, later clicks
            // hide and show its legend row (ref_tile_layers.rs).
            "지형" => {
                self.active_tab = MainTab::Map;
                self.toggle_reference_tile_layer(&crate::ref_tile_layers::TERRAIN_MAP);
            }
            // C++ MainWindowRibbon.cpp:429-436 — `contour` 단추에는 setCheckable 이 없어
            // 토글이 아니고, 누르면 MainWindowSurveyContour.cpp:104 createSurveyContours 만
            // 돈다. 그 함수는 단면선 같은 도메인 레이어를 만들지 않는다(계산이 끝나면
            // :165 SurveyContourStyle::apply 가 결과 레이어를 올린다).
            "등고선" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::SurveyContour);
            }
            // MainWindowOverlay.cpp:438-490 downloadSoilTerrain: with the layer in the legend the button hides and
            // shows it; without it the map is fetched (ref_map.rs). The picture row under it
            // (`토양도(흙토람 그림)`, SoilMapService.cpp:109-138) is order NET-SOIL1b.
            "토양" => self.press_reference_map(&crate::ref_vector::map::SOIL),
            // MainWindowOverlay.cpp:405-472 downloadGeologyMap: with the layer in the legend the button hides and
            // shows it; without it the map is fetched (ref_map.rs). The relief under it (`지질 음영`,
            // GeologyMapService.cpp:609-636) is order NET-GEO1c.
            "지질" => self.press_reference_map(&crate::ref_vector::map::GEOLOGY),
            // MainWindowOverlay.cpp:550-596 downloadRiverMap: with the layer in the legend the button hides and
            // shows it; without it the river network is fetched with the user's VWorld key (ref_map.rs).
            "수계" => self.press_reference_map(&crate::ref_vector::map::RIVER),
            // InstantPopup chooses a map before changing layers (target/cpp-ref/src/app/MainWindowRibbon.cpp:486-500).
            "옛 지도" => {
                let anchor = eframe::egui::Rect::from_min_size(self.ribbon.context_menu_anchor, eframe::egui::Vec2::ZERO);
                self.open_ribbon_context_menu(label, anchor);
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
                    self.request_align_file();
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
                    self.home_map.draw_guide_message = Some(StrataAppState::DRAWING_STUDIO_WELCOME.to_owned());
                } else {
                    self.modals.active = Some(ActiveModalDialog::PaperSetup);
                }
            }
            // Separate app tab, requested by owner (target/cpp-ref/src/app/MainWindowExport.cpp:186-194).
            "인쇄" => self.open_studio_tab(MainTab::Print),
            // Deferred until R7 (gui_ribbon_draw::deferred_ribbon_tooltip): cpp writes a real map
            // GeoTIFF (MainWindowExport.cpp:58-96 + MapGeoTiffExport.cpp); the Rust writer made a
            // header-only TIFF, so the click opens no dialog, writes nothing and says nothing.
            "GeoTIFF" => {}
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            // MainWindowRibbon.cpp:709-711 InstantPopup. The painted chip uses the same opener
            // (gui_ribbon_top.rs). Snap steps and handle_ribbon_click reach this arm directly.
            "더보기" => {
                let anchor = eframe::egui::Rect::from_min_size(self.ribbon.context_menu_anchor, eframe::egui::Vec2::ZERO);
                self.open_ribbon_context_menu(label, anchor);
            }
            _ => self.active_tab = MainTab::Map,
        }
    }
}

#[cfg(test)]
#[path = "gui_ribbon_click_b_tests.rs"]
mod tests;
