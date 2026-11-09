//! polygon_ops 회귀 시험 1/2 — 분할·헬머트·정점·MBR·슬리버.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use geo::{Area, Coord, LineString, Polygon};
use strata_types::error::StrataResult;

use crate::boolean_split::*;
use crate::helmert_solve::*;
use crate::mbr_catalog::*;
use crate::sliver_hole::*;
use crate::snap_vertex_ops::*;

#[test]
fn polygon_split_and_helmert_georef_produce_two_pieces_and_submillimeter_rms(
) -> StrataResult<()> {
    let rect = Polygon::new(
        LineString::from(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 100.0, y: 0.0 },
            Coord { x: 100.0, y: 100.0 },
            Coord { x: 0.0, y: 100.0 },
            Coord { x: 0.0, y: 0.0 },
        ]),
        vec![],
    );
    let pieces = split_polygon_by_line(&rect, [50.0, -10.0], [50.0, 110.0]);
    assert_eq!(pieces.len(), 2);

    let src = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
    let dst = [
        [200_000.0, 500_000.0],
        [200_200.0, 500_000.0],
        [200_000.0, 500_200.0],
    ];
    let tf = solve_helmert_georef(&src, &dst)?;
    assert!((tf.scale - 2.0).abs() < 1e-6);
    assert!(tf.rms_error_m < 1e-6);
    let mapped = tf.apply([50.0, 50.0]);
    assert!((mapped[0] - 200_100.0).abs() < 1e-5);
    assert!((mapped[1] - 500_100.0).abs() < 1e-5);
    Ok(())
}

#[test]
fn test_georef_helmert_45deg_scale2_and_collinear_singular_matrix_detection() {
    let rot = std::f64::consts::FRAC_PI_4; // 45 deg = 0.785398163...
    let scale = 2.0_f64;
    let tx = 100.0_f64;
    let ty = 200.0_f64;
    let c = scale * rot.cos();
    let s = scale * rot.sin();

    let src = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let dst: Vec<[f64; 2]> = src
        .iter()
        .map(|&[x, y]| [c * x - s * y + tx, s * x + c * y + ty])
        .collect();

    let (status, tf_opt) = calculate_helmert_transform_checked(&src, &dst);
    assert_eq!(status, HelmertStatus::Ok);
    let tf = tf_opt.expect("helmert transform must exist");
    assert!((tf.scale - 2.0).abs() < 1e-6);
    assert!((tf.rotation_rad - rot).abs() < 1e-6);
    assert!(tf.rms_error_m < 1e-9);

    // 4 collinear points on y = 2x -> SingularMatrix
    let collinear_src = [[0.0, 0.0], [1.0, 2.0], [2.0, 4.0], [3.0, 6.0]];
    let (col_status, col_tf) = calculate_helmert_transform_checked(&collinear_src, &dst);
    assert_eq!(col_status, HelmertStatus::SingularMatrix);
    assert!(col_tf.is_none());
}

#[test]
fn test_polygon_ops_split_10x10_square_yields_two_50m2_pieces() {
    let sq = Polygon::new(
        LineString::from(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 0.0, y: 10.0 },
            Coord { x: 0.0, y: 0.0 },
        ]),
        vec![],
    );
    let pieces = split_polygon_by_line(&sq, [5.0, -2.0], [5.0, 12.0]);
    assert_eq!(pieces.len(), 2);
    let total_area: f64 = pieces.iter().map(Area::unsigned_area).sum();
    assert!((total_area - 100.0).abs() < 0.1);
    for p in &pieces {
        assert!((p.unsigned_area() - 50.0).abs() < 0.1);
    }

    let checked = split_polygon_with_line_checked(&sq, [5.0, -2.0], [5.0, 12.0], "구덩이-10")
        .expect("valid split");
    assert_eq!(checked.len(), 2);
    assert_eq!(checked[0].0, "구덩이-10");
    assert_eq!(checked[1].0, "구덩이-10-1");

    // 내부에서 끝나는 불완전 절단선은 실패해야 함
    let incomplete = split_polygon_with_line_checked(&sq, [5.0, -2.0], [5.0, 5.0], "구덩이-10");
    assert!(incomplete.is_err());
}

