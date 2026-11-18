//! Resolver boundary proof, target/cpp-ref/src/app/KaAlignMapTool.cpp:530-555.
use super::*;
use crate::gui_ribbon_bar::align_snap_sources::{Availability, Policy, Scope, Source};
use std::sync::mpsc;

fn sources() -> Sources {
    let mut table = strata_storage::edit_buffer::LayerEditBuffer::default();
    table.features = vec![strata_storage::edit_buffer::FeatureRecord {
        fid: 7,
        wkt_geom: "POINT(200000 450000)".to_owned(),
        attributes: Default::default(),
    }]
    .into();
    Sources {
        work: 5187,
        policy: Policy {
            enabled: true,
            tolerance: 12.0,
            scope: Scope::All,
        },
        availability: Availability::Ready,
        layers: vec![Source::Survey {
            key: "fixture".to_owned(),
            label: "fixture".to_owned(),
            epsg: 5187,
            rows: table.snapshot(),
        }],
    }
}
fn building(input: &Sources) -> (Resolver, mpsc::Sender<Result<SnapIndex, String>>) {
    let (send, receive) = mpsc::channel();
    (
        Resolver {
            desired: Some(input.clone()),
            active: Some(Job { sources: input.clone(), receive }),
            ..Default::default()
        },
        send,
    )
}
fn index() -> SnapIndex {
    // Exact integer EPSG:5187 geometry; the expected answers need no metric rounding tolerance.
    let mut index = SnapIndex::new();
    index.rebuild(vec![[200_000.0, 450_000.0]], vec![]);
    index
}
fn defer(resolver: &mut Resolver) {
    resolver.defer(Operation::Pick, [200_000.5, 450_000.5], 1.0, &[], Some([32.0, 48.0]));
}

#[test]
fn pending_click_waits_then_uses_its_captured_reach_and_is_taken_once() {
    let input = sources();
    let (mut resolver, send) = building(&input);
    defer(&mut resolver);
    resolver.prepare(input.clone());
    assert_eq!(resolver.take_ready_action(&[], Some([32.0, 48.0])).expect("recorded resolver result"), None);
    assert!(resolver.has_action(), "pending click was silently dropped");
    send.send(Ok(index())).expect("owned worker receiver");
    resolver.prepare(input);
    assert_eq!(
        resolver.take_ready_action(&[], Some([32.0, 48.0])).expect("recorded resolver result"),
        Some((Operation::Pick, [200_000.0, 450_000.0])),
        "ready index did not resolve the original click"
    );
    assert_eq!(resolver.take_ready_action(&[], Some([32.0, 48.0])).expect("recorded resolver result"), None);
}

#[test]
fn changed_scope_cancels_intent_and_rejects_the_old_completed_index() {
    let input = sources();
    let (mut resolver, send) = building(&input);
    defer(&mut resolver);
    let mut changed = input.clone();
    changed.policy.scope = Scope::Current(None);
    changed.layers.clear();
    resolver.prepare(changed.clone());
    assert!(!resolver.has_action(), "scope change kept a stale target click");
    assert!(resolver.active.as_ref().expect("one owned build").sources.same_revision(&input), "a second build replaced the still-running owned worker");
    send.send(Ok(index())).expect("owned worker receiver");
    resolver.prepare(changed.clone());
    assert_eq!(
        resolver.resolve(&changed, [200_000.5, 450_000.5], 1.0),
        Resolution::Ready {
            point: [200_000.5, 450_000.5],
            snapped: false
        }
    );
    assert!(resolver.active.is_none());
}

#[test]
fn a_changed_source_pixel_or_pair_list_cannot_retarget_a_delayed_click() {
    for (pairs, source) in [
        (vec![], Some([33.0, 48.0])),
        (
            vec![AlignPair {
                src: [1.0, 2.0],
                map: [200_010.0, 450_010.0],
            }],
            Some([32.0, 48.0]),
        ),
    ] {
        let input = sources();
        let (mut resolver, _send) = building(&input);
        defer(&mut resolver);
        assert_eq!(resolver.take_ready_action(&pairs, source).expect("recorded resolver result"), None);
        assert!(!resolver.has_action(), "changed alignment still owns the old target click");
    }
}

#[test]
fn cancel_and_clone_cannot_resurrect_a_pending_action_or_share_a_receiver() {
    let input = sources();
    let (mut resolver, send) = building(&input);
    defer(&mut resolver);
    let clone = resolver.clone();
    assert!(clone.active.is_none() && !clone.has_action());
    resolver.cancel();
    send.send(Ok(index())).expect("owned worker receiver");
    resolver.prepare(input);
    assert_eq!(resolver.take_ready_action(&[], Some([32.0, 48.0])).expect("recorded resolver result"), None);
}

#[test]
fn waiting_or_failed_geometry_is_not_a_successful_raw_target() {
    let mut input = sources();
    input.availability = Availability::Waiting;
    let mut resolver = Resolver::default();
    resolver.prepare(input.clone());
    assert_eq!(resolver.resolve(&input, [200_000.5, 450_000.5], 1.0), Resolution::Waiting);
    input.availability = Availability::Failed("recorded failure".to_owned());
    resolver.prepare(input.clone());
    assert_eq!(resolver.resolve(&input, [200_000.5, 450_000.5], 1.0), Resolution::Failed("recorded failure".to_owned()));
}

#[test]
fn ready_no_hit_and_disabled_snap_accept_raw_but_do_not_report_a_hit() {
    let mut input = sources();
    let mut resolver = Resolver {
        desired: Some(input.clone()),
        ready: Some(Ok(Arc::new(index()))),
        ..Default::default()
    };
    assert_eq!(
        resolver.resolve(&input, [200_000.5, 450_000.5], 0.1),
        Resolution::Ready {
            point: [200_000.5, 450_000.5],
            snapped: false
        }
    );
    input.policy.enabled = false;
    resolver.prepare(input.clone());
    assert_eq!(
        resolver.resolve(&input, [200_000.5, 450_000.5], 1.0),
        Resolution::Ready {
            point: [200_000.5, 450_000.5],
            snapped: false
        }
    );
}

#[test]
fn work_crs_source_crs_and_immutable_record_revision_cancel_old_intents() {
    let input = sources();
    let mut variants = vec![input.clone(), input.clone(), sources()];
    variants[0].work = 5186;
    if let Source::Survey { epsg, .. } = &mut variants[1].layers[0] {
        *epsg = 5186;
    }
    // The third input owns a different FeatureSnapshot Arc even with identical records.
    for changed in variants {
        let (mut resolver, _send) = building(&input);
        defer(&mut resolver);
        resolver.prepare(changed.clone());
        assert!(!resolver.has_action(), "changed CRS or immutable geometry revision kept the old target intent");
        assert!(resolver.active.as_ref().expect("one active build").sources.same_revision(&input));
        assert_eq!(resolver.resolve(&changed, [200_000.5, 450_000.5], 1.0), Resolution::Waiting);
    }
}
