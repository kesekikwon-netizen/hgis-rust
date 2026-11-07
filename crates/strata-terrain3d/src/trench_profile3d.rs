/// C++ `HGIS_TRENCH_PROFILE_SAMPLES` (100) — 트렌치 단면 기본 등간격 샘플 분할 수
pub const HGIS_TRENCH_PROFILE_SAMPLES: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchProfile3d {
    pub start_xy: [f64; 2],
    pub end_xy: [f64; 2],
    pub length_m: f64,
    pub azimuth_deg: f64,
    pub slope_deg: f64,
    pub samples_dist_elev: Vec<[f64; 2]>,
}

impl TrenchProfile3d {
    pub fn from_endpoints(start_xy: [f64; 2], end_xy: [f64; 2], start_z: f64, end_z: f64) -> Self {
        let dx = end_xy[0] - start_xy[0];
        let dy = end_xy[1] - start_xy[1];
        let length_m = (dx * dx + dy * dy).sqrt();
        let azimuth_deg = if length_m > 1e-6 {
            dx.atan2(dy).to_degrees().rem_euclid(360.0)
        } else {
            0.0
        };
        let slope_deg = if length_m > 1e-6 {
            (end_z - start_z).atan2(length_m).to_degrees()
        } else {
            0.0
        };
        let steps = 16usize;
        let mut samples_dist_elev = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = (i as f64) / (steps as f64);
            samples_dist_elev.push([length_m * t, start_z + (end_z - start_z) * t]);
        }
        Self {
            start_xy,
            end_xy,
            length_m,
            azimuth_deg,
            slope_deg,
            samples_dist_elev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trench_profile_3d_slope_angle_45deg_and_zero_length_fallback() {
        // TestKaTrenchProfile3D::testSlopeAngle45Deg: (0,0,100) -> (10,0,110) => 45.0 deg
        let p = TrenchProfile3d::from_endpoints([0.0, 0.0], [10.0, 0.0], 100.0, 110.0);
        assert!((p.length_m - 10.0).abs() < 1e-6);
        assert!((p.slope_deg - 45.0).abs() < 1e-6);
        assert!((p.azimuth_deg - 90.0).abs() < 1e-6);

        // Zero horizontal length fallback -> 0.0 deg slope (no NaN / 90 deg spike)
        let zero = TrenchProfile3d::from_endpoints([5.0, 5.0], [5.0, 5.0], 100.0, 125.0);
        assert_eq!(zero.length_m, 0.0);
        assert_eq!(zero.slope_deg, 0.0);

        // TestKaTerrainMeshExporter::testLocalOffsetCenteringAndNormals
        let dem = [50.0_f32, 50.0, 50.0, 50.0];
        let mesh = generate_terrain_mesh(
            &dem,
            2,
            2,
            200_000.0,
            500_000.0,
            5.0,
            HGIS_DEFAULT_Z_EXAGGERATION,
        );
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert!((mesh.vertices[0].x - (-2.5)).abs() <= 1e-6);
        assert!((mesh.vertices[0].y - (-2.5)).abs() <= 1e-6);
        assert!((mesh.vertices[1].x - 2.5).abs() <= 1e-6);
        assert!((mesh.vertices[1].y - (-2.5)).abs() <= 1e-6);
        assert!((mesh.vertices[2].x - (-2.5)).abs() <= 1e-6);
        assert!((mesh.vertices[2].y - 2.5).abs() <= 1e-6);
        assert!((mesh.vertices[3].x - 2.5).abs() <= 1e-6);
        assert!((mesh.vertices[3].y - 2.5).abs() <= 1e-6);
        for v in &mesh.vertices {
            assert!((v.nx - 0.0).abs() <= 1e-6);
            assert!((v.ny - 0.0).abs() <= 1e-6);
            assert!((v.nz - 1.0).abs() <= 1e-6);
        }
        assert_eq!((mesh.vertices[0].u, mesh.vertices[0].v), (0.0, 0.0));
        assert_eq!((mesh.vertices[1].u, mesh.vertices[1].v), (1.0, 0.0));
        assert_eq!((mesh.vertices[2].u, mesh.vertices[2].v), (0.0, 1.0));
        assert_eq!((mesh.vertices[3].u, mesh.vertices[3].v), (1.0, 1.0));
    }
}

/// 바이너리 glTF (`.glb`) 매직 헤더 (`"glTF"`, C++ `HGIS_GLTF_MAGIC_HEADER`)
pub const HGIS_GLTF_MAGIC_HEADER: u32 = 0x4654_6C67;
/// glTF 포맷 버전 (2.0, C++ `HGIS_GLTF_VERSION_2`)
pub const HGIS_GLTF_VERSION_2: u32 = 2;
/// 고고학 3D 지형 표고 기본 과장 배율 (C++ `HGIS_DEFAULT_Z_EXAGGERATION`)
pub const HGIS_DEFAULT_Z_EXAGGERATION: f32 = 1.0;

/// 3D 지형 메쉬 정점 (로컬 중심화 좌표 + 면 가중치 스무딩 노멀 + 정규화 UV, C++ `TerrainMeshVertex`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainMeshVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub u: f32,
    pub v: f32,
}