#[test]
fn test_delete_vertex_triangle_blocked_and_ortho_45_snap() {
    let mut triangle = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0], [0.0, 0.0]];
    assert_eq!(triangle.len(), 4);
    assert!(!delete_polygon_vertex_checked(&mut triangle, 1));
    assert_eq!(triangle.len(), 4);

    let mut quad = vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [0.0, 0.0],
    ];
    assert!(delete_polygon_vertex_checked(&mut quad, 2));
    assert_eq!(quad.len(), 4);

    let mut seg = vec![[0.0, 0.0], [10.0, 0.0]];
    assert!(!delete_line_vertex_checked(&mut seg, 0));
    assert_eq!(seg.len(), 2);

    let snapped = snap_to_ortho_45([0.0, 0.0], [10.0, 1.0]);
    assert!((snapped[1] - 0.0).abs() < 1e-6);
}

#[test]
fn test_n15e_and_n30w_mbr_azimuth_and_natural_sort_and_null_dash() {
    // 2.0m x 6.0m rectangle rotated 15 deg clockwise (N-15°-E)
    let base_rect = [
        [-1.0_f64, -3.0],
        [1.0, -3.0],
        [1.0, 3.0],
        [-1.0, 3.0],
        [-1.0, -3.0],
    ];
    // Clockwise by 15 deg: x' = x cos15 + y sin15, y' = -x sin15 + y cos15
    let rad = 15.0_f64.to_radians();
    let (s, c) = rad.sin_cos();
    let rot_rect: Vec<[f64; 2]> = base_rect
        .iter()
        .map(|&[x, y]| [x * c + y * s, -x * s + y * c])
        .collect();
    let metrics = calculate_feature_mbr_metrics(&rot_rect);
    assert!((metrics.major_axis_m - 6.0).abs() < 1e-3);
    assert!((metrics.minor_axis_m - 2.0).abs() < 1e-3);
    assert!((metrics.aspect_ratio - 3.0).abs() < 1e-3);
    assert_eq!(metrics.archaeological_azimuth, "N-15°-E");
    assert_eq!(format_archaeological_azimuth(150.0), "N-30°-W");

    let mut list = vec!["10호", "2호", "1호", "10-2호", "10-1호", "20호"];
    list.sort_by(|a, b| natural_sort_compare(a, b));
    assert_eq!(list, vec!["1호", "2호", "10호", "10-1호", "10-2호", "20호"]);
    assert_eq!(format_nullable_catalog_field(None), "-");
    assert_eq!(format_nullable_catalog_field(Some("   ")), "-");
}

#[test]
fn test_interior_hole_punch_10x10_minus_2x2_yields_96m2_and_sliver_cleaning_under_0_05m2() {
    // TestKaBooleanPolygonEditor::testInteriorHolePunch:
    // 10m x 10m outer polygon (100 m²) minus 2m x 2m interior hole (4 m²) -> 1 interior ring, 96.0 m² area
    let outer = Polygon::new(
        LineString::from(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
            Coord { x: 10.0, y: 10.0 },
            Coord { x: 0.0, y: 10.0 },
            Coord { x: 0.0, y: 0.0 },
        ]),
        vec![],
    );
    let hole = [[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]];
    let punched = punch_interior_hole_polygon(&outer, &hole);
    assert_eq!(punched.interiors().len(), 1);
    assert!((punched.unsigned_area() - 96.0).abs() < 1e-6);

    // TestKaBooleanPolygonEditor::testSliverCleaningUnder0_05m2:
    // Sliver interior hole of 0.2m x 0.1m = 0.02 m² (< 0.05 m²) -> removed (0 interior rings)
    let sliver_hole = [[1.0, 1.0], [1.2, 1.0], [1.2, 1.1], [1.0, 1.1], [1.0, 1.0]];
    let with_sliver = punch_interior_hole_polygon(&outer, &sliver_hole);
    assert_eq!(with_sliver.interiors().len(), 1);
    let cleaned = clean_sliver_interior_rings(&with_sliver, HGIS_MIN_SLIVER_AREA_M2);
    assert_eq!(cleaned.interiors().len(), 0);
    assert!((cleaned.unsigned_area() - 100.0).abs() < 1e-6);
}
