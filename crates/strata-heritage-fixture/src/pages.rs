//! The HTML of each fixture page.
//!
//! Each page carries the element ids the ported scripts read, so a run against this fixture
//! exercises the same selectors as a run against the site.

/// The page paths the fixture serves, so a page's own script can point at them.
pub const DOWNLOAD_FILES_ALL: &str = "/user/data/heritageDownload/downloadFilesAll.do";

/// The six kinds and their tab letters, the C++ table (`HeritageStyle.cpp`).
pub const KIND_TABS: [(&str, &str); 6] = [
    ("S", "지정유산"),
    ("P", "현상변경허용기준"),
    ("B", "매장유산유존지역"),
    ("U", "문화유적분포지도"),
    ("R", "지표조사구역"),
    ("E", "발굴조사구역"),
];

/// `checkNet.do` before login (`HeritageIntranetFlow.cpp:135-148`): the form the login script
/// fills, with the site's own `goLogin()` doing the submit.
///
/// The real page's `goLogin()` RSA-encrypts `#userId`/`#pwd` into the hidden `USER_ID`/`USER_PW`
/// fields and posts those (`:137-139`). The fixture does the same copying without the RSA, so the
/// post itself is what the server reads here — a run that fills the visible fields but does not
/// go through `goLogin()` sends nothing, which is the failure it must not hide.
#[must_use]
pub fn login() -> &'static str {
    r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"><title>로그인</title>
<script>
function goLogin(){
  document.getElementById('USER_ID').value=document.getElementById('userId').value;
  document.getElementById('USER_PW').value=document.getElementById('pwd').value;
  document.getElementById('loginForm').submit();
}
</script></head><body>
<h1>국가유산 공간정보 인트라넷</h1>
<form id="loginForm" name="loginForm" action="/j_spring_security_check" method="post">
  <input id="userId" name="userId" type="text">
  <input id="pwd" name="pwd" type="password">
  <input id="USER_ID" name="USER_ID" type="hidden">
  <input id="USER_PW" name="USER_PW" type="hidden">
  <button type="button" onclick="goLogin();">로그인</button>
</form>
</body></html>"#
}

/// After login: a frames page whose single child frame holds the download panel. The scripts'
/// `wrap()` walks into it — a top-frame script would find nothing here.
///
/// A `<frameset>` document has no `<body>`, so the 로그아웃 mark the login probe looks for cannot
/// live here; it lives in the left menu frame, which is where the real site puts it. The probe
/// walks every frame, so it finds it there.
#[must_use]
pub fn main_frames(city: &(String, String), require_pledge: bool) -> String {
    let (sido, sgg) = city;
    let content = if require_pledge {
        "/user/data/heritageAgreement.do"
    } else {
        "/user/data/heritageDownload.do"
    };
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head>
<frameset cols="220,*">
  <frame src="/left.do" name="left">
  <frame src="{content}" name="content">
</frameset>
</html>
<!-- city:{sido} {sgg} -->"#
    )
}

/// A checkbox and its actual confirmation (`HeritageIntranetFlow.cpp:230-243`).
#[must_use]
pub fn agreement() -> &'static str {
    r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head><body>
<section><p>국가유산 공간정보 원본자료 사용 서약서 "사용" \자료</p><label><input type="checkbox">서약서에 동의</label><button onclick="if(document.querySelector('input').checked)location.href='/user/data/heritageDownload.do'">확인</button></section>
</body></html>"#
}

/// What a wrong password gets: the login form again, with the browser's credential refused.
///
/// Distinct from [`login`] by the body word 로그아웃 being absent *and* the form being present,
/// which is what the probe reads. Kept separate so a test can tell "we were sent back" from
/// "the page never loaded".
#[must_use]
pub fn login_failed() -> &'static str {
    r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"><title>로그인</title>
<script>
function goLogin(){
  document.getElementById('USER_ID').value=document.getElementById('userId').value;
  document.getElementById('USER_PW').value=document.getElementById('pwd').value;
  document.getElementById('loginForm').submit();
}
</script></head><body>
<h1>국가유산 공간정보 인트라넷</h1>
<p>아이디 또는 비밀번호가 올바르지 않습니다.</p>
<form id="loginForm" name="loginForm" action="/j_spring_security_check" method="post">
  <input id="userId" name="userId" type="text">
  <input id="pwd" name="pwd" type="password">
  <input id="USER_ID" name="USER_ID" type="hidden">
  <input id="USER_PW" name="USER_PW" type="hidden">
  <button type="button" onclick="goLogin();">로그인</button>
</form>
</body></html>"#
}

/// The left menu frame: the only place a frameset page can carry the 로그아웃 mark.
#[must_use]
pub fn left_menu() -> &'static str {
    r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head><body>
<ul><li>지정유산</li><li>현상변경허용기준</li></ul>
<div><a href="/logout.do">로그아웃</a></div>
</body></html>"#
}

/// The child frame: the download panel with the six kind tabs, the two region selects and the
/// two buttons. `#mode` starts at `S` (지정유산), the letter the C++ opens with.
#[must_use]
pub fn download_page(city: &(String, String)) -> String {
    let (sido, sgg) = city;
    let mut tabs = String::new();
    for (code, name) in KIND_TABS {
        tabs.push_str(&format!(
            "<li><a href=\"#\" onclick=\"changeTab('{code}');return false;\">{name}</a></li>"
        ));
    }
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8">
<script>
function changeTab(code){{
  document.getElementById('mode').value=code;
}}
// The real page's button builds a request and the browser receives a file
// (KaHeritageBrowser.cpp:1450-1524). The fixture sends the browser to the ZIP route, which
// answers with Content-Type: application/zip so WebView2 treats it as a download.
function downloadAll(){{
  var a=document.createElement('a');
  a.href='{files_all}';
  a.download='';
  document.body.appendChild(a);
  a.click();
  a.remove();
}}
</script></head><body>
<div id="leftPanel">
  <select id="bjdcd1_newJbn" name="bjdCdNewAddr"><option>시도선택</option></select>
  <button>검색</button>
</div>
<ul id="kindTabs">{tabs}</ul>
<div id="tabContentDiv">
<form id="searchForm" name="searchForm">
  <input type="hidden" id="mode" name="mode" value="S">
  <select id="codedetaCd0" name="codedetaCd">
    <option>시도선택</option>
    <option>{sido}</option>
  </select>
  <select id="codeCdSg0" name="codeCdSg">
    <option>시군구선택</option>
    <option>{sgg}</option>
  </select>
  <select id="pageSize" name="pageSize"><option>10개씩 보기</option><option>100개씩 보기</option></select>
  <button type="button" id="searchGisChaRirList">검색</button>
  <button type="button" id="downloadFilesAll" onclick="downloadAll();">전체다운로드</button>
</form>
<table><tbody></tbody></table>
</div>
</body></html>"#,
        files_all = DOWNLOAD_FILES_ALL
    )
}

/// What `heritageDownloadList.do` answers: two rows for the city, and a 검색결과 count so
/// `resultPageInfoScript` can read it.
#[must_use]
pub fn search_result(city: &(String, String)) -> String {
    let (sido, sgg) = city;
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head><body>
<div id="tabContentDiv">
<p>검색결과 2 건</p>
<table><thead><tr><th>번호</th><th>명칭</th></tr></thead><tbody>
<tr><td>1</td><td>{sido} {sgg} 유적 A</td></tr>
<tr><td>2</td><td>{sido} {sgg} 유적 B</td></tr>
</tbody></table>
</div></body></html>"#
    )
}
