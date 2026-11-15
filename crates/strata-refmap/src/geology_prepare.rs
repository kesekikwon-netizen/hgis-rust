//! The geology map of the KIGAM service for the area around a survey: fetched, checked, cut to the area,
//! coloured and stored (C++ `GeologyMapService::prepare`, `target/cpp-ref/src/core/GeologyMapService.cpp:
//! 734-913`). Two things differ from the C++ by the owner's decision (docs/RUST_PLAN.md 2.9, 2026-11-15):
//! the box asked for is that of the scope around the survey area, not the map view grown to 80 km, and the
//! rock units are cut to that scope.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;
use strata_engine::{point_on_surface, polygons_area, write_new_table, Cell, ColumnKind, PolygonRings, Row, Scope, ShapeKind, Shapes, TableLayout, Transformer};

use crate::collection::{complete_feature_collection, feature_collection, feature_count};
use crate::geojson::{cell, columns_of, declared_epsg, polygons_of};
use crate::geology_colour::{hex, pick_pixel_color};
use crate::geology_era::{era_class, unit_fallback_color, UNKNOWN_RGB};
use crate::geology_request::{features_url, layer_for, picture_pixel, picture_url, MAX_FEATURES, USER_AGENT};
use crate::tiled_fetch::{fetch, Options, Outcome, Piece, Range};
use crate::transfer::{new_storage, Download, Prepared, Request, Stopped, TransferFailure, Why, CANCELLED, NOT_REACHED, NOT_SAVED, NOT_VERIFIED, TOO_SLOW};

#[cfg(test)]
#[path = "geology_prepare_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "geology_prepare_tests_b.rs"]
mod tests_b;

/// Table, file and CRS of the stored map (`:739`, `MainWindowOverlay.cpp:539`, `:789`).
pub const TABLE: &str = "geology_map";
pub const FILE_NAME: &str = "지질도_KIGAM.gpkg";
pub const STORED_EPSG: u32 = 5186;
/// The class of the unit's era, a column the C++ adds (`kEraField`, `:63`).
pub const ERA_COLUMN: &str = "era_class";
/// The colour the unit is drawn with, `#RRGGBB`. The C++ keeps the colours in the project's layer style;
/// here they are kept in the file, so that the file alone can be drawn.
pub const COLOUR_COLUMN: &str = "ka_color";
/// The unit's symbol and era text as the service names them (`kSymbolField`, `kEraSrcField`, `:62-64`).
pub const SYMBOL_COLUMN: &str = "기호";
const ERA_TEXT_COLUMN: &str = "시대";
/// `maxSpanMeters`, `GeologyMapService.h:74`.
const MAX_SPAN: f64 = 80_000.0;

const TOO_WIDE: &str = "범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요.";
const NO_TRANSFORM: &str = "좌표 변환에 실패했습니다.";
const NOT_GEOJSON: &str = "서버가 GeoJSON 대신 다른 응답을 보냈습니다.";
const NOT_WHOLE: &str = "지질도 전체를 읽지 못했습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다.";
const NOT_FETCHED: &str = "지질도를 내려받지 못했습니다.";
const UNKNOWN_CRS: &str = "받은 지질도의 좌표계를 확인할 수 없습니다. 기존 지도는 유지됩니다.";
const NOT_CARRIED: &str = "지질도 좌표를 변환하지 못했습니다. 기존 지도는 유지됩니다.";
const UNREADABLE: &str = "지질도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다.";
/// `:892`.
pub const NO_DATA: &str = "이 범위에는 지질도 데이터가 없습니다.";
/// `:907`.
pub const NO_OFFICIAL_COLOUR: &str = "공식 지질도 색을 받지 못해 지질시대 기준색으로 표시합니다. 인터넷 연결 후 다시 내려받으면 공식 색을 적용합니다.";

fn failed(why: Why, text: impl Into<String>) -> Stopped {
    Stopped::Failed(why, text.into())
}

