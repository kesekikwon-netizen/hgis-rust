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
mod gui_tab_map_basemap;
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
pub mod svg_curve_flatten;
pub mod svg_path_paint;
pub mod svg_path_tokenize;
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
    if args.iter().any(|a| a == "--diag-layout") {
        diag_layout::run();
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
    // --snap <폴더> --snap-steps 액션:이름,액션:이름 이면 스스로 캡처하고 끝난다.
    let snap_dir = args
        .iter()
        .position(|a| a == "--snap")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let snap_steps = args
        .iter()
        .position(|a| a == "--snap-steps")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default();

    const APP_ICON_RGBA: &[u8] = include_bytes!("../../../data/theme/ka-hgis-app-paper-96.rgba");
    let icon_data = std::sync::Arc::new(eframe::egui::IconData {
        rgba: APP_ICON_RGBA.to_vec(),
        width: 96,
        height: 96,
    });
    // C++ `MainWindow.cpp:243,277` — `resize(1280, 900)` 뒤 `showMaximized()`.
    // 화면 배율이 200% 인 노트북에서는 논리 1600×860 이 실제 3200×1720 픽셀이 되어
    // 화면(1920×1080)을 넘고, 리본 오른쪽 절반이 화면 밖으로 나간다.
    // 최대화로 시작해 화면 안에 항상 들어오게 한다.
    let start_maximized = std::env::var("KA_HGIS_WINDOWED").is_err();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Strata")
        .with_icon(icon_data)
        .with_position([16.0, 16.0])
        .with_inner_size([1280.0, 900.0])
        .with_min_inner_size([960.0, 600.0])
        .with_active(true)
        .with_visible(true);
    if start_maximized {
        viewport = viewport.with_maximized(true);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "Strata",
        options,
        Box::new(|cc| {
            let mut state = app_state::StrataAppState::new_startup();
            gui_theme_fonts::configure_korean_fonts_and_theme(&cc.egui_ctx, &state.theme);
            if let Some(dir) = snap_dir.clone() {
                state.snap = Some(snap_shot::SnapRequest::parse(&dir, &snap_steps));
            }
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

#[cfg(test)]
#[path = "svg_path_paint_tests.rs"]
mod svg_path_paint_tests;

mod basemap_cadastral;
mod basemap_cadastral_jobs;
mod basemap_fetch;
mod basemap_mesh;
mod basemap_paint;
mod basemap_tiles;
mod diag_layout;
mod snap_shot;
mod snap_shot_run;
mod dlg_cad_import_notice;
mod dlg_cadastral_download;
mod dlg_drawing_print;
mod dlg_geotiff_limits;
mod dlg_heritage_browser;
mod dlg_heritage_region_select;
mod dlg_new_survey;
mod dlg_ngii_topo_download;
mod dlg_pdf_preparing;
mod dlg_survey_contour;
mod dlg_topographic_browser;
mod dlg_topographic_import;
mod dlg_trench_generator;

#[cfg(test)]
#[path = "dialogs_download_print_tests.rs"]
mod dialogs_download_print_tests;

mod modal_control;
mod modal_downloads;
mod modal_export;
mod modal_export_footer;
mod modal_export_table;
mod modal_paper_account;
mod modal_reference;
mod modal_reference_data;
mod modal_style;
mod modal_style_demclass;
mod modal_survey;
mod modal_types;

#[cfg(test)]
#[path = "dialogs_modal_tests.rs"]
mod dialogs_modal_tests;
#[cfg(test)]
#[path = "dialogs_modal_tests_b.rs"]
mod dialogs_modal_tests_b;

mod modal_controller_state;
mod modal_controller_impl;
mod modal_topo_rows;

#[cfg(test)]
#[path = "dialogs_modal_tests_c.rs"]
mod dialogs_modal_tests_c;

mod shell_workspace;
mod shell_dialogs;

#[cfg(test)]
#[path = "gui_shell_tests_a.rs"]
mod gui_shell_tests_a;
#[cfg(test)]
#[path = "gui_shell_tests_b.rs"]
mod gui_shell_tests_b;

mod ri_body;

mod ri_body_bottom;

mod ri_body_cards;

mod ri_attr_card;
mod ri_layer_card;

mod ri_types;
#[cfg(test)]
#[path = "ri_tests.rs"]
mod ri_tests;

#[cfg(test)]
#[path = "ui_interaction_tests.rs"]
mod ui_interaction_tests;
#[cfg(test)]
#[path = "basemap_tests.rs"]
mod basemap_tests;
#[cfg(test)]
#[path = "basemap_mesh_tests.rs"]
mod basemap_mesh_tests;
