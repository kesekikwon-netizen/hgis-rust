use crate::address_search::http_get_bytes;

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

    /// C++ `AdminBoundaryService.cpp:50-70` `buildGetFeatureUrl` — 읍면동 경계 GetFeature.
    /// `attrFilter` 는 `full_nm:like:시도 시군구 읍면동`. 값이 비면 파라미터를 넣지 않는다.
    #[must_use]
    pub fn admin_boundary_url(api_key: &str, sido: &str, city: &str, dong: &str) -> String {
        let filter = Self::attr_filter(sido, city, dong);
        let mut url = format!(
            "https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature\
             &format=json&size=10&page=1&geometry=true&attribute=true&crs=EPSG:3857\
             &data={}",
            Self::ADMIN_BOUNDARY_DATASET
        );
        if !filter.is_empty() {
            url.push_str(&format!("&attrFilter={}", percent_encode(&filter)));
        }
        url.push_str(&format!("&key={}", api_key.trim()));
        url
    }

    /// C++ `AdminBoundaryService.cpp:40-48` `attrFilter`
    #[must_use]
    pub fn attr_filter(sido: &str, city: &str, dong: &str) -> String {
        let parts: Vec<&str> = [sido, city, dong]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!("full_nm:like:{}", parts.join(" "))
        }
    }

    /// C++ `AdminBoundaryService.cpp:72-81` `normalizeCrsName`
    #[must_use]
    pub fn normalize_crs_name(raw: &str) -> String {
        if raw.contains("5186") {
            "EPSG:5186".to_owned()
        } else if raw.contains("5187") {
            "EPSG:5187".to_owned()
        } else if raw.contains("4326") {
            "EPSG:4326".to_owned()
        } else if raw.contains("5179") {
            "EPSG:5179".to_owned()
        } else if raw.contains("3857") || raw.contains("900913") {
            "EPSG:3857".to_owned()
        } else if raw.starts_with("EPSG:") {
            raw.to_owned()
        } else {
            "EPSG:3857".to_owned()
        }
    }

    /// C++ `AdminBoundaryService.cpp:133-142` — 오류 분류별 안내문
    #[must_use]
    pub fn admin_boundary_error(status: &str, error_code: &str) -> Option<&'static str> {
        if status == "OK" {
            return None;
        }
        if error_code.to_ascii_uppercase().contains("KEY") {
            Some(ADMIN_KEY_FAILED_MESSAGE)
        } else if status == "NOT_FOUND" {
            Some(ADMIN_NOT_FOUND_MESSAGE)
        } else {
            Some(ADMIN_BAD_RESPONSE_MESSAGE)
        }
    }

    /// C++ `AdminBoundaryService.h:26` `kDataset`
    pub const ADMIN_BOUNDARY_DATASET: &'static str = "LT_C_ADEMD_INFO";

    /// VWorld 행정구역(`LT_C_ADSIGG_INFO`) WFS 2.0 `GetFeature` URL 생성
    /// (C++ `HeritageNearbyRegions::districtsUrl` 1:1 대응)
    #[must_use]
    pub fn districts_url(api_key: &str, box_wgs84: [f64; 4]) -> String {
        format!(
            "https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature&format=json\
             &data=LT_C_ADSIGG_INFO&size=100&page=1&crs=EPSG:4326&geometry=true&attribute=true\
             &geomFilter=BOX({:.8},{:.8},{:.8},{:.8})&key={}",
            box_wgs84[0],
            box_wgs84[1],
            box_wgs84[2],
            box_wgs84[3],
            api_key.trim()
        )
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

    /// C++ `MainWindowCadastral.cpp:122-123` 지적 다운로드 창 제목·부제
    pub const CADASTRAL_DIALOG_TITLE: &'static str = "지적도 다운로드";
    pub const CADASTRAL_DIALOG_SUBTITLE: &'static str =
        "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.";

    /// C++ `BasemapOps.cpp:1015-1016` `requireVworldKey` — 키가 없을 때 안내문 그대로.
    pub const NO_KEY_MESSAGE: &'static str =
        "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요.";

    /// C++ `MainWindowCadastral.cpp:86` — 조사 파일이 없을 때
    pub const NO_SURVEY_MESSAGE: &'static str =
        "먼저 조사를 열거나 저장하고 조사구역을 그려 주세요.";

    /// C++ `MainWindowCadastral.cpp:114` — 조사구역이 없을 때
    pub const NO_SCOPE_MESSAGE: &'static str =
        "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다.";

    /// C++ `CadastralPortal.cpp:214` — 계정(아이디·비밀번호)이 없을 때
    pub const NO_ACCOUNT_MESSAGE: &'static str =
        "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요.";

    /// 지적이 무엇을 받아오는지: VWorld WMS 그림(본번·부번) 이 아니라,
    /// 시·군·구 지적도 ZIP(`LSMD_CONT_LDREG_*.zip`)을 받아 SHP 를 지도에 올린다.
    /// C++ `CadastralPortal.cpp:293-341`(목록 → ZIP → 압축 해제 → SHP) 확인.
    pub const CADASTRAL_ZIP_TITLE_PREFIX: &'static str = "LSMD_CONT_LDREG_";

    /// C++ `CadastralPortal.cpp:322-323` — 시·군·구 지적도 ZIP 내려받기 URL
    #[must_use]
    pub fn cadastral_zip_url(file_no: &str) -> String {
        format!(
            "https://www.vworld.kr/dtmk/downloadResourceFile.do?ds_id=30563&fileNo={}",
            file_no.trim()
        )
    }

    /// C++ `CadastralPortal.cpp:295-297` — 자료 목록 페이지 URL
    #[must_use]
    pub fn cadastral_dataset_page_url(page: u32, page_size: u32) -> String {
        format!(
            "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563&datPageIndex={}&datPageSize={}",
            page.max(1),
            page_size.max(1)
        )
    }
}

