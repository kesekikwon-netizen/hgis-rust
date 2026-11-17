//! The heritage intranet page scripts, based on
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
    wrap(
        "var frames=cands();var login=false;\
          for(var i=0;i<frames.length;i++){var d;try{d=frames[i].document;}catch(e){continue;}\
          var body=d.body?d.body.innerText:'';\
          if(body.indexOf('로그아웃')>=0)return 'past-login';\
          var u=d.getElementById('userId'),p=d.getElementById('pwd');\
          if(visibleElement(u)&&visibleElement(p))login=true;}\
          return login?'still-login':'pending';",
    )
}

/// `dismissTutorialScript` (`:162-201`). The tutorial appears after login and hides the top
/// menu until dismissed; only a **visible** mark is clicked, and the 「다시보지 않기」 box is
/// ticked before the exit button so the walk does not step through 1/5 … 5/5.
#[must_use]
pub fn dismiss_tutorial_script() -> String {
    wrap(
        "function squash(t){return (t||'').replace(/\\s+/g,'');}\
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
          return 'closed';",
    )
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

#[path = "scripts_pages.rs"]
mod pages;
pub use pages::*;
