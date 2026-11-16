//! The heritage intranet page scripts, ported verbatim from
//! `target/cpp-ref/src/core/HeritageIntranetFlow.cpp`.
//!
//! These are the whole trick of the C++ run: the download panel lives in a same-origin
//! child frame, so every script goes through `wrap()` (`:43-92`), which walks the frames,
//! picks the one holding the panel, and rebinds `document` to it. A script run in the top
//! frame finds nothing and the run stalls without saying why.

#[cfg(test)]
#[path = "tests_scripts.rs"]
mod tests;

/// `documentHelpers()` (`:9-40`): `visibleElement`, `downloadForm`, `agreementBox`,
/// `agreementConfirm`. The `#mode` letters are the six the site accepts (`:18`).
const DOCUMENT_HELPERS: &str = "\
function visibleElement(e){if(!e)return false;\
if(!e.isConnected)return false;\
for(var p=e;p&&p.nodeType===1;p=p.parentElement){\
var s=p.ownerDocument.defaultView.getComputedStyle(p);\
if(s.display==='none'||s.visibility==='hidden'||s.visibility==='collapse'||s.opacity==='0')return false;}\
return true;}\
function downloadForm(d){var f=d.getElementById('searchForm');\
var m=f&&f.querySelector('#mode');return f&&m&&/^[SPBURE]$/.test(m.value)&&visibleElement(f)?f:null;}\
function agreementBox(d){var boxes=d.querySelectorAll('input[type=checkbox]');\
for(var i=0;i<boxes.length;i++){var b=boxes[i],shown=visibleElement(b);\
if(b.labels)for(var j=0;j<b.labels.length;j++)shown=shown||visibleElement(b.labels[j]);\
if(!shown)continue;\
var t=b.parentElement?b.parentElement.innerText:'';\
if(b.labels)for(var j=0;j<b.labels.length;j++)t+=' '+b.labels[j].innerText;\
if(t.replace(/\\s+/g,'').indexOf('서약서에동의')>=0)return b;}return null;}\
function agreementConfirm(d,b){\
for(var p=b.parentElement;p&&p!==d.body;p=p.parentElement){\
var nodes=p.querySelectorAll('button,input[type=button],input[type=submit],a'),hits=[];\
for(var i=0;i<nodes.length;i++)if(visibleElement(nodes[i])&&\
(nodes[i].innerText||nodes[i].value||'').replace(/\\s+/g,'')==='확인')hits.push(nodes[i]);\
if(hits.length)return hits.length===1?hits[0]:null;}return null;}";

/// `wrap()` (`:43-92`): find the frame that holds the download panel, rebind `document`,
/// then run `body`. `%2` takes the helpers and `%1` the body, in that order, because the
/// helpers are inserted above the body.
fn wrap(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 2048);
    out.push_str(
        "(function(){\
         function walk(w,out,depth){\
         if(!w||depth>6)return;\
         out.push(w);\
         try{for(var i=0;i<w.frames.length;i++){\
         try{void w.frames[i].document.body;walk(w.frames[i],out,depth+1);}\
         catch(e){}}}catch(e){}}\
         function cands(){var out=[];walk(window,out,0);return out;}\
         ",
    );
    out.push_str(DOCUMENT_HELPERS);
    out.push_str(
        "\
         function hasCodedeta(w){\
         try{return !!downloadForm(w.document);}\
         catch(e){return false;}}\
         function pick(){var c=cands();\
         for(var i=0;i<c.length;i++){if(hasCodedeta(c[i]))return c[i];}\
         for(var i=0;i<c.length;i++){\
         try{var t=(c[i].document.body&&c[i].document.body.innerText)||'';\
         if(t.indexOf('지정유산')>=0&&t.indexOf('발굴조사구역')>=0&&\
         (t.indexOf('전체')>=0||t.indexOf('시도')>=0))return c[i];}catch(e){}}\
         for(var i=0;i<c.length;i++){\
         try{var t=((c[i].document.body&&c[i].document.body.innerText)||'').replace(/\\s+/g,'');\
         if(t.indexOf('서약서에동의')>=0)return c[i];}catch(e){}}\
         for(var i=0;i<c.length;i++){\
         try{if(typeof c[i].showAgreePopup==='function')return c[i];}catch(e){}}\
         return window;}\
         function fn(name){\
         try{if(typeof W[name]==='function')return W[name];}catch(e){}\
         var c=cands();for(var i=0;i<c.length;i++){\
         try{if(typeof c[i][name]==='function')return c[i][name];}catch(e){}}\
         return null;}\
         var W=pick();var document=W.document;\
         try{",
    );
    out.push_str(body);
    out.push_str("}catch(e){return 'error:'+String(e);}})()");
    out
}

