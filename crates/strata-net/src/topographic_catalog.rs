//! C++ `src/core/TopographicCatalog.cpp` 1:1 — 「지도에 올리기」 창이 쓰는 폴더 스캔.
//!
//! C++ 은 GDAL/OGR 로 파일을 열어 레이어·좌표계·도형 범위를 읽는다. 이 모듈은 그중
//! 이 창이 실제로 보여 주는 만큼만 읽는다.
//!
//! - 확장자 거르기: `TopographicCatalog.cpp:41-45`
//! - shp·ngi 곁 파일(.prj) 동반 확인: `:46-56`
//! - 코드북(`data/topographic/layer-codes.json`) 대조 카테고리 판정: `:64-103`
//! - DXF 는 CAD 레이어별로 나눠 레코드를 만든다: `:181-209`
//! - 경고 문구: `:321-327`, 폴더 오류 문구: `:260-263`
//!
//! C++ 과 다른 점 둘(이 창의 동작에는 영향이 없다):
//! - 색인 캐시를 두지 않는다. C++ `cacheHits` 자리는 늘 0 이다.
//! - ZIP 안쪽(`/vsizip/`)은 읽지 않는다. 압축해 둔 도엽은 풀어서 넣어야 한다.

use std::collections::BTreeMap;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;

/// `TopographicCatalog.cpp:41-45` `supported()` — 읽는 확장자 네 가지.
pub const SUPPORTED_EXTENSIONS: [&str; 4] = ["shp", "ngi", "dxf", "gpkg"];

/// C++ `KaTopographicImportDialog.cpp:640` `retainForReview` 기본 설명.
pub const REVIEW_REASON_UNKNOWN_CRS: &str = "원본 좌표계 또는 도형 범위 미확인";

/// C++ `KaTopographicImportDialog.cpp:530` — 범위 없는 항목은 올릴 수 없다.
pub const IMPORT_NEEDS_EXTENT: &str =
    "유효한 도형 범위가 없는 항목은 지도에 추가할 수 없습니다. 해당 체크를 해제하세요.";

/// C++ `KaTopographicImportDialog.cpp:535` — 좌표계 미확인 항목은 올릴 수 없다.
pub const IMPORT_NEEDS_CRS: &str =
    "좌표계가 확인되지 않은 레이어가 있습니다. 해당 좌표계를 선택하거나 추가 체크를 해제하세요.";

const LAYER_CODES_JSON: &str = include_str!("../../../data/topographic/layer-codes.json");

/// C++ `TopographicCatalog.h:14-15` `enum class Category`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Category {
    #[default]
    Unknown,
    Contour,
    Road,
    Building,
    Water,
    Vegetation,
    Boundary,
    Facility,
    ElevationPoint,
    PlaceName,
}

impl Category {
    /// C++ `TopographicCatalog.cpp:90-92` — 이름을 이 순서로 대조한다.
    pub const ALL: [Self; 9] = [
        Self::Contour,
        Self::Road,
        Self::Building,
        Self::Water,
        Self::Vegetation,
        Self::Boundary,
        Self::Facility,
        Self::ElevationPoint,
        Self::PlaceName,
    ];

    /// C++ `TopographicCatalog.cpp:238-251` `categoryName`.
    #[must_use]
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::Contour => "등고선",
            Self::Road => "도로",
            Self::Building => "건물",
            Self::Water => "수계",
            Self::Vegetation => "식생",
            Self::Boundary => "경계",
            Self::Facility => "시설물",
            Self::ElevationPoint => "표고점",
            Self::PlaceName => "지명",
            Self::Unknown => "미분류",
        }
    }

    /// C++ `TopographicCatalog.cpp:72-77` 코드북의 `category` 문자열.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        match key {
            "Contour" => Self::Contour,
            "Road" => Self::Road,
            "Building" => Self::Building,
            "Water" => Self::Water,
            "Vegetation" => Self::Vegetation,
            "Boundary" => Self::Boundary,
            "Facility" => Self::Facility,
            "ElevationPoint" => Self::ElevationPoint,
            "PlaceName" => Self::PlaceName,
            _ => Self::Unknown,
        }
    }
}

