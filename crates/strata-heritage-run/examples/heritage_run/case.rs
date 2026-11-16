//! The cases this example drives. `KaHeritageBrowser.cpp:596-940`.
//!
//! The fixture always accepts the same pair; a case that must fail logs in with a different
//! password. Passing the wrong password to *both* the fixture and the run makes the fixture
//! accept it, which is how the first version of this example tested nothing.

use strata_net::heritage_auto::HeritageStage;

/// The account the fixture accepts, in every case.
pub const FIXTURE_ACCOUNT: (&str, &str) = ("user1", "pass1");

/// What a case expects.
pub struct Case {
    pub name: &'static str,
    /// The id the run logs in with.
    pub run_id: &'static str,
    /// The password the run logs in with.
    pub run_password: &'static str,
    /// The city both the fixture and the run use.
    pub city: (&'static str, &'static str),
    /// Where the run must be when the case is over.
    pub expect: HeritageStage,
}

/// The cases, in the order they run.
#[must_use]
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "right password reaches the download page",
            run_id: FIXTURE_ACCOUNT.0,
            run_password: FIXTURE_ACCOUNT.1,
            city: ("경상북도", "경주시"),
            expect: HeritageStage::DownloadAll,
        },
        Case {
            name: "wrong password stops and says so",
            run_id: FIXTURE_ACCOUNT.0,
            run_password: "not-the-password",
            city: ("경상북도", "경주시"),
            expect: HeritageStage::Stopped(String::new()),
        },
    ]
}

/// Whether `got` satisfies `expect`, treating any stop as equal to a stop.
#[must_use]
pub fn matches(got: &HeritageStage, expect: &HeritageStage) -> bool {
    match (got, expect) {
        (HeritageStage::Stopped(_), HeritageStage::Stopped(_)) => true,
        (a, b) => a == b,
    }
}
