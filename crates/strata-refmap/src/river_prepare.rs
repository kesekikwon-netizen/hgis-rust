//! The river network of VWorld for the area around a survey: fetched page by page, checked, carried into
//! the stored CRS, cut to the area and stored with the water colour of each river (C++
//! `RiverMapService::prepare`, `target/cpp-ref/src/core/RiverMapService.cpp:90-185`, `:274-372`). Two things
//! differ from the C++ by the owner's decision (docs/RUST_PLAN.md 2.9, 2026-11-15): the box asked for is
//! that of the scope around the survey area, not the map view grown to 160 km, and the rivers are cut to
//! that scope.

use std::path::Path;

use serde_json::Value;
use strata_engine::{write_new_table, Cell, ColumnKind, Row, Scope, ShapeKind, Shapes, TableLayout, Transformer};

use crate::collection::{complete_feature_collection, feature_collection};
use crate::geojson::{cell, columns_of, declared_epsg, polygons_of};
use crate::geology_colour::hex;
use crate::river_level::level_of;
use crate::river_request::{page_url, service_exception, MAX_PAGES, PAGE_SIZE, REFERER, USER_AGENT};
use crate::tiled_fetch::{fetch, Options, Outcome, Piece, Range};
use crate::transfer::{new_storage, Download, Prepared, Request, Stopped, TransferFailure, Why, CANCELLED, NOT_REACHED, NOT_SAVED, NOT_VERIFIED, TOO_SLOW};

#[cfg(test)]
#[path = "river_prepare_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "river_prepare_tests_b.rs"]
mod tests_b;

/// Table, file and CRS of the stored map (`:280`, `MainWindowOverlay.cpp:594`, `:322`).
pub const TABLE: &str = "river_map";
pub const FILE_NAME: &str = "수계도_VWorld.gpkg";
pub const STORED_EPSG: u32 = 5186;
/// The river's name and class as the service names them (`kNameField`, `kLevelField`, `:42-43`).
pub const NAME_COLUMN: &str = "riv_nm";
pub const LEVEL_COLUMN: &str = "riv_level";
/// The fill and the outline the river is drawn with, `#RRGGBB`. The C++ keeps them in the project's layer
/// style (`applyRiverStyle`, `:189-222`); here they are kept in the file, so that the file alone can be
/// drawn.
pub const COLOUR_COLUMN: &str = "ka_color";
pub const LINE_COLUMN: &str = "ka_line";

/// `maxSpanMeters`, `RiverMapService.h:43`.
const MAX_SPAN: f64 = 160_000.0;

/// `:283`.
pub const NO_KEY: &str = "VWorld 인증키가 없습니다. 지도 탭의 배경지도 설정에서 키를 먼저 등록하세요.";
/// `:290-293` with `maxSpanMeters` of 160 km (`RiverMapService.h:43`).
pub const TOO_WIDE: &str = "범위가 너무 넓습니다. 지도를 조사구역(한 변 160km 이하)으로 확대한 뒤 다시 내려받으세요.";
/// `:301`.
pub const NO_TRANSFORM: &str = "좌표 변환에 실패했습니다.";
/// `:137-138`.
pub const NOT_GEOJSON: &str = "서버가 GeoJSON 대신 다른 응답을 보냈습니다. VWorld 인증키를 확인하세요.";
/// `:141`.
pub const KEY_REFUSED: &str = "VWorld 인증키가 거부되었습니다. 배경지도 설정에서 키를 확인한 뒤 다시 내려받으세요.";
/// `:142`.
pub const NOT_SENT: &str = "VWorld가 하천 지도를 보내지 못했습니다. 인터넷 연결을 확인한 뒤 잠시 후 다시 내려받으세요.";
/// `:157`.
pub const TOO_MANY: &str = "하천 데이터가 한 번에 받을 수 있는 양을 넘었습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다.";
/// `:311`.
pub const NOT_FETCHED: &str = "수계도를 내려받지 못했습니다.";
/// `:319-320`.
pub const NO_DATA_HERE: &str = "이 범위에는 하천망 데이터가 없습니다. (국가·지방하천이 없는 지역입니다)";
/// `:350`.
pub const NOT_CARRIED: &str = "하천 좌표를 변환하지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.";
/// `:357`.
pub const UNREADABLE: &str = "하천 지도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다.";
/// `:365`.
pub const NO_DATA: &str = "이 범위에는 하천망 데이터가 없습니다.";

