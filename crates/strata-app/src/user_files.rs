//! Files of the user put on the map: dropped on the window (C++ `tryAddDroppedPaths`, MainWindowFiles.cpp:
//! 260-289), opened in 파일함 (`fileActivated`, MainWindow.cpp:902-914) or picked with 「벡터·도면 불러오기」
//! (`openVectorLayer`, :2592-2636). A shapefile of lines or polygons becomes a row of 조사 데이터 named after
//! the file (`addVectorFromPath`, :2939-3033: key `user:<name>`, black 0.2 mm, no fill), read where it lies:
//! nothing is written beside it. A picture becomes a row of 주변·참조 (picture_layers.rs). Points, the tables of
//! other GeoPackages, GeoJSON and CAD drawings are not carried over yet and are refused in words
//! (docs/RUST_PLAN.md 「F-파일 불러오기-1」).

use std::path::{Path, PathBuf};

use strata_engine::{ShapeKind, Transformer};
use strata_types::crs::CrsCode;

use crate::app_state::{MainTab, StrataAppState};
use crate::ref_vector::data::{LoadSpec, Look, PolygonFill};

/// The beginning of a user row's `layer_key` (`LayerOps::markSurveyLayer(layer, "user:%1")`, :2967).
pub(crate) const USER_PREFIX: &str = "user:";
/// The tables that make a GeoPackage a survey file (:2982-2993).
const SURVEY_TABLES: [&str; 4] = ["survey_area", "feature_poly", "trial_trench", "control_points"];
/// The words of the drawer for a file that can be no layer (MainWindow.cpp:905-912).
const DRAWER_CANNOT: &str = "선택한 파일을 지도 레이어로 열지 못했습니다. SHP/DXF/DWG/GPKG/GeoTIFF/JPG만 지도에 올릴 수 있습니다.";

/// What a file is to the window (`GeorefService::isImagePath`, `isCadPath`, GeorefService.cpp:514-524).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Shapefile,
    GeoPackage,
    GeoJson,
    Cad,
    Picture,
    Other,
}

