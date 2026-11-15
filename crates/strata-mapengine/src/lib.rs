//! The one place where Strata calls the RustGIS engine (github.com/opengeos/RustGIS, Apache-2.0, v0.6.0).
//!
//! The C++ app lets QGIS write the labels of a reference map: over the part of each shape that is in
//! view, never one over another (`target/cpp-ref/src/core/GeologyMapService.cpp:426-455`,
//! `SoilMapService.cpp:200-236`). Here that is done by two parts of the Rust GIS ecosystem. The shapes of
//! a labelled table are cut to the view with the `geo` crate (QGIS cuts a shape to the map before it looks
//! for the label's place; the RustGIS engine labels a polygon at the middle of the whole shape, so a unit
//! larger than the view would get no label). The RustGIS engine then carries the cut shapes into the work
//! CRS, as QGIS does on the fly, finds the label point of each (the centroid of its largest part, moved
//! inside it), and keeps labels from covering one another. The app writes the letters in its own font.
//!
//! The engine draws layers too, but its CPU picture has an opaque background and cannot be laid over
//! Strata's map; the shapes are drawn by the app until the whole map moves to the engine's GPU scene.

use std::path::PathBuf;

use geo::{BooleanOps, BoundingRect, Intersects};
use rustgis_data::{Dataset, Field, FieldType, Geometry, GeometryType, Value};
use rustgis_geom::Coord;
use rustgis_map::projection::Projector;
use rustgis_map::{Basemap, Layer, Map, Marker, Projection, Renderer, Rgba, Symbol};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

/// How the labels of a layer look.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelLook {
    /// The column whose text is written.
    pub column: String,
    /// Letter height in screen points.
    pub size_pt: f32,
    pub ink: [u8; 4],
    pub bold: bool,
    /// Colour and width (screen points) of the edge around the letters.
    pub halo: Option<([u8; 4], f32)>,
    /// Labels are written only at this scale or closer (1:40000 is 40000).
    pub closest_scale: Option<f64>,
}

/// A labelled table of the survey folder.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelSource {
    /// The app's name for the layer.
    pub key: String,
    pub file: PathBuf,
    pub look: LabelLook,
}

/// The part of the map in view: centre and scale in the work CRS, size in screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    pub centre: [f64; 2],
    pub metres_per_point: f64,
    pub width_pt: f64,
    pub height_pt: f64,
}

/// A label and the place of its middle in the work CRS.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedText {
    pub text: String,
    pub at: [f64; 2],
    pub size_pt: f32,
    pub ink: [u8; 4],
    pub halo: Option<([u8; 4], f32)>,
    pub bold: bool,
}

/// One row of a labelled table: its polygons (longitude and latitude), their box, and its label value.
type Shape = (geo::Rect<f64>, geo::MultiPolygon<f64>, Value);

/// The labelled tables, read once and kept while they are asked for.
#[derive(Debug, Default)]
pub struct LabelEngine {
    held: Vec<(LabelSource, Vec<Shape>)>,
}

/// The polygons of a row as `geo` has them. Rows of other kinds of shape get no label here.
fn polygons(geometry: Option<&Geometry>) -> Option<geo::MultiPolygon<f64>> {
    let Some(Geometry::Polygon(parts)) = geometry else {
        return None;
    };
    let ring = |ring: &Vec<Coord>| geo::LineString::from(ring.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>());
    let polygon = |rings: &Vec<Vec<Coord>>| rings.split_first().map(|(outer, holes)| geo::Polygon::new(ring(outer), holes.iter().map(ring).collect()));
    Some(geo::MultiPolygon::new(parts.iter().filter_map(polygon).collect()))
}

/// Reads a table and keeps what labels need of it; `None` when it has no such column
/// (`GeologyMapService.cpp:427`).
fn read(source: &LabelSource) -> Result<Option<Vec<Shape>>, String> {
    let (dataset, _) = rustgis_data::io::read_path(&source.file).map_err(|e| e.to_string())?;
    let Some(column) = dataset.field_index(&source.look.column) else {
        return Ok(None);
    };
    let shape = |feature: &rustgis_data::Feature| {
        let shapes = polygons(feature.geometry.as_ref())?;
        Some((shapes.bounding_rect()?, shapes, feature.attrs.get(column).cloned().unwrap_or_default()))
    };
    Ok(Some(dataset.features.iter().filter_map(shape).collect()))
}

