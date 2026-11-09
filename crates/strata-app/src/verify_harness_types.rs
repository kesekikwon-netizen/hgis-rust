//! 검증 게이트 한 칸의 결과 타입.
//!
//! `verify_harness.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 타입은 그대로다.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub title_ko: &'static str,
    pub passed: bool,
    pub detail: String,
}
