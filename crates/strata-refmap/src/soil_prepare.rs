//! The soil map of the NAAS (흙토람) service for the area around a survey: counted, fetched table by table,
//! checked, cut to the area and stored with the colour and the label of each unit (C++
//! `SoilMapService::prepare`, `target/cpp-ref/src/core/SoilMapService.cpp:250-349`). Two things differ from
//! the C++ by the owner's decision (docs/RUST_PLAN.md 2.9, 2026-11-15): the box asked for is that of the
//! scope around the survey area, not the map view, and the units are cut to that scope.

use std::path::Path;

use serde_json::Value;
use strata_engine::{polygons_area, write_new_table, Cell, ColumnKind, Row, Scope, ShapeKind, Shapes, TableLayout, Transformer};

use crate::collection::{complete_feature_collection, feature_count};
use crate::geojson::{cell, declared_epsg, polygons_of};
use crate::geology_colour::hex;
use crate::soil_request::{features_url, number_matched, MAX_FEATURES, TABLES, TERRAIN_FIELD, USER_AGENT};
use crate::soil_terrain::{label_text, terrain_colour};
use crate::tiled_fetch::Range;
use crate::transfer::{new_storage, Download, Prepared, Request, Stopped, TransferFailure, Why, CANCELLED, NOT_REACHED, NOT_SAVED, NOT_VERIFIED, TOO_SLOW};

#[cfg(test)]
#[path = "soil_prepare_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "soil_prepare_tests_b.rs"]
mod tests_b;

/// Table, file and CRS of the stored map (`:254`, `MainWindowOverlay.cpp:481`, `:271`).
pub const TABLE: &str = "soil_map";
pub const FILE_NAME: &str = "토양도_흙토람.gpkg";
pub const STORED_EPSG: u32 = 5186;
/// The colour the unit is drawn with, `#RRGGBB`, and what is written over it. The C++ keeps both in the
/// project's layer style (`applyTerrainStyle`, `:152-171`; `terrainLabelExpression`, `:173-198`); here they
/// are kept in the file, so that the file alone can be drawn.
pub const COLOUR_COLUMN: &str = "ka_color";
pub const LABEL_COLUMN: &str = "ka_label";

/// `maxSpanMeters`, `SoilMapService.h:62`.
const MAX_SPAN: f64 = 80_000.0;
/// A unit smaller than this is not labelled (`applyTerrainLabels`, `SoilMapService.h:49`; `:176`).
const LABEL_MIN_AREA: f64 = 40_000.0;

/// `:259`.
pub const TOO_WIDE: &str = "토양도 범위가 너무 넓습니다. 한 변 80 km 이하로 확대하고 다시 내려받으세요.";
/// `:268`.
pub const NO_TRANSFORM: &str = "지도의 위치를 변환하지 못했습니다. 조사 좌표계를 확인하고 다시 내려받으세요.";
/// `:292`.
pub const TOO_MANY: &str = "토양도 데이터가 한 번에 받을 수 있는 양을 넘었습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다.";
/// `:309`.
pub const NOT_WHOLE: &str = "토양도 일부를 완전히 읽지 못했습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다.";
/// `:330`.
pub const UNREADABLE: &str = "토양도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요.";
/// `:342`.
pub const NO_DATA: &str = "이 범위에는 토양도 데이터가 없습니다. 다른 위치를 확인하세요. 기존 지도는 유지됩니다.";

/// Fetches the soil units around the survey and stores them beside `requested` (the path the map is known
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
    if bbox.iter().any(|v| !v.is_finite()) || bbox[2] <= bbox[0] || bbox[3] <= bbox[1] || bbox[2] - bbox[0] > MAX_SPAN || bbox[3] - bbox[1] > MAX_SPAN {
        return Err(failed(Why::TooWide, TOO_WIDE));
    }
    let range = Transformer::new(STORED_EPSG, 4326).and_then(|to_lonlat| to_lonlat.transform_box(bbox)).map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    let (folder, gpkg) = new_storage(requested)?;
    match build(scope, &range, &gpkg, download, cancelled) {
        Ok(rows) => Ok(Prepared { folder, gpkg, rows, warnings: Vec::new() }),
        Err(stopped) => {
            // The folder was made by this call a moment ago: nobody else has anything in it.
            let _ = std::fs::remove_dir_all(&folder);
            Err(stopped)
        }
    }
}

fn failed(why: Why, text: &str) -> Stopped {
    Stopped::Failed(why, text.to_owned())
}

