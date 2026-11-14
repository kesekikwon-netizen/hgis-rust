//! End-of-frame egui click hook and save-banner observer for UI sounds (SND1/SND1b).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;

use crate::app_state::StrataAppState;
use super::SoundController;

const SAVED_GUIDE_PREFIX_KO: &str = "조사 데이터와 작업 구성을 저장했습니다: ";

impl SoundController {
    pub(super) fn observe_app_save(&mut self, app: &StrataAppState, now: Duration) {
        let is_saved_banner = !app.is_dirty
            && app
                .home_map
                .draw_guide_message
                .as_deref()
                .is_some_and(|m| m.starts_with(SAVED_GUIDE_PREFIX_KO));
        let cur_sig: Option<(PathBuf, usize)> = if is_saved_banner {
            app.survey_path
                .as_ref()
                .map(|p| (p.clone(), app.survey_tables.as_ptr() as usize))
        } else {
            None
        };
        if cur_sig.is_some() && cur_sig != self.prev_save_sig {
            self.on_survey_saved(now, app.display_options.sound_level);
        }
        self.prev_save_sig = cur_sig;
    }
}

/// Central end-of-frame click check using `egui 0.31.1`:
/// 1. `Context::output` (`egui-0.31.1/src/context.rs:981`) -> `PlatformOutput::events`
///    (`egui-0.31.1/src/data/output.rs:132-133`) -> `OutputEvent::Clicked(WidgetInfo)`
///    (`egui-0.31.1/src/data/output.rs:469-471`) for standard controls.
/// 2. `Context::interaction_snapshot` (`egui-0.31.1/src/context.rs:3743`,
///    `InteractionSnapshot::clicked` at `egui-0.31.1/src/interaction.rs:14-15`) +
///    `Context::read_response` (`egui-0.31.1/src/context.rs:1200`) for custom-painted studio
///    tabs, ribbon chips, and popup menu items (`Sense::click()` without `senses_drag()`),
///    excluding map/terrain canvases (`Sense::click_and_drag()`), text edits, sliders, and drags.
#[must_use]
pub fn frame_had_button_click(ctx: &egui::Context) -> bool {
    use egui::output::OutputEvent;
    use egui::WidgetType;

    let std_event = ctx.output(|o| {
        let mut any_clicked = false;
        let mut button_like = false;
        for ev in &o.events {
            if let OutputEvent::Clicked(info) = ev {
                any_clicked = true;
                if info.enabled
                    && matches!(
                        info.typ,
                        WidgetType::Button
                            | WidgetType::Checkbox
                            | WidgetType::RadioButton
                            | WidgetType::RadioGroup
                            | WidgetType::SelectableLabel
                            | WidgetType::ComboBox
                            | WidgetType::ColorButton
                            | WidgetType::ImageButton
                            | WidgetType::CollapsingHeader
                            | WidgetType::Link
                    )
                {
                    button_like = true;
                }
            }
        }
        any_clicked.then_some(button_like)
    });
    if let Some(matched) = std_event {
        return matched;
    }
    let Some(id) = ctx.interaction_snapshot(|i| i.clicked) else {
        return false;
    };
    ctx.read_response(id).is_some_and(|resp| {
        resp.enabled() && resp.clicked() && resp.sense.senses_click() && !resp.sense.senses_drag()
    })
}

fn controller_id() -> egui::Id {
    egui::Id::new("ka_sound_controller")
}

pub fn attach_controller_for_test(ctx: &egui::Context, controller: SoundController) {
    let arc = Arc::new(Mutex::new(controller));
    ctx.data_mut(|d| d.insert_temp(controller_id(), arc));
}

pub fn on_frame_end(ctx: &egui::Context, app: &StrataAppState) {
    if app.snap.is_some() {
        return;
    }
    let handle = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(controller_id(), || {
            let args: Vec<String> = std::env::args().collect();
            Arc::new(Mutex::new(SoundController::for_cli_args(&args)))
        })
        .clone()
    });
    let clicked = frame_had_button_click(ctx);
    if let Ok(mut ctrl) = handle.lock() {
        let now = ctrl.clock_origin.elapsed();
        ctrl.begin_frame();
        if clicked {
            ctrl.on_button_click(now, app.display_options.sound_level);
        }
        ctrl.observe_app_save(app, now);
        ctrl.end_frame();
    };
}
