//! Real raster actions, target/cpp-ref/src/app/MainWindowAlign.cpp:627-880.
use super::gui_ribbon_align::{
    raster::{Outcome, Purpose, RasterSession, Task},
    AlignSessionState,
};
use crate::app_state::StrataAppState;
use crate::ribbon::SubToolsMode;
use strata_canvas2d::map_tools::MapToolKind;

impl StrataAppState {
    pub(crate) fn apply_align_move(&mut self) -> Result<(), String> {
        if self.align.pairs.len() < 2 {
            return Err("점을 2개 이상 찍은 뒤 「이동」을 누르세요".to_owned());
        }
        let fit = self.align.report().affine.ok_or("점 배치로 변환을 만들 수 없습니다")?;
        let session = self.align.raster.as_mut().ok_or("맞출 도면이 없습니다")?;
        if session.task.0.is_some() {
            return Err("도면을 정리하는 중…".to_owned());
        }
        let key = session.original.key.clone();
        let expected_file = self.pictures.layers.iter().find(|l| l.key == key).ok_or("맞출 도면이 없습니다")?.file.clone();
        let source = session.original.file.clone();
        let root = self
            .survey_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .or_else(|| source.parent().map(std::path::Path::to_path_buf))
            .ok_or("백업 폴더가 없습니다")?
            .join("정합백업");
        let work_epsg = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = strata_engine::create_aligned_raster(&source, &root, [fit.c, fit.a, fit.b, fit.f, fit.d, fit.e], work_epsg)
                .map(|out| Outcome::Applied(out, fit))
                .map_err(|why| why.0);
            if let Err(std::sync::mpsc::SendError(Ok(Outcome::Applied(out, _)))) = send.send(result) {
                drop(out.restore());
            }
        });
        session.task.0 = Some(Task { receive, expected_file, work_epsg, pairs: self.align.pairs.clone(), purpose: Purpose::Apply { save: false } });
        self.align.applied = false;
        Ok(())
    }

    pub(crate) fn restore_align_originals(&mut self) -> (bool, String) {
        let Some(session) = &mut self.align.raster else { return (true, "되돌릴 맞추기가 없습니다".to_owned()) };
        if session.task.0.is_some() {
            return (false, "도면을 정리하는 중…".to_owned());
        }
        let Some(out) = session.result.clone() else {
            self.align.pairs.clear();
            self.align.pending_source = None;
            self.align.applied = false;
            self.align.transform = None;
            return (true, "찍은 점을 지웠습니다. 그림 파일은 아직 바뀌지 않았습니다".to_owned());
        };
        let expected_file = out.file().to_path_buf();
        let work_epsg = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let source = session.original.file.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = strata_engine::picture_info(&source).map_err(|why| why.0);
            drop(send.send(result.map(|_| Outcome::Restored)));
        });
        session.task.0 = Some(Task { receive, expected_file, work_epsg, pairs: self.align.pairs.clone(), purpose: Purpose::Restore });
        (true, "도면을 정리하는 중…".to_owned())
    }

    pub(crate) fn save_align_result(&mut self) -> (String, String) {
        if let Some(task) = self.align.raster.as_mut().and_then(|session| session.task.0.as_mut()) {
            if let Purpose::Apply { save } = &mut task.purpose {
                *save = true;
            }
            return (String::new(), "도면을 정리하는 중…".to_owned());
        }
        if self.align.pairs.len() < 2 && !self.align.applied {
            return ("맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요.".to_owned(), "점을 2곳 이상 찍은 뒤 저장하세요.".to_owned());
        }
        if self.align.applied {
            let path = self.align.saved_path.clone().unwrap_or_default();
            self.stop_align_session();
            return ("맞춰 두었습니다. 이 도면은 참고용이니 제출할 구역은 그리기로 직접 그리세요.".to_owned(), format!("맞춤 저장: {path}"));
        }
        match self.apply_align_move() {
            Ok(()) => {
                if let Some(task) = self.align.raster.as_mut().and_then(|s| s.task.0.as_mut()) {
                    task.purpose = Purpose::Apply { save: true };
                }
                (String::new(), "도면을 정리하는 중…".to_owned())
            }
            Err(why) => (format!("도면을 옮기지 못했습니다 — {why}"), why),
        }
    }

    pub(crate) fn stop_align_session(&mut self) {
        if self.align.raster.as_ref().is_some_and(|session| session.result.is_none()) { self.restore_picture_placement(); }
        let pending_view = self.align.pending_view.take();
        self.align.end_session();
        self.align.pending_view = pending_view;
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
        if self.active_tool == MapToolKind::GeorefAlign {
            self.active_tool = MapToolKind::Pan;
        }
    }

    pub(crate) fn alignable_layer_name(&self) -> Option<String> {
        let key = self.right_inspector.selected_layer_key.as_deref()?;
        self.pictures.layers.iter().find(|l| l.key == key).map(|l| l.key.clone())
    }

    pub(crate) fn start_align_session(&mut self, target: &str) {
        self.stop_align_session();
        let Some(raster) = RasterSession::capture(self, target) else {
            self.show_status_message("JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요.");
            return;
        };
        let key = raster.original.key.clone();
        self.align = AlignSessionState { target_layer: Some(key.clone()), raster: Some(raster), ..Default::default() };
        if let Some(layer) = self.pictures.layers.iter_mut().find(|layer| layer.key == key) { layer.align_pending = true; }
        self.active_tab = crate::app_state::MainTab::Map;
        self.ribbon.toggle_sub_mode(SubToolsMode::Align);
        self.active_tool = MapToolKind::GeorefAlign;
        self.status_bar_hint = self.align.status_text();
    }

    pub(crate) fn hide_sub_tools(&mut self) {
        if self.ribbon.sub_mode == SubToolsMode::Align {
            self.stop_align_session();
            return;
        }
        if self.ribbon.sub_mode == SubToolsMode::Draw {
            self.stop_capture_tool();
        }
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
    }
}

#[cfg(test)]
#[path = "align_raster_workflow_tests.rs"]
mod raster_tests;
