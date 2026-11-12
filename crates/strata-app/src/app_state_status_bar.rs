//! 상태줄 좌표·축척 서식 (C++ `KaStatusBar` 1:1).
//!
//! `app_state.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 메서드는 그대로다.

use super::app_state_types::MainTab;
use crate::app_state::StrataAppState;

impl StrataAppState {
    pub const HGIS_STANDARD_DPI: f64 = 96.0;
    pub const HGIS_METERS_PER_INCH: f64 = 0.0254;
    pub const HGIS_FALLBACK_SCALE_DENOM: f64 = 1_000.0;
    pub const HGIS_PRESET_SCALES: [u32; 6] = [100, 500, 1_000, 2_500, 5_000, 10_000];
    pub const HGIS_PRESET_SCALES_7: [u32; 7] = [500, 1_000, 2_500, 5_000, 10_000, 25_000, 50_000];

    /// C++ `KaStatusBar::updateCoordinates(const QgsPointXY &point)` 1:1 대응:
    /// 천단위 콤마 및 소수점 3자리(밀리미터 해상도) 고정폭 포맷팅 (`"X 219,897.093  Y 361,622.354"`).
    /// 부동소수점 오염(`NaN`/`Infinity`) 시 `"X ---.---  Y ---.---"` 반환.
    #[must_use]
    pub fn format_status_bar_coordinates(x: f64, y: f64) -> String {
        if !x.is_finite() || !y.is_finite() {
            return "X ---.---  Y ---.---".to_owned();
        }
        fn fmt_coord(val: f64) -> String {
            let sign = if val < 0.0 { "-" } else { "" };
            let abs_v = val.abs();
            let rounded_millis = (abs_v * 1000.0).round() as u64;
            let int_part = rounded_millis / 1000;
            let frac_part = rounded_millis % 1000;
            let digits = int_part.to_string();
            let mut rev_commas = String::with_capacity(digits.len() + digits.len() / 3);
            for (idx, ch) in digits.chars().rev().enumerate() {
                if idx > 0 && idx % 3 == 0 {
                    rev_commas.push(',');
                }
                rev_commas.push(ch);
            }
            let int_str: String = rev_commas.chars().rev().collect();
            format!("{sign}{int_str}.{frac_part:03}")
        }
        format!("X {:>11}  Y {:>11}", fmt_coord(x), fmt_coord(y))
    }

    /// C++ `KaStatusBar::computeRepresentativeScale` 폴백 버전 — 비정상 입력 시 `1000.0` (`1:1,000`) 반환
    #[must_use]
    pub fn compute_representative_scale_or_fallback(meters_per_pixel: f64, screen_dpi: f64) -> f64 {
        Self::compute_representative_scale(meters_per_pixel, screen_dpi)
            .unwrap_or(Self::HGIS_FALLBACK_SCALE_DENOM)
    }

    /// C++ `KaStatusBar.cpp:82-84,92-98` `scaleEdit` — 축척칸에 `"1000"` 또는 `"1:2500"`을
    /// 적고 Enter를 누르면 그 분모를 그대로 쓴다(`parse_scale_string`).
    ///
    /// 지도 탭은 화면 축척(QGIS 96 dpi: 점당 m × 96 / 0.0254)을, 도면 탭은 도면 축척을 바꾼다.
    /// 읽을 수 없는 글자는 아무것도 바꾸지 않고 `false`를 돌려준다.
    pub fn apply_scale_input(&mut self) -> bool {
        let Ok(denom) = strata_canvas2d::viewport::parse_scale_string(&self.scale_input_buf) else {
            return false;
        };
        if self.active_tab == MainTab::Drawing {
            self.drawing.sheet.scale_denominator = denom.round() as u32;
        } else {
            self.home_map.viewport.meters_per_pixel = denom / crate::status_bar_text::POINTS_PER_METER;
        }
        self.scale_input_buf = strata_canvas2d::viewport::format_scale_denominator(denom);
        true
    }

    /// C++ `KaStatusBar::computeRepresentativeScale(metersPerPixel, screenDpi)` — 0 나누기/NaN 방어 및 1:N 축척 분모 산출
    #[must_use]
    pub fn compute_representative_scale(meters_per_pixel: f64, screen_dpi: f64) -> Option<f64> {
        if !meters_per_pixel.is_finite()
            || meters_per_pixel <= 1e-9
            || !screen_dpi.is_finite()
            || screen_dpi <= 0.0
        {
            return None;
        }
        let pixels_per_meter = screen_dpi / Self::HGIS_METERS_PER_INCH;
        Some(meters_per_pixel * pixels_per_meter)
    }

    /// C++ `KaStatusBar` 축척 라벨 포맷 — `"1:1,000"` 또는 비정상 입력 시 `"1: ---"`
    #[must_use]
    pub fn format_representative_scale(meters_per_pixel: f64, screen_dpi: f64) -> String {
        let Some(scale) = Self::compute_representative_scale(meters_per_pixel, screen_dpi) else {
            return "1: ---".to_owned();
        };
        let n = scale.round() as u64;
        let digits = n.to_string();
        let mut out = String::with_capacity(digits.len() + digits.len() / 3);
        for (idx, ch) in digits.chars().rev().enumerate() {
            if idx > 0 && idx % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let formatted: String = out.chars().rev().collect();
        format!("1:{formatted}")
    }

    /// C++ `KaStatusBar` 더티 플래그 문구 전환
    #[must_use]
    pub const fn dirty_status_label_ko(is_dirty: bool) -> &'static str {
        if is_dirty {
            "● 저장 필요"
        } else {
            "✓ 저장됨"
        }
    }
}
