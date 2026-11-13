//! 「시굴격자 속성」 대화상자 — 요약·목표 눈금 막대·배치 후 편집 안내.
//!
//! `dlg_trench_generator.rs` 가 300줄 규칙에 맞춰 나눈 파일이다 (`KaTrenchDialogPlan.cpp:110-120`).
//! 원래 창 안의 가운데 부분을 **그대로** 옮겼고, 순서·수치·문구는 하나도 바꾸지 않았다.
//! `manual`·`kind` 는 앞부분(`render_trench_controls`)이 정한 값을 그대로 받는다.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use super::dlg_trench_generator_controls::KIND_LABELS;

/// 요약 상자와 목표 막대를 그린다. `manual`·`kind` 는 앞부분이 돌려준 값 그대로다.
pub(crate) fn render_trench_summary(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    manual: bool,
    kind: usize,
) {
ui.add_space(6.0);
// KaTrenchDialogPlan.cpp:110-120 요약 + KaTargetMeter.cpp:36-59
if manual {
    let count = app.modals.trench_rows * app.modals.trench_cols;
    let summary = format!(
        "수동 배치: {count}개 · 총 {:.0}㎡ — 「맵에 찍기」로 맵에서 위치를 정하세요.",
        count as f64 * 40.0
    );
    egui::Frame::NONE
        .fill(Color32::from_rgb(225, 240, 218))
        .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(summary)
                    .size(12.5)
                    .strong()
                    .color(Color32::from_rgb(28, 72, 32)),
            );
        });
} else {
    let trench_count = app.home_map.real_survey.trenches.len().max(1);
    let total_trench_area: f64 =
        app.home_map.real_survey.trenches.iter().map(|t| t.area_m2).sum();
    // pickAutoFillArea — 고른(없으면 fid 가 가장 큰) 조사구역의 넓이, 열어 둔 조사 표에서.
    let survey_area: f64 = app.trench_survey_area_m2().max(1.0);
    let ratio_pct =
        (total_trench_area / survey_area * 100.0).clamp(0.0, 100.0);
    let shape_desc = if app.modals.trench_shape.arm_bend().is_some() {
        format!("{} {}개", app.modals.trench_shape.label_ko(), trench_count)
    } else {
        format!(
            "{} 트렌치 {}개",
            app.modals.trench_length.label_ko(),
            trench_count
        )
    };
    let target = if kind == 1 { 2.0_f64 } else { 10.0_f64 };
    let summary_text = format!(
        "{} · {shape_desc} · 폭 2 m · 둑 {:.0} m ·\n총 {total_trench_area:.2}㎡ · 비율 {ratio_pct:.1}% (목표 {target:.0}%) · 방위 {:.0}°",
        KIND_LABELS[kind],
        app.modals.trench_balk_m,
        app.modals.trench_azimuth_deg
    );
    egui::Frame::NONE
        .fill(Color32::from_rgb(225, 240, 218))
        .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(summary_text)
                    .size(12.5)
                    .strong()
                    .color(Color32::from_rgb(28, 72, 32)),
            );
        });
    // 목표 눈금 막대: 0 .. target*2
    ui.add_space(4.0);
    let (bar_r, _) = ui
        .allocate_exact_size(Vec2::new(340.0, 6.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bar_r, 3.0, Color32::from_rgb(222, 220, 210));
    let top = target * 2.0;
    let fill_w = (340.0_f32 * (ratio_pct as f32 / top as f32))
        .clamp(12.0_f32, 340.0_f32);
    ui.painter().rect_filled(
        Rect::from_min_size(bar_r.min, Vec2::new(fill_w, 6.0)),
        3.0,
        Color32::from_rgb(35, 35, 32),
    );
    ui.label(
        egui::RichText::new(format!("목표 {target:.0}%"))
            .size(11.5)
            .color(Color32::from_rgb(95, 94, 88)),
    );
    // KaTrenchDialog.cpp:114-135 — m_afterPlace (배치 후 편집)
    if !app.home_map.real_survey.trenches.is_empty() {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "깐 격자는 맵에서 끌어 옮깁니다. 개별 편집은 트렌치 하나 이동·삭제입니다.",
            )
            .size(11.5)
            .color(Color32::from_rgb(95, 94, 88)),
        );
        ui.horizontal(|ui| {
            if ui
                .button("개별 편집")
                .on_hover_text(
                    "그래픽처럼 트렌치를 하나씩 선택·이동·삭제합니다.",
                )
                .clicked()
            {
                app.home_map.draw_guide_message = Some(
                    "시굴격자 개별 편집 모드 — 트렌치를 하나씩 선택해 이동하거나 삭제하세요."
                        .to_owned(),
                );
            }
            if ui
                .button("전체 이동")
                .on_hover_text(
                    "격자를 끌어다 놓아 전체를 옮깁니다. Ctrl+Z로 되돌립니다.",
                )
                .clicked()
            {
                app.home_map.draw_guide_message = Some(
                    "시굴격자 전체 이동 모드 — 격자를 끌어 전체 위치를 옮기세요."
                        .to_owned(),
                );
            }
        });
    }
}
}
