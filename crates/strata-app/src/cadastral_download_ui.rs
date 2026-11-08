//! 「지적」 실제 내려받기 — C++ `MainWindowCadastral.cpp:82-174` `downloadCadastral()` 과
//! `core/CadastralPortal.cpp` 의 파이프라인을 Rust 로 옮긴 창이다.
//!
//! C++ 이 실제로 받아오는 것(`CadastralPortal.cpp:204-341`):
//! 1. 조사구역 + 작업 좌표계 → 5km 범위(:222-224)
//! 2. `LT_C_ADSIGG_INFO` GetFeature 로 범위에 걸친 **시·군·구 목록**(:232-262)
//! 3. VWorld 계정 로그인(:264-292) — 아이디·비밀번호는 VWorld 누리집 계정(API 키 아님)
//! 4. 자료 목록 HTML 에서 `LSMD_CONT_LDREG_*.zip` 찾기(:293-303)
//! 5. 시·군·구마다 ZIP 하나를 `지적도/원본` 에 내려받기(:306-337)
//! 6. SHP 를 지도용으로 정리(:338-341, `CadastralImport::prepare`)
//!
//! 여기서는 **로그인 없이 실제로 되는 단계**를 그대로 실행한다: 키 확인 → 시·군·구 목록
//! → 자료 목록 → ZIP 내려받기. 로그인이 필요한 단계는 C++ 이 내는 문구
//! (`CadastralPortal.cpp:214`·`:286-289`) 그대로 안내한다. 새 크레이트 없이 기존
//! `address_search.rs:758` 의 curl 방식을 쓴다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};
use strata_net::vworld_client::{self, VworldClient};
use strata_types::crs::{CrsCode, WorkCrs};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

/// C++ `CadastralPortal.cpp:232-236` — 시·군·구 목록 한 쪽에 100건
const CADASTRAL_CATALOG_PAGE_SIZE: u32 = 100;
/// C++ `CadastralPortal.cpp:231` — 자료 목록 최대 10쪽
const CADASTRAL_CATALOG_MAX_PAGES: u32 = 10;

struct CadastralState {
    running: bool,
    progress: f32,
    status: String,
    cancel: Arc<AtomicBool>,
    progress_slot: Arc<Mutex<f32>>,
    receiver: Option<mpsc::Receiver<CadastralOutcome>>,
    /// 받은 ZIP 경로 (C++ `CadastralPortal.cpp:308` `sources`)
    downloaded: Vec<String>,
}

/// 지적 내려받기 결과
#[derive(Debug, Clone, Default)]
pub struct CadastralOutcome {
    pub ok: bool,
    pub downloaded: Vec<String>,
    pub districts: Vec<String>,
    pub error: String,
}

impl CadastralState {
    fn idle() -> Self {
        Self {
            running: false,
            progress: 0.0,
            status: "조사구역 경계에서 주변 5km의 시·군·구 지적도를 준비합니다.".to_owned(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
            receiver: None,
            downloaded: Vec::new(),
        }
    }
}

fn cadastral_state() -> &'static Mutex<CadastralState> {
    static STATE: OnceLock<Mutex<CadastralState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(CadastralState::idle()))
}

/// C++ `MainWindowCadastral.cpp:121` — 조사 폴더 안 `지적도/원본`
#[must_use]
pub fn cadastral_raw_dir(survey_path: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    let profile = survey_path?;
    let parent = profile.parent().unwrap_or_else(|| std::path::Path::new("."));
    Some(
        parent
            .join(vworld_client::CADASTRAL_FOLDER_NAME)
            .join(vworld_client::CADASTRAL_RAW_FOLDER_NAME),
    )
}

/// C++ `CadastralPortal.cpp:312` — ZIP 크기가 자동 수신 범위를 벗어났을 때
#[must_use]
pub fn cadastral_zip_size_error_ko() -> &'static str {
    "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다."
}

/// 지적이 실제로 하는 일을 한 줄로 알린다 (C++ `MainWindowCadastral.cpp:168` 완료 안내 참고)
#[must_use]
pub fn cadastral_success_message_ko(zip_count: usize, district_count: usize) -> String {
    format!(
        "조사 주변 5km 지적도를 추가했습니다. 시·군·구 {district_count}곳의 원본 ZIP {zip_count}개를 받았습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."
    )
}

