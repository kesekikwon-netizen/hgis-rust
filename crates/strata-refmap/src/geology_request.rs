//! The requests the geology map is fetched with: address, layers and the exact form of the two calls
//! (C++ `GeologyMapService.cpp:55-58`, `:185-208`, `:226-244`, `:638-657`).

use crate::tiled_fetch::Range;

#[cfg(test)]
#[path = "geology_request_tests.rs"]
mod tests;

/// The public map service of KIGAM (no key, no account; `GeologyMapService.cpp:55`).
pub const SERVICE: &str = "https://data.kigam.re.kr/geoserver/ows";
/// Rock units of the mainland sheets (`:56`) and of Jeju (`:57`).
pub const MAINLAND_LAYER: &str = "geoOpen:l_50k_geology_litho_latest";
pub const JEJU_LAYER: &str = "geoOpen:l_jeju_50k_geology_litho_view";
/// The official coloured map as a picture (`:58`).
pub const PICTURE_LAYER: &str = "geoOpen:L_50K_Geology_Map";
/// What the C++ names itself as to this service (`:205`, `:244`).
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3";
/// More features than this in one answer means the answer was cut (`kMaxFeatures`, `:229`).
pub const MAX_FEATURES: usize = 100_000;

/// Where the mainland layer has data, longitude and latitude (`lithoWfsCoversWgs84`, `:640-642`: the
/// layer's own bounding box in the service description of 2026-08-31).
const MAINLAND: Range = [124.609_993_658_599_36, 33.968_707_219_291_59, 129.584_727_630_441_48, 38.605_074_080_919_25];
/// Where the Jeju layer has data (`jejuLithoWfsCoversWgs84`, `:649`).
const JEJU: Range = [126.10, 33.10, 127.00, 33.62];

fn usable(range: &Range) -> bool {
    range.iter().all(|v| v.is_finite()) && range[0] <= range[2] && range[1] <= range[3]
}

/// Boxes that share at least an edge point (`QgsRectangle::intersects`).
fn intersects(a: &Range, b: &Range) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

/// The layer that holds rock units for this box of longitude and latitude: the mainland layer where its
/// data reaches, else the Jeju layer, else none (`lithoTypeNameForWgs84`, `:652-656`).
#[must_use]
pub fn layer_for(range: &Range) -> Option<&'static str> {
    if !usable(range) {
        return None;
    }
    if intersects(&MAINLAND, range) {
        Some(MAINLAND_LAYER)
    } else if intersects(&JEJU, range) {
        Some(JEJU_LAYER)
    } else {
        None
    }
}

/// The request for the rock units of one box (`fetchLithoPiece`, `:230-241`): WFS 2.0.0, coordinates
/// asked for in EPSG:5186, the box given as latitude then longitude, as the URN form of EPSG:4326 orders
/// them, with eight decimals.
#[must_use]
pub fn features_url(layer: &str, range: &Range) -> String {
    format!(
        "{SERVICE}?service=WFS&version=2.0.0&request=GetFeature&typeNames={layer}&outputFormat=application/json&srsName=EPSG:5186&count={MAX_FEATURES}&bbox={:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326",
        range[1], range[0], range[3], range[2]
    )
}

/// Width and height of the picture the colours are read from: 1400 pixels on the longer side of the box,
/// the other side in proportion, never under 64 (`sampleOfficialColors`, `:187-193`).
#[must_use]
pub fn picture_size(range: &Range) -> (u32, u32) {
    let (w, h) = ((range[2] - range[0]).max(1e-9), (range[3] - range[1]).max(1e-9));
    // Between 64 and 1400 by construction.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let other = |ratio: f64| (1400.0 * ratio).round().clamp(64.0, 1400.0) as u32;
    if w >= h {
        (1400, other(h / w))
    } else {
        (other(w / h), 1400)
    }
}

/// The request for that picture (`:195-203`): WMS 1.1.1, EPSG:4326 with the box as longitude then
/// latitude (the order of version 1.1.1), seven decimals, transparent PNG.
#[must_use]
pub fn picture_url(range: &Range) -> String {
    let (width, height) = picture_size(range);
    format!(
        "{SERVICE}?service=WMS&version=1.1.1&request=GetMap&layers={PICTURE_LAYER}&styles=&srs=EPSG:4326&bbox={:.7},{:.7},{:.7},{:.7}&width={width}&height={height}&format=image/png&transparent=true",
        range[0], range[1], range[2], range[3]
    )
}

/// The pixel of that picture where a point of longitude and latitude lies (`:214-215`).
#[must_use]
pub fn picture_pixel(range: &Range, point: [f64; 2]) -> (i64, i64) {
    let (w, h) = ((range[2] - range[0]).max(1e-9), (range[3] - range[1]).max(1e-9));
    let (width, height) = picture_size(range);
    // A point outside the box gives a pixel outside the picture; the caller skips those.
    #[allow(clippy::cast_possible_truncation)]
    let at = |part: f64, side: u32| (part * f64::from(side - 1)).round() as i64;
    (at((point[0] - range[0]) / w, width), at((range[3] - point[1]) / h, height))
}
