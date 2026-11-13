//! `ribbon` 시험 — 컴팩트 모드·도구 전환 러버밴드 초기화, PNU 19자리 조립·필지 여백.
//!
//! 원래 `ribbon.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 는 이 파일의 부모를 가리키므로
//! `crate::ribbon::*` 로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_canvas2d::map_tools::MapToolKind;

use crate::ribbon::*;

#[test]
fn test_ribbon_compact_mode_1024px_and_exclusive_tool_switch_clears_rubberband() {
    assert!(KaBeginnerRibbon::is_compact_mode(1024.0));
    assert!(!KaBeginnerRibbon::is_compact_mode(1100.0));
    assert!(!KaBeginnerRibbon::is_compact_mode(1440.0));

    let mut tool = MapToolKind::DrawPolygon;
    let mut rubberband = vec![[100.0, 200.0], [110.0, 210.0]];
    let hint = KaBeginnerRibbon::switch_active_tool_clearing_rubberband(
        &mut tool,
        &mut rubberband,
        MapToolKind::Measure,
    );
    assert_eq!(tool, MapToolKind::Measure);
    assert!(rubberband.is_empty());
    assert_eq!(
        hint,
        "클릭하여 측정을 시작하세요. [우클릭/더블클릭]: 측정 완료, [ESC]: 취소"
    );
}

#[test]
fn test_build_pnu_gyeongju_4713010300_76_0_yields_exact_19_digits_and_50m_margin() {
    let pnu =
        KaAppBarSearch::build_pnu_code("4713010300", false, 76, 0).expect("valid Gyeongju PNU");
    assert_eq!(pnu, "4713010300100760000");
    assert_eq!(pnu.len(), KaAppBarSearch::HGIS_PNU_TOTAL_LENGTH);

    let err = KaAppBarSearch::build_pnu_code("4713010300", false, 0, 0)
        .expect_err("main_bun 0 must fail");
    assert_eq!(err.0, PnuBuildError::InvalidBunNumber);
    assert_eq!(err.1, "본번은 1 이상의 정수여야 합니다.");

    let zoomed = KaAppBarSearch::zoom_to_parcel_extent([1000.0, 2000.0, 1050.0, 2040.0]);
    assert_eq!(zoomed, [950.0, 1950.0, 1100.0, 2090.0]);
}

/// KaRegionLocator — 시·도를 고르기 전에는 시·도·시·군·구·동·번지 모두 비어 있다(m_sido 기본값,
/// openAddressPopup KaRegionLocator.cpp:205-211 가 칸을 비운다). 경주시 같은 기본값은 없다.
#[test]
fn region_search_starts_empty() {
    let bar = KaAppBarSearch::default();
    assert_eq!(
        (bar.selected_sido.as_str(), bar.city_input.as_str(), bar.dong_input.as_str(), bar.lot_input.as_str()),
        ("", "", "", "")
    );
}
