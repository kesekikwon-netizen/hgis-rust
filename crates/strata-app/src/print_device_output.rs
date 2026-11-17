//! Background image/request handoff, target/cpp-ref/src/app/KaPrintDialog.cpp:640-669.
use super::{validate_receipt, PrintReceipt, PrintRequest};
use crate::gui_tab_drawing::sheet_raster::Picture;
use crate::gui_tab_drawing::sheet_scene::RasterInput;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[derive(Serialize)]
struct Payload<'a> {
    version: u32,
    image: &'a Path,
    request: &'a PrintRequest,
    page_images: &'a [PathBuf],
}

struct Files {
    directory: PathBuf,
    image: PathBuf,
    request: PathBuf,
    output: Option<PathBuf>,
    pages: Vec<PathBuf>,
}

impl Files {
    fn new() -> Result<Self, String> {
        let directory = std::env::temp_dir().join(format!("strata-print-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).map_err(|why| why.to_string())?;
        Ok(Self { image: directory.join("sheet.png"), request: directory.join("request.json"), directory, output: None, pages: Vec::new() })
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        // Remove only these owned files and an empty directory, never recursive user paths.
        drop(std::fs::remove_file(&self.image));
        drop(std::fs::remove_file(&self.request));
        for path in &self.pages {
            drop(std::fs::remove_file(path));
        }
        if let Some(path) = &self.output {
            drop(std::fs::remove_file(path));
        }
        drop(std::fs::remove_dir(&self.directory));
    }
}

pub(crate) fn dispatch(mut request: PrintRequest, raster: RasterInput, cancel: Arc<AtomicBool>) -> Result<PrintReceipt, String> {
    #[cfg(test)]
    if request.output.is_none() {
        return Err("Physical printer dispatch is forbidden in tests".to_owned());
    }
    let mut files = Files::new()?;
    let target = request.output.as_deref().map(std::path::absolute).transpose().map_err(|why| why.to_string())?;
    if let Some(target) = &target {
        if target.exists() {
            return Err("Print output already exists; choose another file".to_owned());
        }
        let parent = target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let staged = parent.join(format!(".strata-print-{}.{}", uuid::Uuid::new_v4(), target.extension().and_then(|v| v.to_str()).unwrap_or("prn")));
        std::fs::OpenOptions::new().create_new(true).write(true).open(&staged).map_err(|why| why.to_string())?;
        request.output = Some(staged.clone());
        files.output = Some(staged);
    }
    write_picture(&files.image, raster.thumbnail())?;
    for (index, page) in request.plan.pages.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.".to_owned());
        }
        let path = files.directory.join(format!("tile-{index:04}.png"));
        files.pages.push(path.clone());
        write_picture(&path, raster.region_mm(page.source_mm, (f64::from(request.dpi) * request.plan.ratio) as f32))?;
    }
    let json = serde_json::to_vec(&Payload { version: 1, image: &files.image, request: &request, page_images: &files.pages }).map_err(|why| why.to_string())?;
    std::fs::write(&files.request, json).map_err(|why| why.to_string())?;
    if cancel.load(Ordering::Acquire) {
        return Err("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.".to_owned());
    }
    let bytes = super::print_device_process::run("Print", Some(&files.request), &cancel)?;
    let receipt = validate_receipt(&request, &bytes)?;
    if let (Some(staged), Some(target)) = (&files.output, target) {
        commit_output(staged, &target, &cancel)?;
    }
    Ok(receipt)
}

#[cfg(test)]
pub(super) fn request_bytes(request: &PrintRequest, image: &Path) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&Payload { version: 1, image, request, page_images: &[] }).map_err(|why| why.to_string())
}

fn write_picture(path: &Path, picture: Picture) -> Result<(), String> {
    if picture.unknown_pictures > 0 {
        return Err("도면 그림을 만들지 못했습니다.".to_owned());
    }
    let [width, height] = picture.size.map(u32::try_from);
    let (width, height) = (width.map_err(|why| why.to_string())?, height.map_err(|why| why.to_string())?);
    image::save_buffer(path, &picture.rgb, width, height, image::ColorType::Rgb8).map_err(|why| why.to_string())
}

fn commit_output(staged: &Path, target: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut stable = None;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.".to_owned());
        }
        if target.exists() {
            return Err("Print output already exists; original file was preserved".to_owned());
        }
        if let Ok(meta) = std::fs::metadata(staged) {
            let stamp = (meta.len(), meta.modified().ok());
            if meta.len() > 0 && stable == Some(stamp) && std::fs::rename(staged, target).is_ok() {
                return Ok(());
            }
            stable = Some(stamp);
        }
        if Instant::now() >= deadline {
            return Err("Printer output did not finish writing".to_owned());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