/// 3D 지형 메쉬 데이터 및 원점 복원 오프셋 (C++ `TerrainMeshData`)
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMeshData {
    pub vertices: Vec<TerrainMeshVertex>,
    pub indices: Vec<u32>,
    pub local_origin_offset: [f64; 3],
}

/// DEM 그리드로부터 로컬 오프셋 센터링, 평면 정규화 UV 및 면 가중치 스무딩 버텍스 노멀이 포함된 3D 지형 삼각망 메쉬 생성
/// (C++ `KaTerrainMeshExporter::generateTerrainMesh` 1:1 대응)
#[must_use]
pub fn generate_terrain_mesh(
    dem_grid: &[f32],
    width: usize,
    height: usize,
    origin_x: f64,
    origin_y: f64,
    cell_size_m: f64,
    vertical_exaggeration: f32,
) -> TerrainMeshData {
    if width < 2 || height < 2 || dem_grid.len() < width * height {
        return TerrainMeshData {
            vertices: Vec::new(),
            indices: Vec::new(),
            local_origin_offset: [origin_x, origin_y, 0.0],
        };
    }
    let span_x = (width - 1) as f64 * cell_size_m;
    let span_y = (height - 1) as f64 * cell_size_m;
    let center_x = origin_x + span_x * 0.5;
    let center_y = origin_y + span_y * 0.5;

    let mut valid_z_sum = 0.0_f64;
    let mut valid_z_cnt = 0_usize;
    for &z in &dem_grid[..width * height] {
        if z.is_finite() && z > -9000.0 {
            valid_z_sum += f64::from(z);
            valid_z_cnt += 1;
        }
    }
    let center_z = if valid_z_cnt > 0 {
        valid_z_sum / valid_z_cnt as f64
    } else {
        0.0
    };
    let z_exag = if vertical_exaggeration.is_finite() && vertical_exaggeration > 0.0 {
        vertical_exaggeration
    } else {
        HGIS_DEFAULT_Z_EXAGGERATION
    };

    let mut vertices = Vec::with_capacity(width * height);
    for r in 0..height {
        for c in 0..width {
            let wx = origin_x + c as f64 * cell_size_m;
            let wy = origin_y + r as f64 * cell_size_m;
            let raw_z = dem_grid[r * width + c];
            let lz = if raw_z.is_finite() && raw_z > -9000.0 {
                (f64::from(raw_z) - center_z) as f32 * z_exag
            } else {
                0.0
            };
            vertices.push(TerrainMeshVertex {
                x: (wx - center_x) as f32,
                y: (wy - center_y) as f32,
                z: lz,
                nx: 0.0,
                ny: 0.0,
                nz: 0.0,
                u: c as f32 / (width - 1) as f32,
                v: r as f32 / (height - 1) as f32,
            });
        }
    }

    let mut indices = Vec::with_capacity((width - 1) * (height - 1) * 6);
    for r in 0..(height - 1) {
        for c in 0..(width - 1) {
            let i00 = r * width + c;
            let i10 = r * width + (c + 1);
            let i01 = (r + 1) * width + c;
            let i11 = (r + 1) * width + (c + 1);
            if [dem_grid[i00], dem_grid[i10], dem_grid[i01], dem_grid[i11]]
                .iter()
                .any(|&z| !z.is_finite() || z <= -9000.0)
            {
                continue;
            }
            let tris = [[i00, i10, i11], [i00, i11, i01]];
            for &[a, b, d] in &tris {
                indices.push(a as u32);
                indices.push(b as u32);
                indices.push(d as u32);
                let va = vertices[a];
                let vb = vertices[b];
                let vd = vertices[d];
                let ux = vb.x - va.x;
                let uy = vb.y - va.y;
                let uz = vb.z - va.z;
                let vx = vd.x - va.x;
                let vy = vd.y - va.y;
                let vz = vd.z - va.z;
                let cx = uy * vz - uz * vy;
                let cy = uz * vx - ux * vz;
                let cz = ux * vy - uy * vx;
                for &vi in &[a, b, d] {
                    vertices[vi].nx += cx;
                    vertices[vi].ny += cy;
                    vertices[vi].nz += cz;
                }
            }
        }
    }

    for v in &mut vertices {
        let len = (v.nx * v.nx + v.ny * v.ny + v.nz * v.nz).sqrt();
        if len <= 1e-6 || !len.is_finite() {
            v.nx = 0.0;
            v.ny = 0.0;
            v.nz = 1.0;
        } else {
            v.nx /= len;
            v.ny /= len;
            v.nz /= len;
        }
    }

    TerrainMeshData {
        vertices,
        indices,
        local_origin_offset: [center_x, center_y, center_z],
    }
}

