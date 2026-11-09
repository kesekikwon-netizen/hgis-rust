#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
pub mod app_state;
pub mod borehole_bridge;
pub mod borehole_card;
pub mod borehole_map_painter;
pub mod cadastral_download_ui;
pub mod debug_control;
pub mod dem_download_ui;
pub mod dialogs_account;
pub mod dialogs_borehole;
pub mod dialogs_borehole_spec;
pub mod dialogs_download_print;
pub mod dialogs_modal;
pub mod gui_layer_panel;
pub mod gui_ribbon_bar;
pub mod gui_shell;
pub mod gui_tab_drawing;
pub mod gui_tab_home;
pub mod gui_tab_map;
pub mod gui_tab_paleo;
pub mod gui_tab_terrain3d;
pub mod gui_theme_fonts;
pub mod left_layer_panel;
pub mod ngii_topographic_ui;
pub mod real_survey;
pub mod ribbon;
pub mod ribbon_icon_svg;
pub mod ribbon_icon_svg_part1;
pub mod ribbon_icon_svg_part2;
pub mod ribbon_icons;
pub mod right_inspector;
pub mod svg_path_paint;
pub mod tab_drawing;
pub mod tabs_home_map;
pub mod tabs_paleo_3d;
pub mod tabs_paleo_3d_views;
pub mod verify_harness;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let app = app_state::StrataAppState::new_startup();
    if args.iter().any(|a| a == "--smoke-quit" || a == "--smoke") {
        println!("{}", app.smoke_summary());
        let (ok, report) = verify_harness::run_cpp_smoke_suite_report();
        println!("{report}");
        if !ok {
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--verify-all") {
        let gates = verify_harness::run_v1_to_v10_verification_harness();
        for g in &gates {
            println!(
                "[{}] {} — {}: {}",
                if g.passed { "PASS" } else { "FAIL" },
                g.gate_id,
                g.title_ko,
                g.detail
            );
        }
        if gates.iter().any(|g| !g.passed) {
            std::process::exit(1);
        }
        return;
    }
    const APP_ICON_RGBA: &[u8] = include_bytes!("../../../data/theme/ka-hgis-app-paper-96.rgba");
    let icon_data = std::sync::Arc::new(eframe::egui::IconData {
        rgba: APP_ICON_RGBA.to_vec(),
        width: 96,
        height: 96,
    });
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Strata")
            .with_icon(icon_data)
            .with_position([16.0, 16.0])
            .with_inner_size([1600.0, 860.0])
            .with_min_inner_size([480.0, 320.0])
            .with_active(true)
            .with_visible(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "Strata",
        options,
        Box::new(|cc| {
            let state = app_state::StrataAppState::new_startup();
            gui_theme_fonts::configure_korean_fonts_and_theme(&cc.egui_ctx, &state.theme);
            Ok(Box::new(state))
        }),
    ) {
        eprintln!("Strata GUI 실행 오류: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
#[path = "tabs_paleo_3d_tests.rs"]
mod tabs_paleo_3d_tests;
