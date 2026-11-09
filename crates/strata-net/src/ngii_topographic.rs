//! `ngii_topographic` 는 국토지리정보원 수치지형도 요청 기능의 재수출 창구다.
//!
//! 원래 807줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::ngii_topographic::<이름>` 경로는 그대로 동작한다.

#[path = "ngii_protocol.rs"]
mod ngii_protocol;
#[path = "ngii_parse.rs"]
mod ngii_parse;
#[path = "ngii_sheets.rs"]
mod ngii_sheets;
#[path = "ngii_fetch.rs"]
mod ngii_fetch;

pub use ngii_fetch::*;
pub use ngii_protocol::*;
pub use ngii_sheets::*;

impl NgiiTopographicClient {
    /// **C++ 에 대응이 없는 옛 계산이다.** 기존 시험이 이 값을 고정하고 있어 지우지 않는다.
    /// 실제 앱 경로는 아래 `sheets_for_bbox_25k` / `TopographicSheets` 를 쓴다.
    pub fn sheet_index_5k_for_tm(easting: f64, northing: f64) -> String {
        let lat_band = ((northing / 10_000.0).floor() as i32).clamp(33, 39);
        let lon_band = ((easting / 10_000.0).floor() as i32).clamp(125, 131);
        format!("{lat_band}{lon_band}01")
    }

    /// **C++ 에 대응이 없는 옛 계산이다.** 기존 시험이 이 값을 고정하고 있어 지우지 않는다.
    /// 실제 앱 경로는 아래 `sheets_for_bbox_25k` 를 쓴다.
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

    /// 조사구역 BBOX(경위도)와 교차하는 1:25,000 도엽 번호 —
    /// C++ `TopographicSheets::select`(TopographicSheets.cpp:110-138)와 같은 격자 규칙.
    #[must_use]
    pub fn sheets_for_bbox_25k(box_wgs84: [f64; 4]) -> Vec<String> {
        TopographicSheets::numbers_in_geographic_box(box_wgs84)
    }
}

#[cfg(test)]
#[path = "ngii_topographic_tests.rs"]
mod tests;
