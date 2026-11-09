//! 유물 재질·3D 점의 단면 투영.
//!
//! `section_renderer` 에서 분리했다.

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
