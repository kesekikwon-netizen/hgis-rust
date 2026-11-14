//! `sheet_composer` 시험.
//!
//! `sheet_composer` 에서 분리했다.

use strata_types::schema::DomainLayerKey;

use crate::sheet_composer::*;
use crate::sheet_labels::rects_overlap_mm;

#[test]
fn paper_size_and_orientation_change_keeps_decorations_in_bounds_and_preserves_off_page_legend()
{
    let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    assert!(sheet.all_decorations_inside_sheet());
    sheet.set_legend_kept_on_page(false);
    assert!(sheet.legend_rect.x_mm >= sheet.width_mm);

    sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Landscape);
    assert_eq!((sheet.width_mm, sheet.height_mm), (420.0, 297.0));
    assert!(sheet.all_decorations_inside_sheet());
    assert!(!sheet.legend_kept_on_page);
    assert!(sheet.legend_rect.x_mm >= sheet.width_mm);
}

#[test]
fn paper_change_puts_chrome_back_in_bottom_strip_and_recovers_orphaned_legend() {
    let mut sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    // Drag chrome and legend to arbitrary/orphaned coordinates
    sheet.scale_bar_rect.x_mm = 380.0;
    sheet.scale_bar_rect.y_mm = 10.0;
    sheet.north_arrow_rect.x_mm = 5.0;
    sheet.north_arrow_rect.y_mm = 5.0;
    sheet.legend_rect.x_mm = 390.0;
    sheet.legend_rect.y_mm = 270.0;

    // Switch down to A4 Portrait
    sheet.change_paper(SheetPaperSize::A4, SheetOrientation::Portrait);
    assert!(sheet.all_decorations_inside_sheet());
    let expected_band_y = sheet.height_mm - 12.0 - 22.0 + 2.0;
    assert!((sheet.scale_bar_rect.y_mm - expected_band_y).abs() < 1e-6);
    assert!((sheet.crs_label_rect.y_mm - expected_band_y).abs() < 1e-6);
    assert!((sheet.north_arrow_rect.y_mm - expected_band_y).abs() < 1e-6);
}

#[test]
fn map_coordinate_grid_syncs_step_and_offset_only_when_north_up() {
    let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    assert!(sheet.apply_map_coordinate_grid(true, 0.0, 20.0, [222_385.0, 260_115.0]));
    assert!(sheet.grid_follow_map);
    assert!((sheet.grid_interval_m - 20.0).abs() < 1e-6);
    assert!((sheet.grid_offset_xy_m[0] - 5.0).abs() < 1e-6);
    assert!((sheet.grid_offset_xy_m[1] - 15.0).abs() < 1e-6);

    // Rotated map grid cannot be applied to sheet coordinate grid
    assert!(!sheet.apply_map_coordinate_grid(true, 15.0, 20.0, [0.0, 0.0]));
    assert!(!sheet.grid_follow_map);
}

#[test]
fn test_plate_scale_bar_nice_interval_1200_at_60mm_yields_50m_and_raster_mask_alpha_255_76() {
    let interval = DrawingSheet::calculate_scale_bar_nice_interval_m(1_200.0, 60.0);
    assert!((interval - 50.0).abs() < 1e-6);

    let mut rgba = vec![200_u8; 4 * 2 * 4];
    apply_polygon_alpha_mask_rgba(&mut rgba, 2, 2, |x, y| x == 0 && y == 0, 0.3);
    assert_eq!(rgba[3], 255);
    assert_eq!(rgba[7], 76);
    assert_eq!(rgba[11], 76);
}

#[test]
fn test_north_arrow_grid_convergence_and_legend_sort_and_title_block_elide() {

    // 중부원점(127.0E) 정중앙에서는 수렴각 0.0도, 128.0E(위도 36.0N)에서는 양(+)의 수렴각
    let zero_conv = compute_grid_convergence_angle_deg(127.0, 36.0, 127.0);
    assert!(zero_conv.abs() < 1e-9);
    let east_conv = compute_grid_convergence_angle_deg(128.0, 36.0, 127.0);
    assert!((east_conv - 36.0_f64.to_radians().sin()).abs() < 1e-3);

    let mut layers = vec![
        DomainLayerKey::SurveyArea,
        DomainLayerKey::FeaturePoly,
        DomainLayerKey::ArtifactPoint,
        DomainLayerKey::TrialTrench,
        DomainLayerKey::FeatureLine,
    ];
    sort_legend_layers_canonical(&mut layers);
    assert_eq!(
        layers,
        vec![
            DomainLayerKey::ArtifactPoint,
            DomainLayerKey::FeatureLine,
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::TrialTrench,
            DomainLayerKey::SurveyArea,
        ]
    );

    let elided = elide_title_block_text_right("경주 월성 발굴조사 지구 종합현황도", 10);
    assert_eq!(elided.chars().count(), 10);
    assert!(elided.ends_with('…'));
}

