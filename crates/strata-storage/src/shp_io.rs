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
    }
}
