//! The official colour of a rock unit, read from the picture of the official map
//! (C++ `pickPixelColor`, `GeologyMapService.cpp:142-162`).

use image::RgbaImage;

use crate::geology_era::lightness;

#[cfg(test)]
#[path = "geology_colour_tests.rs"]
mod tests;

/// The pixels looked at around the point: the point itself, then 3 and 6 pixels to each side, then the four
/// diagonal neighbours at 3 (`kOffsets`, `:144-146`).
const OFFSETS: [(i64, i64); 13] = [(0, 0), (-3, 0), (3, 0), (0, -3), (0, 3), (-6, 0), (6, 0), (0, -6), (0, 6), (-3, -3), (3, 3), (-3, 3), (3, -3)];

/// The colour of the map around pixel (`px`, `py`): of the pixels above that are inside the picture, solid
/// (alpha at least 200) and not dark (lightness at least 80: letters and boundary lines are dark), the one
/// of middle lightness. `None` when no pixel qualifies.
#[must_use]
pub fn pick_pixel_color(picture: &RgbaImage, px: i64, py: i64) -> Option<[u8; 3]> {
    let mut picks: Vec<[u8; 3]> = Vec::new();
    for (dx, dy) in OFFSETS {
        let (Ok(x), Ok(y)) = (u32::try_from(px + dx), u32::try_from(py + dy)) else {
            continue;
        };
        if x >= picture.width() || y >= picture.height() {
            continue;
        }
        let [r, g, b, a] = picture.get_pixel(x, y).0;
        if a >= 200 && lightness([r, g, b]) >= 80 {
            picks.push([r, g, b]);
        }
    }
    // The C++ sorts with `std::sort`, which may order equally light colours either way; here they keep the
    // order they were looked at.
    picks.sort_by_key(|rgb| lightness(*rgb));
    picks.get(picks.len() / 2).copied()
}

/// `#RRGGBB`, the form the stored column `ka_color` holds.
#[must_use]
pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
