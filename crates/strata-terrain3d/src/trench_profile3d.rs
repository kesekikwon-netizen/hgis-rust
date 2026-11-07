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

/// 지형 단면도 기본 수직 과장 배율 (5.0배, C++ `HGIS_PROFILE_Z_EXAGGERATION`)
pub const HGIS_PROFILE_Z_EXAGGERATION: f64 = 5.0;
/// DEM 기본 급간 수 (7단계, C++ `HGIS_DEFAULT_DEM_CLASS_COUNT`)
pub const HGIS_DEFAULT_DEM_CLASS_COUNT: usize = 7;

/// DEM 표고 급간 분류 모드 (C++ `ElevationClassMode`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationClassMode {
    EqualInterval,
    Quantile,
    CustomBreaks,
}

/// DEM 급간 색상 및 범례 항목 (C++ `DemClassInterval`)
#[derive(Debug, Clone, PartialEq)]
pub struct DemClassInterval {
    pub min_elev_m: f64,
    pub max_elev_m: f64,
    pub color_rgba: [u8; 4],
    pub label: String,
}

/// 지형 단면 프로파일 샘플링 점 (C++ `ElevationProfilePoint`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationProfilePoint {
    pub chainage_m: f64,
    pub elevation_m: f64,
    pub slope_deg: f64,
}

/// DEM 고도 구간에 대해 비선형 표준 고도 눈금(`{0, 100, 200, 500, 1000, 1500, 2000}`)과 `5..=7`단계 범례 눈금을 생성
/// (C++ `DemColorRampLegend::buildDemLegendTicks` 1:1 대응)
#[must_use]
pub fn build_dem_legend_ticks(minimum: f64, maximum: f64) -> Vec<f64> {
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return Vec::new();
    }
    const STANDARD_STOPS: [f64; 7] = [0.0, 100.0, 200.0, 500.0, 1000.0, 1500.0, 2000.0];
    let mut ticks = vec![minimum];
    for &s in &STANDARD_STOPS {
        if s > minimum + 1.0 && s < maximum - 1.0 {
            ticks.push(s);
        }
    }
    ticks.push(maximum);

    if ticks.len() < 5 {
        let mut even = Vec::with_capacity(5);
        for i in 0..5 {
            let t = f64::from(i) / 4.0;
            even.push(minimum + (maximum - minimum) * t);
        }
        return even;
    }
    if ticks.len() > HGIS_DEFAULT_DEM_CLASS_COUNT {
        let mut thinned = Vec::with_capacity(HGIS_DEFAULT_DEM_CLASS_COUNT);
        thinned.push(ticks[0]);
        let inner = HGIS_DEFAULT_DEM_CLASS_COUNT - 2;
        for i in 1..=inner {
            let pos = (i as f64) * ((ticks.len() - 1) as f64) / ((inner + 1) as f64);
            let idx = (pos.round() as usize).clamp(1, ticks.len() - 2);
            if thinned
                .last()
                .is_none_or(|&last| (last - ticks[idx]).abs() > 1e-6)
            {
                thinned.push(ticks[idx]);
            }
        }
        thinned.push(*ticks.last().unwrap_or(&maximum));
        return thinned;
    }
    ticks
}

