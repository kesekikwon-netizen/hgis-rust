//! `heritage_intranet` 는 국가유산 공간정보 인트라넷 기능의 재수출 창구다.
//!
//! 원래 1548줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::heritage_intranet::<이름>` 경로는 그대로 동작한다.
//!
//! C++ 정본 그대로 옮겼다.
//! - 단계 이름·주소: `HeritageIntranetFlow.cpp:86-110`, `:203-213`
//! - 요청 헤더: `HeritageHttpClient.cpp:17-19`, `:81-100`
//! - 폼 해석: `HeritageFormParser.cpp:9-212`
//! - 전체다운로드 주소: `HeritageIntranetFlow.cpp:573-619` (요청 기록에서 확인한 모양)
//! - 실패 문구: `HeritageHttpClient.cpp:104-198`, `KaHeritageBrowser.cpp:598-1523`

#[path = "heritage_stage.rs"]
mod heritage_stage;
#[path = "heritage_form.rs"]
mod heritage_form;
#[path = "heritage_form_parse.rs"]
mod heritage_form_parse;
#[path = "heritage_flow.rs"]
mod heritage_flow;
#[path = "heritage_flow_body.rs"]
mod heritage_flow_body;
#[path = "heritage_flow_script.rs"]
mod heritage_flow_script;
#[path = "heritage_http.rs"]
mod heritage_http;
#[path = "heritage_pipeline.rs"]
mod heritage_pipeline;
#[path = "heritage_pipeline_http.rs"]
mod heritage_pipeline_http;

pub use heritage_flow::*;
pub use heritage_form::*;
pub use heritage_form_parse::*;
pub use heritage_http::*;
pub use heritage_pipeline::*;
pub use heritage_pipeline_http::*;
pub use heritage_stage::*;

/// 인트라넷이 실패를 알리는 문구. C++ 문장을 그대로 쓴다.
pub use heritage_stage::messages;

#[cfg(test)]
#[path = "heritage_intranet_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "heritage_intranet_tests_flow.rs"]
mod tests_flow;
