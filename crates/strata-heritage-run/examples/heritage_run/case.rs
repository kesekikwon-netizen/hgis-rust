//! Fixture scenarios from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1147`.

pub use crate::sample_zip::sample_zip;
use strata_net::heritage_auto::HeritageStage;

pub const FIXTURE_ACCOUNT: (&str, &str) = ("user1", "pass1");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Standard,
    Pledge,
    Delayed,
    Absent,
    Popup,
    NestedLogin,
    StorageFailure,
}

pub struct Case {
    pub name: &'static str,
    pub run_id: &'static str,
    pub run_password: &'static str,
    pub city: (&'static str, &'static str),
    pub expect: HeritageStage,
    pub scenario: Scenario,
}

fn case(name: &'static str, expect: HeritageStage, scenario: Scenario) -> Case {
    Case {
        name,
        run_id: FIXTURE_ACCOUNT.0,
        run_password: FIXTURE_ACCOUNT.1,
        city: ("경상북도", "경주시"),
        expect,
        scenario,
    }
}

pub fn cases() -> Vec<Case> {
    let mut refused = case(
        "wrong password stops without resubmission",
        HeritageStage::Stopped(String::new()),
        Scenario::Standard,
    );
    refused.run_password = "not-the-password";
    let cases = vec![
        case(
            "right password reaches the verified download",
            HeritageStage::DownloadAll,
            Scenario::Standard,
        ),
        refused,
        case(
            "six ZIP kinds finish in order",
            HeritageStage::Done,
            Scenario::Standard,
        ),
        case(
            "cancel interrupts an accepted transfer and finishes cleanup",
            HeritageStage::Cancelled,
            Scenario::Standard,
        ),
        case(
            "pledge callback preserves exact terms",
            HeritageStage::Done,
            Scenario::Pledge,
        ),
        case(
            "delayed DOM acknowledgments wait without duplicate actions",
            HeritageStage::Done,
            Scenario::Delayed,
        ),
        case(
            "six absent kinds finish without requesting a ZIP",
            HeritageStage::Done,
            Scenario::Absent,
        ),
        case(
            "popup document owns the pledge and download form",
            HeritageStage::Done,
            Scenario::Popup,
        ),
        case(
            "nested login uses its frame own goLogin",
            HeritageStage::Done,
            Scenario::NestedLogin,
        ),
        case(
            "allocation refusal preserves its I/O cause and original file",
            HeritageStage::Stopped(String::new()),
            Scenario::StorageFailure,
        ),
    ];
    // Optional substring selects one actual browser case for a reproducible red/green run.
    match std::env::args().nth(1) {
        Some(wanted) => cases
            .into_iter()
            .filter(|case| case.name.contains(&wanted))
            .collect(),
        None => cases,
    }
}

pub fn wants_zip(case: &Case) -> bool {
    case.scenario != Scenario::Absent
        && (matches!(case.expect, HeritageStage::Done | HeritageStage::Cancelled)
            || case.scenario == Scenario::StorageFailure)
}

pub fn wants_pledge(case: &Case) -> bool {
    matches!(
        case.scenario,
        Scenario::Pledge | Scenario::Delayed | Scenario::Popup
    )
}

pub fn matches(got: &HeritageStage, expect: &HeritageStage) -> bool {
    match (got, expect) {
        (HeritageStage::Stopped(_), HeritageStage::Stopped(_)) => true,
        (a, b) => a == b,
    }
}
