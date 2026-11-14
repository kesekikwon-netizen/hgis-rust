//! Table-driven wheel-zoom modal/non-modal/menu/popup tests (`MAP1e` `b1` / `d3`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, Event, Modifiers, Order, PointerButton, Pos2, Rect};
use strata_net::address_search::LocationHit;
use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::app_state_layer_menu::RenameLayer;
use crate::dialogs_modal::ActiveModalDialog;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::feature_form::tests::table as mock_table;
use crate::feature_form_state::FeatureForm;
use crate::feature_record::FeatureRecordForm;
use crate::ribbon::SubToolsMode;
use crate::survey_area_setup::SurveyAreaSetup;

use super::tests::{new_map_app, run_frame, windows_wheel_notch};

pub(super) const OVER_UNCOVERED_MAP: Pos2 = Pos2::new(840.0, 850.0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Want {
    Zoom,
    Ignore,
}

fn survey_table_with_row() -> SurveyFileTable {
    mock_table(
        &["kind", "period", "feature_no"],
        &[&[("kind", "주거지"), ("period", "청동기"), ("feature_no", "1호")]],
    )
}

pub(super) fn click_at(
    app: &mut crate::app_state::StrataAppState,
    ctx: &egui::Context,
    pos: Pos2,
    button: PointerButton,
) -> Vec<egui::epaint::ClippedShape> {
    let _ = run_frame(app, ctx, vec![Event::PointerMoved(pos)]);
    let _ = run_frame(
        app,
        ctx,
        vec![Event::PointerButton { pos, button, pressed: true, modifiers: Modifiers::NONE }],
    );
    run_frame(
        app,
        ctx,
        vec![Event::PointerButton { pos, button, pressed: false, modifiers: Modifiers::NONE }],
    )
}

fn walk_shapes(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk_shapes(s, f));
    }
}

pub(super) fn find_text_center(shapes: &[egui::epaint::ClippedShape], needle: &str) -> Option<Pos2> {
    for c in shapes {
        let mut hit = None;
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Text(t) = s {
                if t.galley.text() == needle {
                    hit = Some(Rect::from_min_size(t.pos, t.galley.size()).center());
                }
            }
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

fn find_chevron_near(shapes: &[egui::epaint::ClippedShape], x_min: f32, y_min: f32, y_max: f32) -> Pos2 {
    for c in shapes {
        let mut hit = None;
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::LineSegment { points, .. } = s {
                let [a, b] = *points;
                if a.x > x_min && a.y >= y_min && a.y <= y_max && (b.x - a.x - 4.0).abs() < 1.0 && (b.y - a.y - 4.0).abs() < 1.0 {
                    hit = Some(Pos2::new(b.x, (a.y + b.y) * 0.5));
                }
            }
        });
        if let Some(p) = hit {
            return p;
        }
    }
    panic!("missing card choice chevron in x>{x_min}, y in [{y_min}, {y_max}]");
}

fn assert_overlay_visible(ctx: &egui::Context, label: &str) {
    let any_overlay = ctx.memory(|m| {
        m.areas()
            .visible_layer_ids()
            .iter()
            .any(|l| matches!(l.order, Order::Middle | Order::Foreground))
    });
    assert!(any_overlay, "{label}: expected floating window or popup layer to be visible");
}

