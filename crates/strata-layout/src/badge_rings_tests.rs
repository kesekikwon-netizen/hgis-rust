//! The expectations of the C++ placer tests (`target/cpp-ref/tests/test_layout_badge_placer.cpp`), one test
//! function each, with the same inputs: a number stays on its site, moves only when it would cover another
//! one, to the nearest free place, and the lines from sites to numbers never cross.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const STEP: f64 = 10.0;
const BADGE_MM: f64 = 3.0;

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Sites packed inside a fifth of a step around one spot: no two badges fit on them (`:19-24`).
fn crowd(hub: [f64; 2], count: usize) -> Vec<[f64; 2]> {
    (0..count).map(|i| [hub[0] + (i % 3) as f64, hub[1] + (i / 3) as f64 * 0.5]).collect()
}

fn assert_clear(placed: &[[f64; 2]], step: f64) {
    for (i, a) in placed.iter().enumerate() {
        for b in &placed[i + 1..] {
            assert!(distance(*a, *b) >= step * 0.96 - 1e-6, "number circles must not cover each other: {a:?} {b:?}");
        }
    }
}

fn free_placer(step: f64) -> BadgePlacer {
    BadgePlacer::new(ToPaper::IDENTITY, None, step)
}

fn paper(width: f64, height: f64) -> Option<Paper> {
    Some(Paper { frame: [0.0, 0.0, width, height], blocks: Vec::new() })
}

fn on_paper(at: [f64; 2], width: f64, height: f64) -> bool {
    at[0] - 1.5 >= 0.0 && at[1] - 1.5 >= 0.0 && at[0] + 1.5 <= width && at[1] + 1.5 <= height
}

/// Proper crossing of two leader lines (site to badge); shared end points do not count (`:100-105`).
fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let side = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    side(a, b, c) * side(a, b, d) < -1e-9 && side(c, d, a) * side(c, d, b) < -1e-9
}

/// The pseudo-random sites of the C++ tests (`seed * 1664525 + 1013904223`).
fn next(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed >> 8
}

fn all(count: usize) -> Vec<f64> {
    vec![BADGE_MM; count]
}

/// `loneSitesKeepTheirBadgeOnTheSite` (:38-43)
#[test]
fn lone_sites_keep_their_badge_on_the_site() {
    let sites = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
    assert_eq!(free_placer(STEP).place_all(&sites, &all(3), &[]), sites);
}

/// `smallCrowdStaysOnShortRings` (:45-54)
#[test]
fn a_small_crowd_stays_on_short_rings() {
    let sites = [[50.0, 50.0]; 4];
    let placed = free_placer(STEP).place_all(&sites, &all(4), &[]);
    assert_eq!((placed.len(), placed[0]), (4, sites[0]));
    for at in &placed {
        assert!(distance(*at, sites[0]) <= STEP + 1e-6, "a few stacked sites keep their badges one step away");
    }
    assert_clear(&placed, STEP);
}

/// `crowdBadgesStayAttachedToTheirSites` (:56-70)
#[test]
fn crowd_badges_stay_attached_to_their_sites() {
    let sites = crowd([100.0, 100.0], 12);
    let placed = free_placer(STEP).place_all(&sites, &all(12), &[]);
    assert_eq!(placed.len(), sites.len());
    assert_clear(&placed, STEP);
    let mut on_site = 0;
    for (at, site) in placed.iter().zip(&sites) {
        // One badge on the spot, six around it, the rest on the second ring: nothing farther.
        assert!(distance(*at, *site) <= STEP * 2.3, "a badge moves only as far as overlap forces it");
        on_site += usize::from(distance(*at, *site) < 1e-6);
    }
    assert!(on_site >= 1, "the first badge stays on its own site");
}

/// `crowdAtThePaperCornerStaysOnThePaper` (:72-86): map units are millimetres; paper 100 x 100.
#[test]
fn a_crowd_at_the_paper_corner_stays_on_the_paper() {
    let sites = crowd([95.0, 96.0], 20);
    let placed = BadgePlacer::new(ToPaper::IDENTITY, paper(100.0, 100.0), 4.6).place_all(&sites, &all(20), &[]);
    assert_eq!(placed.len(), sites.len());
    assert_clear(&placed, 4.6);
    for at in &placed {
        assert!(on_paper(*at, 100.0, 100.0), "every badge sits fully on the paper: {at:?}");
        // A quarter disc of 20 badges around the corner reaches about five steps.
        assert!(distance(*at, [96.0, 97.0]) <= 4.6 * 6.5, "the badges stay next to the crowd at the corner: {at:?}");
    }
}

/// `loneSiteNextToAFewStackedSitesKeepsItsOwnSite` (:88-96)
#[test]
fn a_lone_site_next_to_a_few_stacked_sites_keeps_its_own_site() {
    let mut sites = vec![[0.0, 0.0]; 4];
    let lone = [STEP, 0.0];
    sites.push(lone);
    let placed = free_placer(STEP).place_all(&sites, &all(5), &[]);
    assert_eq!(placed[4], lone);
    assert_clear(&placed, STEP);
}

