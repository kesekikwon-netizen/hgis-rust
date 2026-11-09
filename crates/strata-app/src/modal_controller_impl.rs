//! `ModalDialogController` 의 본문 동작(저장·불러오기·검색 등).
//!
//! `modal_types.rs` 가 300줄을 넘어 나눴다.

use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{self};

use crate::modal_controller_state::ModalDialogController;
use crate::modal_topo_rows::{topo_default_crs_idx, topo_record_key};
use crate::modal_topo_rows::{TopoTableRow, TOPO_CRS_UNKNOWN, TOPO_CRS_CANDIDATES, topo_crs_label_ko};

impl ModalDialogController {
    /// C++ `KaLineColorDialog.cpp:48-51` — 면·래스터·음영 레이어는 선 색을 바꾸지 않는다.
    #[must_use]
    pub fn is_line_map_layer(layer_name: &str) -> bool {
        const NON_LINE: [&str; 7] = [
            "지형 음영",
            "지표모델(DSM) 30m · Coperni...",
            "토양도",
            "지질도",
            "고지형",
            "위성",
            "항공",
        ];
        !NON_LINE.iter().any(|n| layer_name.contains(n))
    }

    /// C++ `KaSurveyAreaDialog.cpp:24-43` `readableTextOn` — WCAG 상대 휘도로
    /// 배경 위에서 읽히는 글자색을 고른다(명암비 4.5 이상이면 흰색).
    #[must_use]
    pub fn readable_text_on(r: u8, g: u8, b: u8) -> egui::Color32 {
        let lin = |c: u8| -> f64 {
            let s = f64::from(c) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        let lum = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
        let contrast_white = 1.05 / (lum + 0.05);
        if contrast_white >= 4.5 {
            egui::Color32::from_rgb(255, 255, 255)
        } else {
            egui::Color32::from_rgb(30, 35, 45)
        }
    }

    /// C++ `KaSubmitDialog.cpp:109-121` — 검수 실패 항목의 바로가기 버튼 이름.
    #[must_use]
    pub fn submit_action_label(action: &str) -> Option<&'static str> {
        match action {
            "open_layout" => Some("도면 열기"),
            "draw_survey_area" => Some("조사구역 그리기"),
            "draw_feature" => Some("유구 그리기"),
            "add_control_point" => Some("기준점 찍기"),
            "edit_attributes" => Some("속성 고치기"),
            "vertex_edit" => Some("꼭짓점 편집"),
            "select_tool" => Some("도형 선택"),
            "set_work_crs" => Some("좌표계 고르기"),
            _ => None,
        }
    }

    /// C++ KaClosePrompt.cpp:12-22 `withObjectParticle` — attach 을/를 to a
    /// Korean name using the Hangul batchim (final consonant) rule.
    pub fn with_object_particle(name: &str) -> String {
        match name.chars().last() {
            Some(c) if ('\u{AC00}'..='\u{D7A3}').contains(&c) => {
                let batchim = !((c as u32) - 0xAC00).is_multiple_of(28);
                format!("{name}{}", if batchim { "을" } else { "를" })
            }
            Some(c) if c.is_ascii_digit() => {
                // 0·1·3·6·7·8 read with a final consonant
                let batchim = "013678".contains(c);
                format!("{name}{}", if batchim { "을" } else { "를" })
            }
            _ => format!("{name}을(를)"),
        }
    }

    /// C++ KaClosePrompt.cpp:30-46 — heading and body of the unsaved-work prompt.
    pub fn format_close_prompt(survey_name: &str, unsaved_features: usize) -> (String, String) {
        let heading = format!("{} 저장할까요?", Self::with_object_particle(survey_name));
        let body = if unsaved_features > 0 {
            format!("저장하지 않은 작업이 {unsaved_features}건 있습니다.")
        } else {
            "저장하지 않은 작업이 있습니다.".to_owned()
        };
        (heading, body)
    }

