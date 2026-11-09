//! 「홈」 탭 상단 — 브랜드 히어로(새 조사·조사 열기)와 이어서 작업 표.
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_home_tab_ui` 안의 첫 블록과 `paint_home_title_block` 을 그대로 옮겼고,
//! 순서·수치·문구는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon_icons::paint_strata_logo;

impl StrataAppState {
    /// 브랜드 히어로 줄 — 새 조사 / 조사 열기와 이어서 작업 표.
    pub(crate) fn render_home_hero(&mut self, ui: &mut egui::Ui, inner_w: f32) {
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
    }

    /// 이어서 작업 표(제목 줄·좌표계·조사 건수).
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
}
