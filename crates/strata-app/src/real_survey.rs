use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use strata_geo::crs_transform::CrsTransformer;
use strata_geo::trench_grid::{build_lshape_ring, TrenchShapeKind, TrenchStandardLength};
use strata_storage::wkb_codec::{decode_gpkg_wkb, encode_gpkg_wkb, WkbGeometry};
use strata_types::crs::CrsCode;

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

    pub fn load_from_dir(dir: &Path) -> Self {
        let gpkg_path = dir.join("경주시.gpkg");
        let mut data = Self {
            survey_dir: dir.to_path_buf(),
            gpkg_path: gpkg_path.clone(),
            survey_name: String::new(),
            site_name: String::new(),
            survey_rings: Vec::new(),
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

    fn load_cadastral_gpkg(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir.join("지적도").join("표시")) else {
            return;
        };
        let Some(gpkg) = entries
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().and_then(|s| s.to_str()) == Some("gpkg"))
        else {
            return;
        };
        let Ok(conn) = Connection::open_with_flags(
            &gpkg,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            return;
        };
        let [cx, cy] = self.survey_center;
        let sql = "SELECT c.JIBUN, c.geom FROM cadastral c JOIN rtree_cadastral_geom r ON c.fid = r.id \
                   WHERE r.minx <= ?1 AND r.maxx >= ?2 AND r.miny <= ?3 AND r.maxy >= ?4 LIMIT 1800";
        if let Ok(mut stmt) = conn.prepare(sql) {
            if let Ok(mut rows) = stmt.query([cx + 1500.0, cx - 1500.0, cy + 1300.0, cy - 1300.0]) {
                while let Ok(Some(r)) = rows.next() {
                    let jibun = r
                        .get::<_, Option<String>>(0)
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    if let Ok(blob) = r.get::<_, Vec<u8>>(1) {
                        if let Ok((_, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                            if let Some(ext) = rings.first() {
                                self.cadastral_parcels.push(RealCadastralParcel {
                                    jibun,
                                    ring: ext.clone(),
                                    center: ring_centroid(ext),
                                });
                            }
                        }
                    }
                }
            }
        };
    }

    fn load_heritage_gpkgs(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir.join("가져온자료")) else {
            return;
        };
        let xf = CrsTransformer::new(CrsCode::Epsg5179, CrsCode::Epsg5187);
        let [cx, cy] = self.survey_center;
        for entry in entries.flatten() {
            let layer_name = entry.file_name().to_string_lossy().to_string();
            let gpkg = entry.path().join("heritage.gpkg");
            let is_green = layer_name.contains("매장")
                || layer_name.contains("분포")
                || layer_name.contains("유존")
                || layer_name.contains("보호");
            let Ok(conn) = Connection::open_with_flags(
                &gpkg,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) else {
                continue;
            };
            if let Ok(mut stmt) = conn.prepare("SELECT __hgis_geom FROM heritage LIMIT 250") {
                if let Ok(mut rows) = stmt.query([]) {
                    while let Ok(Some(r)) = rows.next() {
                        if let Ok(blob) = r.get::<_, Vec<u8>>(0) {
                            if let Ok((srs, WkbGeometry::Polygon(rings))) = decode_gpkg_wkb(&blob) {
                                for ring in rings {
                                    let ring_5187: Vec<[f64; 2]> = if srs == 5179 {
                                        ring.into_iter()
                                            .map(|p| xf.transform_xy(p[0], p[1]))
                                            .collect()
                                    } else {
                                        ring
                                    };
                                    let center = ring_centroid(&ring_5187);
                                    if (center[0] - cx).abs() <= 2600.0
                                        && (center[1] - cy).abs() <= 2200.0
                                    {
                                        self.heritage_polys.push(RealHeritagePoly {
                                            layer_name: layer_name.clone(),
                                            site_name: layer_name.clone(),
                                            is_green,
                                            ring: ring_5187,
                                            center,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            };
        }
    }

    pub fn is_layer_visible(&self, name: &str) -> bool {
        *self.layer_visible.get(name).unwrap_or(&true)
    }
    /// `BasemapOps::toggleLayerVisibility` & `revealCheckedLegendNode` (`c9e58e3`) — 전체 끄기 후 접힌 하위 레이어 묶음만 체크해도 자식 레이어와 상위 그룹이 함께 켜짐
    pub fn toggle_layer_visible(&mut self, name: &str) {
        const HERITAGE_LEAVES: [&str; 5] = [
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ];
        let next = !self.is_layer_visible(name);
        self.layer_visible.insert(name.to_owned(), next);
        if name == "지정유산" {
            if next {
                let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
                if !any_leaf_on {
                    for k in HERITAGE_LEAVES {
                        self.layer_visible.insert(k.to_owned(), true);
                    }
                }
                self.layer_visible.insert("참조 지도".to_owned(), true);
            } else {
                for k in HERITAGE_LEAVES {
                    self.layer_visible.insert(k.to_owned(), false);
                }
            }
        } else if name == "참조 지도" {
            if next {
                let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
                if !any_leaf_on {
                    self.layer_visible.insert("지정유산".to_owned(), true);
                    for k in HERITAGE_LEAVES {
                        self.layer_visible.insert(k.to_owned(), true);
                    }
                }
            } else {
                self.layer_visible.insert("지정유산".to_owned(), false);
                for k in HERITAGE_LEAVES {
                    self.layer_visible.insert(k.to_owned(), false);
                }
            }
        } else if HERITAGE_LEAVES.contains(&name) {
            let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
            self.layer_visible
                .insert("지정유산".to_owned(), any_leaf_on);
            self.layer_visible
                .insert("참조 지도".to_owned(), any_leaf_on);
        }
    }

    /// C++ `KaLayerSectionDelegate` (`Qt::CheckState`: 0 = Unchecked, 1 = PartiallyChecked, 2 = Checked)
    pub fn layer_check_state(&self, name: &str) -> u8 {
        const HERITAGE_LEAVES: [&str; 5] = [
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ];
        if name == "지정유산" || name == "참조 지도" {
            if !self.is_layer_visible(name) {
                return 0;
            }
            let on_cnt = HERITAGE_LEAVES
                .iter()
                .filter(|&&k| self.is_layer_visible(k))
                .count();
            if on_cnt == 0 {
                0
            } else if on_cnt < HERITAGE_LEAVES.len() {
                1
            } else {
                2
            }
        } else if self.is_layer_visible(name) {
            2
        } else {
            0
        }
    }
    pub fn is_label_visible(&self, name: &str) -> bool {
        *self.label_visible.get(name).unwrap_or(&true)
    }
    pub fn toggle_label_visible(&mut self, name: &str) {
        let next = !self.is_label_visible(name);
        self.label_visible.insert(name.to_owned(), next);
    }

    pub fn set_all_layers_visible(&mut self, visible: bool) {
        for k in [
            "지적도 · 조사 주변 5km",
            "시굴격자",
            "지형 음영",
            "지표모델(DSM) 30m · Coperni...",
            "조사구역",
            "주변 500m 거리",
            "주변 500m",
            "참조 지도",
            "지정유산",
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ] {
            self.layer_visible.insert(k.to_owned(), visible);
        }
    }

    pub fn refresh_file_browser(&mut self) {
        self.file_browser_entries.clear();
        let Ok(rd) = std::fs::read_dir(&self.file_browser_dir) else {
            return;
        };
        let mut items: Vec<(String, PathBuf, bool)> = rd
            .flatten()
            .map(|e| {
                let p = e.path();
                (
                    e.file_name().to_string_lossy().to_string(),
                    p.clone(),
                    p.is_dir(),
                )
            })
            .filter(|(n, _, _)| !n.starts_with('.'))
            .collect();
        items.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        self.file_browser_entries = items;
    }

    pub fn save_drawn_polygon(&mut self, ring: Vec<[f64; 2]>) {
        if ring.len() < 3 {
            return;
        }
        self.survey_rings.push(ring.clone());
        if let Ok(conn) = Connection::open(&self.gpkg_path) {
            let blob = encode_gpkg_wkb(5187, &WkbGeometry::Polygon(vec![ring]));
            let _ = conn.execute(
                "INSERT INTO feature_poly (geom, name, note) VALUES (?1, '신규 유구', 'Rust 저장')",
                [blob],
            );
        }
    }

    pub fn export_nearby_heritage_csv(&self) -> Option<PathBuf> {
        let out_path = self.survey_dir.join("주변유적_목록.csv");
        let [cx, cy] = self.survey_center;
        let mut csv = String::from(
            "\u{feff}연번,분류,유적명,중심X(EPSG:5187),중심Y(EPSG:5187),이격거리(m)\n",
        );
        for (i, h) in self.heritage_polys.iter().enumerate() {
            let dist = ((h.center[0] - cx).powi(2) + (h.center[1] - cy).powi(2)).sqrt();
            csv.push_str(&format!(
                "{},{},{},{:.2},{:.2},{:.1}\n",
                i + 1,
                h.layer_name,
                h.site_name,
                h.center[0],
                h.center[1],
                dist
            ));
        }
        std::fs::write(&out_path, csv.as_bytes()).ok()?;
        Some(out_path)
    }

    /// `KaTrenchDialogPlan.cpp` & `TrenchGridGenerator.cpp` & `TrenchLShape.cpp` — 고른 길이(20·15·10 m) 또는 ㄱ·ㄴ자(15 m + 15 m) 10% 배치
    pub fn apply_trench_plan(
        &mut self,
        length: TrenchStandardLength,
        shape: TrenchShapeKind,
        grid_snap: bool,
    ) -> (usize, f64, f64) {
        let [cx, cy] = self.survey_center;
        let area_m2 = self.survey_area_m2.max(1000.0);
        let target_area = area_m2 * 0.10;
        let grid_step = if grid_snap { Some(5.0) } else { None };
        let mut built = Vec::new();

        if let Some(bend) = shape.arm_bend() {
            let one_area = 56.0;
            let count = ((target_area / one_area).ceil() as usize).max(1);
            let cols = ((count as f64).sqrt().ceil() as usize).max(1);
            let step = 25.0;
            for idx in 0..count {
                let r = idx / cols;
                let c = idx % cols;
                let u0 = cx - 90.0 + (c as f64) * step;
                let v0 = cy - 120.0 + (r as f64) * step;
                let ring = build_lshape_ring(u0, v0, 15.0, 2.0, bend, grid_step);
                let center = ring_centroid(&ring);
                built.push(RealTrenchPoly {
                    name: format!("Tr-{}", idx + 1),
                    ring,
                    area_m2: one_area,
                    center,
                });
            }
        } else {
            let len_m = length.meters();
            let wid_m = 2.0;
            let one_area = len_m * wid_m;
            let count = ((target_area / one_area).round() as usize).max(1);
            let cols = 8usize;
            let step_x = 12.0;
            let step_y = len_m + 10.0;
            for idx in 0..count {
                let r = idx / cols;
                let c = idx % cols;
                let mut x0 = cx - 42.0 + (c as f64) * step_x;
                let mut y0 = cy - 380.0 + (r as f64) * step_y;
                if let Some(st) = grid_step {
                    x0 = (x0 / st).round() * st;
                    y0 = (y0 / st).round() * st;
                }
                let ring = vec![
                    [x0, y0],
                    [x0 + wid_m, y0],
                    [x0 + wid_m, y0 + len_m],
                    [x0, y0 + len_m],
                    [x0, y0],
                ];
                let center = ring_centroid(&ring);
                built.push(RealTrenchPoly {
                    name: format!("Tr-{}", idx + 1),
                    ring,
                    area_m2: one_area,
                    center,
                });
            }
        }
        let total_area: f64 = built.iter().map(|t| t.area_m2).sum();
        let ratio_pct = (total_area / area_m2) * 100.0;
        let n = built.len();
        self.trenches = built;
        (n, total_area, ratio_pct)
    }

    /// `KaTrenchMoveToolKeys.cpp` — 화살표 키 또는 드래그로 트렌치를 통째로 이동 (좌표격자 켜짐 시 교점 스냅)
    pub fn translate_all_trenches(&mut self, dx: f64, dy: f64, snap_step: Option<f64>) {
        for tr in &mut self.trenches {
            let mut eff_dx = dx;
            let mut eff_dy = dy;
            if let (Some(step), Some(first)) = (snap_step.filter(|&s| s > 0.0), tr.ring.first()) {
                let target_x = ((first[0] + dx) / step).round() * step;
                let target_y = ((first[1] + dy) / step).round() * step;
                eff_dx = target_x - first[0];
                eff_dy = target_y - first[1];
            }
            for pt in &mut tr.ring {
                pt[0] += eff_dx;
                pt[1] += eff_dy;
            }
            tr.center[0] += eff_dx;
            tr.center[1] += eff_dy;
        }
    }
}
