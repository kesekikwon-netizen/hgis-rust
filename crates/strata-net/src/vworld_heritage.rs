//! 지적 폴더 이름·국가유산 후속 지역·서약 문구.
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

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
