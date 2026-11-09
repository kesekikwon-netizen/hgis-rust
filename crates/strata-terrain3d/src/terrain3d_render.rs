//! 3D 지형 RGBA 렌더(`render_terrain3d_rgba`).
//!
//! `viewport3d_renderer` 에서 분리했다.
//! 하늘 채우기와 메시 래스터화는 각각 `terrain3d_sky`·`terrain3d_mesh_raster` 에 있다.

use crate::atmosphere_agx::apply_nishita_aerial_perspective_agx;

use crate::terrain3d_camera::OrbitCamera3D;
use crate::terrain3d_dem::{
    cross3, dot3, normalize3, sample_dem_elevation_m, sub3, HGIS_DEM_NODATA_VAL,
};
use crate::terrain3d_mesh_raster::rasterize_terrain_mesh;
use crate::terrain3d_sky::paint_sky_background;
use crate::terrain3d_zbuf::rasterize_triangle_zbuf;

/// C++ `Terrain3dService::buildMesh` + `Terrain3dService::renderPerspective` (`Terrain3dService.cpp:163-420`) 1:1 재현
#[must_use]
pub fn render_terrain3d_rgba(
    dem: &[f64],
    grid_w: usize,
    grid_h: usize,
    cam: &OrbitCamera3D,
    out_w: u32,
    out_h: u32,
) -> Vec<u8> {
    let w = out_w.max(16) as usize;
    let h = out_h.max(16) as usize;
    let mut buf = vec![0_u8; w * h * 4];
    let mut zbuf = vec![1e9_f32; w * h];

    // 1. Nishita 대기 산란 하늘 및 스튜디오 지평선 배경
    let pitch_clamped = paint_sky_background(&mut buf, w, h, cam.pitch_deg);

    // 2. DEM 표고 범위 계산 및 3D 메시 격자 생성
    let mut dem_min = f64::INFINITY;
    let mut dem_max = f64::NEG_INFINITY;
    for &v in dem {
        if v.is_finite() && v > f64::from(HGIS_DEM_NODATA_VAL) + 1.0 {
            dem_min = dem_min.min(v);
            dem_max = dem_max.max(v);
        }
    }
    if !dem_min.is_finite() || !dem_max.is_finite() {
        dem_min = 18.5;
        dem_max = 142.0;
    }
    let dem_span = (dem_max - dem_min).max(0.0);

    let cols = if w >= 360 { 84usize } else { 64usize };
    let rows = if h >= 240 { 64usize } else { 48usize };
    let half_w = 96.0_f32;
    let half_h = 72.0_f32;
    let z_exag = if cam.z_exaggeration.is_finite() {
        cam.z_exaggeration.clamp(0.2, 12.0)
    } else {
        2.0
    };
    let z_scale = 0.25_f32 * z_exag;

    let n_verts = cols * rows;
    let mut positions = vec![[0.0_f32; 3]; n_verts];
    let mut elev_m_grid = vec![18.5_f64; n_verts];
    let mut river_dist_grid = vec![1.0_f64; n_verts];

    for r in 0..rows {
        let v = r as f64 / (rows.saturating_sub(1).max(1) as f64);
        let my = (0.5 - v as f32) * (half_h * 2.0);
        for c in 0..cols {
            let u = c as f64 / (cols.saturating_sub(1).max(1) as f64);
            let mx = (u as f32 - 0.5) * (half_w * 2.0);
            let (elev_m, d_riv) =
                sample_dem_elevation_m(dem, grid_w, grid_h, dem_min, dem_span, u, v);
            let mz = ((elev_m - 18.5).max(0.0) as f32) * z_scale;
            let idx = r * cols + c;
            positions[idx] = [mx, my, mz];
            elev_m_grid[idx] = elev_m;
            river_dist_grid[idx] = d_riv;
        }
    }

    // 3. 카메라 투영 행렬 설정 (Terrain3dService.cpp:251-278)
    let yaw = if cam.yaw_deg.is_finite() {
        cam.yaw_deg.to_radians()
    } else {
        35.0_f32.to_radians()
    };
    let pitch = pitch_clamped.to_radians();
    let zoom = if cam.zoom.is_finite() {
        cam.zoom.clamp(0.4, 3.0)
    } else {
        1.0
    };
    let target = [0.0_f32, 0.0, 14.0];
    let dir = normalize3([
        yaw.sin() * pitch.cos(),
        -yaw.cos() * pitch.cos(),
        pitch.sin(),
    ]);
    let dist = 218.0 / zoom;
    let eye = [
        target[0] + dir[0] * dist,
        target[1] + dir[1] * dist,
        target[2] + dir[2] * dist,
    ];
    let fwd = normalize3(sub3(target, eye));
    let mut right = cross3(fwd, [0.0, 0.0, 1.0]);
    if dot3(right, right) < 1e-6 {
        right = [1.0, 0.0, 0.0];
    } else {
        right = normalize3(right);
    }
    let up = normalize3(cross3(right, fwd));
    let fov_rad = 45.0_f32.to_radians();
    let focal = (h as f32) / (2.0 * (fov_rad * 0.5).tan());

    let project = |p: [f32; 3]| -> [f32; 3] {
        let rel = sub3(p, eye);
        let cx = dot3(rel, right);
        let cy = dot3(rel, up);
        let cz = dot3(rel, fwd);
        if cz <= 0.4 {
            [-1e6, -1e6, cz]
        } else {
            let s = focal / cz;
            [(w as f32) * 0.5 + cx * s, (h as f32) * 0.54 - cy * s, cz]
        }
    };

    // 4. 정점 법선 및 AgX + Nishita 조명 색상 계산
    let sun_dir = normalize3([-0.48, -0.38, 0.79]);
    let fill_dir = normalize3([0.35, 0.42, 0.84]);
    let view_dir = normalize3(sub3(eye, target));
    let half_vec = normalize3([
        sun_dir[0] + view_dir[0],
        sun_dir[1] + view_dir[1],
        sun_dir[2] + view_dir[2],
    ]);

    let mut proj_pts = vec![[0.0_f32; 3]; n_verts];
    let mut vert_cols = vec![[0_u8; 4]; n_verts];

    for r in 0..rows {
        let v = r as f64 / (rows.saturating_sub(1).max(1) as f64);
        for c in 0..cols {
            let u = c as f64 / (cols.saturating_sub(1).max(1) as f64);
            let idx = r * cols + c;
            let p = positions[idx];
            proj_pts[idx] = project(p);

            let c_prev = c.saturating_sub(1);
            let c_next = (c + 1).min(cols - 1);
            let r_prev = r.saturating_sub(1);
            let r_next = (r + 1).min(rows - 1);

            let dx = sub3(positions[r * cols + c_next], positions[r * cols + c_prev]);
            let dy = sub3(positions[r_prev * cols + c], positions[r_next * cols + c]);
            let nrm = normalize3(cross3(dx, dy));

            let ndotl = dot3(nrm, sun_dir).max(0.0);
            let ndotfill = dot3(nrm, fill_dir).max(0.0);
            let slope = (1.0 - nrm[2].clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let elev_m = elev_m_grid[idx];
            let d_riv = river_dist_grid[idx];
            let ht = ((elev_m - 18.5) / (142.0 - 18.5)).clamp(0.0, 1.0);

            // 조사구역(AOI) 드레이핑 경계선 강조 (+0.25m 오프셋)
            let on_aoi_edge = ((u - 0.36).abs() < 0.011 && (0.36..=0.64).contains(&v))
                || ((u - 0.64).abs() < 0.011 && (0.36..=0.64).contains(&v))
                || ((v - 0.36).abs() < 0.014 && (0.36..=0.64).contains(&u))
                || ((v - 0.64).abs() < 0.014 && (0.36..=0.64).contains(&u));

            // 표고 등고 밴드의 부드러운 입체 음영 변조
            let band_mod = 0.92 + 0.08 * (elev_m * 0.35).sin();

            let (mut albedo, spec_strength) = if on_aoi_edge {
                ([1.60, 0.06, 0.03], 0.15)
            } else if d_riv < 0.026 {
                // 하천 수면 (형산강·남천)
                ([0.015, 0.14, 0.68], 0.55)
            } else if ht < 0.10 {
                // 충적 저지대·범람원·자연제방 (선명한 충적 녹색)
                let t = ht / 0.10;
                ([0.05 + 0.12 * t, 0.32 + 0.06 * t, 0.04 + 0.02 * t], 0.03)
            } else if ht < 0.30 {
                // 선상지·구릉 완사면 (황토·올리브 단구면)
                let t = (ht - 0.10) / 0.20;
                (
                    [
                        0.18 + 0.18 * t + 0.10 * f64::from(slope),
                        0.34 - 0.08 * t,
                        0.05 + 0.02 * t,
                    ],
                    0.02,
                )
            } else if ht < 0.68 {
                // 산지 사면 숲·능선 사면 (짙은 산림 녹색 + 급경사 황갈색 암반)
                let t = (ht - 0.30) / 0.38;
                let rock = f64::from(((slope - 0.28) * 2.2).clamp(0.0, 1.0));
                let forest = [0.04 + 0.14 * t, 0.24 - 0.04 * t, 0.03 + 0.02 * t];
                let ridge_soil = [0.42, 0.24, 0.09];
                (
                    [
                        forest[0] * (1.0 - rock) + ridge_soil[0] * rock,
                        forest[1] * (1.0 - rock) + ridge_soil[1] * rock,
                        forest[2] * (1.0 - rock) + ridge_soil[2] * rock,
                    ],
                    0.05 * rock,
                )
            } else {
                // 고지대 산능선·봉우리 암릉 (고원 갈색~화강암 회백색)
                let t = ((ht - 0.68) / 0.32).clamp(0.0, 1.0);
                ([0.38 + 0.42 * t, 0.24 + 0.42 * t, 0.12 + 0.40 * t], 0.10)
            };

            if !on_aoi_edge && d_riv >= 0.026 {
                albedo = [
                    albedo[0] * band_mod,
                    albedo[1] * band_mod,
                    albedo[2] * band_mod,
                ];
            }

            // 강한 직사광·음영 대비로 산능선과 계곡의 3D 입체 기복을 선명하게 표현
            let sun_term = f64::from(ndotl).powf(1.25);
            let valley_ao = (0.55 + 0.45 * ht).clamp(0.55, 1.0);
            let diffuse = (0.06 + 2.35 * sun_term + 0.18 * f64::from(ndotfill)) * valley_ao;
            let spec = f64::from(dot3(nrm, half_vec).max(0.0).powi(16)) * spec_strength;
            let cz = proj_pts[idx][2];
            let haze_t = ((f64::from(cz) - 185.0) / 320.0).clamp(0.0, 0.14);

            vert_cols[idx] =
                apply_nishita_aerial_perspective_agx(albedo, diffuse, spec, haze_t, 48.0);
        }
    }

    // 5. 지형 상부 삼각형 메시 Z-버퍼 래스터화
    rasterize_terrain_mesh(
        &mut buf,
        &mut zbuf,
        w,
        h,
        &proj_pts,
        &vert_cols,
        cols,
        rows,
    );

    // 6. 3D 지형 블록 측면 지층 단면 스커트(Pedestal Skirt) 렌더링 — 기복과 회전각이 한눈에 보이도록 입체 블록화
    let base_z = -7.5_f32;
    let mut draw_skirt_edge = |idx_a: usize, idx_b: usize, wall_shade: u8| {
        let pa_top = proj_pts[idx_a];
        let pb_top = proj_pts[idx_b];
        let mut wa_bot = positions[idx_a];
        let mut wb_bot = positions[idx_b];
        wa_bot[2] = base_z;
        wb_bot[2] = base_z;
        let pa_bot = project(wa_bot);
        let pb_bot = project(wb_bot);

        let top_col = [
            ((u16::from(wall_shade) * 168) / 255) as u8,
            ((u16::from(wall_shade) * 142) / 255) as u8,
            ((u16::from(wall_shade) * 116) / 255) as u8,
            255,
        ];
        let bot_col = [
            ((u16::from(wall_shade) * 108) / 255) as u8,
            ((u16::from(wall_shade) * 96) / 255) as u8,
            ((u16::from(wall_shade) * 86) / 255) as u8,
            255,
        ];

        rasterize_triangle_zbuf(
            &mut buf, &mut zbuf, w, h, pa_top, pa_bot, pb_top, top_col, bot_col, top_col,
        );
        rasterize_triangle_zbuf(
            &mut buf, &mut zbuf, w, h, pb_top, pa_bot, pb_bot, top_col, bot_col, bot_col,
        );
    };

    // 남측·북측 경계 벽면
    for c in 0..(cols - 1) {
        let south_a = (rows - 1) * cols + c;
        draw_skirt_edge(south_a, south_a + 1, 215);
        draw_skirt_edge(c, c + 1, 155);
    }
    // 서측·동측 경계 벽면
    for r in 0..(rows - 1) {
        let west_a = r * cols;
        draw_skirt_edge(west_a, west_a + cols, 235);
        let east_a = r * cols + (cols - 1);
        draw_skirt_edge(east_a, east_a + cols, 175);
    }

    buf
}
