//! `tabs_paleo_3d` 회귀 시험 — `tabs_paleo_3d` 에서 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::gui_tab_paleo::gui_tab_paleo_map::paleo_preview_rgba;
use crate::tabs_paleo_3d::PaleoTerrain3dTabState;
use crate::tabs_paleo_3d_views::PaleoSectionError;
use strata_paleo::pipeline::PaleoClass;

#[test]
fn test_paleo_studio_cutline_under_5m_rejected_and_1000m_clamped_to_500_samples() {
    let short_line = [[100.0, 100.0], [101.5, 101.3228756]];
    let err = PaleoTerrain3dTabState::generate_cross_section_samples(&short_line, 100.0, 500)
        .expect_err("2m line must fail");
    assert_eq!(err.0, PaleoSectionError::CutLineTooShort);
    assert!(err.1.contains("최소 5.0m"));

    let long_line = [[0.0, 0.0], [1000.0, 0.0]];
    let (samples, corridor) =
        PaleoTerrain3dTabState::generate_cross_section_samples(&long_line, 800.0, 500)
            .expect("1000m line must succeed");
    assert_eq!(samples.len(), 500);
    assert!((corridor - 500.0).abs() < 1e-6);
}

/// Imhof 를 끄면 음영이 빠지고, 켜면 음영 버퍼가 그대로 나온다.
/// 두 결과가 **실제로 달라야** 체크박스가 화면에서 의미를 가진다.
#[test]
fn test_imhof_toggle_changes_preview_pixels() {
    let grid = [PaleoClass::NaturalLevee, PaleoClass::SteepMountain];
    let mut shaded = Vec::new();
    for &c in &grid {
        let [r, g, b, a] = c.color_rgba();
        // 음영 0.75 를 곱한 값 (파이프라인과 같은 형태)
        shaded.extend_from_slice(&[
            ((r as f64) * 0.75).round() as u8,
            ((g as f64) * 0.75).round() as u8,
            ((b as f64) * 0.75).round() as u8,
            a,
        ]);
    }

    let with_imhof = paleo_preview_rgba(&shaded, &grid, 2, 1, true);
    let without = paleo_preview_rgba(&shaded, &grid, 2, 1, false);

    assert_eq!(with_imhof, shaded, "Imhof 를 켜면 음영 버퍼 그대로여야 한다");
    assert_eq!(with_imhof.len(), 8); // 2x1 픽셀 x RGBA
    assert_eq!(without.len(), 8);
    assert_ne!(
        with_imhof, without,
        "체크박스를 끄면 음영이 빠진 분류색이어야 한다 — 같으면 토글이 무의미하다"
    );
    // 평면 색은 분류색 원본과 같아야 한다.
    let [r0, g0, b0, a0] = grid[0].color_rgba();
    assert_eq!(&without[0..4], &[r0, g0, b0, a0]);
}

/// 음영 버퍼가 짧으면(파이프라인 미실행) 평면 색으로 안전하게 떨어지고,
/// 픽셀 수만큼 길이를 채운다 — 모자란 채로 넘기면 egui 가 패닉한다.
#[test]
fn test_imhof_falls_back_when_shaded_buffer_is_short() {
    let grid = [PaleoClass::NaturalLevee];
    let short: Vec<u8> = vec![1, 2, 3, 4]; // 2x1 픽셀에는 모자람
    let out = paleo_preview_rgba(&short, &grid, 2, 1, true);
    assert_eq!(out.len(), 8);
    let [r, g, b, a] = grid[0].color_rgba();
    assert_eq!(&out[0..4], &[r, g, b, a]);
    // 뒤쪽 픽셀도 같은 분류색으로 채워져 있어야 한다.
    assert_eq!(&out[4..8], &[r, g, b, a]);
    let _ = (r, g, b, a);
}
