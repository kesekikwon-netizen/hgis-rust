use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon_icons::paint_strata_logo;

impl StrataAppState {
    pub(crate) fn render_home_tab_ui(&mut self, ui: &mut egui::Ui) {
        let total_w = ui.available_width().clamp(800.0, 3600.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(total_w);
            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 14)).show(ui, |ui| {
                let inner_w = (total_w - 40.0).max(760.0);
                ui.set_max_width(inner_w);

                let hero_w = (inner_w * 0.26).max(240.0);
                let title_w = (inner_w - hero_w - 20.0).max(480.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(hero_w, 146.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let (logo_r, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), egui::Sense::hover());
                            const APP_ICON_RGBA: &[u8] =
                                include_bytes!("../../../data/theme/ka-hgis-app-paper-96.rgba");
                            let tex = ui.ctx().load_texture(
                                "ka_app_icon_paper_96",
                                egui::ColorImage::from_rgba_unmultiplied([96, 96], APP_ICON_RGBA),
                                egui::TextureOptions::LINEAR,
                            );
                            ui.painter_at(logo_r).image(
                                tex.id(),
                                logo_r,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            let _ = paint_strata_logo;
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Strata").size(28.0).strong());
                            ui.label(egui::RichText::new("|").size(20.0).color(Color32::from_rgb(160, 158, 150)));
                            ui.label(egui::RichText::new("필드고고학 GIS").size(14.5).color(Color32::from_rgb(80, 79, 74)));
                        });
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.").size(13.0).color(Color32::from_rgb(75, 74, 70)));
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let (b1_r, b1_resp) =
                                ui.allocate_exact_size(Vec2::new(102.0, 36.0), egui::Sense::click());
                            {
                                let p = ui.painter_at(b1_r);
                                p.rect_filled(b1_r, 6.0, Color32::WHITE);
                                p.rect_stroke(
                                    b1_r,
                                    6.0,
                                    Stroke::new(1.2, Color32::from_rgb(45, 44, 40)),
                                    egui::StrokeKind::Inside,
                                );
                                let ic = Rect::from_center_size(
                                    Pos2::new(b1_r.left() + 21.0, b1_r.center().y),
                                    Vec2::new(13.0, 13.0),
                                );
                                let ink = Color32::from_rgb(35, 35, 32);
                                p.rect_stroke(ic, 2.5, Stroke::new(1.4, ink), egui::StrokeKind::Inside);
                                p.line_segment(
                                    [Pos2::new(ic.center().x - 3.5, ic.center().y), Pos2::new(ic.center().x + 3.5, ic.center().y)],
                                    Stroke::new(1.4, ink),
                                );
                                p.line_segment(
                                    [Pos2::new(ic.center().x, ic.center().y - 3.5), Pos2::new(ic.center().x, ic.center().y + 3.5)],
                                    Stroke::new(1.4, ink),
                                );
                                p.text(
                                    Pos2::new(b1_r.left() + 34.0, b1_r.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    "새 조사",
                                    egui::FontId::proportional(13.0),
                                    ink,
                                );
                            }
                            if b1_resp.clicked() {
                                self.modals.active = Some(ActiveModalDialog::NewSurvey);
                            }
                            let (b2_r, b2_resp) =
                                ui.allocate_exact_size(Vec2::new(112.0, 36.0), egui::Sense::click());
                            {
                                let p = ui.painter_at(b2_r);
                                p.rect_filled(b2_r, 6.0, Color32::WHITE);
                                p.rect_stroke(
                                    b2_r,
                                    6.0,
                                    Stroke::new(1.0, Color32::from_rgb(175, 172, 162)),
                                    egui::StrokeKind::Inside,
                                );
                                let ic = Rect::from_center_size(
                                    Pos2::new(b2_r.left() + 21.0, b2_r.center().y),
                                    Vec2::new(22.0, 22.0),
                                );
                                crate::ribbon_icons::paint_ribbon_icon(
                                    &p,
                                    ic,
                                    "열기",
                                    Color32::from_rgb(55, 54, 50),
                                );
                                p.text(
                                    Pos2::new(b2_r.left() + 34.0, b2_r.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    "조사 열기",
                                    egui::FontId::proportional(13.0),
                                    Color32::from_rgb(35, 35, 32),
                                );
                            }
                            if b2_resp.clicked() {
                                self.open_sample_survey_project(0);
                            }
                        });
                    });
                    ui.add_space(16.0);
                    let (tb_rect, _) = ui.allocate_exact_size(Vec2::new(title_w, 146.0), egui::Sense::hover());
                    self.paint_home_title_block(ui, tb_rect);
                });

                ui.add_space(22.0);

                // C++ KaStartPage.cpp: recent list takes ~77% and the guide column the rest.
                // Reserve the gap AND the guide column before sizing the table, otherwise the
                // guide column is pushed past the right edge (window 2308 -> table ran to 2307).
                const GUIDE_GAP: f32 = 18.0;
                const GUIDE_MIN_W: f32 = 300.0;
                let recent_w = (inner_w - GUIDE_GAP - GUIDE_MIN_W)
                    .min((inner_w * 0.77).max(520.0))
                    .max(320.0);
                let guide_w = (inner_w - recent_w - GUIDE_GAP).max(GUIDE_MIN_W);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(recent_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("최근 조사").size(15.0).strong());
                            ui.label(egui::RichText::new("4개").size(12.0).color(Color32::from_rgb(110, 108, 102)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let rf = ui.add(
                                    egui::TextEdit::singleline(&mut self.home_map.recent_filter)
                                        .hint_text("     이름·폴더로 찾기")
                                        .desired_width(260.0),
                                );
                                let sp = ui.painter();
                                let c = Pos2::new(rf.rect.left() + 11.0, rf.rect.center().y - 1.0);
                                let ink = Color32::from_rgb(110, 108, 102);
                                sp.circle_stroke(c, 4.4, Stroke::new(1.4, ink));
                                sp.line_segment(
                                    [Pos2::new(c.x + 3.2, c.y + 3.2), Pos2::new(c.x + 6.6, c.y + 6.6)],
                                    Stroke::new(1.5, ink),
                                );
                            });
                        });
                        ui.add_space(6.0);
                        self.render_recent_survey_box(ui, recent_w);
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("목록에서 빼려면 조사 위에서 오른쪽 단추를 누르세요.").size(11.5).color(Color32::from_rgb(105, 103, 96)));
                    });
                    ui.add_space(18.0);
                    ui.allocate_ui_with_layout(Vec2::new(guide_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("작업 순서").size(15.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("경주시 기준").size(11.5).color(Color32::from_rgb(105, 103, 96)));
                            });
                        });
                        ui.add_space(10.0);
                        // C++ KaHomeGuideCard.cpp: 18px circle nodes joined by a 1px vertical
                        // connector. Step 1 is done (check), step 2 is current (orange dot),
                        // step 3 is pending (number).
                        let ring_rgb = Color32::from_rgb(45, 44, 40);
                        let link_rgb = Color32::from_rgb(185, 182, 172);
                        let node_size = 18.0;
                        for (step_no, title, is_current, desc, step_id) in [
                            ("1", "새 조사", false, "조사 이름과 좌표계(5186·5187)를 정하면 조사 파일(GPKG)이 만들어집니다.", 1),
                            ("2", "조사구역 그리기", true, "지도 탭에서 위성·지적을 보며 「그리기」로 구역과 유구를 그립니다.", 2),
                            ("3", "도면 · 제출", false, "「도면」으로 종이에 옮기고, 「검수·제출」에서 검수한 뒤 제출 꾸러미(5179 SHP·조사도면.pdf·MANIFEST)를 만듭니다.\n아직 검수하지 않았습니다 — 지도 탭 「검수·제출」에서 확인합니다.", 3),
                        ] {
                            let row_top = ui.cursor().top();
                            ui.horizontal_top(|ui| {
                                let (node_r, _) = ui.allocate_exact_size(
                                    Vec2::new(node_size, node_size),
                                    egui::Sense::hover(),
                                );
                                let np = ui.painter();
                                let c = node_r.center();
                                // connector runs from this node's centre down past the row gap
                                let connector_top = c.y + node_size * 0.5;
                                let connector_bottom = row_top + 100.0;
                                if step_no != "3" {
                                    np.line_segment(
                                        [
                                            Pos2::new(c.x, connector_top),
                                            Pos2::new(c.x, connector_bottom),
                                        ],
                                        Stroke::new(1.0, link_rgb),
                                    );
                                }
                                np.circle_filled(c, node_size * 0.5, Color32::WHITE);
                                if is_current {
                                    np.circle_filled(c, 4.0, Color32::from_rgb(181, 87, 58));
                                } else {
                                    np.circle_stroke(
                                        c,
                                        node_size * 0.5,
                                        Stroke::new(1.2, ring_rgb),
                                    );
                                    if step_no == "1" {
                                        np.line_segment(
                                            [
                                                Pos2::new(c.x - 3.6, c.y + 0.2),
                                                Pos2::new(c.x - 1.0, c.y + 2.8),
                                            ],
                                            Stroke::new(1.5, ring_rgb),
                                        );
                                        np.line_segment(
                                            [
                                                Pos2::new(c.x - 1.0, c.y + 2.8),
                                                Pos2::new(c.x + 3.8, c.y - 2.4),
                                            ],
                                            Stroke::new(1.5, ring_rgb),
                                        );
                                    } else {
                                        np.text(
                                            c,
                                            egui::Align2::CENTER_CENTER,
                                            step_no,
                                            egui::FontId::proportional(9.5),
                                            ring_rgb,
                                        );
                                    }
                                }
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.link(egui::RichText::new(title).strong().size(13.0)).clicked() {
                                            match step_id {
                                                1 => self.start_new_survey_project("경주시"),
                                                2 => self.trigger_map_ribbon_tool(MapToolKind::DrawPolygon),
                                                _ => self.modals.active = Some(ActiveModalDialog::PaperSetup),
                                            }
                                        }
                                        if is_current {
                                            ui.colored_label(Color32::from_rgb(181, 87, 58), egui::RichText::new("• 다음").size(11.5).strong());
                                        }
                                    });
                                    ui.label(egui::RichText::new(desc).size(11.5).color(Color32::from_rgb(95, 94, 88)));
                                });
                            });
                            ui.add_space(8.0);
                        }
                        // Workflow state is still evaluated (callers rely on the side effects of
                        // the counters below), but C++ shows no bottom progress sentence.
                        let _wf_states = strata_storage::project_actions::evaluate_workflow_guide(
                            &strata_storage::project_actions::WorkflowProjectInput {
                                survey_name: self.project_title.clone(),
                                layer_count: self.left_layers.entries.len(),
                                survey_area_count: self.home_map.real_survey.survey_rings.len(),
                                feature_poly_count: self.home_map.real_survey.trenches.len(),
                                feature_line_count: 0,
                                control_points_count: 2,
                                has_datum: true,
                                has_ellipsoid: true,
                                has_projection: true,
                                has_visible_reference_layer: true,
                                checklist_error_count: 0,
                                package_created: false,
                            },
                        );
                    });
                });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("연결 상태").size(14.5).strong());
                    let (lock_rect, _) =
                        ui.allocate_exact_size(Vec2::new(10.0, 13.0), egui::Sense::hover());
                    {
                        let lp = ui.painter_at(lock_rect);
                        let lc = Color32::from_rgb(105, 103, 96);
                        let body = Rect::from_min_max(
                            Pos2::new(lock_rect.left() + 1.5, lock_rect.top() + 5.5),
                            Pos2::new(lock_rect.right() - 1.5, lock_rect.bottom() - 1.0),
                        );
                        lp.rect_stroke(body, 1.5, Stroke::new(1.1, lc), egui::StrokeKind::Inside);
                        lp.circle_stroke(
                            Pos2::new(lock_rect.center().x, lock_rect.top() + 4.5),
                            2.3,
                            Stroke::new(1.1, lc),
                        );
                    }
                    ui.label(
                        egui::RichText::new("비밀값은 표시되지 않습니다")
                            .size(12.0)
                            .color(Color32::from_rgb(105, 103, 96)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // C++ KaHomeConnectionCard.cpp: plain white button with a 1px rule.
                        let net_btn = egui::Button::new(egui::RichText::new("인터넷 확인").size(12.0))
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(120, 118, 110)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(0.0, 26.0));
                        if ui.add(net_btn).clicked() {
                            self.home_map.internet_checked = true;
                            self.home_map.draw_guide_message = Some("인터넷 연결 및 VWorld·국토정보플랫폼·국가유산청 계정 설정을 확인했습니다.".to_owned());
                        }
                        let net_txt = if self.home_map.internet_checked { "인터넷 · 연결됨" } else { "인터넷 · 확인 안 함" };
                        ui.label(egui::RichText::new(net_txt).size(12.0).color(Color32::from_rgb(95, 94, 88)));
                    });
                });
                ui.add_space(8.0);
                let statuses = [
                    ("VWorld 키", self.accounts.vworld_configured),
                    ("역사지리정보DB 키", self.accounts.history_gis_configured),
                    ("VWorld 지적도 계정", self.accounts.cadastral_configured),
                    ("수치지형도 계정", self.accounts.ngii_configured),
                    ("문화재 인트라넷", self.accounts.heritage_configured),
                ];
                let sep_rgb = Color32::from_rgb(212, 209, 200);
                let ink_rgb = Color32::from_rgb(55, 54, 50);
                let cols_rect = ui.available_rect_before_wrap();
                let col_w = cols_rect.width() / 5.0;
                ui.columns(5, |cols| {
                    for (idx, (title, ready)) in statuses.into_iter().enumerate() {
                        // 1px vertical rule between adjacent account columns (C++ draws it
                        // before every column except the first).
                        if idx > 0 {
                            let x = cols[idx].cursor().left() - 6.0;
                            let top = cols_rect.top();
                            let bottom = top + 44.0;
                            cols[idx].painter().line_segment(
                                [Pos2::new(x, top), Pos2::new(x, bottom)],
                                Stroke::new(1.0, sep_rgb),
                            );
                        }
                        let _ = (col_w, ink_rgb);
                        cols[idx].horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(title).size(12.5).strong());
                                let st_txt = if ready { "설정됨" } else { "미설정" };
                                ui.label(egui::RichText::new(st_txt).size(11.5).color(Color32::from_rgb(105, 103, 96)));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // C++ uses dark ink text, not a blue hyperlink.
                                let resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("설정")
                                            .size(12.0)
                                            .color(ink_rgb),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if resp.clicked() {
                                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                                }
                            });
                        });
                    }
                });
            });
        });
    }

    fn paint_home_title_block(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        let border = Stroke::new(1.0, Color32::from_rgb(35, 35, 32));
        let sub_border = Stroke::new(1.0, Color32::from_rgb(185, 182, 172));
        p.rect_filled(rect, 3.0, Color32::WHITE);
        p.rect_stroke(rect, 3.0, border, egui::StrokeKind::Middle);

        let (y1, y2) = (rect.top() + 28.0, rect.top() + 96.0);
        p.line_segment(
            [Pos2::new(rect.left(), y1), Pos2::new(rect.right(), y1)],
            sub_border,
        );
        p.line_segment(
            [Pos2::new(rect.left(), y2), Pos2::new(rect.right(), y2)],
            sub_border,
        );
        p.text(
            Pos2::new(rect.left() + 12.0, rect.top() + 14.0),
            egui::Align2::LEFT_CENTER,
            "이어서 작업",
            egui::FontId::proportional(11.5),
            Color32::from_rgb(95, 94, 88),
        );
        p.text(
            Pos2::new(rect.right() - 12.0, rect.top() + 14.0),
            egui::Align2::RIGHT_CENTER,
            "저장됨",
            egui::FontId::proportional(11.5),
            Color32::from_rgb(95, 94, 88),
        );

        let split_x = rect.right() - 210.0;
        p.line_segment([Pos2::new(split_x, y1), Pos2::new(split_x, y2)], sub_border);
        p.text(
            Pos2::new(rect.left() + 14.0, (y1 + y2) * 0.5),
            egui::Align2::LEFT_CENTER,
            "경주시",
            egui::FontId::proportional(26.0),
            Color32::from_rgb(30, 30, 28),
        );
        let btn_rect = Rect::from_center_size(
            Pos2::new((split_x + rect.right()) * 0.5, (y1 + y2) * 0.5),
            Vec2::new(178.0, 38.0),
        );
        p.rect_filled(btn_rect, 5.0, Color32::from_rgb(181, 87, 58));
        p.rect_stroke(
            btn_rect,
            5.0,
            Stroke::new(1.0, Color32::from_rgb(35, 35, 32)),
            egui::StrokeKind::Inside,
        );
        p.text(
            btn_rect.center(),
            egui::Align2::CENTER_CENTER,
            "이어서 열기  →",
            egui::FontId::proportional(13.5),
            Color32::WHITE,
        );
        if ui
            .interact(
                btn_rect,
                ui.id().with("home_resume_btn"),
                egui::Sense::click(),
            )
            .clicked()
        {
            self.open_sample_survey_project(0);
        }

        // KaHomeTitleBlock.cpp:133-137 crsText — 작업 좌표계는 실제 작업 CRS 에서 만든다.
        let crs_txt = match self.work_crs {
            strata_types::crs::WorkCrs::Epsg5186 => "5186 중부원점",
            strata_types::crs::WorkCrs::Epsg5187 => "5187 동부원점",
        }
        .to_owned();
        // KaHomeTitleBlock.cpp:148-152 — 조사구역·유구는 실제 개수, 없으면 「—」.
        let ring_cnt = self.home_map.real_survey.survey_rings.len();
        let area_cnt = if ring_cnt > 0 {
            format!("{ring_cnt} 곳")
        } else {
            "—".to_owned()
        };
        let cells = [
            ("폴더", "문서 › 경주시"),
            ("마지막 열림", "어제 22:26"),
            ("작업 좌표계", crs_txt.as_str()),
            ("조사구역", area_cnt.as_str()),
            ("유구", "—"),
            ("검수", "검수 전"),
        ];
        let cw = rect.width() / 6.0;
        for (i, (k, v)) in cells.iter().enumerate() {
            let x0 = rect.left() + cw * (i as f32);
            if i > 0 {
                p.line_segment(
                    [Pos2::new(x0, y2), Pos2::new(x0, rect.bottom())],
                    sub_border,
                );
            }
            // KaHomeTitleBlock.cpp:88 — contentsMargins(24, 10, 16, 12), spacing 2
            p.text(
                Pos2::new(x0 + 24.0, y2 + 10.0 + 5.5),
                egui::Align2::LEFT_CENTER,
                *k,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(105, 103, 96),
            );
            p.text(
                Pos2::new(x0 + 24.0, y2 + 10.0 + 11.0 + 2.0 + 7.0),
                egui::Align2::LEFT_CENTER,
                *v,
                egui::FontId::proportional(12.5),
                Color32::from_rgb(35, 35, 32),
            );
        }
    }

    fn render_recent_survey_box(&mut self, ui: &mut egui::Ui, width: f32) {
        let (box_r, _) = ui.allocate_exact_size(Vec2::new(width, 206.0), egui::Sense::hover());
        let p = ui.painter_at(box_r);
        p.rect_filled(box_r, 4.0, Color32::WHITE);
        p.rect_stroke(
            box_r,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(45, 44, 40)),
            egui::StrokeKind::Middle,
        );

        let col_x = [
            box_r.left() + 14.0,
            box_r.left() + width * 0.62,
            box_r.left() + width * 0.71,
            box_r.left() + width * 0.78,
            box_r.left() + width * 0.84,
            box_r.left() + width * 0.91,
        ];
        for (i, h) in ["조사", "상태", "조사구역", "유구", "검수", "마지막 열림"]
            .iter()
            .enumerate()
        {
            p.text(
                Pos2::new(col_x[i], box_r.top() + 15.0),
                egui::Align2::LEFT_CENTER,
                *h,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(95, 94, 88),
            );
        }
        p.line_segment(
            [
                Pos2::new(box_r.left(), box_r.top() + 28.0),
                Pos2::new(box_r.right(), box_r.top() + 28.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(222, 220, 210)),
        );

        let rows = [
            (
                "경주시",
                "문서 › 경주시",
                "저장됨",
                "1",
                "0",
                "검수 전",
                "어제 22:26",
                true,
            ),
            (
                "verify1-저장",
                "... › ka-hgis-survey-verify",
                "⊗ 원본 없음",
                "—",
                "—",
                "—",
                "찾을 수 없음",
                false,
            ),
            (
                "verify1",
                "... › ka-hgis-survey-verify",
                "⊗ 원본 없음",
                "—",
                "—",
                "—",
                "찾을 수 없음",
                false,
            ),
            (
                "andong",
                "바탕 화면 › andong",
                "⊗ 원본 없음",
                "—",
                "—",
                "—",
                "찾을 수 없음",
                false,
            ),
        ];
        for (idx, (name, folder, st, area, feat, chk, opened, ok)) in rows.into_iter().enumerate() {
            if !self.home_map.recent_filter.trim().is_empty()
                && !name.contains(self.home_map.recent_filter.trim())
            {
                continue;
            }
            let ry = box_r.top() + 30.0 + (idx as f32) * 43.0;
            let row_r = Rect::from_min_size(
                Pos2::new(box_r.left() + 1.0, ry),
                Vec2::new(width - 2.0, 42.0),
            );
            if idx == 0 {
                p.rect_filled(row_r, 0.0, Color32::from_rgb(239, 236, 226));
            }
            p.text(
                Pos2::new(col_x[0], ry + 13.0),
                egui::Align2::LEFT_CENTER,
                name,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(35, 35, 32),
            );
            p.text(
                Pos2::new(col_x[0], ry + 30.0),
                egui::Align2::LEFT_CENTER,
                folder,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(115, 113, 106),
            );
            let st_col = if ok {
                Color32::from_rgb(65, 64, 60)
            } else {
                Color32::from_rgb(181, 87, 58)
            };
            for (ci, val) in [(1, st), (2, area), (3, feat), (4, chk), (5, opened)] {
                let c = if ci == 1 {
                    st_col
                } else {
                    Color32::from_rgb(65, 64, 60)
                };
                p.text(
                    Pos2::new(col_x[ci], ry + 21.0),
                    egui::Align2::LEFT_CENTER,
                    val,
                    egui::FontId::proportional(12.0),
                    c,
                );
            }
            if ui
                .interact(
                    row_r,
                    ui.id().with(("recent_row", idx)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                self.open_sample_survey_project(idx);
            }
        }
    }

    pub fn start_new_survey_project(&mut self, name: &str) {
        self.project_title = name.to_owned();
        let path = std::env::temp_dir().join(format!("{name}.gpkg"));
        let _ = GpkgRepository::create_new_survey(&path, self.work_crs);
        self.left_layers
            .ensure_domain_layer(DomainLayerKey::SurveyArea);
        self.home_map.fit_to_survey();
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some("위성과 지적도를 올려 두었습니다.".to_owned());
    }

    /// C++ `MainWindowSession.cpp:1634-1680` `openProject` — 「열기」.
    /// 파일 대화상자로 조사 파일(.gpkg)을 고르면 실제로 읽어 워크스페이스를 되살린다.
    /// 취소하면 아무 것도 바꾸지 않는다. 견본(idx) 경로는 테스트·시연용으로 남긴다.
    pub fn open_sample_survey_project(&mut self, idx: usize) {
        const SAMPLE_IDX: usize = usize::MAX;
        if idx != SAMPLE_IDX {
            // 견본 열기 (기존 동작 유지)
            self.project_title = "경주시".to_owned();
            self.left_layers
                .ensure_domain_layer(DomainLayerKey::SurveyArea);
            self.left_layers.add_cadastral_wms_layer();
            self.home_map.fit_to_survey();
            self.active_tab = MainTab::Map;
            self.home_map.draw_guide_message =
                Some("다 그렸으면 리본의 「도면」으로 종이에 옮기세요.".to_owned());
            return;
        }
        self.open_survey_file_dialog();
    }

    /// C++ `openProject` 의 파일 선택 + `openSurveyGpkg` 에 해당한다.
    pub fn open_survey_file_dialog(&mut self) {
        // C++ QFileDialog 필터: 조사 (*.gpkg *.qgz *.qgs);; GeoPackage (*.gpkg)
        let picked = ::rfd::FileDialog::new()
            .set_title("열기")
            .add_filter("조사", &["gpkg", "qgz", "qgs"])
            .add_filter("GeoPackage", &["gpkg"])
            .pick_file();
        let Some(path) = picked else {
            // 사용자가 취소했다 — C++ 은 아무 것도 하지 않는다.
            return;
        };
        self.open_survey_gpkg(&path);
    }

    /// C++ `openSurveyGpkg` — 파일을 읽어 조사 제목·좌표계·레이어를 되살린다.
    pub fn open_survey_gpkg(&mut self, path: &std::path::Path) {
        use strata_storage::gpkg::GpkgRepository;

        // C++ 은 .qgz/.qgs 를 고르면 같은 이름의 .gpkg 동반 파일을 먼저 찾는다.
        let mut target = path.to_path_buf();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "gpkg" {
            let companion = path.with_extension("gpkg");
            if companion.exists() {
                target = companion;
            } else {
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」은(는) GeoPackage 조사 파일이 아니고 같은 이름의 .gpkg 도 없습니다.",
                    path.display()
                ));
                return;
            }
        }

        match GpkgRepository::open_read_only(&target) {
            Ok(ws) => {
                // 조사 이름: 파일 이름(확장자 제외)
                let title = target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("조사")
                    .to_owned();
                self.project_title = title.clone();
                self.work_crs = ws.work_crs;

                // 레이어 종류별로 도메인 레이어를 되살린다.
                for key_name in ws.layers.keys() {
                    if let Some(key) = DomainLayerKey::from_id(key_name.as_str()) {
                        self.left_layers.ensure_domain_layer(key);
                    }
                }
                self.left_layers.add_cadastral_wms_layer();
                self.home_map.fit_to_survey();
                self.active_tab = MainTab::Map;
                self.is_dirty = false;
                let recent = target.to_string_lossy().to_string();
                if !self.home_map.recent_files.contains(&recent) {
                    self.home_map.recent_files.insert(0, recent);
                    self.home_map.recent_files.truncate(12);
                }
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」 조사를 열었습니다 ({} · 레이어 {}개).",
                    title,
                    self.work_crs.auth_id(),
                    ws.layers.len()
                ));
            }
            Err(e) => {
                // C++ KaUserError::warn — 오류를 알리고 아무 것도 바꾸지 않는다.
                self.home_map.draw_guide_message =
                    Some(format!("조사 파일을 열 수 없습니다: {e}"));
            }
        }
    }

    pub fn import_demo_dxf_and_csv(&mut self) {
        self.left_layers
            .ensure_domain_layer(DomainLayerKey::ControlPoints);
        self.active_tab = MainTab::Map;
    }
}
