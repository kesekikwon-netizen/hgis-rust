//! 「DEM」 실제 내려받기 — C++ `MainWindowDownloads.cpp:123-197` `startDemDownload()` 의
//! 저장 위치 규칙(:134-135)과 `DemDownloadService::prepare`(`DemDownloadService.cpp:109-305`)
//! 파이프라인을 Rust 로 옮긴 창이다.
//!
//! C++ 은 DEM 단추를 누르면 국토지리정보원 창(WebView 자동화, 로그인·신청)을 열고,
//! 공개 Copernicus 타일 경로(`DemDownloadService`)는 별도 서비스로 남아 있다
//! (`target/cpp-ref/docs/ERROR_REGRESSION.md:87`). 이 모듈은 그중 **로그인 없이 실제로
//! 받을 수 있는 Copernicus 30m 타일 경로**를 창에서 바로 실행한다. 타일 규칙·상한·404
//! 건너뛰기·저장 이름은 `strata_net::dem_copernicus` 가 C++ 원문 그대로 갖고 있다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui::{self, Color32, Stroke, Vec2};
use strata_geo::crs_transform::{tm_to_lonlat, TmParams};
use strata_net::dem_copernicus::{
    self, dem_target_file_name, DemProgress, PreparedDem, Wgs84Extent, DEM_RASTER_FILE_NAME,
};
use strata_types::crs::{CrsCode, WorkCrs};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

/// C++ `MainWindowDownloads.cpp:134-135` — 조사 파일 옆 `DEM/원본` 폴더에 받는다.
pub const DEM_FOLDER_NAME: &str = "DEM";
pub const DEM_RAW_FOLDER_NAME: &str = "원본";

/// C++ `MainWindowDownloads.cpp:130-131` — 조사가 없을 때 안내
pub const DEM_NO_SURVEY_KO: &str = "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다.";
/// C++ `MainWindowDownloads.cpp:137-138` — 폴더를 못 만들 때
pub const DEM_FOLDER_CREATE_FAILED_KO: &str = "조사 폴더 안에 DEM 폴더를 만들지 못했습니다.";

/// C++ `BasemapDsm.h:21` 범례 이름 (Rust 범례는 줄여 쓴다 — `gui_ribbon_bar.rs:168-171`)
const DEM_LEGEND_KEYS: [&str; 2] = ["DEM", "지표모델(DSM) 30m · Coperni..."];

struct DemDownloadState {
    running: bool,
    progress: f32,
    status: String,
    cancel: Arc<AtomicBool>,
    progress_slot: Arc<Mutex<f32>>,
    receiver: Option<mpsc::Receiver<PreparedDem>>,
    /// 완료된 결과. `storage_dir` 를 들고 있어야 받은 `dem.tif` 가 지워지지 않는다.
    result: Option<PreparedDem>,
    tile_total: usize,
}

impl DemDownloadState {
    fn idle() -> Self {
        Self {
            running: false,
            progress: 0.0,
            status: "조사구역 주변의 공개 DEM 30m 타일을 받아 한 장으로 붙입니다.".to_owned(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
            receiver: None,
            result: None,
            tile_total: 0,
        }
    }
}

fn dem_state() -> &'static Mutex<DemDownloadState> {
    static STATE: OnceLock<Mutex<DemDownloadState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(DemDownloadState::idle()))
}

/// C++ `MainWindowDownloads.cpp:134-135` — 조사 폴더 아래 `DEM/원본/dem.tif`.
/// 조사 파일이 없으면 `None` (C++ :129-133 은 창을 열지 않고 안내만 한다).
#[must_use]
pub fn dem_target_base_path(survey_path: Option<&Path>) -> Option<String> {
    let profile = survey_path?;
    let parent = profile.parent().unwrap_or_else(|| Path::new("."));
    let target = parent
        .join(DEM_FOLDER_NAME)
        .join(DEM_RAW_FOLDER_NAME)
        .join(DEM_RASTER_FILE_NAME);
    let text = target.to_string_lossy().into_owned();
    // C++ `DemDownloadService.cpp:133-136` 저장 위치 검사와 같은 규칙을 미리 본다.
    dem_target_file_name(&text).map(|_| text)
}

