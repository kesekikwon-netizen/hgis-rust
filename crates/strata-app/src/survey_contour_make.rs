//! Write the contour package and put the reference rows on the map.
//!
//! Folder `<survey>/측량등고선/<stem>/` and the done line are
//! `MainWindowSurveyContour.cpp:122-128` and `:187`. Lines are clipped to the
//! survey-area rings when that table can be read (`SurveyContourClip.cpp`).
//! Points and the raster are not clipped.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use strata_engine::{read_table, write_contour_package, Shapes, Transformer};
use strata_types::schema::LegendGroup;

use super::survey_contour_sheet::contour_xyz;
use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_vector::data::{LoadSpec, Look};
use crate::survey_folder::survey_folder;

const NO_SURVEY: &str = "조사를 연 뒤에 등고선을 만들 수 있습니다.";

/// Inputs copied onto the worker. The UI thread does not build the surface.
#[derive(Clone)]
pub(crate) struct ContourRequest {
    pub path: String,
    pub exclude: bool,
    pub swap_axes: bool,
    pub file_crs: usize,
    pub work: u32,
    pub interval_m: f64,
    pub cell_m: f64,
    pub max_edge_m: f64,
    pub survey: Option<PathBuf>,
}

/// What the worker wrote.
pub(crate) struct ContourMade {
    pub stem: String,
    pub points: usize,
    pub lines: usize,
    pub gpkg: PathBuf,
}

pub(crate) fn request_from(app: &StrataAppState) -> Option<ContourRequest> {
    let root = survey_folder(app.survey_path.as_deref())?;
    let _ = root;
    Some(ContourRequest {
        path: app.modals.contour_file_path.trim().to_owned(),
        exclude: app.modals.contour_exclude_suspicious,
        swap_axes: app.modals.contour_swap_axes,
        file_crs: app.modals.contour_file_crs_idx,
        work: app.work_crs.epsg_code(),
        interval_m: app.modals.contour_interval_m,
        cell_m: app.modals.contour_cell_size_m,
        max_edge_m: app.modals.contour_max_edge_m,
        survey: app.survey_path.clone(),
    })
}

pub(crate) fn build_contour(req: ContourRequest) -> Result<ContourMade, String> {
    let Some(root) = survey_folder(req.survey.as_deref()) else {
        return Err(NO_SURVEY.to_owned());
    };
    let mut points = contour_xyz(&req.path, req.exclude)?;
    if req.swap_axes {
        for point in &mut points {
            point.swap(0, 1);
        }
    }
    let file_epsg = file_epsg(req.file_crs, req.work);
    if file_epsg != req.work {
        let transformer = Transformer::new(file_epsg, req.work)
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
    let cell = if req.cell_m > 0.0 { req.cell_m } else { 0.5 };
    let stem = safe_stem(&req.path);
    let parent = root.join("측량등고선");
    let dest = parent.join(&stem);
    let staging = unique_part(&parent, &stem);
    std::fs::create_dir_all(&staging).map_err(|_| "결과 폴더를 만들지 못했습니다.".to_owned())?;
    let clip = req.survey.as_deref().map(survey_rings).unwrap_or_default();
    let output = match write_contour_package(&staging, req.work, &points, req.interval_m, cell, req.max_edge_m, &clip) {
        Ok(output) => output,
        Err(error) => {
            discard(&staging);
            return Err(error.0);
        }
    };
    if install(&staging, &dest).is_err() {
        discard(&staging);
        return Err("결과 파일을 조사 폴더에 두지 못했습니다. 이전 결과는 그대로 두었습니다.".to_owned());
    }
    Ok(ContourMade {
        stem,
        points: output.points,
        lines: output.lines,
        gpkg: dest.join("contours.gpkg"),
    })
}

pub(crate) fn apply_contour(app: &mut StrataAppState, made: &ContourMade) {
    let line_key = format!("contour:{}:lines", made.stem);
    let spec = LoadSpec {
        file: made.gpkg.clone(),
        table: "contour_lines".to_owned(),
        work_epsg: app.work_crs.epsg_code(),
        look: Look::Lines { stroke: [0x8a, 0x7a, 0x66, 255], width_mm: 0.35 },
        label: None,
    };
    app.show_reference_vector_layer(&line_key, "등고선", spec);
    if let Some(row) = app.left_layers.entries.iter_mut().find(|e| e.layer_key == line_key) {
        row.feature_count = made.lines;
    }
    let point_key = format!("contour:{}:points", made.stem);
    app.left_layers.entries.retain(|entry| entry.layer_key != point_key);
    app.left_layers.entries.insert(0, row(point_key, "측량점", made.points));
    app.status_bar_hint = format!("등고선 {}줄, 색 구간 0개를 올렸습니다.", made.lines);
    app.is_dirty = true;
}

fn survey_rings(path: &Path) -> Vec<Vec<[f64; 2]>> {
    let mut rings = Vec::new();
    let _ = read_table(path, "survey_area", &[], &mut |row| {
        if let Shapes::Polygons(parts) = row.shapes {
            for part in parts {
                if let Some(outer) = part.first() {
                    rings.push(outer.clone());
                }
            }
        }
        true
    });
    rings
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
    let name = Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let stem: String = name.chars().map(|c| match c {
        '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
        other => other,
    }).collect();
    if stem.is_empty() { "측량".to_owned() } else { stem }
}

fn unique_part(parent: &Path, stem: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    parent.join(format!(".{stem}.{nanos}.part"))
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
