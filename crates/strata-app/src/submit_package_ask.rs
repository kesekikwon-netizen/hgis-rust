//! The three questions in front of the folder dialog (`KaSubmitFlow.cpp:84-122`).

use strata_engine::SubmitEncoding;

use super::SubmitPhase;
use crate::app_state::StrataAppState;

pub(crate) fn paint_submit_phase(app: &mut StrataAppState, ui: &mut eframe::egui::Ui) {
    let Some(phase) = app.submit_phase.clone() else {
        return;
    };
    ui.add_space(6.0);
    match phase {
        SubmitPhase::AskErrors { errors, .. } => {
            ui.label(format!(
                "검수 오류 {errors}건이 남아 있습니다.\n그래도 EPSG:5179 제출 꾸러미를 만들까요? 꾸러미 안내 파일(README)에 적힙니다."
            ));
            ui.horizontal(|ui| {
                if ui.button("그래도 만들기").clicked() {
                    app.accept_submit_errors();
                }
                if ui.button("취소").clicked() {
                    app.cancel_submit_package();
                }
            });
        }
        SubmitPhase::AskUnsaved { .. } => {
            ui.label(
                "저장하지 않은 편집이 있습니다.\n제출 꾸러미는 지금 화면의 도형으로 만들어져 조사 파일(마지막 저장본)과 다를 수 있습니다.",
            );
            ui.horizontal(|ui| {
                if ui.button("저장하고 만들기").clicked() {
                    app.answer_submit_unsaved(true);
                }
                if ui.button("저장하지 않고 만들기").clicked() {
                    app.answer_submit_unsaved(false);
                }
                if ui.button("취소").clicked() {
                    app.cancel_submit_package();
                }
            });
        }
        SubmitPhase::AskEncoding { .. } => {
            ui.label("SHP 글자 인코딩 (모르면 UTF-8)");
            ui.horizontal(|ui| {
                if ui.button("UTF-8").clicked() {
                    let _ = app.choose_submit_encoding(SubmitEncoding::Utf8);
                }
                if ui.button("EUC-KR").clicked() {
                    let _ = app.choose_submit_encoding(SubmitEncoding::EucKr);
                }
                if ui.button("취소").clicked() {
                    app.cancel_submit_package();
                }
            });
        }
    }
}
