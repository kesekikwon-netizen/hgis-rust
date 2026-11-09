//! 범례 정렬·표제란 줄임·알파 마스크·투영 수렴각·삽도 상자.
//!
//! `sheet_composer` 에서 분리했다.

use crate::sheet_geometry::SheetRectMm;

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

pub const HGIS_TITLE_BLOCK_DEFAULT_W_MM: f64 = 180.0;

pub const HGIS_TITLE_BLOCK_ROW_H_MM: f64 = 8.0;

pub const HGIS_MIN_INSET_BOX_SIZE_MM: f64 = 3.0;

pub const HGIS_INSET_CROSSHAIR_LEN_MM: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InsetExtentBoxMm {
    pub rect_mm: SheetRectMm,
    pub has_crosshair: bool,
    pub crosshair_len_mm: f64,
}

/// C++ `KaInsetMapItem` / `KaOverviewMapSync` — 광역 위치도 내 메인 지도 영역 사각형 최소 크기(`3.0mm × 3.0mm`) 보장 클램핑 및 중심 십자선(`2.0mm`) 표시
#[must_use]
pub fn compute_inset_overview_box_mm(projected_rect_mm: SheetRectMm) -> InsetExtentBoxMm {
    let cx = projected_rect_mm.x_mm + projected_rect_mm.width_mm * 0.5;
    let cy = projected_rect_mm.y_mm + projected_rect_mm.height_mm * 0.5;
    let clamped_w = projected_rect_mm.width_mm.max(HGIS_MIN_INSET_BOX_SIZE_MM);
    let clamped_h = projected_rect_mm.height_mm.max(HGIS_MIN_INSET_BOX_SIZE_MM);
    let has_crosshair = projected_rect_mm.width_mm <= HGIS_MIN_INSET_BOX_SIZE_MM
        || projected_rect_mm.height_mm <= HGIS_MIN_INSET_BOX_SIZE_MM;
    InsetExtentBoxMm {
        rect_mm: SheetRectMm {
            x_mm: cx - clamped_w * 0.5,
            y_mm: cy - clamped_h * 0.5,
            width_mm: clamped_w,
            height_mm: clamped_h,
        },
        has_crosshair,
        crosshair_len_mm: HGIS_INSET_CROSSHAIR_LEN_MM,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegendLayerCandidate {
    pub layer_key: strata_types::schema::DomainLayerKey,
    pub visible: bool,
    pub feature_count: usize,
}

/// C++ `KaLegendItem::collectVisibleLegendEntries` — 가시(`visible == true`) 및 비어 있지 않은(`featureCount > 0`) 레이어만 범례 항목으로 수집하고 표준 순서로 정렬
#[must_use]
pub fn collect_visible_non_empty_legend_entries(
    candidates: &[LegendLayerCandidate],
) -> Vec<strata_types::schema::DomainLayerKey> {
    let mut keys: Vec<strata_types::schema::DomainLayerKey> = candidates
        .iter()
        .filter(|c| c.visible && c.feature_count > 0)
        .map(|c| c.layer_key)
        .collect();
    sort_legend_layers_canonical(&mut keys);
    keys
}

/// C++ `LayoutService::niceScaleBarSegmentMeters(double mapWidthMm, double scaleDenominator, int segments)` 1:1 대응
#[must_use]
pub fn nice_scale_bar_segment_meters(
    map_width_mm: f64,
    scale_denominator: f64,
    segments: u32,
) -> f64 {
    let segs = f64::from(segments.max(1));
    if !map_width_mm.is_finite()
        || !scale_denominator.is_finite()
        || map_width_mm <= 0.0
        || scale_denominator <= 0.0
    {
        return 1.0;
    }
    let target_bar_mm = (map_width_mm * 0.35).clamp(40.0, 160.0);
    let raw_seg = (target_bar_mm / 1000.0 * scale_denominator) / segs;
    const K_LEN: [f64; 19] = [
        0.1, 0.2, 0.4, 0.5, 1.0, 2.0, 4.0, 5.0, 10.0, 20.0, 40.0, 50.0, 100.0, 200.0, 400.0, 500.0,
        1000.0, 2000.0, 5000.0,
    ];
    let mut best = K_LEN[0];
    let mut best_rel = f64::INFINITY;
    for &n in &K_LEN {
        let rel = (raw_seg - n).abs() / n;
        if rel < best_rel {
            best_rel = rel;
            best = n;
        }
    }
    best
}

/// C++ `LayoutService::niceGridIntervalMeters(double scaleDenominator, double mapWidthMm)` 1:1 대응
#[must_use]
pub fn nice_grid_interval_meters(scale_denominator: f64, map_width_mm: f64) -> f64 {
    if !scale_denominator.is_finite() || scale_denominator <= 0.0 {
        return 100.0;
    }
    let paper_mm = if map_width_mm > 0.0 && map_width_mm.is_finite() {
        map_width_mm
    } else {
        180.0
    };
    let target_mm = (paper_mm * 0.24).clamp(25.0, 70.0);
    let raw_m = target_mm / 1000.0 * scale_denominator;
    const K_STEPS: [f64; 18] = [
        0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0,
        2000.0, 5000.0, 10000.0,
    ];
    let mut best = K_STEPS[0];
    let mut best_rel = f64::INFINITY;
    for &s in &K_STEPS {
        let rel = (raw_m - s).abs() / s;
        if rel < best_rel {
            best_rel = rel;
            best = s;
        }
    }
    best
}

/// C++ `KaTitleBlock::text` (`target/cpp-ref/src/app/KaTitleBlock.cpp:57-65`) — 표제란 5행 생성
/// `도면명 / 조사명 / 유적명 / 축  척 / 작성일`, 빈 값은 `―`(U+2015)로 채움 (`orDash`)
#[must_use]
pub fn format_title_block_lines(
    drawing_title: &str,
    survey_name: &str,
    site_name: &str,
    scale_denominator: u32,
    date_ymd: &str,
) -> [String; 5] {
    let or_dash = |s: &str| -> String {
        let t = s.trim();
        if t.is_empty() {
            "―".to_owned()
        } else {
            t.to_owned()
        }
    };
    let scale_str = if scale_denominator > 0 {
        format!("1 : {scale_denominator}")
    } else {
        "1 : ―".to_owned()
    };
    [
        format!("도면명   {}", or_dash(drawing_title)),
        format!("조사명   {}", or_dash(survey_name)),
        format!("유적명   {}", or_dash(site_name)),
        format!("축  척   {scale_str}"),
        format!("작성일   {}", or_dash(date_ymd)),
    ]
}
