//! Stage facts from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147`.

use std::path::PathBuf;

/// Where the run stands. The C++ keeps this in `m_stage` (`KaHeritageBrowser.cpp:96-121`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HeritageStage {
    /// Nothing started.
    #[default]
    Idle,
    /// Ask the page whether we are past the login form (`:626-632`).
    LoginProbe,
    /// Dismiss the tutorial popup that hides the top menu (`KaHeritageBrowser.cpp:820-821`).
    DismissTutorial,
    /// Recheck authentication before each tutorial action (KaHeritageBrowser.cpp:925-944).
    TutorialProbe,
    /// Fill the form and call the site's `goLogin()` (`:898-933`).
    LoggingIn,
    /// Wait for authenticated DOM after one submit (KaHeritageBrowser.cpp:925-932).
    AwaitLogin,
    /// Open the download panel through `showAgreePopup()` (`:1030-1100`).
    DownloadPage,
    /// Wait for the requested panel (KaHeritageBrowser.cpp:949-973).
    DownloadPageReady,
    /// Tick the pledge box and press 확인 (`:1100-1180`).
    Terms,
    /// Wait for the form after agreement (KaHeritageBrowser.cpp:975-980).
    TermsReady,
    /// Pick the kind's tab (`:1180-1260`).
    Kind,
    /// Wait for the requested replacement form (KaHeritageBrowser.cpp:1003-1027).
    KindReady,
    /// Read a positive pre-search count (KaHeritageBrowser.cpp:1261-1268).
    KindBaseline,
    /// Pick 시·도 and 시·군 (`:1260-1360`).
    Region,
    /// Read the actual selected values (KaHeritageBrowser.cpp:1042-1056).
    RegionReady,
    /// Press 검색 and read how many rows came back (`:1360-1450`).
    Search,
    /// Observe refreshed, filtered results (KaHeritageBrowser.cpp:1101-1147).
    Results,
    /// Press 전체다운로드 for the current kind (`:1450-1524`).
    DownloadAll,
    /// Wait for actual transfers and six quiet polls (`KaHeritageBrowser.cpp:848-869,1150-1165`).
    Waiting,
    /// The completed archive group awaits the app's real import result (`:1206-1217`).
    AwaitingImport,
    /// Recoverable failure waits before recreating the authenticated session (`:504-562`).
    RetryWait,
    /// Every kind is through.
    Done,
    /// The caller cancelled this run (`KaHeritageBrowser.cpp:479-489`).
    Cancelled,
    /// The run stopped, with the words to show.
    Stopped(String),
}

/// What the app must know about a step.
#[derive(Debug, Clone, PartialEq)]
pub enum HeritageEvent {
    /// Words for the status line; the run goes on.
    Note(String),
    /// Exact terms returned after the site's agreement button ran.
    PledgeAgreed { terms: String },
    /// A completed filtered search proved this kind has no records.
    KindAbsent { kind: &'static str },
    /// All six kinds ended successfully.
    Done,
    /// The caller cancelled the run.
    Cancelled,
    /// All archives for this kind are ready for import (`KaHeritageBrowser.h:71`).
    DatasetReady {
        /// The kind's Korean name, one of [`super::scripts::KINDS`].
        kind: &'static str,
        /// Accepted offers in their original order, after every transfer completes.
        files: Vec<PathBuf>,
    },
    /// The run stopped; the words to show the user.
    Stopped(String),
}

/// The app's actual full-group import result (`KaHeritageBrowser.cpp:491-501,1206-1217`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetOutcome {
    Accepted,
    RetryableFailure { reason: String },
    FatalFailure { reason: String },
}

/// What a run is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeritageConfig {
    /// The saved account id; never written to a log or a receipt.
    pub account_id: String,
    /// The saved password; never written to a log or a receipt.
    pub account_password: String,
    /// 시·도 of the survey's city (`HeritageRegionResolver`).
    pub sido: String,
    /// 시·군 of the survey's city.
    pub city: String,
    /// A fixture base address, when a test or `--snap` drives a local server instead of the
    /// site. `None` means the real intranet.
    pub fixture_base: Option<String>,
}

impl HeritageConfig {
    /// The login landing page for this run: the fixture's when one was given.
    #[must_use]
    pub fn login_url(&self) -> String {
        match &self.fixture_base {
            Some(base) => format!("{}/user/checkNet.do", base.trim_end_matches('/')),
            None => super::scripts::LOGIN_URL.to_owned(),
        }
    }

    /// The download panel address for this run.
    #[must_use]
    pub fn download_url(&self) -> String {
        match &self.fixture_base {
            Some(base) => format!(
                "{}/user/data/heritageDownload.do",
                base.trim_end_matches('/')
            ),
            None => format!(
                "https://intranet.gis-heritage.go.kr{}",
                super::scripts::DOWNLOAD_PAGE_PATH
            ),
        }
    }
}
