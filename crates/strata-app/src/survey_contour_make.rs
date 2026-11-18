//! Write the contour package and put the reference rows on the legend.
//!
//! Folder `<survey>/측량등고선/<stem>/` and the done line are
//! `MainWindowSurveyContour.cpp:122-128` and `:187`. The rows are reference layers,
//! never a domain `section_line` (`SurveyContourStyle.cpp:70-87`).

use std::path::Path;

use strata_engine::{write_contour_package, Transformer};
use strata_types::schema::LegendGroup;

use super::survey_contour_sheet::contour_xyz;
use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::survey_folder::survey_folder;

const NO_SURVEY: &str = "조사를 연 뒤에 등고선을 만들 수 있습니다.";

/// Builds the package for the dialog's file. `Err` is shown in the preview and nothing is written.
pub(crate) fn make_survey_contours(app: &mut StrataAppState) -> Result<(), String> {
    let path = app.modals.contour_file_path.trim().to_owned();
    let Some(root) = survey_folder(app.survey_path.as_deref()) else {
        return Err(NO_SURVEY.to_owned());
    };
    let mut points = contour_xyz(&path, app.modals.contour_exclude_suspicious)?;
    let work = app.work_crs.epsg_code();
    let file_epsg = file_epsg(app.modals.contour_file_crs_idx, work);
    if file_epsg != work {
        let transformer = Transformer::new(file_epsg, work)
            .map_err(|_| format!("좌표계 EPSG:{file_epsg} 을 읽지 못했습니다."))?;
        let mut kept = Vec::new();
        let mut dropped = 0usize;
        for point in &points {
            match transformer.transform([point[0], point[1]]) {
                Ok(xy) => kept.push([xy[0], xy[1], point[2]]),
                Err(_) => dropped += 1,
            }
        }
        if kept.is_empty() && dropped > 0 {
            return Err("좌표계를 바꾸지 못한 점만 남았습니다.".to_owned());
        }
        points = kept;
    }
    let cell = if app.modals.contour_cell_size_m > 0.0 {
        app.modals.contour_cell_size_m
    } else {
        0.5
    };
    let stem = safe_stem(&path);
    let parent = root.join("측량등고선");
    let dest = parent.join(&stem);
    let staging = parent.join(format!("{stem}_new"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|_| "결과 폴더를 만들지 못했습니다.".to_owned())?;
    let output =
        match write_contour_package(&staging, work, &points, app.modals.contour_interval_m, cell) {
            Ok(output) => output,
            Err(error) => {
                discard(&staging);
                return Err(error.0);
            }
        };
    if install(&staging, &dest).is_err() {
        discard(&staging);
        return Err(
            "결과 파일을 조사 폴더에 두지 못했습니다. 이전 결과는 그대로 두었습니다.".to_owned(),
        );
    }
    place(app, &stem, output.points, output.lines);
    app.status_bar_hint = format!("등고선 {}줄, 색 구간 0개를 올렸습니다.", output.lines);
    app.is_dirty = true;
    Ok(())
}

fn file_epsg(index: usize, work: u32) -> u32 {
    match index {
        1 => 5185,
        2 => 5186,
        3 => 5179,
        4 => 4326,
        _ => work,
    }
}

fn safe_stem(path: &str) -> String {
    let name = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    if stem.is_empty() {
        "측량".to_owned()
    } else {
        stem
    }
}

fn discard(staging: &Path) {
    let _ = std::fs::remove_dir_all(staging);
    if let Some(parent) = staging.parent() {
        if parent.read_dir().ok().is_some_and(|mut entries| entries.next().is_none()) {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

fn install(staging: &Path, dest: &Path) -> std::io::Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(staging, dest)
}

fn place(app: &mut StrataAppState, stem: &str, points: usize, lines: usize) {
    let point_key = format!("contour:{stem}:points");
    let line_key = format!("contour:{stem}:lines");
    app.left_layers
        .entries
        .retain(|entry| entry.layer_key != point_key && entry.layer_key != line_key);
    app.left_layers
        .entries
        .insert(0, row(line_key, "등고선", lines));
    app.left_layers
        .entries
        .insert(0, row(point_key, "측량점", points));
}

fn row(key: String, label: &str, count: usize) -> LayerTreeEntry {
    LayerTreeEntry {
        layer_key: key,
        label_ko: label.to_owned(),
        group: LegendGroup::ReferenceMap,
        visible: true,
        opacity: 1.0,
        read_only: true,
        snap_enabled: false,
        feature_count: count,
    }
}
