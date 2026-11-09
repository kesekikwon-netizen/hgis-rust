//! C++ `KaBoreholeDialog.cpp` (190줄, 커밋 `d7afcc8`) 의 화면 1:1 이식.
//!
//! 「자료 받기」 그룹의 다섯 번째 단추 「시추」(`btnBoreholeFetch`)가 여는 비모달 설정창이다.
//! 반경(2/5/10km)과 5대 GIS 도면 체크 카드를 읽어 `strata_paleo` 엔진에 넘긴다.
//!
//! C++ 는 `QDialog` + `QgsProject` 를 쓰지만 Rust 는 egui 창과 앱 상태로 같은 일을 한다.
//! 문구·선택지·기본값은 [`crate::dialogs_borehole_spec`] 에 C++ 원문 그대로 있다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_borehole_spec::{
    borehole_count_text, BOREHOLE_BTN_FETCH, BOREHOLE_BTN_LAYOUT, BOREHOLE_BTN_REBUILD,
    BOREHOLE_DIALOG_HEADER, BOREHOLE_DIALOG_SIZE, BOREHOLE_DIALOG_TITLE, BOREHOLE_ELEV_SUBCHECK,
    BOREHOLE_MAP_CARDS, BOREHOLE_NEED_SURVEY, BOREHOLE_RADIUS_CHOICES, BOREHOLE_RADIUS_LABEL,
};

pub use crate::dialogs_borehole_spec::{BoreholeDialogAction, BoreholeDialogState};

/// C++ `KaBoreholeDialog.cpp:37-64` `makeMapCard` 의 Strata 테마 카드 한 장.
fn render_map_card(
    ui: &mut egui::Ui,
    title: &str,
    desc: &str,
    checked: &mut bool,
    sub: Option<(&str, &mut bool)>,
) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(0xF5, 0xF4, 0xED))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0xDE, 0xDC, 0xD1)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut *checked,
                    egui::RichText::new(title)
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(0x14, 0x14, 0x13)),
                );
                if let Some((sub_text, sub_state)) = sub {
                    ui.checkbox(
                        &mut *sub_state,
                        egui::RichText::new(sub_text)
                            .size(12.0)
                            .color(Color32::from_rgb(0x5E, 0x5D, 0x59)),
                    );
                }
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(desc)
                        .size(12.0)
                        .color(Color32::from_rgb(0x5E, 0x5D, 0x59)),
                );
            });
        });
}

/// C++ `KaBoreholeDialog::openBoreholeMapDialog` 의 화면 부분.
///
/// `state` 는 앱이 들고 있는 창 상태다. `loaded_count` 는 지금 적재된 시추공 수,
/// `can_run` 은 조사 폴더가 준비됐는지다. 돌려주는 [`BoreholeDialogAction`] 이 있으면
/// 호출부가 실제 동작(받기/재생성/조판 열기)을 한다.
pub fn render_borehole_dialog(
    ctx: &egui::Context,
    state: &mut BoreholeDialogState,
    loaded_count: usize,
    can_run: bool,
    open: &mut bool,
) -> Option<BoreholeDialogAction> {
    let mut action = None;
    // C++ 는 NonModal 이다(`setWindowModality(Qt::NonModal)`). egui 도 모달 배경을 깔지 않는다.
    egui::Window::new("kaBoreholeMapDialog")
        .title_bar(false)
        .resizable(false)
        .order(egui::Order::Middle)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(BOREHOLE_DIALOG_SIZE.x)
        .show(ctx, |ui| {
            crate::dialogs_modal::render_strata_modal_header(ui, BOREHOLE_DIALOG_TITLE, open);
            ui.label(
                egui::RichText::new(BOREHOLE_DIALOG_HEADER)
                    .size(13.0)
                    .color(Color32::from_rgb(0x14, 0x14, 0x13)),
            );
            ui.add_space(8.0);
            render_radius_row(ui, state, loaded_count);
            ui.add_space(8.0);
            render_map_cards(ui, state);
            ui.add_space(10.0);
            action = render_action_buttons(ui, state, loaded_count, can_run);
            if !can_run {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(BOREHOLE_NEED_SURVEY)
                        .size(12.0)
                        .color(Color32::from_rgb(0x9C, 0x4A, 0x2F)),
                );
            }
        });
    action
}

/// C++ `KaBoreholeDialog.cpp:93-110` 반경 콤보 줄과 적재 시추공 수.
fn render_radius_row(ui: &mut egui::Ui, state: &mut BoreholeDialogState, loaded_count: usize) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(BOREHOLE_RADIUS_LABEL).size(13.0));
        egui::ComboBox::from_id_salt("cmbBoreholeRadius")
            .selected_text(BOREHOLE_RADIUS_CHOICES[state.radius_choice_index()].1)
            .width(280.0)
            .show_ui(ui, |ui| {
                for (meters, label) in BOREHOLE_RADIUS_CHOICES {
                    ui.selectable_value(&mut state.radius_meters, meters, label);
                }
            });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(borehole_count_text(loaded_count))
                .size(12.0)
                .color(Color32::from_rgb(0x5E, 0x5D, 0x59)),
        );
    });
}

