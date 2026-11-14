//! Verifies straight trial-trench planning against 27 C++ `TrenchGridGenerator` outputs
//! (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:72-529`).

use std::fs;
use std::path::Path;

use geo::{Coord, Geometry, LineString, MultiPolygon, Polygon};
use serde::Deserialize;
use strata_geo::trench_plan::{
    area_problem, build_for_target_ratio, build_in_area, meets_target, total_area, Cell, Spec,
};

const TOL: f64 = 1e-9;

#[derive(Debug, Deserialize)]
struct TruthCell {
    name: String,
    width: f64,
    length: f64,
    ring: Vec<[f64; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum TruthCase {
    #[serde(rename = "ratio")]
    Ratio {
        id: String,
        wkt: String,
        target_pct: f64,
        width: f64,
        azimuth: f64,
        count: usize,
        length: f64,
        balk: f64,
        ratio_pct: f64,
        area_m2: f64,
        azimuth_out: f64,
        total_area: f64,
        meets_target: bool,
        error: String,
        cells: Vec<TruthCell>,
    },
    #[serde(rename = "area")]
    Area {
        id: String,
        wkt: String,
        width: f64,
        length: f64,
        balk: f64,
        azimuth: f64,
        count: usize,
        total_area: f64,
        reason: String,
        cells: Vec<TruthCell>,
    },
    #[serde(rename = "problem")]
    Problem {
        id: String,
        wkt: String,
        problem: String,
    },
}

fn split_paren_groups(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (idx, ch) in body.char_indices() {
        match ch {
            '(' => {
                if depth == 0 {
                    start = Some(idx);
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(s) = start.take() {
                        out.push(&body[s..=idx]);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn strip_outer_parens(s: &str) -> &str {
    let t = s.trim();
    assert!(
        t.starts_with('(') && t.ends_with(')'),
        "expected parenthesised WKT group, got: {t}"
    );
    t[1..t.len() - 1].trim()
}

fn parse_ring(s: &str) -> LineString<f64> {
    let inner = strip_outer_parens(s);
    let coords: Vec<Coord<f64>> = inner
        .split(',')
        .map(|pair| {
            let mut nums = pair.split_whitespace();
            let x: f64 = nums
                .next()
                .expect("WKT x")
                .parse()
                .expect("valid f64 x");
            let y: f64 = nums
                .next()
                .expect("WKT y")
                .parse()
                .expect("valid f64 y");
            assert!(nums.next().is_none(), "unexpected extra coordinate in {pair}");
            Coord { x, y }
        })
        .collect();
    LineString::from(coords)
}

fn parse_polygon_body(s: &str) -> Polygon<f64> {
    let inner = strip_outer_parens(s);
    let groups = split_paren_groups(inner);
    assert!(!groups.is_empty(), "empty POLYGON rings in {s}");
    let exterior = parse_ring(groups[0]);
    let interiors = groups[1..].iter().map(|g| parse_ring(g)).collect();
    Polygon::new(exterior, interiors)
}

fn parse_wkt(wkt: &str) -> Geometry<f64> {
    let t = wkt.trim();
    if let Some(rest) = t.strip_prefix("POLYGON") {
        Geometry::Polygon(parse_polygon_body(rest))
    } else if let Some(rest) = t.strip_prefix("MULTIPOLYGON") {
        let inner = strip_outer_parens(rest);
        let polys = split_paren_groups(inner)
            .into_iter()
            .map(parse_polygon_body)
            .collect();
        Geometry::MultiPolygon(MultiPolygon::new(polys))
    } else {
        panic!("unsupported WKT geometry in fixture: {t}");
    }
}

fn compare_cells(case_id: &str, got: &[Cell], expected: &[TruthCell]) -> f64 {
    assert_eq!(got.len(), expected.len(), "{case_id}: cell count mismatch");
    let mut max_diff = 0.0_f64;
    for (idx, (cell, exp)) in got.iter().zip(expected).enumerate() {
        assert_eq!(cell.name, exp.name, "{case_id}: cell[{idx}] name mismatch");
        assert_eq!(
            cell.ring.len(),
            exp.ring.len(),
            "{case_id}: cell {} ring vertex count mismatch",
            cell.name
        );
        max_diff = max_diff.max((cell.width - exp.width).abs());
        max_diff = max_diff.max((cell.length - exp.length).abs());
        for (pt, exp_pt) in cell.ring.iter().zip(&exp.ring) {
            max_diff = max_diff.max((pt[0] - exp_pt[0]).abs());
            max_diff = max_diff.max((pt[1] - exp_pt[1]).abs());
        }
    }
    max_diff
}

#[test]
fn cpp_trench_truth_matches_all_27_cases_within_1e9() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cpp_trench_truth.jsonl");
    let raw = fs::read_to_string(&path).expect("read cpp_trench_truth.jsonl");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 27, "expected 27 C++ truth cases");

    let mut failures = Vec::new();
    for line in lines {
        let case: TruthCase = serde_json::from_str(line).expect("parse fixture JSON line");
        match case {
            TruthCase::Ratio {
                id,
                wkt,
                target_pct,
                width,
                azimuth,
                count,
                length,
                balk,
                ratio_pct,
                area_m2,
                azimuth_out,
                total_area: exp_total_area,
                meets_target: exp_meets_target,
                error,
                cells,
            } => {
                let geom = parse_wkt(&wkt);
                let got = build_for_target_ratio(Some(&geom), target_pct, width, azimuth);
                assert_eq!(got.cells.len(), count, "{id}: count");
                assert_eq!(got.balk, balk, "{id}: balk");
                assert_eq!(got.error, error, "{id}: error");
                assert_eq!(
                    meets_target(&got.cells, got.area_m2, target_pct),
                    exp_meets_target,
                    "{id}: meets_target"
                );
                let ring_len_max = compare_cells(&id, &got.cells, &cells);
                let mut max_diff = ring_len_max;
                max_diff = max_diff.max((got.length - length).abs());
                max_diff = max_diff.max((got.ratio_pct - ratio_pct).abs());
                max_diff = max_diff.max((got.area_m2 - area_m2).abs());
                max_diff = max_diff.max((got.azimuth_deg - azimuth_out).abs());
                max_diff = max_diff.max((total_area(&got.cells) - exp_total_area).abs());
                println!(
                    "case {id} (ratio): max_diff = {max_diff:.6e} (ring_or_len = {ring_len_max:.6e})"
                );
                if max_diff > TOL {
                    failures.push(format!("{id}: max_diff = {max_diff:.6e} > {TOL:.1e}"));
                }
            }
            TruthCase::Area {
                id,
                wkt,
                width,
                length,
                balk,
                azimuth,
                count,
                total_area: exp_total_area,
                reason,
                cells,
            } => {
                let geom = parse_wkt(&wkt);
                let spec = Spec {
                    origin_x: 0.0,
                    origin_y: 0.0,
                    trench_width: width,
                    trench_length: length,
                    balk_width: balk,
                    rows: 1,
                    cols: 1,
                    azimuth_deg: azimuth,
                    name_prefix: "Tr-".to_owned(),
                };
                match build_in_area(&spec, Some(&geom)) {
                    Ok(got_cells) => {
                        assert_eq!(reason, "", "{id}: expected error reason {reason}");
                        assert_eq!(got_cells.len(), count, "{id}: count");
                        let ring_len_max = compare_cells(&id, &got_cells, &cells);
                        let mut max_diff = ring_len_max;
                        max_diff = max_diff.max((total_area(&got_cells) - exp_total_area).abs());
                        println!(
                            "case {id} (area): max_diff = {max_diff:.6e} (ring_or_len = {ring_len_max:.6e})"
                        );
                        if max_diff > TOL {
                            failures.push(format!("{id}: max_diff = {max_diff:.6e} > {TOL:.1e}"));
                        }
                    }
                    Err(got_reason) => {
                        assert_eq!(count, 0, "{id}: unexpected empty result: {got_reason}");
                        assert!(cells.is_empty(), "{id}: expected no cells");
                        assert_eq!(got_reason, reason, "{id}: reason");
                        println!("case {id} (area-empty): max_diff = 0.000e0");
                    }
                }
            }
            TruthCase::Problem { id, wkt, problem } => {
                let geom = parse_wkt(&wkt);
                let got = area_problem(Some(&geom)).unwrap_or_default();
                assert_eq!(got, problem, "{id}: problem text");
                println!("case {id} (problem): max_diff = 0.000e0");
            }
        }
    }

    assert!(
        failures.is_empty(),
        "cases exceeding {TOL:.1e} tolerance:\n{}",
        failures.join("\n")
    );
}