/// C++ `MainWindowDownloads.cpp:136` `QDir().mkpath(rawDir)` — 조사 폴더 안에
/// `DEM/원본` 을 만든다. 실패하면 `false`.
#[must_use]
pub fn ensure_dem_folder(survey_path: Option<&Path>) -> bool {
    let Some(profile) = survey_path else {
        return false;
    };
    let parent = profile.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent.join(DEM_FOLDER_NAME).join(DEM_RAW_FOLDER_NAME)).is_ok()
}

/// C++ `MainWindowDownloads.cpp:176-187` — 작업 좌표계 화면 범위를 EPSG:4326 으로 옮긴
/// 네 모서리의 바깥 범위. `QgsCoordinateTransform::transformBoundingBox` 와 같다.
#[must_use]
pub fn wgs84_extent_from_work(bbox: [f64; 4], work_crs: WorkCrs) -> Option<Wgs84Extent> {
    let params = TmParams::for_crs(match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    })?;
    let [min_x, min_y, max_x, max_y] = bbox;
    if ![min_x, min_y, max_x, max_y].iter().all(|v| v.is_finite()) || max_x <= min_x || max_y <= min_y
    {
        return None;
    }
    let corners = [
        tm_to_lonlat(min_x, min_y, &params),
        tm_to_lonlat(min_x, max_y, &params),
        tm_to_lonlat(max_x, min_y, &params),
        tm_to_lonlat(max_x, max_y, &params),
    ];
    let mut min_lon = f64::INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for [lon, lat] in corners {
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
    }
    let extent = Wgs84Extent::new(min_lon, min_lat, max_lon, max_lat);
    extent.is_valid_wgs84().then_some(extent)
}

/// 완료 안내 문구. C++ `DemDownloadService.cpp:211` 의 경고와 성공 보고를 합친 형태.
#[must_use]
pub fn dem_success_message_ko(downloaded: usize, missing: usize) -> String {
    let mut message = format!("DEM {downloaded}칸을 받아 한 장으로 붙였습니다.");
    if missing > 0 {
        message.push(' ');
        message.push_str(&dem_copernicus::dem_missing_tiles_warning_ko(missing));
    }
    message
}

fn start_download(app: &StrataAppState, state: &mut DemDownloadState) {
    if state.running {
        return;
    }
    state.result = None;
    let target = match dem_target_base_path(app.survey_path.as_deref()) {
        Some(target) => target,
        None => {
            state.status = DEM_NO_SURVEY_KO.to_owned();
            return;
        }
    };
    if !ensure_dem_folder(app.survey_path.as_deref()) {
        state.status = DEM_FOLDER_CREATE_FAILED_KO.to_owned();
        return;
    }
    let Some(extent) = wgs84_extent_from_work(app.home_map.viewport.world_bbox(), app.work_crs)
    else {
        state.status = dem_copernicus::DEM_EXTENT_INVALID_KO.to_owned();
        return;
    };
    let tiles = match dem_copernicus::tiles_for_extent(extent) {
        Ok(tiles) => tiles,
        Err(error) => {
            state.status = error.message_ko();
            return;
        }
    };
    state.tile_total = tiles.len();
    state.running = true;
    state.progress = 0.0;
    state.status = format!("DEM {}칸을 받는 중입니다.", tiles.len());
    let cancel = Arc::new(AtomicBool::new(false));
    let progress_slot = Arc::new(Mutex::new(0.0_f32));
    let (sender, receiver) = mpsc::channel();
    let thread_cancel = Arc::clone(&cancel);
    let thread_progress = Arc::clone(&progress_slot);
    std::thread::spawn(move || {
        let report = |fraction: f32| {
            if let Ok(mut slot) = thread_progress.lock() {
                *slot = fraction;
            }
        };
        let callback: &dyn Fn(f32) = &report;
        let progress = DemProgress {
            on_progress: Some(callback),
            is_cancelled: Some(&thread_cancel),
        };
        let result = dem_copernicus::prepare(extent, &target, &progress);
        let _ = sender.send(result);
    });
    state.cancel = cancel;
    state.progress_slot = progress_slot;
    state.receiver = Some(receiver);
}

