//! Explicit source selection, target/cpp-ref/src/app/MainWindowAlign.cpp:844-876.
use crate::app_state::StrataAppState;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
type Picked = Result<Option<(PathBuf, strata_engine::PictureInfo)>, String>;

#[derive(Debug, Default)]
pub(crate) struct PickerSlot(pub Option<Receiver<Picked>>);
impl Clone for PickerSlot {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl StrataAppState {
    pub(crate) fn request_align_file(&mut self) {
        if self.align.picker.0.is_some() {
            return;
        }
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let path = if cfg!(test) { None } else { rfd::FileDialog::new().set_title("맞출 도면").add_filter("도면", &["png", "jpg", "jpeg", "tif", "tiff"]).pick_file() };
            let result = path.map(|file| strata_engine::picture_info(&file).map(|info| (file, info)).map_err(|why| why.0)).transpose();
            drop(send.send(result));
        });
        self.align.picker.0 = Some(receive);
    }
    pub(crate) fn poll_align_picker(&mut self, ctx: &eframe::egui::Context) {
        let Some(receive) = self.align.picker.0.take() else { return };
        match receive.try_recv() {
            Err(TryRecvError::Empty) => {
                self.align.picker.0 = Some(receive);
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
            Err(TryRecvError::Disconnected) | Ok(Ok(None)) => {}
            Ok(Err(why)) => self.show_status_message(format!("맞출 도면 파일을 열지 못했습니다. {why}")),
            Ok(Ok(Some((file, info)))) => {
                use crate::app_state::picture_layers::{looks_unreferenced, PictureLayer, PICTURE_PREFIX};
                let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let title = self.free_layer_title(&stem, PICTURE_PREFIX);
                let key = format!("{PICTURE_PREFIX}{title}");
                let epsg = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
                let transform = info.transform.unwrap_or([0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
                let layer = PictureLayer::new(key.clone(), file, &info, transform, info.epsg.filter(|code| *code != epsg), looks_unreferenced(&info), false);
                self.put_picture_row(layer, &title);
                self.right_inspector.selected_layer_key = Some(key.clone());
                self.start_align_session(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn align_without_a_selected_picture_requests_explicit_file_selection() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("정합");
        assert!(app.align.picker.0.is_some(), "C++ file selection was replaced by a guide with no action");
        assert!(app.align.raster.is_none());
    }
    #[test]
    fn chosen_real_picture_is_added_and_opens_a_pixel_map_session() {
        let temp = tempfile::tempdir().expect("temporary picture");
        let file = temp.path().join("picked.png");
        image::save_buffer(&file, &[240; 32 * 16 * 3], 32, 16, image::ColorType::Rgb8).expect("actual file");
        let info = strata_engine::picture_info(&file).expect("actual metadata");
        let mut app = StrataAppState::new_startup();
        let (send, receive) = std::sync::mpsc::channel();
        app.align.picker.0 = Some(receive);
        send.send(Ok(Some((file.clone(), info)))).expect("recorded selection boundary");
        app.poll_align_picker(&eframe::egui::Context::default());
        assert!(app.align.raster.as_ref().is_some_and(|s| s.original.file == file), "actual selection did not enter the session");
        assert!(app.pictures.layers.iter().any(|p| p.file == file && p.align_pending));
    }
}