/// One box of rock units (`fetchLithoPiece`, `:226-270`).
fn fetch_piece(layer: &str, range: &Range, download: Download<'_>) -> Piece {
    let piece = |outcome, error: &str| Piece { outcome, body: Vec::new(), error: error.to_owned() };
    let body = match download(&Request::large(features_url(layer, range), USER_AGENT)) {
        Ok(body) => body,
        Err(TransferFailure::Cancelled) => return piece(Outcome::Cancelled, ""),
        Err(TransferFailure::Timeout) => return piece(Outcome::Slow, TOO_SLOW),
        Err(TransferFailure::Network) => return piece(Outcome::Transient, NOT_REACHED),
    };
    if !body.trim_ascii_start().starts_with(b"{") {
        return piece(Outcome::Fatal, NOT_GEOJSON);
    }
    let received = match feature_collection(&body) {
        Ok(collection) => feature_count(&collection),
        Err(error) => return piece(Outcome::Fatal, &error),
    };
    if let Err(error) = complete_feature_collection(&body) {
        return piece(Outcome::TooLarge, &error);
    }
    if received >= MAX_FEATURES {
        return piece(Outcome::TooLarge, NOT_WHOLE);
    }
    Piece { outcome: Outcome::Complete, body, error: String::new() }
}

/// The official colour of each unit, read at its point from one picture of the whole box
/// (`sampleOfficialColors`, `:169-222`). Empty when the picture cannot be had.
fn official_colours(range: &Range, points: &HashMap<String, (f64, [f64; 2])>, to_lonlat: &Transformer, download: Download<'_>) -> HashMap<String, [u8; 3]> {
    if points.is_empty() {
        return HashMap::new();
    }
    let Some(picture) = download(&Request::large(picture_url(range), USER_AGENT)).ok().and_then(|body| image::load_from_memory(&body).ok()) else {
        return HashMap::new();
    };
    let picture = picture.to_rgba8();
    let colour = |point: &[f64; 2]| {
        let (px, py) = picture_pixel(range, to_lonlat.transform(*point).ok()?);
        pick_pixel_color(&picture, px, py)
    };
    points.iter().filter_map(|(symbol, (_, point))| colour(point).map(|rgb| (symbol.clone(), rgb))).collect()
}

