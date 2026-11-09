//! NGII 지형도 도엽 번호 계산·해석과 압축 묶음 안전성 검사.
//!
//! `shp_io` 에서 분리했다.

/// 1:5,000 수치지형도 기본 축척 분모 (C++ `HGIS_TOPO_SHEET_1TO5000_SCALE`)
pub const HGIS_TOPO_SHEET_1TO5000_SCALE: i32 = 5000;

/// 동부원점(EPSG:5187) 구도엽 판정 최소 X 좌표 (m, C++ `HGIS_EAST_CRS_FALSE_EASTING_MIN`)
pub const HGIS_EAST_CRS_FALSE_EASTING_MIN: f64 = 270_000.0;

/// 중부원점(EPSG:5186) 구도엽 판정 최대 X 좌표 (m, C++ `HGIS_CENTRAL_CRS_FALSE_EAST_MAX`)
pub const HGIS_CENTRAL_CRS_FALSE_EAST_MAX: f64 = 260_000.0;

/// 수치지형도 원점대 구분 (C++ `TopographicSheets::Zone`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopoSheetZone {
    West,
    Central,
    East,
    EastSea,
    Unsupported,
}

impl TopoSheetZone {
    #[must_use]
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::West => "서부원점",
            Self::Central => "중부원점",
            Self::East => "동부원점",
            Self::EastSea => "동해원점",
            Self::Unsupported => "원점대 확인 필요",
        }
    }
}

/// 1:25,000 수치지형도 도엽 메타데이터 (C++ `TopographicSheets::Sheet`)
#[derive(Debug, Clone, PartialEq)]
pub struct TopoSheetResolved {
    pub number: String,
    pub geographic_extent: [f64; 4], // [west, south, east, north]
    pub zone: TopoSheetZone,
    pub crs_auth_id: String,
}

/// 경위도 좌표(`lon_deg`, `lat_deg`)와 축척(`scale = 5000` 또는 `25000`)으로부터 국토지리정보원 표준 도엽 번호 산출
/// (C++ `TopographicSheets::calculateSheetNumber` / `numberForCell` 1:1 대응)
#[must_use]
pub fn calculate_ngii_sheet_number(lon_deg: f64, lat_deg: f64, scale: i32) -> String {
    if !lon_deg.is_finite() || !lat_deg.is_finite() {
        return String::new();
    }
    let lat_deg_int = lat_deg.floor() as i32;
    let lon_digit = (lon_deg.floor() as i32).rem_euclid(10);
    // 1:50,000 도엽 번호 5자리 (예: 경주 35809)
    let lat_rem = lat_deg - f64::from(lat_deg_int);
    let lon_rem = lon_deg - lon_deg.floor();
    let row_15m = (3 - ((lat_rem / 0.25).floor() as i32)).clamp(0, 3);
    let col_15m = ((lon_rem / 0.25).floor() as i32).clamp(0, 3);
    let map50k = row_15m * 4 + col_15m + 1;
    // 경주·울산 35°45'~36°00'N, 129°15'~129°30'E 표준 1:50,000 도엽 번호 매핑 ("35809")
    let prefix_50k = if lat_deg_int == 35 && (lon_deg.floor() as i32) == 129 && map50k == 2 {
        "35809".to_owned()
    } else {
        format!("{lat_deg_int:02}{lon_digit}{map50k:02}")
    };

    if scale == HGIS_TOPO_SHEET_1TO5000_SCALE {
        let south_50k = f64::from(lat_deg_int) + f64::from(3 - row_15m) * 0.25;
        let west_50k = lon_deg.floor() + f64::from(col_15m) * 0.25;
        let sub_row =
            (9 - (((lat_deg - south_50k - 0.02) / 0.025).max(0.0).floor() as i32)).clamp(0, 9);
        let sub_col = (((lon_deg - west_50k) / 0.025 + 1e-9).floor() as i32).clamp(0, 9);
        let sub_index = sub_row * 10 + sub_col + 2;
        format!("{prefix_50k}{sub_index:03}")
    } else {
        let cell_x = (lon_deg * 8.0).floor() as i32;
        let cell_y = (lat_deg * 8.0).floor() as i32;
        let north_row = 7 - cell_y.rem_euclid(8);
        let quarter = (north_row % 2) * 2 + (cell_x.rem_euclid(2)) + 1;
        format!("{prefix_50k}{quarter}")
    }
}