/// 측량 의심점 판정 고도차 임계치 (15.0 m, C++ `HGIS_SUSPECT_ELEVATION_DIFF_M`)
pub const HGIS_SUSPECT_ELEVATION_DIFF_M: f64 = 15.0;
/// TIN 삼각망 최대 허용 변 길이 (50.0 m, C++ `HGIS_MAX_TIN_EDGE_LENGTH_M`)
pub const HGIS_MAX_TIN_EDGE_LENGTH_M: f64 = 50.0;
/// 등고선 계산 격자 최대 셀 예산 (4,000,000, C++ `SurveyContourBuilder::kMaxCells`)
pub const HGIS_MAX_CONTOUR_GRID_CELLS: i64 = 4_000_000;
/// 단절선(Breakline) 세분화 정점 최대 허용 상한 (200,000, C++ `kMaxBreaklineNodes`)
pub const HGIS_MAX_BREAKLINE_NODES: usize = 200_000;

/// 3차원 측량점 및 의심점 판정 플래그 (C++ `SurveyPoint3D` / `SurveyPoint`)
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyPoint3D {
    pub fid: i64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub code: String,
    pub is_suspect: bool,
}

/// 주변 이웃점 평균 고도 편차 기반 의심점(Spike/Pit) 자동 필터링
/// (C++ `SurveyPointArrange::filterSuspectPoints` 1:1 대응)
pub fn filter_suspect_points(points: &mut [SurveyPoint3D], threshold_z: f64) {
    if points.len() < 3 {
        return;
    }
    let eff_thresh = threshold_z.max(0.5);
    let snapshot: Vec<(f64, f64, f64)> = points.iter().map(|p| (p.x, p.y, p.z)).collect();
    for (i, pt) in points.iter_mut().enumerate() {
        let mut neighbors_z = Vec::new();
        for (j, &(nx, ny, nz)) in snapshot.iter().enumerate() {
            if i == j || !nz.is_finite() {
                continue;
            }
            let dist = (pt.x - nx).hypot(pt.y - ny);
            if dist <= 100.0 {
                neighbors_z.push((dist, nz));
            }
        }
        neighbors_z.sort_by(|a, b| a.0.total_cmp(&b.0));
        let k_slice = &neighbors_z[..neighbors_z.len().min(8)];
        if k_slice.len() >= 2 {
            // 중앙값(median) 기반 로버스트 평균으로 단일 스파이크 오염을 방지
            let mut zs: Vec<f64> = k_slice.iter().map(|&(_, z)| z).collect();
            zs.sort_by(|a, b| a.total_cmp(b));
            let median_z = zs[zs.len() / 2];
            if (pt.z - median_z).abs() > eff_thresh {
                pt.is_suspect = true;
            }
        }
    }
}

