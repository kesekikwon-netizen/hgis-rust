//! 「주변유적 받기」 진행 상태 카드 — 단계 이름·진행률 막대·서약서 안내.
//!
//! `dlg_heritage_browser.rs` 가 300줄 규칙에 맞춰 나눈 파일이다 (`KaHeritageBrowser.cpp:211-229`).
//! 원래 창 안의 진행 카드 블록을 **그대로** 옮겼고, 순서·수치·문구는 하나도 바꾸지 않았다.
//! `stage_enum` 은 이 블록 안에서만 쓰이므로 함께 옮겼다.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

/// 진행 단계 카드. `modal_w` 는 창 폭이다(막대 폭 계산에 쓴다).
pub(crate) fn render_heritage_progress(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    modal_w: f32,
) {
ui.add_space(6.0);
// 4) 진행 상태 및 진행률 카드 (KaHeritageBrowser.cpp:211-229)
let stage_enum = match app.modals.heritage_stage_idx {
    0 => strata_net::heritage_intranet::HeritageStage::Idle,
    1 => strata_net::heritage_intranet::HeritageStage::Login,
    2 => strata_net::heritage_intranet::HeritageStage::DismissTutorial,
    3 => strata_net::heritage_intranet::HeritageStage::OpenDownloadPage,
    4 => strata_net::heritage_intranet::HeritageStage::AgreeTerms,
    5 => strata_net::heritage_intranet::HeritageStage::SelectDataset,
    6 => strata_net::heritage_intranet::HeritageStage::SelectRegion,
    7 => strata_net::heritage_intranet::HeritageStage::Search,
    8 => strata_net::heritage_intranet::HeritageStage::CollectResults,
    9 => strata_net::heritage_intranet::HeritageStage::Download,
    11 => strata_net::heritage_intranet::HeritageStage::Failed,
    _ => strata_net::heritage_intranet::HeritageStage::Done,
};
egui::Frame::NONE
    .fill(Color32::WHITE)
    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
    .corner_radius(6.0)
    .inner_margin(egui::Margin::symmetric(12, 8))
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "단계: {}",
                    stage_enum.stage_name_ko()
                ))
                .size(13.0)
                .strong(),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(&app.modals.heritage_status_detail)
                    .size(12.0)
                    .color(Color32::from_rgb(55, 54, 50)),
            );
        });
        ui.add_space(4.0);
        let (bar_r, _) = ui.allocate_exact_size(
            Vec2::new(modal_w - 52.0, 20.0),
            egui::Sense::hover(),
        );
        let p = ui.painter_at(bar_r);
        p.rect_filled(bar_r, 4.0, Color32::from_rgb(238, 236, 228));
        let fill_w = (bar_r.width() * app.modals.heritage_progress_pct.clamp(0.0, 1.0)).max(4.0);
        p.rect_filled(
            Rect::from_min_size(bar_r.min, Vec2::new(fill_w, bar_r.height())),
            4.0,
            Color32::from_rgb(181, 87, 58),
        );
        p.rect_stroke(
            bar_r,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
            egui::StrokeKind::Inside,
        );
        p.text(
            bar_r.center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "{:.0}% · {} {}",
                app.modals.heritage_progress_pct * 100.0,
                app.modals.heritage_sido,
                app.modals.heritage_city
            ),
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );
        ui.add_space(3.0);
        // HeritagePledge::preDisclosure()
        ui.label(
            egui::RichText::new(
                "받기를 누르면 국가유산 인트라넷의 「원본자료 사용 서약서」에 자동으로 동의하고, 동의 시각과 서약서 원문을 조사폴더 주변유적/receipts 에 남깁니다. 받은 자료는 동국문화재연구원 소유입니다.",
            )
            .size(11.0)
            .color(Color32::from_rgb(95, 94, 88)),
        );
    });
}
