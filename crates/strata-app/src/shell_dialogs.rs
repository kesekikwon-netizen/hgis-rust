//! Dropped files (target/cpp-ref/src/app/MainWindowFiles.cpp:259-289) and close prompt
//! (target/cpp-ref/src/app/KaClosePrompt.cpp).
use eframe::egui::{self, Color32, Vec2};
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// `MainWindowFiles.cpp:259-289` `tryAddDroppedPaths` — 창에 끌어 놓은 파일을
    /// 확장자에 따라 레이어로 올린다. CAD 도면은 안내 창을 먼저 띄운다.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let mut added = 0usize;
        let mut cad_path: Option<String> = None;
        let mut skipped = 0usize;
        for f in &dropped {
            let Some(path) = f.path.as_ref() else {
                skipped += 1;
                continue;
            };
            let name = path.to_string_lossy().to_string();
            let low = name.to_lowercase();
            let is_cad = low.ends_with(".dxf") || low.ends_with(".dwg");
            let ok = low.ends_with(".shp")
                || is_cad
                || low.ends_with(".gpkg")
                || low.ends_with(".geojson")
                || low.ends_with(".json")
                || low.ends_with(".tif")
                || low.ends_with(".tiff")
                || low.ends_with(".img")
                || low.ends_with(".csv")
                || low.ends_with(".xlsx")
                || low.ends_with(".hgis");
            if !ok {
                skipped += 1;
                continue;
            }
            if is_cad {
                cad_path = Some(name);
                continue;
            }
            added += 1;
        }
        if let Some(path) = cad_path {
            // KaCadImportNotice.cpp — 도면은 좌표계 확인 안내를 먼저 보여준다
            self.modals.active = Some(ActiveModalDialog::CadImportNotice);
            self.modals.cad_import_path = path;
            self.home_map.draw_guide_message =
                Some("CAD 도면을 끌어 놓았습니다. 좌표계를 확인하고 지도에 올리세요.".to_owned());
            return;
        }
        if added > 0 {
            let msg = format!("레이어 {added}개 추가됨 (파일→지도)");
            self.last_drop_message = Some(msg.clone());
            self.home_map.draw_guide_message = Some(msg);
            self.active_tab = MainTab::Map;
            self.is_dirty = true;
        } else if skipped > 0 {
            self.home_map.draw_guide_message =
                Some("지도에 올릴 수 있는 파일이 아닙니다.".to_owned());
        }
    }
    /// `KaClosePrompt.cpp` — 저장하지 않은 편집이 있으면 닫기 전에 한 번 묻는다.
    pub(crate) fn render_close_prompt(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty && !self.close_prompt_open
        {
            self.close_prompt_open = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if !self.close_prompt_open {
            return;
        }
        let full = ctx.screen_rect();
        let w = 420.0_f32.min(full.width() - 32.0);
        let h = 168.0_f32.min(full.height() - 32.0);
        let pos = egui::Pos2::new(
            ((full.width() - w) * 0.5).max(16.0),
            ((full.height() - h) * 0.5).max(16.0),
        );
        egui::Window::new("ka_close_prompt")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(pos)
            .fixed_size([w, h])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("저장하지 않은 편집이 있습니다")
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("끝내기 전에 지금까지 그린 조사 내용을 저장할까요?")
                        .size(12.0)
                        .color(Color32::from_rgb(80, 79, 74)),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("저장하고 끝내기")
                            .color(Color32::WHITE)
                            .strong()
                            .size(12.5),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(130.0, 30.0));
                    // C++ KaClosePrompt — 실제로 저장하고, 모두 저장됐을 때만 닫는다(실패하면 창은 그대로).
                    if ui.add(save_btn).clicked() && self.save_survey_project() && !self.is_dirty {
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("저장하지 않고 끝내기").size(12.0),
                            )
                            .min_size(Vec2::new(150.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("취소").size(12.0))
                                    .min_size(Vec2::new(70.0, 30.0)),
                            )
                            .clicked()
                        {
                            self.close_prompt_open = false;
                        }
                    });
                });
            });
    }
}