fn apply_result(app: &mut StrataAppState, state: &mut DemDownloadState) {
    let Some(result) = state.result.as_ref() else {
        return;
    };
    if !result.is_ready() {
        return;
    }
    // C++ `MainWindowDownloads.cpp:69-96` — 받은 래스터를 참조 지도로 올린다.
    for key in DEM_LEGEND_KEYS {
        app.home_map
            .real_survey
            .layer_visible
            .insert(key.to_owned(), true);
    }
    let message = dem_success_message_ko(result.downloaded_tiles, result.missing_tiles);
    app.status_bar_hint = message.clone();
    app.home_map.draw_guide_message = Some(message);
}

fn poll(app: &mut StrataAppState, ctx: &egui::Context) {
    let mut state = match dem_state().lock() {
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
    if let Some(result) = finished {
        state.running = false;
        state.receiver = None;
        state.progress = if result.is_ready() { 1.0 } else { 0.0 };
        state.status = if result.is_ready() {
            dem_success_message_ko(result.downloaded_tiles, result.missing_tiles)
        } else {
            result.error.clone()
        };
        state.result = Some(result);
        apply_result(app, &mut state);
    }
    if state.running {
        // 진행률은 작업 스레드가 채운다. 창이 멈추지 않도록 다음 프레임을 예약한다.
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}

/// `ActiveModalDialog::DemDownload` 창 (C++ `startDemDownload` + `DemDownloadService::prepare`)
pub fn render_dem_download(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    poll(app, ctx);
    let mut start = false;
    let mut cancel = false;
    let mut close = false;
    let mut state = match dem_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    egui::Window::new("ka_modal_dem_download")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(510.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "국토지리정보원 DEM 다운로드 - Strata", open);
            ui.label(
                egui::RichText::new(
                    "조사구역 주변 공개 DEM 30m(Copernicus)를 받아 조사 폴더 DEM/원본 에 한 장으로 저장합니다.",
                )
                .size(14.0)
                .color(Color32::from_rgb(25, 25, 24)),
            );
            ui.add_space(10.0);
            let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(480.0, 32.0), egui::Sense::hover());
            let painter = ui.painter_at(bar_rect);
            painter.rect_filled(bar_rect, 6.0, Color32::WHITE);
            painter.rect_stroke(
                bar_rect,
                6.0,
                Stroke::new(1.2, Color32::from_rgb(150, 148, 142)),
                egui::StrokeKind::Middle,
            );
            let fraction = state.progress.clamp(0.0, 1.0);
            if fraction > 0.0 {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        bar_rect.min + Vec2::new(2.0, 2.0),
                        Vec2::new(476.0 * fraction, 28.0),
                    ),
                    5.0,
                    Color32::from_rgb(20, 20, 20),
                );
            }
            let pct = (fraction * 100.0).round() as u32;
            painter.text(
                bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{pct}%"),
                egui::FontId::proportional(12.5),
                Color32::from_rgb(55, 54, 50),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(&state.status)
                    .size(13.0)
                    .color(Color32::from_rgb(60, 59, 55)),
            );
            if state.tile_total > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "받을 칸 {}개 (1° 격자) · 한 번에 최대 {}칸",
                        state.tile_total,
                        dem_copernicus::DEM_MAXIMUM_TILES
                    ))
                    .size(12.0)
                    .color(Color32::from_rgb(95, 94, 88)),
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let start_btn = egui::Button::new(
                    egui::RichText::new(if state.result.is_some() {
                        "다시 받기"
                    } else {
                        "DEM 받기"
                    })
                    .color(Color32::WHITE)
                    .strong()
                    .size(13.5),
                )
                .fill(Color32::from_rgb(44, 43, 41))
                .corner_radius(6.0)
                .min_size(Vec2::new(132.0, 36.0));
                if ui
                    .add_enabled(!state.running, start_btn)
                    .on_hover_text("지금 화면 범위를 덮는 1° 공개 DEM 타일을 받습니다.")
                    .clicked()
                {
                    start = true;
                }
                if ui
                    .add_enabled(state.running, egui::Button::new("내려받기 취소"))
                    .clicked()
                {
                    cancel = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // C++ `MainWindowDownloads.cpp:88` `QProgressDialog::canceled` → 창 닫기
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("닫기")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(14.0),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(72.0, 36.0)),
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
        state.status = "DEM 내려받기를 취소하는 중입니다.".to_owned();
    }
    if close {
        // 닫아도 진행 중인 작업은 취소한다 (C++ 취소 버튼과 같은 결과).
        state.cancel.store(true, Ordering::Relaxed);
        *open = false;
    }
}

