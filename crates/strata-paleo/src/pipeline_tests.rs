
use std::sync::atomic::AtomicBool;

use crate::borehole_strata::{BoreholeRawInput, StratumLayer};
use crate::terrain_edt_tpi::GridTransform;

use crate::pipeline::*;

#[test]
fn paleo_pipeline_classifies_natural_levee_and_generates_cross_section() {
    let tf = GridTransform {
        origin_x: 200_000.0,
        origin_y: 500_000.0,
        cell_size: 10.0,
        width: 32,
        height: 32,
    };
    let z_curr = vec![15.0; 32 * 32];
    let mut river_mask = vec![0u8; 32 * 32];
    river_mask[16 * 32 + 16] = 1;
    let aoi_mask = vec![1u8; 32 * 32];
    let bh = vec![BoreholeRawInput {
        hole_id: "BH-1".into(),
        x: 200_160.0,
        y: 499_840.0,
        ground_elev_m: 15.0,
        water_depth_m: 2.5,
        strata: vec![
            StratumLayer {
                depth_from_m: 0.0,
                depth_to_m: 1.0,
                layer_name: "매립토".into(),
                uscs_code: "SM".into(),
                description: "".into(),
                n_value: 5.0,
            },
            StratumLayer {
                depth_from_m: 1.0,
                depth_to_m: 4.0,
                layer_name: "충적사질토".into(),
                uscs_code: "SM".into(),
                description: "".into(),
                n_value: 12.0,
            },
            StratumLayer {
                depth_from_m: 4.0,
                depth_to_m: 8.0,
                layer_name: "풍화암".into(),
                uscs_code: "WR".into(),
                description: "".into(),
                n_value: 50.0,
            },
        ],
    }];
    let res = run_paleo_pipeline(
        &z_curr,
        &tf,
        &bh,
        &[],
        &[],
        &river_mask,
        &aoi_mask,
        (200_160.0, 499_840.0),
    );
    assert_eq!(res.z_paleo.len(), 32 * 32);
    assert_eq!(res.paleo_rgba_buffer.len(), 32 * 32 * 4);
    assert!(!res.center_cross_section.dist_m.is_empty());
    assert!(res.aoi_summary_text.contains("고지형"));
}

#[test]
fn test_paleo_profile_extraction_500m_cutline_samples_501_points_and_projects_2_boreholes() {
    use std::sync::atomic::AtomicBool;

    let tf = GridTransform {
        origin_x: 200_000.0,
        origin_y: 500_000.0,
        cell_size: 10.0,
        width: 60,
        height: 20,
    };
    let z_curr = vec![22.5; 60 * 20];
    let cut_line = [[200_000.0, 499_900.0], [200_500.0, 499_900.0]];
    let bh_list = vec![
        BoreholeRawInput {
            hole_id: "BH-10M".to_owned(),
            x: 200_120.0,
            y: 499_930.0, // 30 m offset (within 50 m / 100 m buffer)
            ground_elev_m: 22.5,
            water_depth_m: 2.0,
            strata: vec![
                StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: 3.0,
                    layer_name: "매립층".to_owned(),
                    uscs_code: "SM".to_owned(),
                    description: "".to_owned(),
                    n_value: 6.0,
                },
                StratumLayer {
                    depth_from_m: 3.0,
                    depth_to_m: 10.0,
                    layer_name: "충적층".to_owned(),
                    uscs_code: "CL".to_owned(),
                    description: "점토".to_owned(),
                    n_value: 8.0,
                },
            ],
        },
        BoreholeRawInput {
            hole_id: "BH-20M".to_owned(),
            x: 200_380.0,
            y: 499_860.0, // 40 m offset
            ground_elev_m: 22.5,
            water_depth_m: 2.5,
            strata: vec![
                StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: 5.0,
                    layer_name: "충적층".to_owned(),
                    uscs_code: "SP".to_owned(),
                    description: "모래".to_owned(),
                    n_value: 14.0,
                },
                StratumLayer {
                    depth_from_m: 5.0,
                    depth_to_m: 20.0,
                    layer_name: "풍화암".to_owned(),
                    uscs_code: "WR".to_owned(),
                    description: "풍화암".to_owned(),
                    n_value: 50.0,
                },
            ],
        },
    ];

    let abort = AtomicBool::new(false);
    let (stations, elevations, projected) =
        generate_cutline_profile(&cut_line, &tf, &z_curr, &bh_list, 100.0, Some(&abort))
            .expect("profile extraction should succeed");
    assert_eq!(stations.len(), 501);
    assert_eq!(elevations.len(), 501);
    assert_eq!(projected.len(), 2);
    assert!((projected[0].station_m - 120.0).abs() < 1e-6);
    assert!((projected[0].total_depth_m - 10.0).abs() < 1e-6);
    assert_eq!(projected[0].strata_count, 2);
    assert!((projected[1].station_m - 380.0).abs() < 1e-6);
    assert!((projected[1].total_depth_m - 20.0).abs() < 1e-6);
    assert_eq!(projected[1].strata_count, 2);
}

#[test]
fn test_stage2_cancellation_stops_progress_under_50pct_and_returns_user_cancelled() {

    // TestKaPaleoPipelineRunner::testStage2CancellationCleanup:
    // Cancel immediately after stage 1 (20%) before stage 2 finishes -> UserCancelled & progress <= 50%
    let cancel_flag = AtomicBool::new(false);
    let mut reported = Vec::new();
    let (status, last_pct) = run_paleo_pipeline_staged(Some(1), &cancel_flag, |pct, _stage| {
        reported.push(pct);
    });
    assert_eq!(status, PaleoPipelineStatus::UserCancelled);
    assert_eq!(last_pct, HGIS_STAGE1_PROGRESS_END);
    assert!(last_pct <= HGIS_STAGE2_PROGRESS_END);
    assert_eq!(reported, vec![20]);

    // Full run reaches 100% across all 4 stages
    let clean_flag = AtomicBool::new(false);
    let mut all_stages = Vec::new();
    let (ok_status, final_pct) = run_paleo_pipeline_staged(None, &clean_flag, |pct, _stage| {
        all_stages.push(pct);
    });
    assert_eq!(ok_status, PaleoPipelineStatus::Completed);
    assert_eq!(final_pct, 100);
    assert_eq!(all_stages, vec![20, 50, 80, 100]);
}
