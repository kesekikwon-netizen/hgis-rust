//! Page replies follow `target/cpp-ref/src/core/HeritageIntranetFlow.cpp:150-478,622-639`.

use strata_net::heritage_auto::{
    scripts, HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage, PageHost, PageId,
    PageRef, ScriptCall,
};

pub(super) const BASELINE: &str = r#"{"rows":10,"lastPage":831,"total":8302,"searchReady":false}"#;
pub(super) const FILTERED: &str = r#"{"rows":10,"lastPage":9,"total":90,"searchReady":true}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    ProbeLogin,
    SubmitLogin,
    DismissTutorial,
    ProbePanel,
    OpenPanel,
    AgreeTerms,
    FormHint,
    SelectKind,
    KindReady,
    SelectRegion,
    ReadRegion,
    Search,
    ResultInfo,
    Download,
}

#[derive(Debug)]
struct Page {
    url: String,
}

impl PageHost for Page {
    fn pages(&self) -> Vec<PageRef> {
        vec![PageRef {
            id: PageId(1),
            url: self.url.clone(),
            opener: None,
        }]
    }

    fn run_script(&mut self, _page: PageId, _script: &str) -> Option<u64> {
        None
    }

    fn navigate(&mut self, url: &str) {
        self.url = url.to_owned();
    }
}

#[derive(Debug)]
pub(super) struct Session {
    pub machine: HeritageAutomation,
    pub now_ms: u64,
    pub requested: Vec<Action>,
    page: Page,
}

impl Session {
    pub fn new() -> Self {
        let mut machine = HeritageAutomation::new(HeritageConfig {
            account_id: "fixture-user".to_owned(),
            account_password: "fixture-password".to_owned(),
            sido: "경상북도".to_owned(),
            city: "포항시".to_owned(),
            fixture_base: Some("http://127.0.0.1:1".to_owned()),
        });
        let mut page = Page {
            url: "about:blank".to_owned(),
        };
        assert!(machine.next_step(&mut page, 0).is_none());
        Self {
            machine,
            now_ms: 0,
            requested: Vec::new(),
            page,
        }
    }

    pub fn request(&mut self) -> (Action, ScriptCall) {
        // Exact logical ticks, from KaHeritageBrowser.cpp:103; no elapsed-time tolerance.
        self.now_ms += 700;
        let call = self
            .machine
            .next_step(&mut self.page, self.now_ms)
            .unwrap_or_else(|| panic!("no page request at {:?}", self.machine.stage()));
        let action = self.action(&call);
        self.requested.push(action);
        (action, call)
    }

    pub fn answer(&mut self, wanted: Action, reply: &str) -> Option<HeritageEvent> {
        let (action, call) = self.request();
        assert_eq!(action, wanted, "unexpected action before replying {reply}");
        self.machine.script_answered(call.token, reply)
    }