#[test]
fn test_scale_bar_nice_1000_yields_50m_and_a4_orientation_swap_preserves_center() {
    let len_1000 = DrawingSheet::compute_nice_scale_bar_length(1_000.0, 40.0, 80.0);
    assert!((len_1000 - 50.0).abs() < 1e-9);
    assert!(
        (DrawingSheet::compute_nice_scale_bar_length(-5.0, 40.0, 80.0) - 50.0).abs() < 1e-9
    );

    // target/cpp-ref/src/app/KaDrawingStudio.h:60-67, KaPaperDialog.cpp:165-168: A4 landscape is 317x220 mm.
    let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Landscape);
    assert_eq!((sheet.width_mm, sheet.height_mm), (317.0, 220.0));
    let new_ext = sheet.set_paper_orientation_preserving_center(
        SheetOrientation::Portrait,
        [199_900.0, 499_900.0, 200_100.0, 500_100.0],
    );
    assert_eq!((sheet.width_mm, sheet.height_mm), (210.0, 297.0));
    let cx = 0.5 * (new_ext[0] + new_ext[2]);
    let cy = 0.5 * (new_ext[1] + new_ext[3]);
    assert!((cx - 200_000.0).abs() < 1e-6);
    assert!((cy - 500_000.0).abs() < 1e-6);
}