fn kind_of(path: &Path) -> FileKind {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    match ext.as_str() {
        "shp" => FileKind::Shapefile,
        "gpkg" => FileKind::GeoPackage,
        "geojson" | "json" => FileKind::GeoJson,
        "dxf" | "dwg" => FileKind::Cad,
        "jpg" | "jpeg" | "png" | "tif" | "tiff" | "gtiff" => FileKind::Picture,
        _ => FileKind::Other,
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A layer put on the map: its title, and the words `addVectorFromPath` / `addRasterFromPath` leave on the status line.
pub(crate) struct Added {
    title: String,
    words: String,
}

/// Why a file did not go on the map.
pub(crate) enum Refused {
    /// A kind this app does not carry over yet: the words say which.
    NotYet(String),
    /// The file cannot be read as what it claims to be.
    Unreadable,
}

/// The look of a user layer: black outlines of 0.2 mm, no fill (`applySimpleVectorStyle`, :2969).
fn user_look(kind: ShapeKind) -> Look {
    match kind {
        ShapeKind::Lines => Look::Lines { stroke: [0, 0, 0, 255], width_mm: 0.2 },
        ShapeKind::Polygons => Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.2, fill: PolygonFill::None },
    }
}

/// The layer of a user shapefile, with its number of shapes and its box in the work CRS (none when the file
/// has no box). A shapefile that names no CRS is taken to be in the work CRS (`openVectorLayer` :2618-2624).
pub(crate) fn user_shapefile(path: &Path, work_epsg: u32) -> Result<(LoadSpec, u64, Option<[f64; 4]>), Refused> {
    let encoding = strata_refmap::heritage::shapefile_encoding(path);
    let title = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    if strata_engine::shapefile_holds_points(path, &encoding).map_err(|_| Refused::Unreadable)? {
        return Err(Refused::NotYet(format!("「{title}」은(는) 점 레이어라 아직 지도에 올리지 못합니다.")));
    }
    let facts = strata_engine::shapefile_facts(path, &encoding).map_err(|_| Refused::Unreadable)?;
    let epsg = facts.epsg.unwrap_or(work_epsg);
    // A CRS PROJ cannot carry into the work CRS would leave a row that draws nothing.
    let carry = Transformer::new(epsg, work_epsg).map_err(|_| Refused::Unreadable)?;
    let source_box = strata_engine::shapefile_extent(path, &encoding).ok().filter(|b| b.iter().all(|v| v.is_finite()) && b[2] >= b[0] && b[3] >= b[1]);
    let work_box = source_box.and_then(|b| if epsg == work_epsg { Some(b) } else { carry.transform_box(b).ok() });
    let spec = LoadSpec { file: path.to_path_buf(), table: String::new(), work_epsg, look: user_look(facts.kind), label: None };
    Ok((spec, facts.rows, work_box))
}

/// Whether the GeoPackage holds the tables of a survey.
fn is_survey_file(path: &Path) -> bool {
    strata_storage::survey_gpkg_read::read_survey_tables(path).is_ok_and(|tables| tables.iter().any(|t| SURVEY_TABLES.contains(&t.table.as_str())))
}

impl StrataAppState {
    /// Files dropped on the window (`tryAddDroppedPaths`): what can go on the map does, the total is said.
    pub(crate) fn drop_paths(&mut self, paths: &[PathBuf]) {
        let (mut added, mut said) = (0_usize, None);
        for path in paths.iter().filter(|path| kind_of(path) != FileKind::Other) {
            match self.add_vector_from_path(path) {
                Ok(Some(_)) => added += 1,
                Ok(None) | Err(Refused::Unreadable) => {}
                Err(Refused::NotYet(words)) => said = Some(words),
            }
        }
        if added > 0 {
            self.active_tab = MainTab::Map;
            self.show_status_message(format!("레이어 {added}개 추가됨 (파일→지도)"));
        } else if let Some(words) = said {
            self.show_status_message(words);
        }
    }

    /// 「벡터·도면 불러오기」: the file it asks for (「SHP/벡터·도면 추가」, :2594-2596).
    pub(crate) fn open_vector_layer(&mut self) {
        let picked = rfd::FileDialog::new().set_title("SHP/벡터·도면 추가").add_filter("벡터·도면", &["shp", "gpkg", "geojson", "dxf", "dwg"]).pick_file();
        if let Some(path) = picked {
            self.open_vector_file(&path);
        }
    }

    /// The file picked with 「벡터·도면 불러오기」, with the words of `openVectorLayer`.
    pub(crate) fn open_vector_file(&mut self, path: &Path) {
        let words = match self.add_vector_from_path(path) {
            Ok(Some(added)) => format!("지도를 올렸습니다: {}", added.title),
            Ok(None) => return,
            Err(Refused::NotYet(words)) => words,
            Err(Refused::Unreadable) => format!("파일을 열지 못했습니다: {}", name_of(path)),
        };
        self.show_status_message(words);
    }

    /// A file opened in 파일함 (`fileActivated`).
    pub(crate) fn activate_drawer_file(&mut self, path: &Path) {
        let words = match self.add_vector_from_path(path) {
            Ok(Some(added)) => added.words,
            Ok(None) => return,
            Err(Refused::NotYet(words)) => words,
            Err(Refused::Unreadable) => DRAWER_CANNOT.to_owned(),
        };
        self.show_status_message(words);
    }

    /// `addVectorFromPath`: `Some` when a layer went on the map, `None` when the file is a survey and is opened.
    fn add_vector_from_path(&mut self, path: &Path) -> Result<Option<Added>, Refused> {
        let name = name_of(path);
        match kind_of(path) {
            FileKind::Shapefile => self.add_user_shapefile(path).map(Some),
            FileKind::GeoPackage if is_survey_file(path) => {
                self.request_open_survey(path.to_path_buf());
                Ok(None)
            }
            FileKind::GeoPackage => Err(Refused::NotYet(format!("GPKG 파일의 표는 아직 지도에 올리지 못합니다: {name}"))),
            FileKind::GeoJson => Err(Refused::NotYet(format!("GeoJSON 파일은 아직 지도에 올리지 못합니다: {name}"))),
            FileKind::Cad => Err(Refused::NotYet(format!("CAD 도면(DXF·DWG)은 아직 지도에 올리지 못합니다: {name}"))),
            FileKind::Picture => match self.add_raster_from_path(path) {
                Some((title, words)) => Ok(Some(Added { title, words })),
                None => Err(Refused::Unreadable),
            },
            FileKind::Other => Err(Refused::Unreadable),
        }
    }

    /// The row of a user shapefile, and the map on it when it lies on the work map (`layerSitsOnWorkMap`,
    /// `zoomToLayerMax`, :3010-3018).
    fn add_user_shapefile(&mut self, path: &Path) -> Result<Added, Refused> {
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let (spec, shapes, work_box) = user_shapefile(path, work_epsg)?;
        let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
        let title = self.free_layer_title(&stem, USER_PREFIX);
        self.show_user_vector_layer(&format!("{USER_PREFIX}{title}"), &title, spec);
        if let Some(found) = work_box.filter(|b| self.sits_on_work_map(*b)) {
            self.home_map.set_view_extent(found);
        }
        let words = format!("레이어 1개 추가 · 도형 {shapes}개 · 화면 {}", self.work_crs.auth_id());
        Ok(Added { title, words })
    }

    /// The name of the file, or the name with 「(2)」, 「(3)」… when a row has it: each row keeps its own check
    /// mark, which goes by the name.
    pub(crate) fn free_layer_title(&self, stem: &str, prefix: &str) -> String {
        let taken = |title: &str| self.left_layers.entries.iter().any(|e| e.label_ko == title || e.layer_key == format!("{prefix}{title}"));
        let mut title = stem.to_owned();
        let mut n = 2;
        while taken(&title) {
            title = format!("{stem} ({n})");
            n += 1;
        }
        title
    }

    /// Whether the box meets Korea in the work CRS (`LayerOps::koreaExtentForCrs`, BasemapOps.cpp:2406-2418).
    pub(crate) fn sits_on_work_map(&self, found: [f64; 4]) -> bool {
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let korea = Transformer::new(4326, work_epsg).and_then(|carry| carry.transform_box([124.5, 33.0, 132.0, 39.5]));
        korea.is_ok_and(|k| found[0] <= k[2] && found[2] >= k[0] && found[1] <= k[3] && found[3] >= k[1])
    }
}

#[cfg(test)]
#[path = "user_files_tests.rs"]
mod tests;
