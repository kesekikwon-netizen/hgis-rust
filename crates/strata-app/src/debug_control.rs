use eframe::egui;
use std::time::Duration;
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::SubToolsMode;
use crate::right_inspector::InspectorCardTab;

impl StrataAppState {
    /// 외부 디버거·검증 스크립트가 `TEMP/ka_hgis_debug_cmd.txt`를 통해 실행 중인 Rust 앱의 15개 화면·모달·도구를 즉시 제어한다.
    pub(crate) fn poll_debug_remote_commands(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(Duration::from_millis(200));
        let cmd_path = std::env::temp_dir().join("ka_hgis_debug_cmd.txt");
        let Ok(raw) = std::fs::read_to_string(&cmd_path) else {
            return;
        };
        let _ = std::fs::remove_file(&cmd_path);
        for line in raw.lines() {
            let cmd = line.trim().trim_start_matches('\u{feff}');
            match cmd {
                "tab:home" => {
                    self.active_tab = MainTab::Home;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = false;
                    self.active_tool = MapToolKind::Pan;
                }
                "tab:map" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = false;
                    self.active_tool = MapToolKind::Pan;
                }
                "tab:drawing" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Drawing;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = false;
                    self.active_tool = MapToolKind::Pan;
                }
                "tab:paleo" => {
                    self.active_tab = MainTab::Paleo;
                    self.modals.active = None;
                }
                "tab:terrain3d" => {
                    self.active_tab = MainTab::Terrain3d;
                    self.modals.active = None;
                }
                "modal:paper_setup" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::PaperSetup);
                }
                "modal:dem_download" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::DemDownload);
                }
                "modal:heritage_region" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect);
                }
                "modal:ngii_topo" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
                }
                "modal:cadastral" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::CadastralDownload);
                }
                "modal:trench" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.ribbon.sub_toolbar_visible = true;
                    self.ribbon.sub_mode = SubToolsMode::Draw;
                    self.active_tool = MapToolKind::TrenchPlace;
                    self.modals.active = Some(ActiveModalDialog::TrenchGenerator);
                }
                "tool:measure" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.ribbon.sub_toolbar_visible = true;
                    self.ribbon.sub_mode = SubToolsMode::Draw;
                    self.active_tool = MapToolKind::Measure;
                    self.modals.active = None;
                }
                "modal:pdf_preparing" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Drawing;
                    self.modals.active = Some(ActiveModalDialog::PdfPreparing);
                }
                "modal:drawing_print" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Drawing;
                    self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                }
                "modal:account" => {
                    self.accounts.show_passwords = false;
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
                "modal:account_show" => {
                    self.accounts.show_passwords = true;
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
                "modal:submit" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                }
                "modal:control_point" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::ControlPoint);
                }
                "modal:survey_area" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::SurveyArea);
                }
                "modal:line_color" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::LineColor);
                }
                "modal:dem_class" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::DemClass);
                }
                "modal:reference_status" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = Some(ActiveModalDialog::ReferenceStatus);
                }
                "modal:none" => {
                    self.modals.active = None;
                }
                "subtool:draw" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = true;
                    self.ribbon.sub_mode = SubToolsMode::Draw;
                }
                "subtool:buffer" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = true;
                    self.ribbon.sub_mode = SubToolsMode::Buffer;
                }
                "subtool:none" => {
                    self.ribbon.sub_toolbar_visible = false;
                    self.ribbon.sub_mode = SubToolsMode::None;
                }
                "popup:region" => {
                    self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
                }
                "inspector:style" => {
                    self.open_sample_survey_project(0);
                    self.active_tab = MainTab::Map;
                    self.modals.active = None;
                    self.ribbon.sub_toolbar_visible = false;
                    self.active_tool = MapToolKind::Pan;
                    self.right_inspector.active_card = InspectorCardTab::LayerStyleCard;
                    self.right_inspector.selected_layer_key = Some("조사구역".to_owned());
                    self.right_inspector.show_draw_tooltip = true;
                }
                _ => {}
            }
        }
        let state_path = std::env::temp_dir().join("ka_hgis_debug_state.json");
        let state_json = format!(
            "{{\"active_tab\":\"{}\",\"project_title\":\"{}\",\"modal\":\"{:?}\",\"layers\":{}}}",
            self.active_tab.label_ko(),
            self.project_title,
            self.modals.active,
            self.left_layers.entries.len()
        );
        let _ = std::fs::write(state_path, state_json);
    }
}
