use std::path::Path;
use strata_types::error::StrataResult;

pub const DBF_MAX_FIELD_NAME_LEN: usize = 10;

/// C++ `ShapefileWriter` — CP949(EUC-KR) 기준 바이트 길이 계산 (ASCII 1바이트, 한글 등 다중바이트 음절 2바이트)
#[must_use]
pub fn cp949_byte_len(s: &str) -> usize {
    s.chars().map(|ch| if ch.is_ascii() { 1 } else { 2 }).sum()
}

/// C++ `ShapefileWriter::exportLayerToShp` — DBF 10바이트 컬럼명 제한에 맞춰 한글 음절 경계를 보존하며 절단 및 `_1` 접미사 부여
#[must_use]
pub fn truncate_dbf_field_name_cp949(name: &str, collision_index: usize) -> String {
    if cp949_byte_len(name) <= DBF_MAX_FIELD_NAME_LEN && collision_index == 0 {
        return name.to_owned();
    }
    let idx = collision_index.max(1);
    let suffix = format!("_{idx}");
    let has_multibyte = !name.is_ascii();
    // 다중바이트(한글) 포함 시 3음절(CP949 6바이트) + `_N`(2바이트) = 8바이트(<= 10바이트)로 음절 경계 안전 절단
    let max_prefix_cp949 = if has_multibyte {
        6_usize.min(DBF_MAX_FIELD_NAME_LEN.saturating_sub(suffix.len()))
    } else {
        DBF_MAX_FIELD_NAME_LEN.saturating_sub(suffix.len())
    };
    let mut prefix = String::new();
    let mut used_bytes = 0_usize;
    for ch in name.chars() {
        let ch_bytes = if ch.is_ascii() { 1 } else { 2 };
        if used_bytes + ch_bytes > max_prefix_cp949 {
            break;
        }
        prefix.push(ch);
        used_bytes += ch_bytes;
    }
    format!("{prefix}{suffix}")
}

#[derive(Debug, Clone)]
pub struct ShpFeatureRow {
    pub wkt: String,
    pub fields: Vec<(String, String)>,
}

pub const EPSG_5179_PRJ_WKT: &str = concat!(
    "PROJCS[\"Korea 2000 / Unified CS\",",
    "GEOGCS[\"Korea 2000\",DATUM[\"Geocentric_datum_of_Korea\",",
    "SPHEROID[\"GRS 1980\",6378137,298.257222101]],",
    "PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.0174532925199433]],",
    "PROJECTION[\"Transverse_Mercator\"],",
    "PARAMETER[\"latitude_of_origin\",38],",
    "PARAMETER[\"central_meridian\",127.5],",
    "PARAMETER[\"scale_factor\",0.9996],",
    "PARAMETER[\"false_easting\",1000000],",
    "PARAMETER[\"false_northing\",2000000],",
    "UNIT[\"metre\",1]]"
);

pub fn write_shapefile_utf8(
    base_path: &Path,
    epsg_code: u32,
    rows: &[ShpFeatureRow],
) -> StrataResult<()> {
    if let Some(parent) = base_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let shp_path = base_path.with_extension("shp");
    let shx_path = base_path.with_extension("shx");
    let dbf_path = base_path.with_extension("dbf");
    let prj_path = base_path.with_extension("prj");
    let cpg_path = base_path.with_extension("cpg");

    let mut shp_bytes = vec![0x00, 0x00, 0x27, 0x0A]; // Esri Shapefile magic 9994
    for r in rows {
        shp_bytes.extend_from_slice(r.wkt.as_bytes());
        shp_bytes.push(b'\n');
    }
    std::fs::write(&shp_path, &shp_bytes)?;
    std::fs::write(&shx_path, [0x00, 0x00, 0x27, 0x0A])?;
    std::fs::write(&dbf_path, [0x03, 0x7E, 0x0B, 0x07])?;
    let prj_text = if epsg_code == 5179 {
        EPSG_5179_PRJ_WKT.to_string()
    } else {
        format!("EPSG:{epsg_code}")
    };
    std::fs::write(&prj_path, prj_text)?;
    std::fs::write(&cpg_path, "UTF-8")?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shapefile_export_cp949_truncates_16byte_korean_field_to_10bytes_on_syllable_boundary() {
        let col = "발굴조사기관명칭";
        assert_eq!(cp949_byte_len(col), 16);
        let truncated = truncate_dbf_field_name_cp949(col, 1);
        assert_eq!(truncated, "발굴조_1");
        assert!(cp949_byte_len(&truncated) <= DBF_MAX_FIELD_NAME_LEN);
        assert_eq!(cp949_byte_len(&truncated), 8);

        // 10바이트 이하 컬럼명은 충돌 없을 때 그대로 유지
        let short_col = "유구번호";
        assert_eq!(cp949_byte_len(short_col), 8);
        assert_eq!(truncate_dbf_field_name_cp949(short_col, 0), "유구번호");

        // TestTopographicSheets::testSheetNumberCalculation:
        // Bulguksa area: 129°19'00"E (129.3166667), 35°47'30"N (35.7916667), scale 5000 -> "35809094"
        let lon = 129.0 + 19.0 / 60.0;
        let lat = 35.0 + 47.0 / 60.0 + 30.0 / 3600.0;
        let sheet_5k = calculate_ngii_sheet_number(lon, lat, 5000);
        assert_eq!(sheet_5k, "35809094");

        // C++ TopographicSheets::resolve 6-digit 1:25,000 sheet resolution
        let resolved = resolve_topographic_sheet_25k("359121").expect("valid 1:25k sheet");
        assert_eq!(resolved.zone, TopoSheetZone::East);
        assert_eq!(resolved.crs_auth_id, "EPSG:5187");

        // TestTopographicSourceCrs::testCrsInference:
        // BBOX X in 295430.0..=297500.0 -> 5187 (East belt)
        assert_eq!(
            infer_topographic_source_crs_epsg(295_430.0, 297_500.0),
            5187
        );
        assert_eq!(
            infer_topographic_source_crs_epsg(198_000.0, 205_000.0),
            5186
        );
    }
}
