//! 「도면 목록」 — C++ `KaDrawingSheetSet` (KaDrawingSheetSet.cpp:51-162) and `storeCurrentSheet`,
//! `restoreStoredSheet`, `removeStoredSheet` (KaDrawingStudioSheets.cpp:71-170): the working sheet is kept
//! whole under a name with the layers checked then, opened again in place of the working sheet, or taken out
//! of the list. Every step asks first with the C++ words.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::tab_drawing::StoredSheet;

/// What the sheet list asks: a name to keep the sheet under, or whether to replace, open or take out the
/// kept sheet of that name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SheetAsk {
    Name(String),
    Replace(String),
    Open(String),
    Remove(String),
}

impl StrataAppState {
    /// 「지금 도면 보관…」: asks for a name, offering the title of the drawing, else 「도면 N」 (`:81-85`).
    pub(crate) fn ask_store_sheet(&mut self) {
        self.end_sheet_map_adjust();
        let title = self.drawing.title_ko.trim();
        let offered = if title.is_empty() { format!("도면 {}", self.drawing.stored_sheets.len() + 1) } else { title.to_owned() };
        self.drawing.sheet_ask = Some(SheetAsk::Name(offered));
    }

    /// The name given: a blank one keeps nothing; one already in the list asks before it is replaced (`:86-91`).
    pub(crate) fn store_sheet_named(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.drawing.sheet_ask = None;
        } else if self.drawing.stored_sheets.iter().any(|kept| kept.title == name) {
            self.drawing.sheet_ask = Some(SheetAsk::Replace(name.to_owned()));
        } else {
            self.store_sheet(name);
        }
    }

    /// Keeps the working sheet whole under `name`, in place of a kept sheet of that name (`:93-104`).
    pub(crate) fn store_sheet(&mut self, name: &str) {
        let d = &self.drawing;
        let kept = StoredSheet {
            title: name.to_owned(),
            sheet: d.sheet.clone(),
            drawing_title: d.title_ko.clone(),
            sheet_center: d.sheet_center,
            item_offsets_mm: d.item_offsets_mm.clone(),
            legend_size_mm: d.legend_size_mm,
            removed_items: d.removed_items.clone(),
            coord_callouts: d.coord_callouts.clone(),
        };
        self.store_layer_set(name);
        self.drawing.stored_sheets.retain(|other| other.title != name);
        self.drawing.stored_sheets.push(kept);
        self.drawing.sheet_ask = None;
        self.show_status_message(format!("도면 「{name}」을 보관했습니다. 「도면 목록」에서 다시 엽니다."));
    }

    /// Opens the kept sheet `name` in place of the working one, with the layers checked then; the kept copy
    /// stays in the list (`:107-156`).
    pub(crate) fn open_stored_sheet(&mut self, name: &str) {
        self.drawing.sheet_ask = None;
        let Some(kept) = self.drawing.stored_sheets.iter().find(|kept| kept.title == name).cloned() else { return };
        self.end_sheet_map_adjust();
        let d = &mut self.drawing;
        d.sheet = kept.sheet;
        d.title_ko = kept.drawing_title;
        d.sheet_center = kept.sheet_center;
        d.item_offsets_mm = kept.item_offsets_mm;
        d.legend_size_mm = kept.legend_size_mm;
        d.removed_items = kept.removed_items;
        d.coord_callouts = kept.coord_callouts;
        (d.coord_callout_mode, d.selected_item, d.scale_entry) = (false, None, None);
        let layers_applied = self.apply_layer_set(name);
        self.show_status_message(if layers_applied { format!("도면 「{name}」을 열고 그때의 레이어 켜짐을 적용했습니다.") } else { format!("도면 「{name}」을 열었습니다.") });
    }

    /// Takes the kept sheet `name` and its layer set out of the list; the working sheet stays (`:158-170`).
    pub(crate) fn remove_stored_sheet(&mut self, name: &str) {
        self.drawing.sheet_ask = None;
        if !self.drawing.stored_sheets.iter().any(|kept| kept.title==name) { return; }
        self.drawing.stored_sheets.retain(|kept| kept.title != name);
        self.remove_layer_set(name);
        self.show_status_message(format!("보관한 도면 「{name}」을 지웠습니다."));
    }

    /// The names of the kept sheets in the order of the list: by name (`:40-42`).
    pub(crate) fn sheet_set_rows(&self) -> Vec<String> {
        let mut rows: Vec<String> = self.drawing.stored_sheets.iter().map(|kept| kept.title.clone()).collect();
        rows.sort();
        rows
    }

    /// The menu of 「도면 목록」 at `at` (`fillMenu`, :131-162); a click elsewhere closes it.
    pub(crate) fn render_sheet_set_menu(&mut self, ctx: &egui::Context, at: Pos2) {
        let rows = self.sheet_set_rows();
        let mut chosen: Option<SheetAsk> = None;
        let area = egui::Area::new(egui::Id::new("ka_sheet_set_menu")).order(egui::Order::Foreground).fixed_pos(at);
        let shown = area.show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(280.0);
                let keep = ui.button("지금 도면 보관…").on_hover_text("지금 용지를 이름 붙여 따로 둡니다. 레이어 켜짐도 같은 이름의 세트로 둡니다.");
                if keep.clicked() {
                    chosen = Some(SheetAsk::Name(String::new()));
                }
                ui.separator();
                if rows.is_empty() {
                    ui.add_enabled(false, egui::Button::new("보관한 도면이 없습니다"));
                } else {
                    for title in &rows {
                        if ui.button(format!("열기: {title}")).clicked() {
                            chosen = Some(SheetAsk::Open(title.clone()));
                        }
                    }
                    ui.menu_button("보관한 도면 지우기", |ui| {
                        for title in &rows {
                            if ui.button(title).clicked() {
                                chosen = Some(SheetAsk::Remove(title.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                }
                ui.separator();
                ui.add_enabled(false, egui::Button::new("제출 PDF는 지금 열린 도면으로 만듭니다"));
            });
        });
        match chosen {
            Some(SheetAsk::Name(_)) => {
                self.drawing.sheet_set_menu_open = false;
                self.ask_store_sheet();
            }
            Some(ask) => {
                self.drawing.sheet_set_menu_open = false;
                self.drawing.sheet_ask = Some(ask);
            }
            None if shown.response.clicked_elsewhere() => self.drawing.sheet_set_menu_open = false,
            None => {}
        }
    }

    /// The question of the sheet list, with the C++ words (`QInputDialog`, `QMessageBox::question`, default 아니요).
    pub(crate) fn render_sheet_ask(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.drawing.sheet_ask.clone() else { return };
        let (title, words) = match &ask {
            SheetAsk::Name(_) => ("도면 보관", "도면 이름 (예: 조사구역도 1:1000, 1호 주거지 실측도)".to_owned()),
            SheetAsk::Replace(name) => ("도면 보관", format!("「{name}」 도면이 이미 있습니다. 지금 도면으로 바꿀까요?")),
            SheetAsk::Open(name) => (
                "도면 열기",
                format!("지금 용지를 보관한 도면 「{name}」로 바꿉니다. 지금 도면을 보관하지 않았다면 사라집니다. 레이어 켜짐도 그 도면의 세트로 바뀝니다. 바꿀까요?"),
            ),
            SheetAsk::Remove(name) => ("도면 목록", format!("보관한 도면 「{name}」과 같은 이름의 레이어 세트를 지울까요? 지금 열린 도면과 레이어는 그대로입니다.")),
        };
        let (mut yes, mut no) = (false, false);
        let mut typed = if let SheetAsk::Name(text) = &ask { text.clone() } else { String::new() };
        egui::Window::new(title).id(egui::Id::new("ka_sheet_set_ask")).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.label(words);
            if matches!(ask, SheetAsk::Name(_)) {
                let field = ui.add(egui::TextEdit::singleline(&mut typed).desired_width(f32::INFINITY));
                if !field.has_focus() && !field.lost_focus() {
                    field.request_focus();
                }
                yes |= field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let (agree, refuse) = if matches!(ask, SheetAsk::Name(_)) { ("확인", "취소") } else { ("예", "아니요") };
                let primary = egui::Button::new(egui::RichText::new(agree).color(Color32::WHITE)).fill(Color32::from_rgb(181, 87, 58));
                yes |= ui.add(primary).clicked();
                no |= ui.add(egui::Button::new(refuse).stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))).clicked();
            });
            no |= ui.input(|i| i.key_pressed(egui::Key::Escape));
        });
        match ask {
            SheetAsk::Name(_) if yes => self.store_sheet_named(&typed),
            SheetAsk::Name(_) if !no => self.drawing.sheet_ask = Some(SheetAsk::Name(typed)),
            SheetAsk::Replace(name) if yes => self.store_sheet(&name),
            SheetAsk::Open(name) if yes => self.open_stored_sheet(&name),
            SheetAsk::Remove(name) if yes => self.remove_stored_sheet(&name),
            _ if no => self.drawing.sheet_ask = None,
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_sheets_tests.rs"]
mod tests;
