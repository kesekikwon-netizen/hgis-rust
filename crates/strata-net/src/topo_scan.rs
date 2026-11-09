//! SHP·DXF 읽기와 폴더 스캔 (C++ `TopographicCatalog.cpp:181-327`).
//!
//! `topographic_catalog` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::BufRead;
use super::topo_codebook::suffix_of;
use super::topo_codebook::companion;
use super::topo_codebook::prj_crs_auth_id;

use super::*;

/// `.shp` 머리말의 경계 상자(바이트 36-68). C++ 은 OGR `GetExtent` 로 같은 값을 읽는다.
fn read_shp_extent(path: &Path) -> Option<Extent> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut header = [0_u8; 100];
    file.read_exact(&mut header).ok()?;
    if u32::from_be_bytes([header[0], header[1], header[2], header[3]]) != 9994 {
        return None;
    }
    let number = |offset: usize| {
        f64::from_le_bytes([
            header[offset],
            header[offset + 1],
            header[offset + 2],
            header[offset + 3],
            header[offset + 4],
            header[offset + 5],
            header[offset + 6],
            header[offset + 7],
        ])
    };
    let extent = Extent {
        min_x: number(36),
        min_y: number(44),
        max_x: number(52),
        max_y: number(60),
    };
    extent.is_finite().then_some(extent)
}

/// DXF 도형 갈래 — C++ `TopographicCatalog.cpp:104-109` `geometryName` 이 돌려주는
/// OGR 표시 문자열과 같은 이름을 쓴다(`LineString` / `Point` / `Polygon`).
fn geometry_family(entity: &str) -> Option<&'static str> {
    match entity {
        "LINE" | "LWPOLYLINE" | "POLYLINE" | "ARC" | "CIRCLE" | "SPLINE" | "ELLIPSE" | "MLINE"
        | "LEADER" | "DIMENSION" => Some("LineString"),
        "POINT" | "TEXT" | "MTEXT" | "INSERT" | "ATTRIB" => Some("Point"),
        "HATCH" | "SOLID" | "TRACE" | "3DFACE" => Some("Polygon"),
        _ => None,
    }
}

/// DXF 한 파일을 CAD 레이어·도형 갈래별로 나눈다 — C++ `TopographicCatalog.cpp:181-209`.
///
/// C++ 은 OGR 로 `entities` 레이어를 훑어 `Layer`(CAD 레이어)와 도형 종류를 짝지어
/// 레코드를 만든다. 여기서는 같은 일을 DXF 그룹 코드로 한다:
/// 코드 `0` = 도형 종류, `8` = CAD 레이어, `10`/`20` = 좌표.
fn read_dxf(path: &Path, crs_auth_id: &str, review_reason: Option<String>) -> Vec<Record> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut reader = std::io::BufReader::new(file);
    let mut code_line = String::new();
    let mut value_line = String::new();
    let mut entity = String::new();
    let mut layer = String::new();
    let mut pending_x: Option<f64> = None;
    let mut found: BTreeMap<(String, String), Option<Extent>> = BTreeMap::new();
    loop {
        code_line.clear();
        if reader.read_line(&mut code_line).unwrap_or(0) == 0 {
            break;
        }
        value_line.clear();
        if reader.read_line(&mut value_line).unwrap_or(0) == 0 {
            break;
        }
        let code = code_line.trim();
        let value = value_line.trim();
        match code {
            "0" => {
                entity = value.to_owned();
                layer.clear();
                pending_x = None;
            }
            "8" => layer = value.to_owned(),
            "10" | "20" => {
                let Ok(number) = value.parse::<f64>() else {
                    pending_x = None;
                    continue;
                };
                if !number.is_finite() {
                    pending_x = None;
                    continue;
                }
                let Some(family) = geometry_family(&entity) else {
                    continue;
                };
                let key = (layer.clone(), family.to_owned());
                let slot = found.entry(key).or_insert(None);
                if code == "10" {
                    pending_x = Some(number);
                    continue;
                }
                let Some(x) = pending_x.take() else {
                    continue;
                };
                let one = Extent {
                    min_x: x,
                    min_y: number,
                    max_x: x,
                    max_y: number,
                };
                *slot = match *slot {
                    Some(mut merged) => {
                        merged.merge(one);
                        Some(merged)
                    }
                    None => Some(one),
                };
            }
            _ => pending_x = None,
        }
    }
    let source = path.to_string_lossy().into_owned();
    found
        .into_iter()
        .filter(|((layer, _), _)| !layer.is_empty())
        .map(|((layer, _family), extent)| {
            let (category, name) = infer_code(&layer);
            Record {
                source: source.clone(),
                layer_name: layer.clone(),
                cad_layer: layer.clone(),
                // C++ `TopographicCatalog.cpp:197` — 못 찾으면 「미분류 · 레이어」.
                display_name: if name.is_empty() {
                    format!("미분류 · {layer}")
                } else {
                    name
                },
                crs_auth_id: crs_auth_id.to_owned(),
                extent,
                category,
                review_reason: review_reason.clone(),
                missing_prj: false,
            }
        })
        .collect()
}

