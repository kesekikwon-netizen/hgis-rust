//! Window-width bounded file rows, target/cpp-ref/src/app/KaFileBrowserPanel.cpp:289-357 and KaHomeRecentCard.cpp:121-191.
use super::explorer::Location;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Vec2};

struct Row {
    path: std::path::PathBuf,
    name: String,
    detail: String,
    kind: String,
    bytes: String,
    modified: String,
    available: bool,
    tip: String,
}

fn bytes(value: Option<u64>) -> String {
    match value {
        None => "—".to_owned(),
        Some(n) if n < 1024 => format!("{n} B"),
        Some(n) if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        Some(n) => format!("{:.1} MB", n as f64 / 1048576.0),
    }
}
fn text(ui: &egui::Ui, at: Pos2, width: f32, value: String, strong: bool) {
    let rich = egui::RichText::new(value).size(if strong { 14.0 } else { 11.5 }).color(if strong { ui.visuals().text_color() } else { Color32::from_rgb(115, 111, 101) });
    let rich = if strong { rich.strong() } else { rich };
    let galley = egui::WidgetText::from(rich).into_galley(ui, Some(egui::TextWrapMode::Truncate), width.max(1.0), egui::TextStyle::Body);
    ui.painter().galley(at, galley, ui.visuals().text_color());
}

impl StrataAppState {
    fn explorer_row(&self, index: usize, recent: bool) -> Row {
        if recent {
            let r = &self.recent_surveys[index];
            let folder = crate::home_text::folder(&r.path);
            let facts = self.facts_of(&r.path);
            let counts = if facts.has_counts() {
                format!("\n조사구역 {} · 유구 {} · {}", facts.areas, facts.features, strata_storage::survey_facts::check_text(&facts))
            } else {
                String::new()
            };
            Row {
                path: r.path.clone().into(),
                name: r.name.clone(),
                detail: crate::home_text::folder_end(&folder),
                kind: if r.available { "조사" } else { "원본 없음" }.to_owned(),
                bytes: "—".to_owned(),
                modified: crate::home_text::when(r.last_opened_ms),
                available: r.available,
                tip: format!("{}\n{}{}", crate::home_text::folder_tail(&folder, &r.name), r.path, counts),
            }
        } else {
            let e = &self.explorer.entries[index];
            Row {
                path: e.path.clone(),
                name: e.name.clone(),
                detail: e.path.display().to_string(),
                kind: if e.directory { "폴더".to_owned() } else { e.path.extension().map_or_else(|| "파일".to_owned(), |v| v.to_string_lossy().to_uppercase()) },
                bytes: bytes(e.bytes),
                modified: e.modified.map_or_else(|| "—".to_owned(), |time| chrono::DateTime::<chrono::Local>::from(time).format("%Y-%m-%d %H:%M").to_string()),
                available: true,
                tip: e.path.display().to_string(),
            }
        }
    }
    pub(super) fn render_explorer_entries(&mut self, ui: &mut egui::Ui) {
        let filter = self.explorer.filter.trim().to_lowercase();
        let recent = self.explorer.location == Location::Recent;
        // O(n) filtering, O(visible rows) formatting and painting; C++ KaFileBrowserPanel.cpp:351-357 lists files.
        let indices: Vec<usize> = if recent {
            self.recent_surveys
                .iter()
                .enumerate()
                .filter_map(|(index, r)| (filter.is_empty() || r.name.to_lowercase().contains(&filter) || r.path.to_lowercase().contains(&filter)).then_some(index))
                .collect()
        } else {
            self.explorer.entries.iter().enumerate().filter_map(|(index, e)| (filter.is_empty() || e.name.to_lowercase().contains(&filter)).then_some(index)).collect()
        };
        let wide = ui.available_width() >= 650.0;
        let width = (ui.available_width() - 12.0).max(1.0);
        let name_width = if wide { width - 320.0 } else { width - 24.0 };
        if ui.available_height() > 110.0 {
            let (header, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), egui::Sense::hover());
            text(ui, header.min + Vec2::new(12.0, 4.0), name_width, "이름".to_owned(), true);
            if wide {
                text(ui, header.min + Vec2::new(width - 300.0, 4.0), 62.0, "종류".to_owned(), false);
                text(ui, header.min + Vec2::new(width - 224.0, 4.0), 72.0, "크기".to_owned(), false);
                text(
                    ui,
                    header.min + Vec2::new(width - 138.0, 4.0),
                    128.0,
                    if self.explorer.location == Location::Recent { "마지막 열림" } else { "수정한 날짜" }.to_owned(),
                    false,
                );
            }
            ui.separator();
        }
        if indices.is_empty() && !self.explorer.listing_busy() {
            ui.add(
                egui::Label::new(if self.explorer.location == Location::Recent {
                    "아직 최근 조사가 없습니다. 새 조사 또는 조사 열기로 시작하세요."
                } else {
                    "0개"
                })
                .wrap(),
            );
        }
        let mut activate = false;
        let mut forget = None;
        let selected_before = self.explorer.selected.clone();
        ui.spacing_mut().item_spacing.y = 4.0;
        egui::ScrollArea::vertical().id_salt("home_explorer_rows").auto_shrink([false, false]).show_rows(ui, 44.0, indices.len(), |ui, range| {
            for index in range {
                let row = self.explorer_row(indices[index], recent);
                let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 44.0), egui::Sense::click());
                let selected = self.explorer.selected.as_ref() == Some(&row.path);
                let fill = if selected {
                    ui.visuals().selection.bg_fill
                } else if response.hovered() {
                    let [r, g, b, a] = self.theme.hover;
                    Color32::from_rgba_unmultiplied(r, g, b, a)
                } else if index % 2 == 0 {
                    let [r, g, b, a] = self.theme.alt_row;
                    Color32::from_rgba_unmultiplied(r, g, b, a)
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 4.0, fill);
                text(ui, rect.min + Vec2::new(12.0, 6.0), name_width, row.name.clone(), true);
                text(ui, rect.min + Vec2::new(12.0, 27.0), name_width, if wide { row.detail.clone() } else { format!("{} · {} · {}", row.kind, row.bytes, row.modified) }, false);
                if wide {
                    text(ui, rect.min + Vec2::new(width - 300.0, 16.0), 62.0, row.kind.clone(), false);
                    text(ui, rect.min + Vec2::new(width - 224.0, 16.0), 72.0, row.bytes.clone(), false);
                    text(ui, rect.min + Vec2::new(width - 138.0, 16.0), 128.0, row.modified.clone(), false);
                }
                let response = response.on_hover_text(&row.tip);
                if recent {
                    response.context_menu(|ui| {
                        if ui.button("목록에서 제거").clicked() {
                            forget = Some(row.path.display().to_string());
                            ui.close_menu();
                        }
                    });
                }
                if response.clicked() {
                    self.explorer.selected = Some(row.path.clone());
                }
                if response.double_clicked() && row.available {
                    self.explorer.selected = Some(row.path.clone());
                    activate = true;
                }
            }
        });
        if let Some(path) = forget {
            self.forget_recent_survey(&path);
            self.explorer.selected = None;
        }
        if !ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && selected_before.is_some() {
            activate = true;
        }
        if activate {
            self.activate_explorer_selection();
        }
    }
}
