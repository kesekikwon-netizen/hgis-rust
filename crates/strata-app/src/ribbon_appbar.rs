//! 상단 검색창(PNU 조립·필지 확대) 상태.
//!
//! C++ `KaAddressSearchService::buildPnuCode` / `zoomToParcelExtent` 1:1.
//! `ribbon.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

#[derive(Debug, Clone)]
pub struct KaAppBarSearch {
    pub query: String,
    pub selected_sido: String,
    pub region_popup_open: bool,
    pub address_popup_open: bool,
    pub region_button_rect: Option<[f32; 4]>,
    pub city_input: String,
    pub dong_input: String,
    pub lot_input: String,
    pub search_results_modal: Vec<strata_net::address_search::LocationHit>,
    pub selected_hit_idx: usize,
    pub search_error_modal: Option<String>,
}

impl Default for KaAppBarSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected_sido: "경북".to_owned(),
            region_popup_open: false,
            address_popup_open: false,
            region_button_rect: None,
            city_input: "경주시".to_owned(),
            dong_input: "인왕동".to_owned(),
            lot_input: "76".to_owned(),
            search_results_modal: Vec::new(),
            selected_hit_idx: 0,
            search_error_modal: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnuBuildError {
    InvalidBjdCode,
    InvalidBunNumber,
}

impl KaAppBarSearch {
    pub const HGIS_PNU_TOTAL_LENGTH: usize = 19;
    pub const HGIS_BUN_PADDING_WIDTH: usize = 4;
    pub const HGIS_PARCEL_ZOOM_MARGIN_M: f64 = 50.0;

    /// C++ `KaAddressSearchService::buildPnuCode` — 10자리 법정동코드 + 산 여부('1'/'2') + 4자리 0패딩 본번 + 4자리 0패딩 부번으로 19자리 표준 PNU 조합
    pub fn build_pnu_code(
        bjd_code10: &str,
        is_mountain: bool,
        main_bun: i32,
        sub_bun: i32,
    ) -> Result<String, (PnuBuildError, String)> {
        let trimmed = bjd_code10.trim();
        if trimmed.len() != 10 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err((
                PnuBuildError::InvalidBjdCode,
                "법정동 코드는 10자리 숫자여야 합니다.".to_owned(),
            ));
        }
        if !(1..=9999).contains(&main_bun) || !(0..=9999).contains(&sub_bun) {
            return Err((
                PnuBuildError::InvalidBunNumber,
                "본번은 1 이상의 정수여야 합니다.".to_owned(),
            ));
        }
        let m_char = if is_mountain { '2' } else { '1' };
        let pnu = format!("{trimmed}{m_char}{main_bun:04}{sub_bun:04}");
        debug_assert_eq!(pnu.len(), Self::HGIS_PNU_TOTAL_LENGTH);
        Ok(pnu)
    }

    /// C++ `KaAddressSearchService::zoomToParcelExtent` — 필지 외곽 경계 사각형 전 방향에 50.0m 여백 가산
    #[must_use]
    pub fn zoom_to_parcel_extent(parcel_bbox: [f64; 4]) -> [f64; 4] {
        let m = Self::HGIS_PARCEL_ZOOM_MARGIN_M;
        [
            parcel_bbox[0] - m,
            parcel_bbox[1] - m,
            parcel_bbox[2] + m,
            parcel_bbox[3] + m,
        ]
    }
}
