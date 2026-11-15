//! The drawing scales Strata offers (C++ `StandardScales.cpp`).

/// The smallest standard scale that still holds what a frame shows at 1:`raw`: it never takes a smaller
/// denominator, so nothing is cut off (`snapUp`, StandardScales.cpp:76-85). Beyond the table the largest
/// one, and 0 for a value that is no scale.
#[must_use]
pub fn snap_up(raw: f64) -> u32 {
    if !(raw > 0.0 && raw.is_finite()) {
        return 0;
    }
    SNAP.iter().copied().find(|scale| f64::from(*scale) + 1e-6 >= raw).unwrap_or(SNAP[SNAP.len() - 1])
}

/// The scales the snap may take (`kTable` entries with `Snap`, StandardScales.cpp:25-57): plans and sections
/// of an excavation, cadastral sheets (1:500, 1:600, 1:1,200, 1:3,000) and topographic maps.
const SNAP: [u32; 30] = [
    10, 20, 30, 40, 50, 60, 80, 100, 150, 200, 250, 300, 400, 500, 600, 1000, 1200, 2000, 2500, 3000, 4000, 5000, 10_000, 20_000, 25_000, 40_000, 50_000,
    100_000, 200_000, 500_000,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// ported from tests/test_layout_sheet_rules.cpp:155-165 (standardScalesAreOneTable).
    #[test]
    fn snap_up_takes_the_next_standard_scale() {
        for (raw, nice) in [(23.0, 30), (55.0, 60), (520.0, 600), (1100.0, 1200), (2300.0, 2500), (0.0, 0)] {
            assert_eq!(snap_up(raw), nice, "{raw}");
        }
        for raw in [12.0, 23.0, 37.0, 487.0, 1847.0, 21000.0] {
            let nice = snap_up(raw);
            assert!(f64::from(nice) >= raw && nice.is_multiple_of(10), "{raw} -> {nice}");
        }
    }

    /// A standard scale stays, 1:25 is a scale of sections only, beyond the table the largest one, and what
    /// is no number gives 0.
    #[test]
    fn snap_up_at_the_edges_of_the_table() {
        assert_eq!(snap_up(5000.0), 5000);
        assert_eq!(snap_up(21.0), 30);
        assert_eq!(snap_up(9.0), 10);
        assert_eq!(snap_up(600_000.0), 500_000);
        assert_eq!(snap_up(f64::NAN), 0);
        assert_eq!(snap_up(-5.0), 0);
    }
}
