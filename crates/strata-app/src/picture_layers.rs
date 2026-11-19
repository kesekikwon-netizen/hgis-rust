//! Pictures of the user on the map — C++ `MainWindow::addRasterFromPath` (MainWindow.cpp:2884-2937): a scanned plan,
//! a photo, a GeoTIFF. Each is a row of 주변·참조 (`markReferenceLayer`, :2904), drawn where its world file or its
//! GeoTIFF says (a turned picture too), read through GDAL at the size it is shown at (strata-engine picture.rs). A
//! picture without coordinates waits for 맞추기 with its row unchecked (`setAlignPending`, :2892-2895, :2910-2913);
//! the paper white of a GeoTIFF is see-through (`knockOutRasterPaper`, BasemapOps.cpp:193-215).

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use eframe::egui::TextureHandle;
use strata_engine::{picture_info, PictureInfo, Transformer};
use strata_types::crs::CrsCode;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;

/// The beginning of a picture row's `layer_key`.
pub(crate) const PICTURE_PREFIX: &str = "picture:";
/// The longest side of what is read for one view (a texture of this size is some 64 MB).
const MOST_SIDE: usize = 4096;
/// A colour within this of 255 in red, green and blue is paper (`TransparentThreeValuePixel(255, 255, 255, 0.0, 16,
/// 16, 16)`, BasemapOps.cpp:211).
const PAPER: u8 = 255 - 16;

/// The words of a picture without coordinates (MainWindow.cpp:2916-2917).
pub(crate) const UNREFERENCED: &str = "좌표 없는 그림입니다. 더보기 → 맞추기로 지적 위에 올리세요.";

/// The part of the picture a view needs: `window` is `[col, row, cols, rows]` of the file, read at `size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Wanted {
    pub(crate) window: [usize; 4],
    pub(crate) size: [usize; 2],
}

/// A picture on the map.
pub(crate) struct PictureLayer {
    pub(crate) key: String,
    pub(crate) file: PathBuf,
    pub(crate) size: [usize; 2],
    /// Pixel → the picture's CRS, GDAL's order.
    pub(crate) transform: [f64; 6],
    /// The picture's CRS when it is not the work CRS.
    pub(crate) source_epsg: Option<u32>,
    /// Without coordinates: it waits for 맞추기.
    pub(crate) align_pending: bool,
    /// A GeoTIFF of three bands or more: its paper white is see-through.
    pub(crate) knock_out_white: bool,
    /// `QgsBrightnessContrastFilter` sample offset, −255..255, 0 is the file (`BasemapOps.cpp:2316-2329`).
    pub(crate) brightness: i32,
    /// How many times the shown picture was dropped (`KaDrawingStudio.cpp:1211` `invalidateCache`).
    pub(crate) cache_epoch: u32,
    shown: Option<(Wanted, TextureHandle)>,
    reading: Option<Reading>,
}

/// A reading of a part of the picture in the background, and where its pixels arrive.
type Reading = (Wanted, Receiver<Result<Vec<u8>, String>>);

impl PictureLayer {
    pub(crate) fn clear_picture_cache(&mut self) {
        self.shown = None;
        self.reading = None;
        self.cache_epoch = self.cache_epoch.saturating_add(1);
    }
    pub(crate) fn new(key: String, file: PathBuf, info: &PictureInfo, transform: [f64; 6], source_epsg: Option<u32>, align_pending: bool, knock_out_white: bool) -> Self {
        Self {
            key,
            file,
            size: [info.width, info.height],
            transform,
            source_epsg,
            align_pending,
            knock_out_white,
            brightness: 0,
            cache_epoch: 0,
            shown: None,
            reading: None,
        }
    }
    /// A picture row with no file read yet, for the drawing opacity card.
    #[cfg(test)]
    pub(crate) fn fixture(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            file: PathBuf::from("picture.png"),
            size: [4, 4],
            transform: [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
            source_epsg: Some(5187),
            align_pending: false,
            knock_out_white: false,
            brightness: 0,
            cache_epoch: 0,
            shown: None,
            reading: None,
        }
    }

