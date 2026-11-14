//! Ported straight-trench plan tests 2/2 (`target/cpp-ref/tests/test_dem_trench.cpp`, `target/cpp-ref/tests/test_trench_plan.cpp`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use geo::algorithm::bool_ops::BooleanOps;
use geo::{wkt, Area, Coord, Geometry, Intersects, LineString, MultiPolygon, Polygon};

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

fn cell_polygon(cell: &Cell) -> Polygon<f64> {
    Polygon::new(
        LineString::from(
            cell.ring
                .iter()
                .map(|&[x, y]| Coord { x, y })
                .collect::<Vec<_>>(),
        ),
        vec![],
    )
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:295-309` (`buildForTargetRatio_hitsTenPercent`).
#[test]
fn build_for_target_ratio_hits_ten_percent() {
    let area = square_geom(0.0, 0.0, 100.0, 100.0);
    let plan = build_for_target_ratio(Some(&area), 10.0, 2.0, 0.0);
    assert!(!plan.cells.is_empty(), "{}", plan.error);
    for c in &plan.cells {
        assert!((c.width - 2.0).abs() < 1e-9);
        assert!(c.length > 0.0);
        for pt in &c.ring {
            assert!((-1e-6..=100.0 + 1e-6).contains(&pt[0]));
            assert!((-1e-6..=100.0 + 1e-6).contains(&pt[1]));
        }
    }
    let pct = total_area(&plan.cells) / 10_000.0 * 100.0;
    assert!((pct - 10.0).abs() < 1e-7, "pct={pct}");
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:311-320` (`buildForTargetRatio_hitsTwoPercent`).
#[test]
fn build_for_target_ratio_hits_two_percent() {
    let area = square_geom(0.0, 0.0, 100.0, 100.0);
    let plan = build_for_target_ratio(Some(&area), 2.0, 2.0, 0.0);
    assert!(!plan.cells.is_empty(), "{}", plan.error);
    assert!(plan.cells[0].name.starts_with("Sp-"));
    for c in &plan.cells {
        assert!((c.width - 2.0).abs() < 1e-9);
    }
    let pct = total_area(&plan.cells) / 10_000.0 * 100.0;
    assert!((pct - 2.0).abs() < 1e-7, "pct={pct}");
    assert!(pct < 8.0);
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:366-382` (`buildForTargetRatio_honoursTerrainAzimuth`).
#[test]
fn build_for_target_ratio_honours_terrain_azimuth() {
    let area = square_geom(200_000.0, 450_000.0, 200_200.0, 450_200.0);
    let plan = build_for_target_ratio(Some(&area), 10.0, 2.0, 45.0);
    assert!(!plan.cells.is_empty(), "{}", plan.error);
    assert!((plan.azimuth_deg - 45.0).abs() < 1e-12);
    assert!((plan.ratio_pct - 10.0).abs() < 1.5, "{}", plan.ratio_pct);

    let c = &plan.cells[0];
    let dx = c.ring[3][0] - c.ring[0][0];
    let dy = c.ring[3][1] - c.ring[0][1];
    let mut az = dx.atan2(dy).to_degrees();
    if az < 0.0 {
        az += 360.0;
    }
    assert!((az - 45.0).abs() < 0.5, "az={az}");
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:489-538` (`ratioFill_respectsGeometryContract`).
#[test]
fn ratio_fill_respects_geometry_contract() {
    let concave = Geometry::Polygon(wkt!(
        POLYGON((0.0 0.0, 100.0 0.0, 100.0 30.0, 30.0 30.0, 30.0 100.0, 0.0 100.0, 0.0 0.0))
    ));
    let hole = Geometry::Polygon(wkt!(
        POLYGON(
            (0.0 0.0, 100.0 0.0, 100.0 100.0, 0.0 100.0, 0.0 0.0),
            (30.0 30.0, 30.0 70.0, 70.0 70.0, 70.0 30.0, 30.0 30.0)
        )
    ));
    let rows: [(&str, Geometry<f64>, f64, f64); 7] = [
        ("formerly-24m", square_geom(0.0, 0.0, 25.0, 115.0), 10.0, 0.0),
        ("5186-trial", square_geom(200_000.0, 550_000.0, 200_100.0, 550_100.0), 10.0, 0.0),
        ("5187-sample", square_geom(200_000.0, 450_000.0, 200_100.0, 450_100.0), 2.0, 0.0),
        ("rotated", square_geom(200_000.0, 450_000.0, 200_100.0, 450_100.0), 10.0, 45.0),
        ("concave", concave, 10.0, 45.0),
        ("hole", hole, 10.0, 0.0),
        ("small-sample", square_geom(0.0, 0.0, 10.0, 10.0), 2.0, 0.0),
    ];
    for (label, area, target, azimuth) in rows {
        let plan = build_for_target_ratio(Some(&area), target, 2.0, azimuth);
        assert!(!plan.cells.is_empty(), "{label}: {}", plan.error);
        assert!(plan.error.is_empty(), "{label}");
        let area_mp = match &area {
            Geometry::Polygon(p) => MultiPolygon::new(vec![p.clone()]),
            Geometry::MultiPolygon(mp) => mp.clone(),
            _ => unreachable!(),
        };
        let mut united = MultiPolygon::new(Vec::new());
        let mut sum = 0.0_f64;
        let mut max_length = 0.0_f64;
        for cell in &plan.cells {
            let width = (cell.ring[1][0] - cell.ring[0][0]).hypot(cell.ring[1][1] - cell.ring[0][1]);
            let length = (cell.ring[3][0] - cell.ring[0][0]).hypot(cell.ring[3][1] - cell.ring[0][1]);
            assert!(width > 0.0 && width <= 2.0 + 1e-7, "{label}");
            assert!(length > 0.0 && length <= 20.0 + 1e-7, "{label}");
            assert!((width - cell.width).abs() < 1e-7, "{label}");
            assert!((length - cell.length).abs() < 1e-7, "{label}");
            let poly = cell_polygon(cell);
            let outside = MultiPolygon::new(vec![poly.clone()]).difference(&area_mp);
            assert!(outside.unsigned_area() < 1e-7, "{label}");
            sum += poly.unsigned_area();
            max_length = max_length.max(length);
            if united.0.is_empty() || !united.intersects(&poly) {
                united.0.push(poly);
            } else {
                united = united.union(&MultiPolygon::new(vec![poly]));
            }
        }
        let union_area = united.unsigned_area();
        let target_area = area.unsigned_area() * target / 100.0;
        assert!((sum - union_area).abs() < 1e-6, "{label}: overlap");
        assert!(
            (union_area - target_area).abs() < 1e-6_f64.max(target_area * 1e-8),
            "{label}: actual {union_area}, target {target_area}"
        );
        assert!((plan.ratio_pct - target).abs() < 1e-7, "{label}");
        assert!((plan.length - max_length).abs() < 1e-7, "{label}");
        assert!(meets_target(&plan.cells, plan.area_m2, target), "{label}");
    }
}

/// Ported from `target/cpp-ref/tests/test_dem_trench.cpp:540-548` (`ratioFill_rejectsNarrowArea`).
#[test]
fn ratio_fill_rejects_narrow_area() {
    let area = square_geom(0.0, 0.0, 1.5, 12.0);
    let spec = Spec { trench_length: 8.0, ..Spec::default() };
    assert!(build_in_area(&spec, Some(&area)).is_err());
    let plan = build_for_target_ratio(Some(&area), 10.0, 2.0, 0.0);
    assert!(plan.cells.is_empty());
    assert_eq!(
        plan.error,
        "현재 구역과 방향에서 폭 2 m·길이 20 m 이하로 목표 10%를 배치할 수 없습니다. 기존 격자는 유지됩니다."
    );
}

/// Ported from `target/cpp-ref/tests/test_trench_plan.cpp:65-86` (`numbersRunFromTheHighSideDown`).
#[test]
fn numbers_run_from_the_high_side_down() {
    let mut spec = Spec { trench_length: 10.0, balk_width: 6.0, ..Spec::default() };
    let centre = |c: &Cell| -> [f64; 2] {
        [(c.ring[0][0] + c.ring[2][0]) * 0.5, (c.ring[0][1] + c.ring[2][1]) * 0.5]
    };
    let number = |c: &Cell| -> i32 { c.name[3..].parse::<i32>().expect("Tr-n") };
    let area = square_geom(0.0, 0.0, 60.0, 60.0);
    let mut north = build_in_area(&spec, Some(&area)).expect("north grid");
    assert!(north.len() > 4);
    north.sort_by_key(number);
    for pair in north.windows(2) {
        let before = centre(&pair[0]);
        let after = centre(&pair[1]);
        assert!(
            before[1] > after[1] + 0.5 || ((before[1] - after[1]).abs() <= 0.5 && before[0] < after[0]),
            "{}",
            pair[1].name
        );
    }
    spec.azimuth_deg = 90.0;
    let east = build_in_area(&spec, Some(&area)).expect("east grid");
    let first = east.iter().find(|c| number(c) == 1).expect("Tr-1");
    for cell in &east {
        assert!(centre(first)[0] >= centre(cell)[0] - 0.5);
    }
}

/// Ported from `target/cpp-ref/tests/test_trench_plan.cpp:89-100` (`invalidAreaExplainsWhyGridIsEmpty`).
#[test]
fn invalid_area_explains_why_grid_is_empty() {
    let bowtie = Geometry::Polygon(wkt!(
        POLYGON((0.0 0.0, 100.0 100.0, 100.0 0.0, 0.0 100.0, 0.0 0.0))
    ));
    let spec = Spec { balk_width: 10.0, ..Spec::default() };
    let reason = build_in_area(&spec, Some(&bowtie)).expect_err("bowtie must fail");
    assert!(reason.contains("유효하지 않은"), "{reason}");
    let plan = build_for_target_ratio(Some(&bowtie), 10.0, 2.0, 0.0);
    assert!(plan.cells.is_empty());
    assert_eq!(plan.error, reason);
}

/// Ported from `target/cpp-ref/tests/test_trench_plan.cpp:102-117` (`everyEmptyGridHasAReason`).
#[test]
fn every_empty_grid_has_a_reason() {
    let mut spec = Spec { balk_width: 10.0, ..Spec::default() };
    let square = square_geom(0.0, 0.0, 100.0, 100.0);
    assert!(!build_in_area(&spec, Some(&square)).expect("100x100 fits").is_empty());
    let narrow = square_geom(0.0, 0.0, 1.5, 12.0);
    let reason_narrow = build_in_area(&spec, Some(&narrow)).expect_err("narrow must fail");
    assert_eq!(
        reason_narrow,
        "현재 규격과 방향으로는 구역 안에 들어가는 트렌치가 없습니다. 구역이 좁거나 짧으면 회전이나 규격을 바꿔 보세요."
    );
    let reason_none = build_in_area(&spec, None).expect_err("missing area must fail");
    assert_eq!(reason_none, "조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.");
    let empty_poly = Geometry::Polygon(Polygon::new(LineString::new(Vec::new()), Vec::new()));
    assert_eq!(
        area_problem(Some(&empty_poly)).as_deref(),
        Some("조사구역 면을 읽지 못했습니다. 조사구역을 다시 그리거나 선택하세요.")
    );
    let line_geom = Geometry::LineString(LineString::from(vec![
        Coord { x: 0.0, y: 0.0 },
        Coord { x: 10.0, y: 10.0 },
    ]));
    assert_eq!(
        area_problem(Some(&line_geom)).as_deref(),
        Some("조사구역이 면(폴리곤)이 아니어서 시굴격자를 놓을 수 없습니다.")
    );
    let huge = square_geom(0.0, 0.0, 10_000.0, 10_000.0);
    let dense = Spec { trench_width: 2.0, trench_length: 2.0, balk_width: 0.0, ..Spec::default() };
    assert_eq!(
        build_in_area(&dense, Some(&huge)).expect_err("too many cells"),
        "구역에 비해 트렌치 간격이 좁아 칸 수가 한도를 넘습니다. 둑(간격)을 키우세요."
    );
    spec.trench_width = 2.5;
    let reason_wide = build_in_area(&spec, Some(&square)).expect_err("wide spec must fail");
    assert_eq!(
        reason_wide,
        "트렌치 규격이 맞지 않습니다. 폭은 2 m 이하, 길이는 20 m 이하, 둑은 0 m 이상이어야 합니다."
    );
}

/// Ported from `target/cpp-ref/tests/test_trench_plan.cpp:119-130` (`cellsValidMatchesWriterRules`)
/// and straight checks of `target/cpp-ref/tests/test_trench_lshape.cpp:138-140` (`targetCheckIsExactForStraightAndJustOverForL`).
#[test]
fn cells_valid_matches_writer_rules() {
    let straight = build(&Spec::default());
    assert!(meets_target(&straight, 400.0, 10.0));
    assert!(!meets_target(&straight, 390.0, 10.0));
    assert!(meets_target(&straight, 1_000.0, 0.0));
    let spec = Spec { cols: 2, ..Spec::default() };
    let mut cells = build(&spec);
    assert_eq!(cells.len(), 2);
    assert!(cells_valid(&cells).is_ok());
    cells[1].length = 21.0;
    let err = cells_valid(&cells).expect_err("length 21 must fail");
    assert_eq!(err, "시굴격자 규격과 도형이 일치하지 않습니다. 폭은 최대 2 m, 길이는 최대 20 m입니다.");
    let empty_err = cells_valid(&[]).expect_err("empty cells must fail");
    assert_eq!(empty_err, "기록할 시굴격자가 없거나 개수가 너무 많습니다.");
}

/// Director's observed C++ summary on `samples/demo_survey/demo.gpkg` (`survey_area` fid=1, azimuth 0, 10%).
#[test]
fn demo_survey_ten_percent_matches_observed_cpp_summary() {
    let demo_area = Geometry::Polygon(Polygon::new(
        LineString::from(vec![
            Coord { x: 199_990.0, y: 449_990.0 },
            Coord { x: 200_110.0, y: 449_990.0 },
            Coord { x: 200_110.0, y: 450_090.0 },
            Coord { x: 199_990.0, y: 450_090.0 },
            Coord { x: 199_990.0, y: 449_990.0 },
        ]),
        vec![],
    ));
    let plan = build_for_target_ratio(Some(&demo_area), 10.0, 2.0, 0.0);
    assert!(plan.error.is_empty(), "{}", plan.error);
    assert_eq!(plan.cells.len(), 50);
    for cell in &plan.cells {
        assert!((cell.width - 2.0).abs() < 1e-9);
    }
    assert!((plan.length - 12.0).abs() < 1e-9, "length={}", plan.length);
    assert!((plan.balk - 10.0).abs() < 1e-9, "balk={}", plan.balk);
    let total = total_area(&plan.cells);
    assert!((total - 1_200.0).abs() < 1e-9, "total={total}");
    assert!((plan.ratio_pct - 10.0).abs() < 1e-9, "ratio={}", plan.ratio_pct);
}
