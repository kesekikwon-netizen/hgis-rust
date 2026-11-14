//! Tests for `canvas_center_longitude`
//! (`target/cpp-ref/src/app/MainWindowSession.cpp:131-144`).

use strata_canvas2d::viewport::ViewportTransform;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::status_bar_text::POINTS_PER_METER;
use crate::tabs_home_map::canvas_center_longitude;

#[test]
fn canvas_center_longitude_returns_none_without_view_or_outside_local_scale() {
    // target/cpp-ref/src/app/MainWindowSession.cpp:133-136 — no canvas, startup whole-country view,
    // narrow canvas (< 40 px), non-positive scale, scale > 1:500000, or invalid CRS returns unknown.
    assert_eq!(canvas_center_longitude(None, 5187), None);

    let startup = StrataAppState::new_startup();
    assert_eq!(
        canvas_center_longitude(
            Some(&startup.home_map.viewport),
            startup.work_crs.epsg_code(),
        ),
        None
    );

    let at_500001 = ViewportTransform::new(
        [200_000.0, 600_000.0],
        500_001.0 / POINTS_PER_METER,
        [1000.0, 600.0],
    );
    assert_eq!(
        canvas_center_longitude(Some(&at_500001), WorkCrs::Epsg5187.epsg_code()),
        None
    );

    let at_500000 = ViewportTransform::new(
        [200_000.0, 600_000.0],
        500_000.0 / POINTS_PER_METER,
        [1000.0, 600.0],
    );
    assert!(canvas_center_longitude(Some(&at_500000), WorkCrs::Epsg5187.epsg_code()).is_some());

    let narrow = ViewportTransform::new([200_000.0, 600_000.0], 1.0, [39.0, 600.0]);
    assert_eq!(canvas_center_longitude(Some(&narrow), 5187), None);

    let non_positive_scale = ViewportTransform {
        center_xy: [200_000.0, 600_000.0],
        meters_per_pixel: 0.0,
        screen_size_px: [1000.0, 600.0],
    };
    assert_eq!(
        canvas_center_longitude(Some(&non_positive_scale), 5187),
        None
    );

    let valid_view = ViewportTransform::new([200_000.0, 600_000.0], 1.0, [1000.0, 600.0]);
    assert_eq!(canvas_center_longitude(Some(&valid_view), 0), None);
    assert_eq!(canvas_center_longitude(Some(&valid_view), 999_999), None);

    let nan_center = ViewportTransform::new([f64::NAN, 600_000.0], 1.0, [1000.0, 600.0]);
    assert_eq!(canvas_center_longitude(Some(&nan_center), 5187), None);
}

#[test]
fn canvas_center_longitude_transforms_5186_and_5187_origins_via_proj() {
    // target/cpp-ref/src/app/MainWindowSession.cpp:138-140 — transforms view extent centre
    // from destinationCrs (EPSG:5186 / EPSG:5187) to EPSG:4326.
    let origin_view = ViewportTransform::new([200_000.0, 600_000.0], 1.0, [1000.0, 600.0]);

    let lon_5186 = canvas_center_longitude(Some(&origin_view), WorkCrs::Epsg5186.epsg_code());
    assert!(
        lon_5186.is_some_and(|lon| (lon - 127.0).abs() < 1e-6),
        "expected ~127.0 for EPSG:5186 origin, got {lon_5186:?}"
    );

    let lon_5187 = canvas_center_longitude(Some(&origin_view), WorkCrs::Epsg5187.epsg_code());
    assert!(
        lon_5187.is_some_and(|lon| (lon - 129.0).abs() < 1e-6),
        "expected ~129.0 for EPSG:5187 origin, got {lon_5187:?}"
    );
}
