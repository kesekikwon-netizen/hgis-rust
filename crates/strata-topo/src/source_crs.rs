//! Which Korean belt a received 1:25,000 sheet is drawn in.
//!
//! A sheet says GRS80 and nothing more: the belts of one zone differ by their false northing (500, 550 or
//! 600 km), so a wrong choice puts the sheet 50 or 100 km away. The C++ compares where the sheet's own
//! layers lie with where the sheet officially is, for every belt of the zone, and takes a belt only when
//! exactly one fits (`target/cpp-ref/src/core/TopographicSourceCrs.cpp`, solver version 4; read in full on
//! 2026-11-15). When none fits and the official position is known, the sheet's own frame (the closed
//! polyline on CAD layer H0017334) decides: in the right belt it is a square of 7.5 minutes to a quarter
//! of a metre. Nothing is ever moved to make it fit.
//!
//! Left out: a CRS declared by the file itself (`:240-246`). Sheets arrive as DXF, which declares none.

use strata_engine::{area_of_use, Transformer};
use strata_net::ngii_topographic::{TopographicSheets, TopographicZone};

use crate::codes::Category;

#[path = "source_crs_frame.rs"]
mod frame;

#[cfg(test)]
#[path = "source_crs_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "source_crs_tests_b.rs"]
mod tests_b;

/// `kSolverVersion`, `TopographicSourceCrs.h:6`.
pub const SOLVER_VERSION: u32 = 4;
/// The CAD layer of a sheet's frame (도곽; `:57`).
pub const FRAME_LAYER: &str = "H0017334";

/// Layers may reach this far past the sheet (`BoundaryMarginMetres`, `:19`).
const MARGIN_M: f64 = 100.0;
/// Korea 2000 in longitude and latitude, and the grid the portal's index is in.
const GEOGRAPHIC: u32 = 4737;
const INVENTORY: u32 = 5179;

/// `:185`.
pub const NOT_GRS80: &str = "공식 도엽의 GRS80 측지계가 확인되지 않아 좌표계를 자동 지정하지 않았습니다.";
/// `:190`.
pub const BAD_SHEET_NUMBER: &str = "공식 1:25,000 도엽번호가 유효하지 않아 좌표계를 확인할 수 없습니다.";
/// `:205`.
pub const BAD_OFFICIAL_BOUNDS: &str = "실제 공식 도곽의 범위 또는 크기가 유효하지 않아 자동 적재하지 않았습니다.";
/// `:224`.
pub const OFFICIAL_TOO_FAR: &str = "실제 공식 도곽과 도엽번호의 위치가 지나치게 달라 자료 연결을 확인해야 합니다.";
/// `:249`.
pub const BAD_LAYER_EXTENT: &str = "수치지형도 기본 레이어의 좌표 범위가 유효하지 않습니다.";
/// `:261`.
pub const NO_PRIMARY_LAYER: &str = "등고선·표고점·도로·건물·수계·행정경계·지명 중 좌표 범위가 확인되는 레이어가 없습니다.";
/// `:347`.
pub const NO_BELT_FITS: &str = "원본 좌표가 공식 도엽 위치와 일치하지 않습니다. 잘못된 좌표계로 자동 적재하지 않았습니다.";
/// `:348`.
pub const SEVERAL_BELTS_FIT: &str = "여러 좌표계가 공식 도엽 위치와 일치하여 하나로 판별할 수 없습니다. 자동 적재하지 않았습니다.";

/// One layer of a sheet as the solver sees it: what it holds and the box of its shapes in the sheet's own
/// coordinates (`[min x, min y, max x, max y]`), when it has shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerBox {
    pub category: Category,
    pub extent: Option<[f64; 4]>,
}

/// The belt a sheet was found to be drawn in.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub epsg: u32,
    /// What the choice rests on, for the record kept beside the sheet.
    pub evidence: String,
    /// Said to the user when the sheet's frame, not the official index, placed the sheet.
    pub warning: Option<String>,
    /// The layers (by their place in the list) that do not lie inside the sheet: they are not put on the map.
    pub unverified: Vec<usize>,
}

