//! 검색·전체다운로드 폼 본문 만들기 (C++ `HeritageIntranetFlow.cpp:573-619`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use super::heritage_form_parse::percent_encode;

use super::*;

impl HeritageIntranetFlow {
    /// `HeritageHttpClient::buildBody` (`HeritageHttpClient.cpp:53-74`)
    /// 폼의 칸을 그대로 되돌려 보내고 시/도·시/군/구만 바꾼다.
    #[must_use]
    pub fn build_search_body(
        form: &HeritageForm,
        sido_value: &str,
        sigungu_value: &str,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (name, value) in &form.fields {
            let mut v = value.as_str();
            if !form.sido_field.is_empty() && *name == form.sido_field {
                v = sido_value;
            }
            if !form.sigungu_field.is_empty() && *name == form.sigungu_field {
                v = sigungu_value;
            }
            parts.push(format!(
                "{}={}",
                percent_encode(name),
                percent_encode(v)
            ));
        }
        // 폼에 없던 칸이면 새로 넣는다(select 가 form 밖에 있는 화면 대비).
        for (name, value) in [
            (form.sido_field.as_str(), sido_value),
            (form.sigungu_field.as_str(), sigungu_value),
        ] {
            if name.is_empty() || form.field_value(name).is_some() {
                continue;
            }
            parts.push(format!(
                "{}={}",
                percent_encode(name),
                percent_encode(value)
            ));
        }
        // 사이트 JS 와 같은 규칙: codedetaCd 값이 "A,B" 꼴이면 뒤쪽을 codeCd 로 함께 보낸다.
        let code_cd = sido_value.split(',').nth(1).unwrap_or("");
        parts.push(format!("codeCd={}", percent_encode(code_cd)));
        parts.join("&")
    }

    /// 사이트의 전체다운로드 주소를 **우리가 만든다** (`HeritageIntranetFlow.cpp:108-112`,
    /// `:573-619`). 성공한 요청에서 확인한 모양 그대로다:
    /// `/user/data/heritageDownload/downloadFilesAll.do?<searchForm 직렬화>&codeCd=..&tab=..`
    #[must_use]
    pub fn build_download_files_all_url(
        form: &HeritageForm,
        sido_value: &str,
        sigungu_value: &str,
        tab_code: &str,
        region_label: &str,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (name, value) in &form.fields {
            let mut v = value.as_str();
            if !form.sido_field.is_empty() && *name == form.sido_field {
                v = sido_value;
            }
            if !form.sigungu_field.is_empty() && *name == form.sigungu_field {
                v = sigungu_value;
            }
            parts.push(format!("{}={}", percent_encode(name), percent_encode(v)));
        }
        let code_cd = sido_value.split(',').nth(1).unwrap_or("");
        parts.push(format!("codeCd={}", percent_encode(code_cd)));
        let mode = form.field_value("mode").filter(|m| !m.is_empty());
        let tab = mode.unwrap_or(tab_code);
        parts.push(format!("tab={}", percent_encode(tab)));
        if mode.is_none() {
            parts.push(format!("mode={}", percent_encode(tab)));
        }
        // searchParams 는 화면에 보여 줄 요약. 날것으로 만든다(인코딩하지 않는다).
        let label = if region_label.is_empty() {
            String::new()
        } else {
            format!("지역:{region_label} / ")
        };
        parts.push(format!(
            "searchParams={}",
            if label.is_empty() {
                String::new()
            } else {
                percent_encode(&label)
            }
        ));
        format!("{}?{}", Self::download_files_all_url(), parts.join("&"))
    }
}
