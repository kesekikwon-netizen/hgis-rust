//! 지도 탭의 축척막대·격자 계산 — C++ `MeasureOps` / `KaShellGridControls` 대응.

use eframe::egui::{Color32, Pos2, Rect, Stroke};
use strata_canvas2d::viewport::ViewportTransform;

use super::gui_tab_map_consts::ShellGridConfig;
pub(crate) fn w2s(rect: Rect, vp: &ViewportTransform, xy: [f64; 2]) -> Pos2 {
    let [sx, sy] = vp.world_to_screen(xy);
    Pos2::new(rect.left() + sx as f32, rect.top() + sy as f32)
}

pub(crate) fn s2w(rect: Rect, vp: &ViewportTransform, pos: Pos2) -> [f64; 2] {
    vp.screen_to_world([
        f64::from(pos.x - rect.left()),
        f64::from(pos.y - rect.top()),
    ])
}

/// `KaMapControls.cpp:148-168` `KaMapScaleBar::refresh` — 화면 폭 120 px에 맞는 1·2·5·10 decade 미터값.
pub fn scale_bar_metres(meters_per_pixel: f64) -> Option<f64> {
    if !meters_per_pixel.is_finite() || meters_per_pixel <= 0.0 {
        return None;
    }
    let per_pixel = meters_per_pixel.max(1e-6);
    let wanted = per_pixel * 120.0;
    let decade = 10.0_f64.powf(wanted.log10().floor());
    let mut round_m = decade;
    for step in [2.0, 5.0, 10.0] {
        if step * decade <= wanted {
            round_m = step * decade;
        }
    }
    Some(round_m)
}

/// `KaMapControls.cpp:165-172` `ticks()` / `unit()` — `0` · 중앙 · 끝 세 숫자와 단위(`m` / `km`).
/// 1 m 미만 간격은 `labelDecimals`처럼 필요한 소수 자리까지만 적는다.
#[must_use]
pub fn scale_bar_tick_text(round_m: f64) -> (String, String, String) {
    let (end_val, unit_str) = if round_m >= 1000.0 {
        (round_m / 1000.0, "km")
    } else {
        (round_m, "m")
    };
    let mid_val = end_val * 0.5;
    let decimals: usize = if round_m >= 1.0 {
        0
    } else {
        (0..3u32)
            .find(|d| {
                let scale = 10.0_f64.powi(*d as i32);
                (round_m * scale - (round_m * scale).round()).abs() < 1e-4
            })
            .unwrap_or(3) as usize
    };
    (
        "0".to_owned(),
        format!("{mid_val:.decimals$}"),
        format!("{end_val:.decimals$} {unit_str}"),
    )
}

/// `KaLayerOpacityRail.cpp:156-173` `setBrightness` — 밝기는 위성·항공사진 같은 그림(래스터) 레이어에만
/// 있다. 벡터 레이어(`조사구역`·`시굴격자`·`지적도 · 조사 주변 5km`·`지정유산`·`주변 500m`)는 `false`.
#[must_use]
pub fn is_raster_image_layer(title: &str) -> bool {
    title.contains("지형 음영")
        || title.contains("DSM")
        || title.contains("위성")
        || title.contains("항공")
        || title.contains("고지형")
}

/// `KaLayerOpacityRail.cpp:167-169` — 그림 레이어는 `-255..=255`, 벡터는 `"그림만"`과 안내말.
#[must_use]
pub fn brightness_row_text(bright_int: i32, is_raster: bool) -> (String, &'static str) {
    if is_raster {
        let txt = if bright_int == 0 {
            "0".to_owned()
        } else {
            format!("{bright_int:+}")
        };
        (
            txt,
            "고른 그림의 밝기. 가운데가 원본입니다. 원본 파일은 바뀌지 않습니다.",
        )
    } else {
        (
            "그림만".to_owned(),
            "밝기는 위성·항공사진 같은 그림 레이어에만 있습니다.",
        )
    }
}

/// `CanvasGridMath::niceStepMeters` (`CanvasGridMath.h:9-21`) — `120 mpp`에 맞춘 격자 간격
#[must_use]
pub fn nice_grid_step_meters(target_meters: f64) -> f64 {
    if target_meters.is_nan() || target_meters <= 0.0 || !target_meters.is_finite() {
        return 10.0;
    }
    let mag = 10.0_f64.powf(target_meters.log10().floor());
    let n = target_meters / mag;
    if n < 1.5 {
        mag
    } else if n < 3.5 {
        2.0 * mag
    } else if n < 7.5 {
        5.0 * mag
    } else {
        10.0 * mag
    }
}

/// `CanvasGridMath::labelDecimals` (`CanvasGridMath.h:112-122`) — 간격·원점을 정확히 적는 최소 소수 자리
#[must_use]
pub fn grid_label_decimals(step: f64, origin: f64) -> usize {
    for decimals in 0..3 {
        let scale = 10.0_f64.powi(decimals);
        let step_exact = (step * scale - (step * scale).round()).abs() < 1e-4;
        let origin_exact = (origin * scale - (origin * scale).round()).abs() < 1e-4;
        if step_exact && origin_exact {
            return decimals as usize;
        }
    }
    3
}

