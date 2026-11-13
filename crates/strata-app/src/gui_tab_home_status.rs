//! 「홈」 탭 — 연결 상태(인터넷·키·계정) 다섯 칸.
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 칸의 글은 `home_connection_model`(C++
//! KaHomeConnectionCard), 넣어져 있는지는 저장소에서 센 `accounts.status`(C++ AccountStatus).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::home_connection_model::{conn_rows, internet_label, InternetCheck, INTERNET_BUTTON_TIP};

/// 종이 모양 토큰(KaThemePaper.cpp:27-29) — 닿음 ok, 안 닿음 danger, 칸 점 borderStrong.
const OK: Color32 = Color32::from_rgb(0x3F, 0x6B, 0x31);
const DANGER: Color32 = Color32::from_rgb(0xA3, 0x3B, 0x3B);
const BORDER_STRONG: Color32 = Color32::from_rgb(0xC2, 0xC0, 0xB6);
/// 없는 설정 앞의 점(KaHomeConnectionCard.cpp:150).
const MISSING_DOT: Color32 = Color32::from_rgb(0xD9, 0x77, 0x57);
const MUTED: Color32 = Color32::from_rgb(105, 103, 96);
const INK: Color32 = Color32::from_rgb(35, 35, 32);

impl StrataAppState {
    /// 연결 상태 줄 — 인터넷 확인과 VWorld·국토정보플랫폼·국가유산청 계정.
    pub(crate) fn render_home_connection_status(&mut self, ui: &mut egui::Ui) {
                if self.home_map.internet.poll() || matches!(self.home_map.internet, InternetCheck::Checking(_)) {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("연결 상태").size(14.5).strong());
                    let (lock_rect, _) =
                        ui.allocate_exact_size(Vec2::new(10.0, 13.0), egui::Sense::hover());
                    {
                        let lp = ui.painter_at(lock_rect);
                        let body = Rect::from_min_max(
                            Pos2::new(lock_rect.left() + 1.5, lock_rect.top() + 5.5),
                            Pos2::new(lock_rect.right() - 1.5, lock_rect.bottom() - 1.0),
                        );
                        lp.rect_stroke(body, 1.5, Stroke::new(1.1, MUTED), egui::StrokeKind::Inside);
                        lp.circle_stroke(
                            Pos2::new(lock_rect.center().x, lock_rect.top() + 4.5),
                            2.3,
                            Stroke::new(1.1, MUTED),
                        );
                    }
                    ui.label(egui::RichText::new("비밀값은 표시되지 않습니다").size(12.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // C++ KaHomeConnectionCard.cpp:66-72 — plain white button with a 1px rule.
                        let net = internet_label(&self.home_map.internet);
                        let net_btn = egui::Button::new(egui::RichText::new("인터넷 확인").size(12.0))
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(120, 118, 110)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(0.0, 26.0));
                        if ui.add_enabled(net.button_enabled, net_btn).on_hover_text(INTERNET_BUTTON_TIP).clicked() {
                            self.home_map.internet = InternetCheck::start();
                        }
                        let text = ui.label(egui::RichText::new(net.text).size(12.0).color(Color32::from_rgb(95, 94, 88)));
                        let dot = net.dot.map(|reached| ui.label(egui::RichText::new("●").size(12.0).color(if reached { OK } else { DANGER })));
                        if let Some(tip) = net.tip {
                            text.on_hover_text(tip);
                            if let Some(dot) = dot {
                                dot.on_hover_text(tip);
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                let rows = conn_rows(&self.accounts.status, self.paper_look);
                let sep_rgb = Color32::from_rgb(212, 209, 200);
                let ink_rgb = Color32::from_rgb(55, 54, 50);
                let cols_rect = ui.available_rect_before_wrap();
                ui.columns(rows.len().max(1), |cols| {
                    for (idx, row) in rows.iter().enumerate() {
                        // 1px vertical rule between adjacent account columns (C++ draws it
                        // before every column except the first).
                        if idx > 0 {
                            let x = cols[idx].cursor().left() - 6.0;
                            let top = cols_rect.top();
                            cols[idx].painter().line_segment(
                                [Pos2::new(x, top), Pos2::new(x, top + 44.0)],
                                Stroke::new(1.0, sep_rgb),
                            );
                        }
                        cols[idx].horizontal(|ui| {
                            if !self.paper_look {
                                // :137-140 — the stock look keeps an 8 px dot (ok / borderStrong).
                                let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_r.center(), 3.5, if row.ready { OK } else { BORDER_STRONG });
                            }
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(row.name).size(12.5).strong()).on_hover_text(&row.name_tip);
                                let state = ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    if row.missing_dot {
                                        ui.label(egui::RichText::new("●").size(11.5).color(MISSING_DOT));
                                    }
                                    ui.label(egui::RichText::new(row.state).size(11.5).color(if row.missing_dot { INK } else { MUTED }));
                                });
                                if let Some(tip) = &row.state_tip {
                                    state.response.on_hover_text(tip);
                                }
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // C++ uses dark ink text, not a blue hyperlink.
                                let resp = ui
                                    .add(egui::Label::new(egui::RichText::new("설정").size(12.0).color(ink_rgb)).sense(egui::Sense::click()))
                                    .on_hover_text(row.setup_tip);
                                if resp.clicked() {
                                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                                }
                            });
                        });
                    }
                });
    }
}
