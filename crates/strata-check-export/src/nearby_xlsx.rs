use std::io::Write;
use std::path::Path;

use strata_types::error::{StrataError, StrataResult};
use zip::write::SimpleFileOptions;

pub fn export_nearby_heritage_xlsx(
    path: &Path,
    rows: &[(u32, String, String, f64)],
) -> StrataResult<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts)
        .map_err(|e| StrataError::Export(e.to_string()))?;
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
          <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
          <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
          </Types>",
    )?;

    zip.start_file("xl/worksheets/sheet1.xml", opts)
        .map_err(|e| StrataError::Export(e.to_string()))?;
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet><sheetData>");
    for (badge_no, name, era, dist_m) in rows {
        xml.push_str(&format!(
            "<row><c>{badge_no}</c><c>{name}</c><c>{era}</c><c>{dist_m:.1}</c></row>"
        ));
    }
    xml.push_str("</sheetData></worksheet>");
    zip.write_all(xml.as_bytes())?;
    zip.finish()
        .map_err(|e| StrataError::Export(e.to_string()))?;
    Ok(())
}