/// C++ `QgsRectangle` 자리 — 원본 좌표계 기준 최소·최대.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Extent {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Extent {
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.min_x.is_finite()
            && self.min_y.is_finite()
            && self.max_x.is_finite()
            && self.max_y.is_finite()
    }

    /// C++ `QgsRectangle::combineExtentWith`.
    fn merge(&mut self, other: Self) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }
}

/// C++ `TopographicCatalog.h:16-29` `struct Record`.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// 원본 파일 경로.
    pub source: String,
    /// 레이어 이름 — 파일 하나짜리 자료는 파일 이름(마지막 확장자만 뗀 것),
    /// DXF 는 CAD 레이어 이름이다(C++ `cadLayer`).
    pub layer_name: String,
    /// DXF 의 CAD 레이어 이름. 다른 형식은 빈 문자열(`TopographicCatalog.h:19-20`).
    pub cad_layer: String,
    /// 표에 보일 이름. 코드북에서 이름을 찾았을 때만 채워진다.
    pub display_name: String,
    /// `.prj` 에서 읽은 좌표계(`EPSG:5187`). 비어 있으면 「좌표계 미확인」.
    pub crs_auth_id: String,
    /// 읽지 못했으면 `None` — C++ 은 `hasExtent=false` 로 두고 「범위 없음」이라 쓴다.
    pub extent: Option<Extent>,
    pub category: Category,
    /// C++ `KaTopographicImportDialog::m_reviewReasons` — 있으면 `[검토 필요]`, 자동 체크 없음.
    pub review_reason: Option<String>,
    /// `.prj` 를 동반하지 않은 파일(`TopographicCatalog.cpp:50-52`).
    pub missing_prj: bool,
}

impl Record {
    /// C++ `KaTopographicImportDialog.cpp:466` — `displayName` 이 비면 레이어 이름.
    #[must_use]
    pub fn label_ko(&self) -> &str {
        if self.display_name.is_empty() {
            &self.layer_name
        } else {
            &self.display_name
        }
    }

    /// C++ `KaTopographicImportDialog.cpp:488-491` 범위 칸 문구.
    #[must_use]
    pub fn extent_label_ko(&self) -> String {
        match self.extent {
            Some(extent) if extent.is_finite() => format!(
                "X {:.1} ~ {:.1}\nY {:.1} ~ {:.1}",
                extent.min_x, extent.max_x, extent.min_y, extent.max_y
            ),
            _ => "범위 없음".to_owned(),
        }
    }

    #[must_use]
    pub fn has_extent(&self) -> bool {
        self.extent.is_some_and(|extent| extent.is_finite())
    }

    /// `.prj` 에서 읽은 EPSG 번호.
    #[must_use]
    pub fn crs_epsg(&self) -> Option<u32> {
        self.crs_auth_id
            .strip_prefix("EPSG:")
            .and_then(|code| code.parse().ok())
    }

    /// C++ `KaTopographicImportDialog.cpp:466` — 표에 그리는 이름.
    #[must_use]
    pub fn display_label_ko(&self) -> String {
        let label = self.label_ko();
        match &self.review_reason {
            Some(_) => format!("{label} [검토 필요]"),
            None => label.to_owned(),
        }
    }

    /// 처음 체크되는가 — C++ `KaTopographicImportDialog.cpp:469-470` + `:60-64`.
    /// 「검토 필요」면 무조건 해제다(C++ 은 사용자가 직접 체크해야 올라간다).
    #[must_use]
    pub fn initially_selected(&self) -> bool {
        self.review_reason.is_none()
            && matches!(
                self.category,
                Category::Contour
                    | Category::Road
                    | Category::Building
                    | Category::Water
                    | Category::Boundary
            )
    }
}

/// C++ `TopographicCatalog.h:30-37` `struct ScanResult`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanResult {
    pub records: Vec<Record>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub files_read: usize,
    /// 색인 캐시를 두지 않으므로 늘 0 이다(C++ 상태줄 문구 자리를 지키려고 남긴다).
    pub cache_hits: usize,
}

