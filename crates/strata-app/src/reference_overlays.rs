//! 「지질」·「토양」·「수계」 단추 — 공개 서비스에서 받아 색 입힌 면 레이어로 올린다.
//!
//! C++ `MainWindowOverlay.cpp` 의 `downloadGeologyMap`(:492)·`downloadSoilTerrain`(:439)·
//! `downloadRiverMap`(:550) 과 공통 `startReferenceDownload`(:603-695):
//! 이미 있으면 켜고 끄기만 하고, 없으면 조사 폴더에 받아 「참조 지도」에 올린다. 한 번에 하나만 받는다.
//! 받는 일은 뒤의 스레드에서 한다.
//!
//! 아직 옮기지 못한 것: 지질도의 공식 도폭 색 맞추기와 「지질 음영」, 토양도 그림 타일,
//! 느릴 때 넷으로 나눠 받기, 범례.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, OnceLock};

use eframe::egui;
use strata_geo::crs_transform::CrsTransformer;
use strata_net::kigam_geology::{fetch_geology, unit_color, GEOLOGY_LAYER_TITLE, GEOLOGY_MAX_SPAN_M};
use strata_net::reference_wfs::{
    fetch_river, fetch_soil, river_color, soil_class, RIVER_LAYER_TITLE, RIVER_MAX_SPAN_M,
    SOIL_LAYER_TITLE, SOIL_MAX_SPAN_M,
};
use strata_types::crs::{CrsCode, WorkCrs};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::reference_layers::{ReferenceLayer, ReferenceShape, ReferenceStyle};

/// 받을 자료의 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayKind {
    Geology,
    Soil,
    River,
}

impl OverlayKind {
    const fn title(self) -> &'static str {
        match self {
            Self::Geology => GEOLOGY_LAYER_TITLE,
            Self::Soil => SOIL_LAYER_TITLE,
            Self::River => RIVER_LAYER_TITLE,
        }
    }

    const fn max_span_m(self) -> f64 {
        match self {
            Self::Geology => GEOLOGY_MAX_SPAN_M,
            Self::Soil => SOIL_MAX_SPAN_M,
            Self::River => RIVER_MAX_SPAN_M,
        }
    }

    const fn short(self) -> &'static str {
        match self {
            Self::Geology => "지질도",
            Self::Soil => "토양도",
            Self::River => "수계도",
        }
    }
}

type Outcome = (OverlayKind, Result<ReferenceLayer, String>);

fn job() -> &'static Mutex<Option<Receiver<Outcome>>> {
    static JOB: OnceLock<Mutex<Option<Receiver<Outcome>>>> = OnceLock::new();
    JOB.get_or_init(|| Mutex::new(None))
}

const fn crs_code(work: WorkCrs) -> CrsCode {
    match work {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    }
}

/// 면 하나를 작업 좌표계의 참조 도형으로.
fn shape(rings: &[Vec<[f64; 2]>], to_work: &CrsTransformer, label: String, fill: [u8; 4]) -> ReferenceShape {
    let parts: Vec<Vec<[f64; 2]>> = rings
        .iter()
        .map(|ring| ring.iter().map(|p| to_work.transform_xy(p[0], p[1])).collect())
        .collect();
    let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in parts.iter().flatten() {
        bbox = [bbox[0].min(p[0]), bbox[1].min(p[1]), bbox[2].max(p[0]), bbox[3].max(p[1])];
    }
    ReferenceShape { parts, label, bbox, fill: Some(fill) }
}

fn layer(kind: OverlayKind, shapes: Vec<ReferenceShape>, source: PathBuf, line_rgb: [u8; 3], line_mm: f32, label_max_mpp: f64) -> ReferenceLayer {
    ReferenceLayer {
        name: kind.title().to_owned(),
        source,
        shape_type: 5,
        shapes,
        style: ReferenceStyle { line_rgb, line_mm, label_max_mpp, group: LegendGroup::ReferenceMap },
    }
}