/// 시·군·구 목록 파싱 — `LT_C_ADSIGG_INFO` GetFeature 응답(C++ `CadastralPortal.cpp:249-258`)
pub fn parse_sigungu_features(body: &[u8]) -> Result<Vec<vworld_client::CadastralDistrict>, String> {
    let root: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| cadastral_district_error_ko().to_owned())?;
    let features = root
        .get("response")
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("featureCollection"))
        .and_then(|c| c.get("features"));
    let Some(features) = features.and_then(serde_json::Value::as_array) else {
        return Err(cadastral_district_error_ko().to_owned());
    };
    let mut out: Vec<vworld_client::CadastralDistrict> = Vec::new();
    for feature in features {
        let props = feature.get("properties").unwrap_or(&serde_json::Value::Null);
        let code = match props.get("sig_cd") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let name = props
            .get("full_nm")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        // C++ `:255` — 5자리 코드와 이름이 모두 있어야 한다.
        if code.len() != 5 || name.is_empty() {
            return Err(cadastral_district_error_ko().to_owned());
        }
        if !out.iter().any(|d| d.code == code) {
            out.push(vworld_client::CadastralDistrict { code, name });
        }
    }
    if out.is_empty() {
        return Err(cadastral_district_error_ko().to_owned());
    }
    Ok(out)
}

/// C++ `CadastralPortal.cpp:242` — 시·군·구 경계를 확인하지 못했을 때
#[must_use]
pub fn cadastral_district_error_ko() -> &'static str {
    "VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요."
}

/// 작업 좌표계 중심을 EPSG:5179(공식 신청 좌표계)로 옮긴다 — C++ `MainWindowDownloads.cpp:183-186`
#[must_use]
pub fn work_to_portal_xy(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
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

/// 내려받기 작업자 — 키 확인 → 시·군·구 → 자료 목록 → ZIP
fn run_download(
    survey_dir: std::path::PathBuf,
    center_work: [f64; 2],
    work_crs: WorkCrs,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<f32>>,
) -> CadastralOutcome {
    let mut outcome = CadastralOutcome::default();
    let report = |fraction: f32, progress: &Arc<Mutex<f32>>| {
        if let Ok(mut slot) = progress.lock() {
            *slot = fraction;
        }
    };

    // C++ `CadastralPortal.cpp:214` — API 키가 없으면 여기서 멈춘다.
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let key = key.trim().to_owned();
    if key.is_empty() {
        outcome.error = VworldClient::NO_KEY_MESSAGE.to_owned();
        return outcome;
    }
    if cancel.load(Ordering::Relaxed) {
        outcome.error = "지적도 받기를 취소했습니다.".to_owned();
        return outcome;
    }

    // C++ `:222-228` — 조사구역 + 5km 를 WGS84 사각 범위로
    let Some(work_box) = cadastral_work_box(center_work, work_crs) else {
        outcome.error = "조사 주변 5km 범위를 만들지 못했습니다.".to_owned();
        return outcome;
    };
    report(0.05, &progress);

    // C++ `:231-262` — 범위에 걸친 시·군·구 목록
    let mut districts: Vec<vworld_client::CadastralDistrict> = Vec::new();
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        let url = VworldClient::districts_url(&key, work_box);
        let url = with_page(&url, page);
        let Ok(bytes) = vworld_client::fetch_bytes(&url) else {
            outcome.error = cadastral_district_error_ko().to_owned();
            return outcome;
        };
        match parse_sigungu_features(&bytes) {
            Ok(rows) => {
                let before = districts.len();
                for row in rows {
                    if !districts.iter().any(|d| d.code == row.code) {
                        districts.push(row);
                    }
                }
                // C++ `:260` — 더 받을 쪽이 없으면 끝
                if districts.len() == before {
                    break;
                }
            }
            Err(_) if page == 1 => {
                // 첫 쪽에서 아무 시·군·구도 못 받으면 C++ 처럼 오류로 끝낸다.
                outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
                return outcome;
            }
            Err(_) => break,
        }
        report(0.05 + 0.15 * f32::from(u8::try_from(page).unwrap_or(1)) / 10.0, &progress);
    }
    if districts.is_empty() {
        outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
        return outcome;
    }
    outcome.districts = districts.iter().map(|d| d.name.clone()).collect();
    report(0.25, &progress);

    // C++ `:293-303` — 자료 목록(ZIP 이름·크기·갱신일)
    let mut resources: Vec<vworld_client::CadastralResource> = Vec::new();
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        let Ok(html) = vworld_client::fetch_cadastral_catalog_page(page, CADASTRAL_CATALOG_PAGE_SIZE)
        else {
            // 로그인 없이 받은 화면에는 ZIP 목록이 없다 — C++ `:264-292` 가 로그인을
            // 먼저 하기 때문이다. 그 사유를 C++ 문구로 알린다.
            outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
            return outcome;
        };
        let rows = vworld_client::parse_cadastral_resources(&html);
        let short_page = rows.len() < CADASTRAL_CATALOG_PAGE_SIZE as usize;
        resources.extend(rows);
        if short_page {
            break;
        }
    }
    if resources.is_empty() {
        outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
        return outcome;
    }
    report(0.4, &progress);

    // C++ `:304-305` — 시·군·구마다 정확히 한 ZIP
    let files = match vworld_client::select_cadastral_resources(&resources, &districts) {
        Ok(files) => files,
        Err(error) => {
            outcome.error = error;
            return outcome;
        }
    };
    // C++ `:306-307`
    if std::fs::create_dir_all(&survey_dir).is_err() {
        outcome.error = "조사 폴더에 지적도 저장 위치를 만들지 못했습니다.".to_owned();
        return outcome;
    }
    // C++ `:310-337`
    let total = files.len().max(1);
    for (index, file) in files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        if file.size_kb == 0 || file.size_kb > vworld_client::CADASTRAL_MAX_ZIP_KB {
            outcome.error = cadastral_zip_size_error_ko().to_owned();
            return outcome;
        }
        let path = survey_dir.join(vworld_client::cadastral_zip_file_name(file));
        if !path.is_file()
            && vworld_client::download_cadastral_zip(&file.file_no, &path).is_err()
        {
            outcome.error = vworld_client::CADASTRAL_NOT_ZIP_MESSAGE.to_owned();
            return outcome;
        }
        outcome.downloaded.push(path.to_string_lossy().into_owned());
        report(0.4 + 0.6 * (index + 1) as f32 / total as f32, &progress);
    }
    outcome.ok = !outcome.downloaded.is_empty();
    if !outcome.ok {
        outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
    }
    outcome
}

