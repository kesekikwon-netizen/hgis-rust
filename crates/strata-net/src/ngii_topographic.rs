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
}