/// Fetches the rivers around the survey with the user's VWorld `key` and stores them beside `requested`
/// (the path the map is known by; the file itself is written into a new folder next to it, see
/// `new_storage`). `scope` is in EPSG:5186. `download` sends the requests; `cancelled` is asked between
/// the steps. The key goes into the request addresses only: no sentence of this function holds it.
///
/// # Errors
/// `Stopped::Cancelled` when the user stopped it, `Stopped::Failed` with the C++ sentence otherwise. Nothing
/// is left on disk in either case.
pub fn prepare(scope: &Scope, requested: &Path, key: &str, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<Prepared, Stopped> {
    if cancelled() {
        return Err(Stopped::Cancelled(CANCELLED.to_owned()));
    }
    let key = key.trim();
    if key.is_empty() {
        return Err(failed(Why::NoKey, NO_KEY));
    }
    let bbox = scope.bbox().map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    if bbox.iter().any(|v| !v.is_finite()) || bbox[2] <= bbox[0] || bbox[3] <= bbox[1] || bbox[2] - bbox[0] > MAX_SPAN || bbox[3] - bbox[1] > MAX_SPAN {
        return Err(failed(Why::TooWide, TOO_WIDE));
    }
    let range = Transformer::new(STORED_EPSG, 4326).and_then(|to_lonlat| to_lonlat.transform_box(bbox)).map_err(|_| failed(Why::Data, NO_TRANSFORM))?;
    let (folder, gpkg) = new_storage(requested)?;
    match build(scope, &range, key, &gpkg, download, cancelled) {
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

/// The rivers of one box, page after page (`fetchRiverPiece`, `:90-171`).
fn fetch_piece(range: &Range, key: &str, download: Download<'_>) -> Piece {
    let piece = |outcome, error: &str| Piece { outcome, body: Vec::new(), error: error.to_owned() };
    let mut first: Option<serde_json::Map<String, Value>> = None;
    let mut all = Vec::new();
    for page in 0..MAX_PAGES {
        let body = match download(&Request::large(page_url(range, page, key), USER_AGENT).with_referer(REFERER)) {
            Ok(body) => body,
            Err(TransferFailure::Cancelled) => return piece(Outcome::Cancelled, ""),
            Err(TransferFailure::Timeout) => return piece(Outcome::Slow, TOO_SLOW),
            Err(TransferFailure::Network) => return piece(Outcome::Transient, NOT_REACHED),
        };
        // VWorld answers 200 with an XML error (`:131-143`). What it says is not shown: it may name the key.
        if !body.trim_ascii().starts_with(b"{") {
            return piece(
                Outcome::Fatal,
                match service_exception(&body) {
                    None => NOT_GEOJSON,
                    Some(reason) if reason.to_ascii_uppercase().contains("KEY") => KEY_REFUSED,
                    Some(_) => NOT_SENT,
                },
            );
        }
        let mut collection = match feature_collection(&body) {
            Ok(collection) => collection,
            Err(error) => return piece(Outcome::Fatal, &error),
        };
        let Some(Value::Array(features)) = collection.remove("features") else {
            return piece(Outcome::Fatal, crate::collection::NOT_MAP_DATA);
        };
        let full = features.len() >= PAGE_SIZE;
        all.extend(features);
        first.get_or_insert(collection);
        if !full {
            break;
        }
        if page + 1 == MAX_PAGES {
            // A smaller box needs fewer pages.
            return piece(Outcome::TooLarge, TOO_MANY);
        }
    }
    let mut joined = first.unwrap_or_default();
    joined.insert("features".to_owned(), Value::Array(all));
    let body = Value::Object(joined).to_string().into_bytes();
    match complete_feature_collection(&body) {
        Ok(_) => Piece { outcome: Outcome::Complete, body, error: String::new() },
        Err(error) => piece(Outcome::TooLarge, &error),
    }
}

fn build(scope: &Scope, range: &Range, key: &str, gpkg: &Path, download: Download<'_>, cancelled: &dyn Fn() -> bool) -> Result<u64, Stopped> {
    let stop = || Stopped::Cancelled(CANCELLED.to_owned());
    let fetched = fetch(range, &mut |piece| fetch_piece(&piece, key, download), cancelled, Options::default());
    if fetched.cancelled || cancelled() {
        return Err(stop());
    }
    if !fetched.ok {
        let why = if fetched.error == KEY_REFUSED { Why::KeyRefused } else { Why::Service };
        return Err(Stopped::Failed(why, if fetched.error.is_empty() { NOT_FETCHED.to_owned() } else { format!("VWorld 서버 연결 실패: {}", fetched.error) }));
    }
    let collection = feature_collection(&fetched.body).map_err(|text| failed(Why::Service, &text))?;
    let features = collection.get("features").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    if features.is_empty() {
        return Err(failed(Why::NoData, NO_DATA_HERE));
    }
    // The request asks for longitude and latitude (`SRSNAME=EPSG:4326`); an answer that names another
    // system is carried from that one.
    let source = declared_epsg(&collection).ok_or_else(|| failed(Why::Data, NOT_CARRIED))?;
    let carry = Transformer::new(source, STORED_EPSG).map_err(|_| failed(Why::Data, NOT_CARRIED))?;
    let columns: Vec<(String, ColumnKind)> = columns_of(features).into_iter().filter(|(name, _)| name != COLOUR_COLUMN && name != LINE_COLUMN).collect();

    let mut rows = Vec::new();
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
        let level = level_of(properties.and_then(|p| p.get(LEVEL_COLUMN)).and_then(Value::as_str).unwrap_or_default());
        let mut cells: Vec<Cell> = columns.iter().map(|(name, kind)| cell(properties.and_then(|p| p.get(name)), *kind)).collect();
        cells.extend([Cell::Text(hex(level.fill)), Cell::Text(hex(level.line))]);
        rows.push(Row { shapes: Shapes::Polygons(polygons), cells });
    }
    if rows.is_empty() {
        return Err(failed(Why::NoData, NO_DATA));
    }
    let mut layout: Vec<(&str, ColumnKind)> = columns.iter().map(|(name, kind)| (name.as_str(), *kind)).collect();
    layout.extend([(COLOUR_COLUMN, ColumnKind::Text), (LINE_COLUMN, ColumnKind::Text)]);
    let expected = rows.len() as u64;
    let table = TableLayout { table: TABLE, epsg: STORED_EPSG, kind: ShapeKind::Polygons, columns: &layout };
    let stored = write_new_table(gpkg, &table, &mut rows.into_iter()).map_err(|_| failed(Why::Storage, NOT_SAVED))?;
    if stored != expected {
        return Err(failed(Why::Storage, NOT_VERIFIED));
    }
    // A stop during the write counts as well (`PreparedReferenceMap.cpp:142`).
    if cancelled() {
        return Err(stop());
    }
    Ok(stored)
}
