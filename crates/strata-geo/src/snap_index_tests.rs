//! Alignment uses nearest vertex or segment, target/cpp-ref/src/core/LayerOps.cpp:2852-2872.
use super::{SnapIndex, SnapKind};

// All coordinates are metres in work CRS EPSG:5187. These representable values have exact analytic answers.
#[test]
fn nearer_segment_is_not_hidden_by_a_more_distant_vertex_within_reach() {
    let mut index = SnapIndex::new();
    index.rebuild(vec![[200_005.0, 450_005.0]], vec![[[199_990.0, 450_001.0], [200_010.0, 450_001.0]]]);
    let hit = index.query([200_000.0, 450_000.0], 10.0).expect("near target");
    assert_eq!(hit.point, [200_000.0, 450_001.0], "a farther vertex overrode the nearest alignment target");
    assert_eq!(hit.kind, SnapKind::Segment);
    assert_eq!(hit.distance, 1.0);
}

#[test]
fn rebuild_removes_the_old_source_geometry() {
    let mut index = SnapIndex::new();
    index.rebuild(vec![[200_000.0, 450_000.0]], vec![[[200_000.0, 450_001.0], [200_010.0, 450_001.0]]]);
    assert!(index.query([200_000.0, 450_000.0], 2.0).is_some());
    index.rebuild(vec![[300_000.0, 450_000.0]], vec![[[300_000.0, 450_001.0], [300_010.0, 450_001.0]]]);
    assert!(index.query([200_000.0, 450_000.0], 2.0).is_none(), "removed geometry remained a snap target");
}

#[test]
fn segment_geometry_distance_wins_over_bbox_and_source_order() {
    let diagonal = [[199_990.0, 449_980.0], [200_010.0, 450_010.0]];
    let nearby = [[199_995.0, 450_001.0], [200_005.0, 450_001.0]];
    // The diagonal's box contains the query, but its line is farther than the horizontal line one metre north.
    for segments in [vec![diagonal, nearby], vec![nearby, diagonal]] {
        let mut index = SnapIndex::new();
        index.rebuild(vec![], segments);
        let hit = index.query([200_000.0, 450_000.0], 5.0).expect("nearest line");
        assert_eq!(hit.point, [200_000.0, 450_001.0]);
        assert_eq!(hit.distance, 1.0);
    }
}

#[test]
fn empty_and_out_of_reach_are_distinct_from_an_exact_point_hit() {
    let mut index = SnapIndex::new();
    assert!(index.query([200_000.0, 450_000.0], 2.0).is_none());
    index.rebuild(vec![[200_000.0, 450_000.0]], vec![]);
    assert_eq!(index.query([200_000.0, 450_000.0], 0.0).expect("exact point").point, [200_000.0, 450_000.0]);
    assert!(index.query([200_004.0, 450_000.0], 2.0).is_none());
}
