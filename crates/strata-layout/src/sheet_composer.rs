#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPaperSize {
    A4,
    A3,
    A2,
    A1,
    A0,
}

impl SheetPaperSize {
    #[must_use]
    pub const fn portrait_dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::A3 => (297.0, 420.0),
            Self::A2 => (420.0, 594.0),
            Self::A1 => (594.0, 841.0),
            Self::A0 => (841.0, 1189.0),
        }
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A3 => "A3",
            Self::A2 => "A2",
            Self::A1 => "A1",
            Self::A0 => "A0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetOrientation {
    Portrait,
    Landscape,
}

impl SheetOrientation {
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Portrait => "세로",
            Self::Landscape => "가로",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetRectMm {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl SheetRectMm {
    #[must_use]
    pub fn inside_sheet(&self, sheet_w: f64, sheet_h: f64) -> bool {
        self.x_mm >= 0.0
            && self.y_mm >= 0.0
            && (self.x_mm + self.width_mm) <= sheet_w
            && (self.y_mm + self.height_mm) <= sheet_h
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawingSheet {
    pub paper_size: SheetPaperSize,
    pub orientation: SheetOrientation,
    pub width_mm: f64,
    pub height_mm: f64,
    pub map_frame: SheetRectMm,
    pub scale_bar_rect: SheetRectMm,
    pub scale_text_rect: SheetRectMm,
    pub crs_label_rect: SheetRectMm,
    pub north_arrow_rect: SheetRectMm,
    pub legend_rect: SheetRectMm,
    pub legend_kept_on_page: bool,
    pub scale_denominator: u32,
    pub grid_enabled: bool,
    pub grid_interval_m: f64,
    pub grid_follow_map: bool,
    pub grid_offset_xy_m: [f64; 2],
    pub show_site_names_under_5000: bool,
}

impl DrawingSheet {
    #[must_use]
    pub fn new(paper_size: SheetPaperSize, orientation: SheetOrientation) -> Self {
        let mut sheet = Self {
            paper_size,
            orientation,
            width_mm: 0.0,
            height_mm: 0.0,
            map_frame: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            scale_bar_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            scale_text_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            crs_label_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            north_arrow_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_kept_on_page: true,
            scale_denominator: 5_000,
            grid_enabled: true,
            grid_interval_m: 100.0,
            grid_follow_map: false,
            grid_offset_xy_m: [0.0, 0.0],
            show_site_names_under_5000: false,
        };
        sheet.recompute_layout();
        sheet
    }

    pub fn set_legend_kept_on_page(&mut self, kept_on_page: bool) {
        self.legend_kept_on_page = kept_on_page;
        self.recompute_layout();
    }

    /// `KaDrawingStudio::resetPaper` (`0001a93`, `c9e58e3`) — 용지를 바꿔도 축척자·축척 글·좌표계·방위는 아래 띠 자리로 고정하고 용지 밖 고립 항목을 복구
    pub fn change_paper(&mut self, paper_size: SheetPaperSize, orientation: SheetOrientation) {
        self.paper_size = paper_size;
        self.orientation = orientation;
        self.recompute_layout();
        self.replace_sheet_chrome();
        self.recover_orphaned_items();
    }

    /// `KaDrawingStudio::replaceSheetChrome` (`0001a93`) — 장식 넷(축척자·축척 글·좌표계·방위)을 어디에 끌어 두었든 새 용지의 아래 띠 자리로 되돌림
    pub fn replace_sheet_chrome(&mut self) {
        let w = self.width_mm;
        let h = self.height_mm;
        let margin = 12.0;
        let bottom_band_h = 22.0;
        let band_y = h - margin - bottom_band_h + 2.0;
        self.scale_bar_rect = SheetRectMm {
            x_mm: margin,
            y_mm: band_y,
            width_mm: 55.0,
            height_mm: 8.5,
        };
        self.scale_text_rect = SheetRectMm {
            x_mm: margin,
            y_mm: band_y + 9.0,
            width_mm: 55.0,
            height_mm: 7.0,
        };
        self.crs_label_rect = SheetRectMm {
            x_mm: margin + 60.0,
            y_mm: band_y,
            width_mm: 65.0,
            height_mm: 16.0,
        };
        self.north_arrow_rect = SheetRectMm {
            x_mm: (w - margin - 18.0).max(margin + 128.0),
            y_mm: band_y,
            width_mm: 16.0,
            height_mm: 16.0,
        };
    }

    /// `KaDrawingStudio.cpp` (`c9e58e3`) — A3 -> A4 전환 등에서 용지 밖으로 밀려난 범례/장식을 `clamp(pos, 4.0, max(4.0, page - size - 4.0))`로 복구
    pub fn recover_orphaned_items(&mut self) {
        if self.legend_kept_on_page {
            let max_x = (self.width_mm - self.legend_rect.width_mm - 4.0).max(4.0);
            let max_y = (self.height_mm - self.legend_rect.height_mm - 4.0).max(4.0);
            self.legend_rect.x_mm = self.legend_rect.x_mm.clamp(4.0, max_x);
            self.legend_rect.y_mm = self.legend_rect.y_mm.clamp(4.0, max_y);
        }
    }

    /// `LayoutService::applyMapCoordinateGrid` (`5362ef6`) — 지도에 미터 좌표격자가 켜져 있고 북쪽이 위(회전 0°)일 때 도면 격자를 지도 좌표격자와 같은 간격·같은 교점 오프셋으로 동기화
    pub fn apply_map_coordinate_grid(
        &mut self,
        map_grid_enabled: bool,
        map_rotation_deg: f64,
        step_m: f64,
        offset_xy_m: [f64; 2],
    ) -> bool {
        let north_up = (map_rotation_deg % 360.0).abs() < 1e-6;
        if map_grid_enabled && north_up && step_m > 0.0 {
            self.grid_enabled = true;
            self.grid_follow_map = true;
            self.grid_interval_m = step_m;
            self.grid_offset_xy_m = [
                offset_xy_m[0].rem_euclid(step_m),
                offset_xy_m[1].rem_euclid(step_m),
            ];
            true
        } else {
            self.grid_follow_map = false;
            false
        }
    }

    fn recompute_layout(&mut self) {
        let (pw, ph) = self.paper_size.portrait_dimensions_mm();
        let (w, h) = match self.orientation {
            SheetOrientation::Portrait => (pw, ph),
            SheetOrientation::Landscape => (ph, pw),
        };
        self.width_mm = w;
        self.height_mm = h;

        let margin = 12.0;
        let bottom_band_h = 22.0;
        self.map_frame = SheetRectMm {
            x_mm: margin,
            y_mm: margin,
            width_mm: (w - margin * 2.0).max(50.0),
            height_mm: (h - margin * 2.0 - bottom_band_h).max(50.0),
        };
        let band_y = h - margin - bottom_band_h + 2.0;
        self.replace_sheet_chrome();
        self.legend_rect = if self.legend_kept_on_page {
            SheetRectMm {
                x_mm: w - margin - 52.0,
                y_mm: band_y - 44.0,
                width_mm: 48.0,
                height_mm: 40.0,
            }
        } else {
            SheetRectMm {
                x_mm: w + 15.0,
                y_mm: margin,
                width_mm: 48.0,
                height_mm: 40.0,
            }
        };
    }

    #[must_use]
    pub fn all_decorations_inside_sheet(&self) -> bool {
        let in_frame = self.map_frame.inside_sheet(self.width_mm, self.height_mm)
            && self.scale_bar_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.scale_text_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.crs_label_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.north_arrow_rect.inside_sheet(self.width_mm, self.height_mm);
        if self.legend_kept_on_page {
            in_frame && self.legend_rect.inside_sheet(self.width_mm, self.height_mm)
        } else {
            in_frame
        }
    }

    /// C++ `PlateScaleBar::calculateDivisions(mapScale, targetWidthMm)` — 1-2-5 계열 정수 단위 축척막대 간격(m) 산출 1:1
    #[must_use]
    pub fn calculate_scale_bar_nice_interval_m(map_scale_denom: f64, target_width_mm: f64) -> f64 {
        if map_scale_denom <= 0.0 || target_width_mm <= 0.0 || !map_scale_denom.is_finite() {
            return 10.0;
        }
        let raw_meters = map_scale_denom * (target_width_mm / 1000.0);
        if raw_meters <= 1.0 {
            return 1.0;
        }
        let exp = raw_meters.log10().floor();
        let base = 10.0_f64.powf(exp);
        let frac = raw_meters / base;
        let nice_mult = if frac >= 5.0 {
            5.0
        } else if frac >= 2.0 {
            2.0
        } else {
            1.0
        };
        nice_mult * base
    }
}

/// C++ `RasterMaskClipper::clipRasterByPolygon` — 조사구역 내·외부 픽셀 알파 채널 마스킹(`inside=255`, `outside=round(outsideAlpha * 255)`) 1:1
pub fn apply_polygon_alpha_mask_rgba(
    rgba_buf: &mut [u8],
    width: usize,
    height: usize,
    inside_predicate: impl Fn(usize, usize) -> bool,
    outside_alpha_ratio: f32,
) {
    let out_a = (outside_alpha_ratio.clamp(0.0, 1.0) * 255.0).floor() as u8;
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            if idx + 3 < rgba_buf.len() {
                rgba_buf[idx + 3] = if inside_predicate(x, y) { 255 } else { out_a };
            }
        }
    }
}

/// C++ `NorthArrowRenderer::computeGridConvergenceAngle` — TM 투영계 중심 경위도 기준 도북-진북 자오선 수렴각(도 단위) 계산
#[must_use]
pub fn compute_grid_convergence_angle_deg(
    lon_deg: f64,
    lat_deg: f64,
    central_meridian_deg: f64,
) -> f64 {
    let dlon_rad = (lon_deg - central_meridian_deg).to_radians();
    let lat_rad = lat_deg.to_radians();
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    // GRS80 제2이심률 제곱 e'^2 ≈ 0.006739496742
    let eta2 = 0.006_739_496_742 * cos_lat * cos_lat;
    let gamma_rad = dlon_rad * sin_lat
        + (dlon_rad.powi(3) / 3.0) * sin_lat * cos_lat * cos_lat * (1.0 + 3.0 * eta2);
    gamma_rad.to_degrees()
}

/// C++ `LegendBoxComposer` — 고고학 도면 표준 범례 레이어 정렬 순서(1. 유물 점 -> 2. 기준점 -> 3. 단면선 -> 4. 유구 선 -> 5. 유구 면 -> 6. 트렌치 -> 7. 조사구역)
pub fn sort_legend_layers_canonical(layers: &mut [strata_types::schema::DomainLayerKey]) {
    use strata_types::schema::DomainLayerKey;
    let rank = |k: DomainLayerKey| -> u8 {
        match k {
            DomainLayerKey::ArtifactPoint => 1,
            DomainLayerKey::ControlPoints => 2,
            DomainLayerKey::SectionLine => 3,
            DomainLayerKey::FeatureLine => 4,
            DomainLayerKey::FeaturePoly => 5,
            DomainLayerKey::TrialTrench => 6,
            DomainLayerKey::SurveyArea => 7,
        }
    };
    layers.sort_by_key(|&k| rank(k));
}

/// C++ `TitleBlockComposer` (`QFontMetrics::elidedText(..., Qt::ElideRight)`) — 표제란 텍스트 우측 말줄임 처리
#[must_use]
pub fn elide_title_block_text_right(raw_text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let char_count = raw_text.chars().count();
    if char_count <= max_chars {
        return raw_text.to_owned();
    }
    let keep = max_chars.saturating_sub(1);
    let prefix: String = raw_text.chars().take(keep).collect();
    format!("{prefix}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_size_and_orientation_change_keeps_decorations_in_bounds_and_preserves_off_page_legend() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.all_decorations_inside_sheet());
        sheet.set_legend_kept_on_page(false);
        assert!(sheet.legend_rect.x_mm >= sheet.width_mm);

        sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Landscape);
        assert_eq!((sheet.width_mm, sheet.height_mm), (420.0, 297.0));
        assert!(sheet.all_decorations_inside_sheet());
        assert!(!sheet.legend_kept_on_page);
        assert!(sheet.legend_rect.x_mm >= sheet.width_mm);
    }

    #[test]
    fn paper_change_puts_chrome_back_in_bottom_strip_and_recovers_orphaned_legend() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
        // Drag chrome and legend to arbitrary/orphaned coordinates
        sheet.scale_bar_rect.x_mm = 380.0;
        sheet.scale_bar_rect.y_mm = 10.0;
        sheet.north_arrow_rect.x_mm = 5.0;
        sheet.north_arrow_rect.y_mm = 5.0;
        sheet.legend_rect.x_mm = 390.0;
        sheet.legend_rect.y_mm = 270.0;

        // Switch down to A4 Portrait
        sheet.change_paper(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.all_decorations_inside_sheet());
        let expected_band_y = sheet.height_mm - 12.0 - 22.0 + 2.0;
        assert!((sheet.scale_bar_rect.y_mm - expected_band_y).abs() < 1e-6);
        assert!((sheet.crs_label_rect.y_mm - expected_band_y).abs() < 1e-6);
        assert!((sheet.north_arrow_rect.y_mm - expected_band_y).abs() < 1e-6);
    }

