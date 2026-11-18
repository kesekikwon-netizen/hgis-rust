//! The question after a polygon is finished on top of another shape of the same layer.
//! `MainWindow::offerEraseWithDrawnShape` (`target/cpp-ref/src/app/MainWindowErase.cpp:120-147`) and the
//! call in `onGeometryCaptured` (`MainWindowEditing.cpp:842-849`). A different layer does not ask.
//! Closing the menu or 「그대로 두기」 keeps the new shape. 「겹친 곳 지우기」 runs the same erase as the button.

use eframe::egui::{self, RichText};

use crate::app_state::StrataAppState;

/// The menu waiting at the cursor (`drawnShapeEraseMenu`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawnErase {
    pub(crate) key: String,
    pub(crate) fid: i64,
    pub(crate) notice: Option<String>,
    /// The first frame ignores the click that finished the sketch.
    armed: bool,
}

impl StrataAppState {
    /// `offerEraseWithDrawnShape`: true when the menu is up. The name form waits until the answer.
    pub(crate) fn queue_drawn_erase(
        &mut self,
        key: &str,
        fid: i64,
        notice: Option<String>,
    ) -> bool {
        let Ok((cutter, targets)) = self.erase_plan(Some(&(key.to_owned(), fid))) else {
            return false;
        };
        if cutter.0 != key || targets.iter().any(|(layer, _)| layer != key) {
            return false;
        }
        self.home_map.drawn_erase = Some(DrawnErase {
            key: key.to_owned(),
            fid,
            notice,
            armed: false,
        });
        true
    }

    /// 「겹친 곳 지우기」 or 「그대로 두기」 / Escape / a click outside (`MainWindowErase.cpp:142-147`).
    pub(crate) fn answer_drawn_erase(&mut self, erase: bool) {
        let Some(offer) = self.home_map.drawn_erase.take() else {
            return;
        };
        if erase {
            self.home_map.selected_shape = Some((offer.key, offer.fid));
            self.erase_overlap_with_shape();
            return;
        }
        self.after_capture(&offer.key, offer.fid, offer.notice);
    }

    pub(crate) fn render_drawn_erase(&mut self, ctx: &egui::Context) {
        let Some(offer) = self.home_map.drawn_erase.clone() else {
            return;
        };
        if !offer.armed {
            if let Some(waiting) = self.home_map.drawn_erase.as_mut() {
                waiting.armed = true;
            }
            self.paint_drawn_erase_menu(ctx);
            return;
        }
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let (erase, keep, outside) = self.paint_drawn_erase_menu(ctx);
        if erase {
            self.answer_drawn_erase(true);
        } else if keep || escape || outside {
            self.answer_drawn_erase(false);
        }
    }

    /// The two rows. Returns (erase clicked, keep clicked, click outside the menu).
    fn paint_drawn_erase_menu(&mut self, ctx: &egui::Context) -> (bool, bool, bool) {
        let pos = ctx
            .input(|i| i.pointer.latest_pos())
            .unwrap_or_else(|| ctx.screen_rect().center());
        let mut erase = false;
        let mut keep = false;
        let mut menu_rect = egui::Rect::NOTHING;
        let area = egui::Area::new(egui::Id::new("drawnShapeEraseMenu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(&ctx.style())
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.set_min_width(160.0);
                        let row = |ui: &mut egui::Ui, label: &str| {
                            ui.add(
                                egui::Button::new(
                                    RichText::new(label).color(egui::Color32::from_rgb(30, 30, 28)),
                                )
                                .fill(egui::Color32::WHITE),
                            )
                            .clicked()
                        };
                        erase = row(ui, "겹친 곳 지우기");
                        keep = row(ui, "그대로 두기");
                        menu_rect = ui.min_rect();
                    });
            });
        if menu_rect == egui::Rect::NOTHING {
            menu_rect = area.response.rect;
        }
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !menu_rect.contains(p))
        });
        (erase, keep, outside)
    }
}

#[cfg(test)]
#[path = "drawn_erase_tests.rs"]
mod tests;
