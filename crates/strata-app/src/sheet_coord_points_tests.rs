//! The coordinate points of a sheet — ported from tests/test_drawing_studio.cpp:101-133
//! (coordCallout_sharedBuilderMakesFourItems) and the magnet of KaDrawingStudio.cpp:1704-1779.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{callout_shape, free_box_rect, inside_map_frame, nearest_on, tag_for, xy_text, BoxMm};

fn intersects(a: BoxMm, b: BoxMm) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The letters run A to Z and again; the text is two lines with X the northing and Y the easting, three
/// decimals (the Korean survey order).
#[test]
fn letters_and_the_text_of_a_point() {
    assert_eq!((tag_for(0), tag_for(1), tag_for(25), tag_for(27)), ('A', 'B', 'Z', 'B'));
    assert_eq!(xy_text(200_123.4, 550_987.6), "X=550987.600\nY=200123.400");
    assert_eq!(xy_text(1.0, 2.0), "X=2.000\nY=1.000");
}

/// A box is 26 x 7.2 mm, 8 mm right of its point and 4 mm above; a box laid out among others keeps 2 mm off
/// them.
#[test]
fn the_box_of_a_point_keeps_off_the_others() {
    let first = free_box_rect([100.0, 80.0], &[]);
    assert_eq!((first.x, first.y, first.w, first.h), (108.0, 68.8, 26.0, 7.2));
    let used = [first];
    let second = free_box_rect([102.0, 81.0], &used);
    let padded = BoxMm { x: second.x - 2.0, y: second.y - 2.0, w: second.w + 4.0, h: second.h + 4.0 };
    assert!(!intersects(padded, used[0]), "{second:?} against {:?}", used[0]);
}

/// The arrow runs from the near side of the box to a head 2.4 mm long at the point; the letter sits just
/// above the point.
#[test]
fn the_arrow_and_the_letter_of_a_point() {
    let tip = [100.0, 80.0];
    let shape = callout_shape(tip, BoxMm { x: 108.0, y: 68.8, w: 26.0, h: 7.2 });
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
    assert!(near(shape.shaft[0], [108.0, 72.4]), "from the left side of a box on the right: {:?}", shape.shaft[0]);
    assert_eq!(shape.head[0], tip);
    let neck = shape.shaft[1];
    assert!(((neck[0] - tip[0]).hypot(neck[1] - tip[1]) - 2.4).abs() < 1e-9);
    let half = (shape.head[1][0] - shape.head[2][0]).hypot(shape.head[1][1] - shape.head[2][1]) / 2.0;
    assert!((half - 2.4 * 0.42).abs() < 1e-9);
    assert!(near([shape.letter.x, shape.letter.y], [97.6, 74.6]) && (shape.letter.w, shape.letter.h) == (5.2, 4.2));
}

/// A point is placed only on the map frame, 0.8 mm around it counting (`layoutMapItemContains`).
#[test]
fn a_point_is_placed_on_the_map_frame_only() {
    let frame = BoxMm { x: 10.0, y: 10.0, w: 190.0, h: 250.0 };
    assert!(inside_map_frame([10.0, 100.0], frame) && inside_map_frame([9.3, 100.0], frame));
    assert!(!inside_map_frame([9.1, 100.0], frame) && !inside_map_frame([100.0, 261.0], frame));
}

/// The magnet takes the nearest place on a line or a ring, or a point, within its reach: the corner when
/// the click is beyond the end of a side, a place on the side otherwise; nothing farther than the reach.
#[test]
fn the_magnet_takes_the_nearest_place_within_reach() {
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]];
    let points = vec![[30.0, 30.0]];
    assert_eq!(nearest_on([5.0, 0.4], &square, &points, 1.0), Some([5.0, 0.0]));
    assert_eq!(nearest_on([10.5, -0.5], &square, &points, 1.0), Some([10.0, 0.0]));
    assert_eq!(nearest_on([30.3, 29.8], &square, &points, 1.0), Some([30.0, 30.0]));
    assert_eq!(nearest_on([5.0, 5.0], &square, &points, 1.0), None);
}
