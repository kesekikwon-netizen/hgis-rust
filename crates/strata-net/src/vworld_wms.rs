//! VWorld WMTS·WMS 요청 주소와 오류 판별 (C++ `BasemapOps.cpp` VWorld 계열).
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


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

    /// C++ `BasemapOps.cpp:1332-1335` `cadastralWmsCrsCandidates` — 4326 → 3857 → 900913 만.
    /// 5186/5187/5179 는 넣지 않는다(Caps 가 4326 bbox 만 알려 주어 레이어가 그려지지 않는다).
    pub const fn cadastral_wms_crs_candidates() -> [&'static str; 3] {
        Self::CADASTRAL_CRS_CANDIDATES
    }

    /// C++ `BasemapOps.cpp:1189-1212` `makeVworldWmsUri` — 지적 레이어 URI.
    ///
    /// 인증은 `http-header:referer` 가 하고, `DOMAIN` 은 보내지 않는다(VWorld 가 같은 키라도
    /// `INCORRECT_KEY` 로 거절한다 — BasemapOps.cpp:1195-1200 실측). `crs` 후보 밖 값은 3857 로
    /// 떨어뜨린다(C++ 과 같은 기본값). `styles` 가 비면 `styles` 파라미터만 이름으로 남는다.
    #[must_use]
    pub fn cadastral_layer_uri(api_key: &str, layers: &str, styles: &str, crs: &str) -> String {
        let trimmed_crs = crs.trim();
        let valid_crs = if Self::CADASTRAL_CRS_CANDIDATES.contains(&trimmed_crs) {
            trimmed_crs
        } else {
            "EPSG:3857"
        };
        let base_url = format!(
            "https://api.vworld.kr/req/wms?KEY={}",
            api_key.trim()
        );
        let enc_url = percent_encode(&base_url);
        let style_part = if styles.is_empty() {
            "styles".to_owned()
        } else {
            format!("styles={styles}")
        };
        format!(
            "IgnoreGetMapUrl=1&IgnoreGetFeatureInfoUrl=1&contextualWMSLegend=0\
             &crs={valid_crs}&dpiMode=7&format=image/png&transparent=true&featureCount=10\
             &tilePixelRatio=2&stepWidth=512&stepHeight=512\
             &http-header:referer={}\
             &layers={layers}&{style_part}&url={enc_url}",
            Self::REFERER_HEADER
        )
    }

    /// C++ `BasemapOps.cpp:1261-1268` GDAL_WMS 설정 XML (지적 WMS 폴백).
    /// `domain=` 은 넣지 않고 `<Referer>` 로 인증한다.
    #[must_use]
    pub fn cadastral_gdal_wms_xml(api_key: &str, layers: &str) -> String {
        format!(
            "<GDAL_WMS><Service name=\"WMS\"><Version>1.3.0</Version>\
             <ServerUrl>https://api.vworld.kr/req/wms?key={}&amp;</ServerUrl>\
             <Layers>{layers}</Layers>\
             <Styles>lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun</Styles>\
             <CRS>EPSG:3857</CRS>\
             <ImageFormat>image/png</ImageFormat>\
             <Transparent>TRUE</Transparent>\
             <BBoxOrder>xyXY</BBoxOrder>\
             </Service>\
             <DataWindow>\
             <UpperLeftX>13500000</UpperLeftX><UpperLeftY>4800000</UpperLeftY>\
             <LowerRightX>14800000</LowerRightX><LowerRightY>3800000</LowerRightY>\
             <SizeX>2600000</SizeX><SizeY>2000000</SizeY>\
             </DataWindow>\
             <Projection>EPSG:3857</Projection>\
             <BandsCount>4</BandsCount>\
             <BlockSizeX>512</BlockSizeX><BlockSizeY>512</BlockSizeY>\
             <UserAgent>Mozilla/5.0 ka-hgis/0.3</UserAgent>\
             <Referer>{}</Referer>\
             </GDAL_WMS>",
            api_key.trim(),
            Self::REFERER_HEADER
        )
    }
}

/// C++ `QUrl::toPercentEncoding` 과 같은 규칙(비예약 문자만 그대로)
pub(super) fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
