//! 브라우저에 주입하는 스크립트와 프레임 허용 목록.
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

impl HeritageIntranetFlow {
    pub fn login_script(user_id: &str, password: &str) -> String {
        format!(
            "(function(){{var id=document.getElementById('userId');\
             var pw=document.getElementById('pwd');\
             if(!id||!pw)return 'no-form';\
             id.value={};pw.value={};\
             if(typeof goLogin!=='function')return 'no-goLogin';\
             goLogin();return 'submitted';}})()",
            Self::js_string(user_id),
            Self::js_string(password)
        )
    }

    pub fn open_download_page_script() -> &'static str {
        "(function(){if(typeof showAgreePopup==='function'){showAgreePopup();return 'opening';}return 'not-found';})()"
    }

    pub fn search_script() -> String {
        String::from(
            "(function(){var scope=document.getElementById('searchForm');\
             if(!scope)return 'not-found';\
             var btns=scope.querySelectorAll('button,input[type=button],input[type=submit],a');\
             for(var i=0;i<btns.length;i++){\
               var t=(btns[i].innerText||btns[i].value||'').replace(/\\s+/g,'');\
               if(t==='검색'){btns[i].click();return 'searching';}\
             }\
             return 'not-found';})()",
        )
    }

    pub fn redact_request_line(method: &str, url: &str) -> String {
        let path = url.split('?').next().unwrap_or(url);
        let lower = path.to_lowercase();
        if lower.contains("security_check") || lower.contains("login") || lower.contains("checknet")
        {
            return format!("{method} <로그인 요청 — 내용을 남기지 않음>");
        }
        if let Some((_, q)) = url.split_once('?') {
            let short_q: String = q.chars().take(400).collect();
            format!("{method} {path}?{short_q}")
        } else {
            format!("{method} {path}")
        }
    }

    pub const ALLOWED_EHERITAGE_HOSTS: [&'static str; 6] = [
        "heritage.go.kr",
        "gis.heritage.go.kr",
        "gis-heritage.go.kr",
        "e-heritage.go.kr",
        "db.history.go.kr",
        "koreanhistory.or.kr",
    ];

    /// C++ `EHeritageIntranetClient::isAllowedFrameUrl` — 허용 도메인 화이트리스트 대조 및 외부 리다이렉트 차단
    pub fn is_allowed_frame_url(url_str: &str) -> Result<(), String> {
        let trimmed = url_str.trim();
        let after_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .ok_or_else(|| format!("허용되지 않은 외부 도메인 접근 차단: [{trimmed}]"))?;
        let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
        let host = authority
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if host.is_empty() {
            return Err("허용되지 않은 외부 도메인 접근 차단: []".to_owned());
        }
        let allowed = Self::ALLOWED_EHERITAGE_HOSTS.iter().any(|&allowed_host| {
            host == allowed_host || host.ends_with(&format!(".{allowed_host}"))
        });
        if allowed {
            Ok(())
        } else {
            Err(format!("허용되지 않은 외부 도메인 접근 차단: [{host}]"))
        }
    }
}
