//! Writes one in-memory table as an EPSG:5179 shapefile like the cpp package (`ExportService.cpp:164-176`).
//! DBF columns per kind as in the cpp truth sets (tests/fixtures/cpp_submit_types_*): Integer N(10,0),
//! Integer64 N(20,0), Real N(24,15), Date D(8), DateTime C(24), Boolean L(1), TEXT(n) C(n), TEXT C(254).

use std::path::Path;

use gdal::spatial_ref::{AxisMappingStrategy, CoordTransform, SpatialRef};
use gdal::vector::{Feature, FieldDefn, LayerAccess, LayerOptions, OGRFieldType};
use gdal::DriverManager;

use super::submit_shp_read::{Kind, Table, Value};
use super::SubmitEncoding;

/// Writes `table` with the SHP field `names` (same order as `table.fields`); the error is GDAL's message.
pub(crate) fn write_table(key: &str, table: &Table, names: &[String], shp: &Path, enc: SubmitEncoding) -> Result<(), String> {
    let mut dst_srs = SpatialRef::from_epsg(5179).map_err(|e| e.to_string())?;
    dst_srs.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    let ct = CoordTransform::new(&table.srs, &dst_srs).map_err(|e| e.to_string())?;
    // ExportService.cpp:117-119 — EUC-KR / CP949 -> CP949, else UTF-8. The cpp EUC-KR packages carry
    // .cpg = 949 (cpp_submit_types_euckr, cpp_submit_bad_euckr_5186), so OGR gets ENCODING=949.
    let encoding = match enc {
        SubmitEncoding::Utf8 => "ENCODING=UTF-8",
        SubmitEncoding::EucKr => "ENCODING=949",
    };
    // The cpp UTF-8 package cuts an over-long value at the byte width, inside a character
    // (cpp_submit_types_utf8 c_long); OGR does that when SHAPE_ENCODING is empty and the text goes in as is.
    // For CP949, OGR's recoding (ENCODING=949) gives the cpp bytes, cut included (cpp_submit_long_euckr).
    // Thread-local, cleared right after.
    let raw_text = enc == SubmitEncoding::Utf8;
    if raw_text {
        gdal::config::set_thread_local_config_option("SHAPE_ENCODING", "").map_err(|e| e.to_string())?;
    }
    let written = write_shapefile(key, table, names, shp, encoding, &dst_srs, &ct);
    if raw_text {
        gdal::config::clear_thread_local_config_option("SHAPE_ENCODING").map_err(|e| e.to_string())?;
    }
    written?;
    let logical: Vec<usize> =
        table.fields.iter().enumerate().filter(|(_, f)| f.kind == Kind::Boolean).map(|(i, _)| i).collect();
    if logical.is_empty() {
        Ok(())
    } else {
        mark_logical(&shp.with_extension("dbf"), &logical)
    }
}

/// Creates the shapefile, its columns and features; GDAL closes it on return.
fn write_shapefile(
    key: &str,
    table: &Table,
    names: &[String],
    shp: &Path,
    encoding: &str,
    dst_srs: &SpatialRef,
    ct: &CoordTransform,
) -> Result<(), String> {
    {
        let driver = DriverManager::get_driver_by_name("ESRI Shapefile").map_err(|e| e.to_string())?;
        let mut out = driver.create_vector_only(shp).map_err(|e| e.to_string())?;
        let dl = out
            .create_layer(LayerOptions { name: key, srs: Some(dst_srs), ty: table.geom_type, options: Some(&[encoding]) })
            .map_err(|e| e.to_string())?;
        for (field, name) in table.fields.iter().zip(names) {
            let (ty, width, precision) = column(field.kind, field.width, field.precision);
            let fd = FieldDefn::new(name, ty).map_err(|e| e.to_string())?;
            if let Some(w) = width {
                fd.set_width(w);
                fd.set_precision(precision);
            }
            fd.add_to_layer(&dl).map_err(|e| e.to_string())?;
        }
        for (values, geom) in &table.rows {
            let mut nf = Feature::new(dl.defn()).map_err(|e| e.to_string())?;
            for (i, value) in values.iter().enumerate() {
                match value {
                    // OGR writes '?' for a NULL logical value.
                    Value::Null if table.fields[i].kind == Kind::Boolean => {
                        nf.set_field_string(i, "?").map_err(|e| e.to_string())?;
                    }
                    Value::Null => {}
                    Value::Int(v) if table.fields[i].kind == Kind::Integer => {
                        let v = i32::try_from(*v).map_err(|e| e.to_string())?;
                        nf.set_field_integer(i, v).map_err(|e| e.to_string())?;
                    }
                    Value::Int(v) => nf.set_field_integer64(i, *v).map_err(|e| e.to_string())?,
                    Value::Bool(b) => nf.set_field_string(i, if *b { "T" } else { "F" }).map_err(|e| e.to_string())?,
                    Value::Real(v) => nf.set_field_double(i, *v).map_err(|e| e.to_string())?,
                    Value::Text(s) => nf.set_field_string(i, s).map_err(|e| e.to_string())?,
                }
            }
            if let Some(g) = geom {
                let mut g = g.clone();
                g.transform_inplace(ct).map_err(|e| e.to_string())?;
                nf.set_geometry(g).map_err(|e| e.to_string())?;
            }
            nf.create(&dl).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// The cpp packages hold a boolean as DBF L(1): 'T' (cpp_submit_types_*); OGR writes 'F' / '?' for false / NULL.
/// gdal 0.19 has no safe field-subtype call (and OGR SQL `ADD COLUMN ... boolean` falls back to
/// VARCHAR(80) on a shapefile), so the column is written as C(1) with the same bytes and its descriptor
/// type byte becomes 'L': DBF header 32 bytes, then one 32-byte descriptor per field, type at offset 11.
fn mark_logical(dbf: &Path, fields: &[usize]) -> Result<(), String> {
    let mut bytes = std::fs::read(dbf).map_err(|e| format!("{}: {e}", dbf.display()))?;
    for i in fields {
        let at = 32 + 32 * i + 11;
        match bytes.get_mut(at) {
            Some(ty) if *ty == b'C' => *ty = b'L',
            _ => return Err(format!("{}: field {i} is not the C(1) column written for a boolean", dbf.display())),
        }
    }
    std::fs::write(dbf, bytes).map_err(|e| format!("{}: {e}", dbf.display()))
}

/// OGR column for a kind: type, width (`None` = driver default) and precision, as the cpp truth sets show.
fn column(kind: Kind, width: i32, precision: i32) -> (OGRFieldType::Type, Option<i32>, i32) {
    match kind {
        Kind::Integer => (OGRFieldType::OFTInteger, Some(if width > 0 && width <= 10 { width } else { 10 }), 0),
        Kind::Integer64 => (OGRFieldType::OFTInteger64, Some(if width > 0 && width <= 20 { width } else { 20 }), 0),
        Kind::Real if width > 0 => (OGRFieldType::OFTReal, Some(width), precision),
        Kind::Real => (OGRFieldType::OFTReal, Some(24), 15),
        Kind::Date => (OGRFieldType::OFTDate, None, 0),
        // A shapefile has no date-time column: the cpp packages use text of 24.
        Kind::DateTime => (OGRFieldType::OFTString, Some(24), 0),
        Kind::Boolean => (OGRFieldType::OFTString, Some(1), 0),
        Kind::String => (OGRFieldType::OFTString, Some(if width > 0 && width < 254 { width } else { 254 }), 0),
    }
}