/// 2D DEM 격자에서 단면 절단선(`section_line_pts`)을 따라 `sample_step_m` 간격으로 양선형(Bilinear) 표고 및 경사도 프로파일 추출
/// (C++ `DemPresentation::sampleElevationProfile` 1:1 대응)
#[must_use]
pub fn sample_elevation_profile_bilinear(
    dem_grid: &[f32],
    cols: usize,
    rows: usize,
    cell_size_m: f64,
    origin_xy: [f64; 2],
    section_line_pts: &[[f64; 2]],
    sample_step_m: f64,
) -> Vec<ElevationProfilePoint> {
    if dem_grid.len() < cols.saturating_mul(rows)
        || cols == 0
        || rows == 0
        || !cell_size_m.is_finite()
        || cell_size_m <= 0.0
        || section_line_pts.len() < 2
    {
        return Vec::new();
    }
    let step = if sample_step_m.is_finite() && sample_step_m > 1e-3 {
        sample_step_m
    } else {
        1.0
    };

    let sample_at = |wx: f64, wy: f64| -> f64 {
        let gx = ((wx - origin_xy[0]) / cell_size_m).clamp(0.0, (cols - 1) as f64);
        let gy = ((wy - origin_xy[1]) / cell_size_m).clamp(0.0, (rows - 1) as f64);
        let c0 = (gx.floor() as usize).min(cols - 1);
        let r0 = (gy.floor() as usize).min(rows - 1);
        let c1 = (c0 + 1).min(cols - 1);
        let r1 = (r0 + 1).min(rows - 1);
        let tx = gx - (c0 as f64);
        let ty = gy - (r0 as f64);
        let z00 = f64::from(dem_grid[r0 * cols + c0]);
        let z10 = f64::from(dem_grid[r0 * cols + c1]);
        let z01 = f64::from(dem_grid[r1 * cols + c0]);
        let z11 = f64::from(dem_grid[r1 * cols + c1]);
        let top = z00 * (1.0 - tx) + z10 * tx;
        let bot = z01 * (1.0 - tx) + z11 * tx;
        top * (1.0 - ty) + bot * ty
    };

    let mut total_len = 0.0_f64;
    for seg in section_line_pts.windows(2) {
        total_len += (seg[1][0] - seg[0][0]).hypot(seg[1][1] - seg[0][1]);
    }
    let n_samples = ((total_len / step).round() as usize) + 1;
    let mut out = Vec::with_capacity(n_samples);

    let first_pt = section_line_pts[0];
    for idx in 0..n_samples {
        let chainage = ((idx as f64) * step).min(total_len);
        // Locate point along polyline at `chainage`
        let mut rem = chainage;
        let mut pos = first_pt;
        for seg in section_line_pts.windows(2) {
            let seg_len = (seg[1][0] - seg[0][0]).hypot(seg[1][1] - seg[0][1]);
            if rem <= seg_len || seg_len <= 1e-9 {
                let t = if seg_len > 1e-9 { rem / seg_len } else { 0.0 };
                pos = [
                    seg[0][0] + (seg[1][0] - seg[0][0]) * t,
                    seg[0][1] + (seg[1][1] - seg[0][1]) * t,
                ];
                break;
            }
            rem -= seg_len;
            pos = seg[1];
        }
        let elev = sample_at(pos[0], pos[1]);
        out.push(ElevationProfilePoint {
            chainage_m: chainage,
            elevation_m: elev,
            slope_deg: 0.0,
        });
    }
    for i in 0..out.len() {
        let prev = if i > 0 { i - 1 } else { i };
        let next = if i + 1 < out.len() { i + 1 } else { i };
        let dx = out[next].chainage_m - out[prev].chainage_m;
        if dx > 1e-6 {
            let dz = (out[next].elevation_m - out[prev].elevation_m).abs();
            out[i].slope_deg = (dz / dx).atan().to_degrees();
        }
    }
    out
}

/// 유구 상면·바닥 표고(`top_elev_m`, `bottom_elev_m`) 및 수동 깊이(`typed_depth_m`)로부터 유효 깊이와 접근성 설명 캡션을 계산
/// (C++ `KaElevationFigure::setValues` / `depthOf` 1:1 대응)
#[must_use]
pub fn compute_pit_elevation_depth_and_caption(
    top_elev_m: Option<f64>,
    bottom_elev_m: Option<f64>,
    typed_depth_m: Option<f64>,
) -> (Option<f64>, String) {
    let depth = match typed_depth_m.filter(|v| v.is_finite() && *v >= 0.0) {
        Some(d) => Some(d),
        None => match (
            top_elev_m.filter(|v| v.is_finite()),
            bottom_elev_m.filter(|v| v.is_finite()),
        ) {
            (Some(t), Some(b)) if t >= b => Some(t - b),
            _ => None,
        },
    };
    let top_text = top_elev_m
        .filter(|v| v.is_finite())
        .map_or_else(|| "미입력".to_owned(), |v| format!("{v:.2}"));
    let bottom_text = bottom_elev_m
        .filter(|v| v.is_finite())
        .map_or_else(|| "미입력".to_owned(), |v| format!("{v:.2}"));
    let caption = match depth {
        Some(d) => format!("상면 표고 {top_text}, 바닥 표고 {bottom_text}, 깊이 {d:.2} 미터"),
        None => format!("상면 표고 {top_text}, 바닥 표고 {bottom_text}"),
    };
    (depth, caption)
}

/// 3D 지형 원근 투영 기본 화각 (45.0°, C++ `HGIS_DEFAULT_FOV_DEGREES`)
pub const HGIS_DEFAULT_FOV_DEGREES: f32 = 45.0;
/// 트렌치 절개벽 최대 쿼드 수 (100,000, C++ `HGIS_MAX_TRENCH_MESH_QUADS`)
pub const HGIS_MAX_TRENCH_MESH_QUADS: usize = 100_000;