/// `jsString()` (`:125-133`): the value as a JavaScript string **literal**, quotes included.
///
/// The C++ serialises a one-element array (`["user1"]`) and cuts only the brackets, so the
/// quotes stay and the escaping is JSON's. Stripping the quotes as well turns `user1` into a
/// bare identifier and the page answers `ReferenceError: user1 is not defined` — which is
/// exactly what the first real run against the fixture did.
#[must_use]
pub fn js_string(value: &str) -> String {
    serde_json::Value::String(value.to_owned()).to_string()
}

/// `loginUrl()` (`:98-100`). The site's login landing page.
pub const LOGIN_URL: &str = "https://intranet.gis-heritage.go.kr/user/checkNet.do";
/// `portalUrl()` (`:94-96`).
pub const PORTAL_URL: &str = "https://intranet.gis-heritage.go.kr/";
/// `downloadPagePath()` (`:203-205`).
pub const DOWNLOAD_PAGE_PATH: &str = "/user/data/heritageDownload.do?pageMenuId=DAT010010";
/// `downloadFilesAllPath()` (`:211-213`).
pub const DOWNLOAD_FILES_ALL_PATH: &str = "/user/data/heritageDownload/downloadFilesAll.do";

/// The six kinds and their tab letters (`HeritageStyle.cpp` entry table). The Korean names
/// must equal `strata_refmap::heritage::DATASETS[].name`; a test holds that.
pub const KINDS: [(&str, &str); 6] = [
    ("S", "지정유산"),
    ("P", "현상변경허용기준"),
    ("B", "매장유산유존지역"),
    ("U", "문화유적분포지도"),
    ("R", "지표조사구역"),
    ("E", "발굴조사구역"),
];

/// `loginScript` (`:135-148`): fill `#userId`/`#pwd` and call the site's own `goLogin()`.
/// A plain POST does not log in — the page RSA-encrypts into hidden `USER_ID`/`USER_PW`.
#[must_use]
pub fn login_script(user_id: &str, password: &str) -> String {
    wrap(&format!(
        "var id=document.getElementById('userId');\
         var pw=document.getElementById('pwd');\
         if(!id||!pw){{return 'no-form';}}\
         id.value={}; pw.value={};\
         if(typeof goLogin!=='function'){{return 'no-goLogin';}}\
         goLogin(); return 'submitted';",
        js_string(user_id),
        js_string(password)
    ))
}

/// `loginProbeScript` (`:150-160`). A blank or still-loading frame is not a login.
#[must_use]
pub fn login_probe_script() -> String {
    wrap("var frames=cands();var login=false;\
          for(var i=0;i<frames.length;i++){var d;try{d=frames[i].document;}catch(e){continue;}\
          var body=d.body?d.body.innerText:'';\
          if(body.indexOf('로그아웃')>=0)return 'past-login';\
          var u=d.getElementById('userId'),p=d.getElementById('pwd');\
          if(visibleElement(u)&&visibleElement(p))login=true;}\
          return login?'still-login':'pending';")
}

/// `dismissTutorialScript` (`:162-201`). The tutorial appears after login and hides the top
/// menu until dismissed; only a **visible** mark is clicked, and the 「다시보지 않기」 box is
/// ticked before the exit button so the walk does not step through 1/5 … 5/5.
#[must_use]
pub fn dismiss_tutorial_script() -> String {
    wrap("function squash(t){return (t||'').replace(/\\s+/g,'');}\
          function vis(el){if(!el)return false;\
          if(el.offsetParent===null&&getComputedStyle(el).position!=='fixed')return false;\
          var r=el.getBoundingClientRect();\
          if(r.width<=0||r.height<=0)return false;\
          var st=getComputedStyle(el);\
          return st.visibility!=='hidden'&&st.display!=='none'&&parseFloat(st.opacity||'1')>0.05;}\
          function findInner(mark){\
          var all=document.querySelectorAll('button,a,span,div,li,p');var hit=null;\
          for(var i=0;i<all.length;i++){\
          if(squash(all[i].innerText).indexOf(mark)<0)continue;\
          if(!vis(all[i]))continue;\
          var inner=false;var kids=all[i].querySelectorAll('*');\
          for(var k=0;k<kids.length;k++){\
          if(vis(kids[k])&&squash(kids[k].innerText).indexOf(mark)>=0){inner=true;break;}}\
          if(!inner){hit=all[i];}}\
          return hit;}\
          var exit=findInner('튜토리얼끄기');\
          var again=findInner('다시보지않기');\
          if(!exit&&!again)return 'none';\
          if(again){\
          var box=again.querySelector('input[type=checkbox]')||\
          (again.parentElement?again.parentElement.querySelector('input[type=checkbox]'):null);\
          if(box&&!box.checked){try{box.click();}catch(e){}\
          if(!box.checked){box.checked=true;\
          box.dispatchEvent(new Event('change',{bubbles:true}));}}}\
          if(exit){exit.click();return 'closed';}\
          return 'closed';")
}

