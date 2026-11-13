//! 「수치지형」 도엽 확인 상태·스레드 폴링·화면 그리기.
//!
//! `ngii_topographic_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 상태 구조체와 그 필드를 쓰는 함수들이 함께 있어야 하므로 한 파일에 모았다.
//! 옮긴 항목의 이름·시그니처·동작은 그대로다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui::{self, Color32, Stroke, Vec2};
use strata_net::ngii_topographic::{self, NgiiTopographicClient, SheetQuery};

use crate::app_state::StrataAppState;

use super::ngii_topographic_geo::{center_wgs84, nearby_sheet_numbers, portal_center};

/// 1단계 도엽 확인 결과 상태
struct NgiiSheetState {
    running: bool,
    status: String,
    query: Option<SheetQuery>,
    requested: Vec<String>,
    cancel: Arc<AtomicBool>,
    receiver: Option<mpsc::Receiver<Result<SheetQuery, String>>>,
}

impl NgiiSheetState {
    fn idle() -> Self {
        Self {
            running: false,
            status: "「도엽 확인」을 누르면 조사 주변 5km의 1:25,000 DXF 도엽을 국토정보플랫폼에 묻습니다.".to_owned(),
            query: None,
            requested: Vec::new(),
            cancel: Arc::new(AtomicBool::new(false)),
            receiver: None,
        }
    }
}

fn sheet_state() -> &'static Mutex<NgiiSheetState> {
    static STATE: OnceLock<Mutex<NgiiSheetState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(NgiiSheetState::idle()))
}
fn start_lookup(app: &StrataAppState) {
    let Ok(mut state) = sheet_state().lock() else {
        return;
    };
    if state.running {
        return;
    }
    // KaTopographicScopePanel.cpp:507 — 지도 화면 가운데.
    let work = app.home_map.viewport.center_xy;
    let Some(portal) = portal_center(work, app.work_crs) else {
        state.status = "현재 위치를 국토지리정보원 검색 좌표로 변환하지 못했습니다.".to_owned();
        return;
    };
    let Some(wgs) = center_wgs84(work, app.work_crs) else {
        state.status = "현재 위치를 도엽 번호로 바꾸지 못했습니다.".to_owned();
        return;
    };
    let wanted = nearby_sheet_numbers(wgs);
    state.requested = wanted.clone();
    state.running = true;
    state.status = "국토정보플랫폼에 1:25,000 DXF 도엽을 묻고 있습니다.".to_owned();
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    state.cancel = cancel;
    state.receiver = Some(receiver);
    std::thread::spawn(move || {
        let result = ngii_topographic::fetch_official_sheets_25k(
            portal[0],
            portal[1],
            NgiiTopographicClient::SCOPE_RADIUS_M,
            &wanted,
        );
        let _ = sender.send(result);
    });
}

fn poll(ctx: &egui::Context) -> Option<String> {
    let mut state = sheet_state().lock().ok()?;
    let finished = state
        .receiver
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(result) = finished {
        state.running = false;
        state.receiver = None;
        match result {
            Ok(query) if query.products.is_empty() => {
                // C++ `KaTopographicBrowser.cpp:843`
                state.status = NgiiTopographicClient::unavailable_message().to_owned();
                state.query = Some(query);
            }
            Ok(query) => {
                // C++ `KaTopographicBrowser.cpp:870`
                state.status = query.order_sheets_text();
                state.query = Some(query);
            }
            Err(error) => {
                state.status = error;
                state.query = None;
            }
        }
    }
    if state.running {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
    state.query.as_ref().map(|_| state.status.clone())
}

/// 도엽 확인 줄 — 창 안에 넣는다. 돌려주는 값은 「상세 보기」 안내문 갱신용 로그.
pub fn render_sheet_lookup(app: &mut StrataAppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    let ready = poll(ctx);
    if let Some(status) = ready {
        if !app.modals.topo_browser_log.iter().any(|line| line == &status) {
            app.modals.topo_browser_log.push(status);
        }
    }
    let Ok(state) = sheet_state().lock() else {
        return;
    };
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn = egui::Button::new(
            egui::RichText::new("도엽 확인")
                .color(Color32::WHITE)
                .strong()
                .size(12.5),
        )
        .fill(Color32::from_rgb(44, 43, 41))
        .corner_radius(5.0)
        .min_size(Vec2::new(96.0, 30.0));
        clicked = ui
            .add_enabled(!state.running, btn)
            .on_hover_text(
                "국토정보플랫폼 공개 조회로 조사 주변 5km의 1:25,000 DXF 도엽을 확인합니다. 로그인은 필요 없습니다.",
            )
            .clicked();
        if state.running {
            ui.spinner();
        }
        ui.label(
            egui::RichText::new(&state.status)
                .size(11.5)
                .color(Color32::from_rgb(95, 94, 88)),
        );
    });
    if clicked {
        drop(state);
        start_lookup(app);
        return;
    }
    // 확인된 도엽 목록 — C++ `KaTopographicBrowser.cpp:442-443` `m_orderSheets`
    if let Some(query) = state.query.as_ref() {
        if !query.products.is_empty() {
            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(Color32::from_rgb(247, 245, 239))
                .stroke(Stroke::new(1.0, Color32::from_rgb(210, 207, 198)))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    for product in query.products.iter().take(8) {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {} · {} · {}",
                                product.num, product.name, product.scale, product.file_ext
                            ))
                            .size(11.5)
                            .color(Color32::from_rgb(60, 59, 55)),
                        );
                    }
                    if query.products.len() > 8 {
                        ui.label(
                            egui::RichText::new(format!("외 {}장", query.products.len() - 8))
                                .size(11.5)
                                .color(Color32::from_rgb(120, 118, 110)),
                        );
                    }
                    // C++ `KaTopographicBrowser.cpp:851` — 이미 다 보관 중
                    if query.products.len() <= query.unavailable.len() {
                        ui.label(
                            egui::RichText::new(NgiiTopographicClient::cached_message())
                                .size(11.5)
                                .color(Color32::from_rgb(21, 128, 61)),
                        );
                    }
                });
        }
    }
    // 로그인이 필요한 다음 단계는 C++ 이 내장 브라우저로 처리한다 — 같은 안내를 보인다.
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(NgiiTopographicClient::login_stage_blocked_message())
            .size(11.5)
            .color(Color32::from_rgb(120, 118, 110)),
    );
}

/// 확인된 도엽 수 (진행 표시용)
#[must_use]
pub fn confirmed_sheet_count() -> usize {
    sheet_state()
        .lock()
        .ok()
        .and_then(|state| state.query.as_ref().map(|q| q.products.len()))
        .unwrap_or(0)
}

/// 요청한 도엽 번호
#[must_use]
pub fn requested_sheet_numbers() -> Vec<String> {
    sheet_state()
        .lock()
        .ok()
        .map(|state| state.requested.clone())
        .unwrap_or_default()
}

/// 진행 중 작업 취소
pub fn cancel_lookup() {
    if let Ok(state) = sheet_state().lock() {
        state.cancel.store(true, Ordering::Relaxed);
    }
}
