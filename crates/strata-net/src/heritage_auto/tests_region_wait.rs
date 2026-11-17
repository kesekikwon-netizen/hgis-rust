//! Shared Region observations (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1036-1079`).

use super::{
    HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage, PageHost, PageId, PageRef,
    LOGIN_POLL_MS,
};

struct Host;
impl PageHost for Host {
    fn pages(&self) -> Vec<PageRef> {
        vec![PageRef {
            id: PageId(1),
            url: "http://127.0.0.1/".into(),
            opener: None,
        }]
    }
    fn run_script(&mut self, _: PageId, _: &str) -> Option<u64> {
        None
    }
    fn navigate(&mut self, _: &str) {}
}

struct Run {
    machine: HeritageAutomation,
    clock: u64,
}
impl Run {
    fn new() -> Self {
        let mut machine = HeritageAutomation::new(HeritageConfig {
            account_id: "fixture-user".into(),
            account_password: "fixture-password".into(),
            sido: "경상북도".into(),
            city: "경주시".into(),
            fixture_base: None,
        });
        machine.force_stage(HeritageStage::Region);
        Self { machine, clock: 0 }
    }
    fn reply(&mut self, value: &str) -> Option<HeritageEvent> {
        self.clock += LOGIN_POLL_MS;
        let token = if self.clock == LOGIN_POLL_MS {
            self.machine.current_token()
        } else {
            self.machine
                .next_step(&mut Host, self.clock)
                .expect("region poll")
                .token
        };
        self.machine.script_answered(token, value)
    }
}

#[test]
fn missing_region_controls_stop_on_observation_21_with_cpp_diagnostics() {
    for (reply, expected) in [
        ("not-found", "다운로드 폼에서 시·도 칸을 찾지 못했습니다."),
        (
            "sigungu-pending",
            "다운로드 폼에서 시·군·구 칸을 찾지 못했습니다.",
        ),
    ] {
        let mut run = Run::new();
        for _ in 0..20 {
            assert!(run.reply(reply).is_none());
            assert_eq!(run.machine.stage(), &HeritageStage::Region);
            assert!(!run.machine.accepts_files());
        }
        assert_eq!(
            run.reply(reply),
            Some(HeritageEvent::Stopped(expected.into()))
        );
        assert!(!run.machine.accepts_files());
    }
}

#[test]
fn mixed_missing_region_replies_share_one_budget() {
    let mut run = Run::new();
    for index in 0..20 {
        assert!(run
            .reply(if index % 2 == 0 {
                "not-found"
            } else {
                "sigungu-pending"
            })
            .is_none());
    }
    assert_eq!(
        run.reply("sigungu-pending"),
        Some(HeritageEvent::Stopped(
            "다운로드 폼에서 시·군·구 칸을 찾지 못했습니다.".into()
        ))
    );
}

#[test]
fn region_readback_and_reselection_do_not_restart_the_budget() {
    let mut run = Run::new();
    for _ in 0..10 {
        assert!(run.reply("selected").is_none());
        assert_eq!(run.machine.stage(), &HeritageStage::RegionReady);
        assert!(run.reply("empty").is_none());
        assert_eq!(run.machine.stage(), &HeritageStage::Region);
    }
    assert_eq!(
        run.reply("not-found"),
        Some(HeritageEvent::Stopped(
            "다운로드 폼에서 시·도 칸을 찾지 못했습니다.".into()
        ))
    );
    assert!(!run.machine.accepts_files());
}

#[test]
fn a_positive_sigungu_count_uses_the_shared_12_observation_bound() {
    let mut run = Run::new();
    for _ in 0..5 {
        assert!(run.reply("not-found").is_none());
    }
    for _ in 0..7 {
        assert!(run.reply("sigungu:22").is_none());
    }
    assert!(matches!(
        run.reply("sigungu:22"),
        Some(HeritageEvent::Stopped(_))
    ));
    assert!(!run.machine.accepts_files());
}

#[test]
fn zero_sigungu_options_keep_waiting_for_the_navigation_bound() {
    let mut run = Run::new();
    for _ in 0..21 {
        assert!(run.reply("sigungu:0").is_none());
    }
    assert_eq!(run.machine.stage(), &HeritageStage::Region);
    assert!(!run.machine.accepts_files());
    assert!(run
        .machine
        .timers(75 * LOGIN_POLL_MS + 1)
        .iter()
        .any(|event| matches!(event, HeritageEvent::Stopped(_))));
}
