//! `tabs_paleo_3d` 회귀 시험 — `tabs_paleo_3d` 에서 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::tabs_paleo_3d::PaleoTerrain3dTabState;
use crate::tabs_paleo_3d_views::PaleoSectionError;

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
