//! 연속지적도 포털: 목록 해석·선택·ZIP 내려받기와 창 문구 (C++ `CadastralPortal.cpp`).
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;
use crate::address_search::http_get_bytes;

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
            // C++ `:190-193` — 첫 `_` 앞이 도, **첫 `_` 뒤 전부**가 시·군·구다.
            // 「수원시_장안구」처럼 두 낱말인 구도 이렇게 해야 맞는다.
            let (file_province, file_county) = stem.split_once('_').unwrap_or((stem, ""));
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
impl VworldClient {
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

    /// C++ `CadastralPortal.cpp:216` — 누리집 계정(아이디·비밀번호)이 없을 때
    pub const NO_ACCOUNT_MESSAGE: &'static str = "지적도 메뉴에서 VWorld 계정을 설정하세요.";

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