/// 측량점 삼각망 생성 가능 여부 사전 검증 (`SurveyContourBuilder::rejection` 1:1 대응)
#[must_use]
pub fn survey_contour_rejection(points: &[SurveyPoint3D]) -> Option<&'static str> {
    let valid: Vec<&SurveyPoint3D> = points.iter().filter(|p| !p.is_suspect).collect();
    if valid.len() < 3 {
        return Some("등고선을 만들려면 점이 3개 이상 필요합니다.");
    }
    let origin = valid[0];
    let mut span = 0.0_f64;
    let mut far_idx = 0usize;
    for (idx, pt) in valid.iter().enumerate().skip(1) {
        let d = (pt.x - origin.x).hypot(pt.y - origin.y);
        if d > span {
            span = d;
            far_idx = idx;
        }
    }
    if span < 1e-6 {
        return Some("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    let axis = valid[far_idx];
    let mut max_cross = 0.0_f64;
    for pt in &valid {
        let cross = ((axis.x - origin.x) * (pt.y - origin.y)
            - (axis.y - origin.y) * (pt.x - origin.x))
            .abs();
        max_cross = max_cross.max(cross);
    }
    if max_cross < 0.05 * span {
        return Some("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    None
}

/// 측량점 간 최근접 거리 중앙값의 절반으로 기본 계산 격자 크기(`[0.05, 1.0]` m) 자동 산출
/// (C++ `SurveyContourBuilder::autoCellSize` 1:1 대응)
#[must_use]
pub fn auto_contour_cell_size(points: &[SurveyPoint3D]) -> f64 {
    let count = points.len();
    if count < 2 {
        return 0.5;
    }
    let sample = count.min(300);
    let step = (count / sample).max(1);
    let mut nearest = Vec::new();
    for i in (0..count).step_by(step) {
        let mut best = f64::INFINITY;
        for (j, pj) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            let d = (points[i].x - pj.x).hypot(points[i].y - pj.y);
            if d > 1e-6 && d < best {
                best = d;
            }
        }
        if best.is_finite() {
            nearest.push(best);
        }
    }
    if nearest.is_empty() {
        return 0.5;
    }
    nearest.sort_by(|a, b| a.total_cmp(b));
    (nearest[nearest.len() / 2] * 0.5).clamp(0.05, 1.0)
}

/// 최대 셀 예산(`max_cells = 4,000,000`)을 초과하지 않도록 격자 크기를 1cm 단위로 확대 보정
/// (C++ `SurveyContourBuilder::fitCellSize` 1:1 대응)
#[must_use]
pub fn fit_contour_cell_size(cell: f64, span_x: f64, span_y: f64, max_cells: i64) -> f64 {
    if !cell.is_finite()
        || cell <= 0.0
        || !span_x.is_finite()
        || !span_y.is_finite()
        || max_cells < 4
    {
        return cell;
    }
    let cells_for = |size: f64| -> i64 {
        let nx = (span_x / size + 1.0).ceil() as i64;
        let ny = (span_y / size + 1.0).ceil() as i64;
        nx.saturating_mul(ny)
    };
    if cells_for(cell) <= max_cells {
        return cell;
    }
    let mut fitted = cell.max(((span_x.max(1e-3) * span_y.max(1e-3)) / (max_cells as f64)).sqrt());
    fitted = (fitted * 100.0).ceil() / 100.0;
    while cells_for(fitted) > max_cells {
        fitted = ((fitted + 0.01) * 100.0).round() / 100.0;
    }
    fitted
}

/// 3D 단절선(Breakline) 세분화 정점 생성 및 1mm 이내 기존 측량점 중복 제거
/// (C++ `densifySurveyBreaklines` 1:1 대응)
#[must_use]
pub fn densify_survey_breaklines(
    lines: &[Vec<SurveyPoint3D>],
    spacing: f64,
    existing: &[SurveyPoint3D],
) -> Vec<SurveyPoint3D> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut total_len = 0.0_f64;
    for line in lines {
        for w in line.windows(2) {
            total_len += (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
        }
    }
    let step = (if spacing > 0.0 { spacing } else { 0.5 })
        .max(total_len / (HGIS_MAX_BREAKLINE_NODES as f64))
        .max(1e-3);
    let mm_key = |x: f64, y: f64| -> (i64, i64) {
        ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64)
    };
    let mut taken: std::collections::BTreeSet<(i64, i64)> =
        existing.iter().map(|p| mm_key(p.x, p.y)).collect();
    let mut added = Vec::new();
    let mut push_node = |x: f64, y: f64, z: f64| {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return;
        }
        let key = mm_key(x, y);
        if taken.insert(key) {
            added.push(SurveyPoint3D {
                fid: -1,
                x,
                y,
                z,
                code: "BREAKLINE".to_owned(),
                is_suspect: false,
            });
        }
    };
    for line in lines {
        for (i, a) in line.iter().enumerate() {
            push_node(a.x, a.y, a.z);
            if let Some(b) = line.get(i + 1) {
                let pieces = ((b.x - a.x).hypot(b.y - a.y) / step).floor() as usize;
                for k in 1..pieces {
                    let t = (k as f64) / (pieces as f64);
                    push_node(
                        a.x + t * (b.x - a.x),
                        a.y + t * (b.y - a.y),
                        a.z + t * (b.z - a.z),
                    );
                }
            }
        }
    }
    added
}

