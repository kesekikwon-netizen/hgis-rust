//! 리본 우클릭 메뉴 열기와 고지형·지질·DEM 메뉴 항목 실행.

use eframe::egui::{self, Pos2};

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// KaPaleoCrossSectionWidget.cpp:99-158 `KaPaleoMenu::install` +
    /// MainWindowRibbon.cpp:390-711 —
    /// 리본 `고지형`/`지질`/`DEM`/`토양`/`지적`/`수치지형`/`옛 지도`/`더보기` 팝업 메뉴.
    pub(crate) fn open_ribbon_context_menu(&mut self, label: &str, anchor: egui::Rect) {
        self.ribbon.paleo_menu_open = false;
        self.ribbon.geology_menu_open = false;
        self.ribbon.dem_menu_open = false;
        self.ribbon.soil_menu_open = false;
        self.ribbon.cadastral_menu_open = false;
        self.ribbon.topo_menu_open = false;
        self.ribbon.old_map_menu_open = false;
        self.ribbon.more_menu_open = false;
        self.ribbon.context_menu_anchor = Pos2::new(anchor.left(), anchor.bottom() + 2.0);
        match label {
            "고지형" => self.ribbon.paleo_menu_open = true,
            "지질" => self.ribbon.geology_menu_open = true,
            "DEM" => self.ribbon.dem_menu_open = true,
            "토양" => self.ribbon.soil_menu_open = true,
            "지적" => self.ribbon.cadastral_menu_open = true,
            "수치지형" => self.ribbon.topo_menu_open = true,
            "옛 지도" => self.ribbon.old_map_menu_open = true,
            "더보기" => {
                self.ribbon.context_menu_anchor =
                    Pos2::new((anchor.right() - 250.0).max(16.0), anchor.bottom() + 2.0);
                self.ribbon.more_menu_open = true;
            }
            _ => {}
        }
    }
    pub(crate) fn run_paleo_menu_action(&mut self, item: &str) {
        if item.contains("전용 탭 열기") {
            self.open_studio_tab(MainTab::Paleo);
        } else if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 5km · 도면 범례") {
            self.paleo_3d.source_dl_radius_km = 5.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("포털 열기") {
            self.home_map.draw_guide_message =
                Some(format!("{item} — 브라우저에서 이어서 받으세요."));
        }
    }
    pub(crate) fn run_geology_menu_action(&mut self, item: &str) {
        if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
            return;
        }
        if item.contains("반경 2km") {
            self.paleo_3d.source_dl_radius_km = 2.0;
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
        } else {
            self.paleo_3d.source_dl_radius_km = 5.0;
        }
        if item.contains("수치지질도") {
            self.active_tab = MainTab::Map;
            let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                strata_net::tile_fetcher::ThematicLayerType::Geology1to50k,
                self.work_crs.auth_id(),
            );
            self.home_map.draw_guide_message = Some(format!(
                "{} — 반경 {:.0} km 기준으로 켰습니다.",
                spec.layer_title_ko, self.paleo_3d.source_dl_radius_km
            ));
        } else {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        }
    }
    pub(crate) fn run_dem_menu_action(&mut self, item: &str) {
        if item.contains("계정 설정") {
            self.modals.active = Some(ActiveModalDialog::AccountSettings);
        } else if item.contains("DEM 불러오기") {
            self.modals.active = Some(ActiveModalDialog::DemDownload);
        } else if item.contains("음영 만들기") || item.contains("DEM 표현") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::DemClass);
        } else if item.contains("등고선") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::SurveyContour);
        }
    }
}