/// `openDownloadPageScript` (`:215-228`). The download page is opened through the site's own
/// `showAgreePopup()`; navigating straight to `heritageDownload.do` gives a fragment with no
/// functions and the run repeats "no-fn" forever (`:217-221`).
#[must_use]
pub fn open_download_page_script() -> String {
    wrap("var c=cands();\
          for(var i=0;i<c.length;i++){\
          try{if(downloadForm(c[i].document)||agreementBox(c[i].document))return 'already-open';}catch(e){}}\
          for(var i=0;i<c.length;i++){\
          try{if(typeof c[i].showAgreePopup==='function'){c[i].showAgreePopup();return 'opening';}}\
          catch(e){return 'error:'+String(e);}}\
          return 'not-found';")
}

/// `agreeTermsScript` (`:230-243`). Ticks 「서약서에 동의」 and presses 확인, then answers the
/// pledge text the page showed so the receipt can hold it.
#[must_use]
pub fn agree_terms_script() -> String {
    wrap("if(downloadForm(document))return 'already-open';\
          var target=agreementBox(document);if(!target)return 'not-found';\
          var confirm=agreementConfirm(document,target);if(!confirm)return 'checked-no-confirm';\
          var scope=target.parentElement;while(scope&&!scope.contains(confirm))scope=scope.parentElement;\
          var receipt=scope?(scope.innerText||'').trim():'';if(!receipt)return 'not-found';\
          if(!target.checked){target.click();\
          if(!target.checked){target.checked=true;\
          target.dispatchEvent(new Event('change',{bubbles:true}));}}\
          confirm.click();return JSON.stringify({status:'agreed',text:receipt});")
}

/// `selectDatasetScript` (`:245-261`). Uses the verified download tab, not an earlier map-menu
/// LI with the same text, and keeps the old form node (two kinds can have the same count).
#[must_use]
pub fn select_dataset_script(code: &str) -> String {
    wrap(&format!(
        "var oldForm=document.getElementById('searchForm');\
         if(!oldForm)return 'not-found';\
         var code={};var mode=document.getElementById('mode');\
         W.__kaHeritagePreviousForm=oldForm;\
         var tab=fn('changeTab');\
         if(!tab){{return 'no-changeTab';}}\
         tab({}); return 'selected';",
        js_string(code),
        js_string(code)
    ))
}

/// `datasetReadyScript` (`:263-270`). The list AJAX replaces `#tabContentDiv`; the form node
/// changing is what proves the new kind is in.
#[must_use]
pub fn dataset_ready_script(code: &str) -> String {
    wrap(&format!(
        "var form=document.getElementById('searchForm');\
         if(!W.__kaHeritagePreviousForm||!form||form===W.__kaHeritagePreviousForm)return 'pending';\
         var code={};var mode=document.getElementById('mode');\
         if(code&&(!mode||mode.value!==code))return 'pending';\
         return 'ready';",
        js_string(code)
    ))
}

