#[derive(Debug, Clone, Copy)]
pub struct KigamPaleoService;

impl KigamPaleoService {
    pub const FAULT_WFS_TYPENAME: &'static str = "geoOpen:l_50k_geology_fault_latest";
    pub const UNDERWATER_WFS_TYPENAME: &'static str = "GeoEnv:underwater_2019_avg";
    pub const UPLIFT_WFS_TYPENAME: &'static str = "GeoEnv:nation_upliftrate_2023";

    pub fn build_wfs_url(type_name: &str, bbox_4326: [f64; 4]) -> String {
        format!(
            "https://data.kigam.re.kr/openapi/wfs?SERVICE=WFS&VERSION=1.1.0\
             &REQUEST=GetFeature&TYPENAME={type_name}&OUTPUTFORMAT=application/json\
             &SRSNAME=EPSG:4326&BBOX={:.5},{:.5},{:.5},{:.5},EPSG:4326",
            bbox_4326[0], bbox_4326[1], bbox_4326[2], bbox_4326[3]
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SoilMapService;

#[derive(Debug, Clone, Copy)]
pub struct GeologyMapService;

#[derive(Debug, Clone, Copy)]
pub struct RiverMapService;