/// 범위(경위도)의 자료를 받아 레이어로 만든다. 뒤의 스레드에서 돈다.
fn fetch_layer(kind: OverlayKind, bbox: [f64; 4], folder: PathBuf, work_crs: WorkCrs, key: String) -> Result<ReferenceLayer, String> {
    let cancel = AtomicBool::new(false);
    let work = crs_code(work_crs);
    let shapes: Vec<ReferenceShape> = match kind {
        // 지질도: 좌표 5186, 채움 알파 220, 테두리 (80,80,80) 0.12 mm, 글자는 기호(축척 무관).
        OverlayKind::Geology => {
            let to_work = CrsTransformer::new(CrsCode::Epsg5186, work);
            fetch_geology(bbox, &folder.join("지질도_KIGAM.geojson"), &cancel)?
                .into_iter()
                .map(|g| {
                    let [r, gr, b] = unit_color(&g.era, &g.symbol);
                    shape(&g.rings, &to_work, g.symbol, [r, gr, b, 220])
                })
                .collect()
        }
        // 토양도: 좌표 5186, 채움 알파 175, 글자는 분포지형 짧은 이름.
        OverlayKind::Soil => {
            let to_work = CrsTransformer::new(CrsCode::Epsg5186, work);
            fetch_soil(bbox, &folder, &cancel)?
                .into_iter()
                .map(|s| {
                    let (_, short, [r, g, b]) = soil_class(s.props.first().map_or("", String::as_str));
                    shape(&s.rings, &to_work, short.to_owned(), [r, g, b, 175])
                })
                .collect()
        }
        // 수계도: 좌표 4326, 채움 알파 230, 글자는 하천 이름.
        OverlayKind::River => {
            let to_work = CrsTransformer::new(CrsCode::Epsg4326, work);
            fetch_river(&key, bbox, &folder, &cancel)?
                .into_iter()
                .map(|s| {
                    let [r, g, b] = river_color(s.props.first().map_or("", String::as_str));
                    shape(&s.rings, &to_work, s.props.get(1).cloned().unwrap_or_default(), [r, g, b, 230])
                })
                .collect()
        }
    };
    if shapes.is_empty() {
        return Err(format!("이 범위에는 받을 {} 자료가 없습니다.", kind.short()));
    }
    Ok(match kind {
        OverlayKind::Geology => layer(kind, shapes, folder, [80, 80, 80], 0.12, f64::INFINITY),
        // 토양 글자는 1:40,000 보다 확대했을 때만(C++ `SoilMapService.cpp:173-240`).
        OverlayKind::Soil => layer(kind, shapes, folder, [40, 44, 52], 0.28, 40_000.0 / 3_780.0),
        OverlayKind::River => layer(kind, shapes, folder, [0, 84, 141], 0.32, f64::INFINITY),
    })
}

