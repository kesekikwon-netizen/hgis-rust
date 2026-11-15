//! The frame cases of `target/cpp-ref/tests/test_topographic_source_crs.cpp`: a sheet whose data lie
//! outside the official index rectangle is placed by its own frame, when that frame is a square of 7.5
//! minutes in exactly one belt (shiftedCoastalFrameIsIndependentCrsEvidence :35-87,
//! jejuFrameSeveralKilometresFromTheIndexSelects5186 :88-171). The C++ writes DXF files for these; here
//! the solver gets the frame's points and the layers' boxes directly, as `sheet.rs` hands them on.

use super::tests::carry;
use super::*;

/// The frame of a sheet drawn in EPSG:5186: five points on each side of the square with these corners
/// (longitude, latitude; the fifth corner is the first), closed.
fn frame_5186(corners: [[f64; 2]; 5]) -> Vec<[f64; 2]> {
    let to_source = carry(4737, 5186);
    let mut points = Vec::new();
    for side in 0..4 {
        for i in 0..5 {
            let t = f64::from(i) / 5.0;
            let (a, b) = (corners[side], corners[side + 1]);
            points.push(to_source.transform([a[0] * (1.0 - t) + b[0] * t, a[1] * (1.0 - t) + b[1] * t]).expect("point"));
        }
    }
    points.push(points[0]);
    points
}

fn box_of(points: &[[f64; 2]]) -> [f64; 4] {
    points.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
}

/// The layers of such a sheet: the frame (a facility, not a placing kind) and one contour.
fn layers(frame: &[Vec<[f64; 2]>], contour: &[[f64; 2]]) -> Vec<LayerBox> {
    let mut layers: Vec<LayerBox> = frame.iter().take(1).map(|line| LayerBox { category: Category::Facility, extent: Some(box_of(line)) }).collect();
    layers.push(LayerBox { category: Category::Contour, extent: Some(box_of(contour)) });
    layers
}

/// The index rectangle of sheet 336064 (Jeju), EPSG:5179.
fn official() -> [f64; 4] {
    carry(4737, 5179).transform_box([126.375, 33.375, 126.5, 33.5]).expect("official")
}

/// A frame 0.025 degrees north of the index: the contour reaches past the index rectangle, so the index
/// places nothing, and the frame does, when it is whole.
#[test]
fn a_frame_beside_the_index_places_the_sheet_when_it_is_whole() {
    let to_source = carry(4737, 5186);
    let corners = [[126.375, 33.4], [126.5, 33.4], [126.5, 33.525], [126.375, 33.525], [126.375, 33.4]];
    for (condition, accepted) in [("shifted-frame", true), ("extra-edge-vertices", true), ("damaged-edge", false), ("offset-frame", true), ("outside-data", false), ("duplicate-frame", false), ("unclosed-frame", false), ("no-frame", false)] {
        let mut points = frame_5186(corners);
        if condition == "extra-edge-vertices" {
            // Five more points on the south side.
            let mut with_extra = Vec::new();
            for (i, point) in points.iter().enumerate() {
                with_extra.push(*point);
                if i < 5 {
                    with_extra.push(to_source.transform([126.375 + 0.025 * i as f64 + 0.0083, 33.4]).expect("point"));
                }
            }
            points = with_extra;
        }
        if condition == "damaged-edge" {
            points[7][0] += 5.0;
        }
        if condition == "offset-frame" {
            points.iter_mut().for_each(|p| p[1] += 365.0);
        }
        if condition == "unclosed-frame" {
            points.pop();
        }
        let frame: Vec<Vec<[f64; 2]>> = match condition {
            "no-frame" => Vec::new(),
            "duplicate-frame" => vec![points.clone(), points.clone()],
            _ => vec![points.clone()],
        };
        let upper = to_source.transform([126.48, if condition == "outside-data" { 33.55 } else { 33.52 }]).expect("point");
        let contour = [to_source.transform([126.4, 33.49]).expect("point"), upper];
        let result = resolve("336064", "GRS80", &layers(&frame, &contour), &frame, Some(official()));
        if !accepted {
            assert_eq!(result, Err(NO_BELT_FITS.to_owned()), "{condition}");
            continue;
        }
        let resolved = result.unwrap_or_else(|e| panic!("{condition}: {e}"));
        assert_eq!(resolved.epsg, 5186, "{condition}");
        assert!(resolved.evidence.contains("원본 도곽") && resolved.evidence.contains("색인"), "{condition}: {}", resolved.evidence);
        assert!(resolved.unverified.is_empty(), "{condition}");
    }
}

/// A Jeju frame 4.6 km east of the index is taken, with a word to the user; the same frame 60 km further
/// east is not.
#[test]
fn a_jeju_frame_kilometres_from_the_index_selects_5186_and_one_60_km_away_does_not() {
    let to_source = carry(4737, 5186);
    let points = frame_5186([[126.425, 33.375], [126.55, 33.375], [126.55, 33.5], [126.425, 33.5], [126.425, 33.375]]);
    let contour = [to_source.transform([126.51, 33.42]).expect("point"), to_source.transform([126.53, 33.46]).expect("point")];
    let frame = vec![points.clone()];
    let resolved = resolve("336064", "GRS80", &layers(&frame, &contour), &frame, Some(official())).expect("resolved");
    assert_eq!(resolved.epsg, 5186);
    assert!(resolved.warning.as_deref().is_some_and(|w| w.contains("336064")), "{:?}", resolved.warning);
    assert!(resolved.unverified.is_empty());

    let far: Vec<[f64; 2]> = points.iter().map(|p| [p[0] + 60_000.0, p[1]]).collect();
    let far_frame = vec![far.clone()];
    assert_eq!(resolve("336064", "GRS80", &layers(&far_frame, &[far[1], far[6]]), &far_frame, Some(official())), Err(NO_BELT_FITS.to_owned()));
}

/// Without an official rectangle the frame is not asked: the sheet-number grid alone decides.
#[test]
fn the_frame_is_asked_only_when_the_official_rectangle_is_known() {
    let to_source = carry(4737, 5186);
    let points = frame_5186([[126.375, 33.4], [126.5, 33.4], [126.5, 33.525], [126.375, 33.525], [126.375, 33.4]]);
    let contour = [to_source.transform([126.4, 33.49]).expect("point"), to_source.transform([126.48, 33.52]).expect("point")];
    let frame = vec![points];
    assert_eq!(resolve("336064", "GRS80", &layers(&frame, &contour), &frame, None), Err(NO_BELT_FITS.to_owned()));
}
