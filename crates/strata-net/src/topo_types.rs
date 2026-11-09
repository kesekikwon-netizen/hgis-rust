//! 폴더 스캔 결과 자료형과 코드북 (C++ `TopographicCatalog.cpp` 의 레코드 구조).
//!
//! `topographic_catalog` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

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

pub(super) const LAYER_CODES_JSON: &str = include_str!("../../../data/topographic/layer-codes.json");

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
    pub(super) fn merge(&mut self, other: Self) {
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