/// 트렌치 3D 지형 수직 절개 및 굴착 토량 산출 결과 (C++ `TrenchCutVolumeResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct TrenchCutVolumeResult {
    pub excavated_volume_m3: f64,
    pub trench_floor_area_m2: f64,
    pub max_excavation_depth_m: f64,
    pub wall_quad_count: usize,
    pub warning_message: Option<String>,
}

/// 평탄/일반 지형에서 트렌치 바닥 면적(`trench_area_m2`), 지표 표고 샘플(`surface_elevations_m`), 계획 바닥고(`target_floor_elev_m`)로부터 굴착 토량(`㎥`) 및 최대 굴착 심도 산출
/// (C++ `TrenchTerrain::carveTrenchIntoDem` 1:1 대응)
#[must_use]
pub fn carve_trench_volume_from_samples(
    surface_elevations_m: &[f64],
    trench_area_m2: f64,
    target_floor_elev_m: f64,
    perimeter_vertices: usize,
) -> TrenchCutVolumeResult {
    if surface_elevations_m.is_empty()
        || !trench_area_m2.is_finite()
        || trench_area_m2 <= 0.0
        || !target_floor_elev_m.is_finite()
    {
        return TrenchCutVolumeResult {
            excavated_volume_m3: 0.0,
            trench_floor_area_m2: 0.0,
            max_excavation_depth_m: 0.0,
            wall_quad_count: 0,
            warning_message: None,
        };
    }
    let cell_area = trench_area_m2 / (surface_elevations_m.len() as f64);
    let mut volume = 0.0_f64;
    let mut max_depth = 0.0_f64;
    for &z_surf in surface_elevations_m {
        if z_surf.is_finite() {
            let dz = (z_surf - target_floor_elev_m).max(0.0);
            volume += dz * cell_area;
            max_depth = max_depth.max(dz);
        }
    }
    let warning_message = if max_depth <= 0.0 {
        Some("굴착 바닥고가 지표면보다 높아 절개가 수행되지 않았습니다.".to_owned())
    } else {
        None
    };
    TrenchCutVolumeResult {
        excavated_volume_m3: volume,
        trench_floor_area_m2: trench_area_m2,
        max_excavation_depth_m: max_depth,
        wall_quad_count: perimeter_vertices.min(HGIS_MAX_TRENCH_MESH_QUADS),
        warning_message,
    }
}

/// 사면 상단 방위각(`upslope_azimuth_deg`)을 따라 높은 쪽부터 낮은 쪽으로, 같은 행에서는 좌에서 우로 시굴 트렌치 번호를 재부여
/// (C++ `TrenchTerrain::numberFromHighSide` 1:1 대응)
#[must_use]
pub fn number_trenches_from_high_side(
    trench_centers: &[[f64; 2]],
    upslope_azimuth_deg: f64,
    prefix: &str,
) -> Vec<(usize, String)> {
    let rad = upslope_azimuth_deg.to_radians();
    let ux = rad.sin();
    let uy = rad.cos();
    let vx = rad.cos();
    let vy = -rad.sin();

    let mut keys: Vec<(i64, f64, usize)> = trench_centers
        .iter()
        .enumerate()
        .map(|(idx, &[cx, cy])| {
            let along_cm = ((cx * ux + cy * uy) * 100.0).round() as i64;
            let across = cx * vx + cy * vy;
            (along_cm, across, idx)
        })
        .collect();

    keys.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.total_cmp(&b.1)));

    keys.into_iter()
        .enumerate()
        .map(|(ord, (_, _, orig_idx))| (orig_idx, format!("{prefix}{}", ord + 1)))
        .collect()
}

/// Copernicus GLO-30 30m 전지구 표고 배경 레이어 제목 (C++ `BasemapDsm::title`)
#[must_use]
pub const fn copernicus_dsm_title_ko() -> &'static str {
    "고도지형 (Copernicus 30 m)"
}

/// Copernicus GLO-30 30m 전지구 표고 배경 레이어 툴팁 (C++ `BasemapDsm::toolTip`)
#[must_use]
pub const fn copernicus_dsm_tooltip_ko() -> &'static str {
    "수치지형도 등고선이 없을 때 쓰는 전지구 30 m 표고 배경 (Copernicus GLO-30)"
}

