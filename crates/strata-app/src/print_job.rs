//! Captured sheet to background device output, target/cpp-ref/src/app/KaPrintDialog.cpp:640-669.
use super::print_device::{PrintReceipt, PrintRequest};
use crate::app_state::StrataAppState;
use crate::gui_tab_drawing::sheet_scene::{RasterInput, SheetScene};
use eframe::egui;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, TryRecvError},
    Arc,
};

pub(crate) type Dispatch = Box<dyn FnOnce(PrintRequest, RasterInput, Arc<AtomicBool>) -> Result<PrintReceipt, String> + Send>;

struct Captured {
    scene: SheetScene,
    app: Box<StrataAppState>,
    request: PrintRequest,
    dispatch: Dispatch,
}

enum Stage {
    Rendering(Box<Captured>),
    Sending(Receiver<Result<PrintReceipt, String>>),
}

pub(crate) struct PrintJob {
    cancel: Arc<AtomicBool>,
    stage: Stage,
}

#[derive(Default)]
pub(crate) struct PrintJobSlot(pub(crate) Option<Box<PrintJob>>);

impl Clone for PrintJobSlot {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for PrintJobSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PrintJob").field(&self.0.is_some()).finish()
    }
}

impl Drop for PrintJobSlot {
    fn drop(&mut self) {
        if let Some(job) = &self.0 {
            job.cancel.store(true, Ordering::Release);
        }
    }
}

impl PrintJobSlot {
    pub(crate) fn cancel(&self) {
        if let Some(job) = &self.0 {
            job.cancel.store(true, Ordering::Release);
        }
    }
    pub(crate) fn words(&self) -> Option<&'static str> {
        self.0.as_ref().map(|job| match job.stage {
            Stage::Rendering(_) => "도면을 정리하는 중…",
            Stage::Sending(_) => "프린터로 보내는 중…",
        })
    }
}

impl StrataAppState {
    pub(crate) fn start_device_print_using(&mut self, dispatch: Dispatch) -> Result<(), String> {
        if self.printing.job.0.is_some() {
            return Err("프린터로 보내는 중…".to_owned());
        }
        let paper = [self.drawing.sheet.width_mm, self.drawing.sheet.height_mm];
        let request = self.printing.request(&self.drawing.title_ko, paper)?;
        let pixels = request.plan.paper_mm[0] * request.plan.paper_mm[1] * (f64::from(request.dpi) / 25.4).powi(2);
        if pixels > 100_000_000.0 {
            return Err("도면이나 용지 크기를 알 수 없습니다.".to_owned());
        }
        let scene = SheetScene::new(paper, self.drawing.screen_px_per_mm.unwrap_or(1.65), request.dpi as f32);
        let captured = Captured { scene, app: Box::new(self.clone()), request, dispatch };
        self.printing.job.0 = Some(Box::new(PrintJob { cancel: Arc::new(AtomicBool::new(false)), stage: Stage::Rendering(Box::new(captured)) }));
        Ok(())
    }

    pub(crate) fn advance_device_print(&mut self, ctx: &egui::Context) {
        let Some(job) = self.printing.job.0.take() else { return };
        let PrintJob { cancel, stage } = *job;
        if cancel.load(Ordering::Acquire) {
            self.show_status_message("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.");
            return;
        }
        let next = match stage {
            Stage::Rendering(mut captured) => {
                captured.scene.pass(&mut captured.app);
                if captured.scene.ready() {
                    let raster = captured.scene.raster();
                    let Captured { request, dispatch, .. } = *captured;
                    let (send, receive) = channel();
                    let worker_cancel = Arc::clone(&cancel);
                    std::thread::spawn(move || {
                        let result = if worker_cancel.load(Ordering::Acquire) {
                            Err("인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.".to_owned())
                        } else {
                            dispatch(request, raster, worker_cancel)
                        };
                        drop(send.send(result));
                    });
                    Some(Stage::Sending(receive))
                } else {
                    Some(Stage::Rendering(captured))
                }
            }
            Stage::Sending(receive) => match receive.try_recv() {
                Ok(Ok(receipt)) => {
                    self.show_status_message(format!("프린터로 {}쪽을 보냈습니다.", receipt.pages));
                    None
                }
                Ok(Err(why)) => {
                    self.show_status_message(format!("인쇄하지 못했습니다 · {why}"));
                    None
                }
                Err(TryRecvError::Empty) => Some(Stage::Sending(receive)),
                Err(TryRecvError::Disconnected) => {
                    self.show_status_message("인쇄하지 못했습니다");
                    None
                }
            },
        };
        if let Some(stage) = next {
            self.printing.job.0 = Some(Box::new(PrintJob { cancel, stage }));
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

#[cfg(test)]
#[path = "print_job_tests.rs"]
mod tests;
