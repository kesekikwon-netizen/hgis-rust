//! 「수치지형」 단추 — 국토정보플랫폼에서 1:25,000 수치지형도(DXF)를 받아 지도에 올린다.
//!
//! C++ `KaTopographicBrowser`(도엽 확인 → 로그인 → 신청 → 파일 준비 → 다운로드 → 지도에 올리기).
//! 받는 길은 「DEM」과 같다(`dem_ngii_worker::receive_from_portal`). 받은 ZIP 을 풀어 DXF 의 선을
//! 「참조 지도」 레이어로 올린다. 단추를 누르면 바로 시작한다(받을 때마다 공식 신청 1건).
//!
//! 아직 옮기지 못한 것: 지형지물 부호별 색·굵기, 지명 글자, 점 기호, 이미 받은 도엽 다시 쓰기.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use strata_geo::crs_transform::CrsTransformer;
use strata_storage::dxf_lines_reader::{read_dxf_shapes, DxfShape};
use strata_types::crs::{CrsCode, WorkCrs};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::dem_ngii_worker::{receive_from_portal, NgiiDemJob};
use crate::reference_layers::{ReferenceLayer, ReferenceShape, ReferenceStyle};

pub(crate) const TOPO_LAYER_TITLE: &str = "수치지형도(1:25,000)";
/// 수치지형도 DXF 가 쓸 수 있는 좌표계 후보(도엽에 따라 원점이 다르다).
const DXF_CRS_CANDIDATES: [CrsCode; 4] = [CrsCode::Epsg5186, CrsCode::Epsg5187, CrsCode::Epsg5179, CrsCode::Epsg5174];
/// 창의 단계 이름(C++ `KaStageRail`).
const STAGES: [&str; 6] = ["도엽 확인", "로그인", "신청", "파일 준비", "다운로드", "지도에 올리기"];

struct TopoJob {
    receiver: Receiver<Result<ReferenceLayer, String>>,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
}

fn job() -> &'static Mutex<Option<TopoJob>> {
    static JOB: OnceLock<Mutex<Option<TopoJob>>> = OnceLock::new();
    JOB.get_or_init(|| Mutex::new(None))
}

const fn crs_code(work: WorkCrs) -> CrsCode {
    match work {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    }
}

/// DXF 좌표가 어느 좌표계인지: 첫 선의 첫 점을 작업 좌표계로 옮겼을 때 받으려던 곳(`near`)에
/// 가장 가까운 후보. 30 km 안에 드는 후보가 없으면 `None`.
fn guess_dxf_crs(shapes: &[DxfShape], work: CrsCode, near: [f64; 2]) -> Option<CrsCode> {
    let sample = shapes.iter().find(|s| s.points.len() >= 2)?.points[0];
    DXF_CRS_CANDIDATES
        .iter()
        .map(|crs| {
            let [x, y] = CrsTransformer::new(*crs, work).transform_xy(sample[0], sample[1]);
            (*crs, (x - near[0]).hypot(y - near[1]))
        })
        .filter(|(_, far)| far.is_finite() && *far < 30_000.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(crs, _)| crs)
}

/// DXF 의 선들을 작업 좌표계의 참조 도형으로.
fn line_shapes(shapes: &[DxfShape], from: CrsCode, work: CrsCode) -> Vec<ReferenceShape> {
    let to_work = CrsTransformer::new(from, work);
    shapes
        .iter()
        .filter(|s| s.points.len() >= 2)
        .map(|s| {
            let mut part: Vec<[f64; 2]> = s.points.iter().map(|p| to_work.transform_xy(p[0], p[1])).collect();
            if s.closed {
                part.push(part[0]);
            }
            let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in &part {
                bbox = [bbox[0].min(p[0]), bbox[1].min(p[1]), bbox[2].max(p[0]), bbox[3].max(p[1])];
            }
            ReferenceShape { parts: vec![part], label: String::new(), bbox, fill: None }
        })
        .collect()
}

