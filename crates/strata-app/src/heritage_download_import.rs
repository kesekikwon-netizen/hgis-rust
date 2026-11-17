//! Serial disk work after actual site events (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`).

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

use strata_engine::Scope;
use strata_refmap::heritage::HeritageDataset;
use strata_refmap::heritage_load::{load_dataset, DatasetLoad, LoadFailure};
use strata_topo::archive::{prepare, Limits};

use super::Context;
use crate::download_jobs::{DownloadJobHandle, StopCode};

pub(super) enum Task {
    Zip(&'static HeritageDataset, PathBuf),
    Pledge(String),
}

pub(crate) enum Reply {
    Kind(
        &'static HeritageDataset,
        Result<DatasetLoad, (StopCode, String)>,
    ),
    Receipt(Result<PathBuf, String>),
}

pub(super) struct Worker {
    pub(super) send: Option<Sender<Task>>,
    pub(super) replies: Receiver<Reply>,
}

impl Worker {
    pub(super) fn start(context: Context, scope: Scope, job: DownloadJobHandle) -> Self {
        let (send, tasks) = channel();
        let (answers, replies) = channel();
        std::thread::spawn(move || {
            // Separate run folders keep a cancelled import from touching a later run's files
            // (`target/cpp-ref/src/core/HeritageImport.cpp:159-212`).
            let library = context.root.join("주변유적").join("SHP");
            let work = library
                .join("UTF8")
                .join(format!("strata-{}", uuid::Uuid::new_v4()));
            while let Ok(task) = tasks.recv() {
                if job.is_cancelled() {
                    break;
                }
                let reply = match task {
                    Task::Pledge(terms) => Reply::Receipt(
                        crate::heritage_receipt::write_pledge_receipt(
                            &context.survey,
                            context.account,
                            &terms,
                            chrono::Local::now().naive_local(),
                        )
                        .map_err(|error| error.to_string()),
                    ),
                    Task::Zip(dataset, path) => {
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
                            std::slice::from_ref(&path),
                            &unpack,
                            Some((&scope, context.work_epsg)),
                            "",
                            &work.join(dataset.key),
                        );
                        // Import acceptance precedes recent.json, as in C++
                        // (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1214`).
                        if job.is_cancelled() {
                            break;
                        }
                        let result = loaded
                            .map_err(|failure| (StopCode::Data1, failure.reason))
                            .and_then(|load| {
                                let region = strata_refmap::heritage_recent::region_folder(
                                    &context.root,
                                    &context.sido,
                                    &context.city,
                                )
                                .ok_or_else(|| {
                                    (
                                        StopCode::Dir1,
                                        "받은 파일을 둘 곳을 찾지 못했습니다.".to_owned(),
                                    )
                                })?;
                                crate::heritage_receipt::record_recent(
                                    &region,
                                    dataset.name,
                                    &[path],
                                    chrono::Local::now().naive_local(),
                                )
                                .map_err(|error| (StopCode::Dir2, error.to_string()))?;
                                Ok(load)
                            });
                        Reply::Kind(dataset, result)
                    }
                };
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

    pub(super) fn queue(&self, task: Task) -> bool {
        self.send
            .as_ref()
            .is_some_and(|send| send.send(task).is_ok())
    }
}
