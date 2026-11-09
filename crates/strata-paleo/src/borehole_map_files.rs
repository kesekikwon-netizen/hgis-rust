//! C++ `GeoInfoBoreholeMaps.cpp` 의 산출 파일명·레이어 표시명 상수.

/// C++ `:251-291` 의 파일명 상수 6종.
pub const FILE_ALLUVIAL_ISOPACH: &str = "시추_자연퇴적층_두께.tif";
pub const FILE_FILL_THICKNESS: &str = "시추_성토매립층_두께.tif";
pub const FILE_BEDROCK_DEPTH: &str = "시추_기반암_출현깊이.tif";
pub const FILE_BEDROCK_ELEV: &str = "시추_기반암_해발고도.tif";
pub const FILE_GROUNDWATER: &str = "시추_평균지하수위.tif";
pub const FILE_ISOPACH_GPKG: &str = "시추_등층후선.gpkg";

/// C++ `:253-290` 의 레이어 표시명(범례 제목).
pub const TITLE_ALLUVIAL_ISOPACH: &str = "[시추·지반] 자연 퇴적층 두께(m)";
pub const TITLE_FILL_THICKNESS: &str = "[시추·지반] 성토·매립층(교란층) 두께 분포도(m)";
pub const TITLE_BEDROCK_DEPTH: &str = "[시추·지반] 풍화암·기반암 출현 깊이(GL-m)";
pub const TITLE_BEDROCK_ELEV: &str = "[시추·지반] 풍화암·기반암 해발고도(EL.m)";
pub const TITLE_GROUNDWATER: &str = "[시추·지반] 평균 지하수위(GL-m) 분포도";
pub const TITLE_ISOPACH_CONTOUR: &str = "[시추·지반] 충적층 등층후선(m)";
/// C++ `:187` 의 시추공 마커 레이어(보어홀 레이어 자체의 표시 토글).
pub const TITLE_BOREHOLE_MARKER: &str = "시추공 분포·주상도 요약 마커";

/// C++ `xs.size() < 2` 조기 반환 기준 (`GeoInfoBoreholeMaps.cpp:226`).
pub const BOREHOLE_MIN_POINTS: usize = 2;
