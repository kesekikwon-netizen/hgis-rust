//! How the rows of a sheet's legend flow into columns (`target/cpp-ref/src/core/LayoutService.cpp:1458-1534`
//! flowSheetLegend): a legend with many rows (the numbered heritage sites) uses its width before its height.

#[cfg(test)]
#[path = "gui_tab_drawing_legend_flow_tests.rs"]
mod tests;

/// The margin inside the legend's frame and the space between two columns, in millimetres (`:1499-1502`).
const BOX_SPACE_MM: f32 = 1.0;
const COLUMN_SPACE_MM: f32 = 1.5;

/// The number of columns of a legend `width_mm` wide with letters of `font_pt` and `rows` rows: as many as
/// fit with a column of 32 mm at 9 pt (a number badge and a short name), never more than there are rows
/// (`:1506-1514`).
pub(crate) fn legend_columns(width_mm: f32, font_pt: f32, rows: usize) -> usize {
    let column_mm = 32.0 * font_pt.max(5.0) / 9.0;
    let available = (width_mm - 2.0 * BOX_SPACE_MM).max(1.0);
    // A count of columns: small and never negative.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let fit = ((available + COLUMN_SPACE_MM) / (column_mm + COLUMN_SPACE_MM)).floor().max(1.0) as usize;
    fit.clamp(1, rows.max(1))
}

/// How many rows the tallest column holds.
pub(crate) fn legend_column_rows(rows: usize, columns: usize) -> usize {
    rows.div_ceil(columns.max(1))
}

/// The column of a row and its place in that column: the rows fill a column before the next begins.
pub(crate) fn legend_cell(row: usize, rows: usize, columns: usize) -> (usize, usize) {
    let per_column = legend_column_rows(rows, columns).max(1);
    (row / per_column, row % per_column)
}