/// Finds the belt. `projection` is the datum the portal names for the sheet; `frame` holds the lines of
/// CAD layer H0017334; `official_5179` is the sheet's rectangle in the portal's index (EPSG:5179), when the
/// download is tied to an index record.
///
/// # Errors
/// The C++ sentence that says why the sheet is not placed automatically.
pub fn resolve(sheet_no: &str, projection: &str, layers: &[LayerBox], frame: &[Vec<[f64; 2]>], official_5179: Option<[f64; 4]>) -> Result<Resolved, String> {
    if !projection.trim().eq_ignore_ascii_case("GRS80") {
        return Err(NOT_GRS80.to_owned());
    }
    let sheet = TopographicSheets::resolve(sheet_no).ok_or_else(|| BAD_SHEET_NUMBER.to_owned())?;
    let place = match official_5179 {
        None => Place { epsg: GEOGRAPHIC, extent: sheet.extent(), centre: centre_of(&sheet.extent()), zone: sheet.zone, evidence: "도엽번호 명목 격자".to_owned() },
        Some(bounds) => official_place(&sheet.extent(), bounds)?,
    };

    // The layers that say where the sheet lies (`:236-262`).
    let mut placing = Vec::new();
    for layer in layers.iter().filter(|layer| layer.category.is_primary()) {
        let Some(extent) = layer.extent else {
            continue;
        };
        if !is_box(&extent) {
            return Err(BAD_LAYER_EXTENT.to_owned());
        }
        placing.push(extent);
    }
    let Some(combined) = placing.iter().copied().reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]) else {
        return Err(NO_PRIMARY_LAYER.to_owned());
    };
    let mut evidence = format!(
        "공식 도엽 {sheet_no} · {} · 기본 레이어 {}개 · 원본 동거리 {:.3}~{:.3}m, 북거리 {:.3}~{:.3}m · 도곽 여유 {MARGIN_M:.0}m · {}",
        place.zone.name_ko(),
        placing.len(),
        combined[0],
        combined[2],
        combined[1],
        combined[3],
        place.evidence
    );

    // Every belt of the zone whose area of use holds the sheet's centre (`:270-300`).
    let fits = |allowed: &[f64; 4]| placing.iter().all(|extent| inside(allowed, extent));
    let (mut matched, mut tested) = (Vec::new(), Vec::new());
    for epsg in belts(place.zone) {
        let area = area_of_use(*epsg).map_err(|_| format!("설치된 좌표계 정의 EPSG:{epsg}을 읽지 못해 후보를 안전하게 비교할 수 없습니다."))?;
        if !area.is_some_and(|area| holds(&area, place.centre)) {
            continue;
        }
        tested.push(format!("EPSG:{epsg}"));
        let transform = Transformer::new(place.epsg, *epsg).map_err(|_| format!("도곽 좌표 변환을 준비하지 못했습니다: EPSG:{epsg}"))?;
        let allowed = transform.transform_box(place.extent).map_err(|e| format!("도곽 좌표 변환 실패(EPSG:{epsg}): {}", e.0))?;
        if !is_box(&allowed) {
            return Err(format!("도곽 좌표 변환 결과를 확인하지 못했습니다: EPSG:{epsg}"));
        }
        let allowed = grown(allowed, MARGIN_M);
        if fits(&allowed) {
            matched.push((*epsg, allowed));
        }
    }
    evidence.push_str(&format!(" · 비교 후보: {}", tested.join(", ")));

    // No belt fits the official rectangle: the sheet's own frame is asked (`:302-341`).
    let mut warning = None;
    if let (true, Some(official), Some(points)) = (matched.is_empty(), official_5179, frame::source_frame(frame)) {
        let mut said = None;
        for epsg in belts(place.zone) {
            let Some(found) = frame::match_source_frame(points, *epsg, &official)?.filter(|found| fits(&found.allowed)) else {
                continue;
            };
            matched.push((*epsg, found.allowed));
            said = Some((
                format!(" · 원본 도곽 전체 변 검증: EPSG:{epsg}, 최대 잔차 {:.4}m (한도 0.25m), 공식 색인과 중심 차이 {:.1}m · 좌표 이동 없이 원본 도곽 범위 적용", found.residual, found.displacement),
                format!("도엽 {sheet_no}: 공식 색인과 원본 도곽 위치가 {:.0}m 다릅니다. 원본 도곽의 좌표계를 별도로 검증해 적재했습니다.", found.displacement),
            ));
        }
        if let (1, Some((frame_evidence, frame_warning))) = (matched.len(), said) {
            evidence.push_str(&frame_evidence);
            warning = Some(frame_warning);
        }
    }

    let [(epsg, allowed)] = matched[..] else {
        return Err(if matched.is_empty() { NO_BELT_FITS } else { SEVERAL_BELTS_FIT }.to_owned());
    };
    let unverified: Vec<usize> = layers.iter().enumerate().filter(|(_, layer)| !layer.extent.is_some_and(|extent| is_box(&extent) && inside(&allowed, &extent))).map(|(i, _)| i).collect();
    evidence.push_str(&format!(" · 유일한 도곽 일치 후보 EPSG:{epsg} · 미확인 레이어 {}개. 도곽 범위 대조이며 위성영상 화소 정합 검증은 아닙니다.", unverified.len()));
    Ok(Resolved { epsg, evidence, warning, unverified })
}

