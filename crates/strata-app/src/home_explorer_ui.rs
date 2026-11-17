//! Explorer home requested by owner; navigation words from target/cpp-ref/src/app/KaFileBrowserPanel.cpp:87-123.
use super::explorer::{Location, Sort};
use crate::app_state::StrataAppState;
use eframe::egui::{self, Button, Vec2};

fn command(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(Button::new(text).min_size(Vec2::new(88.0, 32.0)).truncate())
}
impl StrataAppState {
    pub(crate) fn render_home_explorer(&mut self, ui: &mut egui::Ui) {
        if let Some(file) = self.settings_ini.as_ref().and_then(|p| p.parent()).map(|p| p.join("strata-explorer.json")) {
            if self.snap.is_none() {
                self.explorer.configure(file);
            }
        }
        self.explorer.poll();
        if self.explorer.busy() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        }
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 10)).show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            ui.spacing_mut().interact_size.y = 32.0;
            ui.visuals_mut().selection.bg_fill = egui::Color32::from_rgb(226, 219, 203);
            ui.visuals_mut().selection.stroke.color = ui.visuals().text_color();
            ui.horizontal_wrapped(|ui| {
                ui.heading("Strata");
                ui.strong("파일함");
                ui.separator();
                if command(ui, "새 조사").clicked() {
                    self.request_new_survey();
                }
                if command(ui, "조사 열기").clicked() {
                    self.open_survey_file_dialog();
                }
                if command(ui, "폴더 직접 찾기").clicked() {
                    self.explorer.pick_folders();
                }
                if let Some(recent) = self.continue_survey().cloned() {
                    if command(ui, "이어서 열기").on_hover_text(&recent.path).clicked() {
                        self.request_open_recent(recent.path);
                    }
                }
            });
            ui.separator();
            let wide = ui.available_width() >= 860.0;
            ui.horizontal(|ui| {
                for (text, delta) in [("←", -1), ("→", 1)] {
                    if ui.add_enabled(self.explorer.can_travel(delta), Button::new(text).min_size(Vec2::splat(32.0))).clicked() {
                        self.explorer.travel(delta);
                    }
                }
                if ui.add_enabled(matches!(self.explorer.location, Location::Folder(_)), Button::new("상위").min_size(Vec2::new(48.0, 32.0))).clicked() {
                    self.explorer.up();
                }
                if ui.add(Button::new("새로고침").min_size(Vec2::new(80.0, 32.0))).clicked() {
                    self.explorer.refresh();
                }
                if !wide {
                    ui.menu_button("폴더", |ui| {
                        ui.set_width(220.0);
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| self.render_explorer_places(ui));
                    });
                }
                ui.menu_button("정렬", |ui| {
                    for (sort, label) in [(Sort::Name, "이름"), (Sort::Size, "크기"), (Sort::Modified, "수정한 날짜"), (Sort::Kind, "종류")] {
                        if ui.selectable_label(self.explorer.sort == sort, label).clicked() {
                            self.explorer.set_sort(sort, self.explorer.descending);
                            ui.close_menu();
                        }
                    }
                    let mut descending = self.explorer.descending;
                    if ui.checkbox(&mut descending, "내림차순").changed() {
                        self.explorer.set_sort(self.explorer.sort, descending);
                    }
                });
                let caption = match &self.explorer.location {
                    Location::Recent => "최근 조사".to_owned(),
                    Location::Computer => "내 PC".to_owned(),
                    Location::Folder(p) => p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()),
                };
                ui.add(egui::Label::new(egui::RichText::new(caption).strong()).truncate());
            });
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 64.0).max(20.0);
                let address = ui.add_sized([width, 32.0], egui::TextEdit::singleline(&mut self.explorer.address).hint_text("경로"));
                let enter = address.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add_sized([56.0, 32.0], Button::new("열기")).clicked() || enter {
                    let path = std::path::PathBuf::from(self.explorer.address.trim());
                    if !path.as_os_str().is_empty() {
                        self.explorer.navigate(Location::Folder(path));
                    }
                }
            });
            ui.horizontal(|ui| {
                let mut filter_text = self.explorer.filter.clone();
                let filter = ui.add_sized([(ui.available_width() - 104.0).max(20.0), 32.0], egui::TextEdit::singleline(&mut filter_text).hint_text("이름·폴더로 찾기"));
                if filter.changed() {
                    self.explorer.set_filter(filter_text);
                }
                let current = match &self.explorer.location {
                    Location::Folder(p) => Some(p.clone()),
                    _ => None,
                };
                let pinned = current.as_ref().is_some_and(|p| self.explorer.pins.contains(p));
                if ui.add_enabled(current.is_some(), Button::new(if pinned { "고정 해제" } else { "폴더 고정" }).min_size(Vec2::new(96.0, 32.0))).clicked() {
                    if let Some(path) = current {
                        if pinned {
                            self.explorer.unpin(&path);
                        } else {
                            self.explorer.pin(path);
                        }
                    }
                }
            });
            egui::TopBottomPanel::bottom("home_explorer_footer").exact_height(36.0).show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.explorer.selected.is_some(), Button::new("열기").min_size(Vec2::new(88.0, 32.0))).clicked() {
                        self.activate_explorer_selection();
                    }
                    if self.explorer.listing_busy() {
                        ui.spinner();
                    }
                    self.render_explorer_connection(ui);
                    let text = self.explorer.notice.clone().unwrap_or_else(|| {
                        let count = if self.explorer.location == Location::Recent { self.recent_surveys.len() } else { self.explorer.visible().len() };
                        format!("{count}개")
                    });
                    ui.add(egui::Label::new(text).truncate());
                });
            });
            if wide {
                egui::SidePanel::left("home_explorer_places").default_width(206.0).width_range(160.0..=260.0).show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.render_explorer_places(ui));
                });
            }
            egui::CentralPanel::default().frame(egui::Frame::NONE).show_inside(ui, |ui| self.render_explorer_entries(ui));
        });
    }

    fn render_explorer_places(&mut self, ui: &mut egui::Ui) {
        let mut places = vec![
            ("최근 조사".to_owned(), Location::Recent),
            ("내 PC".to_owned(), Location::Computer),
            ("바탕화면".to_owned(), Location::Folder(crate::user_settings::resolved_desktop_path())),
            ("내려받기".to_owned(), Location::Folder(crate::user_settings::downloads_path())),
        ];
        if let Some(path) = crate::user_settings::documents_path() {
            places.insert(3, ("문서".to_owned(), Location::Folder(path)));
        }
        for (text, location) in places {
            if ui.add_sized([ui.available_width(), 32.0], Button::new(text).selected(self.explorer.location == location).truncate()).clicked() {
                self.explorer.navigate(location);
                ui.close_menu();
            }
        }
        ui.separator();
        ui.strong("폴더");
        for path in self.explorer.pins.clone() {
            let title = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
            let response = ui
                .add_sized([ui.available_width(), 32.0], Button::new(title).selected(self.explorer.location == Location::Folder(path.clone())).truncate())
                .on_hover_text(path.display().to_string());
            if response.clicked() {
                self.explorer.navigate(Location::Folder(path.clone()));
                ui.close_menu();
            }
            response.context_menu(|ui| {
                if ui.button("고정 해제").clicked() {
                    self.explorer.unpin(&path);
                    ui.close_menu();
                }
            });
        }
    }

    pub(super) fn activate_explorer_selection(&mut self) {
        let Some(path) = self.explorer.selected.clone() else { return };
        if self.explorer.location == Location::Recent {
            self.request_open_recent(path.display().to_string());
        } else if self.explorer.entries.iter().any(|entry| entry.path == path && entry.directory) {
            self.explorer.navigate(Location::Folder(path));
        } else {
            let before = self.left_layers.entries.len();
            self.activate_drawer_file(&path);
            if self.left_layers.entries.len() > before {
                self.active_tab = crate::app_state::MainTab::Map;
            } else {
                self.explorer.notice = Some(self.status_left_message().to_owned());
            }
        }
    }
}
