//! Invisible native test parent; the product passes its existing main window to start_run.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

pub fn attributes() -> WindowAttributes {
    WindowAttributes::default()
        .with_title("strata-heritage-run")
        .with_visible(false)
        .with_active(false)
}

pub fn matrix_succeeded(passed: u32, failed: u32, planned: usize) -> bool {
    passed > 0 && failed == 0 && usize::try_from(passed).ok() == Some(planned)
}

struct Host {
    runner: crate::runner::Runner,
    parent: Option<Window>,
    visibility_checks: u64,
    failure: Option<String>,
}

impl Host {
    fn check_hidden(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some(parent) = &self.parent else {
            return false;
        };
        let visible = parent.is_visible();
        self.visibility_checks += 1;
        if self.visibility_checks == 1 {
            println!("PARENT initial visible={visible:?}");
        }
        if visible != Some(false) {
            self.failure = Some(format!(
                "parent visibility is {visible:?}; expected Some(false)"
            ));
            event_loop.exit();
            return false;
        }
        true
    }
}

impl ApplicationHandler for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.parent.is_none() {
            match event_loop.create_window(attributes()) {
                Ok(parent) => self.parent = Some(parent),
                Err(error) => {
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
        }
        self.check_hidden(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, (): ()) {
        if !self.check_hidden(event_loop) {
            return;
        }
        if let Some(parent) = &self.parent {
            if self.runner.tick(parent) {
                event_loop.exit();
            }
        }
        self.check_hidden(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.check_hidden(event_loop);
        if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.check_hidden(event_loop);
    }
}

pub fn run(runner: crate::runner::Runner) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|error| error.to_string())?;
    let proxy = event_loop.create_proxy();
    let stop = Arc::new(AtomicBool::new(false));
    let timer_stop = Arc::clone(&stop);
    // The test event loop needs no rendering to drive the product's existing frame/pump API.
    // The C++ uses a timer too (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:101-104`).
    let timer = std::thread::spawn(move || {
        while !timer_stop.load(Ordering::Acquire) {
            if proxy.send_event(()).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
    let mut host = Host {
        runner,
        parent: None,
        visibility_checks: 0,
        failure: None,
    };
    let result = event_loop.run_app(&mut host);
    stop.store(true, Ordering::Release);
    let joined = timer.join();
    println!("PARENT hidden visibility checks={}", host.visibility_checks);
    result.map_err(|error| error.to_string())?;
    joined.map_err(|_| "event-loop timer panicked".to_owned())?;
    match host.failure {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parent_is_hidden_and_inactive_from_initial_creation() {
        let attributes = super::attributes();
        assert!(
            !attributes.visible,
            "test parent must be hidden before creation"
        );
        assert!(!attributes.active, "test parent must never activate");
    }

    #[test]
    fn an_interrupted_matrix_cannot_report_success() {
        assert!(!super::matrix_succeeded(4, 0, 10));
        assert!(!super::matrix_succeeded(0, 0, 0));
        assert!(!super::matrix_succeeded(9, 1, 10));
    }

    #[test]
    fn a_complete_filtered_case_can_report_success() {
        assert!(super::matrix_succeeded(1, 0, 1));
        assert!(super::matrix_succeeded(10, 0, 10));
    }
}
