use eframe::egui;
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::trench_grid::TrenchStandardLength;
use strata_types::crs::SubmitCrs;

use crate::app_state::StrataAppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModalDialog {
    AccountSettings,
    TrenchGenerator,
    ExportSubmitPackage,
}

#[derive(Debug, Clone)]
pub struct ModalDialogController {
    pub active: Option<ActiveModalDialog>,
    pub vworld_key_input: String,
    pub heritage_id_input: String,
    pub ngii_id_input: String,
    pub trench_length: TrenchStandardLength,
    pub trench_coverage_pct: f32,
    pub last_export_status: String,
}

impl Default for ModalDialogController {
    fn default() -> Self {
        Self {
            active: None,
            vworld_key_input: String::new(),
            heritage_id_input: String::new(),
            ngii_id_input: String::new(),
            trench_length: TrenchStandardLength::M20,
            trench_coverage_pct: 10.0,
            last_export_status: String::new(),
        }
    }
}

pub fn render_active_modal_dialogs(app: &mut StrataAppState, ctx: &egui::Context) {
    let Some(kind) = app.modals.active else {
        return;
    };
    let mut open = true;
    match kind {
        ActiveModalDialog::AccountSettings => {
            egui::Window::new("사이트 로그인 · API 키 설정 (DPAPI/포터블 호환)")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(&app.accounts.status_summary_ko);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("VWorld API 키:");
                        ui.text_edit_singleline(&mut app.modals.vworld_key_input);
                    });
                    ui.horizontal(|ui| {
                        ui.label("국가유산청 인트라넷 ID:");
                        ui.text_edit_singleline(&mut app.modals.heritage_id_input);
                    });
                    ui.horizontal(|ui| {
                        ui.label("국토지리정보원(NGII) ID:");
                        ui.text_edit_singleline(&mut app.modals.ngii_id_input);
                    });
                });
        }
        ActiveModalDialog::TrenchGenerator => {
            egui::Window::new("표준 시굴격자 자동 배치 (단일 규격 준수)")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("선택한 단일 표준 길이로만 트렌치를 배치합니다 (잔여 면적 임의 축소 금지).");
                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut app.modals.trench_length,
                            TrenchStandardLength::M20,
                            "20m × 2m",
                        );
                        ui.radio_value(
                            &mut app.modals.trench_length,
                            TrenchStandardLength::M15,
                            "15m × 2m",
                        );
                        ui.radio_value(
                            &mut app.modals.trench_length,
                            TrenchStandardLength::M10,
                            "10m × 2m",
                        );
                    });
                    ui.add(
                        egui::Slider::new(&mut app.modals.trench_coverage_pct, 2.0..=25.0)
                            .text("목표 시굴 면적 비율(%)"),
                    );
                });
        }
        ActiveModalDialog::ExportSubmitPackage => {
            egui::Window::new("국가유산청 제출 꾸러미 내보내기 (EPSG:5179)")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let findings = ChecklistEngine::evaluate_sheet(
                        &app.drawing.sheet,
                        SubmitCrs::Epsg5179,
                        true,
                    );
                    for f in &findings {
                        let badge = if f.passed { "[통과]" } else { "[안내]" };
                        ui.label(format!("{badge} {} — {}", f.rule_id, f.message_ko));
                    }
                    ui.separator();
                    if ui.button("EPSG:5179 제출 패키지 생성").clicked() {
                        let out_dir = std::env::temp_dir().join("ka_hgis_submit_export");
                        if let Ok(m) = export_submission_package(&out_dir, &findings) {
                            app.modals.last_export_status = format!(
                                "내보내기 완료: {} (파일 {}종, SHA-256 검증됨)",
                                out_dir.display(),
                                m.files_sha256.len()
                            );
                        }
                    }
                    if !app.modals.last_export_status.is_empty() {
                        ui.label(&app.modals.last_export_status);
                    }
                });
        }
    }
    if !open {
        app.modals.active = None;
    }
}
