//! The run of the 지적 button: the counties within 5 km of the survey area are found, the VWorld site is
//! logged in to, the county archives of the continuous cadastral map are received (or found in the survey
//! folder), unpacked, and their parcels near the survey area written as one table
//! (`target/cpp-ref/src/core/CadastralPortal.cpp:204-342`). The requests are sent by what the caller hands
//! in; the steps, limits and words are the C++ ones.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde_json::Value;
use strata_engine::{Scope, Transformer};
use strata_net::vworld_client::{parse_cadastral_resources, select_cadastral_resources, CadastralDistrict, CadastralResource, VworldClient};
use strata_net::vworld_portal::{PortalFailure, PortalHttp, DATASET, LOGIN, SITE};
use strata_topo::archive;

use super::{Ready, Stopped};

#[cfg(test)]
#[path = "cadastral_portal_tests.rs"]
mod tests;

const NEEDS_AREA: &str = "조사구역과 미터 단위 작업 좌표계를 먼저 확인하세요.";
const NOT_CARRIED: &str = "조사 범위를 행정구역 좌표계로 변환하지 못했습니다.";
const COUNTIES_UNKNOWN: &str = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.";
const RECEIVING: &str = "지적도 원본을 받고 있습니다…";
/// The largest archive of one county that is received by itself, in KB (`:312`).
const MOST_KB: u64 = 512_000;

/// What a run is asked for.
#[derive(Debug, Clone)]
pub struct Asked<'a> {
    /// The survey areas as WKT polygons in the work CRS.
    pub survey: &'a [String],
    pub work_epsg: u32,
    /// How far around them parcels are wanted, in metres (`CadastralPortal.h:14`: 5000).
    pub radius_m: f64,
    pub api_key: &'a str,
    pub username: &'a str,
    pub password: &'a str,
    /// `<survey folder>/지적도`.
    pub directory: &'a Path,
}

/// The kind of reason a run ended for (the gauge shows a fixed code for each).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Area,
    NoKey,
    KeyRefused,
    NoAccount,
    /// The site did not take the account, or wants a step on the site first: the account is offered again.
    AccountRefused,
    Site,
    Storage,
    Data,
    NoData,
}

/// Why a run brought no table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalStop {
    Cancelled,
    Failed { why: Why, words: String },
}

fn failed(why: Why, words: &str) -> PortalStop {
    PortalStop::Failed { why, words: words.to_owned() }
}

fn carried(failure: PortalFailure) -> PortalStop {
    match failure {
        PortalFailure::GivenUp => PortalStop::Cancelled,
        PortalFailure::Disk => failed(Why::Storage, failure.words()),
        _ => failed(Why::Site, failure.words()),
    }
}

/// The counties whose boundary meets the scope (`:229-263`): the API is asked page by page for the
/// counties in the scope's box, and each is tested against the scope itself, in the work CRS.
fn counties(asked: &Asked<'_>, scope: &Scope, http: &mut dyn PortalHttp, stop: &dyn Fn() -> bool) -> Result<Vec<CadastralDistrict>, PortalStop> {
    let not_carried = |_| failed(Why::Area, NOT_CARRIED);
    let (to_wgs, to_work) = (Transformer::new(asked.work_epsg, 4326).map_err(not_carried)?, Transformer::new(4326, asked.work_epsg).map_err(not_carried)?);
    let [west, south, east, north] = scope.bbox().and_then(|bbox| to_wgs.transform_box(bbox)).map_err(not_carried)?;
    let test = scope.test().map_err(not_carried)?;
    let (mut found, mut complete) = (Vec::<CadastralDistrict>::new(), false);
    for page in 1..=10_u64 {
        let url = format!("https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature&format=json&data=LT_C_ADSIGG_INFO&size=100&crs=EPSG:4326&geometry=true&attribute=true&page={page}&geomFilter=BOX({west:.8},{south:.8},{east:.8},{north:.8})&key={}", asked.api_key.trim());
        let body = http.get(&url, stop).map_err(carried)?;
        if stop() {
            return Err(PortalStop::Cancelled);
        }
        let root: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        let response = &root["response"];
        if response["status"].as_str() != Some("OK") {
            return Err(failed(Why::KeyRefused, "VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요."));
        }
        let collection = &response["result"]["featureCollection"];
        let named = collection["crs"]["properties"]["name"].as_str().unwrap_or_default();
        if !named.is_empty() && crate::geojson::named_epsg(named) != Some(4326) {
            return Err(failed(Why::Site, "시·군·구 응답 좌표계가 요청과 다릅니다."));
        }
        let features = collection["features"].as_array().map_or(&[][..], Vec::as_slice);
        for feature in features {
            let (code, name) = (feature["properties"]["sig_cd"].as_str().unwrap_or_default(), feature["properties"]["full_nm"].as_str().unwrap_or_default());
            let boundary = crate::geojson::polygons_of(&feature["geometry"]).unwrap_or_default();
            if code.chars().count() != 5 || name.is_empty() || boundary.is_empty() {
                return Err(failed(Why::Site, "시·군·구 경계 응답에 필요한 정보가 없습니다."));
            }
            let mut meets = false;
            for rings in &boundary {
                let rings: Vec<Vec<[f64; 2]>> = rings.iter().map(|ring| to_work.transform_all(ring)).collect::<Result<_, _>>().map_err(not_carried)?;
                if test.touches_polygon(&rings).map_err(not_carried)? {
                    meets = true;
                    break;
                }
            }
            if meets && !found.iter().any(|county| county.code == code) {
                found.push(CadastralDistrict { code: code.to_owned(), name: name.to_owned() });
            }
        }
        let pages = match &response["page"]["total"] {
            Value::String(text) => text.trim().parse().unwrap_or(0),
            Value::Number(number) => number.as_u64().unwrap_or(0),
            _ => 0,
        };
        if (pages > 0 && page >= pages) || (pages == 0 && features.len() < 100) {
            complete = true;
            break;
        }
    }
    if !complete || found.is_empty() {
        return Err(failed(Why::Site, COUNTIES_UNKNOWN));
    }
    Ok(found)
}