    pub fn reach(&mut self, wanted: HeritageStage) {
        // Model the explicit fresh-browser boundary, not a hidden script answer.
        // C++ retry recovery: target/cpp-ref/src/app/KaHeritageBrowser.cpp:523-560,832-856.
        if *self.machine.stage() == HeritageStage::RetryWait {
            assert!(
                self.machine.session_restart_required(),
                "retry must request a fresh session"
            );
            assert!(
                self.machine.retry_due(self.now_ms),
                "reopen cannot precede its backoff"
            );
            assert!(
                self.machine.session_reopened(self.now_ms),
                "fresh-session boundary must be accepted"
            );
            assert_eq!(*self.machine.stage(), HeritageStage::Idle);
            self.page = Page {
                url: "about:blank".to_owned(),
            };
            assert!(
                self.machine
                    .next_step(&mut self.page, self.now_ms)
                    .is_none(),
                "fresh login starts with navigation, not a script"
            );
            assert_eq!(*self.machine.stage(), HeritageStage::LoginProbe);
        }
        // Check every setup action in order. Missing readiness observations cannot be hidden
        // by a later state's plausible reply: target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147.
        let transcript = [
            (HeritageStage::LoginProbe, Action::ProbeLogin, "past-login"),
            (
                HeritageStage::TutorialProbe,
                Action::ProbeLogin,
                "past-login",
            ),
            (
                HeritageStage::DismissTutorial,
                Action::DismissTutorial,
                "none",
            ),
            (
                HeritageStage::DownloadPage,
                Action::OpenPanel,
                "already-open",
            ),
            (
                HeritageStage::DownloadPageReady,
                Action::ProbePanel,
                "ready",
            ),
            (HeritageStage::Terms, Action::AgreeTerms, "already-open"),
            (HeritageStage::Kind, Action::SelectKind, "selected"),
            (HeritageStage::KindReady, Action::KindReady, "ready"),
            (HeritageStage::KindBaseline, Action::ResultInfo, BASELINE),
            (HeritageStage::Region, Action::SelectRegion, "selected"),
            (
                HeritageStage::RegionReady,
                Action::ReadRegion,
                "ok:47|47110",
            ),
            (HeritageStage::Search, Action::Search, "searching"),
            (HeritageStage::Results, Action::ResultInfo, FILTERED),
            (HeritageStage::DownloadAll, Action::Download, "all"),
        ];
        if *self.machine.stage() == wanted {
            return;
        }
        let start = transcript
            .iter()
            .position(|(stage, _, _)| *stage == *self.machine.stage())
            .unwrap_or_else(|| panic!("no setup transcript for {:?}", self.machine.stage()));
        for (stage, action, reply) in &transcript[start..] {
            if *self.machine.stage() == wanted {
                return;
            }
            assert!(
                !self.machine.is_over(),
                "setup stopped at {:?}",
                self.machine.stage()
            );
            assert_eq!(
                *self.machine.stage(),
                *stage,
                "setup skipped a required phase"
            );
            let _ = self.answer(*action, reply);
        }
        assert_eq!(
            *self.machine.stage(),
            wanted,
            "setup ended before its target"
        );
    }

    fn action(&self, call: &ScriptCall) -> Action {
        // Identify returned boundary requests, not expected state transitions. The tests below
        // assert production state and events; these builders only decode the outbound protocol.
        // C++ source: target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147.
        // Literal C++ tab codes: HeritageStyle.cpp:27-32,90-92 and HeritageIntranetFlow.cpp:247-248.
        let code = match self.machine.current_kind() {
            "지정유산" => "S",
            "현상변경허용기준" => "P",
            "매장유산유존지역" => "B",
            "문화유적분포지도" => "U",
            "지표조사구역" => "R",
            "발굴조사구역" => "E",
            other => panic!("unrecognized kind {other}"),
        };
        let known = [
            (Action::ProbeLogin, scripts::login_probe_script()),
            (
                Action::SubmitLogin,
                scripts::login_script("fixture-user", "fixture-password"),
            ),
            (Action::DismissTutorial, scripts::dismiss_tutorial_script()),
            (Action::ProbePanel, scripts::download_page_probe_script()),
            (Action::OpenPanel, scripts::open_download_page_script()),
            (Action::AgreeTerms, scripts::agree_terms_script()),
            (Action::FormHint, scripts::form_hint_script()),
            (Action::SelectKind, scripts::select_dataset_script(code)),
            (Action::KindReady, scripts::dataset_ready_script(code)),
            (
                Action::SelectRegion,
                scripts::select_region_script("경상북도", "포항시"),
            ),
            (Action::ReadRegion, scripts::verify_region_script()),
            (Action::Search, scripts::search_script()),
            (Action::ResultInfo, scripts::result_page_info_script()),
            (Action::Download, scripts::download_all_script()),
        ];
        if let Some((action, _)) = known.into_iter().find(|(_, script)| *script == call.script) {
            return action;
        }
        panic!("unrecognized page request at {:?}", self.machine.stage());
    }
}
