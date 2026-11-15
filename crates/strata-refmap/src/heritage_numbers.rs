//! The numbers of the heritage sites on a sheet (`target/cpp-ref/src/core/HeritageLayoutNumbers.cpp:194-217,
//! 625-960`): only the sites within 500 m of the survey area that the map of the sheet shows get a number
//! and a row of the legend; farther sites stay drawn in the colour of their kind. A kind counts 1, 2, 3 …
//! through its layers in the order of the legend; in a layer the same name is one number.

use std::collections::BTreeMap;

use strata_engine::{point_on_surface, polygons_area, PolygonRings, Scope, Transformer};

#[cfg(test)]
#[path = "heritage_numbers_tests.rs"]
mod tests;

/// `kNumberedNearSurveyM` (`:196`).
pub const NUMBERED_NEAR_M: f64 = 500.0;
/// The name of a site without one (`HeritageStyle.cpp:140`).
pub const UNNAMED: &str = "무명";

/// A row of a heritage layer: its name (empty when it has none) and its shapes.
pub type Site = (String, Vec<PolygonRings>);

/// A heritage layer as the sheet draws it: its kind, the CRS of its shapes, and its sites.
#[derive(Debug, Clone, Copy)]
pub struct SheetLayer<'a> {
    pub kind: &'a str,
    pub epsg: u32,
    pub sites: &'a [Site],
}

/// A site with its number.
#[derive(Debug, Clone, PartialEq)]
pub struct Numbered {
    /// The place of its layer among the layers handed in.
    pub layer: usize,
    pub name: String,
    pub number: u32,
    /// Where the number stands, in the work CRS: inside the largest piece of the site.
    pub at: [f64; 2],
    /// The ground of the site within the numbered area, joined, in the work CRS (`:870-873`): a number
    /// that must move looks for a free place on it first.
    pub pieces: Vec<PolygonRings>,
}

/// Where sites are numbered on a sheet whose map shows `sheet_box` (`[min x, min y, max x, max y]`, work
/// CRS): the survey areas (WKT, work CRS) grown by 500 m, inside the map; without a survey area the whole
/// map. `None`: the map shows nothing near the survey area.
///
/// # Errors
/// The shapes cannot be joined, grown or cut.
pub fn numbered_area(survey_areas: &[String], sheet_box: [f64; 4]) -> Result<Option<Scope>, String> {
    let [min_x, min_y, max_x, max_y] = sheet_box;
    let sheet = vec![vec![[min_x, min_y], [max_x, min_y], [max_x, max_y], [min_x, max_y], [min_x, min_y]]];
    let wkt = |rings: &PolygonRings| {
        let ring = |ring: &Vec<[f64; 2]>| format!("({})", ring.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", "));
        format!("POLYGON ({})", rings.iter().map(ring).collect::<Vec<_>>().join(", "))
    };
    // Without a drawn survey area every site on the paper is numbered (`:198-199`).
    let Some(near) = Scope::around(survey_areas, NUMBERED_NEAR_M).map_err(|e| e.0)? else {
        return Scope::of(&[wkt(&sheet)]).map_err(|e| e.0);
    };
    // The paper of the map, cut to the neighbourhood of the survey (`:757-759`).
    let on_sheet: Vec<String> = near.cut_polygon(&sheet).map_err(|e| e.0)?.iter().map(wkt).collect();
    Scope::of(&on_sheet).map_err(|e| e.0)
}

