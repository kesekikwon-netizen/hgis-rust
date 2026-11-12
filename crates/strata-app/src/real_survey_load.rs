//! 조사 폴더 열기 — 기본 경주 폴더, 빈 조사, `load_from_dir`, 조사구역 GPKG 읽기.
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·시그니처·동작은 그대로다.
//! 지적도·유산 GPKG 읽기는 `real_survey_gpkg_load.rs` 에 있다.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use strata_storage::wkb_codec::{decode_gpkg_wkb, WkbGeometry};

use super::real_survey_types::{
    ring_area_m2, ring_centroid, RealCadastralParcel, RealSurveyData, RealTrenchPoly,
};

impl Default for RealSurveyData {
    fn default() -> Self {
        Self::load_gyeongju_or_fallback()
    }
}

impl RealSurveyData {
    pub fn default_gyeongju_dir() -> PathBuf {
        for cand in [
            r"C:\Users\권을\Documents\경주시",
            "/mnt/c/Users/권을/Documents/경주시",
        ] {
            let p = PathBuf::from(cand);
            if p.is_dir() {
                return p;
            }
        }
        std::env::temp_dir().join("경주시")
    }

    pub fn load_gyeongju_or_fallback() -> Self {
        Self::load_from_dir(&Self::default_gyeongju_dir())
    }

    /// C++ `MainWindow::newSurvey` — 새 조사는 빈 프로젝트에서 시작한다.
    /// 경계·시굴·지적·유산 어느 것도 이전 조사에서 남기지 않는다.
    #[must_use]
    pub fn new_empty_survey(dir: &Path) -> Self {
        Self {
            survey_dir: dir.to_path_buf(),
            gpkg_path: dir.join("경주시.gpkg"),
            survey_name: String::new(),
            site_name: String::new(),
            survey_rings: Vec::new(),
            drawn_rings: 0,
            survey_area_m2: 0.0,
            survey_center: [219_745.6, 360_930.5],
            trenches: Vec::new(),
            buffer_500m: Vec::new(),
            buffer_1000m: Vec::new(),
            show_buffer_1000m: false,
            dist_labels_500m: Vec::new(),
            cadastral_parcels: Vec::new(),
            heritage_polys: Vec::new(),
            layer_visible: BTreeMap::new(),
            label_visible: BTreeMap::new(),
            layer_opacity: 1.0,
            layer_brightness: 0.5,
            file_browser_dir: dir.to_path_buf(),
            file_browser_entries: Vec::new(),
            measure_area_tab: false,
        }
    }

    pub fn load_from_dir(dir: &Path) -> Self {
        let gpkg_path = dir.join("경주시.gpkg");
        let mut data = Self {
            survey_dir: dir.to_path_buf(),
            gpkg_path: gpkg_path.clone(),
            survey_name: String::new(),
            site_name: String::new(),
            survey_rings: Vec::new(),
            drawn_rings: 0,
            survey_area_m2: 86_865.77,
            survey_center: [219_745.6, 360_930.5],
            trenches: Vec::new(),
            buffer_500m: Vec::new(),
            buffer_1000m: Vec::new(),
            show_buffer_1000m: false,
            dist_labels_500m: Vec::new(),
            cadastral_parcels: Vec::new(),
            heritage_polys: Vec::new(),
            layer_visible: BTreeMap::new(),
            label_visible: BTreeMap::new(),
            layer_opacity: 1.0,
            layer_brightness: 0.5,
            file_browser_dir: dir.to_path_buf(),
            file_browser_entries: Vec::new(),
            measure_area_tab: false,
        };
        data.load_main_gpkg(&gpkg_path);
        data.load_cadastral_gpkg(dir);
        data.load_heritage_gpkgs(dir);
        data
    }

