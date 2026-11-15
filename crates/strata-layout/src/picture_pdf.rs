//! A PDF of one page that is one picture: a sheet as the app draws it, at print resolution. The picture goes
//! in as the JPEG it is (`/DCTDecode`), so nothing is packed twice (PDF 1.4, ISO 32000-1 §7.5 file structure,
//! §8.9.5 image dictionaries).

use crate::sheet_pdf_export::mm_to_pdf_points;

/// The bytes of a PDF with one page of `paper_mm` (width, height) that the JPEG `jpeg` of `picture_px`
/// (width, height) fills from edge to edge. `title` is what a reader shows as the title of the document.
#[must_use]
pub fn picture_page_pdf(paper_mm: [f64; 2], title: &str, jpeg: &[u8], picture_px: [u32; 2]) -> Vec<u8> {
    let (wide, high) = (mm_to_pdf_points(paper_mm[0]), mm_to_pdf_points(paper_mm[1]));
    // The picture is one unit square: the matrix stretches it over the page.
    let paint = format!("q {wide:.3} 0 0 {high:.3} 0 0 cm /Im0 Do Q\n");
    // Letters outside ASCII are written as UTF-16BE with its mark, in hexadecimal (§7.9.2.2).
    let title_hex: String = std::iter::once(0xFEFF).chain(title.encode_utf16()).map(|unit| format!("{unit:04X}")).collect();
    let mut picture = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        picture_px[0],
        picture_px[1],
        jpeg.len()
    )
    .into_bytes();
    picture.extend_from_slice(jpeg);
    picture.extend_from_slice(b"\nendstream");
    let objects: [Vec<u8>; 6] = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {wide:.3} {high:.3}] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>").into_bytes(),
        format!("<< /Length {} >>\nstream\n{paint}endstream", paint.len()).into_bytes(),
        picture,
        format!("<< /Title <{title_hex}> /Producer (Strata) >>").into_bytes(),
    ];
    // The second line holds bytes above 127: a reader then takes the file as binary (§7.5.2).
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut starts = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        starts.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let table = out.len();
    // Every row of the table is 20 bytes: ten digits, five digits, one letter, and a two-byte line end (§7.5.4).
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for start in starts {
        out.extend_from_slice(format!("{start:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{table}\n%%EOF\n", objects.len() + 1).as_bytes());
    out
}

#[cfg(test)]
#[path = "picture_pdf_tests.rs"]
mod tests;
