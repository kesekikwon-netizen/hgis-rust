//! 「수치지형」 실제 도엽 확인 — C++ `KaTopographicBrowser` 6단계에서 **로그인 없이 실제
//! HTTP 로 되는 단계**를 Rust 로 옮긴 모듈이다.
//!
//! C++ 이 실제로 하는 일(`data/providers/ngii-topographic.js` 가 공식 화면 안에서 실행된다):
//!
//! | 단계 | C++ 동작 | 실제 HTTP | 로그인 필요 |
//! |---|---|---|---|
//! | 1 도엽 확인 | `combineBufferQuery.do` → `suchiQuery.do` (:43,:63) | **예** | 아니오 |
//! | 2 로그인 | 내장 브라우저가 `/mn/loginPopup.do` 등에서 아이디·비밀번호 입력 (:745-784) | 예 | **예** |
//! | 3 신청 | `window.downloadMapData(...)` 로 공식 신청 (:338-380) | 예 | **예** |
//! | 4 파일 준비 | `innorixdownLoad.do` 의 SDK 파일 목록 (:460-517) | 예 | **예** |
//! | 5 다운로드 | 공식 SDK `control.download(id)` (:509-514) | 예 | **예** |
//! | 6 지도에 올리기 | 받은 DXF 를 `sheetDownloaded` 로 적재 (:1046-1071) | 로컬 | 아니오 |
//!
//! Rust 에는 C++ 의 내장 브라우저(QWebEngine)가 없다(`KaTopographicBrowser.cpp:509-598`).
//! 그래서 1단계는 여기서 직접 실행하고, 2~5단계는 C++ 이 같은 상황에 내는 안내문
//! (`:152`,`:445-446`,`:214-215`)을 그대로 보인 뒤 6단계 「받은 자료 불러오기」로 넘긴다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui::{self, Color32, Stroke, Vec2};
use strata_net::ngii_topographic::{self, NgiiTopographicClient, SheetQuery};
use strata_types::crs::{CrsCode, WorkCrs};

use crate::app_state::StrataAppState;

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

/// 근처 도엽 번호의 후보 — C++ `MainWindowDownloads.cpp:179-182` 는
/// `TopographicSheets::select` 결과를 쓰고, 비면 `359011` 을 넣는다.
#[must_use]
pub fn nearby_sheet_numbers(center_wgs84: [f64; 2]) -> Vec<String> {
    // 중심점 주변 아주 작은 상자를 훑어 그 점이 속한 도엽을 얻는다.
    let [lon, lat] = center_wgs84;
    if !lon.is_finite() || !lat.is_finite() {
        return vec!["359011".to_owned()];
    }
    // C++ `TopographicSheets.cpp:101-105` — 경도 124°~°132° 밖은 도엽 계산을 하지 않는다.
    if !(124.0..132.0).contains(&lon) || !(0.0..90.0).contains(&lat) {
        // C++ `MainWindowDownloads.cpp:182`
        return vec!["359011".to_owned()];
    }
    let numbers =
        NgiiTopographicClient::sheets_for_bbox_25k([lon - 1e-6, lat - 1e-6, lon + 1e-6, lat + 1e-6]);
    if numbers.is_empty() {
        // C++ `MainWindowDownloads.cpp:182`
        vec!["359011".to_owned()]
    } else {
        numbers
    }
}

/// 작업 좌표계 중심 → EPSG:5179(포털 질의 좌표계). C++ `MainWindowDownloads.cpp:183-186`.
#[must_use]
pub fn portal_center(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let out = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg5179)
        .transform_xy(center[0], center[1]);
    (out[0].is_finite() && out[1].is_finite()).then_some(out)
}

/// 작업 좌표계 중심 → 위경도(도엽 번호 후보 계산용)
#[must_use]
pub fn center_wgs84(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let out = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg4326)
        .transform_xy(center[0], center[1]);
    (out[0].is_finite() && out[1].is_finite()).then_some(out)
}

fn start_lookup(app: &StrataAppState) {
    let Ok(mut state) = sheet_state().lock() else {
        return;
    };
    if state.running {
        return;
    }
    let work = app.home_map.real_survey.survey_center;
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

#[cfg(test)]
mod tests {
    use super::*;

    // C++ `MainWindowDownloads.cpp:179-182` — 후보가 비면 `359011`
    #[test]
    fn test_nearby_sheet_numbers_returns_cpp_fallback_when_out_of_range() {
        assert_eq!(nearby_sheet_numbers([f64::NAN, 0.0]), vec!["359011".to_owned()]);
        // 한국 밖(경도 0)은 1:25,000 격자 후보가 없다 → C++ 과 같은 기본값
        assert_eq!(nearby_sheet_numbers([0.0, 0.0]), vec!["359011".to_owned()]);
    }

    #[test]
    fn test_nearby_sheet_numbers_finds_gyeongju_sheet() {
        // 경주 월성 일대(129.0°E, 35.83°N) 근처 도엽
        let numbers = nearby_sheet_numbers([129.005, 35.834]);
        assert_eq!(numbers.len(), 1, "한 점은 도엽 한 장: {numbers:?}");
        let number = &numbers[0];
        assert_eq!(number.len(), 6, "{number}");
        let sheet = ngii_topographic::TopographicSheets::resolve(number).expect(number);
        assert!((sheet.west..=sheet.east + 1e-9).contains(&129.005), "{number}");
        assert!((sheet.south..=sheet.north + 1e-9).contains(&35.834), "{number}");
    }

    // C++ `MainWindowDownloads.cpp:183-186` — 포털 질의는 EPSG:5179
    #[test]
    fn test_portal_center_converts_work_crs_to_5179() {
        let out = portal_center([220_000.0, 360_000.0], WorkCrs::Epsg5187).expect("변환");
        assert!((800_000.0..1_200_000.0).contains(&out[0]), "{out:?}");
        assert!((1_400_000.0..2_100_000.0).contains(&out[1]), "{out:?}");
        assert!(portal_center([f64::INFINITY, 0.0], WorkCrs::Epsg5186).is_none());
    }

    #[test]
    fn test_center_wgs84_for_gyeongju_is_plausible() {
        let wgs = center_wgs84([220_000.0, 360_000.0], WorkCrs::Epsg5187).expect("변환");
        assert!((128.8..129.3).contains(&wgs[0]), "{wgs:?}");
        assert!((35.5..36.1).contains(&wgs[1]), "{wgs:?}");
    }
}
