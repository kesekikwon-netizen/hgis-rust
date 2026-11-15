//! The elevation layers: the colour of a height as the C++ ramp gives it, and cells laid into a picture.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use strata_engine::ElevationGrid;

use super::{cells_of, height_colour, load, render, Drawn};
use crate::ref_vector::data::{HeightRamp, LoadSpec, Look};
use crate::ref_vector::raster::RasterView;

const NATIONAL: Look = Look::Heights { ramp: HeightRamp::National };

/// The expected colours are what QGIS's own `QgsColorRampShader` (linear, not clipped; QGIS 4.3.0-Master of
/// this PC's OSGeo4W, asked on 2026-11-15) returns for the sixteen stops of `DemPresentation.cpp:35-39`
/// under each of the three presets of 「DEM 표현」 (`:68-75`).
#[test]
fn a_height_takes_the_colour_qgis_gives_it() {
    let national: [(f32, [u8; 3]); 13] = [
        (-20.0, [114, 198, 181]),
        (0.0, [114, 198, 181]),
        (5.0, [121, 202, 176]),
        (7.5, [125, 204, 173]),
        (10.0, [129, 206, 170]),
        (62.5, [175, 215, 123]),
        (100.0, [208, 220, 120]),
        (187.55, [232, 212, 123]),
        (500.0, [225, 153, 96]),
        (816.198, [195, 115, 82]),
        (1450.0, [195, 174, 160]),
        (2000.0, [244, 243, 239]),
        (2500.0, [244, 243, 239]),
    ];
    for (height, colour) in national {
        assert_eq!(height_colour(height, HeightRamp::National), colour, "전국 표준, {height} m");
    }
    // 「저지대 강조」: a stop of z metres stands at z / 2 up to 200 m, and at 100 + (z - 200) * 1900 / 1800 above.
    let lowland: [(f32, [u8; 3]); 12] = [
        (-5.0, [114, 198, 181]),
        (0.0, [114, 198, 181]),
        (3.0, [123, 202, 175]),
        (12.5, [145, 211, 150]),
        (20.0, [157, 212, 137]),
        (60.0, [214, 220, 121]),
        (100.0, [235, 210, 122]),
        (150.0, [235, 201, 118]),
        (400.0, [226, 155, 97]),
        (1000.0, [174, 110, 89]),
        (2000.0, [244, 243, 239]),
        (2100.0, [244, 243, 239]),
    ];
    for (height, colour) in lowland {
        assert_eq!(height_colour(height, HeightRamp::Lowland), colour, "저지대 강조, {height} m");
    }
    // 「현재 화면 맞춤」 for a view whose heights reach from 31 m to 152 m: the stops are spread evenly.
    let fitted: [(f32, [u8; 3]); 8] = [
        (10.0, [114, 198, 181]),
        (31.0, [114, 198, 181]),
        (35.0, [121, 201, 176]),
        (60.0, [177, 215, 122]),
        (91.5, [236, 200, 117]),
        (120.0, [196, 116, 82]),
        (152.0, [244, 243, 239]),
        (300.0, [244, 243, 239]),
    ];
    for (height, colour) in fitted {
        assert_eq!(height_colour(height, HeightRamp::Range { low: 31.0, high: 152.0 }), colour, "현재 화면 맞춤 31..152, {height} m");
    }
}

/// 3 x 2 cells of 90 m; the last one holds no height.
fn grid() -> ElevationGrid {
    ElevationGrid { left: 1000.0, top: 2000.0, cell: 90.0, width: 3, height: 2, heights: vec![0.0, 100.0, 2000.0, 5.0, 500.0, f32::NAN], shade: vec![255, 181, 0, 100, 181, 0] }
}

/// 「DEM」: every height is an opaque colour, a cell without height is clear. 「지형 음영」: black whose
/// cover is the light that is missing (drawn at 30 % it multiplies what lies under it by the light, as the
/// C++ relief layer does, `BasemapOps.cpp:2083-2086`), and nothing where there is no height.
#[test]
fn heights_become_colours_and_light_becomes_shadow() {
    let colours = cells_of(&grid(), &NATIONAL);
    assert_eq!((colours.width, colours.height, colours.left, colours.top, colours.cell), (3, 2, 1000.0, 2000.0, 90.0));
    assert_eq!(colours.rgba, [[114, 198, 181, 255], [208, 220, 120, 255], [244, 243, 239, 255], [121, 202, 176, 255], [225, 153, 96, 255], [0, 0, 0, 0]]);
    let shadow = cells_of(&grid(), &Look::Relief { z_factor: 1.0 });
    assert_eq!(shadow.rgba, [[0, 0, 0, 0], [0, 0, 0, 74], [0, 0, 0, 255], [0, 0, 0, 155], [0, 0, 0, 74], [0, 0, 0, 0]]);
    // Another preset gives the same cells other colours.
    let lowland = cells_of(&grid(), &Look::Heights { ramp: HeightRamp::Lowland });
    assert_eq!(lowland.rgba[1], [235, 210, 122, 255], "100 m");
}

