//! 시·도 이동과 지역·주소 검색 — C++ `MainWindow::zoomToLocation` / `searchLocation`.


use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon::PENINSULA_REGION_TILES;

impl StrataAppState {
    /// `MainWindowRibbon.cpp:600-620` — 시·도 칩을 누르면 해당 시·도 개관 범위(overviewBounds)로 지도 이동
    pub(crate) fn zoom_to_sido_overview(&mut self, sido: &str) {
        if let Some(bounds) = strata_net::address_search::KoreaRegionCatalog::overview_bounds(sido)
        {
            let hit = strata_net::address_search::LocationHit {
                title: strata_net::address_search::KoreaRegionCatalog::canonical_sido(sido),
                detail: String::new(),
                lon: (bounds.west + bounds.east) * 0.5,
                lat: (bounds.south + bounds.north) * 0.5,
                bbox: Some([bounds.west, bounds.south, bounds.east, bounds.north]),
                pnu: String::new(),
            };
            self.zoom_to_location_hit(&hit);
        } else if let Some(tile) = PENINSULA_REGION_TILES.iter().find(|t| t.name == sido) {
            self.home_map.viewport.center_xy = tile.center_5187;
            self.active_tab = MainTab::Map;
        }
    }

    /// `MainWindow::zoomToLocation` (`MainWindow.cpp:2833-2890`) 1:1
    pub(crate) fn zoom_to_location_hit(&mut self, hit: &strata_net::address_search::LocationHit) {
        let mut lon = hit.lon;
        let mut lat = hit.lat;
        if lon.abs() <= 90.0 && lat.abs() > 90.0 {
            std::mem::swap(&mut lon, &mut lat);
        }
        let dst_crs = match self.work_crs {
            strata_types::crs::WorkCrs::Epsg5186 => strata_types::crs::CrsCode::Epsg5186,
            strata_types::crs::WorkCrs::Epsg5187 => strata_types::crs::CrsCode::Epsg5187,
        };
        let xf = strata_geo::crs_transform::CrsTransformer::new(
            strata_types::crs::CrsCode::Epsg4326,
            dst_crs,
        );
        let p = xf.transform_xy(lon, lat);
        self.home_map.viewport.center_xy = p;
        if !hit.pnu.is_empty() {
            // 지번 필지 정확 검색: ~1:2,500 축척 (0.65 m/px)
            self.home_map.viewport.meters_per_pixel = 0.65;
        } else if let Some([west, south, east, north]) = hit.bbox {
            let sw = xf.transform_xy(west, south);
            let ne = xf.transform_xy(east, north);
            let span_m = (ne[0] - sw[0]).abs().max((ne[1] - sw[1]).abs()) * 1.2;
            self.home_map.viewport.meters_per_pixel = (span_m / 900.0).clamp(0.5, 250.0);
        } else {
            // 도로명·장소 점 위치: ~1:5,000 축척 (1.25 m/px)
            self.home_map.viewport.meters_per_pixel = 1.25;
        }
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!("위치 이동: {}", hit.title));
    }

    /// `KaAppBar::returnPressed` (`KaAppBar.cpp:54-57`) -> `MainWindow::searchLocation`
    pub(crate) fn run_app_bar_search(&mut self) {
        let q = self.ribbon.app_bar.query.trim().to_owned();
        if q.is_empty() {
            self.home_map.draw_guide_message = Some("주소·지번·지역·상호를 입력하세요".to_owned());
            return;
        }
        let parcel = strata_net::address_search::looks_like_lot(&q);
        self.execute_interactive_location_search(&q, parcel);
    }

    /// `KaRegionLocator::emitSearch` (`KaRegionLocator.cpp:273-286`) -> `MainWindow::searchLocation`
    pub(crate) fn run_region_popup_search(&mut self) {
        let q = strata_net::address_search::KoreaRegionCatalog::compose_address(
            &self.ribbon.app_bar.selected_sido,
            &self.ribbon.app_bar.city_input,
            &self.ribbon.app_bar.dong_input,
            &self.ribbon.app_bar.lot_input,
        );
        if q.is_empty() {
            return;
        }
        let parcel = !self.ribbon.app_bar.lot_input.trim().is_empty();
        self.search_region_locator_address();
        self.ribbon.app_bar.query = q.clone();
        self.execute_interactive_location_search(&q, parcel);
    }

    fn execute_interactive_location_search(&mut self, query: &str, parcel: bool) {
        let key = self.accounts.vworld_key.clone();
        match strata_net::address_search::execute_location_search(query, parcel, &key) {
            Ok(hits) => {
                if hits.len() == 1 {
                    self.zoom_to_location_hit(&hits[0]);
                } else if hits.len() > 1 {
                    self.ribbon.app_bar.selected_hit_idx = 0;
                    self.ribbon.app_bar.search_results_modal = hits;
                } else {
                    self.home_map.draw_guide_message = Some("검색 결과 없음".to_owned());
                    self.ribbon.app_bar.search_error_modal = Some("검색 결과 없음".to_owned());
                }
            }
            Err(msg) => {
                self.home_map.draw_guide_message = Some(msg.clone());
                self.ribbon.app_bar.search_error_modal = Some(msg);
            }
        }
    }

    /// `KaRegionLocator::emitSearch` 오프라인/단위시험 경로 (`gui_shell.rs:820-842`)
    pub fn search_region_locator_address(&mut self) {
        let full_addr = format!(
            "{} {} {} {}",
            self.ribbon.app_bar.selected_sido,
            self.ribbon.app_bar.city_input.trim(),
            self.ribbon.app_bar.dong_input.trim(),
            self.ribbon.app_bar.lot_input.trim()
        );
        self.ribbon.app_bar.query = full_addr.trim().to_owned();
        if let Some(parcel) = self
            .home_map
            .real_survey
            .cadastral_parcels
            .iter()
            .find(|p| {
                !self.ribbon.app_bar.lot_input.trim().is_empty()
                    && p.jibun.contains(self.ribbon.app_bar.lot_input.trim())
            })
        {
            self.home_map.viewport.center_xy = parcel.center;
            self.home_map.viewport.meters_per_pixel = 1.25;
        } else if let Some(tile) = PENINSULA_REGION_TILES
            .iter()
            .find(|t| t.name == self.ribbon.app_bar.selected_sido)
        {
            self.home_map.viewport.center_xy = tile.center_5187;
        }
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!(
            "지역·지번 「{}」 위치로 이동했습니다.",
            self.ribbon.app_bar.query
        ));
        self.ribbon.app_bar.region_popup_open = false;
        self.ribbon.app_bar.address_popup_open = false;
    }
}
