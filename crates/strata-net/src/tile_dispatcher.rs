//! 타일 디스패처·웹뷰 자동화 계획 (C++ `KaTileFetcher`, `KaHeritageBrowser`).
//!
//! `tile_fetcher` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use crate::heritage_intranet::HeritageIntranetFlow;
use crate::ngii_topographic::NgiiTopographicClient;
use crate::vworld_client::VworldClient;

use super::tile_layers::BasemapSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebViewAutomationPlan {
    pub portal_url: String,
    pub referer: String,
    pub injected_scripts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AsyncTileDispatcher {
    pub active_source: Option<BasemapSource>,
}

impl AsyncTileDispatcher {
    #[must_use]
    pub fn synthesize_basemap_backdrop_rgba(
        source: BasemapSource,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let w = width.max(16) as usize;
        let h = height.max(16) as usize;
        let mut buf = vec![0_u8; w * h * 4];
        let base_rgb: [u8; 3] = match source {
            BasemapSource::VworldSatellite => [52, 74, 58],
            BasemapSource::VworldBase => [242, 238, 229],
            BasemapSource::VworldCadastralWms => [248, 244, 236],
            BasemapSource::Daedongyeojido => [232, 218, 190],
            BasemapSource::History1919 => [224, 214, 196],
        };
        for y in 0..h {
            for x in 0..w {
                let grid_line = (x % 32 == 0) || (y % 32 == 0);
                let c = if grid_line {
                    [
                        base_rgb[0].saturating_sub(16),
                        base_rgb[1].saturating_sub(16),
                        base_rgb[2].saturating_sub(16),
                        255,
                    ]
                } else {
                    [base_rgb[0], base_rgb[1], base_rgb[2], 255]
                };
                let off = (y * w + x) * 4;
                buf[off..off + 4].copy_from_slice(&c);
            }
        }
        buf
    }

    #[must_use]
    pub fn build_heritage_webview_plan(
        username: &str,
        password: &str,
        _sido: &str,
        _sigungu: &str,
    ) -> WebViewAutomationPlan {
        WebViewAutomationPlan {
            portal_url: HeritageIntranetFlow::PORTAL_URL.to_owned(),
            referer: VworldClient::REFERER_HEADER.to_owned(),
            injected_scripts: vec![
                HeritageIntranetFlow::login_script(username, password),
                HeritageIntranetFlow::open_download_page_script().to_owned(),
                HeritageIntranetFlow::search_script(),
            ],
        }
    }

    #[must_use]
    pub fn build_ngii_webview_plan(sheet_code: &str) -> WebViewAutomationPlan {
        let _ = NgiiTopographicClient;
        WebViewAutomationPlan {
            portal_url: "https://map.ngii.go.kr/mn/mainPage.do".to_owned(),
            referer: "https://map.ngii.go.kr".to_owned(),
            injected_scripts: vec![format!("window.__ka_ngii_sheet={sheet_code:?};")],
        }
    }
}