/// C++ `QUrl::toPercentEncoding` 과 같은 규칙(비예약 문자만 그대로)
fn percent_encode(input: &str) -> String {
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

/// C++ `AdminBoundaryService.cpp:137` — 키 인증 실패
pub const ADMIN_KEY_FAILED_MESSAGE: &str =
    "행정구역 서비스의 인증에 실패했습니다. 리본의 더보기 → API 키 입력에서 VWorld API 키를 확인한 뒤 다시 불러오세요.";
/// C++ `AdminBoundaryService.cpp:139` — 경계를 찾지 못함
pub const ADMIN_NOT_FOUND_MESSAGE: &str =
    "선택한 읍면동 경계를 찾지 못했습니다. 시·군·구와 읍·면·동을 확인한 뒤 다시 불러오세요.";
/// C++ `AdminBoundaryService.cpp:141` — 그 밖의 응답 오류
pub const ADMIN_BAD_RESPONSE_MESSAGE: &str =
    "행정구역 서버가 올바른 경계를 보내지 않았습니다. 잠시 후 다시 불러오세요.";

/// 지적 다운로드가 실제로 받아오는 것 — 시·군·구 지적도 ZIP 목록 한 줄
/// (C++ `CadastralPortal.cpp:157-172` `parseResources`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadastralResource {
    pub file_no: String,
    pub name: String,
    /// `listFnc.download('30563','<file_no>','<size_kb>')` 의 3번째 인자
    pub size_kb: u64,
    /// `갱신일` (YYYY-MM-DD). 없으면 빈 문자열.
    pub revision: String,
}

/// 시·군·구 하나 — 지적도 ZIP 을 고르는 기준
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadastralDistrict {
    pub code: String,
    pub name: String,
}

/// C++ `CadastralPortal.cpp:157-172` `parseResources` — 자료 목록 HTML 에서 ZIP 한 줄씩 뽑는다.
///
/// 정규식 대신 같은 규칙을 문자열 검색으로 옮겼다. `<li>` 로 나누고, 그 안에서
/// `LSMD_CONT_LDREG_*.zip` 이름과 `listFnc.download('30563','<no>','<size>')` 를 찾는다.
#[must_use]
pub fn parse_cadastral_resources(html: &str) -> Vec<CadastralResource> {
    const NAME_PREFIX: &str = VworldClient::CADASTRAL_ZIP_TITLE_PREFIX;
    let mut out = Vec::new();
    for body in html.split("<li").skip(1) {
        let body = body.split("</li>").next().unwrap_or(body);
        // 이름: LSMD_CONT_LDREG_...zip
        let Some(name_start) = body.find(NAME_PREFIX) else {
            continue;
        };
        let rest = &body[name_start..];
        let Some(zip_end) = rest.find(".zip") else {
            continue;
        };
        let name = &rest[..zip_end + 4];
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ' ' | '.'))
        {
            continue;
        }
        // 링크: listFnc.download('30563','<file_no>','<size_kb>')
        let Some(link_at) = body.find("listFnc.download(") else {
            continue;
        };
        let args = &body[link_at..];
        let nums: Vec<&str> = args
            .split('\'')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .map(|(_, s)| s)
            .collect();
        if nums.len() < 3 || nums[0] != "30563" {
            continue;
        }
        let (file_no, size_kb) = (nums[1], nums[2]);
        if file_no.is_empty() || !file_no.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(size_kb) = size_kb.parse::<u64>() else {
            continue;
        };
        out.push(CadastralResource {
            file_no: file_no.to_owned(),
            name: name.to_owned(),
            size_kb,
            revision: cadastral_revision_of(body),
        });
    }
    out
}

/// `갱신일 <em>2026-01-02</em>` 에서 날짜만. 없으면 빈 문자열(C++ `parseResources` 의 `revision`).
fn cadastral_revision_of(body: &str) -> String {
    let Some(at) = body.find("갱신일") else {
        return String::new();
    };
    for chunk in body[at..].split(|c: char| !c.is_ascii_digit() && c != '-') {
        if chunk.len() == 10 && chunk.as_bytes()[4] == b'-' && chunk.as_bytes()[7] == b'-' {
            return chunk.to_owned();
        }
    }
    String::new()
}

/// C++ `CadastralPortal.cpp:107-112` `provincePrefix`
#[must_use]
pub fn cadastral_province_prefix(code: &str) -> &'static str {
    match code.get(0..2).unwrap_or_default() {
        "11" => "서울",
        "26" => "부산",
        "27" => "대구",
        "28" => "인천",
        "29" => "광주",
        "30" => "대전",
        "31" => "울산",
        "36" => "세종",
        "41" => "경기",
        "42" => "강원",
        "51" => "강원",
        "43" => "충북",
        "44" => "충남",
        "45" => "전북",
        "52" => "전북",
        "46" => "전남",
        "47" => "경북",
        "48" => "경남",
        "50" => "제주",
        _ => "",
    }
}

/// C++ `CadastralPortal.cpp:174-200` `selectResources` — 시·군·구마다 정확히 한 ZIP.
/// 정확히 하나가 아니면 C++ 과 같은 오류 문구를 돌려준다.
pub fn select_cadastral_resources(
    resources: &[CadastralResource],
    districts: &[CadastralDistrict],
) -> Result<Vec<CadastralResource>, String> {
    let mut selected: Vec<CadastralResource> = Vec::new();
    for district in districts {
        let province = cadastral_province_prefix(&district.code);
        // C++ `:179` — full_nm 의 두 번째 낱말이 시·군·구 이름
        let mut parts = district.name.split_whitespace();
        let _sido = parts.next();
        let county = parts.collect::<Vec<_>>().join("_");
        let mut matches: Vec<&CadastralResource> = Vec::new();
        for resource in resources {
            let stem = resource
                .name
                .strip_prefix(VworldClient::CADASTRAL_ZIP_TITLE_PREFIX)
                .map(|s| s.trim_end_matches(".zip"))
                .unwrap_or(&resource.name);
            // C++ `:186-189` 세종은 시(市) 자료를 고른다.
            if district.code == "36110" {
                if stem == "세종시" {
                    matches.push(resource);
                }
                continue;
            }
            let mut sections = stem.split('_');
            let file_province = sections.next().unwrap_or("");
            let file_county = sections.next().unwrap_or("");
            let short = short_province(file_province);
            let correct_province = short == province
                || (short == "전남광주" && (province == "전남" || province == "광주"));
            if !province.is_empty() && correct_province && file_county == county {
                matches.push(resource);
            }
        }
        if matches.len() != 1 {
            return Err(format!(
                "{} 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요.",
                district.name
            ));
        }
        if !selected.iter().any(|s| s.file_no == matches[0].file_no) {
            selected.push(matches[0].clone());
        }
    }
    Ok(selected)
}

