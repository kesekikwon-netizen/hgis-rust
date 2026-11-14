//! Ported straight-trench plan tests 1/2 (`target/cpp-ref/tests/test_dem_trench.cpp`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use geo::{Coord, Geometry, LineString, Polygon};

use crate::trench_plan::*;

fn square_geom(x0: f64, y0: f64, x1: f64, y1: f64) -> Geometry<f64> {
    Geometry::Polygon(Polygon::new(
        LineString::from(vec![
            Coord { x: x0, y: y0 },
            Coord { x: x1, y: y0 },
            Coord { x: x1, y: y1 },
            Coord { x: x0, y: y1 },
            Coord { x: x0, y: y0 },
        ]),
        vec![],
    ))
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:143-164` (`trench1x1Is40m2`).
#[test]
fn trench_1x1_is_40m2() {
    let s = Spec {
        origin_x: 200_000.0,
        origin_y: 450_000.0,
        ..Spec::default()
    };
    let cells = build(&s);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].name, "Tr-1");
    assert_eq!(cells[0].ring.len(), 5);
    assert_eq!(cells[0].ring[0], cells[0].ring[4]);
    let mut area = 0.0;
    for i in 0..4 {
        let a = cells[0].ring[i];
        let b = cells[0].ring[i + 1];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area = area.abs() * 0.5;
    assert!((area - 40.0).abs() < 0.01, "area={area}");
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:166-179` (`trenchTwoColsSpacedByBalk`).
#[test]
fn trench_two_cols_spaced_by_balk() {
    let s = Spec {
        trench_length: 10.0,
        cols: 2,
        ..Spec::default()
    };
    let cells = build(&s);
    assert_eq!(cells.len(), 2);
    assert!((cells[1].ring[0][0] - 3.0).abs() < 1e-9);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:184-211` (`buildInAreaKeepsCellsInsideAndRatio`).
#[test]
fn build_in_area_keeps_cells_inside_and_ratio() {
    let s = Spec { balk_width: 10.0, ..Spec::default() };
    let area = square_geom(0.0, 0.0, 100.0, 100.0);
    let cells = build_in_area(&s, Some(&area)).expect("100x100 grid");
    assert_eq!(cells.len(), 36);
    let mut full = 0usize;
    let mut trimmed = 0usize;
    let mut top_most = -1e300_f64;
    for c in &cells {
        assert!((c.width - 2.0).abs() < 1e-12);
        if (c.length - 20.0).abs() < 1e-6 {
            full += 1;
        } else {
            trimmed += 1;
        }
        for pt in &c.ring {
            assert!(pt[0] >= -1e-9 && pt[0] <= 100.0 + 1e-9, "x out of area");
            assert!(pt[1] >= -1e-9 && pt[1] <= 100.0 + 1e-9, "y out of area");
            top_most = top_most.max(pt[1]);
        }
    }
    assert_eq!(full, 27);
    assert_eq!(trimmed, 9);
    assert!(top_most > 99.0);
    let total = total_area(&cells);
    assert!((total - (27.0 * 40.0 + 9.0 * 20.0)).abs() < 0.01, "{total}");
    let pct = total / (100.0 * 100.0) * 100.0;
    assert!(pct > 12.0 && pct < 13.0, "{pct}");
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:214-229` (`buildInArea_shortAreaTrimsInside`).
#[test]
fn build_in_area_short_area_trims_inside() {
    let s = Spec::default();
    let area = square_geom(0.0, 0.0, 12.0, 18.0);
    let cells = build_in_area(&s, Some(&area)).expect("short area trims");
    assert!(!cells.is_empty());
    for c in &cells {
        for point in &c.ring {
            assert!(
                point[0] >= -1e-6
                    && point[0] <= 12.0 + 1e-6
                    && point[1] >= -1e-6
                    && point[1] <= 18.0 + 1e-6
            );
        }
    }
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:232-254` (`pickAutoFillArea_usesNewestNotAllUnion`).
#[test]
fn pick_auto_fill_area_uses_newest_not_all_union() {
    let old_big = square_geom(0.0, 0.0, 1_000.0, 1_000.0);
    let new_small = square_geom(0.0, 0.0, 20.0, 20.0);
    let feats = vec![
        SurveyPoly { area: Some(old_big), fid: 1 },
        SurveyPoly { area: Some(new_small), fid: 2 },
    ];
    let pick = pick_auto_fill_area(&feats, &[], false);
    assert!(!pick.used_selection);
    assert_eq!(pick.used_count, 1);
    assert_eq!(pick.total_count, 2);
    assert!((pick.area_m2 - 400.0).abs() < 1.0, "{}", pick.area_m2);
    let s = Spec { balk_width: 10.0, ..Spec::default() };
    let cells = build_in_area(&s, pick.area.as_ref()).expect("newest area grid");
    assert!(!cells.is_empty());
    assert!(cells.len() < 20);
    for c in &cells {
        for pt in &c.ring {
            assert!(pt[0] <= 20.0 + 1e-6 && pt[1] <= 20.0 + 1e-6);
        }
    }
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:256-276` (`pickAutoFillArea_usesSelectedOnly`).
#[test]
fn pick_auto_fill_area_uses_selected_only() {
    let old_big = square_geom(0.0, 0.0, 100.0, 100.0);
    let new_small = square_geom(200.0, 200.0, 220.0, 220.0);
    let feats = vec![
        SurveyPoly { area: Some(old_big), fid: 1 },
        SurveyPoly { area: Some(new_small), fid: 2 },
    ];
    let pick = pick_auto_fill_area(&feats, &[1], false);
    assert!(pick.used_selection);
    assert_eq!(pick.used_count, 1);
    assert!((pick.area_m2 - 10_000.0).abs() < 1.0, "{}", pick.area_m2);
    let s = Spec { balk_width: 10.0, ..Spec::default() };
    let cells = build_in_area(&s, pick.area.as_ref()).expect("selected area grid");
    assert!(!cells.is_empty());
    for c in &cells {
        let cx = (c.ring[0][0] + c.ring[2][0]) * 0.5;
        let cy = (c.ring[0][1] + c.ring[2][1]) * 0.5;
        assert!((-1e-6..=100.0 + 1e-6).contains(&cx) && (-1e-6..=100.0 + 1e-6).contains(&cy));
    }
    let pick_all = pick_auto_fill_area(&feats, &[], true);
    assert!(!pick_all.used_selection);
    assert_eq!(pick_all.used_count, 2);
    assert!((pick_all.area_m2 - 10_400.0).abs() < 1.0);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:324-351` (`upslopeAspect_pointsUphillNotAlongContour`).
#[test]
fn upslope_aspect_points_uphill_not_along_contour() {
    let mut samples = Vec::new();
    for i in 0..=10 {
        for j in 0..=10 {
            let x = 200_000.0 + f64::from(i) * 10.0;
            let y = 450_000.0 + f64::from(j) * 10.0;
            samples.push(ElevSample { x, y, z: 0.05 * (x - 200_000.0) });
        }
    }
    let asp = upslope_aspect(&samples, DEFAULT_MIN_SLOPE_PCT);
    assert!(asp.valid);
    assert!((asp.azimuth_deg - 90.0).abs() < 0.5, "{}", asp.azimuth_deg);
    assert!((asp.slope_pct - 5.0).abs() < 0.1, "{}", asp.slope_pct);

    let mut ne = Vec::new();
    for i in 0..=10 {
        for j in 0..=10 {
            let x = 200_000.0 + f64::from(i) * 10.0;
            let y = 450_000.0 + f64::from(j) * 10.0;
            ne.push(ElevSample { x, y, z: 0.03 * (x - 200_000.0) + 0.03 * (y - 450_000.0) });
        }
    }
    let asp2 = upslope_aspect(&ne, DEFAULT_MIN_SLOPE_PCT);
    assert!(asp2.valid);
    assert!((asp2.azimuth_deg - 45.0).abs() < 0.5, "{}", asp2.azimuth_deg);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:354-363` (`upslopeAspect_flatGroundHasNoDirection`).
#[test]
fn upslope_aspect_flat_ground_has_no_direction() {
    let mut flat = Vec::new();
    for i in 0..=6 {
        for j in 0..=6 {
            flat.push(ElevSample {
                x: 200_000.0 + f64::from(i) * 10.0,
                y: 450_000.0 + f64::from(j) * 10.0,
                z: 31.4,
            });
        }
    }
    let flat_res = upslope_aspect(&flat, DEFAULT_MIN_SLOPE_PCT);
    assert_eq!(flat_res.samples, 49);
    assert!(!flat_res.valid);

    let two = [
        ElevSample { x: 0.0, y: 0.0, z: 0.0 },
        ElevSample { x: 10.0, y: 0.0, z: 1.0 },
    ];
    let two_res = upslope_aspect(&two, DEFAULT_MIN_SLOPE_PCT);
    assert_eq!(two_res.samples, 2);
    assert!(!two_res.valid);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:386-422` (`buildInArea_trimsEdgeTrenchesInsteadOfDropping`).
#[test]
fn build_in_area_trims_edge_trenches_instead_of_dropping() {
    let area = square_geom(200_000.0, 450_000.0, 200_045.0, 450_045.0);
    let s = Spec {
        trench_width: 2.0,
        trench_length: 20.0,
        balk_width: 6.0,
        azimuth_deg: 0.0,
        ..Spec::default()
    };
    let cells = build_in_area(&s, Some(&area)).expect("45m square grid");
    assert!(!cells.is_empty());
    let mut saw_trimmed = false;
    let mut top_most = -1e300_f64;
    let mut bot_most = 1e300_f64;
    for c in &cells {
        assert!((c.width - 2.0).abs() < 1e-12);
        assert!(c.length >= 4.0 - 1e-9);
        assert!(c.length <= 20.0 + 1e-9);
        if c.length < 20.0 - 1e-6 {
            saw_trimmed = true;
        }
        for pt in &c.ring {
            assert!(pt[0] >= 200_000.0 - 1e-6 && pt[0] <= 200_045.0 + 1e-6);
            assert!(pt[1] >= 450_000.0 - 1e-6 && pt[1] <= 450_045.0 + 1e-6);
            top_most = top_most.max(pt[1]);
            bot_most = bot_most.min(pt[1]);
        }
    }
    assert!(saw_trimmed);
    assert!(bot_most < 450_004.0);
    assert!(top_most > 450_041.0);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:550-574` (`trenchRejectsInvalidSpec`).
#[test]
fn trench_rejects_invalid_spec() {
    assert_eq!(build(&Spec::default()).len(), 1);
    let cases: [(&str, f64); 9] = [
        ("width", 2.01),
        ("length", 20.01),
        ("width", -2.0),
        ("length", -20.0),
        ("width", f64::NAN),
        ("length", f64::INFINITY),
        ("origin", f64::NAN),
        ("azimuth", f64::INFINITY),
        ("balk", -1.0),
    ];
    for (field, value) in cases {
        let mut spec = Spec::default();
        match field {
            "width" => spec.trench_width = value,
            "length" => spec.trench_length = value,
            "origin" => spec.origin_x = value,
            "azimuth" => spec.azimuth_deg = value,
            "balk" => spec.balk_width = value,
            _ => unreachable!(),
        }
        assert!(build(&spec).is_empty(), "field={field}, value={value}");
    }
}