/// A picture pixel on the centre of a cell takes the cell's colour, one half-way between two centres the
/// mean of both (bilinear, as the C++ sets for the DEM, `DemPresentation.cpp:97-100`), one within half a
/// cell of the outermost centres the outermost cell, and one beside the grid nothing.
#[test]
fn cells_are_laid_into_the_picture_where_they_lie() {
    let drawn = Drawn::Heights(Arc::new(grid()));
    assert_eq!((drawn.rows(), drawn.extent()), (None, Some([1000.0, 1820.0, 1270.0, 2000.0])));
    // 45 m pixels whose centres fall on x = 910, 955 … 1315 and y = 2000, 1955 … 1775; the centres of the
    // cells are at x = 1045, 1135, 1225 and y = 1955, 1865.
    let view = RasterView { left: 887.5, top: 2022.5, metres_per_pixel: 45.0, width: 10, height: 6, pixels_per_mm: 4.0 };
    let picture = render(&drawn, &NATIONAL, view).unwrap();
    assert_eq!((picture.view, picture.rgba.len()), (view, 10 * 6 * 4));
    let at = |x: usize, y: usize| <[u8; 4]>::try_from(&picture.rgba[(y * 10 + x) * 4..(y * 10 + x) * 4 + 4]).unwrap();
    assert_eq!((at(3, 1), at(5, 1), at(7, 1)), ([114, 198, 181, 255], [208, 220, 120, 255], [244, 243, 239, 255]));
    assert_eq!((at(4, 1), at(3, 2)), ([161, 209, 151, 255], [118, 200, 179, 255]), "half-way across and half-way down");
    assert_eq!((at(2, 0), at(8, 1), at(3, 4)), ([114, 198, 181, 255], [244, 243, 239, 255], [121, 202, 176, 255]), "the edges of the grid");
    // The cell without height is clear, and fades in from its neighbour (the picture is premultiplied).
    assert_eq!((at(7, 3), at(6, 3)), ([0, 0, 0, 0], [113, 77, 48, 128]));
    assert_eq!((at(0, 1), at(1, 1), at(9, 1), at(3, 5)), ([0; 4], [0; 4], [0; 4], [0; 4]), "beside the grid");
    // The same heights in the look of the relief: shadow where the light is missing.
    let shadow = render(&drawn, &Look::Relief { z_factor: 1.0 }, view).unwrap();
    assert_eq!(<[u8; 4]>::try_from(&shadow.rgba[(10 + 5) * 4..(10 + 5) * 4 + 4]).unwrap(), [0, 0, 0, 74]);

    let no_scale = RasterView { metres_per_pixel: f64::NAN, ..view };
    assert!(render(&drawn, &NATIONAL, no_scale).is_none());
}

/// 「현재 화면 맞춤」 asks for the lowest and highest height of the view (`DemPresentation.cpp:53-64`): the
/// cells whose centres lie in the box count, cells without height do not, and a box that holds no height
/// has no answer.
#[test]
fn the_heights_of_a_view_have_a_lowest_and_a_highest() {
    let drawn = Drawn::Heights(Arc::new(grid()));
    assert_eq!(drawn.height_range([900.0, 1700.0, 1400.0, 2100.0]), Some((0.0, 2000.0)), "the whole grid");
    // The two western columns: centres at x = 1045 and 1135.
    assert_eq!(drawn.height_range([1000.0, 1820.0, 1180.0, 2000.0]), Some((0.0, 500.0)));
    // The southern row only (centres at y = 1865), from the middle column on: 500 m and a cell without height.
    assert_eq!(drawn.height_range([1100.0, 1830.0, 1300.0, 1900.0]), Some((500.0, 500.0)));
    // Only the cell without height; a box between the centres; a box beside the grid.
    assert_eq!(drawn.height_range([1190.0, 1830.0, 1300.0, 1900.0]), None);
    assert_eq!(drawn.height_range([1050.0, 1960.0, 1130.0, 1999.0]), None);
    assert_eq!(drawn.height_range([5000.0, 5000.0, 6000.0, 6000.0]), None);
}

/// A DEM folder without a received file is no layer.
#[test]
fn a_dem_folder_without_files_cannot_be_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let spec = LoadSpec { file: dir.path().join("DEM"), table: String::new(), work_epsg: 5187, look: NATIONAL, label: None };
    assert!(load(&spec).is_err());
}
