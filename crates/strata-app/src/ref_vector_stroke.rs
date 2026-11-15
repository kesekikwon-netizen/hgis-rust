//! Thin lines of a reference layer, drawn fast.
//!
//! Measured on the four real sheets of one survey (333 390 lines, picture 3045 x 1589): the general
//! rasteriser needed 1.7 to 3.9 s for the lines at 1:25 000 and 1:100 000, because it sets up every one of
//! some million short pieces on its own. Map lines here are 0.1 to 0.3 mm wide, one or two pixels, so their
//! coverage can be written straight into a mask: a pixel is covered by how far its centre lies inside the
//! line, `clamp(half width + 0.5 - distance, 0, 1)` (a box filter one pixel wide over the distance to the
//! segment; lines thinner than a pixel are drawn one pixel wide and lighter, as the rasteriser does).
//! Pieces of one line, and lines that cross, combine by the larger value, so joints are not darkened twice.
//! Ends and joints come out round; the C++ look ends square and joins bevelled, which differs by less than
//! a pixel at these widths. Cost: about (length + 2)(width + 2) pixel tests for a piece; the picture is
//! split into bands of rows that are filled on several threads.

#[cfg(test)]
#[path = "ref_vector_stroke_tests.rs"]
mod tests;

/// Pieces longer than this are cut up, so that the box searched around a slanted piece stays small (pixels).
const PIECE: f32 = 8.0;

/// Lines as picture points: `starts[i]..starts[i + 1]` are the points of line `i`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Polylines {
    pub(crate) points: Vec<[f32; 2]>,
    pub(crate) starts: Vec<u32>,
}

impl Polylines {
    /// The lines, each as its points.
    pub(crate) fn lines(&self) -> impl Iterator<Item = &[[f32; 2]]> {
        let ends = self.starts.iter().skip(1).copied().chain(std::iter::once(self.points.len() as u32));
        self.starts.iter().zip(ends).map(|(start, end)| &self.points[*start as usize..end as usize])
    }
}

/// Adds one piece to the rows `first_row..` held by `rows` (a band of the mask, `width` pixels to the row).
fn add_piece(rows: &mut [u8], width: usize, first_row: usize, a: [f32; 2], z: [f32; 2], reach: f32, gain: f32) {
    let row_count = rows.len() / width;
    // The pixels whose centres can lie within `reach` of the piece.
    let (x_low, x_high) = ((a[0].min(z[0]) - reach - 0.5).ceil().max(0.0), (a[0].max(z[0]) + reach - 0.5).floor().min(width as f32 - 1.0));
    let (y_low, y_high) = ((a[1].min(z[1]) - reach - 0.5).ceil().max(first_row as f32), (a[1].max(z[1]) + reach - 0.5).floor().min((first_row + row_count) as f32 - 1.0));
    if x_low > x_high || y_low > y_high {
        return;
    }
    let (dx, dy) = (z[0] - a[0], z[1] - a[1]);
    let length_squared = dx * dx + dy * dy;
    let inverse = if length_squared > 0.0 { 1.0 / length_squared } else { 0.0 };
    // In range by the clamps above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (x_low, x_high, y_low, y_high) = (x_low as usize, x_high as usize, y_low as usize, y_high as usize);
    for y in y_low..=y_high {
        let py = y as f32 + 0.5;
        let row = &mut rows[(y - first_row) * width..(y - first_row + 1) * width];
        for (x, cell) in row.iter_mut().enumerate().take(x_high + 1).skip(x_low) {
            let px = x as f32 + 0.5;
            let along = (((px - a[0]) * dx + (py - a[1]) * dy) * inverse).clamp(0.0, 1.0);
            let (ex, ey) = (a[0] + along * dx - px, a[1] + along * dy - py);
            let distance_squared = ex * ex + ey * ey;
            if distance_squared < reach * reach {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let cover = ((reach - distance_squared.sqrt()).min(1.0) * gain * 255.0 + 0.5) as u8;
                *cell = (*cell).max(cover);
            }
        }
    }
}

/// Adds every line to one band of the mask.
fn fill_band(rows: &mut [u8], width: usize, first_row: usize, lines: &Polylines, reach: f32, gain: f32) {
    let (top, bottom) = (first_row as f32 - reach, (first_row + rows.len() / width) as f32 + reach);
    for line in lines.lines() {
        if let [only] = line {
            add_piece(rows, width, first_row, *only, *only, reach, gain);
        }
        for pair in line.windows(2) {
            let (a, z) = (pair[0], pair[1]);
            if a[1].max(z[1]) < top || a[1].min(z[1]) > bottom {
                continue;
            }
            let (dx, dy) = (z[0] - a[0], z[1] - a[1]);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let pieces = (dx.abs().max(dy.abs()) / PIECE).ceil().max(1.0) as u32;
            for i in 0..pieces {
                let (from, to) = (i as f32 / pieces as f32, (i + 1) as f32 / pieces as f32);
                add_piece(rows, width, first_row, [a[0] + dx * from, a[1] + dy * from], [a[0] + dx * to, a[1] + dy * to], reach, gain);
            }
        }
    }
}

/// How much each pixel of a `width` x `height` picture is covered by these lines drawn `line_width` pixels
/// wide: 0 to 255, row by row. `threads` bands are filled side by side.
pub(crate) fn coverage(lines: &Polylines, width: usize, height: usize, line_width: f32, threads: usize) -> Vec<u8> {
    let mut mask = vec![0_u8; width * height];
    if width == 0 || height == 0 || !line_width.is_finite() || line_width <= 0.0 {
        return mask;
    }
    // Thinner than a pixel: one pixel wide, lighter.
    let (reach, gain) = (line_width.max(1.0) / 2.0 + 0.5, line_width.min(1.0));
    let band_rows = height.div_ceil(threads.clamp(1, height));
    std::thread::scope(|scope| {
        for (band, rows) in mask.chunks_mut(band_rows * width).enumerate() {
            scope.spawn(move || fill_band(rows, width, band * band_rows, lines, reach, gain));
        }
    });
    mask
}

/// Lays `colour` (straight RGBA) over a premultiplied RGBA picture through the mask.
pub(crate) fn lay_over(picture: &mut [u8], mask: &[u8], colour: [u8; 4]) {
    for (pixel, cover) in picture.chunks_exact_mut(4).zip(mask).filter(|(_, cover)| **cover > 0) {
        // Alpha of the line at this pixel, 0..=255, then source-over on premultiplied values.
        let alpha = (u32::from(colour[3]) * u32::from(*cover) + 127) / 255;
        for channel in 0..3 {
            let source = (u32::from(colour[channel]) * alpha + 127) / 255;
            pixel[channel] = u8::try_from(source + (u32::from(pixel[channel]) * (255 - alpha) + 127) / 255).unwrap_or(u8::MAX);
        }
        pixel[3] = u8::try_from(alpha + (u32::from(pixel[3]) * (255 - alpha) + 127) / 255).unwrap_or(u8::MAX);
    }
}
