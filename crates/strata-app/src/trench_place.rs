//! 「구역에 깔기」 writes `trial_trench` features (`target/cpp-ref/src/app/MainWindow.cpp:1920-2152`,
//! `target/cpp-ref/src/core/TrenchLayerEdit.cpp:101-227`).
//!
//! Coordinates stay in the survey-area layer CRS (EPSG:5186 or EPSG:5187 metres, x then y).
//! Nothing here projects them (`MainWindow.cpp:2114-2119`).

use std::collections::BTreeMap;

use strata_geo::trench_plan::{
    build, cells_valid, meets_target, pick_auto_fill_area, total_area, Cell, PickedArea, Spec,
    SurveyPoly,
};
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::app_state_layer_remove::UndoStep;

#[path = "trench_place_geom.rs"]
mod geom;
#[path = "trench_place_job.rs"]
mod job;
use geom::{auth_of, cell_feature, cells_center, extent_center, geometry_from_wkt, same_layout};
pub(crate) use job::{poll_trench_place, TrenchPlaceJob};

const KEEP_EXISTING: &str = "기존 시굴격자를 그대로 두었습니다.";
const NO_TARGET: &str = "목표 면적에 맞는 격자를 계산하지 못했습니다. 방향과 규격을 확인해 다시 적용하세요. 기존 격자는 유지됩니다.";
const NO_LAYER: &str = "시굴격자 레이어를 찾지 못했습니다. 조사를 다시 열어 확인하세요.";
const ALREADY: &str = "같은 시굴격자가 이미 깔려 있습니다. 옮기거나 지우려면 「개별 편집」이나 「전체 이동」을 누르세요.";
const L_SHAPE: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 WU-04)";

/// Button 「구역에 깔기」 (`MainWindow.cpp:1920`). An empty plan does not report success.
pub(crate) fn place_trenches_in_area(app: &mut StrataAppState) {
    if app.survey_path.is_none() {
        app.show_status_message("먼저 「새 조사」를 만들거나 「열기」로 조사를 여세요.");
        return;
    }
    // C++ lays ㄱ·ㄴ자 through `armLength` (`TrenchGridGenerator.h:119-122`). That search is not in
    // `build_for_target_ratio`, so this button does not pretend a straight grid is an L.
    if app.modals.trench_shape.arm_bend().is_some() {
        app.show_status_message(L_SHAPE);
        return;
    }
    let picked = picked_area(app);
    if app.modals.trench_auto_place && picked.area.is_some() {
        // `KaTrenchDialogPlan.cpp:121-129` runs the search off the UI thread.
        job::start_auto_plan(app, &picked);
    } else {
        place_manual(app, picked.area_m2);
    }
}

pub(super) fn finish_plan(app: &mut StrataAppState, plan: job::Planned) {
    if plan.cells.is_empty() {
        let msg = if plan.error.is_empty() {
            "현재 규격과 방향으로 구역 안에 격자를 배치하지 못했습니다. 회전이나 규격을 바꿔 다시 적용하세요. 기존 격자는 유지됩니다.".to_owned()
        } else if plan.append_keep {
            format!("{} 기존 격자는 유지됩니다.", plan.error)
        } else {
            plan.error
        };
        app.show_status_message(msg);
        return;
    }
    if !meets_target(&plan.cells, plan.area_m2, plan.target_pct) {
        app.show_status_message(NO_TARGET);
        return;
    }
    commit_cells(app, &plan.cells, plan.area_m2, plan.target_pct);
}

/// No area, or the auto-fill box is off: rebuild around the current grid, or ask for an origin.
fn place_manual(app: &mut StrataAppState, area_m2: f64) {
    let Some(trench_i) = trench_index(app) else {
        app.show_status_message(NO_LAYER);
        return;
    };
    if app.survey_tables[trench_i].layer.features.is_empty() {
        // `beginTrenchOriginPick` (`MainWindow.cpp:1998-2021`). Not a placement.
        app.show_status_message("시굴격자: 놓을 자리(원점)를 지도에서 누르세요.");
        return;
    }
    let mut spec = spec_from_dialog(app);
    spec.origin_x = 0.0;
    spec.origin_y = 0.0;
    let mut cells = build(&spec);
    if cells.is_empty() {
        app.show_status_message("격자를 계산하지 못했습니다. 간격과 행·열을 확인하세요.");
        return;
    }
    let (Some(keep), Some(mid)) = (
        extent_center(&app.survey_tables[trench_i]),
        cells_center(&cells),
    ) else {
        app.show_status_message(KEEP_EXISTING);
        return;
    };
    let (dx, dy) = (keep[0] - mid[0], keep[1] - mid[1]);
    for cell in &mut cells {
        for pt in &mut cell.ring {
            pt[0] += dx;
            pt[1] += dy;
        }
    }
    commit_cells(app, &cells, area_m2, 0.0);
}

