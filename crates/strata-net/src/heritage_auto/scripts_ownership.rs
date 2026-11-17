//! Pure document inventory (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:639-778`).

use super::wrap;

/// Capabilities only; no field values or page text cross the browser boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub struct DocumentFacts {
    pub form: bool,
    pub agreement: bool,
    pub login: bool,
    pub menu: bool,
    pub authenticated: bool,
}

/// WebView2 returns a JSON string around the script's JSON result.
#[must_use]
pub fn parse_document_probe(json: &str) -> Option<DocumentFacts> {
    let decoded = serde_json::from_str::<String>(json).ok();
    serde_json::from_str(decoded.as_deref().unwrap_or(json)).ok()
}

/// Observe existing same-origin frames without invoking any site function.
#[must_use]
pub fn document_probe_script() -> String {
    wrap("var facts={form:false,agreement:false,login:false,menu:false,authenticated:false};\
          var frames=cands();for(var i=0;i<frames.length;i++){try{\
          var w=frames[i],d=w.document;\
          facts.form=facts.form||!!downloadForm(d);\
          facts.agreement=facts.agreement||!!agreementBox(d);\
          facts.login=facts.login||(visibleElement(d.getElementById('userId'))&&visibleElement(d.getElementById('pwd')));\
          facts.menu=facts.menu||typeof w.showAgreePopup==='function';\
          facts.authenticated=facts.authenticated||!!(d.body&&d.body.innerText.indexOf('로그아웃')>=0);\
          }catch(e){}}return JSON.stringify(facts);")
}

#[cfg(test)]
#[path = "tests_scripts_ownership.rs"]
mod tests;
