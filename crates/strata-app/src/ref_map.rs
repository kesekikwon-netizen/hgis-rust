//! Reference maps a ribbon button fetches for the area around the drawn survey area (지질; 토양 and 수계
//! follow): what each button is, the area it asks for, and the maps already kept in the survey folder.
//!
//! C++: `MainWindow::downloadGeologyMap` and `startReferenceDownload`
//! (`target/cpp-ref/src/app/MainWindowOverlay.cpp:405-472`, `:529-640`). There the area is the map view;
//! here it is the survey area grown by the button's radius (owner 2026-11-15, docs/RUST_PLAN.md 2.9).

use std::path::{Path, PathBuf};

use strata_engine::{Scope, Transformer};
use strata_refmap::geology_prepare;
use strata_refmap::transfer::{Download, Prepared, Stopped};
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::ref_vector_data::{Look, PolygonFill};
use crate::survey_area_setup::is_survey_area_key;

#[path = "ref_map_run.rs"]
pub(crate) mod run;
#[cfg(test)]
#[path = "ref_map_tests.rs"]
mod tests;

/// The function of strata-refmap that fetches and stores one kind of map.
type Prepare = fn(&Scope, &Path, Download<'_>, &dyn Fn() -> bool) -> Result<Prepared, Stopped>;

/// One kind of reference map and its button.
pub(crate) struct RefMapKind {
    /// `layer_key` of its legend row.
    pub(crate) key: &'static str,
    /// Legend title, as in the C++ (`GeologyMapService.cpp:59`).
    pub(crate) title: &'static str,
    /// Its name in the bottom gauge.
    pub(crate) job_title: &'static str,
    pub(crate) file_name: &'static str,
    pub(crate) table: &'static str,
    /// The CRS the map is stored in.
    pub(crate) stored_epsg: u32,
    /// How far around the survey area the map reaches, metres.
    pub(crate) radius_m: f64,
    pub(crate) look: Look,
    pub(crate) prepare: Prepare,
    /// `MainWindowDownloads.cpp:93-103`.
    pub(crate) needs_survey: &'static str,
    pub(crate) needs_area: &'static str,
    pub(crate) offline: &'static str,
    /// Gauge line while the map is fetched (`MainWindowDownloads.cpp:22`).
    pub(crate) fetching: &'static str,
    /// `MainWindowOverlay.cpp:672`.
    pub(crate) added: &'static str,
    /// A map kept from an earlier fetch for the same area was put on the map without asking the service.
    pub(crate) kept: &'static str,
    /// `MainWindowOverlay.cpp:635`.
    pub(crate) cancelled: &'static str,
}

/// 지질 — the KIGAM 1:50,000 geology, 10 km around the survey area.
pub(crate) const GEOLOGY: RefMapKind = RefMapKind {
    key: "ref_geology",
    title: "지질도(KIGAM 1:5만)",
    job_title: "지질도",
    file_name: geology_prepare::FILE_NAME,
    table: geology_prepare::TABLE,
    stored_epsg: geology_prepare::STORED_EPSG,
    radius_m: 10_000.0,
    // Fill opacity 220, outline (80, 80, 80, 130) 0.12 mm (`eraFillSymbol`, GeologyMapService.cpp:98-108);
    // grey for a unit without a colour (the 「기타」 category, :382-458).
    look: Look::Polygons { stroke: [80, 80, 80, 130], width_mm: 0.12, fill: PolygonFill::Column { name: geology_prepare::COLOUR_COLUMN, alpha: 220, missing: [200, 200, 200] } },
    prepare: geology_prepare::prepare,
    needs_survey: "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.",
    needs_area: "조사구역을 먼저 그려 주세요. 조사구역에서 반경 10km 안의 지질도를 받습니다.",
    offline: "인터넷 접속이 꺼져 있어 지질도 서버에 연결할 수 없습니다.",
    fetching: "지질도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.",
    added: "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.",
    kept: "보관해 둔 지질도를 올렸습니다. 다시 누르면 숨깁니다.",
    cancelled: "지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다.",
};

/// `SurveyScopeClip.cpp:57`.
pub(crate) const AREA_CRS_UNKNOWN: &str = "조사구역 좌표계를 확인하지 못했습니다.";
/// `MainWindowOverlay.cpp:607`.
pub(crate) const BUSY: &str = "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요.";
/// Beside a kept map: the area it was fetched for.
const SCOPE_NOTE: &str = "ka-hgis-scope.json";

fn polygon_wkt(rings: &[Vec<[f64; 2]>]) -> String {
    let ring = |r: &Vec<[f64; 2]>| format!("({})", r.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", "));
    format!("POLYGON ({})", rings.iter().map(ring).collect::<Vec<_>>().join(", "))
}

