//! Elevation files brought to one grid of the work CRS. The files are written here through the same GDAL
//! driver the NGII public DEM comes in (HFA, `.img`), in its CRS (EPSG:5179) and cell size (90 m).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::DriverManager;

use super::{read_elevation_grid, ElevationGrid};
use crate::Transformer;

const CELL: f64 = 90.0;
/// The north-west corner of the real sheet 35901 (경주).
const ORIGIN: [f64; 2] = [1_135_080.0, 1_779_480.0];
const SIZE: (usize, usize) = (40, 30);

#[allow(clippy::cast_precision_loss)]
fn write_dem(path: &Path, origin: [f64; 2], height: impl Fn(f64, f64) -> f32) {
    let driver = DriverManager::get_driver_by_name("HFA").expect("the HFA driver is in the build");
    let mut file = driver.create_with_band_type::<f32, _>(path, SIZE.0, SIZE.1, 1).unwrap();
    file.set_geo_transform(&[origin[0], CELL, 0.0, origin[1], 0.0, -CELL]).unwrap();
    file.set_spatial_ref(&SpatialRef::from_epsg(5179).unwrap()).unwrap();
    let mut cells = Vec::with_capacity(SIZE.0 * SIZE.1);
    for row in 0..SIZE.1 {
        for col in 0..SIZE.0 {
            cells.push(height(origin[0] + (col as f64 + 0.5) * CELL, origin[1] - (row as f64 + 0.5) * CELL));
        }
    }
    let mut band = file.rasterband(1).unwrap();
    band.set_no_data_value(Some(-9999.0)).unwrap();
    band.write((0, 0), SIZE, &mut Buffer::new(SIZE, cells)).unwrap();
}

/// The centre of every cell of the grid, back in the CRS of the files, with its height and light.
#[allow(clippy::cast_precision_loss)]
fn cells_in_5179(grid: &ElevationGrid) -> Vec<([f64; 2], f32, u8)> {
    let back = Transformer::new(5187, 5179).unwrap();
    let mut cells = Vec::new();
    for row in 0..grid.height {
        for col in 0..grid.width {
            let centre = [grid.left + (col as f64 + 0.5) * grid.cell, grid.top - (row as f64 + 0.5) * grid.cell];
            let at = row * grid.width + col;
            cells.push((back.transform(centre).unwrap(), grid.heights[at], grid.shade[at]));
        }
    }
    cells
}

/// How many cells a point lies inside the box of a file written at `origin` (negative: outside).
#[allow(clippy::cast_precision_loss)]
fn cells_inside(origin: [f64; 2], p: [f64; 2]) -> f64 {
    let (east, south) = (origin[0] + SIZE.0 as f64 * CELL, origin[1] - SIZE.1 as f64 * CELL);
    (p[0] - origin[0]).min(east - p[0]).min(origin[1] - p[1]).min(p[1] - south) / CELL
}

/// A plane keeps its height under bilinear resampling, so every cell well inside the file must hold the
/// height of the plane at its own place: a grid put one cell aside would be 1.8 m off. Outside there is
/// nothing.
#[test]
fn a_dem_is_brought_to_the_work_crs_cell_by_cell() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("35901.img");
    let plane = |x: f64, y: f64| 100.0 + 0.02 * (x - ORIGIN[0]) + 0.01 * (ORIGIN[1] - y);
    #[allow(clippy::cast_possible_truncation)]
    write_dem(&file, ORIGIN, |x, y| plane(x, y) as f32);

    let grid = read_elevation_grid(&[file], 5187).unwrap();
    assert!((grid.cell - CELL).abs() < 1e-9, "the cells keep the size of the file's: {}", grid.cell);
    assert_eq!((grid.heights.len(), grid.shade.len()), (grid.width * grid.height, grid.width * grid.height));
    let cells = cells_in_5179(&grid);
    let inside: Vec<_> = cells.iter().filter(|(p, _, _)| cells_inside(ORIGIN, *p) >= 2.0).collect();
    assert!(inside.len() > 800, "most of the 1200 cells are well inside: {}", inside.len());
    for (p, height, _) in &inside {
        assert!((f64::from(*height) - plane(p[0], p[1])).abs() < 0.05, "at {p:?}: {height} for {}", plane(p[0], p[1]));
    }
    // The two systems stand 0.9 degrees askew here, so the grid has corners the file does not reach.
    let outside: Vec<_> = cells.iter().filter(|(p, _, _)| cells_inside(ORIGIN, *p) <= -0.25).collect();
    assert!(!outside.is_empty(), "the grid has cells beside the file");
    assert!(outside.iter().all(|(_, height, shade)| height.is_nan() && *shade == 0), "{outside:?}");
    // The box of the grid, for the map: [min x, min y, max x, max y].
    #[allow(clippy::cast_precision_loss)]
    let extent = [grid.left, grid.top - grid.height as f64 * grid.cell, grid.left + grid.width as f64 * grid.cell, grid.top];
    assert_eq!(grid.extent(), extent);
}

