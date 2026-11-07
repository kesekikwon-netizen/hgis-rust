use geo::algorithm::bool_ops::BooleanOps;
use geo::{Coord, Distance, Euclidean, LineString, MultiPolygon, Point, Polygon};
use std::f64::consts::TAU;

pub fn round_buffer_polygon(poly: &Polygon<f64>, radius_m: f64, segments: usize) -> Polygon<f64> {
    if radius_m <= 0.0 || poly.exterior().0.len() < 4 {
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
                    Coord {
                        x: a.x + nx,
                        y: a.y + ny,
                    },
                    Coord {
                        x: b.x + nx,
                        y: b.y + ny,
                    },
                    Coord {
                        x: b.x - nx,
                        y: b.y - ny,
                    },
                    Coord {
                        x: a.x - nx,
                        y: a.y - ny,
                    },
                    Coord {
                        x: a.x + nx,
                        y: a.y + ny,
                    },
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

pub const HGIS_HERITAGE_BUFFER_URBAN_M: f64 = 300.0;
pub const HGIS_HERITAGE_BUFFER_RURAL_M: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeritageBufferEvaluationResult {
    pub min_distance_m: f64,
    pub is_direct_intersect: bool,
    pub is_within_300m: bool,
    pub is_within_500m: bool,
    pub highest_preservation_zone: u8,
}

/// C++ `KaHeritageBufferAnalyzer::evaluateHeritageImpact` — 최단 이격거리 기반 300m(도시)/500m(비도시) 반경 판정 및 역사문화환경 보존지역 1~4구역 최고 규제 등급 산출
#[must_use]
pub fn evaluate_heritage_impact_from_distance(
    min_distance_m: f64,
    overlapped_zone_numbers: &[u8],
) -> HeritageBufferEvaluationResult {
    let highest_zone = overlapped_zone_numbers
        .iter()
        .copied()
        .filter(|&z| (1..=4).contains(&z))
        .min()
        .unwrap_or(0);
    let d = if min_distance_m.is_finite() {
        min_distance_m.max(0.0)
    } else {
        f64::INFINITY
    };
    let is_direct_intersect = d <= 1e-6 || highest_zone > 0;
    let eff_d = if is_direct_intersect { 0.0 } else { d };
    HeritageBufferEvaluationResult {
        min_distance_m: eff_d,
        is_direct_intersect,
        is_within_300m: eff_d <= HGIS_HERITAGE_BUFFER_URBAN_M,
        is_within_500m: eff_d <= HGIS_HERITAGE_BUFFER_RURAL_M,
        highest_preservation_zone: highest_zone,
    }
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

    #[test]
    fn test_buffer_round_500m_expands_100x100_survey_bbox_by_exact_500m_all_sides() {
        use geo::BoundingRect;

        let site = wkt!(POLYGON((1000.0 2000.0, 1100.0 2000.0, 1100.0 2100.0, 1000.0 2100.0, 1000.0 2000.0)));
        let buffered = round_buffer_polygon(&site, 500.0, 64);
        let bbox = buffered.bounding_rect().expect("buffered bbox must exist");
        assert!((bbox.min().x - 500.0).abs() < 1e-3);
        assert!((bbox.min().y - 1500.0).abs() < 1e-3);
        assert!((bbox.max().x - 1600.0).abs() < 1e-3);
        assert!((bbox.max().y - 2600.0).abs() < 1e-3);
    }

    #[test]
    fn test_heritage_buffer_500m_and_300m_boundary_and_zone1_overlap() {
        let res_a = evaluate_heritage_impact_from_distance(250.0, &[]);
        assert!(res_a.is_within_300m);
        assert!(res_a.is_within_500m);
        assert!(!res_a.is_direct_intersect);

        let res_b = evaluate_heritage_impact_from_distance(400.0, &[]);
        assert!(!res_b.is_within_300m);
        assert!(res_b.is_within_500m);

        let res_c = evaluate_heritage_impact_from_distance(12.0, &[3, 1]);
        assert_eq!(res_c.highest_preservation_zone, 1);
        assert_eq!(res_c.min_distance_m, 0.0);
        assert!(res_c.is_direct_intersect);
    }
}