/// A value of a form: everything but letters, digits and `-._~` as `%XX`.
fn form_value(text: &str) -> String {
    text.bytes().map(|byte| if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) { char::from(byte).to_string() } else { format!("%{byte:02X}") }).collect()
}

/// What the site says, without its markup: a line break is a blank, any other tag nothing (`:279-282`).
fn plain(message: &str) -> String {
    let (mut text, mut rest) = (String::new(), message);
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = "";
            break;
        };
        if rest[open + 1..open + close].trim_start().to_ascii_lowercase().starts_with("br") {
            text.push(' ');
        }
        rest = &rest[open + close + 1..];
    }
    text.push_str(rest);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The login (`:264-292`): the site's page first, then the account as that page sends it.
fn log_in(asked: &Asked<'_>, http: &mut dyn PortalHttp, stop: &dyn Fn() -> bool) -> Result<(), PortalStop> {
    http.get(&format!("{SITE}{DATASET}"), stop).map_err(carried)?;
    if stop() {
        return Err(PortalStop::Cancelled);
    }
    let coded = |text: &str| form_value(&base64::engine::general_purpose::STANDARD.encode(text.as_bytes()));
    let form = format!("usrIdeE={}&usrPwdE={}&nextUrl={}", coded(asked.username), coded(asked.password), form_value(DATASET));
    let answer = http.post_form(&format!("{SITE}{LOGIN}"), &form, stop).map_err(carried)?;
    if stop() {
        return Err(PortalStop::Cancelled);
    }
    // `{"resultMap":{"result","msg","url"}}`: "error" is a wrong ID or password; any other result asks for
    // a step on the site first (a new password, the government's joint login).
    let map: Value = serde_json::from_slice(&answer).unwrap_or(Value::Null);
    let (result, reason) = (map["resultMap"]["result"].as_str().unwrap_or_default(), plain(map["resultMap"]["msg"].as_str().unwrap_or_default()));
    let (why, words) = match result {
        "success" => return Ok(()),
        "" => (Why::Site, "VWorld 로그인 응답을 읽지 못했습니다. 누리집의 로그인 방식이 바뀌었을 수 있습니다. 잠시 뒤 다시 시도하고, 계속되면 개발자에게 알려 주세요."),
        "error" => (Why::AccountRefused, "VWorld 로그인에 실패했습니다. 아이디·비밀번호는 VWorld 누리집(vworld.kr) 로그인 계정입니다(API 키가 아님)."),
        _ => (Why::AccountRefused, "VWorld 누리집이 로그인 전에 추가 절차를 요구합니다. 인터넷 브라우저로 vworld.kr 에 로그인해 안내를 마친 뒤 다시 시도하세요."),
    };
    Err(failed(why, &if reason.is_empty() { words.to_owned() } else { format!("{words}\n\nVWorld 안내: {reason}") }))
}

/// The one archive of each county, from the site's list (`:293-305`).
fn archives(districts: &[CadastralDistrict], http: &mut dyn PortalHttp, stop: &dyn Fn() -> bool) -> Result<Vec<CadastralResource>, PortalStop> {
    let (mut listed, mut complete) = (Vec::new(), false);
    for page in 1..=10 {
        let html = http.get(&VworldClient::cadastral_dataset_page_url(page, 100), stop).map_err(carried)?;
        if stop() {
            return Err(PortalStop::Cancelled);
        }
        let rows = parse_cadastral_resources(&String::from_utf8_lossy(&html));
        let last = rows.len() < 100;
        listed.extend(rows);
        if last {
            complete = true;
            break;
        }
    }
    if !complete {
        return Err(failed(Why::Site, "VWorld 자료 목록이 끝나지 않아 내려받기를 중단했습니다."));
    }
    select_cadastral_resources(&listed, districts).map_err(|words| failed(Why::Site, &words))
}