    #[test]
    fn map_coordinate_grid_syncs_step_and_offset_only_when_north_up() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.apply_map_coordinate_grid(true, 0.0, 20.0, [222_385.0, 260_115.0]));
        assert!(sheet.grid_follow_map);
        assert!((sheet.grid_interval_m - 20.0).abs() < 1e-6);
        assert!((sheet.grid_offset_xy_m[0] - 5.0).abs() < 1e-6);
        assert!((sheet.grid_offset_xy_m[1] - 15.0).abs() < 1e-6);

        // Rotated map grid cannot be applied to sheet coordinate grid
        assert!(!sheet.apply_map_coordinate_grid(true, 15.0, 20.0, [0.0, 0.0]));
        assert!(!sheet.grid_follow_map);
    }

    #[test]
    fn test_plate_scale_bar_nice_interval_1200_at_60mm_yields_50m_and_raster_mask_alpha_255_76() {
        let interval = DrawingSheet::calculate_scale_bar_nice_interval_m(1_200.0, 60.0);
        assert!((interval - 50.0).abs() < 1e-6);

        let mut rgba = vec![200_u8; 4 * 2 * 4];
        apply_polygon_alpha_mask_rgba(&mut rgba, 2, 2, |x, y| x == 0 && y == 0, 0.3);
        assert_eq!(rgba[3], 255);
        assert_eq!(rgba[7], 76);
        assert_eq!(rgba[11], 76);
    }

    #[test]
    fn test_north_arrow_grid_convergence_and_legend_sort_and_title_block_elide() {
        use strata_types::schema::DomainLayerKey;

        // 중부원점(127.0E) 정중앙에서는 수렴각 0.0도, 128.0E(위도 36.0N)에서는 양(+)의 수렴각
        let zero_conv = compute_grid_convergence_angle_deg(127.0, 36.0, 127.0);
        assert!(zero_conv.abs() < 1e-9);
        let east_conv = compute_grid_convergence_angle_deg(128.0, 36.0, 127.0);
        assert!((east_conv - 36.0_f64.to_radians().sin()).abs() < 1e-3);

        let mut layers = vec![
            DomainLayerKey::SurveyArea,
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::ArtifactPoint,
            DomainLayerKey::TrialTrench,
            DomainLayerKey::FeatureLine,
        ];
        sort_legend_layers_canonical(&mut layers);
        assert_eq!(
            layers,
            vec![
                DomainLayerKey::ArtifactPoint,
                DomainLayerKey::FeatureLine,
                DomainLayerKey::FeaturePoly,
                DomainLayerKey::TrialTrench,
                DomainLayerKey::SurveyArea,
            ]
        );

        let elided = elide_title_block_text_right("경주 월성 발굴조사 지구 종합현황도", 10);
        assert_eq!(elided.chars().count(), 10);
        assert!(elided.ends_with('…'));
    }
}