/// Numbers the sites of the layers, which stand in the order of the legend (the top row first).
///
/// # Errors
/// A shape cannot be tested or cut, or a CRS cannot be carried.
pub fn number_sites(layers: &[SheetLayer<'_>], area: &Scope, work_epsg: u32) -> Result<Vec<Numbered>, String> {
    /// A site of a layer: its name, the size of its largest piece within the area, a point on that piece,
    /// and all its pieces within the area.
    struct Found {
        name: String,
        weight: f64,
        at: Option<[f64; 2]>,
        pieces: Vec<PolygonRings>,
    }
    let wkt = |rings: &PolygonRings| {
        let ring = |ring: &Vec<[f64; 2]>| format!("({})", ring.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", "));
        format!("POLYGON ({})", rings.iter().map(ring).collect::<Vec<_>>().join(", "))
    };
    let mut counters: BTreeMap<&str, u32> = BTreeMap::new();
    let mut numbered = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        // The sites are tested where they lie: the area is carried into the CRS of the layer.
        let carried;
        let (near, back) = if layer.epsg == work_epsg {
            (area, None)
        } else {
            carried = area.reprojected(work_epsg, layer.epsg).map_err(|e| e.0)?;
            (&carried, Some(Transformer::new(layer.epsg, work_epsg).map_err(|e| e.0)?))
        };
        let test = near.test().map_err(|e| e.0)?;
        // In the order of the names, as the C++ map of strings keeps them (by UTF-16 units, `:765, 925`).
        let mut sites: BTreeMap<Vec<u16>, Found> = BTreeMap::new();
        for (name, shapes) in layer.sites {
            let mut pieces = Vec::new();
            for rings in shapes {
                if test.touches_polygon(rings).map_err(|e| e.0)? {
                    pieces.extend(near.cut_polygon(rings).map_err(|e| e.0)?);
                }
            }
            if pieces.is_empty() {
                continue;
            }
            let name = Some(name.trim()).filter(|name| !name.is_empty()).unwrap_or(UNNAMED);
            let site = sites.entry(name.encode_utf16().collect()).or_insert_with(|| Found { name: name.to_owned(), weight: 0.0, at: None, pieces: Vec::new() });
            // The number stands on the largest piece of a site of several (`:845-855`).
            let weight = polygons_area(&pieces).map_err(|e| e.0)?.max(1.0);
            if weight > site.weight {
                if let Some(at) = point_on_surface(&pieces).map_err(|e| e.0)? {
                    (site.weight, site.at) = (weight, Some(at));
                }
            }
            site.pieces.extend(pieces);
        }
        for site in sites.into_values() {
            let Some(at) = site.at else {
                continue;
            };
            // The ground of the site: its pieces joined (`:870-873`), carried into the work CRS with the point.
            let joined = Scope::of(&site.pieces.iter().map(wkt).collect::<Vec<_>>()).map_err(|e| e.0)?.map(|ground| ground.polygons()).transpose().map_err(|e| e.0)?.unwrap_or_default();
            let (at, pieces) = match &back {
                Some(back) => {
                    let carried: Result<Vec<PolygonRings>, _> = joined.iter().map(|rings| rings.iter().map(|ring| back.transform_all(ring)).collect()).collect();
                    (back.transform(at).map_err(|e| e.0)?, carried.map_err(|e| e.0)?)
                }
                None => (at, joined),
            };
            let number = counters.entry(layer.kind).or_insert(0);
            *number += 1;
            numbered.push(Numbered { layer: index, name: site.name, number: *number, at, pieces });
        }
    }
    Ok(numbered)
}

/// The sites of a layer's copy (`heritage::import_shapefile`): the CRS of its shapes, and for every row its
/// name (empty when the copy has none) and its shapes.
///
/// # Errors
/// The copy cannot be read, or has no known CRS.
pub fn read_sites(copy: &std::path::Path) -> Result<(u32, Vec<Site>), String> {
    use strata_engine::{read_table, Cell, Shapes};
    let mut sites = Vec::new();
    let facts = read_table(copy, crate::heritage::TABLE, &[crate::heritage::NAME_COLUMN], &mut |row| {
        let name = if let Some(Cell::Text(name)) = row.cells.into_iter().next() { name } else { String::new() };
        // A line has no ground for a number to stand on.
        let shapes = if let Shapes::Polygons(polygons) = row.shapes { polygons } else { Vec::new() };
        sites.push((name, shapes));
        true
    })
    .map_err(|e| e.0)?;
    let epsg = facts.epsg.ok_or_else(|| format!("{}: no known CRS", copy.display()))?;
    Ok((epsg, sites))
}
