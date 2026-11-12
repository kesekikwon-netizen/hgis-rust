//! 위성·지적이 「갖다 붙여놓은 것 같이 지저분하다」(사용자 지적 2026-11-12) 회귀 시험.
//!
//! - 위성 레이어 체크를 끄면 위성이 그려지지 않아야 한다.
//! - 3857 타일은 작업 좌표계(TM)에서 자오선 수렴각만큼 기운 사각형이다.
//!   이웃 타일의 경계 꼭짓점이 정확히 같아야 흰 줄(틈)이 생기지 않는다.
//! - 지적도는 C++ QGIS WMS 처럼 화면 해상도에 맞춰 받아야 한다.
//!   1 km 한 장을 늘려 붙이면 선이 굵고 흐려진다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{vec2, Pos2, Rect};
use strata_geo::crs_transform::lonlat_to_tm;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::basemap_mesh::{screen_tile_range, tile_mesh_points, TILE_MESH_STEPS};
use crate::basemap_paint::{tm_params_for, TileView};
use crate::basemap_tiles::{lon_lat_to_tile, TileKind};

/// 춘천 시내(사용자 화면) — EPSG:5187 중앙자오선 129° 에서 1.27° 서쪽이라 타일이 약 0.8° 기운다.
const CHUNCHEON: (f64, f64) = (127.73, 37.88);

fn chuncheon_view(mpp: f64) -> (TileView, Rect) {
    let rect = Rect::from_min_size(Pos2::ZERO, vec2(1480.0, 625.0));
    let params = tm_params_for(WorkCrs::Epsg5187).unwrap();
    let c = lonlat_to_tm(CHUNCHEON.0, CHUNCHEON.1, &params);
    let vp = strata_canvas2d::viewport::ViewportTransform::new(c, mpp, [1480.0, 625.0]);
    (TileView::from_viewport(&vp, rect, WorkCrs::Epsg5187), rect)
}

/// 레이어 패널에서 「위성지도 (VWorld)」 체크를 끄면 위성을 칠하지 않는다.
#[test]
fn test_satellite_not_painted_when_row_unchecked() {
    let mut app = StrataAppState::new_startup();
    // 지시 S1: 배경지도는 범례에 그 레이어가 있을 때만 칠한다(새 조사·열기가 올리는 두 줄 + 일반지도).
    app.left_layers.apply_default_basemaps();
    app.left_layers.add_vworld_base_layer();
    app.home_map.active_basemap = BasemapSource::VworldSatellite;
    assert_eq!(app.basemap_kind_to_paint(), Some(TileKind::Satellite));

    app.home_map.real_survey.toggle_layer_visible("위성지도 (VWorld)");
    assert_eq!(
        app.basemap_kind_to_paint(),
        None,
        "체크를 껐는데 위성이 계속 그려진다"
    );
    app.home_map.real_survey.toggle_layer_visible("위성지도 (VWorld)");
    assert_eq!(app.basemap_kind_to_paint(), Some(TileKind::Satellite));

    app.home_map.active_basemap = BasemapSource::VworldBase;
    assert_eq!(app.basemap_kind_to_paint(), Some(TileKind::Base));
    app.home_map.real_survey.toggle_layer_visible("일반지도 (VWorld)");
    assert_eq!(app.basemap_kind_to_paint(), None);
}

/// 「전체 끄기」는 위성·지적도 끈다. 「전체 켜기」는 다시 켠다.
#[test]
fn test_toggle_all_off_hides_satellite_and_cadastral() {
    let mut app = StrataAppState::new_startup();
    app.left_layers.apply_default_basemaps();
    app.home_map.active_basemap = BasemapSource::VworldSatellite;

    app.home_map.real_survey.set_all_layers_visible(false);
    assert_eq!(app.basemap_kind_to_paint(), None, "전체 끄기 뒤에도 위성이 남는다");
    assert!(!app
        .home_map
        .real_survey
        .is_layer_visible("연속지적도 (VWorld WMS)"));

    app.home_map.real_survey.set_all_layers_visible(true);
    assert_eq!(app.basemap_kind_to_paint(), Some(TileKind::Satellite));
    assert!(app
        .home_map
        .real_survey
        .is_layer_visible("연속지적도 (VWorld WMS)"));
}

