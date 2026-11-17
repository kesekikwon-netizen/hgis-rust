//! State/quality contracts, target/cpp-ref/src/app/KaAlignMapTool.cpp:212-219,319-341,500-555.
use super::*;

#[test]
fn map_click_waits_for_a_source_and_each_pair_consumes_the_pending_pixel() {
    let mut session = AlignSessionState { target_layer: Some("recorded scan".to_owned()), ..Default::default() };
    assert!(!session.accept_map([200000.0, 450000.0]));
    session.set_source([10.0, 20.0]);
    assert_eq!(session.pending_source, Some([10.0, 20.0]));
    assert!(session.accept_map([200000.0, 450000.0]));
    assert_eq!(session.pairs, vec![AlignPair { src: [10.0, 20.0], map: [200000.0, 450000.0] }]);
    assert!(session.pending_source.is_none());
    assert!(!session.accept_map([200010.0, 450010.0]));
}

#[test]
fn removing_a_finished_pair_invalidates_the_applied_fit() {
    let mut session = AlignSessionState { applied: true, pairs: vec![AlignPair { src: [0.0, 0.0], map: [200000.0, 450000.0] }], ..Default::default() };
    assert!(session.remove_last_pair());
    assert!(!session.applied, "a changed point set cannot keep an applied fit");
}

#[test]
fn moving_either_side_and_deleting_selected_row_changes_only_that_pair() {
    let mut session = AlignSessionState { pairs: vec![AlignPair::default(); 3], applied: true, ..Default::default() };
    session.move_pair(1, true, [7.0, 9.0]);
    session.move_pair(1, false, [200003.0, 450006.0]);
    assert_eq!(session.pairs[1], AlignPair { src: [7.0, 9.0], map: [200003.0, 450006.0] });
    assert!(!session.applied);
    assert!(session.remove_pair_at(0));
    assert_eq!(session.pairs[0].src, [7.0, 9.0]);
    assert_eq!(session.pairs.len(), 2);
}

#[test]
fn status_uses_cpp_affine_residuals_for_four_nonuniformly_scaled_pairs() {
    let mut session = AlignSessionState { target_layer: Some("recorded scan".to_owned()), ..Default::default() };
    session.pairs = vec![
        AlignPair { src: [0.0, 0.0], map: [200000.0, 450000.0] },
        AlignPair { src: [100.0, 0.0], map: [200200.0, 450025.0] },
        AlignPair { src: [0.0, 100.0], map: [200050.0, 449700.0] },
        AlignPair { src: [100.0, 100.0], map: [200250.0, 449725.0] },
    ];
    assert!(session.status_text().ends_with("평균 어긋남 0.00 m · 최대 0.00 m"), "wrong Helmert placeholder: {}", session.status_text());
}
