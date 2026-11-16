//! The HTML of each fixture page.
//!
//! Each page carries the element ids the ported scripts read, so a run against this fixture
//! exercises the same selectors as a run against the site.

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
#[must_use]
pub fn login() -> &'static str {
    r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"><title>로그인</title>
<script>
function goLogin(){document.getElementById('loginForm').submit();}
</script></head><body>
<h1>국가유산 공간정보 인트라넷</h1>
<form id="loginForm" name="loginForm" action="/j_spring_security_check" method="post">
  <input id="userId" name="userId" type="text">
  <input id="pwd" name="pwd" type="password">
  <button type="button" onclick="goLogin();">로그인</button>
</form>
</body></html>"#
}

/// After login: a frames page whose single child frame holds the download panel. The scripts'
/// `wrap()` walks into it — a top-frame script would find nothing here.
#[must_use]
pub fn main_frames(city: &(String, String)) -> String {
    let (sido, sgg) = city;
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8"></head>
<frameset cols="220,*">
  <frame src="/left.do" name="left">
  <frame src="/user/data/heritageDownload.do" name="content">
</frameset>
<noframes><body><div>로그아웃</div></body></noframes>
</html>
<!-- city:{sido} {sgg} -->"#
    )
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
  <button type="button" id="downloadFilesAll">전체다운로드</button>
</form>
<table><tbody></tbody></table>
</div>
</body></html>"#
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
