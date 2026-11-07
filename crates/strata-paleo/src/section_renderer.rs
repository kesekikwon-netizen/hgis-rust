use crate::pipeline::CrossSectionProfile;

/// C++ `HGIS_MIN_SECTION_VERT_RANGE_M` (10.0m) — 완전 평지(`z_max == z_min`) 시 최소 수직 표고 범위 보장
pub const HGIS_MIN_SECTION_VERT_RANGE_M: f32 = 10.0;

/// C++ `KaSectionRenderer2D` 4대 표준 지층 및 지하수위 RGB 색상 상수 (`kyi/main`)
pub const HGIS_STRATUM_FILL_COLOR: [u8; 3] = [194, 158, 120]; // 성토층 (Warm Earth)
pub const HGIS_STRATUM_ALLUVIUM_COLOR: [u8; 3] = [222, 203, 166]; // 충적층 (Sand/Silt)
pub const HGIS_STRATUM_WEATHERED_COLOR: [u8; 3] = [168, 160, 144]; // 풍화암 (Grayish Khaki)
pub const HGIS_STRATUM_BEDROCK_COLOR: [u8; 3] = [120, 115, 108]; // 기반암 (Dark Stone Gray)
pub const HGIS_WATER_TABLE_COLOR: [u8; 3] = [41, 128, 185]; // 지하수위 (Sky Blue)

/// C++ `KaSectionRenderer2D::renderSection` 수직 범위 정규화:
/// `z_max - z_min < 10.0m` 인 평지 단면일 때 중심 표고를 기준으로 상하 `±5.0m` 확장하여 최소 `10.0m` 수직 범위를 보장한다.
#[must_use]
pub fn compute_section_vertical_bounds(z_values_m: &[f32]) -> (f32, f32) {
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    for &z in z_values_m {
        if z.is_finite() {
            z_min = z_min.min(z);
            z_max = z_max.max(z);
        }
    }
    if !z_min.is_finite() || !z_max.is_finite() {
        return (0.0, HGIS_MIN_SECTION_VERT_RANGE_M);
    }
    if (z_max - z_min) < HGIS_MIN_SECTION_VERT_RANGE_M {
        let center = 0.5 * (z_min + z_max);
        let half = 0.5 * HGIS_MIN_SECTION_VERT_RANGE_M;
        (center - half, center + half)
    } else {
        (z_min, z_max)
    }
}

/// C++ `KaSectionRenderer2D::renderSection` 표고 `z` -> 화면 픽셀 `Y` 변환 수식 (0 나눗셈/NaN 방어 포함)
#[must_use]
pub fn section_elevation_to_pixel_y(
    z_m: f32,
    z_min_m: f32,
    z_max_m: f32,
    viewport_height_px: f32,
    top_padding_px: f32,
) -> f32 {
    let span = (z_max_m - z_min_m).max(HGIS_MIN_SECTION_VERT_RANGE_M);
    let norm = ((z_max_m - z_m) / span).clamp(0.0, 1.0);
    top_padding_px + norm * viewport_height_px.max(1.0)
}

pub const HGIS_CROSS_VIEW_THROTTLE_MS: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiElevationProbeResult {
    pub has_valid_sample: bool,
    pub current_elevation_m: f64,
    pub paleo_elevation_m: f64,
    pub fill_depth_m: f64,
    pub section_distance_d: f64,
}

/// C++ `KaCrossViewCursorSync` — 2D 월드 좌표 `world_xy`를 단면선 `[start, end]` 위로 내적 투영하여 거리 `d ∈ [0, L]` 산출 (`L <= 1e-6` 시 0.0 반환)
#[must_use]
pub fn project_world_coord_to_section_distance(
    world_xy: [f64; 2],
    section_start_xy: [f64; 2],
    section_end_xy: [f64; 2],
) -> f64 {
    let vx = section_end_xy[0] - section_start_xy[0];
    let vy = section_end_xy[1] - section_start_xy[1];
    let len = vx.hypot(vy);
    if len <= 1e-6 {
        return 0.0;
    }
    let ux = world_xy[0] - section_start_xy[0];
    let uy = world_xy[1] - section_start_xy[1];
    let proj = (ux * vx + uy * vy) / len;
    proj.clamp(0.0, len)
}

/// C++ `KaElevationTooltipProbe::probeAtWorldCoord` — 단면선 투영 거리 `d` 및 현 지표고·고지표고·성토심(`fillDepthM = Z_curr - Z_paleo`) 동시 산출
#[must_use]
pub fn probe_multi_elevation_at_world_coord(
    world_xy: [f64; 2],
    section_start_xy: [f64; 2],
    section_end_xy: [f64; 2],
    current_elevation_m: Option<f64>,
    paleo_elevation_m: Option<f64>,
) -> MultiElevationProbeResult {
    let section_distance_d =
        project_world_coord_to_section_distance(world_xy, section_start_xy, section_end_xy);
    match (current_elevation_m, paleo_elevation_m) {
        (Some(zc), Some(zp)) if zc.is_finite() && zp.is_finite() && zc > -9000.0 && zp > -9000.0 => {
            MultiElevationProbeResult {
                has_valid_sample: true,
                current_elevation_m: zc,
                paleo_elevation_m: zp,
                fill_depth_m: zc - zp,
                section_distance_d,
            }
        }
        _ => MultiElevationProbeResult {
            has_valid_sample: false,
            current_elevation_m: f64::NAN,
            paleo_elevation_m: f64::NAN,
            fill_depth_m: 0.0,
            section_distance_d,
        },
    }
}

