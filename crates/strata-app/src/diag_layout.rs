//! 리본·화면 배치 진단 — 실제 계산값을 그대로 찍는다.
//! `ka-hgis --diag-layout` 으로 실행.

use eframe::egui;

use crate::gui_ribbon_bar::{
    ribbon_chip_base_width, ribbon_group_layout, UX_RIBBON_GROUPS, UX_RIBBON_HEIGHT,
    RIBBON_CHIP_SCALE,
};

pub fn run() {
    println!("=== 리본 그룹 폭 계산 (h_scale 적용 전) ===");
    let (xs, ws) = ribbon_group_layout();
    let mut total = 0.0_f32;
    for (i, (title, items)) in UX_RIBBON_GROUPS.iter().enumerate() {
        let sum: f32 = items
            .iter()
            .map(|l| ribbon_chip_base_width(i, l) * RIBBON_CHIP_SCALE)
            .sum();
        println!(
            "  {:<10} x={:7.1} w={:6.1}  (칩 {}개, 합계검산 {:.1})",
            title,
            xs[i],
            ws[i],
            items.len(),
            sum
        );
        total = xs[i] + ws[i];
    }
    println!("  → 마지막 그룹 끝: {total:.1} px (h_scale 전)");
    println!("  → RIBBON_CHIP_SCALE = {RIBBON_CHIP_SCALE:.4}");

    println!();
    println!("=== 실제 화면 기준 (egui 가 보는 값) ===");
    let ctx = egui::Context::default();
    let mut printed = false;
    let mut raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(1600.0, 860.0),
        )),
        ..Default::default()
    };
    let mut frame_count = 0;
    while frame_count < 4 && !printed {
        frame_count += 1;
        let _ = ctx.run(raw.clone(), |ctx| {
            if printed {
                return;
            }
            let full = ctx.screen_rect();
            let avail_for_ribbon = (full.width() - 266.0).max(640.0);
            let h_scale = (avail_for_ribbon / 1328.0).clamp(0.72, 1.38);
            println!("  screen_rect       = {:.1} x {:.1}", full.width(), full.height());
            println!("  pixels_per_point  = {:.4}", ctx.pixels_per_point());
            println!("  avail_for_ribbon  = {:.1}", avail_for_ribbon);
            println!("  h_scale           = {:.4}", h_scale);
            println!("  리본 필요 폭(h_scale 후) = {:.1}", total * h_scale);
            println!(
                "  → 화면 폭 대비 {}",
                if total * h_scale > full.width() {
                    format!("초과 {:.1}px — 잘린다!", total * h_scale - full.width())
                } else {
                    format!("여유 {:.1}px", full.width() - total * h_scale)
                }
            );
            println!("  UX_RIBBON_HEIGHT  = {UX_RIBBON_HEIGHT}");
            printed = true;
        });
        raw.screen_rect = None;
    }
}