/// C++ `KaBoreholeDialog.cpp:114-140` 다섯 도면 카드. 4번만 EL.m 서브 체크를 함께 둔다.
fn render_map_cards(ui: &mut egui::Ui, state: &mut BoreholeDialogState) {
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            let cards = BOREHOLE_MAP_CARDS;
            render_map_card(ui, cards[0].0, cards[0].1, &mut state.marker_map, None);
            ui.add_space(4.0);
            render_map_card(
                ui,
                cards[1].0,
                cards[1].1,
                &mut state.alluvial_isopach_map,
                None,
            );
            ui.add_space(4.0);
            render_map_card(
                ui,
                cards[2].0,
                cards[2].1,
                &mut state.fill_thickness_map,
                None,
            );
            ui.add_space(4.0);
            render_map_card(
                ui,
                cards[3].0,
                cards[3].1,
                &mut state.bedrock_depth_map,
                Some((BOREHOLE_ELEV_SUBCHECK, &mut state.bedrock_elev_map)),
            );
            ui.add_space(4.0);
            render_map_card(
                ui,
                cards[4].0,
                cards[4].1,
                &mut state.groundwater_map,
                None,
            );
        });
}

/// C++ `KaBoreholeDialog.cpp:169-192` 하단 세 단추. 폭 비율은 2 : 2 : 1 그대로다.
fn render_action_buttons(
    ui: &mut egui::Ui,
    state: &BoreholeDialogState,
    loaded_count: usize,
    can_run: bool,
) -> Option<BoreholeDialogAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        let total = ui.available_width();
        let gap = 6.0;
        let unit = (total - gap * 2.0) / 5.0;
        let fetch_w = unit * 2.0;
        let rebuild_w = unit * 2.0;

        if ui
            .add_enabled(
                can_run,
                egui::Button::new(
                    egui::RichText::new(BOREHOLE_BTN_FETCH)
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::from_rgb(0xB5, 0x57, 0x3A))
                .corner_radius(6.0)
                .min_size(Vec2::new(fetch_w, 34.0)),
            )
            .clicked()
        {
            action = Some(BoreholeDialogAction::FetchAndGenerate {
                radius_meters: state.radius_meters,
            });
        }
        if ui
            .add_enabled(
                can_run && loaded_count > 0,
                egui::Button::new(egui::RichText::new(BOREHOLE_BTN_REBUILD).size(12.5))
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(0x91, 0x8E, 0x86)))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(rebuild_w, 34.0)),
            )
            .clicked()
        {
            action = Some(BoreholeDialogAction::Regenerate);
        }
        if ui
            .add(
                egui::Button::new(egui::RichText::new(BOREHOLE_BTN_LAYOUT).size(12.5))
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(0x91, 0x8E, 0x86)))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(unit, 34.0)),
            )
            .clicked()
        {
            action = Some(BoreholeDialogAction::OpenLayout);
        }
    });
    action
}

/// 앱이 들고 있는 시추 창 상태. 닫아도 옵션은 남는다(C++ `saveToProject` 와 같은 뜻).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoreholeDialogController {
    pub state: BoreholeDialogState,
    pub open: bool,
    /// 마지막으로 만든 도면 수. C++ `generateBoreholeGisMaps` 의 반환값과 같다.
    pub last_created_count: usize,
}

impl BoreholeDialogController {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: BoreholeDialogState::default(),
            open: false,
            last_created_count: 0,
        }
    }
}

impl StrataAppState {
    /// C++ `KaBoreholeDialog.cpp:66-70` — 조사 폴더가 없으면 안내만 하고 창을 열지 않는다.
    /// 돌려주는 값은 창을 열었는지 여부다.
    pub fn open_borehole_dialog(&mut self) -> bool {
        if self.survey_path.is_none() {
            self.home_map.draw_guide_message = Some(BOREHOLE_NEED_SURVEY.to_owned());
            return false;
        }
        self.borehole_dialog.open = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C++ `KaBoreholeDialog.cpp:66-70` — 조사 폴더가 없으면 창을 열지 않고 안내만 한다.
    #[test]
    fn test_borehole_dialog_refuses_without_survey_path_like_cpp() {
        let mut app = StrataAppState::new_startup();
        assert!(app.survey_path.is_none());
        assert!(!app.open_borehole_dialog());
        assert!(!app.borehole_dialog.open);
        assert_eq!(
            app.home_map.draw_guide_message.as_deref(),
            Some("먼저 「새 조사」 또는 「열기」로 조사 폴더를 준비하세요.")
        );

        app.survey_path = Some(std::path::PathBuf::from(r"D:\조사자료\경주시_성동동"));
        assert!(app.open_borehole_dialog());
        assert!(app.borehole_dialog.open);
    }

    /// 창 상태 기본값이 C++ `MapOptions` 와 같고, 컨트롤러가 닫힌 채로 시작한다.
    #[test]
    fn test_borehole_dialog_controller_defaults_to_closed_cpp_options() {
        let c = BoreholeDialogController::new();
        assert!(!c.open);
        assert_eq!(c.last_created_count, 0);
        assert_eq!(c.state, BoreholeDialogState::default());
        assert_eq!(c.state.radius_meters, 5000);
        assert!(c.state.groundwater_map);
        assert!(!c.state.bedrock_elev_map);
    }
}
