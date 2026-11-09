//! 시추 창·닫기 확인·끌어놓기·시추 도면 재생성.
//!
//! `gui_shell.rs` 가 300줄을 넘어 옮겼다. 본문은 그대로다.
use eframe::egui::{self, Color32, Vec2};
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{ActiveModalDialog};

impl StrataAppState {
    /// C++ `KaBoreholeDialog::openBoreholeMapDialog` 의 매 프레임 화면 (`d7afcc8`).
    /// 창은 비모달이라 다른 작업을 막지 않는다. 세 단추의 동작은 여기서 이어 받는다.
    pub(crate) fn render_borehole_dialog_frame(&mut self, ctx: &egui::Context) {
        if !self.borehole_dialog.open {
            return;
        }
        let loaded_count = self.paleo_3d.declustered_borehole_count;
        let can_run = self.survey_path.is_some();
        let mut open = true;
        let action = crate::dialogs_borehole::render_borehole_dialog(
            ctx,
            &mut self.borehole_dialog.state,
            loaded_count,
            can_run,
            &mut open,
        );
        if !open {
            self.borehole_dialog.open = false;
            return;
        }
        let Some(action) = action else {
            return;
        };
        use crate::dialogs_borehole::BoreholeDialogAction;
        match action {
            // C++ 는 반경을 넘겨 고지형 자료 받기를 시작한다(`MainWindowRibbon.cpp:541-544`).
            BoreholeDialogAction::FetchAndGenerate { radius_meters } => {
                self.paleo_3d.source_dl_radius_km = f64::from(radius_meters) / 1000.0;
                self.paleo_3d.radius_km = f64::from(radius_meters) / 1000.0;
                self.status_bar_hint = format!(
                    "시추자료를 반경 {:.0} km로 받아 5가지 지반·시추 GIS 도면을 만들 준비를 합니다.",
                    self.paleo_3d.source_dl_radius_km
                );
                self.open_studio_tab(MainTab::Paleo);
            }
            // 이미 받은 주상도로 도면만 다시 만든다(`KaBoreholeDialog.cpp:170-181`).
            // C++ `GeoInfoBoreholeService::generateBoreholeGisMaps` 를 그대로 부른다.
            BoreholeDialogAction::Regenerate => {
                let created = self.regenerate_borehole_maps();
                self.borehole_dialog.last_created_count = created;
                if created == 0 {
                    self.status_bar_hint =
                        "적재된 시추공이 2공 미만이라 시추 도면을 만들지 않았습니다.".to_owned();
                } else {
                    self.status_bar_hint = format!(
                        "적재된 시추공 {loaded_count}공으로 시추 도면 {created}장을 다시 만들었습니다."
                    );
                }
                self.open_studio_tab(MainTab::Paleo);
            }
            // 옵션을 읽고 도면 스튜디오로 간다(`KaBoreholeDialog.cpp:182-186`).
            BoreholeDialogAction::OpenLayout => {
                self.borehole_dialog.open = false;
                self.open_studio_tab(MainTab::Drawing);
            }
        }
    }
    /// C++ `GeoInfoBoreholeService::generateBoreholeGisMaps` (`GeoInfoBoreholeMaps.cpp:179-294`)
    /// 를 지금 적재된 주상도로 다시 부른다.
    ///
    /// 돌려주는 값은 C++ `int created` 와 같은 "만든 도면 수"다. 조사 폴더가 없거나
    /// 적재된 시추공이 2공 미만이면 아무것도 만들지 않고 0 을 돌려준다
    /// (`GeoInfoBoreholeMaps.cpp:226` `xs.size() < 2` 조기 반환).
    pub(crate) fn regenerate_borehole_maps(&mut self) -> usize {
        let Some(survey_path) = self.survey_path.clone() else {
            return 0;
        };
        let Some(output) = self.paleo_3d.last_paleo_output.as_ref() else {
            return 0;
        };
        if output.parsed_boreholes.len() < strata_paleo::borehole_maps::BOREHOLE_MIN_POINTS {
            return 0;
        }
        // C++ `:247-250` — 조사 폴더 아래 `지형분석/` 에 쓴다.
        let out_dir = survey_path
            .parent()
            .map(|p| p.join("지형분석"))
            .unwrap_or_else(|| survey_path.clone());
        let holes = crate::borehole_bridge::borehole_records_from_parsed(&output.parsed_boreholes);
        let opts = self.borehole_dialog.state.to_map_options();
        match strata_paleo::borehole_maps::generate_borehole_gis_maps(
            &holes,
            &[],
            self.work_crs,
            &out_dir,
            &opts,
        ) {
            Ok(bundle) => {
                let created = bundle.created_count;
                // 파일만 만들고 끝내면 지도에 색이 안 칠해진다. 화면에 그릴 격자·램프를
                // 함께 들고 있어야 `gui_tab_paleo` 가 실제로 칠할 수 있다.
                if !bundle.painted_grids.is_empty() {
                    self.paleo_3d.shown_borehole_map_idx = Some(0);
                }
                self.paleo_3d.last_borehole_maps = Some(bundle);
                created
            }
            Err(_) => 0,
        }
    }
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
                    if ui.add(save_btn).clicked() {
                        self.is_dirty = false;
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