impl StrataAppState {
    /// 단추를 눌렀을 때: 이미 올린 레이어가 있으면 켜고 끄고, 없으면 받는다.
    pub(crate) fn toggle_or_download_overlay(&mut self, kind: OverlayKind) {
        if self.toggle_existing_overlay(kind.title()) {
            return;
        }
        if job().lock().is_ok_and(|j| j.is_some()) {
            self.status_bar_hint = "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요.".to_owned();
            return;
        }
        // C++ `MainWindowDownloads.cpp:98-100` — 받은 자료는 조사 폴더에 둔다.
        let Some(folder) = self.survey_path.as_deref().and_then(|p| p.parent()).map(|p| p.join("참조자료")) else {
            self.status_bar_hint = "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.".to_owned();
            return;
        };
        let key = strata_secrets::vworld::VworldSettings::load_api_key();
        if kind == OverlayKind::River && key.trim().is_empty() {
            self.status_bar_hint = "VWorld 인증키가 없습니다. 배경지도 설정에서 키를 먼저 등록하세요.".to_owned();
            return;
        }
        // 화면 범위(작업 좌표계).
        let vp = &self.home_map.viewport;
        let half = [
            vp.screen_size_px[0] * 0.5 * vp.meters_per_pixel,
            vp.screen_size_px[1] * 0.5 * vp.meters_per_pixel,
        ];
        let longest = half[0].max(half[1]) * 2.0;
        if longest > kind.max_span_m() {
            self.status_bar_hint = format!(
                "범위가 너무 넓습니다. 지도를 조사구역(한 변 {:.0}km 이하)으로 확대한 뒤 다시 내려받으세요.",
                kind.max_span_m() / 1000.0
            );
            return;
        }
        // 지질도만 긴 변이 80 km 가 되도록 가운데에서 넓혀 한 번에 받는다(C++ `expandExtentToMaxSpan`).
        let grow = if kind == OverlayKind::Geology { kind.max_span_m() / longest.max(1.0) } else { 1.0 };
        let (hx, hy) = (half[0] * grow, half[1] * grow);
        let [cx, cy] = vp.center_xy;
        let to_wgs = CrsTransformer::new(crs_code(self.work_crs), CrsCode::Epsg4326);
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for (x, y) in [(cx - hx, cy - hy), (cx - hx, cy + hy), (cx + hx, cy - hy), (cx + hx, cy + hy)] {
            let [lon, lat] = to_wgs.transform_xy(x, y);
            bbox = [bbox[0].min(lon), bbox[1].min(lat), bbox[2].max(lon), bbox[3].max(lat)];
        }

        let work_crs = self.work_crs;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send((kind, fetch_layer(kind, bbox, folder, work_crs, key)));
        });
        if let Ok(mut slot) = job().lock() {
            *slot = Some(receiver);
        }
        self.status_bar_hint = format!("{}를 받는 중입니다. 지도 작업을 계속할 수 있습니다.", kind.title());
    }

    /// 받기가 끝났는지 본다(지도 탭을 그릴 때마다 부른다).
    pub(crate) fn poll_overlay_download(&mut self, ctx: &egui::Context) {
        let (pending, finished) = job()
            .lock()
            .map(|slot| (slot.is_some(), slot.as_ref().and_then(|r| r.try_recv().ok())))
            .unwrap_or((false, None));
        let Some((kind, outcome)) = finished else {
            if pending {
                // 받는 동안에는 가만히 있어도 결과를 보러 다시 그린다.
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
            return;
        };
        if let Ok(mut slot) = job().lock() {
            *slot = None;
        }
        match outcome {
            Ok(layer) => {
                let n = layer.shapes.len();
                self.install_reference_layer(layer);
                self.status_bar_hint = match kind {
                    OverlayKind::Geology => format!(
                        "지질도를 추가했습니다(지질 단위 면 {n}개). 공식 도폭 색 맞추기는 아직 옮기지 못해 지질시대 기준색으로 표시합니다."
                    ),
                    _ => format!("{}를 추가했습니다(면 {n}개).", kind.short()),
                };
            }
            Err(why) => self.status_bar_hint = format!("{} 내려받기 실패: {why}", kind.short()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_received_polygon_becomes_a_filled_shape_in_the_work_crs() {
        let ring = vec![[200_000.0, 450_000.0], [200_100.0, 450_000.0], [200_100.0, 450_100.0], [200_000.0, 450_000.0]];
        // 작업 좌표계가 5186 이면 좌표가 그대로다.
        let same = shape(std::slice::from_ref(&ring), &CrsTransformer::new(CrsCode::Epsg5186, CrsCode::Epsg5186), "Qa".to_owned(), [1, 2, 3, 220]);
        assert_eq!(same.parts[0][0], [200_000.0, 450_000.0]);
        assert_eq!((same.label.as_str(), same.fill), ("Qa", Some([1, 2, 3, 220])));
        assert_eq!(same.bbox, [200_000.0, 450_000.0, 200_100.0, 450_100.0]);
        // 5187 이면 동쪽 원점 기준 좌표로 옮겨진다(서쪽으로 약 180 km).
        let moved = shape(&[ring], &CrsTransformer::new(CrsCode::Epsg5186, CrsCode::Epsg5187), String::new(), [0; 4]);
        assert!(moved.parts[0][0][0] < 30_000.0, "{:?}", moved.parts[0][0]);
    }

    #[test]
    fn without_an_open_survey_the_buttons_explain_and_download_nothing() {
        for kind in [OverlayKind::Geology, OverlayKind::Soil, OverlayKind::River] {
            let mut app = StrataAppState::new_startup();
            app.toggle_or_download_overlay(kind);
            assert!(app.status_bar_hint.starts_with("먼저 「새 조사」를 만들거나 조사를 여세요."), "{kind:?}");
            assert!(app.home_map.reference_layers.is_empty());
        }
    }
}
