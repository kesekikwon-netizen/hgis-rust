//! Controlled AJAX acknowledgments (`target/cpp-ref/src/core/HeritageIntranetFlow.cpp:245-413`).

use crate::pages::{DOWNLOAD_FILES_ALL, KIND_TABS};

pub(crate) fn download_page(city: &(String, String), delay: u64, empty: bool) -> String {
    let (sido, sgg) = city;
    let tabs: String = KIND_TABS
        .iter()
        .map(|(code, name)| {
            format!(
                "<li><a href=\"#\" onclick=\"changeTab('{code}');return false;\">{name}</a></li>"
            )
        })
        .collect();
    format!(
        r#"<!doctype html><html lang="ko"><head><meta charset="utf-8">
<script>
var fixtureReady=false;
function mark(path){{fetch('/fixture/'+path);}}
function resetResult(){{
  fixtureReady=false;
  document.getElementById('count').textContent='검색결과 37 건';
  var old=document.querySelector('table tbody'), fresh=old.cloneNode(false);
  fresh.innerHTML='<tr><td>1</td></tr><tr><td>2</td></tr>';old.replaceWith(fresh);
}}
function installForm(code){{
  var old=document.getElementById('searchForm');
  var fresh=document.getElementById('formTemplate').content.firstElementChild.cloneNode(true);
  fresh.querySelector('#mode').value=code;old.replaceWith(fresh);resetResult();
}}
function changeTab(code){{
  mark('kind-request');
  if({delay}>0)setTimeout(function(){{installForm(code==='S'?'R':'S');}},{delay}/2);
  setTimeout(function(){{installForm(code);}},{delay});
}}
function loadCities(){{
  var city=document.getElementById('codeCdSg0');
  city.innerHTML='<option value="">시군구선택</option>';
  setTimeout(function(){{var option=document.createElement('option');
    option.value='4713';option.text='{sgg}';city.appendChild(option);}},{delay});
}}
function search(){{
  mark('search-request');fixtureReady=false;
  setTimeout(function(){{
    var old=document.querySelector('table tbody'), fresh=old.cloneNode(false);
    fresh.innerHTML={empty}?'':'<tr><td>1</td></tr><tr><td>2</td></tr>';
    old.replaceWith(fresh);
    document.getElementById('count').textContent='검색결과 '+({empty}?'0':'2')+' 건';
    fixtureReady=true;
  }},{delay});
}}
// A click cannot manufacture filtered data (`KaHeritageBrowser.cpp:1106-1147`).
function downloadAll(){{
  if(!fixtureReady){{mark('unfiltered-download');return;}}
  if(!confirm('EPSG:5179'))return;
  mark('confirm-accepted');
  var a=document.createElement('a');a.href='{files_all}';a.download='';
  document.body.appendChild(a);a.click();a.remove();
}}
</script></head><body>
<div id="leftPanel"><select id="bjdcd1_newJbn" name="bjdCdNewAddr"><option>시도선택</option></select><button>검색</button></div>
<ul id="kindTabs">{tabs}</ul>
<template id="formTemplate"><form id="searchForm" name="searchForm">
  <input type="hidden" id="mode" name="mode" value="S">
  <select id="codedetaCd0" name="codedetaCd" onchange="loadCities()">
    <option value="">시도선택</option><option value="47">{sido}</option>
  </select>
  <select id="codeCdSg0" name="codeCdSg"><option value="">시군구선택</option></select>
  <select id="pageSize" name="pageSize"><option>10개씩 보기</option><option>100개씩 보기</option></select>
  <button type="button" id="searchGisChaRirList" onclick="search()">검색</button>
  <button type="button" id="downloadFilesAll" onclick="downloadAll()">전체다운로드</button>
</form></template>
<div id="tabContentDiv"><form id="searchForm"></form><p id="count">검색결과 37 건</p>
<table><tbody><tr><td>1</td></tr><tr><td>2</td></tr></tbody></table></div>
<script>installForm('S');</script>
</body></html>"#,
        files_all = DOWNLOAD_FILES_ALL
    )
}