    /// Whether a part of it is on the screen now.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> Option<Wanted> {
        self.shown.as_ref().map(|(wanted, _)| *wanted)
    }
}

/// The pictures of the open survey.
#[derive(Default)]
pub(crate) struct PictureLayers {
    pub(crate) layers: Vec<PictureLayer>,
}

/// A copy of the app state knows which pictures there are; what is shown stays with the original.
impl Clone for PictureLayers {
    fn clone(&self) -> Self {
        let layers = self
            .layers
            .iter()
            .map(|l| PictureLayer {
                key: l.key.clone(),
                file: l.file.clone(),
                size: l.size,
                transform: l.transform,
                source_epsg: l.source_epsg,
                align_pending: l.align_pending,
                knock_out_white: l.knock_out_white,
                brightness: l.brightness,
                cache_epoch: 0,
                shown: None,
                reading: None,
            })
            .collect();
        Self { layers }
    }
}

impl std::fmt::Debug for PictureLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.layers.iter().map(|l| &l.key)).finish()
    }
}

/// Where a pixel lies: GDAL's transform.
pub(crate) fn place(t: &[f64; 6], col: f64, row: f64) -> [f64; 2] {
    [t[0] + col * t[1] + row * t[2], t[3] + col * t[4] + row * t[5]]
}

/// The pixel at a point, the transform turned round; none when the transform is flat.
pub(crate) fn pixel_at(t: &[f64; 6], xy: [f64; 2]) -> Option<[f64; 2]> {
    let det = t[1] * t[5] - t[2] * t[4];
    if det.abs() < 1e-18 {
        return None;
    }
    let (dx, dy) = (xy[0] - t[0], xy[1] - t[3]);
    Some([(t[5] * dx - t[2] * dy) / det, (t[1] * dy - t[4] * dx) / det])
}

/// C++ `GeorefService::looksUnreferencedRaster` (GeorefService.cpp:612-631): a picture that sits on its own pixel
/// plane (no transform, or one that keeps it within 0 … width, 0 … height) has no coordinates.
pub(crate) fn looks_unreferenced(info: &PictureInfo) -> bool {
    let (w, h) = (info.width as f64, info.height as f64);
    if info.width < 2 || info.height < 2 {
        return true;
    }
    let Some(t) = info.transform else { return true };
    let corners = [place(&t, 0.0, 0.0), place(&t, w, 0.0), place(&t, w, h), place(&t, 0.0, h)];
    let (min_x, max_x) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), c| (lo.min(c[0]), hi.max(c[0])));
    let (min_y, max_y) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), c| (lo.min(c[1]), hi.max(c[1])));
    if !(min_x.is_finite() && max_y.is_finite()) || max_x - min_x <= 0.0 || max_y - min_y <= 0.0 {
        return true;
    }
    if min_x > -2.0 && min_y > -2.0 && max_x < w + 2.0 && max_y < h + 2.0 {
        return true;
    }
    // A box of about one unit a pixel is a raw scan only near the pixel origin (1 m pixels in EPSG:5186 are not).
    ((max_x - min_x) - w).abs() < 2.0 && ((max_y - min_y) - h).abs() < 2.0 && min_x < w + 10.0 && min_y < h + 10.0
}

/// The paper white of a picture made see-through.
pub(crate) fn knock_out_paper(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0.iter_mut().filter(|p| p[0] >= PAPER && p[1] >= PAPER && p[2] >= PAPER) {
        pixel[3] = 0;
    }
}