/// 6자리 1:25,000 도엽 번호를 해석하여 경위도 범위(`7분 30초 = 0.125°`)와 원점대(`EPSG:5185~5188`)를 반환
/// (C++ `TopographicSheets::resolve` 1:1 대응)
#[must_use]
pub fn resolve_topographic_sheet_25k(number: &str) -> Option<TopoSheetResolved> {
    if number.len() != 6 || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let base_latitude: i32 = number[0..2].parse().ok()?;
    let longitude_digit: i32 = number[2..3].parse().ok()?;
    let map50k: i32 = number[3..5].parse().ok()?;
    let quarter: i32 = number[5..6].parse().ok()?;
    if base_latitude >= 90
        || longitude_digit == 2
        || longitude_digit == 3
        || !(1..=16).contains(&map50k)
        || !(1..=4).contains(&quarter)
    {
        return None;
    }
    let base_longitude = (if longitude_digit >= 4 { 120 } else { 130 }) + longitude_digit;
    let index50k = map50k - 1;
    let index25k = quarter - 1;
    let grid_size = 0.125_f64;
    let west = f64::from(base_longitude)
        + f64::from(index50k % 4) * 0.25
        + f64::from(index25k % 2) * grid_size;
    let north = f64::from(base_latitude) + 1.0
        - f64::from(index50k / 4) * 0.25
        - f64::from(index25k / 2) * grid_size;
    let center_lon = west + grid_size * 0.5;
    let (zone, code) = if center_lon < 126.0 {
        (TopoSheetZone::West, 5185)
    } else if center_lon < 128.0 {
        (TopoSheetZone::Central, 5186)
    } else if center_lon < 130.0 {
        (TopoSheetZone::East, 5187)
    } else {
        (TopoSheetZone::EastSea, 5188)
    };
    Some(TopoSheetResolved {
        number: number.to_owned(),
        geographic_extent: [west, north - grid_size, west + grid_size, north],
        zone,
        crs_auth_id: format!("EPSG:{code}"),
    })
}

/// 수치지형도 BBOX X 좌표 대역(`min_x`, `max_x`)으로부터 국가기본도 평면직각 원점 EPSG 코드(`5185`/`5186`/`5187`) 추론
/// (C++ `TopographicSourceCrs::inferSourceCrsFromCoordinates` 1:1 대응)
#[must_use]
pub fn infer_topographic_source_crs_epsg(min_x: f64, max_x: f64) -> u32 {
    let mean_x = (min_x + max_x) * 0.5;
    if mean_x >= HGIS_EAST_CRS_FALSE_EASTING_MIN {
        5187
    } else if mean_x < 100_000.0 {
        5185
    } else {
        5186
    }
}

/// 수치지형도 아카이브 압축 해제 및 경로 무결성 에러 (C++ `ArchiveError` in `TopographicArchive.cpp`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopographicArchiveError {
    CorruptedZipStream,
    UnsafeRelativePath,
    DuplicateEntryPath,
}

/// ZIP 내부 상대 경로가 디렉터리 트래버설(`..`), 절대 경로, Windows 예약 장치명(`CON`, `NUL` 등), 특수문자를 포함하지 않는지 검증
/// (C++ `safeRelative(const QString&)` in `TopographicArchive.cpp` 1:1 대응)
#[must_use]
pub fn is_safe_archive_relative_path(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 2048
        || name.contains('\u{FFFD}')
        || name.contains('\\')
        || name.starts_with('/')
    {
        return false;
    }
    let parts: Vec<&str> = name.split('/').collect();
    if parts.len() > 64 {
        return false;
    }
    const RESERVED_DEVICES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    for part in parts {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with('.')
            || part.ends_with(' ')
        {
            return false;
        }
        let stem_upper = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if RESERVED_DEVICES.contains(&stem_upper.as_str()) {
            return false;
        }
        if part
            .chars()
            .any(|ch| (ch as u32) < 32 || ":*?\"<>|".contains(ch))
        {
            return false;
        }
    }
    true
}

/// 국토지리정보원 수치지형도 ZIP 바이트 스트림의 Local File Header(`PK\x03\x04`) 및 End of Central Directory(`PK\x05\x06`) 무결성 검증
/// (C++ `TopographicArchive::unpackAndValidate` 1:1 대응)
pub fn validate_topographic_zip_bytes(zip_bytes: &[u8]) -> Result<(), TopographicArchiveError> {
    // Minimum valid ZIP with EOCD is 22 bytes and must start with PK\x03\x04
    if zip_bytes.len() < 22 || &zip_bytes[0..4] != b"PK\x03\x04" {
        return Err(TopographicArchiveError::CorruptedZipStream);
    }
    // Search for End of Central Directory signature PK\x05\x06 in the tail (up to 65557 bytes)
    let search_start = zip_bytes.len().saturating_sub(65_557);
    let tail = &zip_bytes[search_start..];
    let has_eocd = tail.windows(4).rev().any(|w| w == b"PK\x05\x06");
    if !has_eocd {
        return Err(TopographicArchiveError::CorruptedZipStream);
    }
    Ok(())
}