#[cfg(test)]
mod batch56_contour_tests {
    use super::*;

    #[test]
    fn test_suspect_point_filtering_and_survey_contour_tin_builder() {
        // TestSurveyPointArrange::testSuspectPointFiltering:
        // 10 flat ground points at Z = 30.0m + 1 spike at Z = 55.0m (diff 25.0m > 15.0m)
        let mut pts = Vec::new();
        for i in 0..10 {
            pts.push(SurveyPoint3D {
                fid: i + 1,
                x: (i % 5) as f64 * 5.0,
                y: (i / 5) as f64 * 5.0,
                z: 30.0,
                code: "G".to_owned(),
                is_suspect: false,
            });
        }
        let bad_idx = 5usize;
        pts[bad_idx].z = 55.0;

        filter_suspect_points(&mut pts, HGIS_SUSPECT_ELEVATION_DIFF_M);
        assert!(pts[bad_idx].is_suspect);
        assert!(pts
            .iter()
            .enumerate()
            .filter(|&(idx, _)| idx != bad_idx)
            .all(|(_, p)| !p.is_suspect));

        // Collinear / <3 points rejection check
        assert_eq!(
            survey_contour_rejection(&pts[..2]),
            Some("등고선을 만들려면 점이 3개 이상 필요합니다.")
        );
        assert_eq!(survey_contour_rejection(&pts), None);

        // Breakline densification & cell fitting
        let line = vec![
            SurveyPoint3D {
                fid: 100,
                x: 0.0,
                y: 0.0,
                z: 30.0,
                code: "T".to_owned(),
                is_suspect: false,
            },
            SurveyPoint3D {
                fid: 101,
                x: 2.0,
                y: 0.0,
                z: 31.0,
                code: "T".to_owned(),
                is_suspect: false,
            },
        ];
        let densified = densify_survey_breaklines(&[line], 0.5, &pts);
        assert_eq!(densified.len(), 4); // (0,0) skipped because in `pts`, 0.5, 1.0, 1.5, 2.0 added
        let fitted = fit_contour_cell_size(0.1, 1000.0, 1000.0, HGIS_MAX_CONTOUR_GRID_CELLS);
        assert!(fitted >= 0.5);
    }
}
