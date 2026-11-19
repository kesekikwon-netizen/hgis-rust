//! WKT and layout checks for a `trial_trench` write (`TrenchLayerEdit.cpp:43-45`, `:122-130`).
//! Rings are work-CRS metres (EPSG:5186 or 5187), x then y.

use std::collections::BTreeMap;

use geo::{Coord, Geometry, LineString, MultiPolygon, Polygon};
use strata_geo::trench_plan::Cell;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_storage::wkb_codec::WkbGeometry;

pub(super) fn geometry_from_wkt(wkt: &str) -> Option<Geometry<f64>> {
    let parts = strata_storage::gpkg_wkt::wkt_to_parts(wkt).ok()?;
    let mut polys = Vec::new();
    for part in parts {
        let WkbGeometry::Polygon(rings) = part else {
            return None;
        };
        let mut rings = rings.into_iter();
        let exterior = rings.next()?;
        polys.push(Polygon::new(
            line_of(exterior),
            rings.map(line_of).collect(),
        ));
    }
    match polys.len() {
        0 => None,
        1 => polys.pop().map(Geometry::Polygon),
        _ => Some(Geometry::MultiPolygon(MultiPolygon::new(polys))),
    }
}

fn line_of(ring: Vec<[f64; 2]>) -> LineString<f64> {
    LineString::from(
        ring.into_iter()
            .map(|p| Coord { x: p[0], y: p[1] })
            .collect::<Vec<_>>(),
    )
}

pub(super) fn cell_feature(cell: &Cell) -> (String, BTreeMap<String, String>) {
    let mut attributes = BTreeMap::new();
    attributes.insert("name".to_owned(), cell.name.clone());
    attributes.insert("width".to_owned(), cell.width.to_string());
    attributes.insert("length".to_owned(), cell.length.to_string());
    (polygon_wkt(&cell.ring), attributes)
}

fn polygon_wkt(ring: &[[f64; 2]]) -> String {
    let body = ring
        .iter()
        .map(|p| format!("{} {}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(", ");
    format!("POLYGON (({body}))")
}

pub(super) fn auth_of(srs: i32) -> String {
    match srs {
        5186 => "EPSG:5186".to_owned(),
        5187 => "EPSG:5187".to_owned(),
        other => format!("EPSG:{other}"),
    }
}

pub(super) fn same_layout(table: &SurveyFileTable, cells: &[Cell]) -> bool {
    if table.layer.features.len() != cells.len() {
        return false;
    }
    let mut got = Vec::new();
    for feature in &table.layer.features {
        let (Some(name), Some(ring)) =
            (feature.attributes.get("name"), exterior(&feature.wkt_geom))
        else {
            return false;
        };
        got.push((name.clone(), ring));
    }
    let mut want: Vec<(String, Vec<[f64; 2]>)> = cells
        .iter()
        .map(|c| (c.name.clone(), c.ring.clone()))
        .collect();
    got.sort_by(|a, b| a.0.cmp(&b.0));
    want.sort_by(|a, b| a.0.cmp(&b.0));
    got.iter()
        .zip(&want)
        .all(|(g, w)| g.0 == w.0 && rings_near(&g.1, &w.1))
}

fn exterior(wkt: &str) -> Option<Vec<[f64; 2]>> {
    let parts = strata_storage::gpkg_wkt::wkt_to_parts(wkt).ok()?;
    let WkbGeometry::Polygon(rings) = parts.into_iter().next()? else {
        return None;
    };
    rings.into_iter().next()
}

fn rings_near(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    // Millimetres, same as the C++ layout signature (`TrenchLayerEdit.cpp:43-45`).
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| (p[0] - q[0]).abs() <= 1e-3 && (p[1] - q[1]).abs() <= 1e-3)
}

pub(super) fn extent_center(table: &SurveyFileTable) -> Option<[f64; 2]> {
    let mut rings = Vec::new();
    for feature in &table.layer.features {
        rings.push(exterior(&feature.wkt_geom)?);
    }
    box_center(rings)
}

pub(super) fn cells_center(cells: &[Cell]) -> Option<[f64; 2]> {
    box_center(cells.iter().map(|c| c.ring.clone()))
}

fn box_center(rings: impl IntoIterator<Item = Vec<[f64; 2]>>) -> Option<[f64; 2]> {
    let mut box_ = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for ring in rings {
        for p in ring {
            box_[0] = box_[0].min(p[0]);
            box_[1] = box_[1].min(p[1]);
            box_[2] = box_[2].max(p[0]);
            box_[3] = box_[3].max(p[1]);
        }
    }
    (box_[0].is_finite() && box_[2].is_finite())
        .then_some([(box_[0] + box_[2]) * 0.5, (box_[1] + box_[3]) * 0.5])
}