/// 이웃 타일은 경계 꼭짓점을 정확히 같이 써야 한다(틈·어긋남 없음).
#[test]
fn test_neighbour_tiles_share_edges_in_tm() {
    let (view, rect) = chuncheon_view(1.56);
    let params = tm_params_for(WorkCrs::Epsg5187).unwrap();
    let z = 17;
    let (tx, ty) = lon_lat_to_tile(CHUNCHEON.0, CHUNCHEON.1, z);
    let a = tile_mesh_points(&view, &params, rect, z, tx, ty).unwrap();
    let right = tile_mesh_points(&view, &params, rect, z, tx + 1, ty).unwrap();
    let below = tile_mesh_points(&view, &params, rect, z, tx, ty + 1).unwrap();
    let n = TILE_MESH_STEPS;
    let w = n + 1;
    assert_eq!(a.len(), w * w);
    for j in 0..=n {
        assert_eq!(a[j * w + n], right[j * w], "오른쪽 타일과 경계가 어긋남 (행 {j})");
    }
    for i in 0..=n {
        assert_eq!(a[n * w + i], below[i], "아래 타일과 경계가 어긋남 (열 {i})");
    }
    // 이 자리에서는 타일이 실제로 기울어 있다 — 두 모서리로 축에 나란한
    // 사각형을 그리면(예전 방식) 이 차이만큼 흰 줄이 생긴다.
    let ne = a[n];
    let se = a[n * w + n];
    assert!(
        (ne.x - se.x).abs() > 1.0,
        "춘천(5187)에서 타일 동쪽 변은 기울어야 한다: NE.x={} SE.x={}",
        ne.x,
        se.x
    );
}

/// 지적 그림 한 픽셀이 화면 한 픽셀과 거의 같아야 한다(늘려 붙이지 않는다).
#[test]
fn test_cadastral_zoom_matches_screen_pixels() {
    use crate::basemap_cadastral::{
        cadastral_image_ground_mpp, cadastral_zoom_for, CADASTRAL_MAX_ZOOM, CADASTRAL_MIN_ZOOM,
    };
    let lat = CHUNCHEON.1;
    for m in [0.05, 0.1, 0.2, 0.37, 0.6, 0.9] {
        let z = cadastral_zoom_for(m, lat).unwrap();
        let ratio = cadastral_image_ground_mpp(z, lat) / m;
        assert!(
            (0.70..=1.42).contains(&ratio),
            "화면 {m} m/px 에서 z={z} 그림이 {ratio:.2}배로 늘어난다"
        );
    }
    // VWorld 는 요청 1픽셀이 2 m(3857)보다 거칠면 빈 그림을 준다(2026-11-12 실측:
    // 1.95 m 는 필지선, 2.39 m 는 빈 PNG). 그 사이는 z16 을 조금 줄여 그린다.
    let cos = lat.to_radians().cos();
    assert_eq!(cadastral_zoom_for(1.95 * cos, lat), Some(CADASTRAL_MIN_ZOOM));
    assert_eq!(cadastral_zoom_for(2.39 * cos, lat), None);
    assert_eq!(CADASTRAL_MIN_ZOOM, 16);
    assert_eq!(cadastral_zoom_for(0.001, lat), Some(CADASTRAL_MAX_ZOOM));
    assert_eq!(cadastral_zoom_for(f64::NAN, lat), None);
}

/// 지적 요청 경계는 정확한 웹 메르카토르 타일이고, 이웃과 경계를 같이 쓴다.
#[test]
fn test_cadastral_tile_bbox_is_exact_web_mercator_tile() {
    use crate::basemap_cadastral::cadastral_tile_bbox_3857;
    const H: f64 = 20_037_508.342_789_244;
    let s = 2.0 * H / 65_536.0;
    let b = cadastral_tile_bbox_3857(16, 55_883, 25_378);
    assert!((b[0] - (-H + 55_883.0 * s)).abs() < 1e-6);
    assert!((b[3] - (H - 25_378.0 * s)).abs() < 1e-6);
    assert!((b[2] - b[0] - s).abs() < 1e-6);
    assert!((b[3] - b[1] - s).abs() < 1e-6);
    let right = cadastral_tile_bbox_3857(16, 55_884, 25_378);
    let below = cadastral_tile_bbox_3857(16, 55_883, 25_379);
    assert_eq!(b[2], right[0]);
    assert_eq!(b[1], below[3]);
}