/// Fetches the rock units around the survey and stores them beside `requested` (the path the map is known
/// by; the file itself is written into a new folder next to it, see `new_storage`). `scope` is in
/// EPSG:5186. `download` sends the requests; `cancelled` is asked between the steps.
///
/// # Errors
/// `Stopped::Cancelled` when the user stopped it, `Stopped::Failed` with the C++ sentence otherwise. Nothing
/// is left on disk in either case.
pub fn prepare(scope: &Scope, requested: &Path, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<Prepared, Stopped> {
    if cancelled() {
        return Err(Stopped::Cancelled(CANCELLED.to_owned()));
    }
    let bbox = scope.bbox().map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    if bbox.iter().any(|v| !v.is_finite()) || bbox[2] - bbox[0] > MAX_SPAN || bbox[3] - bbox[1] > MAX_SPAN {
        return Err(failed(Why::TooWide, TOO_WIDE));
    }
    let to_lonlat = Transformer::new(STORED_EPSG, 4326).map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    let range = to_lonlat.transform_box(bbox).map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    let Some(layer) = layer_for(&range) else {
        return Err(failed(Why::NoData, NO_DATA));
    };
    let (folder, gpkg) = new_storage(requested)?;
    match build(scope, layer, &range, &to_lonlat, &gpkg, download, cancelled) {
        Ok((rows, warnings)) => Ok(Prepared { folder, gpkg, rows, warnings }),
        Err(stopped) => {
            // The folder was made by this call a moment ago: nobody else has anything in it.
            let _ = std::fs::remove_dir_all(&folder);
            Err(stopped)
        }
    }
}

/// A unit that will be stored: its shapes inside the scope, its cells, and what its colour depends on.
struct Unit {
    polygons: Vec<PolygonRings>,
    cells: Vec<Cell>,
    symbol: String,
    era_text: String,
}

fn build(scope: &Scope, layer: &str, range: &Range, to_lonlat: &Transformer, gpkg: &Path, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<(u64, Vec<String>), Stopped> {
    let stop = || Stopped::Cancelled(CANCELLED.to_owned());
    let fetched = fetch(range, &mut |piece| fetch_piece(layer, &piece, download), cancelled, Options::default());
    if fetched.cancelled || cancelled() {
        return Err(stop());
    }
    if !fetched.ok {
        return Err(failed(Why::Service, if fetched.error.is_empty() { NOT_FETCHED.to_owned() } else { format!("KIGAM 서버 연결 실패: {}", fetched.error) }));
    }
    let collection = feature_collection(&fetched.body).map_err(|text| failed(Why::Service, text))?;
    let features = collection.get("features").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    if features.is_empty() {
        return Err(failed(Why::NoData, NO_DATA));
    }
    // An answer that names a CRS nobody knows is refused, not read as longitude and latitude (`:795-816`).
    let source = declared_epsg(&collection).ok_or_else(|| failed(Why::Data, UNKNOWN_CRS))?;
    let carry = Transformer::new(source, STORED_EPSG).map_err(|_| failed(Why::Data, UNKNOWN_CRS))?;
    let columns: Vec<(String, ColumnKind)> = columns_of(features).into_iter().filter(|(name, _)| name != ERA_COLUMN && name != COLOUR_COLUMN).collect();

    let mut units = Vec::new();
    // The largest shape of each symbol and a point inside it: where its official colour is read (`:877-887`).
    let mut colour_points: HashMap<String, (f64, [f64; 2])> = HashMap::new();
    for feature in features {
        if cancelled() {
            return Err(stop());
        }
        let shapes = feature.get("geometry").and_then(polygons_of).filter(|p| !p.is_empty()).ok_or_else(|| failed(Why::Data, UNREADABLE))?;
        let mut polygons = Vec::new();
        for rings in shapes {
            let mut carried = Vec::with_capacity(rings.len());
            for ring in rings {
                let ring = if source == STORED_EPSG { ring } else { carry.transform_all(&ring).map_err(|_| failed(Why::Data, NOT_CARRIED))? };
                if ring.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(failed(Why::Data, NOT_CARRIED));
                }
                carried.push(ring);
            }
            polygons.extend(scope.cut_polygon(&carried).map_err(|_| failed(Why::Data, UNREADABLE))?);
        }
        if polygons.is_empty() {
            continue;
        }
        let properties = feature.get("properties").and_then(Value::as_object);
        let text = |name: &str| properties.and_then(|p| p.get(name)).and_then(Value::as_str).unwrap_or_default().to_owned();
        let (symbol, era_text) = (text(SYMBOL_COLUMN), text(ERA_TEXT_COLUMN));
        if !symbol.is_empty() {
            let area = polygons_area(&polygons).unwrap_or(0.0);
            if area > colour_points.get(&symbol).map_or(-1.0, |known| known.0) {
                if let Ok(Some(point)) = point_on_surface(&polygons) {
                    colour_points.insert(symbol.clone(), (area, point));
                }
            }
        }
        let cells = columns.iter().map(|(name, kind)| cell(properties.and_then(|p| p.get(name)), *kind)).collect();
        units.push(Unit { polygons, cells, symbol, era_text });
    }
    if units.is_empty() {
        return Err(failed(Why::NoData, NO_DATA));
    }

    let official = official_colours(range, &colour_points, to_lonlat, download);
    if cancelled() {
        return Err(stop());
    }
    let warnings = if official.is_empty() { vec![NO_OFFICIAL_COLOUR.to_owned()] } else { Vec::new() };
    let mut layout: Vec<(&str, ColumnKind)> = columns.iter().map(|(name, kind)| (name.as_str(), *kind)).collect();
    layout.extend([(ERA_COLUMN, ColumnKind::Text), (COLOUR_COLUMN, ColumnKind::Text)]);
    let expected = units.len() as u64;
    let mut rows = units.into_iter().map(|mut unit| {
        // The official colour of the symbol; without one, the era's colour shaded by the symbol; a unit
        // without a symbol is the grey 「기타」 of the C++ legend (`:382-458`).
        let rgb = if unit.symbol.is_empty() { UNKNOWN_RGB } else { official.get(&unit.symbol).copied().unwrap_or_else(|| unit_fallback_color(&unit.era_text, &unit.symbol)) };
        unit.cells.extend([Cell::Text(era_class(&unit.era_text).to_owned()), Cell::Text(hex(rgb))]);
        Row { shapes: Shapes::Polygons(unit.polygons), cells: unit.cells }
    });
    let table = TableLayout { table: TABLE, epsg: STORED_EPSG, kind: ShapeKind::Polygons, columns: &layout };
    let stored = write_new_table(gpkg, &table, &mut rows).map_err(|_| failed(Why::Storage, NOT_SAVED))?;
    if stored != expected {
        return Err(failed(Why::Storage, NOT_VERIFIED));
    }
    Ok((stored, warnings))
}
