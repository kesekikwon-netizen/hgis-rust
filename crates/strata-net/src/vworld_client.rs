#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VworldError {
    InvalidKey,
    QuotaExceeded,
    ServiceException,
}

#[derive(Debug, Clone, Copy)]
pub struct VworldClient;

impl VworldClient {
    pub const REFERER_HEADER: &'static str = "https://localhost";
    pub const CADASTRAL_CRS_CANDIDATES: [&'static str; 3] =
        ["EPSG:4326", "EPSG:3857", "EPSG:900913"];

    pub fn satellite_wmts_url(api_key: &str, z: u32, y: u32, x: u32) -> String {
        let key = api_key.trim();
        if !key.is_empty() {
            format!("https://api.vworld.kr/req/wmts/1.0.0/{key}/Satellite/{z}/{y}/{x}.jpeg")
        } else {
            format!("https://xdworld.vworld.kr:8080/2d/Satellite/service/{z}/{x}/{y}.jpeg")
        }
    }

    pub fn cadastral_wms_url(api_key: &str, bbox: [f64; 4], crs: &str) -> String {
        let valid_crs = if Self::CADASTRAL_CRS_CANDIDATES.contains(&crs) {
            crs
        } else {
            "EPSG:3857"
        };
        format!(
            "https://api.vworld.kr/req/wms?SERVICE=WMS&REQUEST=GetMap&VERSION=1.3.0\
             &LAYERS=lp_pa_cbnd_bubun,lp_pa_cbnd_bonbun\
             &STYLES=lp_pa_cbnd_bubun,lp_pa_cbnd_bonbun\
             &FORMAT=image/png&TRANSPARENT=TRUE&WIDTH=512&HEIGHT=512\
             &crs={valid_crs}&BBOX={:.3},{:.3},{:.3},{:.3}&KEY={}",
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            api_key.trim()
        )
    }

    pub fn is_xml_error_tile(bytes: &[u8]) -> bool {
        Self::classify_service_exception(bytes).is_some()
    }

    /// C++ `VWorldClient` & `TileFetcher` — HTTP 200 내 OGC `ServiceExceptionReport` XML 감지 및 `VWorldError` 분류 1:1
    pub fn classify_service_exception(bytes: &[u8]) -> Option<VworldError> {
        let prefix_len = bytes.len().min(512);
        let Ok(prefix) = std::str::from_utf8(&bytes[..prefix_len]) else {
            return None;
        };
        let trimmed = prefix.trim_start();
        if !trimmed.starts_with("<?xml")
            && !trimmed.contains("ServiceExceptionReport")
            && !trimmed.contains("<ServiceException")
            && !trimmed.contains("<ERROR")
        {
            return None;
        }
        let upper = trimmed.to_uppercase();
        if upper.contains("INCORRECT_KEY")
            || upper.contains("INVALID KEY")
            || upper.contains("INVALID_KEY")
        {
            Some(VworldError::InvalidKey)
        } else if upper.contains("QUOTA") || upper.contains("OVER_LIMIT") {
            Some(VworldError::QuotaExceeded)
        } else {
            Some(VworldError::ServiceException)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_cadastral_wms_url_never_sends_domain_and_detects_xml_error_on_http_200() {
        let url = VworldClient::cadastral_wms_url(
            "TEST_KEY",
            [14135000.0, 4518000.0, 14136000.0, 4519000.0],
            "EPSG:3857",
        );
        assert!(url.contains("crs=EPSG:3857"));
        assert!(url.contains("KEY=TEST_KEY"));
        assert!(!url.to_lowercase().contains("domain="));
        let xml_body = b"<?xml version=\"1.0\"?><ServiceExceptionReport><ServiceException code=\"INCORRECT_KEY\">Invalid Key</ServiceException></ServiceExceptionReport>";
        assert!(VworldClient::is_xml_error_tile(xml_body));
        assert_eq!(
            VworldClient::classify_service_exception(xml_body),
            Some(VworldError::InvalidKey)
        );
    }
}