/// 받은 파일(ZIP·DXF)에서 선을 읽어 레이어 하나로 만든다.
fn layer_from_files(received: &[String], folder: &Path, work: CrsCode, near: [f64; 2]) -> Result<ReferenceLayer, String> {
    let mut dxfs: Vec<PathBuf> = Vec::new();
    for path in received.iter().map(PathBuf::from) {
        let ext = |e: &str| path.extension().is_some_and(|x| x.eq_ignore_ascii_case(e));
        if ext("zip") {
            let stem = path.file_stem().map_or_else(|| "sheet".into(), |s| s.to_string_lossy().into_owned());
            let files = strata_storage::zip_extract::extract_zip(&path, &folder.join(stem)).map_err(|e| format!("받은 압축 파일을 풀지 못했습니다: {e}"))?;
            dxfs.extend(files.into_iter().filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dxf"))));
        } else if ext("dxf") {
            dxfs.push(path);
        }
    }
    if dxfs.is_empty() {
        return Err("받은 파일에 DXF 지도가 없습니다. 파일은 조사 폴더의 수치지형도 에 있습니다.".to_owned());
    }
    let mut shapes = Vec::new();
    for dxf in &dxfs {
        let read = read_dxf_shapes(dxf)?;
        let crs = guess_dxf_crs(&read, work, near).ok_or("받은 수치지형도의 좌표계를 알아내지 못했습니다.")?;
        shapes.extend(line_shapes(&read, crs, work));
    }
    if shapes.is_empty() {
        return Err("받은 수치지형도에서 선을 읽지 못했습니다.".to_owned());
    }
    Ok(ReferenceLayer {
        name: TOPO_LAYER_TITLE.to_owned(),
        source: folder.to_path_buf(),
        shape_type: 3,
        shapes,
        // 어두운 갈색 가는 선(등고선·도로·하천이 한 색 — 부호별 색은 아직 옮기지 못했다).
        style: ReferenceStyle { line_rgb: [96, 74, 52], line_mm: 0.18, label_max_mpp: 0.0, group: LegendGroup::ReferenceMap },
    })
}

impl StrataAppState {
    /// 「수치지형」 단추: 올려 둔 것이 있으면 켜고 끄고, 없으면 바로 받는다.
    pub(crate) fn start_topographic_download(&mut self) {
        if self.toggle_existing_overlay(TOPO_LAYER_TITLE) {
            return;
        }
        if job().lock().is_ok_and(|j| j.is_some()) {
            self.status_bar_hint = "수치지형도를 받는 중입니다. 완료를 기다리거나 취소하세요.".to_owned();
            return;
        }
        let Some(folder) = self.survey_path.as_deref().and_then(|p| p.parent()).map(|p| p.join("수치지형도")) else {
            self.status_bar_hint = "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.".to_owned();
            return;
        };
        // 받기는 공식 신청을 내는 일이므로 시험과 화면 캡처 실행에서는 시작하지 않는다.
        if cfg!(test) || (self.snap.is_some() && std::env::var_os("KA_HGIS_SNAP_REAL_ORDER").is_none()) {
            return;
        }
        let vp = &self.home_map.viewport;
        let radius_m = (vp.screen_size_px[0].max(vp.screen_size_px[1]) * 0.5 * vp.meters_per_pixel).clamp(500.0, 5_000.0);
        let work = crs_code(self.work_crs);
        let near = vp.center_xy;
        let cancel = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new("국토정보플랫폼에 연결하는 중입니다.".to_owned()));
        let portal_job = NgiiDemJob {
            center_5179: CrsTransformer::new(work, CrsCode::Epsg5179).transform_xy(near[0], near[1]),
            radius_m,
            folder: folder.clone(),
            topographic: true,
            cancel: Arc::clone(&cancel),
            progress: Arc::new(Mutex::new(0.0)),
            status: Arc::clone(&status),
        };
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let outcome = receive_from_portal(&portal_job)
                .map_err(|failure| failure.error)
                .and_then(|received| {
                    if let Ok(mut slot) = portal_job.status.lock() {
                        "국토정보플랫폼 · 지도에 올리기".clone_into(&mut slot);
                    }
                    layer_from_files(&received, &folder, work, near)
                });
            let _ = sender.send(outcome);
        });
        if let Ok(mut slot) = job().lock() {
            *slot = Some(TopoJob { receiver, cancel, status });
        }
        self.modals.topo_browser_stage = 0;
        self.modals.topo_browser_log.clear();
        self.status_bar_hint = "수치지형도를 받는 중입니다. 지도 작업을 계속할 수 있습니다.".to_owned();
    }

    /// 받기를 그만둔다(이미 낸 신청은 공식 신청 내역에 남는다).
    pub(crate) fn cancel_topographic_download(&mut self) {
        if let Ok(slot) = job().lock() {
            if let Some(running) = slot.as_ref() {
                running.cancel.store(true, Ordering::Relaxed);
            }
        }
    }

    /// 받기의 진행을 창에 옮기고, 끝났으면 지도에 올린다(지도 탭과 창을 그릴 때마다 부른다).
    pub(crate) fn poll_topographic_download(&mut self, ctx: &egui::Context) {
        let (said, finished) = {
            let Ok(slot) = job().lock() else { return };
            let Some(running) = slot.as_ref() else { return };
            (running.status.lock().map(|s| s.clone()).unwrap_or_default(), running.receiver.try_recv().ok())
        };
        // 안내 글의 단계 이름으로 단계 띠를 옮긴다.
        if let Some(stage) = STAGES.iter().position(|name| said.contains(name)) {
            self.modals.topo_browser_stage = stage;
        }
        if self.modals.topo_browser_log.last() != Some(&said) && !said.is_empty() {
            self.modals.topo_browser_log.push(said);
        }
        let Some(outcome) = finished else {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        };
        if let Ok(mut slot) = job().lock() {
            *slot = None;
        }
        let message = match outcome {
            Ok(layer) => {
                let n = layer.shapes.len();
                self.install_reference_layer(layer);
                self.modals.topo_browser_stage = 5;
                format!("수치지형도를 추가했습니다(선 {n}개).")
            }
            Err(why) => format!("수치지형도 내려받기 실패: {why}"),
        };
        self.modals.topo_browser_log.push(message.clone());
        self.status_bar_hint = message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(points: Vec<[f64; 2]>, closed: bool) -> DxfShape {
        DxfShape { layer: "F0017111".to_owned(), points, closed, elevation: 100.0, text: String::new() }
    }

    #[test]
    fn the_dxf_crs_is_the_candidate_that_lands_where_we_asked() {
        // 경도 127.06, 위도 36.65 를 중부원점(5186)으로 적은 DXF, 작업 좌표계는 동부원점(5187).
        let central = CrsTransformer::new(CrsCode::Epsg4326, CrsCode::Epsg5186).transform_xy(127.06, 36.65);
        let near = CrsTransformer::new(CrsCode::Epsg4326, CrsCode::Epsg5187).transform_xy(127.06, 36.65);
        let shapes = vec![line(vec![central, [central[0] + 50.0, central[1]]], false)];
        assert_eq!(guess_dxf_crs(&shapes, CrsCode::Epsg5187, near), Some(CrsCode::Epsg5186));
        // 엉뚱한 좌표는 어느 후보로도 가까이 오지 않는다.
        let far = vec![line(vec![[1.0, 2.0], [3.0, 4.0]], false)];
        assert_eq!(guess_dxf_crs(&far, CrsCode::Epsg5187, near), None);
        // 옮긴 선은 받으려던 곳에 놓이고, 닫힌 선은 처음 점으로 돌아온다.
        let moved = line_shapes(&[line(vec![central, [central[0] + 50.0, central[1]], [central[0], central[1] + 50.0]], true)], CrsCode::Epsg5186, CrsCode::Epsg5187);
        let part = &moved[0].parts[0];
        assert_eq!(part.len(), 4);
        assert!((part[0][0] - near[0]).hypot(part[0][1] - near[1]) < 0.01 && part[0] == part[3]);
    }

    #[test]
    fn without_an_open_survey_the_button_explains_and_starts_nothing() {
        let mut app = StrataAppState::new_startup();
        app.start_topographic_download();
        assert!(app.status_bar_hint.starts_with("먼저 「새 조사」를 만들거나 조사를 여세요."));
        assert!(job().lock().is_ok_and(|j| j.is_none()));
    }
}