/// `selectRegionScript` (`:272-328`). The download page has two `select` pairs; the 지적
/// pair (`bjdcd…`) comes first in the DOM and is not the one. The download pair
/// (`codedeta…`) is found first and the 시·군 is the **next select in the same form**.
#[must_use]
pub fn select_region_script(sido: &str, city: &str) -> String {
    wrap(&format!(
        "function squash(t){{return (t||'').replace(/\\s+/g,'');}}\
         function core(t){{var s=squash(t);\
         s=s.replace('특별자치시','').replace('특별자치도','')\
         .replace('특별시','').replace('광역시','');\
         if(s.length>2&&s.charAt(s.length-1)==='도')s=s.slice(0,-1);\
         var map={{'전라북':'전북','전라남':'전남','경상북':'경북','경상남':'경남',\
         '충청북':'충북','충청남':'충남'}};\
         return map[s]||s;}}\
         function same(a,b){{var x=squash(a),y=squash(b);\
         if(!x||!y)return false;\
         if(x===y||x.indexOf(y)>=0||y.indexOf(x)>=0)return true;\
         var cx=core(x),cy=core(y);\
         if(cx===cy)return true;\
         function merged(c,raw){{return (c==='전남'||c==='광주')&&raw.indexOf('전남광주')>=0;}}\
         return merged(cx,y)||merged(cy,x);}}\
         function firstOption(sel){{return sel.options.length?squash(sel.options[0].text):'';}}\
         function key(sel){{return ((sel.id||'')+' '+(sel.name||'')).toLowerCase();}}\
         var df=downloadForm(document);\
         var all=Array.prototype.slice.call((df||document).querySelectorAll('select'));\
         var sidoAt=-1;\
         for(var i=0;i<all.length;i++){{\
         if(key(all[i]).indexOf('codedeta')>=0&&firstOption(all[i]).indexOf('시도선택')>=0){{sidoAt=i;break;}}}}\
         if(sidoAt<0){{for(var i=0;i<all.length;i++){{\
         if(!df&&key(all[i]).indexOf('bjdcd')>=0)continue;\
         var f=firstOption(all[i]);\
         if(f.indexOf('시도선택')>=0||f.indexOf('광역시선택')>=0||f.indexOf('전체선택')>=0){{sidoAt=i;break;}}}}}}\
         if(sidoAt<0)return 'not-found';\
         var sidoSel=all[sidoAt];\
         var citySel=null;\
         for(var i=sidoAt+1;i<all.length;i++){{\
         if(sidoSel.form&&all[i].form!==sidoSel.form)continue;\
         var f=firstOption(all[i]);\
         if(f.indexOf('시군구선택')>=0||f.indexOf('시군선택')>=0||f.indexOf('시·군·구선택')>=0){{citySel=all[i];break;}}}}\
         function pick(sel,text,exact){{if(!sel)return false;\
         for(var i=0;i<sel.options.length;i++){{\
         if(exact?squash(sel.options[i].text)!==squash(text):!same(sel.options[i].text,text))continue;\
         if(sel.selectedIndex!==i){{sel.selectedIndex=i;\
         sel.dispatchEvent(new Event('input',{{bubbles:true}}));\
         sel.dispatchEvent(new Event('change',{{bubbles:true}}));}}\
         return true;}}\
         return false;}}\
         if(!pick(sidoSel,{},false))return 'no-sido';\
         if(!citySel)return 'sigungu-pending';\
         if(pick(citySel,{},true))return 'selected';\
         return 'sigungu:'+(citySel.options.length-1);",
        js_string(sido),
        js_string(city)
    ))
}

/// `maximizePageSizeScript` (`:330-344`). Picks the largest 「N개씩 보기」 so fewer pages are
/// walked; when there is no such select the run just carries on.
#[must_use]
pub fn maximize_page_size_script() -> String {
    wrap("var all=document.querySelectorAll('select');\
          for(var i=0;i<all.length;i++){\
          var txt=(all[i].options.length?all[i].options[0].text:'')||'';\
          if(txt.indexOf('개씩')<0)continue;\
          var best=0,bestN=-1;\
          for(var j=0;j<all[i].options.length;j++){\
          var n=parseInt((all[i].options[j].text||'').replace(/[^0-9]/g,''),10);\
          if(!isNaN(n)&&n>bestN){bestN=n;best=j;}}\
          if(bestN>0){all[i].selectedIndex=best;\
          all[i].dispatchEvent(new Event('change',{bubbles:true}));return 'max:'+bestN;}}\
          return 'none';")
}

/// `formHintScript` (`:346-355`). The only script that does **not** wrap: it reports whether
/// the frame it runs in holds the panel, which is how the C++ picks the frame to talk to.
#[must_use]
pub fn form_hint_script() -> String {
    format!(
        "(function(){{{DOCUMENT_HELPERS}\
         try{{\
         if(downloadForm(document))return 'codedeta';\
         return 'other';\
         }}catch(e){{return 'error';}}}})()"
    )
}

