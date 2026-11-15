//! The fit and the quality words of 맞추기 — ported from the C++ tests `tests/test_georef_quality.cpp` (all seven)
//! and `tests/test_georef.cpp` `twoPointRaster_flipsYAndKeepsThirdPointOnTarget` (:63-107), `invertRoundTrip`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{align_from_pairs, assess, notes, row_text, summary, AlignPair, RESIDUAL_WARN_M};

/// A CAD drawing already in map coordinates moved onto another map: 3 degrees, scale 1.0005, kilometres of shift.
fn truth_map(sx: f64, sy: f64) -> [f64; 2] {
    let (th, k) = (3.0_f64.to_radians(), 1.0005);
    let (x, y) = (sx - 200_000.0, sy - 500_000.0);
    [k * (th.cos() * x - th.sin() * y) + 194_000.0, k * (th.sin() * x + th.cos() * y) + 574_000.0]
}

fn pair_at(sx: f64, sy: f64) -> AlignPair {
    AlignPair { src: [sx, sy], map: truth_map(sx, sy) }
}

fn square_pairs(span: f64, with_centre: bool) -> Vec<AlignPair> {
    let (x0, y0) = (200_000.0, 500_000.0);
    let mut p = vec![pair_at(x0, y0), pair_at(x0 + span, y0), pair_at(x0 + span, y0 + span), pair_at(x0, y0 + span)];
    if with_centre {
        p.push(pair_at(x0 + span * 0.5, y0 + span * 0.5));
    }
    p
}