/// Where a sheet officially is: a rectangle in some CRS, its centre in longitude and latitude, its zone.
struct Place {
    epsg: u32,
    extent: [f64; 4],
    centre: [f64; 2],
    zone: TopographicZone,
    evidence: String,
}

/// The place the portal's index gives (`:195-234`). The limits are checks that the rectangle belongs to
/// this sheet, not claims about map standards.
fn official_place(nominal: &[f64; 4], bounds: [f64; 4]) -> Result<Place, String> {
    const MAX_CENTRE_DISPLACEMENT_M: f64 = 50_000.0;
    const SHEET_SIDE_M: std::ops::RangeInclusive<f64> = 1_000.0..=30_000.0;
    if !is_box(&bounds) || !SHEET_SIDE_M.contains(&(bounds[2] - bounds[0])) || !SHEET_SIDE_M.contains(&(bounds[3] - bounds[1])) {
        return Err(BAD_OFFICIAL_BOUNDS.to_owned());
    }
    let not_ready = |_| "공식 도곽의 좌표 변환을 준비하지 못했습니다.".to_owned();
    let (to_official, to_geographic) = (Transformer::new(GEOGRAPHIC, INVENTORY).map_err(not_ready)?, Transformer::new(INVENTORY, GEOGRAPHIC).map_err(not_ready)?);
    let failed = |e: strata_engine::EngineError| format!("공식 도곽의 위치 확인 실패: {}", e.0);
    let (by_number, actual) = (to_official.transform(centre_of(nominal)).map_err(failed)?, centre_of(&bounds));
    let centre = to_geographic.transform(actual).map_err(failed)?;
    let displacement = (by_number[0] - actual[0]).hypot(by_number[1] - actual[1]);
    if !displacement.is_finite() || displacement > MAX_CENTRE_DISPLACEMENT_M || !centre.iter().all(|v| v.is_finite()) || !(124.0..132.0).contains(&centre[0]) {
        return Err(OFFICIAL_TOO_FAR.to_owned());
    }
    let zone = match centre[0] {
        lon if lon < 126.0 => TopographicZone::West,
        lon if lon < 128.0 => TopographicZone::Central,
        lon if lon < 130.0 => TopographicZone::East,
        _ => TopographicZone::EastSea,
    };
    Ok(Place { epsg: INVENTORY, extent: bounds, centre, zone, evidence: format!("실제 공식 도곽(EPSG:5179) · 명목 격자 중심 차이 {displacement:.1}m · 자료 연결 점검 한도 50000m") })
}

/// The belts a sheet of a zone may be drawn in (`codes`, `:37-45`): the old ones with a false northing of
/// 500 km (550 km for Jeju) and the ones of 2010 with 600 km.
const fn belts(zone: TopographicZone) -> &'static [u32] {
    match zone {
        TopographicZone::West => &[5180, 5185],
        TopographicZone::Central => &[5181, 5182, 5186],
        TopographicZone::East => &[5183, 5187],
        TopographicZone::EastSea => &[5184, 5188],
    }
}

fn is_box(b: &[f64; 4]) -> bool {
    b.iter().all(|v| v.is_finite()) && b[2] >= b[0] && b[3] >= b[1]
}

fn inside(outer: &[f64; 4], inner: &[f64; 4]) -> bool {
    inner[0] >= outer[0] && inner[1] >= outer[1] && inner[2] <= outer[2] && inner[3] <= outer[3]
}

fn grown(b: [f64; 4], by: f64) -> [f64; 4] {
    [b[0] - by, b[1] - by, b[2] + by, b[3] + by]
}

fn centre_of(b: &[f64; 4]) -> [f64; 2] {
    [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0]
}

fn box_of(points: &[[f64; 2]]) -> [f64; 4] {
    points.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
}

/// Whether an area of use `[west, south, east, north]` holds a point of longitude and latitude, its edge
/// included (`QgsRectangle::contains`).
fn holds(area: &[f64; 4], point: [f64; 2]) -> bool {
    (area[0]..=area[2]).contains(&point[0]) && (area[1]..=area[3]).contains(&point[1])
}