impl ScanResult {
    /// C++ `KaTopographicImportDialog.cpp:494-495` 상태줄.
    #[must_use]
    pub fn summary_ko(&self) -> String {
        format!(
            "레이어 {}개 · 파일 {}개 확인 · 캐시 {}개. 추가할 항목에 체크하세요.\n{}",
            self.records.len(),
            self.files_read,
            self.cache_hits,
            self.warnings.join("\n")
        )
    }

    /// C++ `KaTopographicImportDialog.cpp:465-467` — 표에 그릴 (이름, 검토 사유) 목록.
    #[must_use]
    pub fn rows_ko(&self) -> Vec<(String, Option<String>)> {
        self.records
            .iter()
            .map(|record| (record.display_label_ko(), record.review_reason.clone()))
            .collect()
    }
}

/// 체크된 항목 가운데 올릴 수 없는 것이 있으면 C++ 과 같은 안내문.
/// C++ `KaTopographicImportDialog.cpp:529-537` 순서대로 범위를 먼저 본다.
#[must_use]
pub fn blocking_message_ko(checked: &[&Record]) -> Option<&'static str> {
    if checked.iter().any(|record| !record.has_extent()) {
        return Some(IMPORT_NEEDS_EXTENT);
    }
    if checked.iter().any(|record| record.crs_epsg().is_none()) {
        return Some(IMPORT_NEEDS_CRS);
    }
    None
}

#[derive(Debug, Clone)]
struct CodeInfo {
    category: Category,
    name: String,
}

/// C++ `TopographicCatalog.cpp:64-88` `codeBook()` — `data/topographic/layer-codes.json`.
fn code_book() -> &'static BTreeMap<String, CodeInfo> {
    static BOOK: OnceLock<BTreeMap<String, CodeInfo>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let mut entries = BTreeMap::new();
        let document: Value = serde_json::from_str(LAYER_CODES_JSON).unwrap_or(Value::Null);
        let Some(codes) = document.get("codes").and_then(Value::as_array) else {
            return entries;
        };
        for value in codes {
            let code = value.get("code").and_then(Value::as_str).unwrap_or_default();
            let name = value.get("name").and_then(Value::as_str).unwrap_or_default();
            if code.is_empty() || name.is_empty() {
                continue;
            }
            let category = value
                .get("category")
                .and_then(Value::as_str)
                .map_or(Category::Unknown, Category::from_key);
            entries.insert(
                code.to_owned(),
                CodeInfo {
                    category,
                    name: name.to_owned(),
                },
            );
        }
        entries
    })
}

/// C++ `TopographicCatalog.cpp:94` `(?<![\p{L}\p{N}])([A-H][0-9]{7})(?![\p{L}\p{N}])`.
fn code_tokens(name: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index + 8 <= chars.len() {
        let window = &chars[index..index + 8];
        let shape_ok =
            matches!(window[0].1, 'A'..='H') && window[1..].iter().all(|(_, c)| c.is_ascii_digit());
        let before_ok = index == 0 || !chars[index - 1].1.is_alphanumeric();
        let after_ok = index + 8 == chars.len() || !chars[index + 8].1.is_alphanumeric();
        if shape_ok && before_ok && after_ok {
            tokens.push(&name[window[0].0..=window[7].0]);
            index += 8;
        } else {
            index += 1;
        }
    }
    tokens
}

/// C++ `TopographicCatalog.cpp:89-103` `inferCode` — 카테고리 한글명, 이어서 코드북 대조.
/// 코드북에 없거나 카테고리가 서로 다른 코드가 섞이면 미분류다.
#[must_use]
pub fn infer_code(name: &str) -> (Category, String) {
    let trimmed = name.trim();
    for category in Category::ALL {
        if trimmed == category.name_ko() {
            return (category, trimmed.to_owned());
        }
    }
    let mut found: Option<&CodeInfo> = None;
    for token in code_tokens(name) {
        let Some(info) = code_book().get(token) else {
            return (Category::Unknown, String::new());
        };
        if found.is_some_and(|known| known.category != info.category) {
            return (Category::Unknown, String::new());
        }
        found = Some(info);
    }
    found.map_or((Category::Unknown, String::new()), |info| {
        (info.category, info.name.clone())
    })
}

