//! 「지질」 단추 — KIGAM 지질도를 받아 색 입힌 면 레이어로 올린다.
//!
//! C++ `MainWindowOverlay.cpp:492-546` `downloadGeologyMap` + `GeologyMapService`:
//! 이미 있으면 켜고 끄기만 하고, 없으면 조사 폴더에 받아 「참조 지도」에 올린다.
//! 범위는 지금 화면이고, 긴 변이 80 km 가 되도록 넓혀 한 번에 받는다(다시 받지 않게).
//! 받는 일은 뒤의 스레드에서 한다.
//!
//! 아직 옮기지 못한 것: 공식 도폭 색 맞추기(WMS 그림에서 색 뽑기), 「지질 음영」 겹침,
//! 너무 크거나 느릴 때 넷으로 나눠 받기. 색은 C++ 이 색을 못 받았을 때 쓰는 지질시대 기준색이다.

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, OnceLock};

use eframe::egui;
use strata_geo::crs_transform::CrsTransformer;
use strata_net::kigam_geology::{fetch_geology, unit_color, GEOLOGY_LAYER_TITLE, GEOLOGY_MAX_SPAN_M};
use strata_types::crs::{CrsCode, WorkCrs};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::reference_layers::{ReferenceLayer, ReferenceShape, ReferenceStyle};

type Outcome = Result<ReferenceLayer, String>;

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

/// 받은 면(EPSG:5186)을 작업 좌표계의 참조 레이어로 만든다.
fn build_layer(polygons: Vec<strata_net::kigam_geology::GeologyPolygon>, work_crs: WorkCrs, source: std::path::PathBuf) -> ReferenceLayer {
    let to_work = CrsTransformer::new(CrsCode::Epsg5186, crs_code(work_crs));
    let shapes = polygons
        .into_iter()
        .map(|g| {
            let parts: Vec<Vec<[f64; 2]>> = g
                .rings
                .iter()
                .map(|ring| ring.iter().map(|p| to_work.transform_xy(p[0], p[1])).collect())
                .collect();
            let [r, gr, b] = unit_color(&g.era, &g.symbol);
            let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
            for p in parts.iter().flatten() {
                bbox = [bbox[0].min(p[0]), bbox[1].min(p[1]), bbox[2].max(p[0]), bbox[3].max(p[1])];
            }
            ReferenceShape {
                parts,
                label: g.symbol,
                bbox,
                // C++ `eraFillSymbol` — 채움 알파 220.
                fill: Some([r, gr, b, 220]),
            }
        })
        .collect();
    ReferenceLayer {
        name: GEOLOGY_LAYER_TITLE.to_owned(),
        source,
        shape_type: 5,
        shapes,
        // 테두리 (80,80,80) 0.12 mm, 기호 글자는 축척과 상관없이 보인다.
        style: ReferenceStyle {
            line_rgb: [80, 80, 80],
            line_mm: 0.12,
            label_max_mpp: f64::INFINITY,
            group: LegendGroup::ReferenceMap,
        },
    }
}

