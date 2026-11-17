//! Observed browser outcomes (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1101-1147,1293-1400`).

use crate::case::{Case, Scenario};
use std::path::Path;
use strata_heritage_fixture::Fixture;
use strata_net::heritage_auto::{HeritageEvent, HeritageStage};

const KINDS: [&str; 6] = [
    "지정유산",
    "현상변경허용기준",
    "매장유산유존지역",
    "문화유적분포지도",
    "지표조사구역",
    "발굴조사구역",
];

pub struct TransportProof {
    pub accepted_transfer: bool,
    pub acknowledged: Option<usize>,
    pub recreated_session: bool,
}

pub fn held(
    case: &Case,
    stage: &HeritageStage,
    events: &[HeritageEvent],
    fixture: &Fixture,
    root: &Path,
    proof: &TransportProof,
) -> bool {
    let agreements: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            HeritageEvent::PledgeAgreed { terms } => Some(terms.as_str()),
            _ => None,
        })
        .collect();
    let pledge = if crate::case::wants_pledge(case) {
        agreements
            == ["국가유산 공간정보 원본자료 사용 서약서 \"사용\" \\자료\n\n서약서에 동의확인"]
    } else {
        agreements.is_empty()
    };
    if !pledge
        || !crate::case::matches(stage, &case.expect)
        || fixture.hits("/j_spring_security_check")
            != if case.scenario == Scenario::RetryOnce {
                2
            } else {
                1
            }
        || fixture.hits("/fixture/unfiltered-download") != 0
    {
        return false;
    }
    let zips: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            HeritageEvent::DatasetReady { kind, files } => Some((*kind, files)),
            _ => None,
        })
        .collect();
    let absent: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            HeritageEvent::KindAbsent { kind } => Some(*kind),
            _ => None,
        })
        .collect();
    let done = events
        .iter()
        .filter(|event| matches!(event, HeritageEvent::Done))
        .count();
    let cancelled = events
        .iter()
        .filter(|event| matches!(event, HeritageEvent::Cancelled))
        .count();
    let stopped: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            HeritageEvent::Stopped(why) => Some(why),
            _ => None,
        })
        .collect();
    match case.expect {
        HeritageStage::Done => {
            let names: Vec<_> = zips.iter().map(|(kind, _)| *kind).collect();
            let empty = case.scenario == Scenario::Absent;
            let retried = usize::from(case.scenario == Scenario::RetryOnce);
            done == 1
                && (retried == 0 || proof.recreated_session)
                && proof.acknowledged == Some(if empty { 0 } else { 6 })
                && cancelled == 0
                && stopped.is_empty()
                && matches!(events.last(), Some(HeritageEvent::Done))
                && if empty {
                    absent == KINDS && zips.is_empty()
                } else {
                    names == KINDS
                        && absent.is_empty()
                        && zips.iter().all(|(_, files)| {
                            !files.is_empty() && files.iter().all(|p| p.is_file())
                        })
                }
                && fixture.hits("/fixture/panel-open") == 1 + retried
                && fixture.hits("/fixture/kind-request") == 6 + retried
                && fixture.hits("/fixture/search-request") == 6 + retried
                && fixture.hits("/fixture/confirm-accepted") == if empty { 0 } else { 6 + retried }
                && fixture.hits("/user/data/heritageDownload/downloadFilesAll.do")
                    == if empty { 0 } else { 6 + retried }
        }
        HeritageStage::Cancelled => {
            proof.accepted_transfer
                && cancelled == 1
                && done == 0
                && stopped.is_empty()
                && zips.is_empty()
                && absent.is_empty()
                && fixture.hits("/fixture/confirm-accepted") == 1
        }
        HeritageStage::Stopped(_) if case.scenario == Scenario::StorageFailure => {
            let cause = std::fs::create_dir_all(root.join("probe"))
                .expect_err("root remains a file")
                .to_string();
            stopped.len() == 1
                && stopped[0].starts_with("저장할 폴더가 없어 내려받기를 취소했습니다. (")
                && stopped[0].contains(&cause)
                && std::fs::read(root).ok().as_deref() == Some(b"keep original")
                && done == 0
                && cancelled == 0
                && zips.is_empty()
                && absent.is_empty()
        }
        HeritageStage::Stopped(_) => {
            stopped.len() == 1
                && done == 0
                && cancelled == 0
                && fixture.hits("/fixture/panel-open") == 0
                && zips.is_empty()
        }
        HeritageStage::DownloadAll => fixture.hits("/fixture/search-request") == 1 && done == 0,
        _ => false,
    }
}

pub fn offer_was_accepted(root: &Path, city: (&str, &str)) -> bool {
    let path = root
        .join("주변유적")
        .join(format!("{} {}", city.0, city.1))
        .join("원본");
    std::fs::read_dir(path)
        .is_ok_and(|mut entries| entries.any(|entry| entry.is_ok_and(|e| e.path().is_dir())))
}
