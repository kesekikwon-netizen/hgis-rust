//! 「홈」 탭 — 연결 상태(인터넷·키·계정) 다섯 칸.
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_home_tab_ui` 안의 셋째 블록을 그대로 옮겼고, 순서·수치·문구는 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 연결 상태 줄 — 인터넷 확인과 VWorld·국토정보플랫폼·국가유산청 계정.
    pub(crate) fn render_home_connection_status(&mut self, ui: &mut egui::Ui) {
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
    }
}
