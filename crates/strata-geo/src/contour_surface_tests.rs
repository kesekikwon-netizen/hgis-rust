//! A plane rising to the north. Interval 2 m, cell 10 m: one line at 2, 4, 6 and 8 m,
//! each sitting on the height it names (`SurveyContourWrite` linear grid).

#![allow(clippy::unwrap_used)]

use super::surface_contours;

#[test]
fn a_north_slope_emits_one_line_per_interval() {
    let points = [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [0.0, 100.0, 10.0],
        [100.0, 100.0, 10.0],
    ];
    let (_grid, lines) = surface_contours(&points, 2.0, 10.0).unwrap();
    let mut levels: Vec<i32> = lines
        .iter()
        .map(|line| (line.elevation_m * 100.0).round() as i32)
        .collect();
    levels.sort();
    assert_eq!(levels, vec![200, 400, 600, 800]);
    let two = lines
        .iter()
        .find(|line| (line.elevation_m - 2.0).abs() < 1e-9)
        .unwrap();
    assert!(
        two.xy.iter().all(|p| (p[1] - 20.0).abs() < 1.0),
        "{:?}",
        two.xy
    );
    assert!(lines.iter().all(|line| !line.index_line));
}

#[test]
fn two_points_and_a_straight_line_are_refused() {
    assert!(
        surface_contours(&[[0.0, 0.0, 1.0], [1.0, 1.0, 2.0]], 1.0, 1.0)
            .unwrap_err()
            .contains("3개")
    );
    let straight = [[0.0, 0.0, 1.0], [10.0, 0.0, 2.0], [20.0, 0.0, 3.0]];
    assert!(surface_contours(&straight, 1.0, 1.0)
        .unwrap_err()
        .contains("일직선"));
}
