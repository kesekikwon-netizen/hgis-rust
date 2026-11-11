//! 받은 DEM(표고 GeoTIFF)을 지도에 색띠 + 음영 그림으로 올린다.
//!
//! C++ 은 QgsRasterLayer 에 `DemPresentation`(표고 색띠)을 입혀 「참조 지도」에 올린다
//! (`MainWindowDownloads.cpp:69-96`). 여기서는 화면 주변만 읽어(`dem_tiff_read`) 그림으로 만들고
//! (`dem_shade`), 경위도 격자를 작업 좌표계로 옮긴 그물에 입혀 그린다.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use eframe::egui::{self, Color32, Pos2, Rect};
use strata_geo::crs_transform::CrsTransformer;
use strata_paleo::dem_shade::{shade_dem, ShadedDem};
use strata_paleo::dem_hfa_read::{mosaic_dems, read_hfa_dem};
use strata_paleo::dem_tiff_read::{read_dem_window, DemRaster};
use strata_types::crs::{CrsCode, WorkCrs};

use crate::app_state::StrataAppState;
use crate::gui_tab_map::w2s;

/// 레이어 목록에서 DEM 그림을 켜고 끄는 이름.
pub(crate) const DEM_IMAGE_LAYER: &str = "지형 음영";
/// 화면 가운데에서 적어도 이만큼(m)은 읽는다 — 조금 움직여도 그림이 끊기지 않게.
const MIN_HALF_SPAN_M: f64 = 10_000.0;
/// 그림을 입히는 그물의 한 변 칸 수(경위도 → 작업 좌표계의 휨을 따라간다).
const MESH_CELLS: usize = 16;

type CachedTexture = Option<(([u64; 4], usize), egui::TextureHandle)>;

fn texture_slot() -> &'static Mutex<CachedTexture> {
    static SLOT: OnceLock<Mutex<CachedTexture>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

const fn crs_code(work: WorkCrs) -> CrsCode {
    match work {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    }
}

impl StrataAppState {
    /// 화면 주변의 경위도 범위 `[west, south, east, north]`.
    fn dem_clip_lonlat(&self) -> [f64; 4] {
        let vp = &self.home_map.viewport;
        let half = [
            (vp.screen_size_px[0] * 0.5 * vp.meters_per_pixel).max(MIN_HALF_SPAN_M),
            (vp.screen_size_px[1] * 0.5 * vp.meters_per_pixel).max(MIN_HALF_SPAN_M),
        ];
        let [cx, cy] = vp.center_xy;
        let to_wgs = CrsTransformer::new(crs_code(self.work_crs), CrsCode::Epsg4326);
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for (x, y) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            let [lon, lat] = to_wgs.transform_xy(cx + x * half[0], cy + y * half[1]);
            bbox = [bbox[0].min(lon), bbox[1].min(lat), bbox[2].max(lon), bbox[3].max(lat)];
        }
        bbox
    }

    /// 받은 표고를 지도에 올린다. 올린 칸 수를 돌려준다.
    /// `path`: 경위도 GeoTIFF(화면 주변만 읽는다), 공개DEM `.img` 한 장, 또는 `.img` 들이 든 폴더(붙인다).
    pub(crate) fn load_dem_image(&mut self, path: &Path) -> Result<usize, String> {
        let is_img = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("img"));
        let raster: DemRaster = if path.is_dir() {
            let mut sheets = Vec::new();
            for file in img_files_under(path, 3) {
                sheets.push(read_hfa_dem(&file)?);
            }
            mosaic_dems(&sheets).ok_or("이 폴더에서 붙일 수 있는 공개DEM(.img)을 찾지 못했습니다.")?
        } else if is_img(path) {
            read_hfa_dem(path)?
        } else {
            read_dem_window(path, Some(self.dem_clip_lonlat()))?
        };
        if raster.min_max().is_none() {
            return Err("이 범위의 DEM 에는 표고 값이 없습니다.".to_owned());
        }
        let shaded = shade_dem(&raster);
        let cells = shaded.width * shaded.height;
        self.home_map.dem_image = Some(shaded);
        self.home_map.real_survey.layer_visible.insert(DEM_IMAGE_LAYER.to_owned(), true);
        Ok(cells)
    }

    /// DEM 그림을 그린다(참조 레이어·조사 도형 아래).
    pub(crate) fn paint_dem_image(&self, p: &egui::Painter, rect: Rect) {
        let Some(image) = self.home_map.dem_image.as_ref() else {
            return;
        };
        if !self.home_map.real_survey.is_layer_visible(DEM_IMAGE_LAYER) {
            return;
        }
        let Some(texture) = dem_texture(p.ctx(), image) else {
            return;
        };
        let from = if image.projected { CrsCode::Epsg5179 } else { CrsCode::Epsg4326 };
        let to_work = CrsTransformer::new(from, crs_code(self.work_crs));
        let vp = &self.home_map.viewport;
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for (lon, lat, u, v) in mesh_points(image.bbox) {
            let pos: Pos2 = w2s(rect, vp, to_work.transform_xy(lon, lat));
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::pos2(u, v), color: Color32::WHITE });
        }
        let side = (MESH_CELLS + 1) as u32;
        for row in 0..MESH_CELLS as u32 {
            for col in 0..MESH_CELLS as u32 {
                let a = row * side + col;
                mesh.add_triangle(a, a + 1, a + side);
                mesh.add_triangle(a + 1, a + side + 1, a + side);
            }
        }
        p.with_clip_rect(rect).add(egui::Shape::mesh(mesh));
    }
}

