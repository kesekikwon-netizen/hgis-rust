use crate::borehole_strata::*;

#[test]
fn borehole_declustering_keeps_deepest_hole_per_grid_cell_and_normalizes_multipolygons() {
    let h1 = BoreholeRawInput {
        hole_id: "BH-SHALLOW".to_owned(),
        x: 105.0,
        y: 205.0,
        ground_elev_m: 15.0,
        water_depth_m: 2.0,
        strata: vec![StratumLayer {
            depth_from_m: 0.0,
            depth_to_m: 3.0,
            layer_name: "충적층".to_owned(),
            uscs_code: "SM".to_owned(),
            description: "모래".to_owned(),
            n_value: 8.0,
        }],
    };
    let h2 = BoreholeRawInput {
        hole_id: "BH-DEEP".to_owned(),
        x: 112.0,
        y: 208.0,
        ground_elev_m: 15.0,
        water_depth_m: 2.0,
        strata: vec![
            StratumLayer {
                depth_from_m: 0.0,
                depth_to_m: 4.5,
                layer_name: "충적층".to_owned(),
                uscs_code: "CL".to_owned(),
                description: "유기질 점토".to_owned(),
                n_value: 4.0,
            },
            StratumLayer {
                depth_from_m: 4.5,
                depth_to_m: 9.0,
                layer_name: "풍화토".to_owned(),
                uscs_code: "SM".to_owned(),
                description: "풍화대".to_owned(),
                n_value: 50.0,
            },
        ],
    };
    let declustered = decluster_boreholes_by_grid(&[h1, h2], 50.0, 50);
    assert_eq!(declustered.len(), 1);
    assert_eq!(declustered[0].hole_id, "BH-DEEP");

    let polys = normalize_polygons_to_multipolygon(vec![
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], // unclosed triangle -> auto-closed to 4 pts
        vec![[0.0, 0.0], [1.0, 1.0]],                // degenerate 2-pt -> dropped
    ]);
    assert_eq!(polys.len(), 1);
    assert_eq!(polys[0].len(), 4);
}

