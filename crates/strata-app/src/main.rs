pub mod app_state;
pub mod debug_control;
pub mod dialogs_account;
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
pub mod ribbon;
pub mod ribbon_icons;
pub mod right_inspector;
pub mod tab_drawing;
pub mod tabs_home_map;
pub mod tabs_paleo_3d;
pub mod verify_harness;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let app = app_state::StrataAppState::new_startup();
    if args.iter().any(|a| a == "--smoke-quit") {
        println!("{}", app.smoke_summary());
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
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Strata")
            .with_position([40.0, 16.0])
            .with_inner_size([1620.0, 860.0])
            .with_min_inner_size([1120.0, 720.0])
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