fn off(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Raw sums of 200 000-class coordinates lose every digit; centred sums fit four points exactly.
#[test]
fn absolute_source_four_points_fit_exactly() {
    for span in [30.0, 100.0, 300.0] {
        let a = align_from_pairs(&square_pairs(span, false), false).expect("a fit");
        assert_eq!(a.pair_count, 4);
        assert!(a.rms_m < 1e-6, "span {span} rms {}", a.rms_m);
    }
}

#[test]
fn absolute_source_three_points_predict_an_unpicked_point() {
    let p = [pair_at(200_000.0, 500_000.0), pair_at(200_100.0, 500_010.0), pair_at(200_020.0, 500_090.0)];
    let a = align_from_pairs(&p, false).expect("a fit");
    assert!(off(a.apply([200_070.0, 500_060.0]), truth_map(200_070.0, 500_060.0)) < 1e-6);
}

/// Points in a line still align, with the two-point similarity.
#[test]
fn absolute_source_collinear_falls_back_to_helmert() {
    let p = [pair_at(200_000.0, 500_000.0), pair_at(200_050.0, 500_000.0), pair_at(200_100.0, 500_000.0)];
    let a = align_from_pairs(&p, false).expect("a line of points still aligns with the 2-point formula");
    assert!(off(a.apply([200_030.0, 500_040.0]), truth_map(200_030.0, 500_040.0)) < 1e-4, "Helmert keeps rotation and scale");
}

/// Picture rows grow downwards: two points fit with the flip, and an unpicked third point lands right; a CAD
/// drawing (map units on the left too) is not flipped.
#[test]
fn two_point_raster_flips_y_and_keeps_the_third_point() {
    let to_map = |col: f64, row: f64| [100_000.0 + col, 500_000.0 + (800.0 - row)];
    let p = [AlignPair { src: [100.0, 100.0], map: to_map(100.0, 100.0) }, AlignPair { src: [900.0, 700.0], map: to_map(900.0, 700.0) }];
    let a = align_from_pairs(&p, true).expect("two points align");
    assert!(a.a * a.e - a.b * a.d < 0.0, "the picture must be flipped");
    assert!(off(a.apply([700.0, 200.0]), to_map(700.0, 200.0)) < 1e-6);
    let v = align_from_pairs(&p, false).expect("two points align");
    assert!(v.a * v.e - v.b * v.d > 0.0, "a vector drawing is not flipped");

    let q = [AlignPair { src: [100.0, 100.0], map: [100_100.0, 500_700.0] }, AlignPair { src: [900.0, 700.0], map: [100_900.0, 500_100.0] }];
    let r = align_from_pairs(&q, true).expect("two points align");
    assert!(r.a * r.e - r.b * r.d < 0.0);
}

/// Map → drawing → map comes back; fewer than two pairs, or two at one spot, give no fit.
#[test]
fn invert_round_trip_and_no_fit_without_two_spots() {
    let a = align_from_pairs(&square_pairs(100.0, true), false).expect("a fit");
    let src = a.invert(a.apply([200_033.0, 500_077.0])).expect("invertible");
    assert!(off(src, [200_033.0, 500_077.0]) < 1e-6);
    assert!(align_from_pairs(&square_pairs(100.0, false)[..1], false).is_none());
    let same = [pair_at(200_000.0, 500_000.0), pair_at(200_000.0, 500_000.0)];
    assert!(align_from_pairs(&same, false).is_none());
}

#[test]
fn residuals_flag_the_wrong_point() {
    let mut p = square_pairs(100.0, true);
    p[4].map[0] += 3.0; // the centre point was clicked 3 m off
    let r = assess(&p, false);
    assert!(r.measurable);
    assert_eq!(r.residuals.len(), 5);
    assert_eq!(r.max_index, Some(4));
    assert!(r.max_residual > RESIDUAL_WARN_M, "{}", r.max_residual);
    assert!(row_text(&r, 4).contains("(가장 큼)"));
    assert!(row_text(&r, 0).contains(" m"));
    let said = notes(&r);
    assert!(said.first().is_some_and(|n| n.starts_with("주의: 5번 점")), "{said:?}");
    assert!(summary(&r).contains("최대"));
}

/// Three points always fit exactly: no residual is shown, the note says a fourth point measures it.
#[test]
fn three_points_say_the_residual_needs_a_fourth_point() {
    let p = [pair_at(200_000.0, 500_000.0), pair_at(200_100.0, 500_000.0), pair_at(200_000.0, 500_100.0)];
    let r = assess(&p, false);
    assert!(!r.measurable && r.residuals.is_empty());
    assert!(!row_text(&r, 0).contains("어긋남"), "a 0.00 m residual would mislead");
    assert!(notes(&r).iter().any(|n| n == "점 4개부터 점마다 어긋남을 잽니다"));
    assert!(summary(&r).contains("한 점 더"));
}

#[test]
fn anisotropy_and_shear_are_warned() {
    // x keeps its scale, y is stretched 20 %: a wrongly clicked third point looks like this.
    let p = [AlignPair { src: [0.0, 0.0], map: [1000.0, 2000.0] }, AlignPair { src: [100.0, 0.0], map: [1100.0, 2000.0] }, AlignPair { src: [0.0, 100.0], map: [1000.0, 2120.0] }];
    let r = assess(&p, false);
    let shape = r.shape.expect("a shape");
    assert!((shape.anisotropy - 1.2).abs() < 1e-9, "{}", shape.anisotropy);
    assert!(notes(&r).join("|").contains("20% 다릅니다"));

    let p = [AlignPair { src: [0.0, 0.0], map: [1000.0, 2000.0] }, AlignPair { src: [100.0, 0.0], map: [1100.0, 2000.0] }, AlignPair { src: [0.0, 100.0], map: [1020.0, 2100.0] }];
    let r = assess(&p, false);
    let shear = r.shape.expect("a shape").shear_deg;
    assert!(shear > 10.0 && shear < 12.0, "{shear}");
    assert!(notes(&r).join("|").contains("비뚤어집니다"));
}

/// Scan pixels onto the map by rotation, scale and the flip only: nothing to warn about.
#[test]
fn a_similarity_has_no_warnings() {
    let (th, k) = (0.2_f64, 0.05);
    let p: Vec<AlignPair> = [[0.0, 0.0], [4000.0, 0.0], [4000.0, 3000.0], [0.0, 3000.0]]
        .iter()
        .map(|s| AlignPair { src: *s, map: [194_000.0 + k * (th.cos() * s[0] + th.sin() * s[1]), 574_000.0 + k * (th.sin() * s[0] - th.cos() * s[1])] })
        .collect();
    let r = assess(&p, true);
    assert!(r.measurable && r.max_residual < 1e-6, "{}", r.max_residual);
    let shape = r.shape.expect("a shape");
    assert!((shape.anisotropy - 1.0).abs() < 1e-9 && shape.shear_deg < 1e-6);
    assert!(notes(&r).is_empty(), "{:?}", notes(&r));
}