#[test]
fn test_borehole_declustering_15_holes_within_100m_reduces_to_at_most_4_and_keeps_45m_deepest()
{
    use std::sync::atomic::AtomicBool;

    let holes: Vec<BoreholeRawInput> = (0..15)
        .map(|i| {
            let depth = if i == 7 {
                45.0
            } else {
                5.0 + f64::from(i) * 2.5
            };
            let x = 200_000.0 + f64::from(i % 5) * 12.0;
            let y = 350_000.0 + f64::from(i / 5) * 14.0;
            BoreholeRawInput {
                hole_id: format!("BH-{i:02}"),
                x,
                y,
                ground_elev_m: 20.0,
                water_depth_m: 2.0,
                strata: vec![StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: depth,
                    layer_name: "충적층".to_owned(),
                    uscs_code: "SM".to_owned(),
                    description: "실트질 모래".to_owned(),
                    n_value: 15.0,
                }],
            }
        })
        .collect();

    let cancel = AtomicBool::new(false);
    let declustered = decluster_boreholes_with_cancel(&holes, 30.0, 50, Some(&cancel))
        .expect("decluster should succeed when not cancelled");
    assert!(declustered.len() <= 4);
    assert_eq!(declustered[0].hole_id, "BH-07");
    assert!((declustered[0].strata[0].depth_to_m - 45.0).abs() < 1e-6);

    let cancelled = AtomicBool::new(true);
    assert!(decluster_boreholes_with_cancel(&holes, 30.0, 50, Some(&cancelled)).is_err());
    assert_eq!(
        classify_uscs_to_4_paleo("SM", 15.0, 2.5),
        PaleoSedimentType::LeveeFanSand
    );
    assert_eq!(
        classify_uscs_to_4_paleo("CL", 4.0, 3.0),
        PaleoSedimentType::BackswampClay
    );
    assert_eq!(
        classify_uscs_to_4_paleo("GP", 25.0, 6.0),
        PaleoSedimentType::FluvialTerrace
    );
    assert_eq!(
        classify_uscs_to_4_paleo("SM", 50.0, 12.0),
        PaleoSedimentType::BedrockResidual
    );

    let bh_wt = BoreholeRawInput {
        hole_id: "BH-WT1".to_owned(),
        x: 1000.0,
        y: 2000.0,
        ground_elev_m: 30.0,
        water_depth_m: 2.0, // EL. 28.0m
        strata: vec![],
    };
    let report = evaluate_excavation_safety([1000.0, 2000.0], 27.0, &[bh_wt], 100.0)
        .expect("safety report");
    assert!(report.has_water_table_breach);
    assert!((report.max_breach_depth_m - 1.0).abs() < 1e-9);

    // TestKaMunsellSoilColorParser::testStandardYellowishBrownParsing
    let munsell =
        parse_munsell_notation("10YR 4/6").expect("10YR 4/6 should parse as Yellowish Brown");
    assert_eq!(munsell.hue, "10YR");
    assert!((munsell.value - 4.0).abs() < 1e-6);
    assert!((munsell.chroma - 6.0).abs() < 1e-6);
    assert_eq!(munsell.korean_color_name, "황갈색");
    assert!((140..=170).contains(&munsell.srgb_color[0]));
    assert!((90..=120).contains(&munsell.srgb_color[1]));
    assert!((20..=50).contains(&munsell.srgb_color[2]));
    assert_eq!(
        parse_munsell_notation("10YR 4-6"),
        Err(MunsellParseError::InvalidFormat)
    );

    // TestKaSoilDescriptionGenerator::testSentenceComposition
    let desc = generate_soil_description("황갈색(10YR 4/6)", "사질점토", "미세 목탄립", "치밀");
    assert_eq!(
        desc,
        "황갈색(10YR 4/6)의 사질점토로서, 미세 목탄립이(가) 포함되어 있으며 층위 상태는 치밀함."
    );

    // TestKaStratigraphicColumnLog::testDoubleRulerDepthCalculations
    let tick = compute_double_ruler_tick(25.00, 60.0);
    assert_eq!(tick.depth_label, "60 cm");
    assert_eq!(tick.elevation_label, "EL. 24.40 m");
    assert!((tick.absolute_elevation_m - 24.40).abs() < 1e-6);

    // TestKaPaleoCrossSectionWidget::testBoreholePerpendicularDistance:
    // Section line (0,0) -> (100,0), borehole at (50.0, 30.0) -> d_perp = 30.0m, d_proj = 50.0m
    let proj = project_borehole_onto_section_line(
        "BH-01",
        [50.0, 30.0],
        [0.0, 0.0],
        [100.0, 0.0],
        HGIS_BOREHOLE_MAX_PERP_DIST_M,
    );
    assert!((proj.perp_distance_m - 30.0).abs() <= 0.01);
    assert!((proj.proj_distance_m - 50.0).abs() <= 0.01);
    assert!(proj.within_tolerance);

    // Inverted stratum depth guard (top_depth_m > bottom_depth_m auto-swapped)
    let mut inv = vec![StratumLayer {
        depth_from_m: 4.5,
        depth_to_m: 1.2,
        layer_name: "충적층".to_owned(),
        uscs_code: "SM".to_owned(),
        description: "모래".to_owned(),
        n_value: 10.0,
    }];
    sanitize_borehole_strata_depths(&mut inv);
    assert!((inv[0].depth_from_m - 1.2).abs() < 1e-9);
    assert!((inv[0].depth_to_m - 4.5).abs() < 1e-9);

    // TestGeoInfoBoreholeParser::testXmlStratumParsing:
    // elevation 32.5m, stratum 1 (0.0~1.5m 표토), stratum 2 (1.5~4.2m, SM 모래, N=15)
    let xml = r#"<GeoInfo><Borehole id="BH-1" elevation="32.5" depth="4.2"><Stratum top="0.0" bottom="1.5" uscs="ML" n="4" desc="표토"/><Stratum top="1.5" bottom="4.2" uscs="SM" n="15" desc="SM 모래"/></Borehole></GeoInfo>"#;
    let parsed = parse_geoinfo_borehole_xml(xml);
    assert_eq!(parsed.len(), 1);
    assert!((parsed[0].ground_elev_m - 32.5).abs() < 1e-6);
    assert_eq!(parsed[0].strata.len(), 2);
    assert!((parsed[0].strata[1].depth_to_m - 4.2).abs() < 1e-6);
    assert!((parsed[0].strata[1].n_value - 15.0).abs() < 1e-6);
    assert_eq!(
        classify_uscs_deposit_env_ko("CL", "암회색 점토", 2.7),
        "점토·실트 충적(구하도·저습지)"
    );
}
