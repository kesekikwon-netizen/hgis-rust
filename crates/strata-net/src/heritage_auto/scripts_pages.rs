//! Download document scripts from `target/cpp-ref/src/core/HeritageIntranetFlow.cpp:245-478`.

use super::{js_string, wrap};

/// `selectDatasetScript` (`:245-261`). Uses the verified download tab, not an earlier map-menu
/// LI with the same text, and keeps the old form node (two kinds can have the same count).
#[must_use]
pub fn select_dataset_script(code: &str) -> String {
    wrap(&format!(
        "var oldForm=document.getElementById('searchForm');\
         if(!oldForm)return 'not-found';\
         var code={};var mode=document.getElementById('mode');\
         W.__kaHeritagePreviousForm=oldForm;W.__kaHeritageExpectedMode=code;\
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
         if(f.indexOf('시도선택')>=0||f.indexOf('시·도선택')>=0||f.indexOf('시/도선택')>=0){{sidoAt=i;break;}}}}}}\
         if(sidoAt<0)return 'not-found';\
         var sidoSel=all[sidoAt];\
         var citySel=null;\
         for(var i=sidoAt+1;i<all.length;i++){{\
         if(sidoSel.form&&all[i].form!==sidoSel.form)continue;\
         var f=firstOption(all[i]);\
         if(f.indexOf('시/군/구선택')>=0||f.indexOf('시군구선택')>=0||f.indexOf('시·군·구선택')>=0){{citySel=all[i];break;}}}}\
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
    wrap(
        "var all=document.querySelectorAll('select');\
          for(var i=0;i<all.length;i++){\
          var txt=(all[i].options.length?all[i].options[0].text:'')||'';\
          if(txt.indexOf('개씩')<0)continue;\
          var best=0,bestN=-1;\
          for(var j=0;j<all[i].options.length;j++){\
          var n=parseInt((all[i].options[j].text||'').replace(/[^0-9]/g,''),10);\
          if(!isNaN(n)&&n>bestN){bestN=n;best=j;}}\
          if(bestN>0){all[i].selectedIndex=best;\
          all[i].dispatchEvent(new Event('change',{bubbles:true}));return 'max:'+bestN;}}\
          return 'none';",
    )
}

/// The C++ targets native child frames (`HeritageIntranetFlow.cpp:346-355`);
/// the Rust host targets a view, so the same readback walks its child documents.
#[must_use]
pub fn form_hint_script() -> String {
    wrap("return downloadForm(document)?'codedeta':'other';")
}

/// Actual region values, as read by `HeritageIntranetFlow.cpp:622-639`.
#[must_use]
pub fn verify_region_script() -> String {
    wrap("var df=downloadForm(document);\
          function pickSel(kind){var all=(df||document).querySelectorAll('select');\
          for(var i=0;i<all.length;i++){var k=((all[i].id||'')+' '+(all[i].name||'')).toLowerCase();\
          if(k.indexOf('bjdcd')>=0)continue;if(k.indexOf(kind)>=0)return all[i];}return null;}\
          var a=(df?df.querySelector('select[name=codedetaCd],select[name=bjdCd]'):null)||pickSel('codedeta');\
          var b=(df?df.querySelector('select[name=codeCdSg],select[name=bjdCd1]'):null)||pickSel('codecdsg');\
          if(!a||!a.value||!b||!b.value)return 'empty';\
          return 'ok:'+a.value+'|'+b.value;")
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
    wrap("var form=downloadForm(document),mode=form&&form.querySelector('#mode');\
          if(W.__kaHeritageExpectedMode&&(!mode||mode.value!==W.__kaHeritageExpectedMode))return 'wrong-kind';\
          var rows=document.querySelectorAll('table tbody tr');\
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
    wrap(
        "function squash(t){return (t||'').replace(/\\s+/g,'');}\
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
          return error?('error:'+error):'all';",
    )
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
    wrap(
        "var body=document.body?document.body.innerText:'';\
          if(/찾을\\s*수\\s*없는\\s*페이지/.test(body))return 'page-error';\
          if(downloadForm(document)||agreementBox(document))return 'ready';\
          return 'not-yet';",
    )
}
