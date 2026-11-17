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
        let (mut action, mut call) = self.request();
        // C++ rechecks authenticated DOM before dismissing a tutorial, even when the
        // previous login probe was positive: target/cpp-ref/src/app/KaHeritageBrowser.cpp:925-935.
        if wanted == Action::DismissTutorial && action == Action::ProbeLogin {
            let _ = self.machine.script_answered(call.token, "past-login");
            (action, call) = self.request();
        }
        assert_eq!(action, wanted, "unexpected action before replying {reply}");
        self.machine.script_answered(call.token, reply)
    }

    pub fn reach(&mut self, wanted: HeritageStage) {
        for _ in 0..40 {
            if *self.machine.stage() == wanted {
                return;
            }
            assert!(
                !self.machine.is_over(),
                "setup stopped at {:?}",
                self.machine.stage()
            );
            let (action, call) = self.request();
            let reply = match action {
                Action::ProbeLogin => "past-login",
                Action::SubmitLogin => "submitted",
                Action::DismissTutorial => "none",
                Action::ProbePanel | Action::KindReady => "ready",
                Action::OpenPanel | Action::AgreeTerms => "already-open",
                Action::FormHint => "codedeta",
                Action::SelectKind | Action::SelectRegion => "selected",
                Action::ReadRegion => "ok:47|47110",
                Action::Search => "searching",
                Action::ResultInfo if *self.machine.stage() == HeritageStage::Kind => BASELINE,
                Action::ResultInfo => FILTERED,
                Action::Download => "all",
            };
            let _ = self.machine.script_answered(call.token, reply);
        }
        panic!(
            "setup never reached {wanted:?}, got {:?}",
            self.machine.stage()
        );
    }

    fn action(&self, call: &ScriptCall) -> Action {
        // Identify returned boundary requests, not expected state transitions. The tests below
        // assert production state and events; these builders only decode the outbound protocol.
        // C++ source: target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147.
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
            (Action::SelectKind, scripts::select_dataset_script("S")),
            (Action::KindReady, scripts::dataset_ready_script("S")),
            (
                Action::SelectRegion,
                scripts::select_region_script("경상북도", "포항시"),
            ),
            (Action::Search, scripts::search_script()),
            (Action::ResultInfo, scripts::result_page_info_script()),
            (Action::Download, scripts::download_all_script()),
        ];
        if let Some((action, _)) = known.into_iter().find(|(_, script)| *script == call.script) {
            return action;
        }
        // The baseline exports no verifyRegionScript builder. At this boundary the C++ asks
        // for a readback, whose actual replies are "empty" and "ok:47|47110".
        // Source: target/cpp-ref/src/core/HeritageIntranetFlow.cpp:622-639.
        assert_eq!(
            *self.machine.stage(),
            HeritageStage::Region,
            "unrecognized page request"
        );
        Action::ReadRegion
    }
}
