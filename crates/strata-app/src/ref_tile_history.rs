//! 「1919 조선지형도 1:5만」: the 1:50,000 sheets of 1919 from the history-GIS server of 국사편찬위원회
//! (`target/cpp-ref/src/core/BasemapOps.cpp:1486-1511`, `src/app/MainWindowOffline.cpp:467-477`). A live WMTS
//! in the server's own grid (EPSG:5179); it needs the user's history-GIS key (ini key `HistoryGis/ApiKey`).
//! The C++ shows it wherever the view is; here it is seen within 10 km of the survey area (owner 2026-11-15).

use super::{RefTileLayer, ViewOnAdd};
use crate::app_state::StrataAppState;
use crate::basemap_tiles::TileKind;

#[cfg(test)]
#[path = "ref_tile_history_tests.rs"]
mod tests;

/// `LayerOps` text when no key is stored (`BasemapOps.cpp`, pinned by the C++ test_cadastral.cpp:143-153).
const NO_KEY: &str = "1919 조선지형도는 국사편찬위원회 역사지리정보DB API 키가 필요합니다. 더보기 → API 키 입력에서 키를 저장하세요. https://hgis.history.go.kr/api/intro.do";

/// The 1919 map. The finest level of the server's grid is 16.
pub(crate) const MAP_1919: RefTileLayer = RefTileLayer {
    key: "ref_history_1919",
    title: "1919 조선지형도 1:5만",
    kind: TileKind::History1919,
    max_zoom: 16,
    radius_m: 10_000.0,
    added_text: "1919 조선지형도를 올렸습니다. 조사구역에서 반경 10km 안에 보입니다.",
    needs_area_text: "조사구역을 먼저 그려 주세요. 조사구역에서 반경 10km 안의 1919 조선지형도를 올립니다.",
    on_add: ViewOnAdd::FrameCircle,
};

impl StrataAppState {
    /// The menu row 「1919 조선지형도 1:5만」 (`MainWindow::addHistoryGisMap1919`), with the stored key.
    pub(crate) fn add_history_map_1919(&mut self) {
        self.add_history_map_1919_with(&strata_secrets::vworld::VworldSettings::load_history_gis_api_key());
    }

    /// The same with the key given.
    pub(crate) fn add_history_map_1919_with(&mut self, key: &str) {
        if key.trim().is_empty() {
            return self.show_status_message(NO_KEY);
        }
        if self.add_reference_tile_layer(&MAP_1919) {
            key.trim().clone_into(&mut self.history_gis_key);
        }
    }

    /// The key a tile of this kind is asked for with: the history-GIS key for the 1919 map, none otherwise.
    pub(crate) fn tile_key_for(&self, kind: TileKind) -> String {
        if kind == TileKind::History1919 { self.history_gis_key.clone() } else { String::new() }
    }

    /// Whether one of the two old maps is shown: the 옛 지도 button then looks pressed (`MainWindowChrome.cpp:245`).
    pub(crate) fn old_map_shown(&self) -> bool {
        self.reference_tile_layer_shown(&super::DAEDONGYEOJIDO) || self.reference_tile_layer_shown(&MAP_1919)
    }
}
