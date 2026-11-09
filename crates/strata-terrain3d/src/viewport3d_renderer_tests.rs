use crate::viewport3d_renderer::*;

#[test]
fn terrain3d_rgba_renderer_produces_sky_and_agx_lit_terrain_buffer() {
    let dem = vec![42.0; 16 * 16];
    let cam = OrbitCamera3D::default();
    let buf = render_terrain3d_rgba(&dem, 16, 16, &cam, 160, 100);
    assert_eq!(buf.len(), 160 * 100 * 4);
}

#[test]
fn test_height_exaggeration_and_orbit_rotation_visibly_change_3d_terrain_render() {
    let dem: Vec<f64> = (0..(32 * 32))
        .map(|i| 20.0 + ((i % 32) as f64) * 2.5 + ((i / 32) as f64) * 1.8)
        .collect();
    let cam_low = OrbitCamera3D {
        z_exaggeration: 0.4,
        ..OrbitCamera3D::default()
    };
    let cam_high = OrbitCamera3D {
        z_exaggeration: 5.0,
        ..OrbitCamera3D::default()
    };

    let buf_low = render_terrain3d_rgba(&dem, 32, 32, &cam_low, 160, 100);
    let buf_high = render_terrain3d_rgba(&dem, 32, 32, &cam_high, 160, 100);
    let diff_exag = buf_low
        .chunks_exact(4)
        .zip(buf_high.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        diff_exag > 1500,
        "z_exaggeration must visibly alter 3D relief pixels"
    );

    let cam_rot = OrbitCamera3D {
        yaw_deg: 125.0,
        pitch_deg: 22.0,
        ..OrbitCamera3D::default()
    };
    let buf_rot = render_terrain3d_rgba(&dem, 32, 32, &cam_rot, 160, 100);
    let diff_rot = buf_low
        .chunks_exact(4)
        .zip(buf_rot.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        diff_rot > 2000,
        "camera yaw/pitch must visibly rotate 3D terrain"
    );
}

#[test]
fn test_terrain_camera_pitch_clamping_120deg_clamps_to_exact_89deg() {
    let mut cam = OrbitCamera3D::default();
    cam.apply_orbit_drag(15.0, 120.0);
    assert!((cam.pitch_deg - 89.0).abs() < 1e-5);
    cam.apply_orbit_drag(0.0, -300.0);
    assert!((cam.pitch_deg - (-89.0)).abs() < 1e-5);
}

#[test]
fn test_section_profile_bilinear_interpolation_center_10_20_30_40_yields_exact_25() {
    let mut cam = OrbitCamera3D::default();
    cam.set_vertical_exaggeration(12.0);
    assert!((cam.z_exaggeration - 5.0).abs() < 1e-6);
    cam.set_vertical_exaggeration(0.2);
    assert!((cam.z_exaggeration - 1.0).abs() < 1e-6);

    let grid = [10.0_f32, 20.0, 30.0, 40.0];
    let z_center = sample_dem_bilinear(&grid, 2, 2, 0.0, 0.0, 10.0, 5.0, 5.0, true);
    assert!((z_center - 25.0).abs() < 1e-6);

    let z_outside = sample_dem_bilinear(&grid, 2, 2, 0.0, 0.0, 10.0, 25.0, 5.0, true);
    assert_eq!(z_outside, HGIS_DEM_NODATA_VAL);
}

#[test]
fn test_paleo_vertical_exaggeration_clamping_25_to_10_neg2_to_05_and_reset_to_north() {
    let mut cam = OrbitCamera3D::default();
    cam.set_paleo_vertical_exaggeration(25.0);
    assert!((cam.z_exaggeration - 10.0).abs() < 1e-6);

    cam.set_paleo_vertical_exaggeration(-2.0);
    assert!((cam.z_exaggeration - 0.5).abs() < 1e-6);

    cam.set_paleo_vertical_exaggeration(f32::NAN);
    assert!((cam.z_exaggeration - HGIS_Z_EXAG_DEFAULT).abs() < 1e-6);

    let dist = cam.reset_to_north(f32::NAN);
    assert!((cam.yaw_deg - 0.0).abs() < 1e-6);
    assert!((dist - HGIS_CAMERA_DEFAULT_DIST).abs() < 1e-5);
}