impl StrataAppState {
    /// 「지질」 단추. 이미 올린 지질도가 있으면 켜고 끄고, 없으면 받는다.
    pub(crate) fn toggle_or_download_geology(&mut self) {
        if self.toggle_existing_overlay(GEOLOGY_LAYER_TITLE) {
            return;
        }
        if job().lock().is_ok_and(|j| j.is_some()) {
            self.status_bar_hint = "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요.".to_owned();
            return;
        }
        // C++ `MainWindowDownloads.cpp:98-100` — 받은 자료는 조사 폴더에 둔다.
        let Some(folder) = self.survey_path.as_deref().and_then(|p| p.parent()).map(std::path::Path::to_path_buf) else {
            self.status_bar_hint = "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.".to_owned();
            return;
        };
        // 화면 범위(작업 좌표계). 긴 변이 80 km 가 되도록 가운데에서 넓힌다.
        let vp = &self.home_map.viewport;
        let half = [
            vp.screen_size_px[0] * 0.5 * vp.meters_per_pixel,
            vp.screen_size_px[1] * 0.5 * vp.meters_per_pixel,
        ];
        if half[0].max(half[1]) * 2.0 > GEOLOGY_MAX_SPAN_M {
            self.status_bar_hint = "범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요.".to_owned();
            return;
        }
        let grow = GEOLOGY_MAX_SPAN_M * 0.5 / half[0].max(half[1]).max(1.0);
        let (hx, hy) = (half[0] * grow, half[1] * grow);
        let [cx, cy] = vp.center_xy;
        let to_wgs = CrsTransformer::new(crs_code(self.work_crs), CrsCode::Epsg4326);
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for (x, y) in [(cx - hx, cy - hy), (cx - hx, cy + hy), (cx + hx, cy - hy), (cx + hx, cy + hy)] {
            let [lon, lat] = to_wgs.transform_xy(x, y);
            bbox = [bbox[0].min(lon), bbox[1].min(lat), bbox[2].max(lon), bbox[3].max(lat)];
        }

        let work_crs = self.work_crs;
        let dest = folder.join("참조자료").join("지질도_KIGAM.geojson");
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let cancel = AtomicBool::new(false);
            let outcome = fetch_geology(bbox, &dest, &cancel).and_then(|polygons| {
                if polygons.is_empty() {
                    Err("이 범위에는 받을 지질 단위가 없습니다.".to_owned())
                } else {
                    Ok(build_layer(polygons, work_crs, dest))
                }
            });
            let _ = sender.send(outcome);
        });
        if let Ok(mut slot) = job().lock() {
            *slot = Some(receiver);
        }
        self.status_bar_hint = "지질도(KIGAM 1:5만)를 받는 중입니다. 지도 작업을 계속할 수 있습니다.".to_owned();
    }

    /// 지질도 받기가 끝났는지 본다(지도 탭을 그릴 때마다 부른다).
    pub(crate) fn poll_geology_download(&mut self, ctx: &egui::Context) {
        let (pending, finished) = job()
            .lock()
            .map(|slot| (slot.is_some(), slot.as_ref().and_then(|r| r.try_recv().ok())))
            .unwrap_or((false, None));
        let Some(outcome) = finished else {
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
                let units = layer.shapes.len();
                self.install_reference_layer(layer);
                self.status_bar_hint = format!(
                    "지질도를 추가했습니다(지질 단위 면 {units}개). 공식 도폭 색 맞추기는 아직 옮기지 못해 지질시대 기준색으로 표시합니다."
                );
            }
            Err(why) => self.status_bar_hint = format!("지질도 내려받기 실패: {why}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_net::kigam_geology::parse_geology_geojson;

    #[test]
    fn received_units_become_filled_shapes_in_the_work_crs_labelled_by_symbol() {
        let json = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"시대":"현생이언 신생대 제4기","기호":"Qa","지층":"충적층"},
          "geometry":{"type":"Polygon","coordinates":[[[200000,450000],[200100,450000],[200100,450100],[200000,450100],[200000,450000]]]}}]}"#;
        let polys = parse_geology_geojson(json.as_bytes()).expect("해석");
        // 작업 좌표계가 5186 이면 좌표가 그대로다.
        let same = build_layer(polys.clone(), WorkCrs::Epsg5186, std::path::PathBuf::new());
        assert_eq!(same.name, "지질도(KIGAM 1:5만)");
        assert_eq!(same.shapes[0].parts[0][0], [200_000.0, 450_000.0]);
        assert_eq!(same.shapes[0].label, "Qa");
        let fill = same.shapes[0].fill.expect("채움");
        assert_eq!(fill[3], 220, "채움 알파는 C++ 과 같은 220");
        // 5187 이면 동쪽 원점 기준 좌표로 옮겨진다(서쪽으로 약 180 km).
        let moved = build_layer(polys, WorkCrs::Epsg5187, std::path::PathBuf::new());
        assert!(moved.shapes[0].parts[0][0][0] < 30_000.0, "{:?}", moved.shapes[0].parts[0][0]);
    }

    #[test]
    fn without_an_open_survey_the_button_explains_and_downloads_nothing() {
        let mut app = StrataAppState::new_startup();
        app.toggle_or_download_geology();
        assert!(app.status_bar_hint.starts_with("먼저 「새 조사」를 만들거나 조사를 여세요."));
        assert!(app.home_map.reference_layers.is_empty());
    }
}
