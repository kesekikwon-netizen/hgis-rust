//! The run of the 지적 button against a made-up VWorld site (`CadastralPortal.cpp:204-342`). No request
//! leaves the PC: `Site` answers in the test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::Path;

use serde_json::{json, Value};
use strata_engine::Transformer;
use strata_net::vworld_portal::{PortalFailure, PortalHttp};

use super::super::tests::{parcels, square};
use super::*;

type Answer = Box<dyn FnMut(&str, Option<&str>) -> Result<Vec<u8>, PortalFailure>>;

/// The made-up site: every request is noted, and `answer` says what comes back.
struct Site {
    asked: Vec<String>,
    answer: Answer,
}

impl Site {
    fn new(answer: impl FnMut(&str, Option<&str>) -> Result<Vec<u8>, PortalFailure> + 'static) -> Self {
        Self { asked: Vec::new(), answer: Box::new(answer) }
    }
}

impl PortalHttp for Site {
    fn get(&mut self, url: &str, _: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure> {
        self.asked.push(format!("GET {url}"));
        (self.answer)(url, None)
    }

    fn post_form(&mut self, url: &str, form: &str, _: &dyn Fn() -> bool) -> Result<Vec<u8>, PortalFailure> {
        self.asked.push(format!("POST {url} {form}"));
        (self.answer)(url, Some(form))
    }

    fn download(&mut self, url: &str, to: &Path, _: &dyn Fn() -> bool, watch: &mut dyn FnMut(u64)) -> Result<(), PortalFailure> {
        self.asked.push(format!("FILE {url}"));
        let bytes = (self.answer)(url, None)?;
        if !bytes.starts_with(b"PK\x03\x04") {
            return Err(PortalFailure::NotZip);
        }
        watch(bytes.len() as u64 / 2);
        std::fs::write(to, bytes).unwrap();
        Ok(())
    }
}

/// A county of the API's answer: a square of `half` metres around the survey area moved `east` metres, in
/// longitude and latitude.
fn county(code: &str, name: &str, east: f64, half: f64) -> Value {
    let to_wgs = Transformer::new(5186, 4326).unwrap();
    let (x, y) = (200_050.0 + east, 550_050.0);
    let ring: Vec<[f64; 2]> = [[x - half, y - half], [x + half, y - half], [x + half, y + half], [x - half, y + half], [x - half, y - half]].iter().map(|corner| to_wgs.transform(*corner).unwrap()).collect();
    json!({"type": "Feature", "properties": {"sig_cd": code, "full_nm": name}, "geometry": {"type": "Polygon", "coordinates": [ring]}})
}

fn counties_page(features: &[Value], pages: &str) -> Vec<u8> {
    json!({"response": {"status": "OK", "page": {"total": pages}, "result": {"featureCollection": {"type": "FeatureCollection", "features": features}}}}).to_string().into_bytes()
}

fn catalogue(rows: &[(&str, &str, u64)]) -> Vec<u8> {
    rows.iter().map(|(name, number, size)| format!("<li class=\"row\"><p>{name}</p><a href=\"#\" onclick=\"listFnc.download('30563','{number}','{size}');\">내려받기</a><span>갱신일 <em>2026-10-14</em></span></li>\n")).collect::<String>().into_bytes()
}

/// A county archive as the site sends it: the files of a made-up county shapefile.
fn archive(rows: &[(u32, &str, String)]) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    parcels(dir.path(), "LSMD_CONT_LDREG_47130_202610", rows, true, true);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for file in std::fs::read_dir(dir.path()).unwrap().map(Result::unwrap) {
        zip.start_file(file.file_name().to_string_lossy(), zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(&std::fs::read(file.path()).unwrap()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

const SUCCESS: &[u8] = br#"{"resultMap":{"result":"success","msg":"","url":"/"}}"#;

/// The usual site: one county around the survey area and one far away, a login that is accepted, a list
/// of two archives, and the archive of 경주시 with three parcels (one far outside 5 km).
fn usual(url: &str, _form: Option<&str>) -> Result<Vec<u8>, PortalFailure> {
    Ok(if url.starts_with("https://api.vworld.kr/") {
        counties_page(&[county("47130", "경상북도 경주시", 0.0, 20_000.0), county("47190", "경상북도 구미시", 100_000.0, 20_000.0)], "1")
    } else if url.ends_with("/v4po_usrlogin_a004.do") {
        SUCCESS.to_vec()
    } else if url.contains("datPageIndex") {
        catalogue(&[("LSMD_CONT_LDREG_경북_경주시.zip", "73", 85_446), ("LSMD_CONT_LDREG_경북_구미시.zip", "74", 50_000)])
    } else if url.contains("downloadResourceFile") {
        archive(&[(1, "산 12-3", square(200_000.0, 550_000.0, 10.0, 10.0)), (2, "15", square(203_000.0, 550_000.0, 10.0, 10.0)), (3, "999", square(230_000.0, 550_000.0, 10.0, 10.0))])
    } else {
        b"<html>page</html>".to_vec()
    })
}

fn asked<'a>(folder: &'a Path, areas: &'a [String]) -> Asked<'a> {
    Asked { survey: areas, work_epsg: 5186, radius_m: 5000.0, api_key: "KEY", username: "field", password: "pw+1/=", directory: folder }
}

fn area() -> Vec<String> {
    vec![square(200_000.0, 550_000.0, 100.0, 100.0)]
}

fn failed(why: Why, words: &str) -> Result<Ready, PortalStop> {
    Err(PortalStop::Failed { why, words: words.to_owned() })
}

/// The whole run (`:204-342`): the counties, the login, the list, the archive, the table; with the stages
/// and words of the C++. A second run finds the archive and the table and receives nothing again.
#[test]
fn a_run_receives_the_county_and_writes_the_table() {
    let (folder, areas) = (tempfile::tempdir().unwrap(), area());
    let (mut site, mut said) = (Site::new(usual), Vec::new());
    let ready = receive(&asked(folder.path(), &areas), &mut site, &|| false, &mut |value, words| said.push((value, words.to_owned()))).unwrap();

    assert_eq!((ready.parcels, ready.reused), (2, false));
    assert_eq!(ready.file.parent(), Some(folder.path().join("표시").as_path()));
    assert!(folder.path().join("원본").join("73-85446-2026-10-14-LSMD_CONT_LDREG_경북_경주시.zip").is_file());
    assert!(folder.path().join("원본").join("추출").is_dir());
    let kinds: Vec<&str> = site.asked.iter().map(|line| line.split('?').next().unwrap()).collect();
    assert_eq!(kinds, ["GET https://api.vworld.kr/req/data", "GET https://www.vworld.kr/dtmk/dtmk_ntads_s002.do", "POST https://www.vworld.kr/v4po_usrlogin_a004.do usrIdeE=ZmllbGQ%3D&usrPwdE=cHcrMS89&nextUrl=%2Fdtmk%2Fdtmk_ntads_s002.do%3FdsId%3D30563", "GET https://www.vworld.kr/dtmk/dtmk_ntads_s002.do", "FILE https://www.vworld.kr/dtmk/downloadResourceFile.do"]);
    assert!(site.asked[0].contains("&page=1&") && site.asked[0].ends_with("&key=KEY") && site.asked[0].contains("data=LT_C_ADSIGG_INFO&size=100"), "{}", site.asked[0]);
    assert!(site.asked[3].ends_with("dsId=30563&datPageIndex=1&datPageSize=100") && site.asked[4].ends_with("ds_id=30563&fileNo=73"));
    let words: Vec<&str> = said.iter().map(|(_, words)| words.as_str()).collect();
    assert_eq!(words, ["조사 주변 5km의 시·군·구를 확인하고 있습니다…", "VWorld 계정을 확인하고 있습니다…", "해당 시·군·구의 지적도 파일을 찾고 있습니다…", "지적도 원본을 받고 있습니다…", "5km 범위의 경계선과 지번을 정리하고 있습니다…", "조사 주변 필지와 지번을 정리하고 있습니다…", "주변 필지 2개를 준비했습니다."]);
    assert_eq!(said.iter().map(|(value, _)| *value).collect::<Vec<u8>>(), [2, 8, 12, 15, 65, 65, 100]);

    let mut again = Site::new(usual);
    let second = receive(&asked(folder.path(), &areas), &mut again, &|| false, &mut |_, _| {}).unwrap();
    assert_eq!((second.file, second.parcels, second.reused), (ready.file, 2, true));
    assert!(again.asked.iter().all(|line| !line.starts_with("FILE")), "the archive that is there is not received again");
    assert_eq!(again.asked.len(), 4, "the counties, the login and the list are asked again (`:231-303`)");
}

/// What is missing is said before anything is asked of the site (`:210-217`).
#[test]
fn nothing_is_asked_without_area_key_or_account() {
    let (folder, areas) = (tempfile::tempdir().unwrap(), area());
    let run = |change: &dyn Fn(&mut Asked<'_>)| {
        let (mut request, mut site) = (asked(folder.path(), &areas), Site::new(usual));
        change(&mut request);
        let outcome = receive(&request, &mut site, &|| false, &mut |_, _| {});
        assert!(site.asked.is_empty());
        outcome
    };
    assert_eq!(run(&|request| request.survey = &[]), failed(Why::Area, "조사구역과 미터 단위 작업 좌표계를 먼저 확인하세요."));
    assert_eq!(run(&|request| request.radius_m = 0.0), failed(Why::Area, "조사구역과 미터 단위 작업 좌표계를 먼저 확인하세요."));
    assert_eq!(run(&|request| request.api_key = ""), failed(Why::NoKey, "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요."));
    assert_eq!(run(&|request| request.password = ""), failed(Why::NoAccount, "지적도 메뉴에서 VWorld 계정을 설정하세요."));
    assert_eq!(receive(&asked(folder.path(), &areas), &mut Site::new(usual), &|| true, &mut |_, _| {}), Err(PortalStop::Cancelled));
}

/// The counties (`:229-263`): every page of the answer is asked for, a county is taken once and only when
/// it meets the scope, and an answer that cannot be trusted ends the run with the C++ words.
#[test]
fn the_counties_within_the_scope() {
    let (folder, areas) = (tempfile::tempdir().unwrap(), area());
    let scope = Scope::around(&areas, 5000.0).unwrap().unwrap();
    let found = |answer: fn(&str) -> Vec<u8>| {
        let mut site = Site::new(move |url, _| Ok(answer(url)));
        (counties(&asked(folder.path(), &areas), &scope, &mut site, &|| false), site.asked.len())
    };
    let two_pages = |url: &str| if url.contains("&page=1&") { counties_page(&[county("47130", "경상북도 경주시", 0.0, 20_000.0), county("47190", "경상북도 구미시", 100_000.0, 20_000.0)], "2") } else { counties_page(&[county("47111", "경상북도 포항시 남구", 6000.0, 2000.0), county("47130", "경상북도 경주시", 0.0, 20_000.0)], "2") };
    let (districts, requests) = found(two_pages);
    assert_eq!(districts.unwrap().iter().map(|d| (d.code.as_str(), d.name.as_str())).collect::<Vec<_>>(), [("47130", "경상북도 경주시"), ("47111", "경상북도 포항시 남구")]);
    assert_eq!(requests, 2);

    let words = |answer: fn(&str) -> Vec<u8>| match found(answer).0 {
        Err(PortalStop::Failed { why, words }) => (why, words),
        other => panic!("{other:?}"),
    };
    assert_eq!(words(|_| br#"{"response":{"status":"ERROR"}}"#.to_vec()), (Why::KeyRefused, "VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요.".to_owned()));
    assert_eq!(words(|_| counties_page(&[json!({"properties": {"sig_cd": "4713", "full_nm": "경주"}, "geometry": null})], "1")), (Why::Site, "시·군·구 경계 응답에 필요한 정보가 없습니다.".to_owned()));
    assert_eq!(words(|_| json!({"response": {"status": "OK", "result": {"featureCollection": {"crs": {"type": "name", "properties": {"name": "EPSG:3857"}}, "features": []}}}}).to_string().into_bytes()), (Why::Site, "시·군·구 응답 좌표계가 요청과 다릅니다.".to_owned()));
    assert_eq!(words(|_| counties_page(&[county("47190", "경상북도 구미시", 100_000.0, 20_000.0)], "1")), (Why::Site, "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned()));
    let never_ends = |_: &str| counties_page(&vec![county("47190", "경상북도 구미시", 100_000.0, 20_000.0); 100], "0");
    assert_eq!((found(never_ends).1, words(never_ends).1), (10, "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned()));
}

/// The login (`:264-292`): the site's page is opened first, the account is sent as the page sends it, and
/// each kind of answer has its own words; what the site says is added without its markup.
#[test]
fn a_login_that_is_not_accepted() {
    let (folder, areas) = (tempfile::tempdir().unwrap(), area());
    let tried = |answer: &'static [u8]| {
        let mut site = Site::new(move |url, _| Ok(if url.ends_with("/v4po_usrlogin_a004.do") { answer.to_vec() } else { b"<html/>".to_vec() }));
        let outcome = log_in(&asked(folder.path(), &areas), &mut site, &|| false);
        assert_eq!(site.asked.len(), 2);
        assert_eq!(site.asked[0], "GET https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563");
        outcome
    };
    assert_eq!(tried(SUCCESS), Ok(()));
    let refused = "VWorld 로그인에 실패했습니다. 아이디·비밀번호는 VWorld 누리집(vworld.kr) 로그인 계정입니다(API 키가 아님).\n\nVWorld 안내: 아이디 또는 비밀번호를 확인하세요.";
    assert_eq!(tried("{\"resultMap\":{\"result\":\"error\",\"msg\":\"아이디 또는<br/>비밀번호를  <b>확인</b>하세요.\"}}".as_bytes()), Err(PortalStop::Failed { why: Why::AccountRefused, words: refused.to_owned() }));
    let more = "VWorld 누리집이 로그인 전에 추가 절차를 요구합니다. 인터넷 브라우저로 vworld.kr 에 로그인해 안내를 마친 뒤 다시 시도하세요.";
    assert_eq!(tried(br#"{"resultMap":{"result":"pwChange","msg":""}}"#), Err(PortalStop::Failed { why: Why::AccountRefused, words: more.to_owned() }));
    let unread = "VWorld 로그인 응답을 읽지 못했습니다. 누리집의 로그인 방식이 바뀌었을 수 있습니다. 잠시 뒤 다시 시도하고, 계속되면 개발자에게 알려 주세요.";
    assert_eq!(tried(b"<html>login</html>"), Err(PortalStop::Failed { why: Why::Site, words: unread.to_owned() }));
}

/// The list and the archives (`:293-337`): a list that does not end, an archive of a size that cannot be
/// one county, a page in place of the archive, and an archive without shapes each end the run.
#[test]
fn what_is_wrong_with_the_list_or_an_archive() {
    let (folder, areas) = (tempfile::tempdir().unwrap(), area());
    let run = |answer: fn(&str) -> Option<Vec<u8>>| match receive(&asked(folder.path(), &areas), &mut Site::new(move |url, form| answer(url).map_or_else(|| usual(url, form), Ok)), &|| false, &mut |_, _| {}) {
        Err(PortalStop::Failed { why, words }) => (why, words),
        other => panic!("{other:?}"),
    };
    let long: fn(&str) -> Option<Vec<u8>> = |url| url.contains("datPageIndex").then(|| catalogue(&vec![("LSMD_CONT_LDREG_경북_구미시.zip", "74", 50_000); 100]));
    assert_eq!(run(long), (Why::Site, "VWorld 자료 목록이 끝나지 않아 내려받기를 중단했습니다.".to_owned()));
    let none: fn(&str) -> Option<Vec<u8>> = |url| url.contains("datPageIndex").then(|| catalogue(&[("LSMD_CONT_LDREG_경북_구미시.zip", "74", 50_000)]));
    assert_eq!(run(none), (Why::Site, "경상북도 경주시 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요.".to_owned()));
    let huge: fn(&str) -> Option<Vec<u8>> = |url| url.contains("datPageIndex").then(|| catalogue(&[("LSMD_CONT_LDREG_경북_경주시.zip", "73", 512_001)]));
    assert_eq!(run(huge), (Why::Data, "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다.".to_owned()));
    let page: fn(&str) -> Option<Vec<u8>> = |url| url.contains("downloadResourceFile").then(|| b"<html>login</html>".to_vec());
    assert_eq!(run(page), (Why::Site, "VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요.".to_owned()));
    let empty: fn(&str) -> Option<Vec<u8>> = |url| {
        url.contains("downloadResourceFile").then(|| {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            zip.start_file("읽어보기.txt", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"no shapes").unwrap();
            zip.finish().unwrap().into_inner()
        })
    };
    assert_eq!(run(empty), (Why::Data, "받은 시·군·구 ZIP에 SHP 도형이 없습니다.".to_owned()));
}
