//! 조사 데이터의 기하 타입(지적·유산·시굴·조사구역)과 면적/둘레/중심 계산.
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·필드·동작은 그대로다.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RealCadastralParcel {
    pub jibun: String,
    pub ring: Vec<[f64; 2]>,
    pub center: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct RealHeritagePoly {
    pub layer_name: String,
    pub site_name: String,
    pub is_green: bool,
    pub ring: Vec<[f64; 2]>,
    pub center: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct RealTrenchPoly {
    pub name: String,
    pub ring: Vec<[f64; 2]>,
    pub area_m2: f64,
    pub center: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct RealSurveyData {
    pub survey_dir: PathBuf,
    pub gpkg_path: PathBuf,
    pub survey_name: String,
    pub site_name: String,
    pub survey_rings: Vec<Vec<[f64; 2]>>,
    pub survey_area_m2: f64,
    pub survey_center: [f64; 2],
    pub trenches: Vec<RealTrenchPoly>,
    pub buffer_500m: Vec<[f64; 2]>,
    pub buffer_1000m: Vec<[f64; 2]>,
    pub show_buffer_1000m: bool,
    pub dist_labels_500m: Vec<(String, [f64; 2])>,
    pub cadastral_parcels: Vec<RealCadastralParcel>,
    pub heritage_polys: Vec<RealHeritagePoly>,
    pub layer_visible: BTreeMap<String, bool>,
    pub label_visible: BTreeMap<String, bool>,
    pub layer_opacity: f32,
    pub layer_brightness: f32,
    pub file_browser_dir: PathBuf,
    pub file_browser_entries: Vec<(String, PathBuf, bool)>,
    pub measure_area_tab: bool,
}

pub fn ring_area_m2(ring: &[[f64; 2]]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        s += ring[i][0] * ring[j][1] - ring[j][0] * ring[i][1];
    }
    0.5 * s.abs()
}

pub fn ring_perimeter_m(ring: &[[f64; 2]]) -> f64 {
    if ring.len() < 2 {
        return 0.0;
    }
    let mut p = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        let dx = ring[j][0] - ring[i][0];
        let dy = ring[j][1] - ring[i][1];
        p += (dx * dx + dy * dy).sqrt();
    }
    p
}

pub fn ring_centroid(ring: &[[f64; 2]]) -> [f64; 2] {
    if ring.is_empty() {
        return [219_745.6, 360_930.5];
    }
    let (mut sx, mut sy) = (0.0, 0.0);
    for p in ring {
        sx += p[0];
        sy += p[1];
    }
    let n = ring.len() as f64;
    [sx / n, sy / n]
}