    /// C++ `KaPaperDialog.cpp:149-153` `paperScaleNote` — A4 가로만 현장 용지를 쓴다는 안내
    #[must_use]
    pub fn paper_scale_note(paper: SheetPaperSize, orientation: SheetOrientation) -> &'static str {
        // The field sheet is larger than A4 on purpose. Say so where it is chosen:
        // printed on one A4 sheet it shrinks and the written 1:N no longer holds on paper.
        if paper == SheetPaperSize::A4 && orientation == SheetOrientation::Landscape {
            "A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다. A4 종이 한 장에 찍으면 약 90%로 줄어 종이 위 축척이 달라집니다. 축척대로 찍으려면 인쇄 창에서 A3 또는 여러 장 나눠 찍기를 고르세요."
        } else {
            "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."
        }
    }

    /// C++ `KaPaperDialog.cpp:39` `plain(mm)` — `297`, not `297.0`
    #[must_use]
    pub fn plain_mm(mm: f64) -> String {
        format!("{mm:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }

    /// C++ `KaModalDialogController::showExclusiveModal` — 이미 활성화된 모달이 존재할 때 중복 모달 팝업을 차단하는 단일 가드
    pub fn show_exclusive_modal(&mut self, dialog: ActiveModalDialog) -> bool {
        if self.active.is_some() {
            return false;
        }
        self.active = Some(dialog);
        true
    }

    /// C++ `KaTopographicImportDialog.cpp:433-448` `scanFolder` — 폴더를 실제로 훑는다.
    /// 안내문은 `:437`, 결과는 `:449-497` `populate` 를 따른다.
    pub fn rescan_topo_folder(&mut self, folder: &std::path::Path) {
        self.topo_folder = folder.to_string_lossy().into_owned();
        self.topo_status_text =
            "받은 자료를 확인하고 있습니다… 지도 작업을 계속할 수 있습니다.".to_owned();
        self.topo_progress_pct = 0.0;
        let result = strata_net::topographic_catalog::scan_folder(folder);
        self.topo_scanned = true;
        // C++ `:450-453` — 취소·오류면 기존 목록을 건드리지 않고 문구만 바꾼다.
        if let Some(error) = result.error {
            self.topo_scan_error = Some(error.clone());
            self.topo_status_text = error;
            self.topo_progress_pct = 0.0;
            return;
        }
        self.topo_scan_error = None;
        self.topo_files_read = result.files_read;
        self.topo_cache_hits = result.cache_hits;
        self.topo_warnings = result.warnings.clone();
        // C++ `:454-459` — 이전에 사용자가 만진 체크·좌표계 선택을 같은 항목에 옮긴다.
        let previous: Vec<(String, bool, usize)> = self
            .topo_records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                let checked = *self.topo_row_checks.get(index)?;
                let crs_idx = *self.topo_row_crs_idx.get(index)?;
                Some((topo_record_key(record), checked, crs_idx))
            })
            .collect();
        self.topo_row_checks = result
            .records
            .iter()
            .map(|record| {
                let key = topo_record_key(record);
                previous
                    .iter()
                    .find(|(known, _, _)| *known == key)
                    .map_or_else(|| record.initially_selected(), |(_, checked, _)| *checked)
            })
            .collect();
        self.topo_row_crs_idx = result
            .records
            .iter()
            .map(|record| {
                let key = topo_record_key(record);
                previous
                    .iter()
                    .find(|(known, _, _)| *known == key)
                    .map_or_else(|| topo_default_crs_idx(record.crs_epsg()), |(_, _, index)| *index)
            })
            .collect();
        self.topo_row_already_loaded = vec![false; result.records.len()];
        self.topo_selected_row = 0;
        // C++ `:494-495` 상태줄 — 레코드를 옮기기 전에 만든다.
        let summary = result.summary_ko();
        self.topo_records = result.records;
        self.topo_progress_pct = 1.0;
        self.topo_status_text = summary;
    }

    /// 표에 그릴 줄들 — `KaTopographicImportDialog.cpp:462-491` `populate` 의 표 부분.
    #[must_use]
    pub fn topo_table_rows(&self) -> Vec<TopoTableRow> {
        self.topo_records
            .iter()
            .map(|record| {
                // C++ `:475-482` — 0 번은 「좌표계 미확인」, 1 번은 .prj 파일 정보,
                // 이어서 후보 열두 개.
                let mut choices = vec![TOPO_CRS_UNKNOWN.to_owned()];
                if let Some(epsg) = record.crs_epsg() {
                    choices.push(format!("파일 정보: {}", topo_crs_label_ko(epsg)));
                }
                for epsg in TOPO_CRS_CANDIDATES {
                    choices.push(topo_crs_label_ko(epsg).to_owned());
                }
                TopoTableRow {
                    label: record.label_ko().to_owned(),
                    file_name: std::path::Path::new(&record.source)
                        .file_name()
                        .map_or_else(
                            || record.source.clone(),
                            |name| name.to_string_lossy().into_owned(),
                        ),
                    review_reason: record.review_reason.clone(),
                    extent_label: record.extent_label_ko(),
                    choices,
                }
            })
            .collect()
    }

    /// C++ `:433-434` 색인 취소 — 목록을 비우고 문구만 남긴다.
    pub fn cancel_topo_scan(&mut self) {
        self.topo_status_text = "색인을 취소했습니다. 기존 지도는 유지됩니다.".to_owned();
        self.topo_progress_pct = 0.0;
    }

    /// 체크된 항목 가운데 올릴 수 없는 것이 있으면 C++ 문구(`:529-537`).
    /// C++ 은 범위를 먼저 보고, 이어서 **콤보에서 고른** 좌표계가 유효한지 본다.
    /// `.prj` 를 못 읽어 0(「좌표계 미확인」)으로 남은 항목은 사용자가 골라야 한다.
    #[must_use]
    pub fn topo_blocking_message(&self) -> Option<&'static str> {
        if self.topo_records.iter().enumerate().any(|(index, record)| {
            self.topo_row_checks.get(index).copied().unwrap_or(false) && !record.has_extent()
        }) {
            return Some(strata_net::topographic_catalog::IMPORT_NEEDS_EXTENT);
        }
        if self.topo_records.iter().enumerate().any(|(index, _)| {
            self.topo_row_checks.get(index).copied().unwrap_or(false)
                && self.topo_row_crs_idx.get(index).copied().unwrap_or(0) == 0
        }) {
            return Some(strata_net::topographic_catalog::IMPORT_NEEDS_CRS);
        }
        None
    }

    /// 체크된 항목의 행 번호들.
    #[must_use]
    pub fn topo_checked_rows(&self) -> Vec<usize> {
        self.topo_row_checks
            .iter()
            .enumerate()
            .filter(|(_, checked)| **checked)
            .map(|(index, _)| index)
            .collect()
    }

    /// C++ `KaModalDialogController` ESC 취소 및 미저장 입력값 존재 시 닫기 확인(`"저장되지 않은 변경사항이 있습니다. 닫으시겠습니까?"`)
    pub fn try_reject_active_modal(
        &mut self,
        confirm_on_dirty: bool,
        is_form_dirty: bool,
        user_confirmed_discard: bool,
    ) -> Result<(), &'static str> {
        if confirm_on_dirty && is_form_dirty && !user_confirmed_discard {
            return Err("저장되지 않은 변경사항이 있습니다. 닫으시겠습니까?");
        }
        self.active = None;
        Ok(())
    }
}