#[must_use]
pub fn render_cross_section_rgba(
    profile: &CrossSectionProfile,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let w = width.max(16) as usize;
    let h = height.max(16) as usize;
    // 기본 종이 배경 (#FAF6EE)
    let mut buf = vec![0_u8; w * h * 4];
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&[250, 246, 238, 255]);
    }
    let n = profile.dist_m.len();
    if n < 2 {
        return buf;
    }

    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for (&zs, (&zp, &zb)) in profile
        .z_surface_curr_m
        .iter()
        .zip(profile.z_paleo_top_m.iter().zip(profile.z_bedrock_base_m.iter()))
    {
        min_z = min_z.min(zb).min(zp).min(zs);
        max_z = max_z.max(zs).max(zp).max(zb);
    }
    let z_span = (max_z - min_z).max(2.0);
    let z_to_y = |z: f64| -> usize {
        let norm = ((max_z - z) / z_span).clamp(0.05, 0.95);
        (norm * (h as f64)) as usize
    };

    for col in 0..w {
        let t = (col as f64 / (w.saturating_sub(1).max(1) as f64)) * ((n - 1) as f64);
        let idx = (t.floor() as usize).min(n - 2);
        let frac = t - (idx as f64);
        let lerp = |a: f64, b: f64| a + (b - a) * frac;

        let zs = lerp(profile.z_surface_curr_m[idx], profile.z_surface_curr_m[idx + 1]);
        let zp = lerp(profile.z_paleo_top_m[idx], profile.z_paleo_top_m[idx + 1]);
        let zb = lerp(profile.z_bedrock_base_m[idx], profile.z_bedrock_base_m[idx + 1]);

        let ys = z_to_y(zs).min(h - 1);
        let yp = z_to_y(zp).min(h - 1);
        let yb = z_to_y(zb).min(h - 1);

        for row in ys..h {
            let color = if row < yp {
                [156, 163, 175, 255] // 매립·표토층 (회갈색)
            } else if row < yb {
                [217, 119, 6, 230] // 충적·고지형 퇴적층 (황토색)
            } else {
                [120, 113, 108, 255] // 풍화대·기반암 (암회색)
            };
            let off = (row * w + col) * 4;
            buf[off..off + 4].copy_from_slice(&color);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_section_rgba_renderer_draws_three_stratigraphic_horizons() {
        let profile = CrossSectionProfile {
            dist_m: vec![0.0, 50.0, 100.0],
            z_surface_curr_m: vec![45.0, 44.5, 44.0],
            z_paleo_top_m: vec![43.0, 42.5, 42.0],
            z_bedrock_base_m: vec![38.0, 37.5, 37.0],
        };
        let rgba = render_cross_section_rgba(&profile, 240, 120);
        assert_eq!(rgba.len(), 240 * 120 * 4);
    }

    #[test]
    fn test_flat_surface_min_range_10m_and_four_stratum_palette_constants() {
        // TestKaSectionRenderer2D::testFlatSurfaceMinRange:
        // Flat surface at 30.0m -> expanded to [25.0m, 35.0m] (10.0m span), no NaN or division by zero
        let flat_profile = [30.0_f32, 30.0, 30.0, 30.0];
        let (z_min, z_max) = compute_section_vertical_bounds(&flat_profile);
        assert!((z_max - z_min - HGIS_MIN_SECTION_VERT_RANGE_M).abs() < 1e-6);
        assert!((z_min - 25.0).abs() < 1e-6);
        assert!((z_max - 35.0).abs() < 1e-6);

        let y_mid = section_elevation_to_pixel_y(30.0, z_min, z_max, 200.0, 10.0);
        assert!(y_mid.is_finite());
        assert!((y_mid - 110.0).abs() < 1e-5);

        assert_eq!(HGIS_STRATUM_FILL_COLOR, [194, 158, 120]);
        assert_eq!(HGIS_STRATUM_ALLUVIUM_COLOR, [222, 203, 166]);
        assert_eq!(HGIS_STRATUM_WEATHERED_COLOR, [168, 160, 144]);
        assert_eq!(HGIS_STRATUM_BEDROCK_COLOR, [120, 115, 108]);
        assert_eq!(HGIS_WATER_TABLE_COLOR, [41, 128, 185]);

        // TestKaCrossViewCursorSync::testSectionProjectionDistance & TestKaElevationTooltipProbe::testFillDepthCalculation
        let probe = probe_multi_elevation_at_world_coord(
            [45.0, 20.0],
            [0.0, 0.0],
            [100.0, 0.0],
            Some(32.50),
            Some(29.30),
        );
        assert!(probe.has_valid_sample);
        assert!((probe.section_distance_d - 45.0).abs() < 1e-6);
        assert!((probe.fill_depth_m - 3.20).abs() < 1e-6);

        // TestKaArtifactSectionProjector::testBufferWidthFilteringAndProjection
        let artifacts = vec![
            ArtifactPoint3D {
                fid: 1,
                x: 50.0,
                y: 1.5,
                z: 35.0,
                material: ArtifactMaterial::Pottery,
                artifact_no: "토기-1".to_string(),
            },
            ArtifactPoint3D {
                fid: 2,
                x: 50.0,
                y: 2.5,
                z: 36.0,
                material: ArtifactMaterial::Stone,
                artifact_no: "석기-2".to_string(),
            },
        ];
        let projected = project_artifacts_to_section(
            &artifacts,
            [0.0, 0.0],
            [100.0, 0.0],
            HGIS_DEFAULT_SECTION_BUFFER_M,
        );
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].fid, 1);
        assert!((projected[0].d_proj_m - 50.0).abs() < 1e-6);
        assert!((projected[0].elevation_z - 35.0).abs() < 1e-6);
        assert!((projected[0].d_perp_m - 1.5).abs() < 1e-6);
    }
}

