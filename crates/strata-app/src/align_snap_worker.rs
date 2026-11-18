//! Work-CRS snap index construction, target/cpp-ref/src/app/KaAlignMapTool.cpp:482-497.
use super::align_snap_sources::{Source, Sources};
use strata_engine::Transformer;
use strata_geo::snap_index::SnapIndex;
use strata_storage::{gpkg_wkt::wkt_to_parts, wkb_codec::WkbGeometry};

pub(super) fn build(sources: &Sources) -> Result<SnapIndex, String> {
    let (mut vertices, mut segments) = (Vec::new(), Vec::new());
    // Owned input only. Decode/transform O(vertices), bulk-load O(n log n) off the UI thread.
    // target/cpp-ref/src/core/LayerOps.cpp:2863-2897; KaAlignMapTool.cpp:98-108,482-497.
    for source in &sources.layers {
        match source {
            Source::Survey { label, epsg, rows, .. } => {
                let source_epsg = u32::try_from(*epsg).map_err(|_| format!("Unknown source CRS EPSG:{epsg}"))?;
                let transform = if source_epsg == sources.work {
                    None
                } else {
                    Some(Transformer::new(source_epsg, sources.work).map_err(|e| e.to_string())?)
                };
                for row in rows.records() {
                    for part in wkt_to_parts(&row.wkt_geom).map_err(|e| format!("{label}: {e}"))? {
                        let runs = match part {
                            WkbGeometry::Point(point) => vec![vec![point]],
                            WkbGeometry::LineString(line) => vec![line],
                            WkbGeometry::Polygon(rings) => rings,
                        };
                        for run in runs {
                            let points = run
                                .into_iter()
                                .map(|p| match &transform {
                                    Some(t) => t.transform(p).map_err(|e| e.to_string()),
                                    None => Ok(p),
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            add_run(&points, label, &mut vertices, &mut segments)?;
                        }
                    }
                }
            }
            Source::Reference { label, epsg, data, .. } => {
                if *epsg != sources.work {
                    return Err(format!("{label}: {}", crate::ref_vector::OPEN_FAILED));
                }
                for run in &data.runs {
                    let start = run.start as usize;
                    let points: Vec<_> = data.coords[start..start + run.len as usize].iter().map(|p| [data.origin[0] + f64::from(p[0]), data.origin[1] + f64::from(p[1])]).collect();
                    add_run(&points, label, &mut vertices, &mut segments)?;
                }
            }
        }
    }
    let mut index = SnapIndex::new();
    index.rebuild(vertices, segments);
    Ok(index)
}

fn add_run(points: &[[f64; 2]], label: &str, vertices: &mut Vec<[f64; 2]>, segments: &mut Vec<[[f64; 2]; 2]>) -> Result<(), String> {
    // Reject bad external coordinates before rstar ordering, never treat them as an empty successful layer.
    // target/cpp-ref/src/core/SurveyScopeClip.cpp:97-108.
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return Err(format!("{label}: {}", crate::ref_vector::OPEN_FAILED));
    }
    vertices.extend_from_slice(points);
    segments.extend(points.windows(2).map(|p| [p[0], p[1]]));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::align_snap_sources::{Availability, Policy, Scope, Source};
    use super::*;
    use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};

    fn sources(epsg: i32) -> Sources {
        let mut table = LayerEditBuffer::default();
        table.features = vec![FeatureRecord {
            fid: 7,
            wkt_geom: "LINESTRING(200000 450000,200020 450000)".to_owned(),
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
                key: "feature_line".to_owned(),
                label: "fixture".to_owned(),
                epsg,
                rows: table.snapshot(),
            }],
        }
    }

    #[test]
    fn index_builds_the_owned_source_geometry() {
        let index = build(&sources(5187)).expect("known work CRS");
        let hit = index.query([200_010.0, 450_000.5], 1.0).expect("owned source line missing from index");
        assert_eq!(hit.point, [200_010.0, 450_000.0]);
    }

    #[test]
    fn invalid_source_crs_is_an_error_instead_of_identity_or_empty_success() {
        for epsg in [0, -1, 999_999] {
            assert!(build(&sources(epsg)).is_err(), "source EPSG:{epsg} became a successful index");
        }
    }

    #[test]
    fn different_known_crs_is_converted_by_proj_before_indexing() {
        let reference = strata_engine::Transformer::new(5186, 5187).expect("pinned PROJ control").transform([200_000.0, 450_000.0]).expect("control point");
        let index = build(&sources(5186)).expect("known source CRS");
        let hit = index.query(reference, 0.001).expect("transformed source vertex missing");
        // Wiring against a separate pinned-PROJ control, not an accuracy bound for PROJ's whole area.
        // 1e-8m exceeds16 f64 ULPs at450km. Source contract: target/cpp-ref/src/app/KaAlignMapTool.cpp:98-108.
        for (actual, expected) in hit.point.into_iter().zip(reference) {
            assert!((actual - expected).abs() <= 1e-8);
        }
        assert!(index.query([200_000.0, 450_000.0], 1.0).is_none(), "source CRS numbers were also indexed as work coordinates");
    }

    #[test]
    fn reference_snapshot_in_a_different_work_crs_is_rejected() {
        use crate::ref_vector::data::RefVectorData;
        let mut input = sources(5187);
        input.layers = vec![Source::Reference {
            key: "fixture-reference".to_owned(),
            label: "fixture".to_owned(),
            epsg: 5186,
            data: std::sync::Arc::new(RefVectorData {
                origin: [200_000.0, 450_000.0],
                bbox: [200_000.0, 450_000.0, 200_000.0, 450_000.0],
                coords: vec![],
                runs: vec![],
                items: vec![],
                paints: vec![],
                closed: false,
                rows: 0,
            }),
        }];
        assert!(build(&input).is_err(), "a stale reference work-CRS snapshot was accepted");
    }
}
