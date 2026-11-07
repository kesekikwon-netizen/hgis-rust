#[derive(Debug, Clone, PartialEq)]
pub struct HeritageBadge {
    pub number: u32,
    pub x_mm: f64,
    pub y_mm: f64,
    pub radius_mm: f64,
}

#[derive(Debug, Default, Clone)]
pub struct LayoutBadgePlacer;

impl LayoutBadgePlacer {
    #[must_use]
    pub fn place_non_overlapping_badges(points_mm: &[(u32, f64, f64)], radius_mm: f64) -> Vec<HeritageBadge> {
        let mut placed: Vec<HeritageBadge> = Vec::with_capacity(points_mm.len());
        let min_dist = radius_mm * 2.2;
        for &(number, mut x, mut y) in points_mm {
            for existing in &placed {
                let dx = x - existing.x_mm;
                let dy = y - existing.y_mm;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    x += min_dist - dist + 0.5;
                    y -= 0.5;
                }
            }
            placed.push(HeritageBadge {
                number,
                x_mm: x,
                y_mm: y,
                radius_mm,
            });
        }
        placed
    }
}