/// 유물 단면 투영 기본 완충 폭 (m, C++ `HGIS_DEFAULT_SECTION_BUFFER_M`)
pub const HGIS_DEFAULT_SECTION_BUFFER_M: f64 = 2.0;
/// 단면도 유물 심볼 표준 지름/크기 (mm, C++ `HGIS_ARTIFACT_SYMBOL_SIZE_MM`)
pub const HGIS_ARTIFACT_SYMBOL_SIZE_MM: f64 = 3.0;

/// 출토 유물 재질 분류 및 단면 심볼 매핑 (C++ `ArtifactMaterial`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactMaterial {
    /// 토기/도기 (원형 심볼, 적갈색 `#D35400`)
    Pottery,
    /// 석기/석제품 (마름모, 회색 `#7F8C8D`)
    Stone,
    /// 철기/금속기 (정삼각형, 암갈색 `#2C3E50`)
    Iron,
    /// 기와/전돌 (정사각형, 흑회색 `#34495E`)
    RoofTile,
    /// 골각기 (별표, 상아색 `#F5EEF8`)
    Bone,
    /// 기타 (십자 마커, 청록색 `#16A085`)
    Other,
}

impl ArtifactMaterial {
    /// 재질별 표준 RGB 색상 반환
    #[must_use]
    pub fn symbol_rgb(self) -> [u8; 3] {
        match self {
            Self::Pottery => [0xD3, 0x54, 0x00],
            Self::Stone => [0x7F, 0x8C, 0x8D],
            Self::Iron => [0x2C, 0x3E, 0x50],
            Self::RoofTile => [0x34, 0x49, 0x5E],
            Self::Bone => [0xF5, 0xEE, 0xF8],
            Self::Other => [0x16, 0xA0, 0x85],
        }
    }
}

/// 3차원 측량 출토 유물 입력 구조체
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactPoint3D {
    pub fid: i64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub material: ArtifactMaterial,
    pub artifact_no: String,
}

/// 단면선으로 수직 입면 투영된 유물 점 (C++ `ProjectedArtifactPoint`)
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedArtifactPoint {
    pub fid: i64,
    pub d_proj_m: f64,
    pub elevation_z: f64,
    pub d_perp_m: f64,
    pub material: ArtifactMaterial,
    pub artifact_no: String,
}

/// 단면 기준선으로부터 수직 이격거리 `d_perp <= buffer_width_m` 이내 유물만 단면 좌표 `(d_proj_m, elevation_z)`로 투영
/// (C++ `KaArtifactSectionProjector::projectArtifactsToSection` 1:1 대응)
#[must_use]
pub fn project_artifacts_to_section(
    artifacts: &[ArtifactPoint3D],
    section_start_xy: [f64; 2],
    section_end_xy: [f64; 2],
    buffer_width_m: f64,
) -> Vec<ProjectedArtifactPoint> {
    let vx = section_end_xy[0] - section_start_xy[0];
    let vy = section_end_xy[1] - section_start_xy[1];
    let len = vx.hypot(vy);
    if len < 1e-6 || !len.is_finite() {
        return Vec::new();
    }
    let bw = buffer_width_m.max(0.0);
    artifacts
        .iter()
        .filter_map(|art| {
            let ux = art.x - section_start_xy[0];
            let uy = art.y - section_start_xy[1];
            let d_perp_m = (ux * vy - uy * vx).abs() / len;
            let d_proj_m = (ux * vx + uy * vy) / len;
            if d_perp_m <= bw && d_proj_m >= 0.0 && d_proj_m <= len {
                Some(ProjectedArtifactPoint {
                    fid: art.fid,
                    d_proj_m,
                    elevation_z: art.z,
                    d_perp_m,
                    material: art.material,
                    artifact_no: art.artifact_no.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}