#[test]
fn test_exclude_empty_layer_from_legend_4_visible_with_1_empty_yields_3() {
    let candidates = [
        LegendLayerCandidate {
            layer_key: DomainLayerKey::SurveyArea,
            visible: true,
            feature_count: 10,
        },
        LegendLayerCandidate {
            layer_key: DomainLayerKey::TrialTrench,
            visible: true,
            feature_count: 5,
        },
        LegendLayerCandidate {
            layer_key: DomainLayerKey::FeaturePoly,
            visible: true,
            feature_count: 0,
        },
        LegendLayerCandidate {
            layer_key: DomainLayerKey::ArtifactPoint,
            visible: true,
            feature_count: 2,
        },
    ];
    let entries = collect_visible_non_empty_legend_entries(&candidates);
    assert_eq!(entries.len(), 3);
    assert!(!entries.contains(&DomainLayerKey::FeaturePoly));

    let tiny = SheetRectMm {
        x_mm: 20.0,
        y_mm: 30.0,
        width_mm: 1.2,
        height_mm: 0.8,
    };
    let inset_box = compute_inset_overview_box_mm(tiny);
    assert!((inset_box.rect_mm.width_mm - 3.0).abs() < 1e-9);
    assert!((inset_box.rect_mm.height_mm - 3.0).abs() < 1e-9);
    assert!(inset_box.has_crosshair);

    // TestKaFigureNumberingEngine::testCascadingFigureNumbering
    let roles = [
        ViewportRole::MainPlan,
        ViewportRole::CrossSection,
        ViewportRole::DetailFeature,
    ];
    let labels = assign_cascading_figure_numbers(2, &roles);
    assert_eq!(labels, vec!["도면 2-1", "도면 2-2", "도면 2-3"]);
    assert!((sanitize_viewport_scale(0.0) - HGIS_DEFAULT_PLAN_SCALE).abs() < 1e-6);

    // TestKaLabelPlacementEngine::testCollisionAvoidanceLeaderLine
    let obstacles = [SheetRectMm {
        x_mm: 40.0,
        y_mm: 40.0,
        width_mm: 20.0,
        height_mm: 20.0,
    }];
    let tasks = [LabelPlacementTask {
        fid: 1,
        label_text: "1호 주거지".to_string(),
        feature_anchor_point_mm: [50.0, 50.0],
        text_size_mm: [14.0, 5.0],
        priority: 1.0,
    }];
    let viewport = SheetRectMm {
        x_mm: 0.0,
        y_mm: 0.0,
        width_mm: 200.0,
        height_mm: 200.0,
    };
    let placed = solve_label_placement(&tasks, &obstacles, viewport);
    assert_eq!(placed.len(), 1);
    assert!(placed[0].needs_leader_line);
    assert_eq!(
        placed[0].leader_line_path_mm.first().copied(),
        Some([50.0, 50.0])
    );
    assert!(rects_overlap_mm(placed[0].final_label_rect_mm, obstacles[0]) <= 1e-9);

    // TestLayoutScaleBar::testDoubleBoxSegmentWidthCalculation
    let seg_w = compute_scalebar_segment_width_mm(500.0, 10.0);
    assert!((seg_w - 20.0).abs() < 1e-4);
    assert!((compute_scalebar_segment_width_mm(500.0, 0.0) - 2.0).abs() < 1e-4);

    // TestLayoutNorthArrow::testNorthArrowRotationWithDeclination
    let rot = compute_north_arrow_rotation_with_declination(30.0, -5.5);
    assert!((rot - 24.5).abs() < 1e-6);

    // TestPrintLayoutTemplate::testSlotSubstitution
    let qpt = r#"<LayoutItemLabel text="조사명: [% $SURVEY_NAME %]"/>"#;
    let instantiated =
        substitute_qpt_template_slots(qpt, &[("$SURVEY_NAME", "경주 황남동 유적")]);
    assert_eq!(
        instantiated,
        r#"<LayoutItemLabel text="조사명: 경주 황남동 유적"/>"#
    );

    // TestHeritageLayoutNumbers::testNorthWestOrdering
    let sites = [
        HeritageSiteCandidate {
            site_name: "유적B".to_owned(),
            era_category: "신라".to_owned(),
            dataset: HeritageDatasetKind::DesignatedHeritage,
            pos_xy: [300.0, 200.0],
        },
        HeritageSiteCandidate {
            site_name: "유적A".to_owned(),
            era_category: "청동기".to_owned(),
            dataset: HeritageDatasetKind::BuriedHeritageArea,
            pos_xy: [100.0, 500.0],
        },
    ];
    let legend = assign_sequential_layout_numbers(&sites, [0.0, 0.0, 1000.0, 1000.0]);
    assert_eq!(legend.len(), 2);
    assert_eq!(legend[0].index_number, 1);
    assert_eq!(legend[0].site_name, "유적A");
    assert_eq!(legend[1].index_number, 2);
    assert_eq!(legend[1].site_name, "유적B");
    assert_eq!(ink_for_badge_rgb([255, 255, 0]), [0, 0, 0]);
    assert_eq!(ink_for_badge_rgb([231, 76, 60]), [255, 255, 255]);
    assert!(layout_badge_circle_size_mm(12) > layout_badge_circle_size_mm(1));
}

#[test]
fn test_world_to_sheet_mm_scale_and_nice_scalebar_grid_and_title_block() {
    let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    sheet.scale_denominator = 5_000;
    let center = [219_745.6, 360_930.5];
    let p_center = sheet.world_to_sheet_mm(center, center);
    let p_east_500m = sheet.world_to_sheet_mm(center, [center[0] + 500.0, center[1]]);
    // At 1:5,000, 500m on ground = 100mm on paper
    assert!((p_east_500m[0] - p_center[0] - 100.0).abs() < 1e-6);

    let seg_m = nice_scale_bar_segment_meters(186.0, 5_000.0, 4);
    assert!((seg_m - 100.0).abs() < 1e-6);
    let grid_m = nice_grid_interval_meters(5_000.0, 186.0);
    assert!((grid_m - 250.0).abs() < 1e-6);

    let tb_lines = format_title_block_lines("조사도면", "조사구역", "", 5_000, "2026-11-08");
    assert_eq!(tb_lines[0], "도면명   조사도면");
    assert_eq!(tb_lines[1], "조사명   조사구역");
    assert_eq!(tb_lines[2], "유적명   ―");
    assert_eq!(tb_lines[3], "축  척   1 : 5000");
    assert_eq!(tb_lines[4], "작성일   2026-11-08");
    assert!(sheet.all_decorations_inside_sheet());
}