/// The features of one table (`:278-312`), or none when the table matches nothing in the box.
fn fetch_table(table: u8, range: &Range, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<Option<serde_json::Map<String, Value>>, Stopped> {
    let stop = || Stopped::Cancelled(CANCELLED.to_owned());
    // The count first. One that cannot be had or read is gone without (`:283-295`).
    let counted = match download(&Request::plain(features_url(table, range, true), USER_AGENT)) {
        Ok(body) => number_matched(&body),
        Err(TransferFailure::Cancelled) => return Err(stop()),
        Err(_) => None,
    };
    if cancelled() {
        return Err(stop());
    }
    match counted {
        Some(0) => return Ok(None),
        Some(count) if count > MAX_FEATURES => return Err(failed(Why::TooWide, TOO_MANY)),
        _ => {}
    }
    let body = match download(&Request::large(features_url(table, range, false), USER_AGENT)) {
        Ok(body) => body,
        Err(TransferFailure::Cancelled) => return Err(stop()),
        Err(TransferFailure::Timeout) => return Err(failed(Why::Service, TOO_SLOW)),
        Err(TransferFailure::Network) => return Err(failed(Why::Service, NOT_REACHED)),
    };
    let collection = complete_feature_collection(&body).map_err(|text| failed(Why::Service, &text))?;
    let received = feature_count(&collection) as u64;
    if received >= MAX_FEATURES || counted.is_some_and(|count| received < count) {
        return Err(failed(Why::Service, NOT_WHOLE));
    }
    Ok(Some(collection))
}

fn build(scope: &Scope, range: &Range, gpkg: &Path, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<u64, Stopped> {
    let stop = || Stopped::Cancelled(CANCELLED.to_owned());
    let mut rows = Vec::new();
    for table in TABLES {
        if cancelled() {
            return Err(stop());
        }
        let Some(collection) = fetch_table(table, range, download, cancelled)? else {
            continue;
        };
        // The request asks for EPSG:5186. An answer that names another system is carried there; one that
        // names a system nobody knows cannot be placed (the C++ copies the coordinates as they come).
        let source = declared_epsg(&collection).ok_or_else(|| failed(Why::Service, NOT_WHOLE))?;
        let carry = Transformer::new(source, STORED_EPSG).map_err(|_| failed(Why::Service, NOT_WHOLE))?;
        for feature in collection.get("features").and_then(Value::as_array).map_or(&[][..], Vec::as_slice) {
            if cancelled() {
                return Err(stop());
            }
            let shapes = feature.get("geometry").and_then(polygons_of).filter(|p| !p.is_empty()).ok_or_else(|| failed(Why::Data, UNREADABLE))?;
            let mut polygons = Vec::new();
            for rings in shapes {
                let mut carried = Vec::with_capacity(rings.len());
                for ring in rings {
                    let ring = if source == STORED_EPSG { ring } else { carry.transform_all(&ring).map_err(|_| failed(Why::Data, UNREADABLE))? };
                    if ring.iter().flatten().any(|v| !v.is_finite()) {
                        return Err(failed(Why::Data, UNREADABLE));
                    }
                    carried.push(ring);
                }
                polygons.extend(scope.cut_polygon(&carried).map_err(|_| failed(Why::Data, UNREADABLE))?);
            }
            if polygons.is_empty() {
                continue;
            }
            let code = feature.get("properties").and_then(|p| p.get(TERRAIN_FIELD));
            let name = code.and_then(Value::as_str).unwrap_or_default();
            // `area($geometry) < 40000` writes nothing (`:176`); here the area is that of the stored shape.
            let label = if polygons_area(&polygons).unwrap_or(0.0) < LABEL_MIN_AREA { "" } else { label_text(name) };
            let cells = vec![cell(code, ColumnKind::Text), Cell::Text(hex(terrain_colour(name))), Cell::Text(label.to_owned())];
            rows.push(Row { shapes: Shapes::Polygons(polygons), cells });
        }
    }
    if rows.is_empty() {
        return Err(failed(Why::NoData, NO_DATA));
    }
    let expected = rows.len() as u64;
    let layout = TableLayout { table: TABLE, epsg: STORED_EPSG, kind: ShapeKind::Polygons, columns: &[(TERRAIN_FIELD, ColumnKind::Text), (COLOUR_COLUMN, ColumnKind::Text), (LABEL_COLUMN, ColumnKind::Text)] };
    let stored = write_new_table(gpkg, &layout, &mut rows.into_iter()).map_err(|_| failed(Why::Storage, NOT_SAVED))?;
    if stored != expected {
        return Err(failed(Why::Storage, NOT_VERIFIED));
    }
    // A stop during the write counts as well (`PreparedReferenceMap.cpp:142`).
    if cancelled() {
        return Err(stop());
    }
    Ok(stored)
}
