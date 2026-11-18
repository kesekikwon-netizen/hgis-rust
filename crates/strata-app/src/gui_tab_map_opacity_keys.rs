//! ClickFocus keyboard adapter, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:40-47,60-69.
//! Public Qt6.11 QSlider/QAbstractSlider docs define horizontal step1/page10 and Home/End bounds.
//! https://doc.qt.io/qt-6.11/qslider.html and https://doc.qt.io/qt-6.11/qabstractslider.html
use super::{placement, SliderControl};
use eframe::egui::{self, Event, Id, Key, Rect, Response, Sense};

pub(super) fn interact(ui: &egui::Ui, rect: Rect, id: Id, control: SliderControl) -> Response {
    let requested = matches!(placement::focus_request(ui.ctx()), Some(placement::FocusRequest::Slider(wanted)) if wanted == control);
    // Qt ClickFocus admits a click, not an additional Tab stop (KaLayerOpacityRail.cpp:47,69).
    let mut sense = Sense::CLICK | Sense::DRAG;
    if requested || ui.memory(|m| m.has_focus(id)) {
        sense |= Sense::FOCUSABLE;
    }
    let response = ui.interact(rect, id, sense);
    if requested {
        response.request_focus();
    }
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
    }
    response
}

pub(super) fn keyboard_value(ui: &egui::Ui, response: &Response, control: SliderControl, current: f32) -> Option<f32> {
    if !response.has_focus() {
        return None;
    }
    let (low, high, mut value) = match control {
        SliderControl::Opacity => (0, 100, (current * 100.0).round() as i32),
        SliderControl::Brightness => (-255, 255, ((current - 0.5) * 510.0).round() as i32),
    };
    // O(events), preserving repeats and event order; no pointer-derived fake Enter/Space click (:40-69).
    let accepted = ui.input_mut(|input| {
        let mut accepted = false;
        input.events.retain(|event| {
            let Event::Key { key, pressed: true, .. } = event else { return true };
            let next = match key {
                Key::ArrowLeft => value - 1,
                Key::ArrowRight => value + 1,
                Key::PageUp => value + 10,
                Key::PageDown => value - 10,
                Key::Home => low,
                Key::End => high,
                _ => return true,
            };
            value = next.clamp(low, high);
            accepted = true;
            false
        });
        accepted
    });
    accepted.then(|| match control {
        SliderControl::Opacity => value as f32 / 100.0,
        SliderControl::Brightness => (value + 255) as f32 / 510.0,
    })
}
