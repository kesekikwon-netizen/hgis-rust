//! The cases of `target/cpp-ref/tests/test_topographic_source_crs.cpp` that need no frame: the belt is
//! found from where the layers lie (choosesNorthingAndZone :172-193, rejectsMissingLegacyAndWrongSheet
//! :194-205, preservesMetadataAndFlagsUnverified :206-219, rejectsMissingAndNonFiniteGeometry :220-226,
//! edgeMarginAndEveryPrimaryLayerAreChecked :227-242, officialGosanBoundsOverrideNominalGrid :243-257,
//! officialCenterSelectsZoneAcrossNominalSeam :258-271, explicitInvalidOrUnrelatedOfficialBoundsNeverFallBack
//! :272-281).

use strata_engine::Transformer;
use strata_net::ngii_topographic::TopographicSheets;

use super::*;

/// A rectangle scaled about its centre (`QgsRectangle::scale`).
pub(super) fn scaled(rect: [f64; 4], factor: f64) -> [f64; 4] {
    let (cx, cy) = ((rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0);
    let (half_w, half_h) = ((rect[2] - rect[0]) / 2.0 * factor, (rect[3] - rect[1]) / 2.0 * factor);
    [cx - half_w, cy - half_h, cx + half_w, cy + half_h]
}

pub(super) fn carry(from: u32, to: u32) -> Transformer {
    Transformer::new(from, to).expect("transformer")
}

/// `fixture` (:16-30): a contour layer that fills the middle 80 % of the sheet, drawn in `epsg`; or one
/// point at the sheet's centre.
fn fixture(sheet_no: &str, epsg: u32, point: bool) -> LayerBox {
    let bounds = scaled(TopographicSheets::resolve(sheet_no).expect("sheet").extent(), 0.8);
    let to_source = carry(4737, epsg);
    let extent = if point {
        let c = to_source.transform([(bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0]).expect("centre");
        [c[0], c[1], c[0], c[1]]
    } else {
        to_source.transform_box(bounds).expect("box")
    };
    LayerBox { category: Category::Contour, extent: Some(extent) }
}

#[test]
fn the_belt_is_told_by_northing_and_zone() {
    for (sheet, epsg) in [("367154", 5186), ("367154", 5181), ("378044", 5187), ("378044", 5183), ("336064", 5182), ("336064", 5186), ("377164", 5186), ("378133", 5187)] {
        for point in [false, true] {
            let resolved = resolve(sheet, " grs80 ", &[fixture(sheet, epsg, point)], &[], None).unwrap_or_else(|e| panic!("{sheet} {epsg}: {e}"));
            assert_eq!(resolved.epsg, epsg, "{sheet}");
            assert!(!resolved.evidence.is_empty() && resolved.unverified.is_empty() && resolved.warning.is_none());
        }
    }
}

#[test]
fn a_sheet_without_grs80_a_shifted_one_and_a_bad_number_are_refused() {
    let sheet = "367154";
    let record = fixture(sheet, 5186, false);
    for projection in ["", "BESSEL", "Tokyo", "GRS80 EPSG:5186"] {
        assert_eq!(resolve(sheet, projection, std::slice::from_ref(&record), &[], None), Err(NOT_GRS80.to_owned()), "{projection:?}");
    }
    let e = record.extent.expect("extent");
    let shifted = LayerBox { extent: Some([e[0] + 30_000.0, e[1], e[2] + 30_000.0, e[3]]), ..record.clone() };
    assert_eq!(resolve(sheet, "GRS80", &[shifted], &[], None), Err(NO_BELT_FITS.to_owned()));
    assert_eq!(resolve("bad", "GRS80", &[record], &[], None), Err(BAD_SHEET_NUMBER.to_owned()));
}

/// A layer that is not one of the placing kinds and lies elsewhere is flagged, not fatal; a placing kind
/// that lies elsewhere refuses the sheet.
#[test]
fn a_layer_outside_the_sheet_is_flagged_or_refuses_the_sheet_by_its_kind() {
    let sheet = "367154";
    let good = fixture(sheet, 5186, false);
    let stray = LayerBox { category: Category::Unknown, extent: Some([0.0, 0.0, 1.0, 1.0]) };
    let resolved = resolve(sheet, "GRS80", &[good.clone(), stray.clone()], &[], None).expect("resolved");
    assert_eq!((resolved.epsg, resolved.unverified), (5186, vec![1]));
    let building = LayerBox { category: Category::Building, ..stray };
    assert_eq!(resolve(sheet, "GRS80", &[good, building], &[], None), Err(NO_BELT_FITS.to_owned()));
}

#[test]
fn a_sheet_without_a_placing_layer_or_with_a_box_that_is_no_box_is_refused() {
    let nothing = LayerBox { category: Category::Unknown, extent: None };
    assert_eq!(resolve("367154", "GRS80", &[nothing], &[], None), Err(NO_PRIMARY_LAYER.to_owned()));
    assert_eq!(resolve("367154", "GRS80", &[], &[], None), Err(NO_PRIMARY_LAYER.to_owned()));
    let endless = LayerBox { category: Category::Contour, extent: Some([0.0, 0.0, f64::INFINITY, 5.0]) };
    assert_eq!(resolve("367154", "GRS80", &[endless], &[], None), Err(BAD_LAYER_EXTENT.to_owned()));
}

/// 100 m beyond the sheet's edge are allowed, 101 m are not; every placing layer is looked at; the
/// coordinates of another zone do not fit.
#[test]
fn the_margin_is_100_metres_and_every_placing_layer_counts() {
    let sheet = "367154";
    let projected = carry(4737, 5186).transform_box(TopographicSheets::resolve(sheet).expect("sheet").extent()).expect("box");
    let y = (projected[1] + projected[3]) / 2.0;
    let at = |beyond: f64| LayerBox { category: Category::Contour, extent: Some([projected[2] + beyond, y, projected[2] + beyond, y]) };
    assert_eq!(resolve(sheet, "GRS80", &[at(99.0)], &[], None).expect("inside").epsg, 5186);
    assert_eq!(resolve(sheet, "GRS80", &[fixture(sheet, 5186, false), at(101.0)], &[], None), Err(NO_BELT_FITS.to_owned()));
    assert_eq!(resolve(sheet, "GRS80", &[fixture(sheet, 5187, false)], &[], None), Err(NO_BELT_FITS.to_owned()));
}

/// The official rectangle of sheet 336101 (고산, 2024) from the portal's index, EPSG:5179: it lies well to
/// the south-west of the sheet-number grid, so the grid alone refuses the sheet and the index places it.
const GOSAN_5179: [f64; 4] = [871_973.078, 1_478_540.778_7, 883_778.940_8, 1_492_547.054_2];

#[test]
fn the_official_rectangle_goes_before_the_sheet_number_grid() {
    for epsg in [5182, 5186] {
        let record = LayerBox { category: Category::Contour, extent: Some(scaled(carry(5179, epsg).transform_box(GOSAN_5179).expect("box"), 0.85)) };
        assert_eq!(resolve("336101", "GRS80", std::slice::from_ref(&record), &[], None), Err(NO_BELT_FITS.to_owned()));
        let resolved = resolve("336101", "GRS80", &[record], &[], Some(GOSAN_5179)).unwrap_or_else(|e| panic!("{epsg}: {e}"));
        assert_eq!(resolved.epsg, epsg);
        assert!(resolved.evidence.contains("실제 공식 도곽"), "{}", resolved.evidence);
    }
}

/// An official rectangle just west of 128 E for a sheet whose number lies east of it: the zone is that of
/// the official centre.
#[test]
fn the_official_centre_decides_the_zone_across_the_seam() {
    let official = carry(4737, 5179).transform_box([127.975, 37.03, 127.999, 37.08]).expect("official");
    let record = LayerBox { category: Category::Contour, extent: Some(scaled(carry(5179, 5186).transform_box(official).expect("box"), 0.8)) };
    assert_eq!(resolve("378133", "GRS80", &[record], &[], Some(official)).expect("resolved").epsg, 5186);
}

/// An official rectangle that is given and wrong is never set aside for the grid.
#[test]
fn a_wrong_official_rectangle_never_falls_back_to_the_grid() {
    let record = fixture("367154", 5186, false);
    assert_eq!(resolve("367154", "GRS80", std::slice::from_ref(&record), &[], None).expect("by the grid").epsg, 5186);
    // No rectangle at all (QGIS's null rectangle), one of a metre, one of 100 km: not a sheet.
    for bounds in [[f64::MAX, f64::MAX, -f64::MAX, -f64::MAX], [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 100_000.0, 100_000.0]] {
        assert_eq!(resolve("367154", "GRS80", std::slice::from_ref(&record), &[], Some(bounds)), Err(BAD_OFFICIAL_BOUNDS.to_owned()), "{bounds:?}");
    }
    // The rectangle of another sheet, 300 km away.
    assert_eq!(resolve("367154", "GRS80", &[record], &[], Some(GOSAN_5179)), Err(OFFICIAL_TOO_FAR.to_owned()));
}
