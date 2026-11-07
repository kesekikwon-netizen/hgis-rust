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