/// The drawn survey areas grown by `radius`, in the work CRS: every survey-area shape is carried into the
/// work CRS first, as `SurveyScopeClip::surveyUnion` does (`SurveyScopeClip.cpp:40-70`). `Ok(None)`: no
/// survey area is drawn.
pub(crate) fn survey_scope(app: &StrataAppState, radius: f64) -> Result<Option<Scope>, String> {
    let work_epsg = CrsCode::from(app.work_crs).epsg_number();
    let mut shapes = Vec::new();
    for table in app.survey_tables.iter().filter(|t| is_survey_area_key(&t.table)) {
        let source = u32::try_from(table.srs_id).ok().filter(|s| *s > 0).ok_or_else(|| AREA_CRS_UNKNOWN.to_owned())?;
        let carry = Transformer::new(source, work_epsg).map_err(|_| AREA_CRS_UNKNOWN.to_owned())?;
        for feature in &table.layer.features {
            for part in wkt_to_parts(&feature.wkt_geom).unwrap_or_default() {
                let WkbGeometry::Polygon(rings) = part else {
                    continue;
                };
                let rings = if source == work_epsg { rings } else { rings.iter().map(|r| carry.transform_all(r)).collect::<Result<Vec<_>, _>>().map_err(|_| AREA_CRS_UNKNOWN.to_owned())? };
                shapes.push(polygon_wkt(&rings));
            }
        }
    }
    Scope::around(&shapes, radius).map_err(|e| e.0)
}

/// What is written beside a stored map: the radius and the box (stored CRS) of the scope it was fetched for.
pub(crate) fn scope_note(radius: f64, bbox: [f64; 4]) -> String {
    serde_json::json!({"radius_m": radius, "bbox": bbox}).to_string()
}

pub(crate) fn write_scope_note(folder: &Path, radius: f64, bbox: [f64; 4]) -> std::io::Result<()> {
    std::fs::write(folder.join(SCOPE_NOTE), scope_note(radius, bbox))
}

/// The newest map of this kind in the survey folder that was fetched for this very area (same radius, box
/// within a metre). A map fetched for another area is not it; a map without the note (one the C++ app
/// stored, whose area is not known and which has no colour column) is not it either.
pub(crate) fn kept_map(survey_folder: &Path, kind: &RefMapKind, bbox: [f64; 4]) -> Option<PathBuf> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(survey_folder).ok()?.flatten() {
        let folder = entry.path();
        let is_ours = folder.is_dir() && folder.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("ka-hgis-reference-"));
        let file = folder.join(kind.file_name);
        if !is_ours || !file.is_file() {
            continue;
        }
        let Some(note) = std::fs::read(folder.join(SCOPE_NOTE)).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else {
            continue;
        };
        let same_radius = note["radius_m"].as_f64().is_some_and(|r| (r - kind.radius_m).abs() < 1e-6);
        let same_box = note["bbox"].as_array().is_some_and(|b| b.len() == 4 && b.iter().zip(bbox).all(|(have, want)| have.as_f64().is_some_and(|v| (v - want).abs() < 1.0)));
        let written = std::fs::metadata(&file).and_then(|m| m.modified()).ok();
        if let (true, true, Some(written)) = (same_radius, same_box, written) {
            if newest.as_ref().is_none_or(|(known, _)| written > *known) {
                newest = Some((written, file));
            }
        }
    }
    newest.map(|(_, file)| file)
}
