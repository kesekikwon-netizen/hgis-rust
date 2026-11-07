use geo::{Area, Geometry, Polygon};

pub fn split_polygon_pieces(geom: &Geometry<f64>) -> Vec<Polygon<f64>> {
    let mut out = Vec::new();
    collect_valid_polygons(geom, &mut out);
    out
}

fn collect_valid_polygons(geom: &Geometry<f64>, out: &mut Vec<Polygon<f64>>) {
    match geom {
        Geometry::Polygon(p) => {
            if p.unsigned_area() > 1e-9 {
                out.push(p.clone());
            }
        }
        Geometry::MultiPolygon(mp) => {
            for p in &mp.0 {
                if p.unsigned_area() > 1e-9 {
                    out.push(p.clone());
                }
            }
        }
        Geometry::GeometryCollection(gc) => {
            for item in &gc.0 {
                collect_valid_polygons(item, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn multipolygon_splits_into_non_zero_single_polygons_on_gpkg_write() {
        let mp = wkt!(MULTIPOLYGON(
            ((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)),
            ((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0)),
            ((5.0 5.0, 5.0 5.0, 5.0 5.0, 5.0 5.0))
        ));
        let pieces = split_polygon_pieces(&Geometry::MultiPolygon(mp));
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn test_multipolygon_decompose_3_islands_preserves_exact_count_and_total_area() {
        let mp = wkt!(MULTIPOLYGON(
            ((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)),
            ((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0)),
            ((40.0 40.0, 50.0 40.0, 50.0 45.0, 40.0 45.0, 40.0 40.0))
        ));
        let orig_geom = Geometry::MultiPolygon(mp);
        let orig_area = orig_geom.unsigned_area();
        let pieces = split_polygon_pieces(&orig_geom);
        assert_eq!(pieces.len(), 3);
        let sum_area: f64 = pieces.iter().map(|p| p.unsigned_area()).sum();
        assert!((sum_area - orig_area).abs() < 1e-6);
    }
}
