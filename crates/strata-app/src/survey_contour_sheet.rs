//! CSV/TXT preview for the survey-contour dialog.
//!
//! The sentence is `target/cpp-ref/src/app/KaSurveyContourDialog.cpp:211-232`.
//! DXF and Excel stay unread until order W-29, which is the GDAL reader
//! (`SurveyPointReader::read`) plus the TIN and `contours.gpkg` write.

use std::path::Path;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

#[path = "survey_contour_parse.rs"]
mod parse;

use parse::{read_table, Elev};

/// `SurveyPointReader.cpp:114`.
const OPEN_FAIL: &str = "파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요.";
/// 이 앱의 말. C++ opens these with GDAL; that reader is order W-29.
const DEFERRED_BINARY: &str = "엑셀은 아직 읽지 않습니다 (지시 W-29).";

#[derive(Debug, Clone)]
struct Sheet {
    fatal: Option<String>,
    points: Vec<Elev>,
}

struct Slot {
    key: String,
    sheet: Sheet,
}

static CACHE: Mutex<Option<Slot>> = Mutex::new(None);

/// Point count and elevation range for `path`, or a fatal line from the C++ reader.
pub(crate) fn contour_preview(path: &str, interval_m: f64, exclude_suspicious: bool) -> String {
    let sheet = sheet_for(path);
    if let Some(fatal) = sheet.fatal {
        return fatal;
    }
    let mut count = 0usize;
    let mut min_cm = 0i32;
    let mut max_cm = 0i32;
    for point in sheet.points {
        if exclude_suspicious && point.suspicious {
            continue;
        }
        let cm = meters_to_cm(point.z);
        if count == 0 {
            min_cm = cm;
            max_cm = cm;
        } else {
            min_cm = min_cm.min(cm);
            max_cm = max_cm.max(cm);
        }
        count += 1;
    }
    // `SurveyContourMath.h:9-10` and `KaSurveyContourDialog.cpp:224-232`.
    let interval_cm = meters_to_cm(interval_m);
    let lines = if interval_cm > 0 {
        (max_cm - min_cm) / interval_cm
    } else {
        0
    };
    let mut text = format!(
        "점 {count}개, 표고 {}\u{2013}{} m. 등고선 약 {lines}줄.",
        format_cm(min_cm),
        format_cm(max_cm)
    );
    if lines > 500 {
        text.push('\n');
        text.push_str("줄이 많습니다. 간격을 넓히세요.");
    }
    text
}

fn sheet_for(path: &str) -> Sheet {
    let key = cache_key(path);
    let mut guard = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(slot) = guard.as_ref() {
        if slot.key == key {
            return slot.sheet.clone();
        }
    }
    let sheet = read_sheet(Path::new(path));
    *guard = Some(Slot {
        key,
        sheet: sheet.clone(),
    });
    sheet
}

fn cache_key(path: &str) -> String {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let stamp = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            format!("{path}\0{}\0{stamp}", meta.len())
        }
        Err(_) => format!("missing\0{path}"),
    }
}

fn read_sheet(path: &Path) -> Sheet {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Sheet {
                fatal: Some(OPEN_FAIL.to_owned()),
                points: Vec::new(),
            }
        }
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("dxf") {
        return dxf_sheet(path);
    }
    // gdal-src 0.3.0 build.rs never turns on the XLSX driver (it needs expat and is not a cargo feature).
    if ext.eq_ignore_ascii_case("xlsx") || ext.eq_ignore_ascii_case("xls") {
        return Sheet {
            fatal: Some(DEFERRED_BINARY.to_owned()),
            points: Vec::new(),
        };
    }
    match read_table(&bytes) {
        Ok(points) => Sheet {
            fatal: None,
            points,
        },
        Err(text) => Sheet {
            fatal: Some(text.to_owned()),
            points: Vec::new(),
        },
    }
}

fn dxf_sheet(path: &Path) -> Sheet {
    match strata_engine::read_dxf_survey_points(path) {
        Ok(points) => Sheet {
            fatal: None,
            points: points
                .into_iter()
                .map(|p| Elev {
                    x: p[0],
                    y: p[1],
                    z: p[2],
                    suspicious: p[2] == 0.0 || p[2].abs() >= 999.0,
                })
                .collect(),
        },
        Err(_) => Sheet {
            fatal: Some(OPEN_FAIL.to_owned()),
            points: Vec::new(),
        },
    }
}

/// Points the preview counted, for the package writer. `Err` is the preview's fatal line.
pub(crate) fn contour_xyz(path: &str, exclude_suspicious: bool) -> Result<Vec<[f64; 3]>, String> {
    let sheet = sheet_for(path);
    if let Some(fatal) = sheet.fatal {
        return Err(fatal);
    }
    Ok(sheet
        .points
        .into_iter()
        .filter(|point| !(exclude_suspicious && point.suspicious))
        .map(|point| [point.x, point.y, point.z])
        .collect())
}

/// `qRound(meters * 100)` (`SurveyContourMath.h:10`).
fn meters_to_cm(meters: f64) -> i32 {
    let rounded = (meters * 100.0).round();
    if rounded >= f64::from(i32::MAX) {
        i32::MAX
    } else if rounded <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// One decimal, half away from zero, the same direction as `qRound`.
/// Rust's default format prints 15.25 as 15.2.
pub(super) fn format_cm(cm: i32) -> String {
    let sign = if cm < 0 { "-" } else { "" };
    let abs = cm.unsigned_abs();
    let tenths = (abs + 5) / 10;
    let whole = tenths / 10;
    let frac = tenths % 10;
    format!("{sign}{whole}.{frac}")
}

#[cfg(test)]
#[path = "survey_contour_sheet_tests.rs"]
mod tests;