/// Cells the file marks as empty stay empty: they are no heights to draw or to mix into their neighbours.
#[test]
fn empty_cells_stay_empty() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sea.img");
    let hole = |x: f64, y: f64| (x - ORIGIN[0]) > 20.0 * CELL && (ORIGIN[1] - y) > 15.0 * CELL;
    write_dem(&file, ORIGIN, |x, y| if hole(x, y) { -9999.0 } else { 250.0 });

    let grid = read_elevation_grid(&[file], 5187).unwrap();
    let cells = cells_in_5179(&grid);
    let deep: Vec<_> = cells.iter().filter(|(p, _, _)| cells_inside(ORIGIN, *p) >= 2.0 && (p[0] - ORIGIN[0]) > 22.0 * CELL && (ORIGIN[1] - p[1]) > 17.0 * CELL).collect();
    assert!(deep.len() > 100 && deep.iter().all(|(_, height, _)| height.is_nan()), "{} cells of the hole", deep.len());
    assert!(cells.iter().filter(|(_, height, _)| !height.is_nan()).all(|(_, height, _)| (*height - 250.0).abs() < 0.01), "no empty value is mixed in");
}

/// Two sheets side by side become one grid without a gap along their common edge.
#[test]
fn two_sheets_become_one_grid() {
    let dir = tempfile::tempdir().unwrap();
    let (west, east) = (dir.path().join("35901.img"), dir.path().join("35902.img"));
    #[allow(clippy::cast_precision_loss)]
    let east_origin = [ORIGIN[0] + SIZE.0 as f64 * CELL, ORIGIN[1]];
    write_dem(&west, ORIGIN, |_, _| 100.0);
    write_dem(&east, east_origin, |_, _| 300.0);

    let grid = read_elevation_grid(&[west, east], 5187).unwrap();
    assert!(grid.width >= 2 * SIZE.0 && grid.width <= 2 * SIZE.0 + 3, "both sheets side by side: {}", grid.width);
    let cells = cells_in_5179(&grid);
    let of = |origin: [f64; 2]| cells.iter().filter(move |(p, _, _)| cells_inside(origin, *p) >= 1.0).map(|(_, height, _)| *height).collect::<Vec<f32>>();
    assert!(of(ORIGIN).len() > 900 && of(ORIGIN).iter().all(|h| (*h - 100.0).abs() < 0.01));
    assert!(of(east_origin).len() > 900 && of(east_origin).iter().all(|h| (*h - 300.0).abs() < 0.01));
    // Along a row through the middle there is no empty cell between the first and the last height.
    let row = &grid.heights[(grid.height / 2) * grid.width..(grid.height / 2 + 1) * grid.width];
    let (first, last) = (row.iter().position(|h| !h.is_nan()).unwrap(), row.iter().rposition(|h| !h.is_nan()).unwrap());
    assert!(last - first >= 2 * SIZE.0 - 3 && row[first..=last].iter().all(|h| !h.is_nan()));
}

/// The light comes from the north-west, 45 degrees high (`BasemapOps.cpp:2072`): level ground is lit by
/// sin 45 degrees of full light, a slope that faces the light is brighter and one that faces away darker.
#[test]
fn a_slope_towards_the_light_is_brighter() {
    let dir = tempfile::tempdir().unwrap();
    let light = |name: &str, height: &dyn Fn(f64, f64) -> f64| {
        let file = dir.path().join(name);
        #[allow(clippy::cast_possible_truncation)]
        write_dem(&file, ORIGIN, |x, y| height(x, y) as f32);
        let grid = read_elevation_grid(&[file], 5187).unwrap();
        let inside: Vec<f64> = cells_in_5179(&grid).iter().filter(|(p, _, _)| cells_inside(ORIGIN, *p) >= 3.0).map(|(_, _, shade)| f64::from(*shade)).collect();
        #[allow(clippy::cast_precision_loss)]
        let mean = inside.iter().sum::<f64>() / inside.len() as f64;
        (mean, inside.iter().copied().fold(f64::INFINITY, f64::min), inside.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    };
    let level = light("level.img", &|_, _| 50.0);
    // Heights that grow towards the south-east: the ground faces north-west.
    let facing = light("facing.img", &|x, y| 0.3 * (x - ORIGIN[0]) + 0.3 * (ORIGIN[1] - y));
    let away = light("away.img", &|x, y| 5000.0 - 0.3 * (x - ORIGIN[0]) - 0.3 * (ORIGIN[1] - y));
    assert!((level.1 - level.2).abs() < 0.5 && (179.0..=182.0).contains(&level.0), "level ground: {level:?}");
    assert!(facing.0 > level.0 + 20.0 && away.0 < level.0 - 20.0, "facing {facing:?}, level {level:?}, away {away:?}");
}

/// What is no elevation raster is refused with words, not with a grid of nothing.
#[test]
fn what_is_no_elevation_raster_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let text = dir.path().join("note.img");
    std::fs::write(&text, "not a raster").unwrap();
    assert!(read_elevation_grid(&[text], 5187).is_err());
    assert!(read_elevation_grid(&[], 5187).is_err());
    assert!(read_elevation_grid(&[dir.path().join("missing.img")], 5187).is_err());
}
