//! Colour picking on small pictures painted for the purpose (`GeologyMapService.cpp:142-162`).

use image::{Rgba, RgbaImage};

use super::*;

const UNIT: [u8; 4] = [250, 244, 180, 255];
const LINE: [u8; 4] = [40, 40, 40, 255];

fn picture(fill: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(40, 40, Rgba(fill))
}

#[test]
fn a_plain_area_gives_its_colour() {
    assert_eq!(pick_pixel_color(&picture(UNIT), 20, 20), Some([250, 244, 180]));
    // At the corner only the pixels inside the picture are looked at: (0,0), (3,0), (0,3), (6,0), (0,6), (3,3).
    assert_eq!(pick_pixel_color(&picture(UNIT), 0, 0), Some([250, 244, 180]));
    assert_eq!(hex([250, 244, 180]), "#FAF4B4");
    assert_eq!(hex([0, 9, 255]), "#0009FF");
}

/// A dark letter or boundary line on the point does not decide: the pixels around it do.
#[test]
fn dark_pixels_are_passed_over() {
    let mut on_a_line = picture(UNIT);
    for x in 0..40 {
        on_a_line.put_pixel(x, 20, Rgba(LINE));
    }
    assert_eq!(pick_pixel_color(&on_a_line, 20, 20), Some([250, 244, 180]));
    // Lightness 79 is dark, 80 is not (Qt: mean of the largest and smallest channel, halves up).
    assert_eq!(pick_pixel_color(&picture([79, 79, 79, 255]), 20, 20), None);
    assert_eq!(pick_pixel_color(&picture([80, 80, 80, 255]), 20, 20), Some([80, 80, 80]));
    assert_eq!(pick_pixel_color(&picture([159, 0, 0, 255]), 20, 20), Some([159, 0, 0]), "159 and 0: 79.5 counts as 80");
    assert_eq!(pick_pixel_color(&picture([158, 0, 0, 255]), 20, 20), None);
}

/// Where the map is see-through there is no map: alpha under 200 does not count.
#[test]
fn see_through_pixels_are_passed_over() {
    assert_eq!(pick_pixel_color(&picture([250, 244, 180, 199]), 20, 20), None);
    assert_eq!(pick_pixel_color(&picture([250, 244, 180, 200]), 20, 20), Some([250, 244, 180]));
    assert_eq!(pick_pixel_color(&picture(UNIT), 500, 20), None, "a point outside the picture");
    assert_eq!(pick_pixel_color(&picture(UNIT), -7, 20), None);
    // Six pixels left of the picture: the one pixel six to the right is inside.
    assert_eq!(pick_pixel_color(&picture(UNIT), -6, 20), Some([250, 244, 180]));
}

/// Of the pixels that count, the one of middle lightness: here 13 pixels, 4 of a lighter neighbour unit and
/// 9 of the unit itself, so the 7th by lightness is the unit's colour.
#[test]
fn the_colour_of_middle_lightness_is_taken() {
    let mut two_units = picture(UNIT);
    // The neighbour unit, lighter (lightness 240 against 215), fills everything right of x = 21.
    for y in 0..40 {
        for x in 22..40 {
            two_units.put_pixel(x, y, Rgba([255, 255, 225, 255]));
        }
    }
    // Looked at around (20, 20): x = 14, 17, 20 lie in the unit (9 pixels), x = 23, 26 in the neighbour (4).
    assert_eq!(pick_pixel_color(&two_units, 20, 20), Some([250, 244, 180]));
    // Around (22, 20) the neighbour has the majority: x = 16, 19 in the unit (4), x = 22, 25, 28 in it (9).
    assert_eq!(pick_pixel_color(&two_units, 22, 20), Some([255, 255, 225]));
}
