//! 홈 「연결 상태」 글 — 가짜 상태만(저장소는 읽지 않는다).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use strata_secrets::account_status::{snapshot, Source};

use super::*;

/// ported from tests/test_home_cards.cpp:connection_rows… — 이름·상태 글, 비밀값·「연결됨」 없음.
#[test]
fn rows_show_display_names_and_state_words() {
    let entries = snapshot(&|s| s == Source::VworldKey);
    let rows = conn_rows(&entries, false);
    assert_eq!(rows.len(), 5);
    assert_eq!((rows[0].state, rows[1].state), ("설정됨", "설정 필요"));
    let names: Vec<&str> = rows.iter().map(|r| r.name).collect();
    assert_eq!(names, ["VWorld 키", "역사지리정보DB 키", "VWorld 지적도 계정", "수치지형도 계정", "문화재 인트라넷"]);
    assert!(rows.iter().all(|r| !r.state.contains("연결됨") && !r.missing_dot), "no plate, no dot");
    assert_eq!(rows[0].name_tip, "VWorld 키 — 배경 지도 · 지적 그림 · 시·군 판정 · 더보기 → API 키 입력");
    assert_eq!(rows[0].state_tip, None);
    assert_eq!(rows[3].state_tip.as_deref(), Some("더보기 → 수치지형도 아이디·비밀번호에서 넣습니다."));
    assert_eq!(rows[3].setup_tip, "더보기 → 수치지형도 아이디·비밀번호");
}

/// ported from tests/test_home_plate.cpp:connectionRowIsPlainWords — 종이 모양은 「설정됨」 그대로, 없는 것만 「● 설정 필요」.
#[test]
fn plate_marks_only_the_missing_setting() {
    let rows = conn_rows(&snapshot(&|s| s == Source::VworldKey), true);
    assert!(!rows[0].missing_dot && rows[0].state == "설정됨");
    assert!(rows[3].missing_dot && rows[3].state == "설정 필요");
}

/// checkInternet — 누르기 전 「확인 안 함」, 확인 중엔 단추를 막고, 답이 오면 점과 글(안 닿으면 도움말).
#[test]
fn internet_label_follows_the_check() {
    let idle = internet_label(&InternetCheck::NotChecked);
    assert_eq!((idle.text, idle.dot, idle.tip, idle.button_enabled), ("인터넷 · 확인 안 함", None, None, true));
    let slot = Arc::new(Mutex::new(None));
    let mut check = InternetCheck::Checking(slot.clone());
    let busy = internet_label(&check);
    assert_eq!((busy.text, busy.button_enabled), ("인터넷 · 확인 중…", false));
    assert!(!check.poll(), "no answer yet");
    *slot.lock().unwrap() = Some(false);
    assert!(check.poll());
    let off = internet_label(&check);
    assert_eq!((off.text, off.dot, off.button_enabled), ("인터넷 · 연결 안 됨", Some(false), true));
    assert!(off.tip.unwrap().starts_with("오프라인이어도 조사 열기·그리기·저장은 됩니다."));
    let on = internet_label(&InternetCheck::Done(true));
    assert_eq!((on.text, on.dot, on.tip), ("인터넷 · 연결됨", Some(true), None));
}