/// C++ `QFileInfo::completeBaseName` — 마지막 확장자만 뗀 이름(`a.b.dxf` → `a.b`).
fn complete_base_name(path: &Path) -> String {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return String::new();
    };
    match name.rfind('.') {
        Some(position) => name[..position].to_owned(),
        None => name.to_owned(),
    }
}

/// 파일 하나를 읽어 레코드들로 만든다. C++ `TopographicCatalog.cpp:163-234` 의 이 창분.
/// 빈 목록이면 읽을 레이어가 없었다는 뜻이다.
fn read_one(path: &Path) -> (Vec<Record>, Vec<String>) {
    let mut warnings = Vec::new();
    let source = path.to_string_lossy().into_owned();
    let suffix = suffix_of(path);
    let layer_name = complete_base_name(path);
    let prj = prj_crs_auth_id(path);
    // C++ `TopographicCatalog.cpp:50-52` — shp·ngi 는 .prj 를 동반해야 한다.
    let missing_prj = matches!(suffix.as_str(), "shp" | "ngi")
        && prj.is_none()
        && companion(path, "prj").is_none();
    let crs_auth_id = prj.unwrap_or_default();

    if suffix == "dxf" {
        // .prj 없는 DXF 는 어느 원점대인지 확정할 수 없다.
        // C++ 규칙(`KaTopographicImportDialog.cpp:465-470`)대로 [검토 필요]로 보류한다.
        let review_reason =
            crs_auth_id.is_empty().then(|| REVIEW_REASON_UNKNOWN_CRS.to_owned());
        let records = read_dxf(path, &crs_auth_id, review_reason);
        if records.is_empty() {
            warnings.push(format!("자료를 읽지 못했습니다: {source} · 도형 없음"));
            return (records, warnings);
        }
        for record in &records {
            if record.crs_auth_id.is_empty() {
                warnings.push(format!(
                    "좌표계 확인 필요: {} · {}",
                    record.source, record.layer_name
                ));
            }
            if !record.has_extent() {
                warnings.push(format!(
                    "도형 범위 없음: {} · {}",
                    record.source, record.layer_name
                ));
            }
        }
        return (records, warnings);
    }

    let extent = match suffix.as_str() {
        "shp" => read_shp_extent(path),
        // gpkg 는 GDAL 없이 레이어를 열 수 없어 범위를 읽지 않는다.
        _ => None,
    };
    let (category, display_name) = infer_code(&layer_name);
    let record = Record {
        source,
        layer_name,
        cad_layer: String::new(),
        display_name,
        crs_auth_id,
        extent,
        category,
        review_reason: None,
        missing_prj,
    };
    // C++ `TopographicCatalog.cpp:325-326` 경고 두 가지.
    if record.crs_auth_id.is_empty() {
        warnings.push(format!(
            "좌표계 확인 필요: {} · {}",
            record.source, record.layer_name
        ));
    }
    if !record.has_extent() {
        warnings.push(format!(
            "도형 범위 없음: {} · {}",
            record.source, record.layer_name
        ));
    }
    (vec![record], warnings)
}

/// 폴더 아래 파일을 모두 모은다(하위 폴더 포함, C++ `QDirIterator::Subdirectories`).
fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    children.sort();
    for child in children {
        if child.is_dir() {
            collect_files(&child, out);
        } else if child.is_file() {
            out.push(child);
        }
    }
}

/// C++ `TopographicCatalog::scan` (`TopographicCatalog.cpp:252-340`) 이 이 창에 주는 만큼.
#[must_use]
pub fn scan_folder(folder: &Path) -> ScanResult {
    let mut result = ScanResult::default();
    if folder.as_os_str().is_empty() || !folder.is_dir() || std::fs::read_dir(folder).is_err() {
        result.error = Some(format!(
            "수치지형도 폴더를 읽을 수 없습니다: {}",
            folder.to_string_lossy()
        ));
        return result;
    }
    let mut files = Vec::new();
    collect_files(folder, &mut files);
    for path in files {
        if !is_supported(&path) {
            continue;
        }
        result.files_read += 1;
        let (records, warnings) = read_one(&path);
        result.records.extend(records);
        result.warnings.extend(warnings);
    }
    result
}