/// C++ `CadastralPortal.cpp:114-120` `shortProvince` — 전체 이름을 짧은 이름으로
fn short_province(text: &str) -> &str {
    if text.starts_with("강원") {
        return "강원";
    }
    if text.starts_with("전북") {
        return "전북";
    }
    match text {
        "서울특별시" | "서울" => "서울",
        "부산광역시" | "부산" => "부산",
        "대구광역시" | "대구" => "대구",
        "인천광역시" | "인천" => "인천",
        "광주광역시" | "광주" => "광주",
        "대전광역시" | "대전" => "대전",
        "울산광역시" | "울산" => "울산",
        "세종특별자치시" | "세종" => "세종",
        "경기도" | "경기" => "경기",
        "충청북도" | "충북" => "충북",
        "충청남도" | "충남" => "충남",
        "전라북도" | "전북" => "전북",
        "전라남도" | "전남" => "전남",
        "경상북도" | "경북" => "경북",
        "경상남도" | "경남" => "경남",
        "제주특별자치도" | "제주" => "제주",
        other => other,
    }
}

/// C++ `CadastralPortal.cpp:99-101` — ZIP 이 아니라 로그인·오류 화면을 받았을 때
pub const CADASTRAL_NOT_ZIP_MESSAGE: &str =
    "VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요.";
/// C++ `CadastralPortal.cpp:80` — 저장 공간 문제
pub const CADASTRAL_DISK_MESSAGE: &str = "지적도 저장 공간을 확인하세요.";
/// C++ `CadastralPortal.cpp:67` — 응답 시간 초과
pub const CADASTRAL_TIMEOUT_MESSAGE: &str = "지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요.";

/// 실제 HTTP 로 시·군·구 지적도 ZIP 을 받아 `target` 에 저장한다.
///
/// C++ `CadastralPortal.cpp:322-327` 과 같은 URL·같은 ZIP 판정을 쓴다. HTTP 는 기존
/// `address_search.rs:758` 의 curl 방식을 그대로 재사용한다(새 크레이트 없음).
pub fn download_cadastral_zip(file_no: &str, target: &std::path::Path) -> Result<(), String> {
    let bytes = http_get_bytes(&VworldClient::cadastral_zip_url(file_no))
        .map_err(|_| CADASTRAL_TIMEOUT_MESSAGE.to_owned())?;
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(CADASTRAL_NOT_ZIP_MESSAGE.to_owned());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|_| CADASTRAL_DISK_MESSAGE.to_owned())?;
    }
    std::fs::write(target, &bytes).map_err(|_| CADASTRAL_DISK_MESSAGE.to_owned())
}

/// C++ `CadastralPortal.cpp:295-302` — VWorld 자료 목록 HTML 을 쪽 단위로 받는다.
pub fn fetch_cadastral_catalog_page(page: u32, page_size: u32) -> Result<String, String> {
    let bytes = http_get_bytes(&VworldClient::cadastral_dataset_page_url(page, page_size))
        .map_err(|_| CADASTRAL_TIMEOUT_MESSAGE.to_owned())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// C++ `AdminBoundaryService.cpp:186-230` `fetchEmd` — 읍면동 경계 WKT 를 받아 온다.
/// 키가 없으면 C++ `:193` 안내문 그대로 실패한다.
pub fn fetch_admin_boundary_wkt(
    sid: &str,
    city: &str,
    dong: &str,
) -> Result<AdminBoundaryParsed, String> {
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let key = key.trim();
    if key.is_empty() {
        return Err(ADMIN_KEY_MISSING_MESSAGE.to_owned());
    }
    if city.trim().is_empty() || dong.trim().is_empty() {
        return Err("시·군·구와 읍·면·동을 모두 고르세요".to_owned());
    }
    let bytes = http_get_bytes(&VworldClient::admin_boundary_url(key, sid, city, dong))
        .map_err(|_| ADMIN_CONNECT_MESSAGE.to_owned())?;
    let mut hint = city.trim().to_owned();
    if !sid.trim().is_empty() {
        hint = format!("{} {hint}", sid.trim());
    }
    parse_admin_boundary(&bytes, &hint)
}

/// C++ `AdminBoundaryService.cpp:193` — 키가 없을 때(행정구역 경로 문구)
pub const ADMIN_KEY_MISSING_MESSAGE: &str =
    "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣은 뒤 다시 누르세요.";
/// C++ `AdminBoundaryService.cpp:218` — 행정구역 서버 연결 실패
pub const ADMIN_CONNECT_MESSAGE: &str =
    "행정구역 서버에 연결하지 못했습니다. 인터넷 연결을 확인하고 잠시 후 다시 불러오세요.";

/// 기존 `address_search.rs:758` 의 curl GET 을 그대로 쓰는 공개 진입점.
/// VWorld·국토정보플랫폼 공개 조회가 모두 이 한 경로를 지난다(새 크레이트 없음).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    http_get_bytes(url).map_err(|_| ADMIN_CONNECT_MESSAGE.to_owned())
}

/// 읍면동 경계 파싱 결과(C++ `AdminBoundaryParse`)
#[derive(Debug, Clone, PartialEq)]
pub struct AdminBoundaryParsed {
    pub title: String,
    pub emd_code: String,
    pub wkt: String,
    pub crs_auth_id: String,
}