/// Mapbox / VWorld Terrain-RGB 24비트 타일 픽셀(`R, G, B`)을 실수 표고(`m`)로 디코딩
/// `elevation_m = -10000.0 + ((R * 65536 + G * 256 + B) * 0.1)`
/// (C++ `BasemapDsm::decodeTerrainRgbPixel` 1:1 대응)
#[must_use]
pub fn decode_terrain_rgb_pixel(r: u8, g: u8, b: u8) -> f32 {
    let packed = ((u32::from(r)) << 16) | ((u32::from(g)) << 8) | u32::from(b);
    -10_000.0_f32 + (packed as f32) * 0.1_f32
}

/// DEM 파일 경로에서 음영기복(`_hillshade.tif`) 출력 경로 도출 (C++ `DemHillshadeFile::outputPath`)
#[must_use]
pub fn hillshade_output_path(dem_path: &str) -> String {
    let trimmed = dem_path.trim();
    if let Some((stem, _ext)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() {
            return format!("{stem}_hillshade.tif");
        }
    }
    format!("{trimmed}_hillshade.tif")
}

/// Horn (1981) 8방향 차분 커널을 이용해 DEM 격자에서 8비트 그레이스케일 음영기복(`0..=255`) 생성
/// 평탄 지형(`slope = 0`)에서 태양 고도 `45°`일 때 `floor(255.0 * cos(45°)) = 180` 반환
/// (C++ `HillshadeTileRenderer::renderHillshade` / `DemHillshadeFile::write` 1:1 대응)
#[must_use]
pub fn compute_horn_hillshade_byte_grid(
    grid: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    sun_altitude_deg: f64,
    sun_azimuth_deg: f64,
) -> Vec<u8> {
    if width == 0 || height == 0 || grid.len() < width * height {
        return Vec::new();
    }
    let cs = if cell_size_m.is_finite() && cell_size_m > 1e-6 {
        cell_size_m
    } else {
        1.0
    };
    let zenith_rad = (90.0 - sun_altitude_deg.clamp(0.0, 90.0)).to_radians();
    let azimuth_math_rad = (360.0 - sun_azimuth_deg + 90.0)
        .rem_euclid(360.0)
        .to_radians();

    let sample = |r: isize, c: isize| -> f64 {
        let rr = r.clamp(0, (height as isize) - 1) as usize;
        let cc = c.clamp(0, (width as isize) - 1) as usize;
        let v = f64::from(grid[rr * width + cc]);
        if v.is_finite() {
            v
        } else {
            0.0
        }
    };

    let mut out = vec![0_u8; width * height];
    for r in 0..height {
        for c in 0..width {
            let ri = r as isize;
            let ci = c as isize;
            let z_nw = sample(ri - 1, ci - 1);
            let z_n = sample(ri - 1, ci);
            let z_ne = sample(ri - 1, ci + 1);
            let z_w = sample(ri, ci - 1);
            let z_e = sample(ri, ci + 1);
            let z_sw = sample(ri + 1, ci - 1);
            let z_s = sample(ri + 1, ci);
            let z_se = sample(ri + 1, ci + 1);

            let dz_dx = ((z_ne + 2.0 * z_e + z_se) - (z_nw + 2.0 * z_w + z_sw)) / (8.0 * cs);
            let dz_dy = ((z_sw + 2.0 * z_s + z_se) - (z_nw + 2.0 * z_n + z_ne)) / (8.0 * cs);
            let slope_rad = dz_dx.hypot(dz_dy).atan();
            let aspect_rad = if dz_dx.abs() > 1e-12 || dz_dy.abs() > 1e-12 {
                dz_dy.atan2(-dz_dx).rem_euclid(std::f64::consts::TAU)
            } else {
                0.0
            };

            let cos_i = zenith_rad.cos() * slope_rad.cos()
                + zenith_rad.sin() * slope_rad.sin() * (azimuth_math_rad - aspect_rad).cos();
            let val = (255.0 * cos_i.max(0.0)).floor().clamp(0.0, 255.0) as u8;
            out[r * width + c] = val;
        }
    }
    out
}

#[cfg(test)]
mod batch62_trench_terrain_tests {
    use super::*;

