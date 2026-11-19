//! Writes the package folder. `ExportService.cpp:73-241`. DXF is not a submit file.

use std::path::{Path, PathBuf};

use strata_check_export::submit_builder::{export_submission_package, SubmitPackageInputs};
use strata_check_export::{survey_user_sheet, SheetCheck};
use strata_engine::{SubmitEncoding, SubmitSource, SUBMIT_KEYS};

use super::package_folder_name;
use crate::app_state::StrataAppState;

const NO_SURVEY: &str = "먼저 조사 파일을 열어 주세요.";
const MADE_WITH_PDF: &str = "제출 꾸러미를 만들었습니다. 인트라넷에는 SHP를 하나씩 올리세요.";
const MADE_WITHOUT_PDF: &str =
    "제출 꾸러미를 만들었습니다. 도면 용지가 없어 조사도면.pdf는 빠졌습니다. 인트라넷에는 SHP를 하나씩 올리세요.";

fn native(path: &Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(windows) {
        s.replace('/', "\\")
    } else {
        s.into_owned()
    }
}

fn titles_for(app: &StrataAppState, gpkg: &Path) -> Vec<(&'static str, Vec<SubmitSource>)> {
    SUBMIT_KEYS
        .iter()
        .filter(|key| {
            app.survey_tables.iter().any(|t| t.table == **key)
                && app.left_layers.entries.iter().any(|e| e.layer_key == **key)
        })
        .map(|key| {
            let title = app
                .left_layers
                .entries
                .iter()
                .find(|e| e.layer_key == *key)
                .map(|e| e.label_ko.clone())
                .unwrap_or_else(|| (*key).to_owned());
            (
                *key,
                vec![SubmitSource {
                    gpkg: gpkg.to_path_buf(),
                    table: (*key).to_owned(),
                    title,
                }],
            )
        })
        .collect()
}

struct HeldCopy {
    dir: PathBuf,
    gpkg: PathBuf,
}

impl Drop for HeldCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl StrataAppState {
    pub(super) fn write_submit_package(
        &mut self,
        parent: &Path,
        encoding: SubmitEncoding,
        summary: &str,
        errors_remain: bool,
        save_first: bool,
    ) -> Result<PathBuf, String> {
        let Some(survey) = self.survey_path.clone() else {
            let err = NO_SURVEY.to_owned();
            self.show_status_message(err.clone());
            return Err(err);
        };
        let unsaved = self.is_dirty && !save_first;
        let held = if unsaved {
            Some(self.copy_for_unsaved_export(&survey)?)
        } else {
            None
        };
        let gpkg = held
            .as_ref()
            .map(|h| h.gpkg.clone())
            .unwrap_or_else(|| survey.clone());
        let composed = match survey_user_sheet(&survey) {
            Ok(SheetCheck::UserSheetWithMap) => true,
            Ok(_) => false,
            Err(reason) => {
                let err =
                    format!("도면 용지를 확인하지 못해 제출 꾸러미를 만들지 않았습니다. {reason}");
                self.show_status_message(err.clone());
                return Err(err);
            }
        };
        let sources = titles_for(self, &gpkg);
        let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let survey_name = survey
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folder = package_folder_name(&survey_name, &stamp, |name| parent.join(name).exists());
        let out = parent.join(folder);
        let app_line = format!("Strata {} (unknown)", env!("CARGO_PKG_VERSION"));
        let inputs = SubmitPackageInputs {
            sources: &sources,
            encoding,
            survey_name: &survey_name,
            survey_path: Some(&survey),
            unsaved_edits_included: unsaved,
            checklist_summary: summary,
            checklist_errors_remain: errors_remain,
            reference_lines: &[],
            app_line: &app_line,
            block_on_checklist_errors: false,
            composed_user_sheet: composed,
        };
        let built = export_submission_package(&out, &inputs, &mut |_, _, _| true);
        drop(held);
        match built {
            Ok(path) => {
                let msg = if path.join("조사도면.pdf").is_file() {
                    MADE_WITH_PDF
                } else {
                    MADE_WITHOUT_PDF
                };
                self.show_status_message(format!("{msg}\n제출 꾸러미: {}", native(&path)));
                Ok(path)
            }
            Err(err) => {
                self.show_status_message(err.clone());
                Err(err)
            }
        }
    }

    /// 「저장하지 않고 만들기」 reads the edit buffer (`KaSubmitFlow.cpp:111-128`) from a copy.
    /// The open survey file is not written.
    fn copy_for_unsaved_export(&self, survey: &Path) -> Result<HeldCopy, String> {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("ka-hgis-submit-{}-{millis}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("제출 임시 폴더를 만들 수 없습니다: {e}"))?;
        let copy = dir.join("survey.gpkg");
        if let Err(e) = strata_storage::survey_gpkg_save::copy_survey_snapshot(survey, &copy) {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
        let gpkg = match strata_storage::survey_gpkg_save::save_survey(
            &copy,
            &self.survey_tables,
            &self.home_map.real_survey.distance_rings,
        ) {
            Ok(path) => path,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(match e {
                    strata_storage::survey_gpkg_save::SaveFailure::Message(m) => m,
                    strata_storage::survey_gpkg_save::SaveFailure::Layer { table, detail } => {
                        format!("{table}: {detail}")
                    }
                });
            }
        };
        Ok(HeldCopy { dir, gpkg })
    }
}
