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

/// Post-login frames (`KaHeritageBrowser.cpp:639-778`): the site's own menu opens the panel.
#[must_use]
pub fn main_frames() -> &'static str {
    r#"<!doctype html><html><head><meta charset="utf-8"></head>
<frameset cols="220,*"><frame src="/left.do" name="left">
<frame src="/fixture/waiting" name="content"></frameset></html>"#
}

/// A checkbox and its actual confirmation (`HeritageIntranetFlow.cpp:230-243`).
#[must_use]
pub fn agreement(delay: u64) -> String {
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head><body>
<section><p>국가유산 공간정보 원본자료 사용 서약서 "사용" \자료</p><label><input type="checkbox">서약서에 동의</label><button onclick="if(document.querySelector('input').checked)setTimeout(function(){{location.href='/user/data/heritageDownload.do'}},{delay})">확인</button></section>
</body></html>"#
    )
}

/// The login form stays visible while the fixture's own submit is pending.
#[must_use]
pub fn delayed_login(delay: u64) -> String {
    login().replace(
        "document.getElementById('loginForm').submit();",
        &format!(
            "setTimeout(function(){{document.getElementById('loginForm').submit();}},{delay});"
        ),
    )
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

/// The menu function owns frame navigation or popup creation (`HeritageIntranetFlow.cpp:215-228`).
#[must_use]
pub fn left_menu(pledge: bool, popup: bool, delay: u64) -> String {
    let target = if pledge {
        "/user/data/heritageAgreement.do"
    } else {
        "/user/data/heritageDownload.do"
    };
    let open = if popup {
        format!("window.open('{target}','heritageDownload');")
    } else {
        format!("parent.frames.content.location.href='{target}';")
    };
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8">
<script>function showAgreePopup(){{fetch('/fixture/panel-open');setTimeout(function(){{{open}}},{delay});}}</script>
</head><body><ul><li>지정유산</li><li>현상변경허용기준</li></ul>
<div><a href="/logout.do">로그아웃</a></div></body></html>"#
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
