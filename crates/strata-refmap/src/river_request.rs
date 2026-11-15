//! The requests the river network is fetched with (C++ `RiverMapService.cpp:39-43`, `:82-129`).

use crate::tiled_fetch::Range;

#[cfg(test)]
#[path = "river_request_tests.rs"]
mod tests;

/// The WFS of VWorld (`kWfsUrl`, `:39`). It asks for the user's own key.
pub const SERVICE: &str = "https://api.vworld.kr/req/wfs";
/// The river network: national and local rivers as polygons (`kTypeName`, `:40`).
pub const LAYER: &str = "lt_c_wkmstrm";
/// The program says what it is (docs/RUST_PLAN.md 2.8); the C++ puts a browser's name before it (`:126`).
pub const USER_AGENT: &str = "ka-hgis/0.3";
/// The address the user's key is registered for: VWorld checks it against this header, and refuses the
/// same key when `DOMAIN=` is sent instead (`:112-115`, `:128`).
pub const REFERER: &str = "https://localhost";
/// `MAXFEATURES` of one answer and how many answers one box is asked for (`kPageSize`, `kMaxPages`,
/// `:99-100`).
pub const PAGE_SIZE: usize = 1000;
pub const MAX_PAGES: usize = 10;

/// The request for one page of one box of longitude and latitude (`:105-123`). `key` is the user's VWorld
/// key: the address must never be logged or shown.
#[must_use]
pub fn page_url(range: &Range, page: usize, key: &str) -> String {
    // The box as latitude then longitude, as the URN form of EPSG:4326 orders them, with eight decimals.
    // No `DOMAIN=`: VWorld refuses the same key with it (`:112-115`).
    format!(
        "{SERVICE}?SERVICE=WFS&REQUEST=GetFeature&VERSION=2.0.0&TYPENAME={LAYER}&OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES={PAGE_SIZE}&STARTINDEX={}&BBOX={:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326&KEY={key}",
        page * PAGE_SIZE,
        range[1],
        range[0],
        range[3],
        range[2]
    )
}

/// What VWorld says when it answers with an XML error instead of data: the text of the first
/// `<ServiceException …>` (`vworldExceptionText`, `:82-88`), trimmed.
#[must_use]
pub fn service_exception(body: &[u8]) -> Option<String> {
    // `<ServiceException[^>]*>([^<]+)</ServiceException>`
    const OPEN: &str = "<ServiceException";
    const CLOSE: &str = "</ServiceException>";
    let text = String::from_utf8_lossy(body);
    let mut rest = text.as_ref();
    while let Some(found) = rest.find(OPEN) {
        rest = &rest[found + OPEN.len()..];
        // The tag ends at the next `>`; `<ServiceExceptionReport …>` is such a tag too, with a `<` after it.
        let Some(end_of_tag) = rest.find('>') else {
            break;
        };
        if rest[..end_of_tag].contains('<') {
            continue;
        }
        let inside = &rest[end_of_tag + 1..];
        let reason = &inside[..inside.find('<').unwrap_or(inside.len())];
        if !reason.is_empty() && inside[reason.len()..].starts_with(CLOSE) {
            return Some(reason.trim().to_owned());
        }
    }
    None
}