/// 화면에 올라온 DEM 타일 파일 경로 (완료 뒤 읽기용)
#[must_use]
pub fn current_dem_raster_path() -> Option<PathBuf> {
    dem_state()
        .lock()
        .ok()
        .and_then(|state| state.result.as_ref().and_then(|r| r.raster_uri.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // C++ `MainWindowDownloads.cpp:134-135` — 조사 폴더 옆 `DEM/원본/dem.tif`
    #[test]
    fn test_dem_target_base_path_follows_cpp_dem_raw_folder_rule() {
        let survey = Path::new(r"C:\조사\경주_월성북편.gpkg");
        let target = dem_target_base_path(Some(survey)).expect("조사 파일이 있으면 경로가 나온다");
        assert!(target.ends_with(r"DEM\원본\dem.tif"), "{target}");
        assert!(target.starts_with(r"C:\조사\"), "{target}");

        // 조사가 없으면 C++ :129-133 처럼 받지 않는다.
        assert_eq!(dem_target_base_path(None), None);
    }

    // C++ `MainWindowDownloads.cpp:176-187` — 화면 범위를 위경도로 옮긴다
    #[test]
    fn test_dem_wgs84_extent_matches_gyeongju_work_area() {
        // 경주 월성북편 일대 EPSG:5187 대략 범위
        let extent = wgs84_extent_from_work(
            [219_000.0, 360_400.0, 220_500.0, 361_400.0],
            WorkCrs::Epsg5187,
        )
        .expect("5187 범위는 위경도로 바꿀 수 있다");
        assert!(extent.is_valid_wgs84());
        // 동부원점(129°E) 부근, 북위 35°대
        assert!((128.9..129.4).contains(&extent.min_lon), "{extent:?}");
        assert!((128.9..129.5).contains(&extent.max_lon), "{extent:?}");
        assert!((35.5..36.0).contains(&extent.min_lat), "{extent:?}");
        assert!((35.6..36.1).contains(&extent.max_lat), "{extent:?}");
        // 이 범위는 1~4칸이라 상한(24) 안이다.
        let tiles = dem_copernicus::tiles_for_extent(extent).expect("칸 수가 상한 안이다");
        assert!(!tiles.is_empty());
        assert!(tiles.len() <= dem_copernicus::DEM_MAXIMUM_TILES);

        // 뒤집히거나 비어 있는 범위는 변환하지 않는다.
        assert!(wgs84_extent_from_work([1.0, 2.0, 0.0, 0.0], WorkCrs::Epsg5187).is_none());
        assert!(wgs84_extent_from_work(
            [f64::NAN, 0.0, 1.0, 1.0],
            WorkCrs::Epsg5186
        )
        .is_none());
    }

    // C++ `DemDownloadService.cpp:211` — 빠진 칸이 있으면 경고를 붙인다
    #[test]
    fn test_dem_success_message_reports_tiles_and_missing_cells() {
        assert_eq!(
            dem_success_message_ko(4, 0),
            "DEM 4칸을 받아 한 장으로 붙였습니다."
        );
        assert_eq!(
            dem_success_message_ko(2, 2),
            "DEM 2칸을 받아 한 장으로 붙였습니다. 요청 범위 중 2칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다."
        );
    }

    // C++ 문구는 원문 그대로여야 한다 (`MainWindowDownloads.cpp:130-138`)
    #[test]
    fn test_dem_folder_messages_match_cpp_text() {
        assert_eq!(
            DEM_NO_SURVEY_KO,
            "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다."
        );
        assert_eq!(
            DEM_FOLDER_CREATE_FAILED_KO,
            "조사 폴더 안에 DEM 폴더를 만들지 못했습니다."
        );
        assert_eq!(DEM_FOLDER_NAME, "DEM");
        assert_eq!(DEM_RAW_FOLDER_NAME, "원본");
    }
}