/// `KaCanvasGridOverlay.cpp:166-180` — 회전한 격자는 동쪽 좌표 하나로 줄을 부를 수 없어
/// 격자 원점에서 잰 거리(`원점` / `+20 m` / `−20 m`)로 적는다.
#[must_use]
pub fn grid_label_text(grid_value: f64, origin_value: f64, draw_step: f64, turned: bool) -> String {
    if turned {
        if grid_value.abs() < draw_step * 1e-6 {
            return "원점".to_owned();
        }
        let decimals = grid_label_decimals(draw_step, 0.0);
        return format!(
            "{}{:.*} m",
            if grid_value > 0.0 { "+" } else { "−" },
            decimals,
            grid_value.abs()
        );
    }
    let value = grid_value + origin_value;
    if draw_step >= 1000.0 && grid_label_decimals(origin_value, 0.0) == 0 {
        format!(
            "{:.*} km",
            grid_label_decimals(draw_step / 1000.0, origin_value / 1000.0),
            value / 1000.0
        )
    } else {
        format!(
            "{:.*} m",
            grid_label_decimals(draw_step, origin_value),
            value
        )
    }
}

/// `CanvasGridMath::lineStride` (`CanvasGridMath.h:83-88`) — 선이 `kMaxLines`보다 많으면 n번째마다
#[must_use]
pub fn grid_line_stride(span: f64, step: f64, max_lines: usize) -> usize {
    if step.is_nan() || step <= 0.0 || span.is_nan() || span < 0.0 || max_lines < 1 {
        return 1;
    }
    if !(span / step).is_finite() {
        return 1;
    }
    let lines = (span / step).floor() + 1.0;
    if lines <= max_lines as f64 {
        1
    } else {
        (lines / max_lines as f64).ceil() as usize
    }
}

/// `KaCanvasGridOverlay.cpp:107-110` — `QPen(m_cfg.color, std::max(0.5, m_cfg.lineWidth))`.
/// egui는 점선·점 선을 낱개로 이어 그리므로 `0` 실선·`1` 점선·`2` 점의 길이를 여기서 정한다.
#[must_use]
pub fn grid_stroke_for(cfg: &ShellGridConfig, ink: Color32) -> Stroke {
    let width = f64::from(cfg.line_width).clamp(0.5, 5.0) as f32;
    Stroke::new(
        width,
        Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 150),
    )
}

/// C++ `MeasureOps::formatLengthM` — `MeasureOps.cpp:163-171`.
/// 음수·비유한 값은 `—`, 1 km 이상은 km(소수 3), 10 m 이상은 m(소수 2), 그 아래는 m(소수 3).
#[must_use]
pub fn format_length_m(meters: f64) -> String {
    if !meters.is_finite() || meters < 0.0 {
        return "—".to_owned();
    }
    if meters >= 1000.0 {
        return format!("{:.3} km", meters / 1000.0);
    }
    if meters >= 10.0 {
        return format!("{meters:.2} m");
    }
    format!("{meters:.3} m")
}

/// C++ `MeasureOps::formatAreaM2` — `MeasureOps.cpp:188-198`.
/// 음수·비유한 값은 `—`, 1 ha 이상은 `ha (㎡)`, 100 ㎡ 이상은 소수 1, 그 아래는 소수 2.
#[must_use]
pub fn format_area_m2(square_meters: f64) -> String {
    if !square_meters.is_finite() || square_meters < 0.0 {
        return "—".to_owned();
    }
    if square_meters >= 10000.0 {
        return format!(
            "{:.3} ha ({:.1} ㎡)",
            square_meters / 10000.0,
            square_meters
        );
    }
    if square_meters >= 100.0 {
        return format!("{square_meters:.1} ㎡");
    }
    format!("{square_meters:.2} ㎡")
}

/// C++ `MeasureOps::gridBearingDegrees` — `MeasureOps.cpp:73-82`.
/// 도북 기준 시계방향 방위각. 두 점이 같으면 `NaN`.
#[must_use]
pub fn grid_bearing_degrees(from: [f64; 2], to: [f64; 2]) -> f64 {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    if dx.abs() < 1e-12 && dy.abs() < 1e-12 {
        return f64::NAN;
    }
    let mut deg = dx.atan2(dy).to_degrees();
    if deg < 0.0 {
        deg += 360.0;
    }
    if deg >= 360.0 {
        deg -= 360.0;
    }
    deg
}

/// C++ `MeasureOps::polygonPerimeterMeters` — `MeasureOps.cpp:129-147`.
/// 점이 3개 미만이면 0(둘레가 성립하지 않는다).
#[must_use]
pub fn ring_perimeter_m(ring: &[[f64; 2]]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    (0..ring.len())
        .map(|i| {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            (b[0] - a[0]).hypot(b[1] - a[1])
        })
        .sum()
}

/// 격자 선 한 조각(`a`→`b`)을 `dash_idx` 모양대로 서브 세그먼트로 나눈다
#[must_use]
pub fn grid_dash_segments(a: Pos2, b: Pos2, dash_idx: usize, width: f32) -> Vec<[Pos2; 2]> {
    let (on, off) = match dash_idx {
        1 => ((6.0 * width).max(3.0), (3.0 * width).max(2.0)),
        2 => (width.max(1.0), (3.0 * width).max(3.0)),
        _ => return vec![[a, b]],
    };
    let d = b - a;
    let len = d.length();
    if !len.is_finite() || len <= 0.0 {
        return Vec::new();
    }
    let dir = d / len;
    let mut out = Vec::new();
    let mut t = 0.0;
    while t < len {
        let t1 = (t + on).min(len);
        out.push([a + dir * t, a + dir * t1]);
        t = t1 + off;
    }
    out
}