/// `TopographicCatalog.cpp:41-45` — 확장자(대소문자 무관)가 네 가지 중 하나인가.
#[must_use]
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

fn suffix_of(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// C++ `TopographicCatalog.cpp:50-52` — 같은 이름의 곁 파일(`.prj`, 대소문자 무관).
fn companion(path: &Path, wanted: &str) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let folder = path.parent()?;
    let mut found: Option<PathBuf> = None;
    for entry in std::fs::read_dir(folder).ok()?.flatten() {
        let candidate = entry.path();
        if !candidate.is_file() {
            continue;
        }
        let Some(candidate_stem) = candidate.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let matches_extension = candidate
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted));
        if matches_extension && candidate_stem.eq_ignore_ascii_case(stem) {
            found = Some(candidate);
        }
    }
    found
}

/// `.prj` 글 안의 원점 이름 — `CadCrsHints.cpp:48-57` 이 후보 정의와 맞춰 보던 자리.
/// ESRI 글에는 권위 번호가 없는 일이 흔해 이름으로 읽는다.
/// (원점 이름, GRS80 계열, 보정 베셀 계열) — `CadCrsHints.cpp:50-54`.
const PRJ_NAME_CODES: [(&str, u32, u32); 7] = [
    ("JEJU", 5182, 5175),
    ("EAST_SEA", 5188, 5177),
    ("WEST", 5185, 5173),
    ("EAST", 5187, 5176),
    ("CENTRAL", 5186, 5174),
    ("UNIFIED", 5179, 5179),
    ("UTM", 5179, 5179),
];

