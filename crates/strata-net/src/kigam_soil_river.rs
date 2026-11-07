#[derive(Debug, Clone, Copy)]
pub struct KigamPaleoService;

impl KigamPaleoService {
    pub const FAULT_WFS_TYPENAME: &'static str = "geoOpen:l_50k_geology_fault_latest";
    pub const UNDERWATER_WFS_TYPENAME: &'static str = "GeoEnv:underwater_2019_avg";
    pub const UPLIFT_WFS_TYPENAME: &'static str = "GeoEnv:nation_upliftrate_2023";
    pub const HGIS_KIGAM_BUFFER_RADIUS_M: f64 = 10_000.0;

    pub fn build_wfs_url(type_name: &str, bbox_4326: [f64; 4]) -> String {
        format!(
            "https://data.kigam.re.kr/openapi/wfs?SERVICE=WFS&VERSION=1.1.0\
             &REQUEST=GetFeature&TYPENAME={type_name}&OUTPUTFORMAT=application/json\
             &SRSNAME=EPSG:4326&BBOX={:.5},{:.5},{:.5},{:.5},EPSG:4326",
            bbox_4326[0], bbox_4326[1], bbox_4326[2], bbox_4326[3]
        )
    }

    /// C++ `KigamSoilRiverClient::parseRiverPolygons` — 조사구역 중심 기준 반경 10km BBOX 교차 필터링 및 4점 미만 퇴화 링 배제 1:1
    #[must_use]
    pub fn filter_river_polygons_within_10km(
        polygons: &[Vec<[f64; 2]>],
        center_xy: [f64; 2],
    ) -> Vec<Vec<[f64; 2]>> {
        let r = Self::HGIS_KIGAM_BUFFER_RADIUS_M;
        let filter_min_x = center_xy[0] - r;
        let filter_max_x = center_xy[0] + r;
        let filter_min_y = center_xy[1] - r;
        let filter_max_y = center_xy[1] + r;

        let mut kept = Vec::new();
        for ring in polygons {
            if ring.len() < 4 {
                continue;
            }
            let mut min_x = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            for &[x, y] in ring {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
            let intersects = !(max_x < filter_min_x
                || min_x > filter_max_x
                || max_y < filter_min_y
                || min_y > filter_max_y);
            if intersects {
                kept.push(ring.clone());
            }
        }
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kigam_river_polygons_10km_bbox_filter_prunes_distant_and_degenerate_rings() {
        let center = [200_000.0, 500_000.0];
        let inside_poly = vec![
            [205_000.0, 505_000.0],
            [205_100.0, 505_000.0],
            [205_100.0, 505_100.0],
            [205_000.0, 505_000.0],
        ];
        let outside_poly = vec![
            [225_000.0, 525_000.0],
            [225_100.0, 525_000.0],
            [225_100.0, 525_100.0],
            [225_000.0, 525_000.0],
        ];
        let degenerate_poly = vec![[200_000.0, 500_000.0], [200_010.0, 500_010.0]];
        let input = vec![inside_poly.clone(), outside_poly, degenerate_poly];
        let filtered = KigamPaleoService::filter_river_polygons_within_10km(&input, center);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], inside_poly);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SoilMapService;

#[derive(Debug, Clone, Copy)]
pub struct GeologyMapService;

#[derive(Debug, Clone, Copy)]
pub struct RiverMapService;
