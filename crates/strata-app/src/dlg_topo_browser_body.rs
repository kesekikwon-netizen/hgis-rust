//! 수치지형도 브라우저의 상세/요약 본문 — 진행 표·로그인 폼·다운로드 표·진행 막대.
//!
//! `dlg_topographic_browser.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_topographic_browser` 안의 `if detail { … } else { … }` 블록을 **그대로** 옮겼다.
//! 위젯 순서·문구·열 개수는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

/// 상세(표·로그인·다운로드) 또는 요약(진행 막대) 본문.
pub(crate) fn render_topo_browser_body(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    stage: usize,
    detail: bool,
    stages: &[&str],
) {
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
        format!("{} / 6 — {}", stage + 1, stages[stage]),
        egui::FontId::proportional(12.5),
        Color32::from_rgb(35, 35, 32),
    );
}
}
