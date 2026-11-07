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