/// `searchScript` (`:357-394`). Presses 검색 in the same frame that holds `codedeta`, without
/// navigating the page: the 지적 panel on the left also has a 검색 button, so the scope must
/// be narrowed or the run searches the wrong thing.
#[must_use]
pub fn search_script() -> String {
    wrap("function squash(t){return (t||'').replace(/\\s+/g,'');}\
          function key(e){return ((e.id||'')+' '+(e.name||'')).toLowerCase();}\
          function findSearch(root){\
          if(!root||!root.querySelectorAll)return null;\
          var n=root.querySelectorAll('button,input[type=button],input[type=submit],a');\
          for(var i=0;i<n.length;i++){\
          if(squash(n[i].innerText||n[i].value)==='검색')return n[i];}\
          return null;}\
          var c=cands();\
          for(var w=0;w<c.length;w++){\
          var doc;try{doc=c[w].document;}catch(e){continue;}\
          var df=downloadForm(doc);var anchor=df?df.querySelector('select'):null;var sels=doc.querySelectorAll('select');\
          for(var i=0;!anchor&&i<sels.length;i++){\
          if(key(sels[i]).indexOf('codedeta')>=0){anchor=sels[i];break;}}\
          if(!anchor)continue;\
          var scope=doc.getElementById('searchForm');\
          var btn=scope?findSearch(scope):null;\
          if(!btn){var p=anchor;\
          for(var d=0;d<10&&p;d++){p=p.parentElement;if(!p)break;\
          var found=findSearch(p);\
          if(found){btn=found;break;}}}\
          if(btn){c[w].__kaHeritageSearchNode=doc.querySelector('#tabContentDiv table tbody')||scope;\
          btn.click();return 'searching';}}\
          return 'not-found';")
}

/// `resultPageInfoScript` (`:396-413`). How many rows and pages came back, and whether the
/// search actually ran (the remembered table body left the document).
#[must_use]
pub fn result_page_info_script() -> String {
    wrap("var rows=document.querySelectorAll('table tbody tr');\
          var pages=[];var links=document.querySelectorAll('a');\
          for(var i=0;i<links.length;i++){\
          var t=(links[i].innerText||'').trim();\
          if(/^[0-9]+$/.test(t)){pages.push(parseInt(t,10));}}\
          var last=pages.length?Math.max.apply(null,pages):1;\
          var total=-1;\
          var body=document.body?document.body.innerText:'';\
          var m=body.match(/검색결과[^0-9]*([0-9,]+)/);\
          if(m)total=parseInt(m[1].replace(/,/g,''),10);\
          var previous=W.__kaHeritageSearchNode;\
          var searchReady=!!previous&&!previous.isConnected;\
          return JSON.stringify({rows:rows.length,lastPage:last,total:total,searchReady:searchReady});")
}

/// `downloadAllScript` (`:415-440`). Clicks the real 「전체다운로드」 button. The button's own
/// handler must run (it builds the request); the page's other buttons and a separate URL are
/// not used.
#[must_use]
pub fn download_all_script() -> String {
    wrap("function squash(t){return (t||'').replace(/\\s+/g,'');}\
          var form=downloadForm(document);if(!form)return 'not-found';\
          var sido=form.querySelector('select[name=codedetaCd],select[name=bjdCd]');\
          var sgg=form.querySelector('select[name=codeCdSg],select[name=bjdCd1]');\
          if(!sido||!sido.value)return 'no-region';\
          if(!sgg||!sgg.value)return 'no-region';\
          var panel=form.closest('#tabContentDiv');if(!panel)return 'not-found';\
          var n=panel.querySelectorAll('button,input[type=button],input[type=submit],a'),hits=[];\
          for(var i=0;i<n.length;i++)if(visibleElement(n[i])&&!n[i].disabled&&\
          n[i].getAttribute('aria-disabled')!=='true'&&\
          squash(n[i].innerText||n[i].value)==='전체다운로드')hits.push(n[i]);\
          if(!hits.length)return 'not-found';\
          if(hits.length!==1)return 'ambiguous';\
          var error='';var host=document.defaultView;\
          function onError(e){error=(e.error&&e.error.name)||'JavaScriptError';}\
          host.addEventListener('error',onError);\
          try{hits[0].click();}catch(e){error=e.name||'JavaScriptError';}\
          finally{host.removeEventListener('error',onError);}\
          return error?('error:'+error):'all';")
}

/// `goToPageScript` (`:442-450`).
#[must_use]
pub fn go_to_page_script(page_number: i32) -> String {
    wrap(&format!(
        "var want=String({page_number});\
         var links=document.querySelectorAll('a');\
         for(var i=0;i<links.length;i++){{\
         if((links[i].innerText||'').trim()===want){{links[i].click();return 'moved';}}}}\
         return 'last-page';"
    ))
}

/// `downloadPageProbeScript` (`:470-478`). After 「전체다운로드」 the site answers with a page,
/// not a file; this tells "panel is back, try again" from "we were taken somewhere else".
#[must_use]
pub fn download_page_probe_script() -> String {
    wrap("var body=document.body?document.body.innerText:'';\
          if(/찾을\\s*수\\s*없는\\s*페이지/.test(body))return 'page-error';\
          if(downloadForm(document)||agreementBox(document))return 'ready';\
          return 'not-yet';")
}