/// 지적이 보이는 축척이면 타일 수가 상한 안이어야 한다(넘으면 지적이 통째로 사라진다).
/// 1:25,000 처럼 넓게 보면 VWorld 가 빈 그림을 주므로 C++ 처럼 그리지 않는다.
#[test]
fn test_cadastral_tiles_fit_on_screen_at_common_scales() {
    use crate::basemap_cadastral::{cadastral_zoom_for, CADASTRAL_MAX_TILES};
    let params = tm_params_for(WorkCrs::Epsg5187).unwrap();
    for ppp in [1.0_f64, 1.5, 2.0] {
        for scale in [500.0, 2_500.0, 5_000.0, 7_500.0, 10_000.0] {
            let mpp = scale / 3780.0;
            let (view, rect) = chuncheon_view(mpp);
            let Some(z) = cadastral_zoom_for(mpp / ppp, CHUNCHEON.1) else {
                continue;
            };
            let (x0, y0, x1, y1) = screen_tile_range(&view, &params, rect, z).unwrap();
            let count = ((x1 - x0 + 1) * (y1 - y0 + 1)) as usize;
            assert!(
                count <= CADASTRAL_MAX_TILES,
                "1:{scale} 배율 {ppp} 에서 지적 타일 {count}장 — 상한 {CADASTRAL_MAX_TILES}"
            );
        }
        // 가까이 본 1:2,500 은 어느 배율에서도 보여야 한다.
        assert!(cadastral_zoom_for(2_500.0 / 3780.0 / ppp, CHUNCHEON.1).is_some());
        // 1:25,000 은 서버가 빈 그림을 주는 축척 — C++ 처럼 그리지 않는다.
        assert!(cadastral_zoom_for(25_000.0 / 3780.0 / ppp, CHUNCHEON.1).is_none());
    }
}

/// 지적 캐시는 줌마다 따로 둔다.
#[test]
fn test_cadastral_cache_path_has_zoom() {
    use crate::basemap_cadastral::cadastral_tile_path;
    let a = cadastral_tile_path(16, 100, 200);
    let b = cadastral_tile_path(17, 100, 200);
    assert_ne!(a, b);
    assert!(a
        .to_string_lossy()
        .replace('\\', "/")
        .ends_with("cadastral/16/100/200.png"));
}

fn wait_until(mut ok: impl FnMut() -> bool) {
    for _ in 0..400 {
        if ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("2초 안에 끝나지 않았다");
}

/// 지적 그림은 화면을 멈추지 않고 뒤에서 받는다. 같은 칸은 한 번만,
/// 실패한 칸은 매 프레임 다시 받지 않는다(예전에는 실패하면 매 프레임 15초씩 멈출 수 있었다).
#[test]
fn test_cadastral_fetch_runs_once_and_failures_wait() {
    use crate::basemap_cadastral_jobs::{CadastralFetcher, MAX_IN_FLIGHT};
    let f = CadastralFetcher::default();
    let k = (17_u8, 1_u32, 2_u32);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    assert!(f.request_with(k, None, move || {
        let _ = rx.recv();
        None
    }));
    assert!(f.is_in_flight(k));
    assert!(!f.request_with(k, None, || None), "같은 칸을 두 번 받으면 안 된다");
    tx.send(()).unwrap();
    wait_until(|| !f.is_in_flight(k));
    assert!(f.take_done().is_empty());
    assert!(
        !f.request_with(k, None, || None),
        "실패 직후 바로 다시 받으면 안 된다"
    );

    // 성공하면 그림이 넘어온다.
    let ok = (17_u8, 3_u32, 4_u32);
    assert!(f.request_with(ok, None, || {
        Some(eframe::egui::ColorImage::new(
            [1, 1],
            eframe::egui::Color32::WHITE,
        ))
    }));
    wait_until(|| !f.is_in_flight(ok));
    let done = f.take_done();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].0, ok);

    // 동시에 받는 수에는 상한이 있다.
    let (tx2, rx2) = std::sync::mpsc::channel::<()>();
    let rx2 = std::sync::Arc::new(std::sync::Mutex::new(rx2));
    for i in 0..MAX_IN_FLIGHT {
        let r = rx2.clone();
        assert!(f.request_with((18, 10 + i as u32, 0), None, move || {
            let _ = r.lock().map(|g| g.recv());
            None
        }));
    }
    assert!(!f.request_with((18, 999, 0), None, || None), "상한을 넘겨 받으면 안 된다");
    for _ in 0..MAX_IN_FLIGHT {
        let _ = tx2.send(());
    }
}