    #[test]
    fn test_trench_volume_calculation_and_number_from_high_side() {
        // TestTrenchTerrain::testVolumeCalculation:
        // Flat ground Z = 50.0m, 2.0m x 20.0m trench (area = 40.0 m²), floor Z = 48.5m (depth = 1.5m) -> 60.0 m³
        let res = carve_trench_volume_from_samples(&[50.0, 50.0, 50.0, 50.0], 40.0, 48.5, 4);
        assert!((res.excavated_volume_m3 - 60.0).abs() <= 1e-2);
        assert!((res.max_excavation_depth_m - 1.5).abs() <= 1e-6);
        assert!(res.warning_message.is_none());

        // Negative cut guard (floor higher than ground)
        let neg = carve_trench_volume_from_samples(&[50.0], 40.0, 52.0, 4);
        assert!((neg.excavated_volume_m3 - 0.0).abs() < 1e-9);
        assert_eq!(
            neg.warning_message.as_deref(),
            Some("굴착 바닥고가 지표면보다 높아 절개가 수행되지 않았습니다.")
        );

        // TrenchTerrain::numberFromHighSide: north (0 deg) is high side -> higher Y gets T1, left-to-right gets T1 then T2
        let centers = [[20.0, 10.0], [10.0, 100.0], [20.0, 100.0]];
        let numbered = number_trenches_from_high_side(&centers, 0.0, "T");
        assert_eq!(numbered[0], (1, "T1".to_owned()));
        assert_eq!(numbered[1], (2, "T2".to_owned()));
        assert_eq!(numbered[2], (0, "T3".to_owned()));
    }

    #[test]
    fn test_terrain_rgb_decoding_and_flat_terrain_hillshade() {
        // TestBasemapDsm::testTerrainRgbDecoding: R=1, G=134 (0x86), B=160 (0xA0) -> 100,000 -> 0.0m
        let elev = decode_terrain_rgb_pixel(1, 134, 160);
        assert!((elev - 0.0).abs() <= 1e-4);
        assert_eq!(copernicus_dsm_title_ko(), "고도지형 (Copernicus 30 m)");
        assert_eq!(
            hillshade_output_path("/tmp/site_dem.tif"),
            "/tmp/site_dem_hillshade.tif"
        );

        // TestHillshadeTileRenderer::testFlatTerrainHillshade: flat 50.0m grid, sun altitude 45.0 deg -> 180
        let flat = vec![50.0_f32; 20 * 20];
        let hs = compute_horn_hillshade_byte_grid(&flat, 20, 20, 1.0, 45.0, 315.0);
        assert_eq!(hs.len(), 400);
        assert_eq!(hs[10 * 20 + 10], 180);
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

    #[test]
    fn test_suspect_point_filtering_and_survey_contour_tin_builder() {
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

        assert_eq!(
            survey_contour_rejection(&pts[..2]),
            Some("등고선을 만들려면 점이 3개 이상 필요합니다.")
        );
        assert_eq!(survey_contour_rejection(&pts), None);

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
        assert_eq!(densified.len(), 4);
        let fitted = fit_contour_cell_size(0.1, 1000.0, 1000.0, HGIS_MAX_CONTOUR_GRID_CELLS);
        assert!(fitted >= 0.5);
    }

    #[test]
    fn test_profile_length_and_elevation_sampling_and_dem_legend_ticks() {
        let mut grid = Vec::with_capacity(22);
        for _r in 0..2 {
            for c in 0..=10 {
                grid.push(10.0_f32 + (c as f32));
            }
        }
        let pts = sample_elevation_profile_bilinear(
            &grid,
            11,
            2,
            1.0,
            [0.0, 0.0],
            &[[0.0, 0.5], [10.0, 0.5]],
            1.0,
        );
        assert_eq!(pts.len(), 11);
        assert!((pts[0].chainage_m - 0.0).abs() < 1e-6);
        assert!((pts[0].elevation_m - 10.0).abs() < 1e-5);
        assert!((pts[5].chainage_m - 5.0).abs() < 1e-6);
        assert!((pts[5].elevation_m - 15.0).abs() < 1e-5);
        assert!((pts[10].chainage_m - 10.0).abs() < 1e-6);
        assert!((pts[10].elevation_m - 20.0).abs() < 1e-5);

        let ticks = build_dem_legend_ticks(-12.0, 1850.0);
        assert!((5..=7).contains(&ticks.len()));
        assert!((ticks[0] - (-12.0)).abs() < 1e-6);
        assert!((*ticks.last().unwrap_or(&0.0) - 1850.0).abs() < 1e-6);

        let (depth, caption) =
            compute_pit_elevation_depth_and_caption(Some(48.50), Some(47.20), None);
        assert!((depth.unwrap_or(0.0) - 1.30).abs() < 1e-6);
        assert_eq!(caption, "상면 표고 48.50, 바닥 표고 47.20, 깊이 1.30 미터");
    }
}
