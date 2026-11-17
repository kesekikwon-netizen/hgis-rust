//! Native document capability selection (target/cpp-ref/src/app/KaHeritageBrowser.cpp:639-778).
use super::{DocumentFacts, HeritageStage};

#[derive(Debug, Clone, Copy)]
pub(super) enum Scope {
    Login,
    Form,
    General,
}

impl Scope {
    pub(super) fn for_stage(stage: &HeritageStage) -> Self {
        match stage {
            HeritageStage::LoggingIn => Self::Login,
            HeritageStage::Kind
            | HeritageStage::KindReady
            | HeritageStage::KindBaseline
            | HeritageStage::Region
            | HeritageStage::RegionReady
            | HeritageStage::Search
            | HeritageStage::Results
            | HeritageStage::DownloadAll => Self::Form,
            HeritageStage::Idle
            | HeritageStage::LoginProbe
            | HeritageStage::DismissTutorial
            | HeritageStage::TutorialProbe
            | HeritageStage::AwaitLogin
            | HeritageStage::DownloadPage
            | HeritageStage::DownloadPageReady
            | HeritageStage::Terms
            | HeritageStage::TermsReady
            | HeritageStage::Waiting
            | HeritageStage::AwaitingImport
            | HeritageStage::RetryWait
            | HeritageStage::Done
            | HeritageStage::Cancelled
            | HeritageStage::Stopped(_) => Self::General,
        }
    }
    pub(super) fn rank(self, facts: DocumentFacts) -> Option<u8> {
        match self {
            Self::Login => facts.login.then_some(0),
            Self::Form => facts.form.then_some(0),
            Self::General => Some(if facts.form {
                0
            } else if facts.agreement {
                1
            } else if facts.menu {
                2
            } else if facts.authenticated {
                3
            } else if facts.login {
                4
            } else {
                5
            }),
        }
    }
    pub(super) fn empty(self) -> &'static str {
        match self {
            Self::Login => "no-form",
            Self::Form | Self::General => "not-found",
        }
    }
}
