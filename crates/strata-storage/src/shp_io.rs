use std::path::Path;
use strata_types::error::StrataResult;

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
