use geo::algorithm::bool_ops::BooleanOps;
use geo::{Coord, Distance, Euclidean, LineString, MultiPolygon, Point, Polygon};
use std::f64::consts::TAU;

pub fn round_buffer_polygon(poly: &Polygon<f64>, radius_m: f64, segments: usize) -> Polygon<f64> {
    if radius_m <= 0.0 {
        return poly.clone();
    }
    let segs = segments.max(16);
    let mut acc = MultiPolygon(vec![poly.clone()]);
    let coords: Vec<Coord<f64>> = poly.exterior().0.clone();
    for &c in &coords {
        let disk = circle_polygon(c.x, c.y, radius_m, segs);
        acc = acc.union(&MultiPolygon(vec![disk]));
    }
    for win in coords.windows(2) {
        let a = win[0];
        let b = win[1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-9 {
            let nx = -dy / len * radius_m;
            let ny = dx / len * radius_m;
            let rect = Polygon::new(
                LineString::from(vec![
                    Coord { x: a.x + nx, y: a.y + ny },
                    Coord { x: b.x + nx, y: b.y + ny },
                    Coord { x: b.x - nx, y: b.y - ny },
                    Coord { x: a.x - nx, y: a.y - ny },
                    Coord { x: a.x + nx, y: a.y + ny },
                ]),
                vec![],
            );
            acc = acc.union(&MultiPolygon(vec![rect]));
        }
    }
    acc.0.into_iter().next().unwrap_or_else(|| poly.clone())
}

fn circle_polygon(cx: f64, cy: f64, r: f64, segments: usize) -> Polygon<f64> {
    let mut pts = Vec::with_capacity(segments + 1);
    for i in 0..segments {
        let theta = TAU * (i as f64) / (segments as f64);
        pts.push(Coord {
            x: cx + r * theta.cos(),
            y: cy + r * theta.sin(),
        });
    }
    if let Some(&first) = pts.first() {
        pts.push(first);
    }
    Polygon::new(LineString::from(pts), vec![])
}

pub fn filter_within_euclidean_radius<'a, T>(
    site: &Polygon<f64>,
    candidates: &'a [(Polygon<f64>, T)],
    radius_m: f64,
) -> Vec<&'a T> {
    candidates
        .iter()
        .filter_map(|(geom, item)| {
            if Euclidean::distance(site, geom) <= radius_m {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}

pub fn point_within_euclidean_radius(site: &Polygon<f64>, pt: [f64; 2], radius_m: f64) -> bool {
    Euclidean::distance(site, &Point::new(pt[0], pt[1])) <= radius_m
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn round_buffer_500m_has_no_miter_corner_spikes() {
        let site = wkt!(POLYGON((0.0 0.0, 100.0 0.0, 100.0 100.0, 0.0 100.0, 0.0 0.0)));
        let ring = round_buffer_polygon(&site, 500.0, 36);
        assert!(ring.exterior().0.len() > 20);
        for coord in ring.exterior().coords() {
            let pt = geo::Point::new(coord.x, coord.y);
            let dist = Euclidean::distance(&site, &pt);
            assert!(
                (dist - 500.0).abs() < 2.5,
                "corner distance {dist} deviated from 500m"
            );
        }
    }
}