pub(super) fn commit_cells(
    app: &mut StrataAppState,
    cells: &[Cell],
    area_m2: f64,
    target_pct: f64,
) {
    if let Err(detail) = cells_valid(cells) {
        app.show_status_message(detail);
        return;
    }
    let Some(trench_i) = trench_index(app) else {
        app.show_status_message(NO_LAYER);
        return;
    };
    if let Some(area_i) = app
        .survey_tables
        .iter()
        .position(|t| t.table == "survey_area")
    {
        let (trench_srs, area_srs) = (
            app.survey_tables[trench_i].srs_id,
            app.survey_tables[area_i].srs_id,
        );
        if trench_srs != area_srs {
            // `TrenchLayerEdit.cpp:109-112`.
            app.show_status_message(format!(
                "기존 시굴격자의 좌표계({})가 조사구역 좌표계({})와 달라 새 격자를 놓지 못했습니다. 기존 격자는 유지됩니다.",
                auth_of(trench_srs),
                auth_of(area_srs)
            ));
            return;
        }
    }
    if same_layout(&app.survey_tables[trench_i], cells) {
        app.show_status_message(ALREADY);
        return;
    }
    let deletions: Vec<i64> = app.survey_tables[trench_i]
        .layer
        .features
        .iter()
        .map(|f| f.fid)
        .collect();
    let additions: Vec<(String, BTreeMap<String, String>)> =
        cells.iter().map(cell_feature).collect();
    if app.survey_tables[trench_i]
        .layer
        .change_add_delete(&[], additions, &deletions)
        .is_none()
    {
        app.show_status_message("새 시굴격자를 놓지 못했습니다. 기존 격자는 유지됩니다.");
        return;
    }
    app.redo_tables.clear();
    app.undo_tables
        .push(UndoStep::Table("trial_trench".to_owned()));
    app.is_dirty = true;
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::TrialTrench);
    if let Some(entry) = app
        .left_layers
        .entries
        .iter_mut()
        .find(|e| e.layer_key == "trial_trench")
    {
        entry.visible = true;
    }
    app.home_map
        .real_survey
        .layer_visible
        .insert("시굴격자".to_owned(), true);
    app.after_layer_edit("trial_trench");
    // `MainWindow.cpp:2138-2152`.
    app.show_status_message(success_message(cells, area_m2, target_pct));
}

fn success_message(cells: &[Cell], area_m2: f64, target_pct: f64) -> String {
    let t = total_area(cells);
    let mut msg = format!("시굴격자 {}개 · 총 {t:.2}㎡", cells.len());
    if area_m2 > 0.0 {
        let pct = t / area_m2 * 100.0;
        let kind = if target_pct > 0.0 && target_pct < 5.0 {
            "표본 기준 2%"
        } else {
            "시굴 기준 10%"
        };
        msg.push_str(&format!(" · 조사구역의 {pct:.1}% ({kind})"));
    }
    msg.push_str(" — 격자를 끌어 옮기세요. 우클릭 = 개별 삭제");
    msg
}

pub(super) fn spec_from_dialog(app: &StrataAppState) -> Spec {
    let mut name_prefix = app.modals.trench_prefix.trim().to_owned();
    if name_prefix.is_empty() {
        name_prefix = "Tr-".to_owned();
    }
    Spec {
        origin_x: 0.0,
        origin_y: 0.0,
        trench_width: 2.0,
        trench_length: app.modals.trench_length.meters(),
        balk_width: app.modals.trench_balk_m,
        rows: i32::try_from(app.modals.trench_rows).unwrap_or(i32::MAX),
        cols: i32::try_from(app.modals.trench_cols).unwrap_or(i32::MAX),
        azimuth_deg: app.modals.trench_azimuth_deg,
        name_prefix,
    }
}

fn picked_area(app: &StrataAppState) -> PickedArea {
    let Some(table) = app.survey_tables.iter().find(|t| t.table == "survey_area") else {
        return PickedArea::default();
    };
    let features: Vec<SurveyPoly> = table
        .layer
        .features
        .iter()
        .map(|f| SurveyPoly {
            area: geometry_from_wkt(&f.wkt_geom),
            fid: f.fid,
        })
        .collect();
    // C++ `areaVl->selectedFeatureIds` (`MainWindow.cpp:1792-1797`): every selected survey polygon,
    // including Shift and box hits kept in `also_selected`.
    let selected = selected_survey_fids(app);
    pick_auto_fill_area(&features, &selected, false)
}

pub(super) fn selected_survey_fids(app: &StrataAppState) -> Vec<i64> {
    app.all_selected()
        .into_iter()
        .filter(|(table, _)| table == "survey_area")
        .map(|(_, fid)| fid)
        .collect()
}

/// Written `trial_trench` rows. Width × length is the cell area (`TrenchGridGenerator.cpp:524-528`).
pub(crate) fn placed_trench_totals(app: &StrataAppState) -> (usize, f64) {
    let Some(table) = app.survey_tables.iter().find(|t| t.table == "trial_trench") else {
        return (0, 0.0);
    };
    let area = table.layer.features.iter().map(cell_area).sum();
    (table.layer.features.len(), area)
}

fn cell_area(feature: &strata_storage::edit_buffer::FeatureRecord) -> f64 {
    let num = |key: &str| {
        feature
            .attributes
            .get(key)
            .and_then(|s| s.parse::<f64>().ok())
    };
    match (num("width"), num("length")) {
        (Some(width), Some(length)) if width > 0.0 && length > 0.0 => width * length,
        _ => 0.0,
    }
}

fn trench_index(app: &StrataAppState) -> Option<usize> {
    app.survey_tables
        .iter()
        .position(|t| t.table == "trial_trench")
}

#[cfg(test)]
#[path = "trench_place_tests.rs"]
mod tests;
