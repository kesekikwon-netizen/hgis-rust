//! A layer whose look changes while it is on the map (「DEM 표현」).

use strata_engine::ElevationGrid;

use super::*;
use crate::ref_vector::data::HeightRamp;

const DEM: &str = "ref_dem";
const RELIEF: &str = "ref_dem_relief";

/// 10 x 10 cells of 90 m around the middle of the test map, heights 0 m, 10 m … 990 m.
fn grid() -> Arc<ElevationGrid> {
    #[allow(clippy::cast_precision_loss)]
    let heights = (0..100).map(|i| i as f32 * 10.0).collect();
    Arc::new(ElevationGrid { left: CENTRE[0] - 450.0, top: CENTRE[1] + 450.0, cell: 90.0, width: 10, height: 10, heights, shade: vec![181; 100] })
}

fn elevation_spec(look: Look) -> LoadSpec {
    LoadSpec { file: PathBuf::from("a DEM folder that is not there"), table: String::new(), work_epsg: 5187, look, label: None }
}

/// Another colour preset makes another picture from the heights that were read: the DEM is not read again
/// and the map is not left without its picture meanwhile. The same preset again asks for nothing.
#[test]
fn another_colour_preset_makes_another_picture_from_the_heights_already_read() {
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(DEM, "DEM", elevation_spec(Look::Heights { ramp: HeightRamp::National }));
    let heights = grid();
    app.ref_vectors.set_ready(DEM, Drawn::Heights(Arc::clone(&heights)));
    let ctx = egui::Context::default();
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);
    assert_eq!(app.ref_vectors.pictures_made(), 1);

    let lowland = Look::Heights { ramp: HeightRamp::Lowland };
    app.ref_vectors.set_look(DEM, lowland.clone());
    assert_eq!(app.ref_vectors.look_of(DEM), Some(&lowland));
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1, "a picture is on the map in every frame");
    assert_eq!(app.ref_vectors.pictures_made(), 2, "made again in the new colours");
    let all = [CENTRE[0] - 1000.0, CENTRE[1] - 1000.0, CENTRE[0] + 1000.0, CENTRE[1] + 1000.0];
    assert_eq!(app.ref_vectors.height_range(DEM, all), Some((0.0, 990.0)), "the heights are still those that were read");
    assert!(Arc::strong_count(&heights) >= 2);

    app.ref_vectors.set_look(DEM, lowland);
    let _ = frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.pictures_made(), 2);
    // A table has no heights to ask for.
    assert_eq!(app.ref_vectors.height_range("no such layer", all), None);
}

/// The light of a relief is made when its DEM is read, so another exaggeration reads it again (here the
/// folder is not there: the reading fails and the layer stands without data).
#[test]
fn another_exaggeration_reads_the_relief_again() {
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(RELIEF, "지형 음영", elevation_spec(Look::Relief { z_factor: 1.0 }));
    app.ref_vectors.set_ready(RELIEF, Drawn::Heights(grid()));
    let ctx = egui::Context::default();
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);
    assert!(app.ref_vectors.height_range(RELIEF, [0.0, 0.0, 1e9, 1e9]).is_some());

    app.ref_vectors.set_look(RELIEF, Look::Relief { z_factor: 2.0 });
    let _ = frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.look_of(RELIEF), Some(&Look::Relief { z_factor: 2.0 }));
    assert!(app.ref_vectors.height_range(RELIEF, [0.0, 0.0, 1e9, 1e9]).is_none(), "what was read with the old exaggeration is let go");
}
