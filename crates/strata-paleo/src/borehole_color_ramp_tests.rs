//! `borehole_color_ramp` 시험 — C++ `ri(...)` 항목 1:1 대조.

use crate::borehole_color_ramp::*;

#[test]
fn five_ramps_match_cpp_hex_labels_and_ranges() {
    let alluv = alluvial_isopach_ramp();
    assert_eq!((alluv.min_value, alluv.max_value), (0.0, 8.0));
    assert_eq!(alluv.stops.len(), 5);
    assert_eq!(alluv.stops[0].color_hex, "#fef3c7");
    assert_eq!(alluv.stops[0].label, "0.5m 이하");
    assert_eq!(alluv.stops[2].value, 3.0);
    assert_eq!(alluv.stops[2].label, "3.0m 문화층 유망");
    assert_eq!(alluv.stops[4].color_hex, "#92400e");
    assert_eq!(alluv.stop_rgb(4), Some([0x92, 0x40, 0x0e]));

    let fill = fill_thickness_ramp();
    assert_eq!((fill.min_value, fill.max_value), (0.0, 5.0));
    assert_eq!(fill.stops.len(), 4);
    assert_eq!(fill.stops[3].label, "5.0m+ 심부 매립");

    let depth = bedrock_depth_ramp();
    assert_eq!((depth.min_value, depth.max_value), (0.0, 12.0));
    assert_eq!(depth.stops[0].label, "GL-1.5m 천부 기반암");
    assert_eq!(depth.stops[3].label, "GL-12.0m+ 옛 골짜기");

    let water = groundwater_ramp();
    assert_eq!((water.min_value, water.max_value), (0.5, 6.0));
    assert_eq!(water.stops[0].label, "GL-1.0m 양수기 필수");
    assert_eq!(water.stops[3].label, "GL-6.0m+ 양호");

    let elev = bedrock_elev_ramp(10.0, 30.0);
    assert_eq!(elev.stops[0].value, 10.0);
    assert_eq!(elev.stops[1].value, 20.0);
    assert_eq!(elev.stops[2].value, 30.0);
    assert_eq!(elev.stops[1].label, "옛 사면");
}

#[test]
fn hex_parser_and_linear_interpolation_between_stops() {
    assert_eq!(parse_hex_rgb("#fef3c7"), Some([254, 243, 199]));
    // C++ `ri()` 는 항상 '#rrggbb' 형태로만 받으므로 '#' 없는 표기는 거부한다.
    assert_eq!(parse_hex_rgb("92400e"), None);
    assert_eq!(parse_hex_rgb("#zzzzzz"), None);
    assert_eq!(parse_hex_rgb("#fff"), None);

    let ramp = BoreholeColorRamp {
        min_value: 0.0,
        max_value: 2.0,
        stops: vec![
            RampStop {
                value: 0.0,
                color_hex: "#000000",
                label: "a",
            },
            RampStop {
                value: 2.0,
                color_hex: "#ffffff",
                label: "b",
            },
        ],
    };
    // 범위 밖 클램프
    assert_eq!(interpolate_ramp_color(&ramp, -5.0), [0, 0, 0]);
    assert_eq!(interpolate_ramp_color(&ramp, 99.0), [255, 255, 255]);
    // 중간값 선형 보간 (1.0 -> 50%)
    assert_eq!(interpolate_ramp_color(&ramp, 1.0), [128, 128, 128]);
}

/// C++ `GeoInfoBoreholeMaps.cpp:71` `setOpacity(0.72)` — 래스터 알파가 72%다.
#[test]
fn rasterize_uses_cpp_opacity_72_percent_and_transparent_nodata() {
    let ramp = alluvial_isopach_ramp();
    // 0.5m 이하 구간값 + NoData
    let grid = [0.0_f64, 1.5, f64::NAN, f64::INFINITY];
    let rgba = rasterize_ramp_rgba(&grid, &ramp);

    assert_eq!(rgba.len(), 4);
    let expected_alpha = (BOREHOLE_RASTER_OPACITY * 255.0).round() as u8;
    assert_eq!(expected_alpha, 184, "0.72 * 255 = 183.6 -> 184");
    // 실제 값 셀은 모두 72% 알파
    assert_eq!(rgba[0][3], 184);
    assert_eq!(rgba[1][3], 184);
    // NoData 는 투명
    assert_eq!(rgba[2], [0, 0, 0, 0]);
    assert_eq!(rgba[3], [0, 0, 0, 0]);
}

/// 램프 양 끝을 벗어난 값은 양끝 색으로 잘린다(C++ QgsColorRampShader 와 같다).
#[test]
fn rasterize_clamps_to_ends_and_packs_rgba8() {
    let ramp = BoreholeColorRamp {
        min_value: 0.0,
        max_value: 2.0,
        stops: vec![
            RampStop { value: 0.0, color_hex: "#000000", label: "a" },
            RampStop { value: 2.0, color_hex: "#ffffff", label: "b" },
        ],
    };
    let rgba = rasterize_ramp_rgba(&[-5.0, 99.0, 1.0], &ramp);
    assert_eq!([rgba[0][0], rgba[0][1], rgba[0][2]], [0, 0, 0]);
    assert_eq!([rgba[1][0], rgba[1][1], rgba[1][2]], [255, 255, 255]);
    assert_eq!([rgba[2][0], rgba[2][1], rgba[2][2]], [128, 128, 128]);

    let packed = rasterize_ramp_rgba8(&[-5.0, 99.0, 1.0], &ramp);
    assert_eq!(packed.len(), 3 * 4);
    assert_eq!(&packed[0..4], &[0, 0, 0, 184]);
    assert_eq!(&packed[4..8], &[255, 255, 255, 184]);
}

/// 다섯 램프 모두 한 픽셀 이상 실제 색을 낸다(빈 램프가 아님).
#[test]
fn all_five_borehole_ramps_rasterize_to_visible_colors() {
    for (name, ramp) in [
        ("alluvial", alluvial_isopach_ramp()),
        ("fill", fill_thickness_ramp()),
        ("bedrock_depth", bedrock_depth_ramp()),
        ("bedrock_elev", bedrock_elev_ramp(10.0, 30.0)),
        ("groundwater", groundwater_ramp()),
    ] {
        let mid = (ramp.min_value + ramp.max_value) * 0.5;
        let px = rasterize_ramp_rgba(&[mid], &ramp);
        assert_eq!(px.len(), 1, "{name}");
        assert_eq!(px[0][3], 184, "{name}: alpha");
        // u8 세 개를 그냥 더하면 넘친다(#f59e0b = 245+158+11 = 414). u32 로 넓혀 더한다.
        let luma = u32::from(px[0][0]) + u32::from(px[0][1]) + u32::from(px[0][2]);
        assert!(
            luma > 0,
            "{name}: rasterized colour must not be pure black"
        );
    }
}
