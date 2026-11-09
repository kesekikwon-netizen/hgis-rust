//! 수치지형도 가져오기 브라우저 (`KaTopographicBrowser.cpp:117-260`).
//!
//! `dialogs_download_print.rs` 가 300줄을 넘어 이 파일로 옮겼다.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_download_print::render_ka_download_header;
use crate::dialogs_modal::ActiveModalDialog;
use crate::dialogs_modal::render_strata_modal_header;

// C++ KaTopographicBrowser.cpp:117-260 — "수치지형도 내려받기"
// 6-step KaStageRail, compact/expanded toggle, NGII login form, downloads table.
const TOPO_STAGES: [&str; 6] = [
    "도엽 확인",
    "로그인",
    "신청",
    "파일 준비",
    "다운로드",
    "지도에 올리기",
];

pub(crate) fn render_topographic_browser(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let is_open = app.modals.active == Some(ActiveModalDialog::NgiiTopoDownload);
    if !is_open {
        return;
    }
    let stage = app.modals.topo_browser_stage.min(5);
    let detail = app.modals.topo_browser_detail;

    egui::Window::new("ka_modal_topo_browser")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(620.0)
        .show(ctx, |ui| {
            // KaTopographicBrowser.cpp:120 setWindowTitle("수치지형도 내려받기")
            render_strata_modal_header(ui, "수치지형도 내려받기 - Strata", open);
            // KaTopographicBrowser.cpp:126-127
            render_ka_download_header(
                ui,
                "수치지형도 다운로드",
                "국토정보플랫폼 · 조사 주변 도엽을 확인하고 지도 자료를 받습니다.",
                588.0,
            );

            ui.add_space(8.0);
            // KaStageRail.cpp — 6단계 원형 배지 + 연결선
            // (완료 ●테라코타, 현재 ◉강조 테두리, 대기 ○회색)
            {
                let (rail_r, _) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width().min(560.0), 30.0),
                    egui::Sense::hover(),
                );
                let rp = ui.painter_at(rail_r);
                let n = TOPO_STAGES.len();
                let step = rail_r.width() / n as f32;
                for i in 0..n {
                    let cx = rail_r.left() + step * (i as f32 + 0.5);
                    let cy = rail_r.top() + 9.0;
                    let center = egui::Pos2::new(cx, cy);
                    // 연결선
                    if i + 1 < n {
                        rp.line_segment(
                            [center, egui::Pos2::new(cx + step, cy)],
                            egui::Stroke::new(
                                1.5,
                                if i < stage {
                                    Color32::from_rgb(181, 87, 58)
                                } else {
                                    Color32::from_rgb(210, 207, 198)
                                },
                            ),
                        );
                    }
                    if i < stage {
                        // 완료 — 테라코타 채움
                        rp.circle_filled(center, 6.0, Color32::from_rgb(181, 87, 58));
                        rp.text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "✓",
                            egui::FontId::proportional(8.5),
                            Color32::WHITE,
                        );
                    } else if i == stage {
                        // 현재 — 강조 테두리
                        rp.circle_filled(center, 7.5, Color32::WHITE);
                        rp.circle_stroke(
                            center,
                            7.5,
                            egui::Stroke::new(2.2, Color32::from_rgb(181, 87, 58)),
                        );
                        rp.circle_filled(center, 3.4, Color32::from_rgb(181, 87, 58));
                    } else {
                        // 대기 — 회색
                        rp.circle_filled(center, 6.0, Color32::from_rgb(238, 236, 228));
                        rp.circle_stroke(
                            center,
                            6.0,
                            egui::Stroke::new(1.0, Color32::from_rgb(190, 187, 178)),
                        );
                    }
                    rp.text(
                        egui::Pos2::new(cx, rail_r.top() + 23.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", i + 1),
                        egui::FontId::proportional(9.0),
                        if i <= stage {
                            Color32::from_rgb(45, 44, 40)
                        } else {
                            Color32::from_rgb(150, 148, 140)
                        },
                    );
                }
            }
            ui.label(
                egui::RichText::new(TOPO_STAGES.join(" → "))
                    .size(10.5)
                    .color(Color32::from_rgb(120, 118, 110)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "[{}/6] {}",
                    stage + 1,
                    TOPO_STAGES[stage]
                ))
                .size(14.5)
                .strong(),
            );

            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(556.0);
                    // KaTopographicBrowser.cpp:152 hint
                    ui.label(
                        egui::RichText::new(
                            "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요.",
                        )
                        .size(12.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.add_space(6.0);
                    // 1단계 「도엽 확인」은 로그인 없이 실제 HTTP 로 된다
                    // (`ngii-topographic.js:43,63` → `combineBufferQuery.do` → `suchiQuery.do`).
                    crate::ngii_topographic_ui::render_sheet_lookup(app, ui, ctx);
                    ui.add_space(6.0);
                    // 진행 로그
                    for line in app.modals.topo_browser_log.iter().rev().take(3) {
                        ui.label(
                            egui::RichText::new(format!("· {line}"))
                                .size(11.5)
                                .color(Color32::from_rgb(110, 108, 102)),
                        );
                    }
                });

            if detail {
                ui.add_space(10.0);
                // KaTopographicBrowser.cpp: expanded toolbar (6 buttons)
                ui.horizontal_wrapped(|ui| {
                    for label in [
                        "뒤로",
                        "앞으로",
                        "새로고침",
                        "국토정보맵",
                        "받은 폴더 열기",
                        "받은 자료 불러오기…",
                    ] {
                        let b = egui::Button::new(egui::RichText::new(label).size(12.0))
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(160, 158, 150)))
                            .corner_radius(4.0);
                        if ui.add(b).clicked() {
                            if label == "받은 자료 불러오기…" {
                                // KaTopographicBrowser.cpp:215 fallback -> import dialog
                                app.modals.active =
                                    Some(ActiveModalDialog::TopographicImport);
                            } else {
                                app.modals
                                    .topo_browser_log
                                    .push(format!("{label} — 공식 화면에서 직접 진행합니다."));
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                // KaTopographicBrowser.cpp:155-170 login row + form row
                egui::Grid::new("ka_topo_login_grid")
                    .num_columns(3)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_login_id,
                            )
                            .hint_text("국토지리정보원 아이디")
                            .desired_width(200.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_login_pw,
                            )
                            .hint_text("비밀번호 — 이번 로그인에만 사용")
                            .password(true)
                            .desired_width(220.0),
                        );
                        if ui.button("로그인 다시 시도").clicked() {
                            app.modals.topo_browser_stage = 1;
                            app.modals.topo_browser_log.push(
                                "로그인 다시 시도 — 계정 설정과 연결 상태를 확인하세요."
                                    .to_owned(),
                            );
                        }
                        ui.end_row();

                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_birth_date,
                            )
                            .hint_text("생년월일 (공식 신청서 형식)")
                            .desired_width(200.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(
                                &mut app.modals.topo_browser_purpose,
                            )
                            .desired_width(220.0),
                        );
                        if ui
                            .button(if app.modals.topo_browser_auto_stopped {
                                "자동 진행 재개"
                            } else {
                                "자동 진행 중지"
                            })
                            .clicked()
                        {
                            app.modals.topo_browser_auto_stopped =
                                !app.modals.topo_browser_auto_stopped;
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);
                // KaTopographicBrowser.cpp: downloads table, 4 columns
                egui::Grid::new("ka_topo_downloads")
                    .num_columns(4)
                    .striped(true)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for h in ["파일", "진행", "상태", "취소"] {
                            ui.label(
                                egui::RichText::new(h)
                                    .size(11.5)
                                    .color(Color32::from_rgb(95, 94, 88)),
                            );
                        }
                        ui.end_row();

                        let rows = [
                            ("37704055.dxf", "100%", "받음"),
                            ("37704056.dxf", "62%", "받는 중"),
                            ("37704057.dxf", "0%", "대기"),
                        ];
                        for (file, prog, state) in rows {
                            ui.label(egui::RichText::new(file).size(11.5));
                            ui.label(egui::RichText::new(prog).size(11.5));
                            ui.label(egui::RichText::new(state).size(11.5).color(
                                if state == "받음" {
                                    Color32::from_rgb(21, 128, 61)
                                } else {
                                    Color32::from_rgb(105, 103, 96)
                                },
                            ));
                            let _ = ui.small_button("취소");
                            ui.end_row();
                        }
                    });
            } else {
                // KaTopographicBrowser.cpp compact: progress bar only
                ui.add_space(8.0);
                let (bar_r, _) =
                    ui.allocate_exact_size(Vec2::new(588.0, 26.0), egui::Sense::hover());
                let p = ui.painter_at(bar_r);
                p.rect_filled(bar_r, 4.0, Color32::WHITE);
                p.rect_stroke(
                    bar_r,
                    4.0,
                    Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                    egui::StrokeKind::Middle,
                );
                let frac = (stage as f32 + 1.0) / 6.0;
                p.rect_filled(
                    Rect::from_min_size(
                        bar_r.min + Vec2::new(1.0, 1.0),
                        Vec2::new((bar_r.width() - 2.0) * frac, 24.0),
                    ),
                    3.0,
                    Color32::from_rgb(181, 87, 58),
                );
                p.text(
                    bar_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} / 6 — {}", stage + 1, TOPO_STAGES[stage]),
                    egui::FontId::proportional(12.5),
                    Color32::from_rgb(35, 35, 32),
                );
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                // KaTopographicBrowser.cpp:206 expand toggle
                let toggle_label = if detail { "상세 접기" } else { "상세 보기" };
                let tb = egui::Button::new(
                    egui::RichText::new(toggle_label)
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(96.0, 32.0));
                if ui.add(tb).clicked() {
                    app.modals.topo_browser_detail = !detail;
                }
                let step_b = egui::Button::new(egui::RichText::new("다음 단계").size(13.0))
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(96.0, 32.0));
                if ui.add(step_b).clicked() {
                    let next = (stage + 1).min(5);
                    app.modals.topo_browser_stage = next;
                    app.modals
                        .topo_browser_log
                        .push(format!("단계 진행 — {}", TOPO_STAGES[next]));
                    if next == 5 {
                        app.modals.active = Some(ActiveModalDialog::TopographicImport);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cb = egui::Button::new(egui::RichText::new("취소").size(13.0))
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(145, 142, 134)))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(62.0, 32.0));
                    if ui.add(cb).clicked() {
                        app.modals.active = None;
                    }
                });
            });
        });
}