/// The layer of a picture file; with whether it has no coordinates. `None` when GDAL cannot open it.
pub(crate) fn picture_layer_of(path: &Path, key: String, work_epsg: u32) -> Option<(PictureLayer, PictureInfo)> {
    let info = picture_info(path).ok()?;
    let unreferenced = looks_unreferenced(&info);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let geotiff = matches!(ext.as_str(), "tif" | "tiff" | "gtiff");
    // Without coordinates it sits on its pixel plane until 맞추기 (GDAL's own transform for such a file).
    let transform = info.transform.unwrap_or([0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    let source_epsg = info.epsg.filter(|epsg| *epsg != work_epsg);
    let layer = PictureLayer::new(key, path.to_path_buf(), &info, transform, source_epsg, unreferenced, geotiff && info.bands >= 3);
    Some((layer, info))
}

impl PictureLayer {
    /// The box of the whole picture in the work CRS.
    pub(crate) fn work_box(&self, work_epsg: u32) -> Option<[f64; 4]> {
        let (w, h) = (self.size[0] as f64, self.size[1] as f64);
        let corners = [
            place(&self.transform, 0.0, 0.0),
            place(&self.transform, w, 0.0),
            place(&self.transform, w, h),
            place(&self.transform, 0.0, h),
        ];
        let fold = |pick: fn(&[f64; 2]) -> f64| corners.iter().map(pick).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
        let ((x0, x1), (y0, y1)) = (fold(|c| c[0]), fold(|c| c[1]));
        let source = [x0, y0, x1, y1];
        match self.source_epsg {
            Some(epsg) if epsg != work_epsg => Transformer::new(epsg, work_epsg).and_then(|carry| carry.transform_box(source)).ok(),
            _ => Some(source),
        }
    }
}

impl StrataAppState {
    /// `addRasterFromPath` (MainWindow.cpp:2884-2937): the row of a picture at the top of 주변·참조, unchecked while it
    /// waits for 맞추기; the map goes to a GeoTIFF on the work map. The title and the words it leaves on the status
    /// line; `None` when GDAL cannot open the file.
    pub(crate) fn add_raster_from_path(&mut self, path: &Path) -> Option<(String, String)> {
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let stem = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
        let title = self.free_layer_title(&stem, PICTURE_PREFIX);
        let key = format!("{PICTURE_PREFIX}{title}");
        let (layer, info) = picture_layer_of(path, key.clone(), work_epsg)?;
        let (unreferenced, geotiff_box) = (layer.align_pending, layer.knock_out_white.then(|| layer.work_box(work_epsg)).flatten());
        self.put_picture_row(layer, &title);
        if unreferenced {
            return Some((title, UNREFERENCED.to_owned()));
        }
        if let Some(found) = geotiff_box.filter(|b| self.sits_on_work_map(*b)) {
            self.home_map.set_view_extent(found);
        }
        let crs = info.epsg.map_or_else(|| self.work_crs.auth_id().to_string(), |epsg| format!("EPSG:{epsg}"));
        let words = format!("그림 추가: {title} · {crs} · 화면 {}", self.work_crs.auth_id());
        Some((title, words))
    }

    /// The layer and its row at the top of 주변·참조, checked unless the picture waits for 맞추기.
    pub(crate) fn put_picture_row(&mut self, layer: PictureLayer, title: &str) {
        let (key, waiting) = (layer.key.clone(), layer.align_pending);
        self.pictures.layers.retain(|l| l.key != key);
        self.pictures.layers.push(layer);
        if !self.left_layers.entries.iter().any(|e| e.layer_key == key) {
            let entries = &self.left_layers.entries;
            let at = entries
                .iter()
                .position(|e| matches!(e.group, LegendGroup::ReferenceMap | LegendGroup::Basemap))
                .unwrap_or(entries.len());
            let row = LayerTreeEntry {
                layer_key: key,
                label_ko: title.to_owned(),
                group: LegendGroup::ReferenceMap,
                visible: !waiting,
                opacity: 1.0,
                read_only: true,
                snap_enabled: false,
                feature_count: 0,
            };
            self.left_layers.entries.insert(at, row);
        }
        self.home_map.real_survey.layer_visible.insert(title.to_owned(), !waiting);
        self.is_dirty = true;
    }

    /// Rows of an earlier survey do not belong to the next one.
    pub(crate) fn remove_picture_layers(&mut self) {
        self.pictures.layers.clear();
        self.left_layers.entries.retain(|e| !e.layer_key.starts_with(PICTURE_PREFIX));
    }
}

#[cfg(test)]
#[path = "picture_layers_tests.rs"]
mod tests;

#[path = "picture_layers_paint.rs"]
mod paint;

#[path = "picture_layers_read.rs"]
mod read;

#[path = "picture_layers_pixels.rs"]
pub(crate) mod pixels;