/// C++ `CadastralPortal.cpp:222-227` — 중심 ± 5km 를 WGS84 사각 범위로.
/// 정확한 값은 조사구역 경계가 있어야 나오므로, 중심 기준 반경으로 잡는다.
fn cadastral_work_box(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 4]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    let r = strata_net::ngii_topographic::NgiiTopographicClient::SCOPE_RADIUS_M;
    let box_work = vworld_client::cadastral_search_box(
        [center[0] - r, center[1] - r, center[0] + r, center[1] + r],
        0.0,
    )?;
    let to_wgs = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg4326);
    let corners = [
        to_wgs.transform_xy(box_work[0], box_work[1]),
        to_wgs.transform_xy(box_work[0], box_work[3]),
        to_wgs.transform_xy(box_work[2], box_work[1]),
        to_wgs.transform_xy(box_work[2], box_work[3]),
    ];
    let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for [lon, lat] in corners {
        if !lon.is_finite() || !lat.is_finite() {
            return None;
        }
        out[0] = out[0].min(lon);
        out[1] = out[1].min(lat);
        out[2] = out[2].max(lon);
        out[3] = out[3].max(lat);
    }
    Some(out)
}

/// C++ `CadastralPortal.cpp:235` — `QUrlQuery page` 를 붙인다.
fn with_page(url: &str, page: u32) -> String {
    // `districts_url` 은 `page=1` 을 이미 넣고 있다.
    url.replacen("&page=1&", &format!("&page={page}&"), 1)
}

fn start_download(app: &StrataAppState, state: &mut CadastralState) {
    if state.running {
        return;
    }
    state.downloaded.clear();
    let Some(survey_path) = app.survey_path.as_deref() else {
        state.status = VworldClient::NO_SURVEY_MESSAGE.to_owned();
        return;
    };
    let Some(dir) = cadastral_raw_dir(Some(survey_path)) else {
        state.status = VworldClient::NO_SURVEY_MESSAGE.to_owned();
        return;
    };
    let center = app.home_map.real_survey.survey_center;
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(Mutex::new(0.0_f32));
    let (sender, receiver) = mpsc::channel();
    state.running = true;
    state.progress = 0.0;
    state.status = "조사구역 경계에서 주변 5km의 시·군·구 지적도를 확인하고 있습니다.".to_owned();
    let thread_cancel = Arc::clone(&cancel);
    let thread_progress = Arc::clone(&progress);
    let work_crs = app.work_crs;
    std::thread::spawn(move || {
        let outcome = run_download(dir, center, work_crs, thread_cancel, thread_progress);
        let _ = sender.send(outcome);
    });
    state.cancel = cancel;
    state.progress_slot = progress;
    state.receiver = Some(receiver);
}