impl LabelEngine {
    /// Brings what is held in line with `sources`: a table no longer asked for goes, a new one (or one
    /// whose file or look changed) is read.
    fn hold(&mut self, sources: &[LabelSource]) -> Result<(), String> {
        self.held.retain(|(source, _)| sources.contains(source));
        let fresh: Vec<&LabelSource> = sources.iter().filter(|source| !self.held.iter().any(|(held, _)| held == *source)).collect();
        for source in fresh {
            self.held.push((source.clone(), read(source)?.unwrap_or_default()));
        }
        Ok(())
    }

    /// Where the labels of these layers go for this view of the map drawn in `work_epsg`.
    ///
    /// # Errors
    /// The engine does not know the coordinate system, or a table cannot be read; the text is for the log.
    pub fn place(&mut self, work_epsg: u32, sources: &[LabelSource], view: &ViewBox) -> Result<Vec<PlacedText>, String> {
        let code = format!("EPSG:{work_epsg}");
        let projector = Projector::new(&code)?;
        self.hold(sources)?;
        // The view as longitude and latitude: its four corners carried back from the work CRS.
        let (half_w, half_h) = (view.width_pt / 2.0 * view.metres_per_point, view.height_pt / 2.0 * view.metres_per_point);
        let corner = |dx: f64, dy: f64| projector.inverse(Coord { x: view.centre[0] + dx, y: view.centre[1] + dy }).map(|c| (c.x, c.y));
        let Some(corners) = [corner(-half_w, -half_h), corner(half_w, -half_h), corner(half_w, half_h), corner(-half_w, half_h)].into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(Vec::new());
        };
        let in_view = geo::MultiPolygon::new(vec![geo::Polygon::new(geo::LineString::from(corners), Vec::new())]);
        let Some(view_box) = in_view.bounding_rect() else {
            return Ok(Vec::new());
        };

        let mut map = Map::new("strata");
        map.basemap = Basemap::Blank;
        map.set_projection(Projection::Custom(code));
        for (id, (source, shapes)) in (1_u32..).zip(&self.held) {
            let mut cut = Dataset::new(source.key.clone(), GeometryType::Polygon, vec![Field::new(source.look.column.clone(), FieldType::Text)]);
            for (bounds, shape, value) in shapes.iter().filter(|(bounds, ..)| bounds.intersects(&view_box)) {
                let _ = bounds;
                let part = shape.intersection(&in_view);
                let rings = |polygon: &geo::Polygon<f64>| std::iter::once(polygon.exterior()).chain(polygon.interiors()).map(|ring| ring.coords().map(|c| Coord { x: c.x, y: c.y }).collect::<Vec<_>>()).collect::<Vec<_>>();
                let parts: Vec<Vec<Vec<Coord>>> = part.iter().map(rings).collect();
                if !parts.is_empty() {
                    cut.push(Some(Geometry::Polygon(parts)), vec![value.clone()]);
                }
            }
            let mut layer = Layer::new(id, cut);
            // Nothing of the shapes is drawn: the app draws them.
            let unseen = Rgba([0, 0, 0, 0]);
            layer.renderer = Renderer::Single(Symbol { fill: unseen, stroke: unseen, stroke_width: 0.0, size: 0.0, marker: Marker::Circle });
            layer.labels.enabled = true;
            let class = &mut layer.labels.default;
            class.field.clone_from(&source.look.column);
            class.size = source.look.size_pt;
            class.color = Rgba(source.look.ink);
            class.bold = source.look.bold;
            class.halo = source.look.halo.is_some();
            class.halo_color = source.look.halo.map(|(colour, _)| Rgba(colour));
            class.halo_width = source.look.halo.map(|(_, width)| width);
            class.min_scale = source.look.closest_scale;
            map.add_layer(layer);
        }
        map.camera.center = Coord { x: view.centre[0], y: view.centre[1] };
        map.camera.res = view.metres_per_point;
        let asked = rustgis_render::View { width: view.width_pt, height: view.height_pt, pixels_per_point: 1.0, camera: map.camera, sky: false };
        let Some(drawn) = rustgis_render::render(&mut map, &asked) else {
            return Ok(Vec::new());
        };
        let world = |x: f32, y: f32| [view.centre[0] + (f64::from(x) - view.width_pt / 2.0) * view.metres_per_point, view.centre[1] - (f64::from(y) - view.height_pt / 2.0) * view.metres_per_point];
        Ok(drawn
            .labels
            .into_iter()
            .map(|label| PlacedText { at: world(label.x, label.y), text: label.text, size_pt: label.size, ink: label.color.0, halo: label.halo.map(|colour| (colour.0, label.halo_width.unwrap_or(1.0))), bold: label.bold })
            .collect())
    }
}