    fn load_main_gpkg(&mut self, gpkg_path: &Path) {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let Ok(conn) = Connection::open_with_flags(gpkg_path, flags) else {
            self.populate_fallback_geometry();
            self.build_buffer_1000m();
            return;
        };
        if let Ok(mut stmt) = conn.prepare("SELECT survey_name, site_name FROM survey_area LIMIT 1")
        {
            if let Ok((s_name, st_name)) = stmt.query_row([], |r| {
                let s = r
                    .get::<_, Option<String>>(0)
                    .unwrap_or(None)
                    .unwrap_or_default();
                let st = r
                    .get::<_, Option<String>>(1)
                    .unwrap_or(None)
                    .unwrap_or_default();
                Ok((s, st))
            }) {
                self.survey_name = s_name.trim().to_owned();
                self.site_name = st_name.trim().to_owned();
            }
        }
        if let Ok(mut stmt) = conn.prepare("SELECT geom FROM survey_area") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(r)) = rows.next() {
                    if let Ok(blob) = r.get::<_, Vec<u8>>(0) {
                        if let Ok((_, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                            if let Some(ext) = rings.first() {
                                self.survey_area_m2 = ring_area_m2(ext);
                                self.survey_center = ring_centroid(ext);
                                self.survey_rings.push(ext.clone());
                            }
                        }
                    }
                }
            }
        };
        if let Ok(mut stmt) = conn.prepare("SELECT name, geom FROM trial_trench") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(r)) = rows.next() {
                    let name = r.get::<_, String>(0).unwrap_or_default();
                    if let Ok(blob) = r.get::<_, Vec<u8>>(1) {
                        if let Ok((_, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                            if let Some(ext) = rings.first() {
                                self.trenches.push(RealTrenchPoly {
                                    name,
                                    ring: ext.clone(),
                                    area_m2: ring_area_m2(ext),
                                    center: ring_centroid(ext),
                                });
                            }
                        }
                    }
                }
            }
        };
        if let Ok(mut stmt) = conn.prepare("SELECT geom FROM \"주변_500m\" LIMIT 1") {
            if let Ok(blob) = stmt.query_row([], |r| r.get::<_, Vec<u8>>(0)) {
                if let Ok((_, WkbGeometry::LineString(pts))) = decode_gpkg_wkb(&blob) {
                    self.buffer_500m = pts;
                }
            }
        };
        if let Ok(mut stmt) = conn.prepare("SELECT label, geom FROM \"주변_500m_거리\"") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(r)) = rows.next() {
                    let lbl = r.get::<_, String>(0).unwrap_or_default();
                    if let Ok(blob) = r.get::<_, Vec<u8>>(1) {
                        if let Ok((_, WkbGeometry::Point(pt))) = decode_gpkg_wkb(&blob) {
                            self.dist_labels_500m.push((lbl, pt));
                        }
                    }
                }
            }
        };
        if self.survey_rings.is_empty() {
            self.populate_fallback_geometry();
        }
        self.build_buffer_1000m();
    }

    fn populate_fallback_geometry(&mut self) {
        self.survey_rings = vec![vec![
            [219_754.87, 361_410.69],
            [219_712.66, 361_397.50],
            [219_683.64, 360_632.36],
            [219_712.66, 360_450.31],
            [219_779.93, 360_463.50],
            [219_807.64, 361_384.31],
            [219_754.87, 361_410.69],
        ]];
        self.survey_center = [219_745.6, 360_930.5];
        self.buffer_500m = (0..=64)
            .map(|i| {
                let a = (i as f64 / 64.0) * std::f64::consts::TAU;
                [
                    self.survey_center[0] + 550.0 * a.cos(),
                    self.survey_center[1] + 980.0 * a.sin(),
                ]
            })
            .collect();
        if self.cadastral_parcels.is_empty() {
            self.cadastral_parcels.push(RealCadastralParcel {
                jibun: "76 전".to_owned(),
                ring: vec![
                    [219_730.0, 360_920.0],
                    [219_760.0, 360_920.0],
                    [219_760.0, 360_940.0],
                    [219_730.0, 360_940.0],
                    [219_730.0, 360_920.0],
                ],
                center: [219_745.0, 360_930.0],
            });
        }
    }

    pub fn build_buffer_1000m(&mut self) {
        let [cx, cy] = self.survey_center;
        self.buffer_1000m = self
            .buffer_500m
            .iter()
            .map(|p| {
                let (dx, dy) = (p[0] - cx, p[1] - cy);
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                [p[0] + dx / len * 500.0, p[1] + dy / len * 500.0]
            })
            .collect();
    }
}