/// `crowdLeaderLinesDoNotCrossEachOther` (:107-128)
#[test]
fn crowd_leader_lines_do_not_cross_each_other() {
    let mut seed = 7_u32;
    let sites: Vec<[f64; 2]> = (0..16)
        .map(|_| {
            let x = 90.0 + f64::from(next(&mut seed) % 700) / 100.0;
            [x, 97.0 + f64::from(next(&mut seed) % 700) / 100.0]
        })
        .collect();
    let placed = BadgePlacer::new(ToPaper::IDENTITY, paper(100.0, 200.0), 4.6).place_all(&sites, &all(16), &[]);
    assert_clear(&placed, 4.6);
    for i in 0..sites.len() {
        assert!(on_paper(placed[i], 100.0, 200.0), "every badge sits fully on the paper");
        for j in i + 1..sites.len() {
            assert!(!crosses(sites[i], placed[i], sites[j], placed[j]), "leader lines {i} and {j} cross");
        }
    }
}

/// `leaderLinesOfNeighbouringGroupsDoNotCross` (:130-154)
#[test]
fn leader_lines_of_neighbouring_groups_do_not_cross() {
    for round in 1..=60_u32 {
        let mut seed = round;
        let spread = 1000 * (1 + round % 3);
        let sites: Vec<[f64; 2]> = (0..16)
            .map(|_| {
                let x = 96.0 - f64::from(next(&mut seed) % spread) / 100.0;
                [x, 80.0 + f64::from(next(&mut seed) % (spread * 13 / 10)) / 100.0]
            })
            .collect();
        let placed = BadgePlacer::new(ToPaper::IDENTITY, paper(100.0, 200.0), 4.6).place_all(&sites, &all(16), &[]);
        assert_clear(&placed, 4.6);
        for i in 0..sites.len() {
            for j in i + 1..sites.len() {
                assert!(!crosses(sites[i], placed[i], sites[j], placed[j]), "round {round}: leader lines {i} and {j} cross");
            }
        }
    }
}

/// `crowdedBadgesSitInsideTheirOwnAreaFirst` (:156-167): eight sites on one spot share a strip that runs
/// east; the numbers sit in free places inside it instead of around the spot.
#[test]
fn crowded_badges_sit_inside_their_own_area_first() {
    let sites = [[100.0, 100.0]; 8];
    let strip: Rings = vec![vec![[95.0, 95.0], [200.0, 95.0], [200.0, 105.0], [95.0, 105.0], [95.0, 95.0]]];
    let areas = vec![vec![strip]; 8];
    let placed = free_placer(STEP).place_all(&sites, &all(8), &areas);
    for at in &placed {
        assert!((95.0..=200.0).contains(&at[0]) && (95.0..=105.0).contains(&at[1]), "{at:?}");
    }
    assert_clear(&placed, STEP);
}

/// `loneSiteNextToACrowdKeepsItsOwnSite` (:169-177)
#[test]
fn a_lone_site_next_to_a_crowd_keeps_its_own_site() {
    let mut sites = crowd([100.0, 100.0], 9);
    let lone = [100.0, 130.0];
    sites.push(lone);
    let placed = free_placer(STEP).place_all(&sites, &all(10), &[]);
    assert_eq!(placed[9], lone);
    assert_clear(&placed, STEP);
}

/// `nameBoxesNeverCoverEachOther` (:179-202): names are long boxes; crowded names do not cover each other
/// and a lone name stays in its place.
#[test]
fn name_boxes_never_cover_each_other() {
    let mut sites = vec![[100.0, 100.0]; 5];
    let lone = [100.0, 200.0];
    sites.push(lone);
    let boxes: Vec<[f64; 2]> = (0..sites.len()).map(|i| [20.0 + i as f64 * 4.0, 3.0]).collect();
    let placed = free_placer(3.5).place_all_boxes(&sites, &boxes, &[]);
    assert_eq!((placed.len(), placed[5]), (sites.len(), lone));
    let box_at = |i: usize| [placed[i][0] - boxes[i][0] / 2.0, placed[i][1] - boxes[i][1] / 2.0, placed[i][0] + boxes[i][0] / 2.0, placed[i][1] + boxes[i][1] / 2.0];
    for i in 0..placed.len() {
        assert!(distance(placed[i], sites[i]) <= 30.0, "{:?}", placed[i]);
        for j in i + 1..placed.len() {
            let (a, b) = (box_at(i), box_at(j));
            assert!(a[0] >= b[2] || b[0] >= a[2] || a[1] >= b[3] || b[1] >= a[3], "name {i} covers name {j}");
        }
    }
}

/// `slotsOnRing` (`LayoutBadgePlacer.cpp:36-40`): the places of a ring are one step apart, four at least.
#[test]
fn the_places_of_a_ring_are_one_step_apart() {
    assert_eq!([1, 2, 3, 10].map(BadgePlacer::slots_on_ring), [6, 12, 18, 62]);
    assert_eq!(BadgePlacer::slots_on_ring(0), 6);
}
