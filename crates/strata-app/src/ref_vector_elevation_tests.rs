//! The elevation layers: the colour of a height as the C++ ramp gives it, and cells laid into a picture.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_engine::ElevationGrid;

use super::{cells_of, height_colour, load, render, Drawn};
use crate::ref_vector::data::{LoadSpec, Look};
use crate::ref_vector::raster::RasterView;

/// The expected colours are what QGIS's own `QgsColorRampShader` (linear, not clipped; QGIS 4.3.0-Master of
/// this PC's OSGeo4W, asked on 2026-11-15) returns for the sixteen stops of `DemPresentation.cpp:35-39`.
#[test]
fn a_height_takes_the_colour_qgis_gives_it() {
    let expected: [(f32, [u8; 3]); 13] = [
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
    for (height, colour) in expected {
        assert_eq!(height_colour(height), colour, "{height} m");
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
    let colours = cells_of(&grid(), &Look::Heights);
    assert_eq!((colours.width, colours.height, colours.left, colours.top, colours.cell), (3, 2, 1000.0, 2000.0, 90.0));
    assert_eq!(colours.rgba, [[114, 198, 181, 255], [208, 220, 120, 255], [244, 243, 239, 255], [121, 202, 176, 255], [225, 153, 96, 255], [0, 0, 0, 0]]);
    let shadow = cells_of(&grid(), &Look::Relief);
    assert_eq!(shadow.rgba, [[0, 0, 0, 0], [0, 0, 0, 74], [0, 0, 0, 255], [0, 0, 0, 155], [0, 0, 0, 74], [0, 0, 0, 0]]);
}

/// A picture pixel on the centre of a cell takes the cell's colour, one half-way between two centres the
/// mean of both (bilinear, as the C++ sets for the DEM, `DemPresentation.cpp:97-100`), one within half a
/// cell of the outermost centres the outermost cell, and one beside the grid nothing.
#[test]
fn cells_are_laid_into_the_picture_where_they_lie() {
    let drawn = Drawn::Cells(std::sync::Arc::new(cells_of(&grid(), &Look::Heights)));
    assert_eq!((drawn.rows(), drawn.extent()), (None, Some([1000.0, 1820.0, 1270.0, 2000.0])));
    // 45 m pixels whose centres fall on x = 910, 955 … 1315 and y = 2000, 1955 … 1775; the centres of the
    // cells are at x = 1045, 1135, 1225 and y = 1955, 1865.
    let view = RasterView { left: 887.5, top: 2022.5, metres_per_pixel: 45.0, width: 10, height: 6, pixels_per_mm: 4.0 };
    let picture = render(&drawn, view).unwrap();
    assert_eq!((picture.view, picture.rgba.len()), (view, 10 * 6 * 4));
    let at = |x: usize, y: usize| <[u8; 4]>::try_from(&picture.rgba[(y * 10 + x) * 4..(y * 10 + x) * 4 + 4]).unwrap();
    assert_eq!((at(3, 1), at(5, 1), at(7, 1)), ([114, 198, 181, 255], [208, 220, 120, 255], [244, 243, 239, 255]));
    assert_eq!((at(4, 1), at(3, 2)), ([161, 209, 151, 255], [118, 200, 179, 255]), "half-way across and half-way down");
    assert_eq!((at(2, 0), at(8, 1), at(3, 4)), ([114, 198, 181, 255], [244, 243, 239, 255], [121, 202, 176, 255]), "the edges of the grid");
    // The cell without height is clear, and fades in from its neighbour (the picture is premultiplied).
    assert_eq!((at(7, 3), at(6, 3)), ([0, 0, 0, 0], [113, 77, 48, 128]));
    assert_eq!((at(0, 1), at(1, 1), at(9, 1), at(3, 5)), ([0; 4], [0; 4], [0; 4], [0; 4]), "beside the grid");

    let no_scale = RasterView { metres_per_pixel: f64::NAN, ..view };
    assert!(render(&drawn, no_scale).is_none());
}

/// A DEM folder without a received file is no layer.
#[test]
fn a_dem_folder_without_files_cannot_be_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let spec = LoadSpec { file: dir.path().join("DEM"), table: String::new(), work_epsg: 5187, look: Look::Heights, label: None };
    assert!(load(&spec).is_err());
}