/// The shapefiles of the archives: an archive that is in the survey folder and unpacks is not received
/// again (`:306-337`).
fn shapefiles(asked: &Asked<'_>, files: &[CadastralResource], http: &mut dyn PortalHttp, stop: &dyn Fn() -> bool, stage: &mut dyn FnMut(u8, &str)) -> Result<Vec<PathBuf>, PortalStop> {
    let (originals, limits) = (asked.directory.join("원본"), archive::Limits::default());
    std::fs::create_dir_all(&originals).map_err(|_| failed(Why::Storage, "조사 폴더에 지적도 저장 위치를 만들지 못했습니다."))?;
    let (extracted, mut sources) = (originals.join("추출"), Vec::new());
    for (index, file) in files.iter().enumerate() {
        if stop() {
            return Err(PortalStop::Cancelled);
        }
        if file.size_kb == 0 || file.size_kb > MOST_KB {
            return Err(failed(Why::Data, "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다."));
        }
        // The size the site names is part of the name: an archive replaced under the same number is new.
        let path = originals.join(format!("{}-{}-{}-{}", file.file_no, file.size_kb, file.revision, file.name));
        let mut unpacked = if path.exists() { archive::prepare(&path, &extracted, &limits, stop) } else { archive::Prepared::default() };
        if stop() {
            return Err(PortalStop::Cancelled);
        }
        if unpacked.files.is_empty() {
            let (from, span, whole) = (15 + index * 50 / files.len(), 50 / files.len(), (file.size_kb * 1024).max(1));
            let mut watch = |done: u64| stage(u8::try_from(from + span * usize::try_from(done.min(whole) * 100 / whole).unwrap_or(100) / 100).unwrap_or(65), RECEIVING);
            http.download(&VworldClient::cadastral_zip_url(&file.file_no), &path, stop, &mut watch).map_err(carried)?;
            if stop() {
                return Err(PortalStop::Cancelled);
            }
            unpacked = archive::prepare(&path, &extracted, &limits, stop);
        }
        if stop() {
            return Err(PortalStop::Cancelled);
        }
        if !unpacked.error.is_empty() {
            return Err(failed(Why::Data, &unpacked.error));
        }
        let before = sources.len();
        sources.extend(unpacked.files.into_iter().filter(|file| file.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("shp"))));
        if sources.len() == before {
            return Err(failed(Why::Data, "받은 시·군·구 ZIP에 SHP 도형이 없습니다."));
        }
    }
    Ok(sources)
}

/// Runs the whole of it. `stage` is told how far the run is (0 to 100) and what it does, in the C++ words.
///
/// # Errors
/// See [`PortalStop`]; the words of a failure are the C++ ones.
pub fn receive(asked: &Asked<'_>, http: &mut dyn PortalHttp, stop: &dyn Fn() -> bool, stage: &mut dyn FnMut(u8, &str)) -> Result<Ready, PortalStop> {
    if stop() {
        return Err(PortalStop::Cancelled);
    }
    if asked.survey.is_empty() || !asked.radius_m.is_finite() || asked.radius_m <= 0.0 || Transformer::new(asked.work_epsg, 4326).is_err() {
        return Err(failed(Why::Area, NEEDS_AREA));
    }
    if asked.api_key.trim().is_empty() {
        return Err(failed(Why::NoKey, "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요."));
    }
    if asked.username.is_empty() || asked.password.is_empty() {
        return Err(failed(Why::NoAccount, "지적도 메뉴에서 VWorld 계정을 설정하세요."));
    }
    let scope = Scope::around(asked.survey, asked.radius_m).ok().flatten().ok_or_else(|| failed(Why::Area, "조사 주변 5km 범위를 만들지 못했습니다."))?;
    stage(2, "조사 주변 5km의 시·군·구를 확인하고 있습니다…");
    let districts = counties(asked, &scope, http, stop)?;
    stage(8, "VWorld 계정을 확인하고 있습니다…");
    log_in(asked, http, stop)?;
    stage(12, "해당 시·군·구의 지적도 파일을 찾고 있습니다…");
    let files = archives(&districts, http, stop)?;
    let sources = shapefiles(asked, &files, http, stop, stage)?;
    stage(65, "5km 범위의 경계선과 지번을 정리하고 있습니다…");
    super::prepare(&sources, Some(&scope), asked.work_epsg, &asked.directory.join("표시"), stop, &mut |value, words| stage(65 + u8::try_from(u16::from(value) * 35 / 100).unwrap_or(35), words)).map_err(|stopped| match stopped {
        Stopped::Cancelled => PortalStop::Cancelled,
        Stopped::Failed(words) => failed(if words == super::NO_PARCELS { Why::NoData } else { Why::Data }, &words),
    })
}
