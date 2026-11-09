//! 스플래시 지층 곡선과 팝오버 위치 계산.
//!
//! `theme_tokens` 에서 분리했다.

/// 스플래시 지층 단면 3차 베지어 곡선 데이터 (C++ `StrataLayerCurve` in `KaSplashStrata.cpp`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrataLayerCurve {
    pub layer_color_hex: &'static str,
    pub start_pt: [f64; 2],
    pub control_pt1: [f64; 2],
    pub control_pt2: [f64; 2],
    pub end_pt: [f64; 2],
}

/// OutCubic 이징 함수: `1.0 - (1.0 - clamp(x, 0, 1))^3` (C++ `ease` in `KaSplashStrata.cpp`)
#[must_use]
pub fn ease_out_cubic(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// 스플래시 지층 경계선 `index` (`0..4`)의 시간(`seconds`)에 따른 진행 비율 (`0.0..=1.0`)
/// (C++ `KaSplashStrata::boundaryReveal` 1:1 대응)
#[must_use]
pub fn splash_boundary_reveal(index: usize, seconds: f64) -> f64 {
    ease_out_cubic((seconds - 0.4 - 0.35 * (index as f64)) / 0.35)
}

/// 스플래시 표척(Staff)의 시간(`seconds`)에 따른 성장 비율 (`0.0..=1.0`)
/// (C++ `KaSplashStrata::staffReveal` 1:1 대응)
#[must_use]
pub fn splash_staff_reveal(seconds: f64) -> f64 {
    ease_out_cubic((seconds - 0.4) / 1.4)
}

/// 스플래시 하단 영역에 4개 퇴적 지층 단면 3차 베지어 곡선 생성
/// (C++ `KaSplashStrata::generateProceduralStrata` / `boundaries()` 1:1 대응)
#[must_use]
pub fn generate_procedural_strata_curves(
    splash_width: f64,
    splash_height: f64,
) -> [StrataLayerCurve; 4] {
    let sx = (splash_width / 700.0).max(0.1);
    let sy = (splash_height / 400.0).max(0.1);
    let base_rows: [(&str, [f64; 8]); 4] = [
        (
            "#D7CCC8",
            [0.0, 258.0, 200.0, 262.0, 480.0, 243.0, 632.0, 246.0],
        ),
        (
            "#BCAAA4",
            [0.0, 277.0, 190.0, 283.0, 490.0, 262.0, 632.0, 267.0],
        ),
        (
            "#A1887F",
            [0.0, 300.0, 210.0, 306.0, 480.0, 286.0, 632.0, 291.0],
        ),
        (
            "#8D6E63",
            [0.0, 324.0, 220.0, 328.0, 520.0, 312.0, 632.0, 316.0],
        ),
    ];
    base_rows.map(|(hex, r)| StrataLayerCurve {
        layer_color_hex: hex,
        start_pt: [r[0] * sx, r[1] * sy],
        control_pt1: [r[2] * sx, r[3] * sy],
        control_pt2: [r[4] * sx, r[5] * sy],
        end_pt: [r[6] * sx, r[7] * sy],
    })
}

/// 타깃 위젯 사각형(`target_rect = [x, y, w, h]`), 팝오버 크기(`[pop_w, pop_h]`), 화면 영역(`screen_rect = [sx, sy, sw, sh]`)에 대해 화면 밖 잘림을 방지하는 팝오버 좌상단 좌표 계산
/// (C++ `KaPopover::calculateClampedPosition` 1:1 대응)
#[must_use]
pub fn calculate_clamped_popover_position(
    target_rect: [i32; 4],
    popover_size: [i32; 2],
    screen_rect: [i32; 4],
) -> [i32; 2] {
    let margin = 8;
    let [tx, ty, tw, th] = target_rect;
    let [pw, ph] = popover_size;
    let [sx, sy, sw, sh] = screen_rect;

    let raw_x = tx + tw / 2 - pw / 2;
    let min_x = sx + margin;
    let max_x = (sx + sw - pw - margin).max(min_x);
    let clamped_x = raw_x.clamp(min_x, max_x);

    let below_y = ty + th + 6;
    let clamped_y = if below_y + ph <= sy + sh - margin {
        below_y
    } else {
        (ty - ph - 6).max(sy + margin)
    };
    [clamped_x, clamped_y]
}
