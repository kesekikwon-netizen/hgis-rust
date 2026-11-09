use crate::theme_display::{
    ButtonMetrics, DisplayOptions, HGIS_FONT_LARGE_RATIO, HGIS_WCAG_AA_MIN_CONTRAST,
    HGIS_WCAG_AAA_MIN_CONTRAST,
};
use crate::theme_era::{ArchaeologicalFeatureStyleRule, EraPreset};
use crate::theme_splash::{calculate_clamped_popover_position, generate_procedural_strata_curves};
use crate::theme_tokens_base::ThemeTokens;

#[test]
fn paper_look_tokens_satisfy_wcag_contrast_and_clay_accent() {
    let opts = DisplayOptions {
        paper_look: true,
        ..Default::default()
    };
    let t = ThemeTokens::for_options(&opts);
    assert_eq!(t.accent, [0xB5, 0x57, 0x3A, 0xFF]);
    assert_eq!(t.dialog_ground, [0xFA, 0xF9, 0xF5, 0xFF]);
    assert!(t.contrast_ratio(t.ink_muted, t.surface) >= HGIS_WCAG_AA_MIN_CONTRAST);
    assert!(t.contrast_ratio(t.primary_text, t.primary) >= HGIS_WCAG_AA_MIN_CONTRAST);
}

#[test]
fn test_large_font_mode_12pt_to_15pt_and_paper_bg_f5f3ed_fg_222222_wcag_13_72() {
    let base_opts = DisplayOptions::default();
    assert_eq!(base_opts.font_point_size(), 12);
    let large_opts = DisplayOptions {
        large_text: true,
        ..Default::default()
    };
    assert_eq!(large_opts.font_point_size(), 15);
    assert!(
        ((f64::from(base_opts.font_point_size()) * HGIS_FONT_LARGE_RATIO) - 15.0).abs() < 1e-9
    );

    let ratio = ThemeTokens::compute_wcag_contrast_ratio(
        [0x22, 0x22, 0x22, 0xFF],
        [0xF5, 0xF3, 0xED, 0xFF],
    );
    assert!(
        ratio >= 13.7,
        "expected >= 13.7 contrast ratio, got {ratio}"
    );
    assert!(ratio >= HGIS_WCAG_AA_MIN_CONTRAST);
    assert!(ratio >= HGIS_WCAG_AAA_MIN_CONTRAST);
}

#[test]
fn test_three_kingdoms_color_mapping_and_era_presets() {        // TestKaArchaeologicalThemePreset::testThreeKingdomsColorMapping:
    // EraPreset::ThreeKingdoms -> #2C3E50 (RGB: 44, 62, 80)
    assert_eq!(EraPreset::ThreeKingdoms.standard_rgb(), [44, 62, 80]);
    assert_eq!(EraPreset::ThreeKingdoms.hex_color(), "#2C3E50");
    assert_eq!(EraPreset::Paleolithic.standard_rgb(), [110, 44, 0]);
    assert_eq!(EraPreset::BronzeAge.standard_rgb(), [211, 84, 0]);
    assert_eq!(EraPreset::Goryeo.standard_rgb(), [17, 120, 100]);

    let dwelling =
        ArchaeologicalFeatureStyleRule::for_feature_type("Dwelling", EraPreset::ThreeKingdoms);
    assert_eq!(dwelling.stroke_rgb, [44, 62, 80]);
    assert!((dwelling.outline_width_mm - 0.35).abs() < 1e-6);

    let kiln =
        ArchaeologicalFeatureStyleRule::for_feature_type("Kiln", EraPreset::ThreeKingdoms);
    assert_eq!(kiln.fill_rgb, [0xE7, 0x4C, 0x3C]);
    assert!((kiln.fill_opacity - 0.70).abs() < 1e-6);

    // TestKaSplashStrata::testProceduralStrataCurveGeneration (600x400 -> 4 curves in 150..=400 Y)
    let curves = generate_procedural_strata_curves(600.0, 400.0);
    assert_eq!(curves.len(), 4);
    for c in &curves {
        assert!((150.0..=400.0).contains(&c.control_pt1[1]));
        assert!((150.0..=400.0).contains(&c.control_pt2[1]));
    }

    // TestKaPopover::testClampingToScreenBoundary:
    // screen 1920x1080, target (1850, 500, 60, 30), popover width 200 -> clamped X = 1712
    let pos =
        calculate_clamped_popover_position([1850, 500, 60, 30], [200, 120], [0, 0, 1920, 1080]);
    assert_eq!(pos[0], 1712);
}

/// C++ `TestKaTheme::testRibbonChipWidthTenPercentReduction` (`d7afcc8`):
/// 「시추」 단추를 더한 1줄 리본이 되도록 칩 최소 너비를 40 → 36 (10% 축소) 으로 고정한다.
#[test]
fn test_ribbon_chip_width_is_36_after_ten_percent_reduction() {
    let m = ButtonMetrics::default();
    assert_eq!(m.ribbon_chip_width, 36);
    assert_eq!(m.ribbon_min_width, 36);
    assert_eq!(m.ribbon_icon_size, 32);
    assert_eq!(m.ribbon_label_padding, 8);
    assert_eq!(m.ribbon_max_label_width, 64);
}

/// C++ `KaBeginnerRibbon.cpp:31-36` `chipWidth` 규칙 1:1.
///
/// 라벨이 보이는 칩(tile 32)은 `max(ribbon_chip_width, tile + 8, min(label_px, 64) + 8)` 이다.
/// `tile + 8 = 40` 이므로 좁은 라벨에서는 36 이 아니라 40 이 하한이다 — C++ 도 같다.
#[test]
fn test_ribbon_chip_width_rule_matches_cpp() {
    let m = ButtonMetrics::default();
    // tile + 8 = 40 이 세 항 중 가장 크다.
    assert_eq!(m.ribbon_chip_width_for_label(6.0), 40);
    assert_eq!(m.ribbon_chip_width_for_label(14.0), 40);
    assert_eq!(m.ribbon_chip_width_for_label(30.0), 40);
    // 64 + 8 = 72 이 가장 크다 (`ribbon_max_label_width` 절단).
    assert_eq!(m.ribbon_chip_width_for_label(100.0), 72);
    // 라벨 폭이 커지는 경계: min(label,64)+8 > 40 이려면 label > 32.
    assert_eq!(m.ribbon_chip_width_for_label(33.0), 41);
}
