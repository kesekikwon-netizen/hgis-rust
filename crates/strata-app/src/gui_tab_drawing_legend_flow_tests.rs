//! How the rows of a sheet's legend flow into columns (`LayoutService.cpp:1458-1534` flowSheetLegend).

use super::*;

/// The columns of a legend: as many as fit with a column of 32 mm at 9 pt (wider with larger letters,
/// counted as 5 pt at least), 1.5 mm between columns and 1 mm of margin on both sides; never more than
/// there are rows, and one at least.
#[test]
fn a_wide_legend_flows_into_columns() {
    assert_eq!(legend_columns(60.0, 10.0, 40), 1);
    assert_eq!(legend_columns(100.0, 10.0, 40), 2);
    assert_eq!(legend_columns(100.0, 9.0, 40), 2);
    assert_eq!(legend_columns(104.0, 9.0, 40), 3);
    assert_eq!(legend_columns(200.0, 10.0, 2), 2, "never more columns than rows");
    assert_eq!(legend_columns(200.0, 10.0, 0), 1);
    assert_eq!(legend_columns(10.0, 10.0, 40), 1);
    assert_eq!(legend_columns(100.0, 3.0, 40), 5, "letters count as 5 pt at least");
    assert_eq!(legend_columns(100.0, 3.0, 40), legend_columns(100.0, 5.0, 40));
}

/// The rows fill a column before the next begins.
#[test]
fn rows_fill_a_column_before_the_next() {
    assert_eq!((legend_column_rows(7, 3), legend_column_rows(7, 1), legend_column_rows(0, 3), legend_column_rows(6, 3)), (3, 7, 0, 2));
    let cells: Vec<(usize, usize)> = (0..7).map(|row| legend_cell(row, 7, 3)).collect();
    assert_eq!(cells, [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (2, 0)]);
    assert_eq!(legend_cell(4, 5, 1), (0, 4));
}