/// 폴더 아래(`depth` 단계까지)의 `.img` 파일들, 이름 순.
fn img_files_under(dir: &Path, depth: usize) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() && depth > 0 {
            found.extend(img_files_under(&path, depth - 1));
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("img")) {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// 그물의 꼭짓점: (경도, 위도, 그림의 가로 위치 0~1, 세로 위치 0~1). 북서쪽부터 줄 순서.
fn mesh_points(bbox: [f64; 4]) -> Vec<(f64, f64, f32, f32)> {
    let [west, south, east, north] = bbox;
    let mut out = Vec::with_capacity((MESH_CELLS + 1) * (MESH_CELLS + 1));
    for row in 0..=MESH_CELLS {
        for col in 0..=MESH_CELLS {
            let (u, v) = (col as f64 / MESH_CELLS as f64, row as f64 / MESH_CELLS as f64);
            out.push((west + (east - west) * u, north - (north - south) * v, u as f32, v as f32));
        }
    }
    out
}

/// 그림을 화면용 텍스처로(같은 그림이면 만들어 둔 것을 다시 쓴다).
fn dem_texture(ctx: &egui::Context, image: &ShadedDem) -> Option<egui::TextureHandle> {
    let key = (image.bbox.map(f64::to_bits), image.rgba.len());
    let mut slot = texture_slot().lock().ok()?;
    if let Some((cached, texture)) = slot.as_ref() {
        if *cached == key {
            return Some(texture.clone());
        }
    }
    let pixels = egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.rgba);
    let texture = ctx.load_texture("dem-image", pixels, egui::TextureOptions::LINEAR);
    *slot = Some((key, texture.clone()));
    Some(texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mesh_runs_from_the_north_west_corner_row_by_row() {
        let points = mesh_points([127.0, 36.0, 128.0, 37.0]);
        assert_eq!(points.len(), (MESH_CELLS + 1) * (MESH_CELLS + 1));
        assert_eq!(points[0], (127.0, 37.0, 0.0, 0.0));
        assert_eq!(points[MESH_CELLS], (128.0, 37.0, 1.0, 0.0));
        assert_eq!(points[points.len() - 1], (128.0, 36.0, 1.0, 1.0));
    }

    #[test]
    fn a_real_dem_file_becomes_an_image_around_the_view_and_turns_its_layer_on() {
        let tif = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/probe/dem/N36E127.tif");
        if !tif.is_file() {
            eprintln!("받은 DEM 타일이 없어 건너뜁니다: {}", tif.display());
            return;
        }
        let mut app = StrataAppState::new_startup();
        // 견본 조사 위치(경도 127.06, 위도 36.65)를 작업 좌표계로.
        let to_work = CrsTransformer::new(CrsCode::Epsg4326, crs_code(app.work_crs));
        app.home_map.viewport.center_xy = to_work.transform_xy(127.06, 36.65);
        app.home_map.viewport.meters_per_pixel = 5.0;
        let cells = app.load_dem_image(&tif).expect("DEM 읽기");
        let image = app.home_map.dem_image.as_ref().expect("그림");
        assert_eq!(cells, image.width * image.height);
        // 가운데에서 사방 10 km 이상(타일 서쪽 끝 127.0 에서 잘린다): 약 0.17° × 0.18° 이상.
        assert!(image.bbox[0] <= 127.0 + 1e-6 && image.bbox[2] > 127.15, "{:?}", image.bbox);
        assert!(image.bbox[1] < 36.57 && image.bbox[3] > 36.73, "{:?}", image.bbox);
        assert!(app.home_map.real_survey.is_layer_visible(DEM_IMAGE_LAYER));
        // 범위 밖(타일에 없는 곳)에서는 알려 주고 그림을 바꾸지 않는다.
        app.home_map.viewport.center_xy = to_work.transform_xy(129.0, 35.0);
        assert!(app.load_dem_image(&tif).is_err());
    }

    #[test]
    fn received_ngii_open_dem_sheets_are_joined_into_one_image() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/probe/dem_survey/DEM/원본");
        if img_files_under(&folder, 3).len() < 2 {
            eprintln!("받은 공개DEM 이 없어 건너뜁니다: {}", folder.display());
            return;
        }
        let mut app = StrataAppState::new_startup();
        let cells = app.load_dem_image(&folder).expect("공개DEM 읽기");
        let image = app.home_map.dem_image.as_ref().expect("그림");
        // 도엽 36608·36705 두 장: EPSG:5179, 90 m 격자가 한 장으로 붙는다.
        assert!(image.projected && cells == image.width * image.height && image.width >= 256 && image.height >= 315);
        assert!(image.bbox[0] < 955_080.0 + 1.0 && image.bbox[3] > 1_861_380.0 - 1.0, "{:?}", image.bbox);
    }
}