/// `.prj` WKT 에서 좌표계를 읽는다. 못 읽으면 `None`.
#[must_use]
pub fn crs_auth_id_from_wkt(wkt: &str) -> Option<String> {
    let upper = wkt.to_ascii_uppercase();
    if let Some(position) = upper.find("AUTHORITY[\"EPSG\",\"") {
        let rest = &upper[position + "AUTHORITY[\"EPSG\",\"".len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        return (!digits.is_empty()).then(|| format!("EPSG:{digits}"));
    }
    let bessel = upper.contains("BESSEL");
    for (needle, code_grs80, code_bessel) in PRJ_NAME_CODES {
        if upper.contains(needle) {
            let code = if bessel { code_bessel } else { code_grs80 };
            return Some(format!("EPSG:{code}"));
        }
    }
    None
}

/// 곁 파일 `.prj` 를 읽어 좌표계를 얻는다. 없거나 못 읽으면 `None`.
fn prj_crs_auth_id(path: &Path) -> Option<String> {
    let prj = companion(path, "prj")?;
    let bytes = std::fs::read(prj).ok()?;
    if bytes.len() > 65_536 {
        return None;
    }
    crs_auth_id_from_wkt(&String::from_utf8_lossy(&bytes))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트용 임시 폴더. 끝나면 스스로 지운다(C++ `QTemporaryDir` 자리).
    struct TempFolder {
        path: PathBuf,
    }

    impl TempFolder {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos());
            let path = std::env::temp_dir()
                .join(format!("ka_hgis_{tag}_{}_{nanos}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("임시 폴더를 만들지 못했습니다");
            Self { path }
        }

        fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
            let target = self.path.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).expect("상위 폴더를 만들지 못했습니다");
            }
            std::fs::write(&target, bytes).expect("가짜 자료를 쓰지 못했습니다");
            target
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// `.shp` 머리말 100바이트(매직 9994 + 경계 상자)를 만든다.
    fn shp_bytes(extent: Extent) -> Vec<u8> {
        let mut header = vec![0_u8; 100];
        header[..4].copy_from_slice(&9994_u32.to_be_bytes());
        for (offset, value) in [
            (36, extent.min_x),
            (44, extent.min_y),
            (52, extent.max_x),
            (60, extent.max_y),
        ] {
            header[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        header
    }

    /// CAD 레이어 `layer` 에 선 두 개를 둔 최소 DXF.
    fn dxf_bytes(layer: &str) -> Vec<u8> {
        format!(
            "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n{layer}\n10\n1000.0\n20\n2000.0\n\
             0\nLINE\n8\n{layer}\n10\n1500.0\n20\n2500.0\n0\nENDSEC\n0\nEOF\n"
        )
        .into_bytes()
    }

    const PRJ_5186: &str = "PROJCS[\"Korea 2000 / Central Belt\",GEOGCS[\"Korea 2000\",\
        DATUM[\"Korea_2000\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],\
        PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
        PROJECTION[\"Transverse_Mercator\"],UNIT[\"metre\",1],AUTHORITY[\"EPSG\",\"5186\"]]";

    const PRJ_5187_NAMED: &str = "PROJCS[\"Korea_2000_Korea_East_Belt\",GEOGCS[\"Korea 2000\",\
        DATUM[\"Korea_2000\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],\
        PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
        PROJECTION[\"Transverse_Mercator\"],UNIT[\"metre\",1]]";

    // C++ `TopographicCatalog.cpp:41-45,50-52` — 확장자 네 가지, shp 는 .prj 동반 확인.
    #[test]
    fn test_r14_scan_folder_filters_extensions_and_checks_prj_companion() {
        let folder = TempFolder::new("r14_filter");
        folder.write("A0010000.shp", &shp_bytes(Extent {
            min_x: 218_450.0,
            min_y: 359_800.0,
            max_x: 221_250.0,
            max_y: 362_400.0,
        }));
        folder.write("A0010000.prj", PRJ_5186.as_bytes());
        // .prj 가 없는 shp — 좌표계를 확인할 수 없다.
        folder.write("B0010000.shp", &shp_bytes(Extent {
            min_x: 1.0,
            min_y: 2.0,
            max_x: 3.0,
            max_y: 4.0,
        }));
        // 읽지 않는 확장자들.
        folder.write("메모.txt", b"not a layer");
        folder.write("도면.dwg", b"AC1032");
        folder.write("묶음.zip", b"PK\x03\x04");
        // 하위 폴더도 본다(C++ `QDirIterator::Subdirectories`).
        folder.write("하위/G0010000.ngi", b"\x00\x00\x00\x00");

        let result = scan_folder(&folder.path);
        assert_eq!(result.error, None);
        assert_eq!(result.files_read, 3, "shp 둘 + ngi 하나만 센다");
        assert_eq!(result.records.len(), 3);
        assert_eq!(result.cache_hits, 0, "색인 캐시를 두지 않는다");

        let road = result
            .records
            .iter()
            .find(|record| record.layer_name == "A0010000")
            .expect("A0010000.shp 레코드");
        // 코드북 대조(`TopographicCatalog.cpp:99`) — A0010000 은 도로다.
        assert_eq!(road.category, Category::Road);
        assert_eq!(road.display_name, "도로경계(미분류)");
        assert_eq!(road.crs_auth_id, "EPSG:5186", ".prj 를 읽어 좌표계를 채운다");
        assert!(!road.missing_prj);
        assert_eq!(
            road.extent_label_ko(),
            "X 218450.0 ~ 221250.0\nY 359800.0 ~ 362400.0"
        );
        assert!(road.review_reason.is_none());
        assert!(road.initially_selected(), "도로는 처음부터 체크된다");

        let building = result
            .records
            .iter()
            .find(|record| record.layer_name == "B0010000")
            .expect("B0010000.shp 레코드");
        assert_eq!(building.category, Category::Building);
        assert!(building.missing_prj, ".prj 없는 shp 는 동반 확인에 걸린다");
        assert!(building.crs_auth_id.is_empty());
        assert!(
            result
                .warnings
                .iter()
                .any(|line| line == &format!("좌표계 확인 필요: {} · B0010000", building.source)),
            "C++ :325 경고 문구"
        );

        let boundary = result
            .records
            .iter()
            .find(|record| record.layer_name == "G0010000")
            .expect("하위 폴더의 ngi 레코드");
        assert_eq!(boundary.category, Category::Boundary);
        assert_eq!(boundary.extent_label_ko(), "범위 없음");
        assert!(
            result
                .warnings
                .iter()
                .any(|line| line == &format!("도형 범위 없음: {} · G0010000", boundary.source)),
            "C++ :326 경고 문구"
        );
    }

    // C++ `KaTopographicImportDialog.cpp:465-470` — .prj 없는 DXF 는 [검토 필요], 자동 체크 없음.
    #[test]
    fn test_r14_dxf_without_prj_is_held_for_review_not_auto_checked() {
        let folder = TempFolder::new("r14_dxf_review");
        folder.write("등고선없음/35905086_행정경계.dxf", &dxf_bytes("G0010000"));
        folder.write("등고선있음/35905085_등고선.dxf", &dxf_bytes("F0010000"));
        folder.write(
            "등고선있음/35905085_등고선.prj",
            PRJ_5187_NAMED.as_bytes(),
        );

        let result = scan_folder(&folder.path);
        assert_eq!(result.error, None);
        assert_eq!(result.files_read, 2);
        assert_eq!(result.records.len(), 2, "DXF 한 파일이 CAD 레이어 하나다");

        let without_prj = result
            .records
            .iter()
            .find(|record| record.source.contains("등고선없음"))
            .expect("검토 필요 레코드");
        assert_eq!(without_prj.category, Category::Boundary);
        assert!(without_prj.crs_auth_id.is_empty());
        assert_eq!(
            without_prj.review_reason.as_deref(),
            Some(REVIEW_REASON_UNKNOWN_CRS)
        );
        assert!(
            !without_prj.initially_selected(),
            "C++ :469-470 은 검토 필요 항목을 체크하지 않는다"
        );
        assert!(
            without_prj.display_label_ko().ends_with(" [검토 필요]"),
            "C++ :466-467 은 이름 뒤에 [검토 필요]를 붙인다"
        );
        assert_eq!(
            without_prj.extent_label_ko(),
            "X 1000.0 ~ 1500.0\nY 2000.0 ~ 2500.0"
        );

        let with_prj = result
            .records
            .iter()
            .find(|record| record.source.contains("등고선있음"))
            .expect("좌표계를 읽은 레코드");
        assert_eq!(with_prj.category, Category::Contour);
        assert_eq!(with_prj.crs_auth_id, "EPSG:5187", "이름만 있는 ESRI 글도 읽는다");
        assert!(with_prj.review_reason.is_none());
        assert!(with_prj.initially_selected(), "등고선은 처음부터 체크된다");
        assert_eq!(with_prj.display_label_ko(), "등고선(미분류)");
        assert_eq!(with_prj.cad_layer, "F0010000");
    }

    // C++ `TopographicCatalog.cpp:260-263` — 폴더를 읽을 수 없을 때 문구.
    #[test]
    fn test_r14_unreadable_folder_reports_the_cpp_message() {
        let folder = TempFolder::new("r14_missing");
        let missing = folder.path.join("없는폴더");
        let result = scan_folder(&missing);
        assert_eq!(
            result.error.as_deref(),
            Some(
                format!(
                    "수치지형도 폴더를 읽을 수 없습니다: {}",
                    missing.to_string_lossy()
                )
                .as_str()
            )
        );
        assert!(result.records.is_empty());
        assert_eq!(result.files_read, 0);

        let empty = scan_folder(Path::new(""));
        assert!(empty.error.is_some(), "빈 경로도 같은 오류다");
    }

    // C++ `TopographicCatalog.cpp:89-103` — 이름이 카테고리 한글명이거나, 코드북 코드를 품었을 때.
    #[test]
    fn test_r14_infer_code_matches_the_cpp_codebook_rules() {
        assert_eq!(infer_code("등고선"), (Category::Contour, "등고선".to_owned()));
        assert_eq!(infer_code(" 표고점 ").0, Category::ElevationPoint);

        let (category, name) = infer_code("N3L_F0010000_등고선도엽");
        assert_eq!(category, Category::Contour);
        assert_eq!(name, "등고선(미분류)");

        // 카테고리가 서로 다른 코드가 섞이면 판정을 포기한다(C++ :99).
        assert_eq!(
            infer_code("F0010000_A0010000"),
            (Category::Unknown, String::new())
        );
        // 같은 카테고리 코드 둘은 뒤엣것을 쓴다(C++ 은 마지막 일치를 남긴다).
        assert_eq!(infer_code("A0010000_A0013111").0, Category::Road);
        // 코드북에 없는 코드는 미분류.
        assert_eq!(
            infer_code("Z9999999"),
            (Category::Unknown, String::new())
        );
        // 글자에 붙은 8자 토큰은 코드가 아니다(C++ 전후방 경계 규칙).
        assert_eq!(infer_code("xF00100001").0, Category::Unknown);
        // 파일 이름처럼 카테고리 이름을 품기만 해도 코드가 아니면 미분류다.
        assert_eq!(infer_code("35905085_등고선").0, Category::Unknown);
    }

    // C++ `KaTopographicImportDialog.cpp:494-495, 529-537` — 상태줄과 추가 거부 문구.
    #[test]
    fn test_r14_summary_and_blocking_messages_match_the_cpp_dialog() {
        let folder = TempFolder::new("r14_messages");
        folder.write("F0010000.shp", &shp_bytes(Extent {
            min_x: 10.0,
            min_y: 20.0,
            max_x: 30.0,
            max_y: 40.0,
        }));
        folder.write("F0010000.prj", PRJ_5186.as_bytes());
        let result = scan_folder(&folder.path);
        assert_eq!(
            result.summary_ko(),
            "레이어 1개 · 파일 1개 확인 · 캐시 0개. 추가할 항목에 체크하세요.\n"
        );

        let record = &result.records[0];
        assert_eq!(blocking_message_ko(&[record]), None);

        let extentless = Record {
            extent: None,
            ..record.clone()
        };
        assert_eq!(
            blocking_message_ko(&[&extentless]),
            Some(IMPORT_NEEDS_EXTENT),
            "범위를 먼저 본다"
        );

        let crsless = Record {
            crs_auth_id: String::new(),
            ..record.clone()
        };
        assert_eq!(blocking_message_ko(&[&crsless]), Some(IMPORT_NEEDS_CRS));
        assert_eq!(blocking_message_ko(&[]), None, "체크가 없으면 막지 않는다");
    }

    // `.prj` 좌표계 읽기 — 권위 번호가 있으면 그것, 없으면 원점 이름(C++ `CadCrsHints.cpp:31-45`).
    #[test]
    fn test_r14_prj_wkt_reads_authority_number_then_origin_name() {
        assert_eq!(
            crs_auth_id_from_wkt(PRJ_5186).as_deref(),
            Some("EPSG:5186")
        );
        assert_eq!(
            crs_auth_id_from_wkt(PRJ_5187_NAMED).as_deref(),
            Some("EPSG:5187"),
            "이름에 EAST 가 있으면 동부원점(GRS80)"
        );
        assert_eq!(
            crs_auth_id_from_wkt("PROJCS[\"Korea_2000_Korea_Central_Belt\",\
                 GEOGCS[\"Korea 2000\",DATUM[\"D\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528]]]]")
                .as_deref(),
            Some("EPSG:5174"),
            "베셀 원점은 보정 베셀 번호를 쓴다"
        );
        assert_eq!(crs_auth_id_from_wkt("LOCAL_CS[\"없는 좌표계\"]"), None);
    }

    // C++ `TopographicCatalog.cpp:50-52` — 대소문자가 달라도 같은 이름의 곁 파일을 찾는다.
    #[test]
    fn test_r14_companion_prj_is_case_insensitive() {
        let folder = TempFolder::new("r14_prj_case");
        folder.write("등고선.SHP", &shp_bytes(Extent {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        }));
        folder.write("등고선.PrJ", PRJ_5186.as_bytes());
        let result = scan_folder(&folder.path);
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].crs_auth_id, "EPSG:5186");
        assert!(!result.records[0].missing_prj);
        assert_eq!(result.records[0].crs_epsg(), Some(5186));
    }
}
