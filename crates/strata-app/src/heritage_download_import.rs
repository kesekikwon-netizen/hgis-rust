//! Serial disk work after actual site events (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`).

#[cfg(any(windows, test))]
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use strata_engine::Scope;
#[cfg(any(windows, test))]
use strata_refmap::heritage::HeritageDataset;
#[cfg(any(windows, test))]
use strata_refmap::heritage_load::{load_dataset, DatasetLoad, LoadFailure};
#[cfg(any(windows, test))]
use strata_topo::archive::{prepare, Limits};

use super::Context;
use crate::download_jobs::DownloadJobHandle;
#[cfg(any(windows, test))]
use crate::download_jobs::{DownloadJobId, StopCode};

#[cfg(any(windows, test))]
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Group {
    pub(super) job: DownloadJobId,
    pub(super) dataset: &'static HeritageDataset,
    pub(super) files: Vec<PathBuf>,
}

#[cfg(any(windows, test))]
pub(crate) struct ImportFailure {
    pub(super) code: StopCode,
    pub(super) reason: String,
    pub(super) download_again: bool,
}

pub(super) enum Task {
    #[cfg(any(windows, test))]
    Group(Group),
    #[cfg(any(windows, test))]
    Publish(Group, DatasetLoad),
    #[cfg(any(windows, test))]
    Pledge(String),
}

pub(crate) enum Reply {
    #[cfg(any(windows, test))]
    Kind(Group, Result<DatasetLoad, ImportFailure>),
    #[cfg(any(windows, test))]
    Published(Group, Result<DatasetLoad, ImportFailure>),
    #[cfg(any(windows, test))]
    Receipt(Result<PathBuf, String>),
}

pub(super) struct Worker {
    pub(super) send: Option<Sender<Task>>,
    pub(super) replies: Receiver<Reply>,
}

impl Worker {
    pub(super) fn start(
        #[cfg(any(windows, test))] context: Context,
        #[cfg(not(any(windows, test)))] _context: Context,
        #[cfg(any(windows, test))] scope: Scope,
        #[cfg(not(any(windows, test)))] _scope: Scope,
        #[cfg(any(windows, test))] job: DownloadJobHandle,
        #[cfg(not(any(windows, test)))] _job: DownloadJobHandle,
    ) -> Self {
        let (send, tasks) = channel();
        let (answers, replies) = channel();
        std::thread::spawn(move || {
            // No task can be sent on this OS. Wait until the download drops the sender so the
            // reply channel stays open (`drive_heritage_download`).
            #[cfg(not(any(windows, test)))]
            {
                let _hold_answers = answers;
                drop(tasks.recv());
                drop(_hold_answers);
            }
            #[cfg(any(windows, test))]
            while let Ok(task) = tasks.recv() {
                if job.is_cancelled() {
                    break;
                }
                let reply = match task {
                    #[cfg(any(windows, test))]
                    Task::Pledge(terms) => Reply::Receipt(
                        crate::heritage_receipt::write_pledge_receipt(
                            &context.survey,
                            context.account,
                            &terms,
                            chrono::Local::now().naive_local(),
                        )
                        .map_err(|error| error.to_string()),
                    ),
                    #[cfg(any(windows, test))]
                    Task::Group(group) => {
                        // Separate run folders keep a cancelled import from touching a later run's files
                        // (`target/cpp-ref/src/core/HeritageImport.cpp:159-212`).
                        let library = context.root.join("주변유적").join("SHP");
                        let work = library.join("UTF8").join(format!("strata-{}", uuid::Uuid::new_v4()));
                        let dataset = group.dataset;
                        let received = &group.files;
                        let cancelled = || job.is_cancelled();
                        let unpack = |archive: &std::path::Path| {
                            let prepared = prepare(
                                archive,
                                &library,
                                &Limits {
                                    korean_names: true,
                                    ..Limits::default()
                                },
                                &cancelled,
                            );
                            if prepared.error.is_empty() && !prepared.canceled {
                                Ok(prepared.files)
                            } else {
                                Err(LoadFailure {
                                    reason: prepared.error,
                                    download_again: prepared.invalid_archive,
                                })
                            }
                        };
                        let loaded = load_dataset(
                            dataset,
                            received,
                            &unpack,
                            Some((&scope, context.work_epsg)),
                            "",
                            &work
                                .join(dataset.key)
                                .join(uuid::Uuid::new_v4().to_string()),
                        );
                        if job.is_cancelled() {
                            break;
                        }
                        let result = loaded.map_err(|failure| ImportFailure {
                            code: StopCode::Data1,
                            reason: failure.reason,
                            download_again: failure.download_again,
                        });
                        Reply::Kind(group, result)
                    }
                    #[cfg(any(windows, test))]
                    Task::Publish(group, load) => {
                        // UI acceptance precedes cache publication; the next browser action waits
                        // for this reply (target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1217).
                        let result = (|| {
                            let region = strata_refmap::heritage_recent::region_folder(
                                &context.root,
                                &context.sido,
                                &context.city,
                            )
                            .ok_or_else(|| ImportFailure {
                                code: StopCode::Dir1,
                                reason: "받은 파일을 둘 곳을 찾지 못했습니다.".to_owned(),
                                download_again: false,
                            })?;
                            crate::heritage_receipt::record_recent(
                                &region,
                                group.dataset.name,
                                &group.files,
                                chrono::Local::now().naive_local(),
                            )
                            .map_err(|error| ImportFailure {
                                code: StopCode::Dir2,
                                reason: error.to_string(),
                                download_again: false,
                            })?;
                            Ok(load)
                        })();
                        Reply::Published(group, result)
                    }
                };
                #[cfg(any(windows, test))]
                if job.is_cancelled() || answers.send(reply).is_err() {
                    break;
                }
            }
        });
        Self {
            send: Some(send),
            replies,
        }
    }

    #[cfg(any(windows, test))]
    pub(super) fn queue(&self, task: Task) -> bool {
        self.send
            .as_ref()
            .is_some_and(|send| send.send(task).is_ok())
    }
}