fn poll(app: &mut StrataAppState, ctx: &egui::Context) {
    let mut state = match cadastral_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    let shared_progress = state.progress_slot.lock().map(|slot| *slot).ok();
    if let Some(progress) = shared_progress {
        state.progress = progress;
    }
    let finished = state
        .receiver
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(outcome) = finished {
        state.running = false;
        state.receiver = None;
        state.downloaded = outcome.downloaded.clone();
        if outcome.ok {
            state.progress = 1.0;
            let message =
                cadastral_success_message_ko(outcome.downloaded.len(), outcome.districts.len());
            state.status = message.clone();
            app.status_bar_hint = message;
            // C++ `MainWindowCadastral.cpp:166-168` — 받은 지적도를 최상위 레이어로 올린다.
            app.home_map
                .real_survey
                .layer_visible
                .insert("지적도".to_owned(), true);
        } else {
            state.progress = 0.0;
            state.status = outcome.error;
        }
    }
    if state.running {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}

/// `ActiveModalDialog::CadastralDownload` 창 (C++ `MainWindowCadastral.cpp:122-174`)
pub fn render_cadastral_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    poll(app, ctx);
    let mut start = false;
    let mut cancel = false;
    let mut close = false;
    let key_present = !strata_secrets::vworld::VworldSettings::load_api_key()
        .trim()
        .is_empty();
    let mut state = match cadastral_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    egui::Window::new("ka_modal_cadastral")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(540.0)
        .show(ctx, |ui| {
            // C++ `MainWindowCadastral.cpp:122-123` 창 제목·부제
            render_strata_modal_header(ui, "지적도 다운로드 - Strata", open);
            ui.label(
                egui::RichText::new(VworldClient::CADASTRAL_DIALOG_TITLE)
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(25, 25, 24)),
            );
            ui.label(
                egui::RichText::new(VworldClient::CADASTRAL_DIALOG_SUBTITLE)
                    .size(12.5)
                    .color(Color32::from_rgb(95, 94, 88)),
            );
            ui.add_space(10.0);
            egui::Frame::NONE
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    ui.set_min_width(508.0);
                    ui.label(
                        egui::RichText::new("자료 준비 및 다운로드")
                            .size(14.5)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&state.status).size(13.5));
                    // C++ `BasemapOps.cpp:1015` — 키가 없으면 같은 안내를 그대로 보인다.
                    if !key_present {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(VworldClient::NO_KEY_MESSAGE)
                                .size(12.5)
                                .color(Color32::from_rgb(181, 87, 58)),
                        );
                    }
                    ui.add_space(8.0);
                    let (bar_r, _) =
                        ui.allocate_exact_size(Vec2::new(492.0, 26.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_r);
                    p.rect_filled(bar_r, 4.0, Color32::WHITE);
                    p.rect_stroke(
                        bar_r,
                        4.0,
                        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                        egui::StrokeKind::Middle,
                    );
                    let fraction = state.progress.clamp(0.0, 1.0);
                    if fraction > 0.0 {
                        p.rect_filled(
                            Rect::from_min_size(
                                bar_r.min + Vec2::new(1.0, 1.0),
                                Vec2::new((bar_r.width() - 2.0) * fraction, 24.0),
                            ),
                            3.0,
                            Color32::from_rgb(181, 87, 58),
                        );
                    }
                    let pct = (fraction * 100.0).round() as u32;
                    p.text(
                        bar_r.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{pct}%"),
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(35, 35, 32),
                    );
                    if !state.downloaded.is_empty() {
                        ui.add_space(6.0);
                        for path in state.downloaded.iter().take(4) {
                            ui.label(
                                egui::RichText::new(format!("· {path}"))
                                    .size(11.5)
                                    .color(Color32::from_rgb(110, 108, 102)),
                            );
                        }
                    }
                });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let start_btn = egui::Button::new(
                    egui::RichText::new("지적도 받기")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.5),
                )
                .fill(Color32::from_rgb(44, 43, 41))
                .corner_radius(6.0)
                .min_size(Vec2::new(124.0, 34.0));
                if ui
                    .add_enabled(!state.running, start_btn)
                    .on_hover_text("조사구역 주변 5km 시·군·구 지적도 ZIP 을 조사 폴더 지적도/원본 에 받습니다.")
                    .clicked()
                {
                    start = true;
                }
                if ui
                    .add_enabled(state.running, egui::Button::new("취소"))
                    .clicked()
                {
                    cancel = true;
                }
                if ui.button("계정 설정").clicked() {
                    app.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::AccountSettings);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("닫기")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.5),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(62.0, 34.0)),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });
    if start {
        start_download(app, &mut state);
    }
    if cancel {
        state.cancel.store(true, Ordering::Relaxed);
        state.status = "지적도 받기를 취소하는 중입니다.".to_owned();
    }
    if close {
        state.cancel.store(true, Ordering::Relaxed);
        *open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // C++ `MainWindowCadastral.cpp:121` — 조사 폴더 안 `지적도/원본`
    #[test]
    fn test_cadastral_raw_dir_follows_cpp_folder_rule() {
        let survey = std::path::Path::new(r"C:\조사\경주_월성북편.gpkg");
        let dir = cadastral_raw_dir(Some(survey)).expect("조사 파일이 있으면 경로가 나온다");
        assert!(dir.ends_with(r"지적도\원본"), "{}", dir.display());
        assert!(dir.starts_with(r"C:\조사"), "{}", dir.display());
        assert_eq!(cadastral_raw_dir(None), None);
    }

    #[test]
    fn test_cadastral_no_key_message_is_cpp_text() {
        assert_eq!(
            VworldClient::NO_KEY_MESSAGE,
            "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요."
        );
        assert_eq!(
            VworldClient::NO_ACCOUNT_MESSAGE,
            "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요."
        );
        assert_eq!(cadastral_zip_size_error_ko(), "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다.");
    }

    // C++ `CadastralPortal.cpp:249-258` — 5자리 코드 + full_nm
    #[test]
    fn test_parse_sigungu_features_keeps_valid_districts() {
        let body = r#"{"response":{"status":"OK","result":{"featureCollection":{"features":[
            {"properties":{"sig_cd":"47110","full_nm":"경상북도 경주시"}},
            {"properties":{"sig_cd":"47110","full_nm":"경상북도 경주시"}},
            {"properties":{"sig_cd":"47111","full_nm":"경상북도 포항시"}}]}}}}"#;
        let rows = parse_sigungu_features(body.as_bytes()).expect("parse");
        assert_eq!(rows.len(), 2, "중복 코드는 한 번만: {rows:?}");
        assert_eq!(rows[0].code, "47110");
        assert_eq!(rows[1].name, "경상북도 포항시");

        // 코드가 5자리가 아니면 C++ `:255` 처럼 오류
        let bad = r#"{"response":{"status":"OK","result":{"featureCollection":{"features":[
            {"properties":{"sig_cd":"4711","full_nm":"x"}}]}}}}"#;
        assert_eq!(
            parse_sigungu_features(bad.as_bytes()).expect_err("5자리 코드가 아니면 오류"),
            cadastral_district_error_ko()
        );
    }

    #[test]
    fn test_cadastral_success_message_names_zip_and_district_counts() {
        let message = cadastral_success_message_ko(2, 2);
        assert!(message.starts_with("조사 주변 5km 지적도를 추가했습니다."));
        assert!(message.contains("시·군·구 2곳의 원본 ZIP 2개를 받았습니다."));
    }

    // C++ `MainWindowDownloads.cpp:183-186` — 공식 신청 좌표계(EPSG:5179)
    #[test]
    fn test_work_to_portal_xy_converts_5187_center_to_5179() {
        // 경주 일대 동부원점 좌표
        let out = work_to_portal_xy([220_000.0, 360_000.0], WorkCrs::Epsg5187).expect("변환");
        assert!(out[0].is_finite() && out[1].is_finite());
        // 5179 는 false easting 1,000,000 이므로 x 가 1,000,000 근처
        assert!((800_000.0..1_200_000.0).contains(&out[0]), "{out:?}");
        assert!((1_400_000.0..2_100_000.0).contains(&out[1]), "{out:?}");
        assert!(work_to_portal_xy([f64::NAN, 0.0], WorkCrs::Epsg5187).is_none());
    }

    #[test]
    fn test_with_page_replaces_the_cpp_page_parameter() {
        let url = VworldClient::districts_url("K", [127.0, 35.0, 128.0, 36.0]);
        assert!(with_page(&url, 3).contains("&page=3&"));
        assert!(!with_page(&url, 3).contains("&page=1&"));
    }
}
