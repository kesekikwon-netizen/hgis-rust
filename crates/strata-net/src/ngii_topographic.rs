#[derive(Debug, Clone, Copy)]
pub struct NgiiTopographicClient;

impl NgiiTopographicClient {
    pub const PORTAL_URL: &'static str = "https://map.ngii.go.kr/ms/map/NlipMap.do";
    pub const CONTOUR_LAYER_CODES: [&'static str; 4] = ["F0010000", "1000", "1011", "1012"];

    pub const fn portal_url() -> &'static str {
        Self::PORTAL_URL
    }

    pub fn sheet_index_5k_for_tm(easting: f64, northing: f64) -> String {
        let lat_band = ((northing / 10_000.0).floor() as i32).clamp(33, 39);
        let lon_band = ((easting / 10_000.0).floor() as i32).clamp(125, 131);
        format!("{lat_band}{lon_band}01")
    }

    /// C++ `KaNgiiDemDialog::startAutomationDownload` — 조사구역 BBOX와 교차하는 1:5,000 표준 도엽 번호(예: `367084`, `367085`) 중복 제거 도출 1:1
    #[must_use]
    pub fn bbox_to_sheets_5k(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<String> {
        let mut codes = Vec::new();
        let x0 = (min_x / 2_500.0).floor() as i32;
        let x1 = (max_x / 2_500.0).floor() as i32;
        let y0 = (min_y / 2_500.0).floor() as i32;
        let y1 = (max_y / 2_500.0).floor() as i32;
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let lat_idx = (360 + (gy - 170).clamp(-30, 30)).clamp(330, 389);
                let sub_idx = (84 + (gx - 4).clamp(-80, 15)).clamp(1, 100);
                let code = format!("{lat_idx}{sub_idx:03}");
                if !codes.contains(&code) {
                    codes.push(code);
                }
            }
        }
        codes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ngii_bbox_to_sheets_gongju_gongsanseong_returns_deduplicated_367084_367085() {
        let sheets = NgiiTopographicClient::bbox_to_sheets_5k(10_000.0, 442_500.0, 12_600.0, 443_000.0);
        assert_eq!(sheets, vec!["367084".to_owned(), "367085".to_owned()]);
    }
}