fn assert_wheel_outcome(app: &mut crate::app_state::StrataAppState, ctx: &egui::Context, label: &str, want: Want) {
    let _ = run_frame(app, ctx, vec![Event::PointerMoved(OVER_UNCOVERED_MAP)]);
    assert_overlay_visible(ctx, label);
    assert_eq!(
        ctx.layer_id_at(OVER_UNCOVERED_MAP).map(|l| l.order),
        Some(Order::Background),
        "{label}: {OVER_UNCOVERED_MAP:?} must sit over uncovered map background"
    );
    app.home_map.viewport.meters_per_pixel = 12.0;
    let center0 = app.home_map.viewport.center_xy;
    let _ = run_frame(
        app,
        ctx,
        vec![Event::PointerMoved(OVER_UNCOVERED_MAP), windows_wheel_notch(1.0, Modifiers::NONE)],
    );
    match want {
        Want::Zoom => assert!(
            (12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9,
            "{label}: expected map wheel to zoom by 1.2 (got mpp={})",
            app.home_map.viewport.meters_per_pixel
        ),
        Want::Ignore => {
            assert_eq!(app.home_map.viewport.meters_per_pixel, 12.0, "{label}: expected map wheel to be ignored");
            assert_eq!(app.home_map.viewport.center_xy, center0, "{label}: center must stay unchanged");
        }
    }
}

#[test]
fn table_driven_wheel_zoom_only_ignores_cpp_modals_menus_and_address_popup() {
    type Setup = fn(&mut crate::app_state::StrataAppState);
    let cases: &[(&str, Want, Setup)] = &[
        // 12 C++ modal ActiveModalDialog variants
        ("NewSurvey", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::NewSurvey)),
        ("SurveyContour", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::SurveyContour)),
        ("AccountSettings", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::AccountSettings)),
        ("ExportSubmitPackage", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::ExportSubmitPackage)),
        ("PaperSetup", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::PaperSetup)),
        ("HeritageRegionSelect", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::HeritageRegionSelect)),
        ("PaleoSourceDownload", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::PaleoSourceDownload)),
        ("PdfPreparing", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::PdfPreparing)),
        ("DrawingPrint", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::DrawingPrint)),
        ("ControlPoint", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::ControlPoint)),
        ("SurveyArea", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::SurveyArea)),
        ("LineColor", Want::Ignore, |a| a.modals.active = Some(ActiveModalDialog::LineColor)),
        // 4 modal_window_open() dialogs
        ("survey_area_setup", Want::Ignore, |a| a.home_map.survey_area_setup = Some(SurveyAreaSetup::default())),
        ("feature_form", Want::Ignore, |a| a.home_map.feature_form = FeatureForm::new(&survey_table_with_row(), 1)),
        ("feature_record", Want::Ignore, |a| {
            a.home_map.feature_record = FeatureRecordForm::new(&survey_table_with_row(), 1, "유구 (Polygon)").ok();
        }),
        ("rename_layer", Want::Ignore, |a| {
            a.home_map.rename_layer = Some(RenameLayer {
                key: "feature_poly".to_owned(),
                text: "유구".to_owned(),
                keys_armed: false,
            });
        }),
        // 3 additional modal prompts
        ("ka_close_prompt", Want::Ignore, |a| a.close_prompt_open = true),
        ("위치 선택 (search_results_modal)", Want::Ignore, |a| {
            a.ribbon.app_bar.search_results_modal = vec![LocationHit {
                title: "경주".to_owned(),
                detail: String::new(),
                lon: 129.2,
                lat: 35.8,
                bbox: None,
                pnu: String::new(),
            }];
        }),
        ("위치 검색 (search_error_modal)", Want::Ignore, |a| {
            a.ribbon.app_bar.search_error_modal = Some("결과 없음".to_owned());
        }),
        // 14 popup menus
        ("ka_paleo_menu", Want::Ignore, |a| a.ribbon.paleo_menu_open = true),
        ("ka_geology_menu", Want::Ignore, |a| a.ribbon.geology_menu_open = true),
        ("ka_dem_menu", Want::Ignore, |a| a.ribbon.dem_menu_open = true),
        ("ka_soil_menu", Want::Ignore, |a| a.ribbon.soil_menu_open = true),
        ("ka_cadastral_menu", Want::Ignore, |a| a.ribbon.cadastral_menu_open = true),
        ("ka_topo_menu", Want::Ignore, |a| a.ribbon.topo_menu_open = true),
        ("ka_old_map_menu", Want::Ignore, |a| a.ribbon.old_map_menu_open = true),
        ("ka_more_menu", Want::Ignore, |a| a.ribbon.more_menu_open = true),
        ("ka_map_context_menu", Want::Ignore, |a| {
            a.map_context_menu_open = true;
            a.map_context_menu_pos = [400.0, 220.0];
        }),
        ("ka_layer_context_menu", Want::Ignore, |a| {
            a.layer_context_menu_open = true;
            a.layer_context_menu_pos = [120.0, 250.0];
        }),
        ("ka_app_bar_region_menu", Want::Ignore, |a| a.ribbon.app_bar.region_popup_open = true),
        ("ka_snap_assist_menu", Want::Ignore, |a| {
            a.ribbon.sub_toolbar_visible = true;
            a.ribbon.sub_mode = SubToolsMode::Draw;
            a.ribbon.snap_assist_menu_open = true;
        }),
        ("ka_theme_menu_popup", Want::Ignore, |a| a.theme_menu_open = true),
        ("ka_status_scale_popup", Want::Ignore, |a| a.scale_menu_open = true),
        // Region address popup
        ("ka_region_address_popup", Want::Ignore, |a| {
            a.ribbon.app_bar.selected_sido = "경상북도".to_owned();
            a.ribbon.app_bar.address_popup_open = true;
        }),
        // 9 C++ non-modal ActiveModalDialog windows -> wheel zooms uncovered map
        ("NgiiTopoDownload (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::NgiiTopoDownload)),
        ("HeritageBrowser (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::HeritageBrowser)),
        ("CadastralDownload (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::CadastralDownload)),
        ("DemDownload (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::DemDownload)),
        ("DemClass (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::DemClass)),
        ("ReferenceStatus (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::ReferenceStatus)),
        ("TrenchGenerator (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::TrenchGenerator)),
        ("TopographicImport (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::TopographicImport)),
        ("CadImportNotice (non-modal)", Want::Zoom, |a| a.modals.active = Some(ActiveModalDialog::CadImportNotice)),
    ];
    for &(label, want, setup) in cases {
        let (mut app, ctx) = new_map_app();
        app.ribbon.context_menu_anchor = Pos2::new(400.0, 120.0);
        setup(&mut app);
        assert_wheel_outcome(&mut app, &ctx, label, want);
    }

    // ka_download_gauge_popup keeps DownloadGuard alive in this scope.
    let (mut app, ctx) = new_map_app();
    let (_id, _guard) = app.download_jobs.add_job(
        "DEM",
        DownloadJobState::Stopped { code: StopCode::None1, reason: "자료 없음".to_owned() },
        1.0,
    );
    app.download_jobs.popup_open = true;
    assert_wheel_outcome(&mut app, &ctx, "ka_download_gauge_popup", Want::Zoom);

    // Memory-only popup: ka_draw_combo_popup (d2) does not block map wheel zoom.
    let (mut app, ctx) = new_map_app();
    app.ribbon.sub_toolbar_visible = true;
    app.ribbon.sub_mode = SubToolsMode::Draw;
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let combo_pos = find_text_center(&shapes, "모든 조사 레이어").expect("draw snap combo");
    let _ = click_at(&mut app, &ctx, combo_pos, PointerButton::Primary);
    assert!(ctx.memory(|m| m.any_popup_open()), "ka_draw_combo_popup must be open in egui memory");
    assert_wheel_outcome(&mut app, &ctx, "ka_draw_combo_popup", Want::Zoom);

    // Memory-only popup: ka_card_choice (d2) does not block map wheel zoom.
    let (mut app, ctx) = new_map_app();
    app.survey_tables.push(survey_table_with_row());
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), 1));
    app.sync_card_with_selection();
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let kind_y = find_text_center(&shapes, "종류").expect("card kind label").y;
    let chevron_pos = find_chevron_near(&shapes, 1440.0, kind_y - 20.0, kind_y + 20.0);
    let _ = click_at(&mut app, &ctx, chevron_pos, PointerButton::Primary);
    assert!(ctx.memory(|m| m.any_popup_open()), "ka_card_choice popup must be open in egui memory");
    assert_wheel_outcome(&mut app, &ctx, "ka_card_choice", Want::Zoom);
}