/// C++ `AdminBoundaryService.cpp:127-184` `parseGetFeature`
pub fn parse_admin_boundary(body: &[u8], city_hint: &str) -> Result<AdminBoundaryParsed, String> {
    use serde_json::Value;
    let root: Value =
        serde_json::from_slice(body).map_err(|_| ADMIN_BAD_RESPONSE_MESSAGE.to_owned())?;
    let resp = root.get("response").unwrap_or(&Value::Null);
    let status = resp.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "OK" {
        let code = resp
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(VworldClient::admin_boundary_error(status, code)
            .unwrap_or(ADMIN_BAD_RESPONSE_MESSAGE)
            .to_owned());
    }
    let fc = resp
        .get("result")
        .and_then(|r| r.get("featureCollection"))
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let crs_auth_id = VworldClient::normalize_crs_name(
        fc.get("crs")
            .and_then(|c| c.get("properties"))
            .and_then(|p| p.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let features = fc
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let hint = city_hint.trim();
    let mut chosen = None;
    for feature in features {
        let full = feature
            .get("properties")
            .and_then(|p| p.get("full_nm"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !hint.is_empty() && !full.contains(hint) {
            continue;
        }
        chosen = Some(feature);
        break;
    }
    let chosen = chosen
        .or_else(|| features.first())
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let props = chosen.get("properties").unwrap_or(&Value::Null);
    let mut title = props
        .get("full_nm")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if title.is_empty() {
        title = props
            .get("emd_kor_nm")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
    }
    let emd_code = match props.get("emd_cd") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let wkt = geometry_to_wkt(chosen.get("geometry").unwrap_or(&Value::Null));
    if wkt.is_empty() {
        return Err("행정구역 서버가 경계 좌표를 보내지 않았습니다. 잠시 후 다시 불러오세요.".to_owned());
    }
    Ok(AdminBoundaryParsed {
        title,
        emd_code,
        wkt,
        crs_auth_id,
    })
}

/// C++ `AdminBoundaryService.cpp:109-125` `geometryToWkt`
fn geometry_to_wkt(geom: &serde_json::Value) -> String {
    let kind = geom.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let rings = geom.get("coordinates");
    let Some(rings) = rings else {
        return String::new();
    };
    if kind.eq_ignore_ascii_case("Polygon") {
        polygon_wkt(rings)
    } else if kind.eq_ignore_ascii_case("MultiPolygon") {
        let Some(polys) = rings.as_array() else {
            return String::new();
        };
        let mut parts = Vec::new();
        for poly in polys {
            let w = polygon_wkt(poly);
            // "POLYGON(" 8자와 끝 ')' 를 떼어 MULTIPOLYGON 안에 넣는다.
            if w.starts_with("POLYGON(") && w.ends_with(')') {
                parts.push(w[8..w.len() - 1].to_owned());
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("MULTIPOLYGON({})", parts.join(", "))
        }
    } else {
        String::new()
    }
}

/// C++ `AdminBoundaryService.cpp:99-107` `polygonWkt`
fn polygon_wkt(rings: &serde_json::Value) -> String {
    let Some(rings) = rings.as_array() else {
        return String::new();
    };
    let mut parts = Vec::new();
    for ring in rings {
        let ring = ring_wkt(ring);
        if !ring.is_empty() {
            parts.push(format!("({ring})"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("POLYGON({})", parts.join(", "))
    }
}

/// C++ `AdminBoundaryService.cpp:90-97` `ringWkt`
fn ring_wkt(ring: &serde_json::Value) -> String {
    let Some(points) = ring.as_array() else {
        return String::new();
    };
    let mut pts = Vec::new();
    for point in points {
        let Some(pair) = point.as_array() else {
            continue;
        };
        if pair.len() < 2 {
            continue;
        }
        let num = |v: &serde_json::Value| -> Option<f64> {
            match v {
                serde_json::Value::String(s) => s.parse::<f64>().ok(),
                serde_json::Value::Number(n) => n.as_f64(),
                _ => None,
            }
        };
        let (Some(x), Some(y)) = (num(&pair[0]), num(&pair[1])) else {
            continue;
        };
        pts.push(format!("{x:.8} {y:.8}"));
    }
    pts.join(", ")
}

/// C++ `MainWindowCadastral.cpp:121` — 받은 지적도는 조사 폴더 안 `지적도` 에 둔다.
pub const CADASTRAL_FOLDER_NAME: &str = "지적도";
/// C++ `CadastralPortal.cpp:306` — ZIP 원본은 `지적도/원본`
pub const CADASTRAL_RAW_FOLDER_NAME: &str = "원본";

/// C++ `CadastralPortal.cpp:312` — 자동 수신 가능한 시·군·구 ZIP 크기 상한(KB)
pub const CADASTRAL_MAX_ZIP_KB: u64 = 512_000;

/// C++ `CadastralPortal.cpp:315` — ZIP 원본 파일 이름
/// `"<fileNo>-<sizeKb>-<revision>-<name>"`. 갱신일이 바뀌면 다른 파일로 본다.
#[must_use]
pub fn cadastral_zip_file_name(resource: &CadastralResource) -> String {
    format!(
        "{}-{}-{}-{}",
        resource.file_no, resource.size_kb, resource.revision, resource.name
    )
}

/// C++ `CadastralPortal.cpp:222-223` — 조사구역을 작업 좌표계에서 `buffer_m` 만큼 넓힌
/// 범위. C++ 은 GEOS 버퍼(`buffer(bufferMeters, 24)`)를 쓰지만, 여기서는 시·군·구
/// 후보를 고르는 데만 쓰이므로 사각 범위로 같은 반경을 잡는다.
#[must_use]
pub fn cadastral_search_box(work_bbox: [f64; 4], buffer_m: f64) -> Option<[f64; 4]> {
    let [min_x, min_y, max_x, max_y] = work_bbox;
    if ![min_x, min_y, max_x, max_y].iter().all(|v| v.is_finite())
        || !buffer_m.is_finite()
        || buffer_m <= 0.0
    {
        return None;
    }
    let (lo_x, hi_x) = (min_x.min(max_x), min_x.max(max_x));
    let (lo_y, hi_y) = (min_y.min(max_y), min_y.max(max_y));
    Some([
        lo_x - buffer_m,
        lo_y - buffer_m,
        hi_x + buffer_m,
        hi_y + buffer_m,
    ])
}

/// 단일 질의 최대 허용 타일 분할 수 (C++ `HGIS_WFS_MAX_TILE_SPLITS`)
pub const HGIS_WFS_MAX_TILE_SPLITS: usize = 64;
/// WFS 동시 네트워크 요청 상한선 (C++ `HGIS_WFS_MAX_CONCURRENT_NET`)
pub const HGIS_WFS_MAX_CONCURRENT_NET: usize = 4;
/// 단일 타일 WFS 응답 최대 피처 한도 (C++ `HGIS_MAX_FEATURE_LIMIT_PER_TILE`)
pub const HGIS_MAX_FEATURE_LIMIT_PER_TILE: usize = 1000;
/// 문화유산 로컬 캐시 만료 주기 (30일, C++ `HGIS_HERITAGE_CACHE_TTL_DAYS`)
pub const HGIS_HERITAGE_CACHE_TTL_DAYS: u32 = 30;

/// 국가유산 인트라넷 조회 대상 시·군 단위 (C++ `HeritageCity` in `HeritageFetchPlan.cpp`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeritageCity {
    pub sido: String,
    pub city: String,
}

impl HeritageCity {
    #[must_use]
    pub fn ok(&self) -> bool {
        !self.sido.trim().is_empty() && !self.city.trim().is_empty()
    }

    /// 시·도와 시·군이 같으면 하나만, 다르면 공백으로 연결 (C++ `HeritageCity::display`)
    #[must_use]
    pub fn display(&self) -> String {
        if !self.ok() {
            return String::new();
        }
        let s = self.sido.trim();
        let c = self.city.trim();
        if s == c {
            s.to_owned()
        } else {
            format!("{s} {c}")
        }
    }

    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        self.sido.trim() == other.sido.trim() && self.city.trim() == other.city.trim()
    }
}

/// 첫 번째 시·군을 제외하고 중복 제거된 후속 시·군 수집 목록 반환
/// (C++ `HeritageFetchPlanning::normalizedFollowUps` 1:1 대응)
#[must_use]
pub fn normalized_heritage_follow_ups(
    first: &HeritageCity,
    picked: &[HeritageCity],
) -> Vec<HeritageCity> {
    let mut out: Vec<HeritageCity> = Vec::new();
    for c in picked {
        if !c.ok() || c.same_as(first) {
            continue;
        }
        if out.iter().any(|kept| kept.same_as(c)) {
            continue;
        }
        out.push(HeritageCity {
            sido: c.sido.trim().to_owned(),
            city: c.city.trim().to_owned(),
        });
    }
    out
}

/// 현재 조사 폴더 루트(`current_root`)에서 이웃 시·군(`next`)의 형제 폴더 경로 계산
/// `<base>/<시도 시군>/원본` 구조이면 `<base>/<다음 시도 시군>/원본` 유지
/// (C++ `HeritageFetchPlanning::siblingRoot` 1:1 대응)
#[must_use]
pub fn sibling_heritage_root(current_root: &str, next: &HeritageCity) -> String {
    let folder = format!("{} {}", next.sido.trim(), next.city.trim());
    let cleaned = current_root.trim().replace('\\', "/");
    let cleaned = cleaned.trim_end_matches('/');
    if cleaned.ends_with("/원본") {
        let without_orig = cleaned.trim_end_matches("/원본");
        if let Some((base, _prev_city)) = without_orig.rsplit_once('/') {
            return format!("{base}/{folder}/원본");
        }
    }
    format!("{cleaned}/{folder}")
}

/// 조사구역 BBOX(`survey_extent`)에 반경 `buffer_meters` 버퍼를 확장하여 교차하는 모든 인접 시·군·구 코드 추출
/// (C++ `HeritageNearbyRegions::resolveAdjacentSigunguCodes` 1:1 대응)
#[must_use]
pub fn resolve_adjacent_sigungu_codes(
    survey_extent: [f64; 4],
    district_bboxes: &[(String, [f64; 4])],
    buffer_meters: f64,
) -> Vec<String> {
    let r = if buffer_meters.is_finite() && buffer_meters > 0.0 {
        buffer_meters
    } else {
        0.0
    };
    let buf_min_x = survey_extent[0].min(survey_extent[2]) - r;
    let buf_min_y = survey_extent[1].min(survey_extent[3]) - r;
    let buf_max_x = survey_extent[0].max(survey_extent[2]) + r;
    let buf_max_y = survey_extent[1].max(survey_extent[3]) + r;

    let mut out = Vec::new();
    for (code, bbox) in district_bboxes {
        let d_min_x = bbox[0].min(bbox[2]);
        let d_min_y = bbox[1].min(bbox[3]);
        let d_max_x = bbox[0].max(bbox[2]);
        let d_max_y = bbox[1].max(bbox[3]);
        let intersects = d_max_x >= buf_min_x
            && d_min_x <= buf_max_x
            && d_max_y >= buf_min_y
            && d_min_y <= buf_max_y;
        if intersects && !out.contains(code) {
            out.push(code.clone());
        }
    }
    out
}

/// 국가유산 인트라넷 원본자료 사용 서약서 사전 고지 문구 (C++ `HeritagePledge::preDisclosure`)
#[must_use]
pub const fn heritage_pledge_pre_disclosure_ko() -> &'static str {
    "받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고, \
     동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. \
     받은 자료는 동국문화재연구원 소유입니다. 도면·보고서에 넣거나 밖으로 보내기 전에 \
     서약 범위를 확인하세요."
}

/// 국가유산 서약서 체크리스트 상태 키 (C++ `HeritagePledge::checklistStateKey`)
#[must_use]
pub const fn heritage_pledge_checklist_state_key() -> &'static str {
    "HERITAGE_PLEDGE_SCOPE"
}

fn breakable_pledge_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut run = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() {
            run = 0;
        } else {
            run += 1;
            if run > 16 {
                out.push('\u{200B}');
                run = 1;
            }
        }
        out.push(ch);
    }
    out
}

/// 서약서 원문에서 외부 반출·제3자 제공·보고서 게재 등 핵심 이용 범위 문장만 발췌
/// (C++ `HeritagePledge::highlights` 1:1 대응)
#[must_use]
pub fn heritage_pledge_highlights(
    terms_text: &str,
    max_lines: usize,
    max_chars: usize,
) -> Vec<String> {
    const USE_WORDS: [&str; 14] = [
        "목적",
        "외부",
        "반출",
        "제3자",
        "제공",
        "배포",
        "공개",
        "복제",
        "유출",
        "게재",
        "인쇄",
        "도면",
        "보고서",
        "상업",
    ];
    if terms_text.trim().is_empty() || max_lines == 0 {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    for raw in terms_text.split(['\r', '\n', '.', '!', '?', '。']) {
        let sentence = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if sentence.chars().count() < 6 {
            continue;
        }
        if !USE_WORDS.iter().any(|w| sentence.contains(w)) || out.contains(&sentence) {
            continue;
        }
        let clipped = if sentence.chars().count() > max_chars && max_chars > 1 {
            let prefix: String = sentence.chars().take(max_chars - 1).collect();
            format!("{prefix}…")
        } else {
            sentence
        };
        out.push(clipped);
        if out.len() >= max_lines {
            break;
        }
    }
    out
}

/// 서약서 동의 직후 표시할 안내문 및 원문 요지 (C++ `HeritagePledge::noticeAfterAgreement`)
#[must_use]
pub fn heritage_pledge_notice_after_agreement(terms_text: &str) -> String {
    let lines = heritage_pledge_highlights(terms_text, 3, 120);
    let base = "서약서에 자동 동의했습니다. 원문은 조사폴더 주변유적/receipts 에 남습니다.";
    if lines.is_empty() {
        format!("{base} 이용 범위 문장을 찾지 못했으니 원문을 확인하세요.")
    } else {
        format!(
            "{base}\n서약 요지(원문 그대로): {}",
            breakable_pledge_line(&lines.join(" / "))
        )
    }
}

/// BBOX를 4사분면(NW, NE, SW, SE)으로 4등분 (C++ `ReferenceTiledFetch::quarters`)
#[must_use]
pub fn quarters_extent(range: [f64; 4]) -> [[f64; 4]; 4] {
    let mid_x = (range[0] + range[2]) * 0.5;
    let mid_y = (range[1] + range[3]) * 0.5;
    [
        [range[0], mid_y, mid_x, range[3]],
        [mid_x, mid_y, range[2], range[3]],
        [range[0], range[1], mid_x, mid_y],
        [mid_x, range[1], range[2], mid_y],
    ]
}

/// 대용량 WFS/WMS 타일 분할 수집 계획 (C++ `TiledFetchPlan`)
#[derive(Debug, Clone, PartialEq)]
pub struct TiledFetchPlan {
    pub total_extent: [f64; 4],
    pub num_tiles_x: usize,
    pub num_tiles_y: usize,
    pub sub_tile_extents: Vec<[f64; 4]>,
}

/// 광역 조사구역 BBOX를 단일 타일 최대 폭/높이(`max_tile_width_m`, `max_tile_height_m`) 이하의 격자 서브 타일로 분할
/// (C++ `ReferenceTiledFetch::calculateTileSplits` 1:1 대응)
#[must_use]
pub fn calculate_tile_splits(
    target_extent: [f64; 4],
    max_tile_width_m: f64,
    max_tile_height_m: f64,
) -> TiledFetchPlan {
    let min_x = target_extent[0].min(target_extent[2]);
    let min_y = target_extent[1].min(target_extent[3]);
    let max_x = target_extent[0].max(target_extent[2]);
    let max_y = target_extent[1].max(target_extent[3]);
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    let tile_w = max_tile_width_m.max(1.0);
    let tile_h = max_tile_height_m.max(1.0);

    let nx = ((width / tile_w).ceil() as usize).clamp(1, HGIS_WFS_MAX_TILE_SPLITS);
    let ny = ((height / tile_h).ceil() as usize).clamp(1, HGIS_WFS_MAX_TILE_SPLITS);
    let step_x = width / (nx as f64);
    let step_y = height / (ny as f64);

    let mut sub_tile_extents = Vec::with_capacity(nx * ny);
    for iy in 0..ny {
        let y0 = min_y + (iy as f64) * step_y;
        let y1 = if iy + 1 == ny { max_y } else { y0 + step_y };
        for ix in 0..nx {
            let x0 = min_x + (ix as f64) * step_x;
            let x1 = if ix + 1 == nx { max_x } else { x0 + step_x };
            sub_tile_extents.push([x0, y0, x1, y1]);
        }
    }

    TiledFetchPlan {
        total_extent: [min_x, min_y, max_x, max_y],
        num_tiles_x: nx,
        num_tiles_y: ny,
        sub_tile_extents,
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

    #[test]
    fn test_vworld_cadastral_layer_uri_matches_cpp_make_vworld_wms_uri() {
        // C++ `BasemapOps.cpp:1344-1349` — layers 는 본번,부번 순서, styles 는 빈 값
        let uri = VworldClient::cadastral_layer_uri(
            "TEST_KEY",
            "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun",
            "",
            "EPSG:3857",
        );
        assert!(uri.starts_with("IgnoreGetMapUrl=1&IgnoreGetFeatureInfoUrl=1&contextualWMSLegend=0"));
        assert!(uri.contains("&crs=EPSG:3857&"));
        assert!(uri.contains("&dpiMode=7&"));
        assert!(uri.contains("&format=image/png&"));
        assert!(uri.contains("&transparent=true&"));
        // C++ 과 같은 타일 규격
        assert!(uri.contains("&tilePixelRatio=2&stepWidth=512&stepHeight=512&"));
        // 인증은 Referer, DOMAIN 은 절대 보내지 않는다(CLAUDE.md 절대 규칙)
        assert!(uri.contains("&http-header:referer=https://localhost&"));
        assert!(!uri.to_uppercase().contains("DOMAIN="));
        // layers 순서·빈 styles
        assert!(uri.contains("&layers=lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun&styles&"));
        assert!(uri.contains("&url=https%3A%2F%2Fapi.vworld.kr%2Freq%2Fwms%3FKEY%3DTEST_KEY"));
    }

    #[test]
    fn test_vworld_cadastral_layer_uri_crs_candidates_are_only_4326_3857_900913() {
        // C++ `BasemapOps.cpp:1327-1336`
        assert_eq!(
            VworldClient::cadastral_wms_crs_candidates(),
            ["EPSG:4326", "EPSG:3857", "EPSG:900913"]
        );
        for forbidden in ["EPSG:5186", "EPSG:5187", "EPSG:5179"] {
            let uri = VworldClient::cadastral_layer_uri("K", "lp_pa_cbnd_bonbun", "", forbidden);
            assert!(
                uri.contains("&crs=EPSG:3857&"),
                "{forbidden} 은 3857 로 떨어져야 한다: {uri}"
            );
            assert!(!uri.contains(forbidden));
        }
        // 후보 안의 값은 그대로 쓴다.
        let uri = VworldClient::cadastral_layer_uri("K", "lp_pa_cbnd_bubun", "", "EPSG:4326");
        assert!(uri.contains("&crs=EPSG:4326&"));
    }

    #[test]
    fn test_vworld_cadastral_gdal_wms_xml_has_no_domain_and_keeps_cpp_extent() {
        // C++ `BasemapOps.cpp:1236-1269`
        let xml = VworldClient::cadastral_gdal_wms_xml("TEST_KEY", "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun");
        assert!(xml.starts_with("<GDAL_WMS><Service name=\"WMS\">"));
        assert!(xml.contains("<Version>1.3.0</Version>"));
        assert!(xml.contains("<ServerUrl>https://api.vworld.kr/req/wms?key=TEST_KEY&amp;</ServerUrl>"));
        assert!(xml.contains("<Layers>lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun</Layers>"));
        assert!(xml.contains("<CRS>EPSG:3857</CRS>"));
        assert!(xml.contains("<Referer>https://localhost</Referer>"));
        // domain= 은 해가 되기만 한다 — 넣지 않는다.
        assert!(!xml.to_lowercase().contains("domain="));
        // C++ 과 같은 DataWindow 상수
        assert!(xml.contains("<UpperLeftX>13500000</UpperLeftX>"));
        assert!(xml.contains("<LowerRightY>3800000</LowerRightY>"));
        assert!(xml.contains("<SizeX>2600000</SizeX>"));
        assert!(xml.contains("<SizeY>2000000</SizeY>"));
        assert!(xml.contains("<BlockSizeX>512</BlockSizeX>"));
    }

    #[test]
    fn test_vworld_cadastral_messages_match_cpp_when_key_missing() {
        // C++ `BasemapOps.cpp:1015-1016` requireVworldKey
        assert_eq!(
            VworldClient::NO_KEY_MESSAGE,
            "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요."
        );
        // C++ `MainWindowCadastral.cpp:86,114,122-123`
        assert_eq!(
            VworldClient::NO_SURVEY_MESSAGE,
            "먼저 조사를 열거나 저장하고 조사구역을 그려 주세요."
        );
        assert_eq!(
            VworldClient::NO_SCOPE_MESSAGE,
            "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다."
        );
        assert_eq!(VworldClient::CADASTRAL_DIALOG_TITLE, "지적도 다운로드");
        assert_eq!(
            VworldClient::CADASTRAL_DIALOG_SUBTITLE,
            "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."
        );
        // C++ `CadastralPortal.cpp:99` ZIP 판정 문구
        assert!(CADASTRAL_NOT_ZIP_MESSAGE.contains("ZIP 대신 로그인·오류 화면"));
    }

    #[test]
    fn test_vworld_cadastral_urls_match_cpp_portal() {
        // C++ `CadastralPortal.cpp:322-323`
        assert_eq!(
            VworldClient::cadastral_zip_url(" 987654 "),
            "https://www.vworld.kr/dtmk/downloadResourceFile.do?ds_id=30563&fileNo=987654"
        );
        // C++ `CadastralPortal.cpp:295-297`
        assert_eq!(
            VworldClient::cadastral_dataset_page_url(3, 100),
            "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563&datPageIndex=3&datPageSize=100"
        );
        // 페이지/크기 하한
        assert!(VworldClient::cadastral_dataset_page_url(0, 0).contains("datPageIndex=1&datPageSize=1"));
    }

    #[test]
    fn test_vworld_parse_cadastral_resources_reads_zip_rows() {
        // C++ `CadastralPortal.cpp:157-172` — 이름·링크·크기·갱신일
        let html = r##"
        <ul>
          <li><a href="#" onclick="listFnc.download('30563','111222','512');">LSMD_CONT_LDREG_경북_경주시.zip</a>
              <span>갱신일 <em>2026-01-05</em></span></li>
          <li><a href="#" onclick="listFnc.download('30563','333444','2048');">LSMD_CONT_LDREG_경북_포항시.zip</a></li>
          <li><a href="#">LSMD_CONT_LDREG_경북_안동시.zip</a></li>
        </ul>"##;
        let rows = parse_cadastral_resources(html);
        assert_eq!(rows.len(), 2, "링크 없는 줄은 버린다: {rows:?}");
        assert_eq!(rows[0].file_no, "111222");
        assert_eq!(rows[0].name, "LSMD_CONT_LDREG_경북_경주시.zip");
        assert_eq!(rows[0].size_kb, 512);
        assert_eq!(rows[0].revision, "2026-01-05");
        assert_eq!(rows[1].file_no, "333444");
        assert_eq!(rows[1].revision, "");
    }

    #[test]
    fn test_vworld_select_cadastral_resources_picks_one_zip_per_district() {
        let resources = vec![
            CadastralResource {
                file_no: "1".to_owned(),
                name: "LSMD_CONT_LDREG_경북_경주시.zip".to_owned(),
                size_kb: 100,
                revision: "2026-01-05".to_owned(),
            },
            CadastralResource {
                file_no: "2".to_owned(),
                name: "LSMD_CONT_LDREG_경북_포항시.zip".to_owned(),
                size_kb: 100,
                revision: String::new(),
            },
        ];
        let picked = select_cadastral_resources(
            &resources,
            &[CadastralDistrict {
                code: "47110".to_owned(),
                name: "경상북도 경주시".to_owned(),
            }],
        )
        .expect("select");
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].file_no, "1");

        // 둘이 걸리면 C++ `:196` 처럼 오류
        let ambiguous = vec![
            resources[0].clone(),
            CadastralResource {
                file_no: "3".to_owned(),
                name: "LSMD_CONT_LDREG_경상북도_경주시.zip".to_owned(),
                size_kb: 100,
                revision: String::new(),
            },
        ];
        let err = select_cadastral_resources(
            &ambiguous,
            &[CadastralDistrict {
                code: "47110".to_owned(),
                name: "경상북도 경주시".to_owned(),
            }],
        )
        .expect_err("두 ZIP 이 걸리면 C++ 처럼 오류");
        assert!(err.contains("지적도 파일을 정확히 하나로 확인하지 못했습니다"), "{err}");
        assert!(err.starts_with("경상북도 경주시 "));
    }

    #[test]
    fn test_vworld_admin_boundary_url_matches_cpp_getfeature() {
        // C++ `AdminBoundaryService.cpp:50-70`
        let url = VworldClient::admin_boundary_url("KEY123", "경상북도", "경주시", "황남동");
        assert!(url.starts_with("https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature"));
        assert!(url.contains("&data=LT_C_ADEMD_INFO"));
        assert!(url.contains("&size=10&page=1&geometry=true&attribute=true&crs=EPSG:3857"));
        assert!(url.contains("&key=KEY123"));
        assert!(url.contains(&format!(
            "&attrFilter={}",
            percent_encode("full_nm:like:경상북도 경주시 황남동")
        )));
        assert_eq!(
            VworldClient::attr_filter("경상북도", "경주시", "황남동"),
            "full_nm:like:경상북도 경주시 황남동"
        );
        // 빈 값은 빠진다.
        assert_eq!(VworldClient::attr_filter("", "경주시", ""), "full_nm:like:경주시");
        assert_eq!(VworldClient::attr_filter("", "", ""), "");
        let bare = VworldClient::admin_boundary_url("K", "", "", "");
        assert!(!bare.contains("attrFilter"));
    }

    #[test]
    fn test_vworld_normalize_crs_name_and_admin_errors_match_cpp() {
        // C++ `AdminBoundaryService.cpp:72-81`
        assert_eq!(VworldClient::normalize_crs_name("urn:ogc:def:crs:EPSG::5186"), "EPSG:5186");
        assert_eq!(VworldClient::normalize_crs_name("EPSG:5187"), "EPSG:5187");
        assert_eq!(VworldClient::normalize_crs_name("EPSG:5179"), "EPSG:5179");
        assert_eq!(VworldClient::normalize_crs_name("urn:x:4326"), "EPSG:4326");
        assert_eq!(VworldClient::normalize_crs_name("EPSG:900913"), "EPSG:3857");
        assert_eq!(VworldClient::normalize_crs_name("EPSG:32652"), "EPSG:32652");
        assert_eq!(VworldClient::normalize_crs_name("nonsense"), "EPSG:3857");
        // C++ `AdminBoundaryService.cpp:133-142`
        assert_eq!(VworldClient::admin_boundary_error("OK", ""), None);
        assert_eq!(
            VworldClient::admin_boundary_error("ERROR", "INVALID_KEY"),
            Some(ADMIN_KEY_FAILED_MESSAGE)
        );
        assert_eq!(
            VworldClient::admin_boundary_error("NOT_FOUND", "NOT_FOUND"),
            Some(ADMIN_NOT_FOUND_MESSAGE)
        );
        assert_eq!(
            VworldClient::admin_boundary_error("ERROR", "SYSTEM"),
            Some(ADMIN_BAD_RESPONSE_MESSAGE)
        );
    }

    #[test]
    fn test_vworld_parse_admin_boundary_builds_wkt_and_crs() {
        let body = r#"{"response":{"status":"OK","result":{"featureCollection":{
          "crs":{"properties":{"name":"urn:ogc:def:crs:EPSG::3857"}},
          "features":[{"properties":{"full_nm":"경상북도 경주시 황남동","emd_cd":"4711025000"},
            "geometry":{"type":"Polygon","coordinates":[[[129.0,35.8],[129.1,35.8],[129.1,35.9],[129.0,35.8]]]}}]}}}}"#;
        let parsed = parse_admin_boundary(body.as_bytes(), "경주시").expect("parse");
        assert_eq!(parsed.title, "경상북도 경주시 황남동");
        assert_eq!(parsed.emd_code, "4711025000");
        assert_eq!(parsed.crs_auth_id, "EPSG:3857");
        assert!(parsed.wkt.starts_with("POLYGON(("));
        assert!(parsed.wkt.contains("129.00000000 35.80000000"));
        // 상태가 OK 가 아니면 C++ 문구
        let failed = br#"{"response":{"status":"ERROR","error":{"code":"INVALID_KEY"}}}"#;
        assert_eq!(
            parse_admin_boundary(failed, "").expect_err("INVALID_KEY 는 오류"),
            ADMIN_KEY_FAILED_MESSAGE
        );
    }

    #[test]
    fn test_heritage_nearby_regions_and_tiled_fetch_and_pledge() {
        // TestHeritageNearbyRegions::testBoundaryBufferExpansion:
        // Gyeongju ("47110") and Pohang Nam-gu ("47111") separated by 150.0m, buffer = 500.0m -> both included (len 2)
        let survey_bbox = [200_000.0, 300_000.0, 200_500.0, 300_500.0];
        let districts = vec![
            (
                "47110".to_owned(),
                [190_000.0, 290_000.0, 200_500.0, 310_000.0],
            ),
            (
                "47111".to_owned(),
                [200_650.0, 290_000.0, 220_000.0, 310_000.0],
            ),
            (
                "47130".to_owned(),
                [250_000.0, 290_000.0, 260_000.0, 310_000.0],
            ),
        ];
        let codes = resolve_adjacent_sigungu_codes(survey_bbox, &districts, 500.0);
        assert_eq!(codes.len(), 2);
        assert!(codes.contains(&"47110".to_owned()));
        assert!(codes.contains(&"47111".to_owned()));

        // TestReferenceTiledFetch::testTileSplitCalculation:
        // 5,000m x 3,000m with 2,000m limit -> 3 x 2 = 6 tiles
        let plan = calculate_tile_splits([0.0, 0.0, 5_000.0, 3_000.0], 2_000.0, 2_000.0);
        assert_eq!(plan.num_tiles_x, 3);
        assert_eq!(plan.num_tiles_y, 2);
        assert_eq!(plan.sub_tile_extents.len(), 6);

        // HeritageFetchPlan::siblingRoot
        let next = HeritageCity {
            sido: "경상북도".to_owned(),
            city: "포항시".to_owned(),
        };
        assert_eq!(
            sibling_heritage_root("/data/survey/경상북도 경주시/원본", &next),
            "/data/survey/경상북도 포항시/원본"
        );

        // HeritagePledge::noticeAfterAgreement
        let notice = heritage_pledge_notice_after_agreement(
            "제1조 본 자료는 지표조사 보고서 작성 목적 외 외부 반출을 금합니다.",
        );
        assert!(notice.contains("서약 요지(원문 그대로):"));
    }
}
