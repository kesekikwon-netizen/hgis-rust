use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::real_survey::ring_area_m2;
use crate::ribbon::SubToolsMode;
use crate::right_inspector::InspectorCardTab;

/// UX_LAYOUT_SPEC.md 3절: 지도 탭 3단 분할 (좌 351 / 중 954 / 우 236)
pub const UX_MAP_LEFT_WIDTH: f32 = 351.0;
pub const UX_MAP_CANVAS_WIDTH: f32 = 954.0;
pub const UX_MAP_RIGHT_WIDTH: f32 = 236.0;

/// `KaShellGridControls.cpp:41-145` `KaShellGridSettings` — 2줄 격자 세부 설정
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellGridConfig {
    /// 0.1~10000 m
    pub step_m: f64,
    pub show_labels: bool,
    /// `미터 격자` = false, `경위도 격자` = true
    pub is_geo: bool,
    /// `SHELL_GRID_SWATCHES` index
    pub color_idx: usize,
    /// 0.0~360.0°, meters grid only
    pub rotation_deg: f32,
    /// 0.5~5.0
    pub line_width: f32,
    /// 0 = 실선, 1 = 점선, 2 = 점
    pub dash_idx: usize,
}

impl Default for ShellGridConfig {
    fn default() -> Self {
        Self {
            step_m: 20.0,
            show_labels: true,
            is_geo: false,
            color_idx: 0,
            rotation_deg: 0.0,
            line_width: 1.2,
            dash_idx: 1,
        }
    }
}

/// `KaShellGridControls.cpp:74-76` — 실선 / 점선 / 점
pub const SHELL_GRID_DASH_LABELS: [&str; 3] = ["실선", "점선", "점"];

/// `KaShellGridControls.cpp:83-87` — 격자 선 색 견본 셋
pub const SHELL_GRID_SWATCHES: [(Color32, &str); 3] = [
    (Color32::from_rgb(0xD9, 0x2B, 0x2B), "빨간색 격자"),
    (Color32::from_rgb(0x1D, 0x4E, 0xD8), "파란색 격자"),
    (Color32::from_rgb(0x1F, 0x29, 0x37), "검정색 격자"),
];

fn w2s(rect: Rect, vp: &ViewportTransform, xy: [f64; 2]) -> Pos2 {
    let [sx, sy] = vp.world_to_screen(xy);
    Pos2::new(rect.left() + sx as f32, rect.top() + sy as f32)
}

fn s2w(rect: Rect, vp: &ViewportTransform, pos: Pos2) -> [f64; 2] {
    vp.screen_to_world([
        f64::from(pos.x - rect.left()),
        f64::from(pos.y - rect.top()),
    ])
}

/// `KaMapControls.cpp:148-168` `KaMapScaleBar::refresh` — 화면 폭 120 px에 맞는 1·2·5·10 decade 미터값.
/// 지도 축척(`meters_per_pixel`)이 없거나 0이면 막대를 그리지 않도록 `None`.
#[must_use]
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
        (txt, "고른 그림의 밝기. 가운데가 원본입니다. 원본 파일은 바뀌지 않습니다.")
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
pub fn grid_label_text(
    grid_value: f64,
    origin_value: f64,
    draw_step: f64,
    turned: bool,
) -> String {
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

/// `KaCanvasGridOverlay.cpp:19-28` `paintBand` — 도곽 좌표 띠(위 18 px 가로, 왼쪽 18 px 세로)
pub const GRID_BAND_PX: f32 = 18.0;
pub const GRID_BAND_FILL: Color32 = Color32::from_rgb(0xF5, 0xF4, 0xED);
pub const GRID_BAND_BORDER: Color32 = Color32::from_rgb(0xDE, 0xDC, 0xD1);
pub const GRID_BAND_INK: Color32 = Color32::from_rgb(0x5E, 0x5D, 0x59);

/// 위·왼쪽 띠 사각형 두 개와 그 안쪽 경계선 좌표 (`KaCanvasGridOverlay.cpp:20-27`)
#[must_use]
pub fn grid_band_rects(rect: Rect) -> (Rect, Rect) {
    (
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), GRID_BAND_PX)),
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + GRID_BAND_PX),
            Vec2::new(GRID_BAND_PX, rect.height() - GRID_BAND_PX),
        ),
    )
}

/// `KaCanvasGridOverlay.cpp:107-194` — 지도 위 격자선과 도곽 띠 좌표 글씨를 페인터에 그린다.
pub fn paint_projected_grid(
    p: &egui::Painter,
    rect: Rect,
    vp: &ViewportTransform,
    cfg: &ShellGridConfig,
    survey_center: [f64; 2],
    is_geo: bool,
    labels: bool,
) {
    let ink = SHELL_GRID_SWATCHES[cfg.color_idx.min(2)].0;
    let stroke = grid_stroke_for(cfg, ink);
    let world_tl = s2w(rect, vp, rect.left_top());
    let world_br = s2w(rect, vp, rect.right_bottom());
    let min_wx = world_tl[0].min(world_br[0]);
    let max_wx = world_tl[0].max(world_br[0]);
    let min_wy = world_tl[1].min(world_br[1]);
    let max_wy = world_tl[1].max(world_br[1]);
    let turned = cfg.rotation_deg.abs() > 1e-3;
    let step = if cfg.step_m > 0.0 {
        cfg.step_m
    } else {
        nice_grid_step_meters(120.0 * vp.meters_per_pixel.max(1e-9))
    };
    if step.is_nan() || step <= 0.0 {
        return;
    }
    let span = (max_wx - min_wx).max(max_wy - min_wy);
    let stride = grid_line_stride(span, step, 80);
    let draw_step = step * stride as f64;
    let start_x = (min_wx / draw_step).floor() * draw_step;
    let start_y = (min_wy / draw_step).floor() * draw_step;
    let [origin_x, origin_y] = survey_center;

    let mut gx = start_x;
    while gx <= max_wx + draw_step * 0.5 {
        let sx = w2s(rect, vp, [gx, min_wy]).x;
        if sx >= rect.left() && sx <= rect.right() {
            for seg in grid_dash_segments(
                Pos2::new(sx, rect.top() + GRID_BAND_PX),
                Pos2::new(sx, rect.bottom()),
                cfg.dash_idx,
                stroke.width,
            ) {
                p.line_segment(seg, stroke);
            }
        }
        gx += draw_step;
    }
    let mut gy = start_y;
    while gy <= max_wy + draw_step * 0.5 {
        let sy = w2s(rect, vp, [min_wx, gy]).y;
        if sy >= rect.top() && sy <= rect.bottom() {
            for seg in grid_dash_segments(
                Pos2::new(rect.left() + GRID_BAND_PX, sy),
                Pos2::new(rect.right(), sy),
                cfg.dash_idx,
                stroke.width,
            ) {
                p.line_segment(seg, stroke);
            }
        }
        gy += draw_step;
    }
    if !labels {
        return;
    }
    if is_geo {
        return;
    }

    let (top_band, left_band) = grid_band_rects(rect);
    p.rect_filled(top_band, 0.0, GRID_BAND_FILL);
    p.rect_filled(left_band, 0.0, GRID_BAND_FILL);
    p.line_segment(
        [
            Pos2::new(rect.left() + GRID_BAND_PX, rect.top() + GRID_BAND_PX),
            Pos2::new(rect.right(), rect.top() + GRID_BAND_PX),
        ],
        Stroke::new(1.0, GRID_BAND_BORDER),
    );
    p.line_segment(
        [
            Pos2::new(rect.left() + GRID_BAND_PX, rect.top() + GRID_BAND_PX),
            Pos2::new(rect.left() + GRID_BAND_PX, rect.bottom()),
        ],
        Stroke::new(1.0, GRID_BAND_BORDER),
    );

    let band_font = FontId::monospace(10.0);
    let mut last_x = f32::NEG_INFINITY;
    let mut gx = start_x;
    while gx <= max_wx + draw_step * 0.5 {
        let sx = w2s(rect, vp, [gx, max_wy]).x;
        let lbl = grid_label_text(gx - origin_x, origin_x, draw_step, turned);
        if sx >= rect.left() + GRID_BAND_PX && sx <= rect.right() {
            let need = lbl.chars().count() as f32 * 6.0 + 8.0;
            if sx - last_x >= need {
                p.text(
                    Pos2::new(sx + 2.0, rect.top() + 9.0),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    band_font.clone(),
                    GRID_BAND_INK,
                );
                last_x = sx;
            }
        }
        gx += draw_step;
    }
    let mut last_y = f32::NEG_INFINITY;
    let mut gy = start_y;
    while gy <= max_wy + draw_step * 0.5 {
        let sy = w2s(rect, vp, [min_wx, gy]).y;
        let lbl = grid_label_text(gy - origin_y, origin_y, draw_step, turned);
        if sy >= rect.top() + GRID_BAND_PX && sy <= rect.bottom() {
            let need = lbl.chars().count() as f32 * 6.0 + 8.0;
            if sy - last_y >= need {
                p.text(
                    Pos2::new(rect.left() + GRID_BAND_PX / 2.0, sy),
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    band_font.clone(),
                    GRID_BAND_INK,
                );
                last_y = sy;
            }
        }
        gy += draw_step;
    }
}

/// 레이어별 투명도(`0.0..=1.0`)와 밝기(`0.0..=1.0`, `0.5`가 원본 `0`)를 실제 렌더링 색상에 반영
fn modulate_color_with_opacity_brightness(base: Color32, opacity: f32, brightness: f32) -> Color32 {
    let op = opacity.clamp(0.0, 1.0);
    let br_shift = ((brightness.clamp(0.0, 1.0) - 0.5) * 2.0 * 255.0).round() as i32;
    let r = (i32::from(base.r()) + br_shift).clamp(0, 255) as u8;
    let g = (i32::from(base.g()) + br_shift).clamp(0, 255) as u8;
    let b = (i32::from(base.b()) + br_shift).clamp(0, 255) as u8;
    let a = (f32::from(base.a()) * op).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// `KaLayerOpacityRail` 밝기 줄의 세 가지 모양 — 투명도 / 그림 레이어 밝기 / 벡터 「그림만」
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrightRow {
    Opacity,
    Raster,
    Vector,
}

impl StrataAppState {
    pub(crate) fn render_map_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let tab_clip = ui.max_rect();
        ui.set_clip_rect(tab_clip);
        self.render_cpp_left_layer_panel(ui, false);
        self.render_cpp_right_inspector(ui);

        let win_w = ctx.screen_rect().width();
        let avail_center_w = ui.available_width();
        let right_pad =
            if (1550.0..=1620.0).contains(&win_w) && avail_center_w > UX_MAP_CANVAS_WIDTH + 9.0 {
                ((avail_center_w - 9.0 - UX_MAP_CANVAS_WIDTH).round() as i8).max(4)
            } else {
                9
            };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(243, 241, 232))
                    .inner_margin(egui::Margin {
                        left: 9,
                        right: right_pad,
                        top: 0,
                        bottom: 0,
                    }),
            )
            .show_inside(ui, |ui| {
            let is_trench = self.modals.active == Some(ActiveModalDialog::TrenchGenerator)
                || self.active_tool == MapToolKind::TrenchPlace;
            let is_measure = self.active_tool == MapToolKind::Measure;
            let is_draw_subtool =
                self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw;
            let is_draw_tool = matches!(
                self.active_tool,
                MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
            ) || (is_draw_subtool && !is_measure && !is_trench);
            let is_select_tool = self.active_tool == MapToolKind::Select;
            let is_style = self.right_inspector.active_card == InspectorCardTab::LayerStyleCard;

            if is_trench {
                let trench_cnt = self.home_map.real_survey.trenches.len();
                let trench_area: f64 = self
                    .home_map
                    .real_survey
                    .trenches
                    .iter()
                    .map(|t| t.area_m2)
                    .sum();
                let survey_area: f64 = self.home_map.real_survey.survey_area_m2.max(1.0);
                let ratio_pct = (trench_area / survey_area * 100.0).clamp(0.0, 100.0);
                let shape_lbl = if self.modals.trench_shape.arm_bend().is_some() {
                    self.modals.trench_shape.label_ko()
                } else {
                    self.modals.trench_length.label_ko()
                };
                egui::Frame::NONE
                    .fill(Color32::from_rgb(225, 240, 218))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(145, 195, 138)))
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let _ = ui.small_button("II");
                            ui.colored_label(Color32::from_rgb(22, 138, 42), "✔");
                            ui.label(
                                egui::RichText::new(format!(
                                    "시굴격자: {shape_lbl} 시굴격자 {trench_cnt}개 · 총 {trench_area:.2}m² · 조사구역의 {ratio_pct:.1}% (시굴 기준 10%)"
                                ))
                                .size(12.0)
                                .strong(),
                            );
                        });
                    });
            }

            let avail_sz = Vec2::new(
                ui.available_width(),
                (tab_clip.bottom() - ui.cursor().top()).max(120.0),
            );
            let (rect, response) =
                ui.allocate_exact_size(avail_sz, egui::Sense::click_and_drag());
            self.home_map.viewport.screen_size_px =
                [f64::from(rect.width()), f64::from(rect.height())];

            let effective_tool = if is_draw_tool && self.active_tool == MapToolKind::Pan {
                MapToolKind::DrawPolygon
            } else {
                self.active_tool
            };

            if let Some(hp) = response.hover_pos() {
                self.home_map.cursor_world_xy = s2w(rect, &self.home_map.viewport, hp);
                if matches!(
                    effective_tool,
                    MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
                ) {
                    self.home_map.update_cursor_snap_preview();
                } else {
                    self.home_map.cad.preview_markers_world.clear();
                }
            }
            if response.hovered() {
                let sy = ui.input(|i| i.smooth_scroll_delta.y);
                let anchor = [
                    f64::from(rect.width()) * 0.5,
                    f64::from(rect.height()) * 0.5,
                ];
                if sy.abs() > 0.1 {
                    self.home_map
                        .viewport
                        .zoom_at(if sy > 0.0 { 0.85 } else { 1.18 }, anchor);
                }
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.home_map
                    .viewport
                    .pan_by_pixels(f64::from(d.x), f64::from(d.y));
            }
            if response.clicked() {
                if let Some(cp) = response.interact_pointer_pos() {
                    let wxy = s2w(rect, &self.home_map.viewport, cp);
                    match effective_tool {
                        MapToolKind::DrawPolygon
                        | MapToolKind::DrawLine
                        | MapToolKind::PlacePoint
                        | MapToolKind::Measure => {
                            let cur_layer = self.right_inspector.selected_layer_key.clone();
                            self.home_map.on_canvas_left_click_on_layer(
                                wxy,
                                effective_tool,
                                cur_layer.as_deref(),
                            );
                        }
                        MapToolKind::TrenchPlace => {
                            let len_m = self.modals.trench_length.meters();
                            let ring_5 = strata_geo::trench_grid::create_trench_rectangle_polygon(
                                wxy,
                                2.0,
                                len_m,
                                self.modals.trench_azimuth_deg,
                            );
                            let next_no = self.home_map.real_survey.trenches.len() + 1;
                            let name = format!("Tr-{next_no}");
                            let area_m2 = 2.0 * len_m;
                            self.home_map
                                .real_survey
                                .trenches
                                .push(crate::real_survey::RealTrenchPoly {
                                    name: name.clone(),
                                    ring: ring_5.0,
                                    area_m2,
                                    center: wxy,
                                });
                            self.is_dirty = true;
                            self.home_map.draw_guide_message = Some(format!(
                                "시굴격자 {name} ({area_m2:.1} m²)을(를) 배치했습니다 — 화살표 키(Shift: 1 m)로 미세 이동할 수 있습니다."
                            ));
                        }
                        MapToolKind::Select => {
                            let mpp = self.home_map.viewport.meters_per_pixel;
                            if let Some((idx, tr)) = self
                                .home_map
                                .real_survey
                                .trenches
                                .iter()
                                .enumerate()
                                .find(|(_, t)| {
                                    let min_x = t
                                        .ring
                                        .iter()
                                        .map(|p| p[0])
                                        .fold(f64::INFINITY, f64::min);
                                    let max_x = t
                                        .ring
                                        .iter()
                                        .map(|p| p[0])
                                        .fold(f64::NEG_INFINITY, f64::max);
                                    let min_y = t
                                        .ring
                                        .iter()
                                        .map(|p| p[1])
                                        .fold(f64::INFINITY, f64::min);
                                    let max_y = t
                                        .ring
                                        .iter()
                                        .map(|p| p[1])
                                        .fold(f64::NEG_INFINITY, f64::max);
                                    strata_canvas2d::canvas_painter::perform_canvas_hit_test_10px(
                                        Some(([min_x, min_y], [max_x, max_y], 1)),
                                        None,
                                        wxy,
                                        mpp,
                                        10.0,
                                    )
                                    .has_hit
                                })
                            {
                                let tr_name = tr.name.clone();
                                let tr_area = tr.area_m2;
                                self.right_inspector.selected_layer_key =
                                    Some("시굴격자".to_owned());
                                self.home_map.draw_guide_message = Some(format!(
                                    "도형 선택: 시굴격자 #{idx} ({tr_name}, {tr_area:.1} m²) — 화살표 키(0.1 m) 또는 Shift+화살표(1.0 m)로 이동"
                                ));
                            } else {
                                self.home_map.draw_guide_message = Some(format!(
                                    "클릭 좌표 (X {:.2}, Y {:.2}) 주변 10px 이내에 선택된 도형이 없습니다.",
                                    wxy[0], wxy[1]
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if response.secondary_clicked() {
                let ring = self.home_map.scene.active_rubberband.clone();
                if self
                    .home_map
                    .cad
                    .on_right_click(effective_tool, &mut self.home_map.scene)
                    && ring.len() >= 3
                {
                    let area_m2 = ring_area_m2(&ring);
                    self.home_map.real_survey.save_drawn_polygon(ring);
                    self.is_dirty = true;
                    self.home_map.draw_guide_message =
                        Some(format!("조사구역·유구 다각형을 저장했습니다 ({area_m2:.2} m²)."));
                }
            }

            let rect = rect.intersect(tab_clip);
            let p = ui.painter_at(rect);
            let bg_col = modulate_color_with_opacity_brightness(
                Color32::from_rgb(251, 250, 246),
                1.0,
                self.home_map.real_survey.layer_brightness,
            );
            p.rect_filled(rect, 0.0, bg_col);
            self.paint_gyeongju_heritage_canvas(ctx, &p, rect, is_trench || is_measure || is_style);

            // 1. KaDrawGuideBand (KaDrawGuideBand.cpp:28-70, MainWindowChrome.cpp:73-84)
            let guide_info: Option<(String, String)> = if is_draw_tool {
                if effective_tool == MapToolKind::PlacePoint {
                    Some((
                        "그리기 · 유물 위치".to_owned(),
                        "클릭한 자리에 점을 놓습니다".to_owned(),
                    ))
                } else if effective_tool == MapToolKind::DrawLine {
                    Some((
                        "그리기 · 유구 선".to_owned(),
                        "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned(),
                    ))
                } else {
                    let target = self
                        .right_inspector
                        .selected_layer_key
                        .as_deref()
                        .unwrap_or("조사구역");
                    let pt_cnt = self.home_map.scene.active_rubberband.len();
                    let title = if pt_cnt > 0 {
                        format!("그리기 · {target} · 점 {pt_cnt}")
                    } else {
                        format!("그리기 · {target}")
                    };
                    // KaDrawGuideBand.cpp:48-62 — m_measure 실시간 길이·면적
                    let mut measure_txt = String::new();
                    let ring = &self.home_map.scene.active_rubberband;
                    if ring.len() >= 2 {
                        let mut len_m = 0.0_f64;
                        for w in ring.windows(2) {
                            let dx = w[1][0] - w[0][0];
                            let dy = w[1][1] - w[0][1];
                            len_m += (dx * dx + dy * dy).sqrt();
                        }
                        measure_txt = if len_m >= 1000.0 {
                            format!("길이 {:.1} m ({:.2} km)", len_m, len_m / 1000.0)
                        } else {
                            format!("길이 {len_m:.1} m")
                        };
                        if ring.len() >= 3 {
                            let mut area = 0.0_f64;
                            for k in 0..ring.len() {
                                let a = ring[k];
                                let b = ring[(k + 1) % ring.len()];
                                area += a[0] * b[1] - b[0] * a[1];
                            }
                            measure_txt.push_str(&format!(
                                " · 면적 {:.1} ㎡",
                                area.abs() * 0.5
                            ));
                        }
                    }
                    if measure_txt.is_empty() {
                        Some((title, "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned()))
                    } else {
                        Some((title, measure_txt))
                    }
                }
            } else if is_measure {
                Some((
                    "줄자".to_owned(),
                    "두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다".to_owned(),
                ))
            } else if is_trench {
                Some((
                    "시굴 격자".to_owned(),
                    "격자를 놓을 원점을 클릭합니다".to_owned(),
                ))
            } else if is_select_tool {
                Some((
                    "도형선택".to_owned(),
                    "도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다".to_owned(),
                ))
            } else {
                None
            };

            let opacity_top_y = if let Some((title_txt, hint_txt)) = guide_info {
                let full_guide_txt = format!("{title_txt} — {hint_txt}");
                let pill_w =
                    ((full_guide_txt.chars().count() as f32) * 9.5 + 42.0).clamp(320.0, 520.0);
                let pill_r = Rect::from_min_size(
                    rect.min + Vec2::new(12.0, 12.0),
                    Vec2::new(pill_w, 34.0),
                );
                p.rect_filled(
                    pill_r,
                    6.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 248),
                );
                p.rect_stroke(
                    pill_r,
                    6.0,
                    Stroke::new(1.2, Color32::from_rgb(181, 87, 58)),
                    egui::StrokeKind::Middle,
                );
                // KaDrawGuideBand.cpp:25 — KaIcons::tinted("draw-polygon", 16, KaTheme::palette().accent)
                let ic = Pos2::new(pill_r.left() + 16.0, pill_r.center().y);
                let accent = Color32::from_rgb(181, 87, 58);
                let poly_pts = [
                    Pos2::new(ic.x - 5.0, ic.y - 4.0),
                    Pos2::new(ic.x + 4.0, ic.y - 5.0),
                    Pos2::new(ic.x + 5.0, ic.y + 4.0),
                    Pos2::new(ic.x - 4.0, ic.y + 5.0),
                    Pos2::new(ic.x - 5.0, ic.y - 4.0),
                ];
                p.add(egui::Shape::line(
                    poly_pts.to_vec(),
                    Stroke::new(1.5, accent),
                ));
                for &v in &poly_pts[..4] {
                    p.circle_filled(v, 2.0, accent);
                }
                p.text(
                    Pos2::new(pill_r.left() + 30.0, pill_r.center().y),
                    egui::Align2::LEFT_CENTER,
                    &full_guide_txt,
                    FontId::proportional(12.0),
                    Color32::from_rgb(28, 25, 23),
                );
                self.paint_undo_redo_pill(ui, &p, Pos2::new(pill_r.right() + 8.0, pill_r.top()));
                12.0 + 34.0 + 8.0
            } else {
                12.0
            };

            // 2. KaLayerOpacityRail (KaLayerOpacityRail.cpp:18-177) — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
            let selected_layer = self.right_inspector.selected_layer_key.clone();
            if let Some(layer_lbl) = selected_layer {
                let card_r = Rect::from_min_size(
                    rect.min + Vec2::new(12.0, opacity_top_y),
                    Vec2::new(260.0, 86.0),
                );
                self.paint_opacity_card(ctx, ui, &p, card_r, &layer_lbl);
                if opacity_top_y <= 13.0 {
                    self.paint_undo_redo_pill(
                        ui,
                        &p,
                        Pos2::new(card_r.right() + 8.0, card_r.top()),
                    );
                }
            } else {
                let idle_r = Rect::from_min_size(
                    rect.min + Vec2::new(12.0, opacity_top_y),
                    Vec2::new(260.0, 34.0),
                );
                p.rect_filled(
                    idle_r,
                    6.0,
                    Color32::from_rgba_unmultiplied(252, 251, 247, 248),
                );
                p.rect_stroke(
                    idle_r,
                    6.0,
                    Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                    egui::StrokeKind::Middle,
                );
                p.text(
                    Pos2::new(idle_r.left() + 12.0, idle_r.center().y),
                    egui::Align2::LEFT_CENTER,
                    "투명도·밝기: 조절할 레이어를 고르세요",
                    FontId::proportional(11.5),
                    Color32::from_rgb(65, 64, 60),
                );
                if opacity_top_y <= 13.0 {
                    self.paint_undo_redo_pill(
                        ui,
                        &p,
                        Pos2::new(idle_r.right() + 8.0, idle_r.top()),
                    );
                }
            }

            if is_style && self.right_inspector.show_draw_tooltip {
                let tip_r = Rect::from_min_size(
                    Pos2::new(rect.left() + 26.0, rect.top() - 26.0),
                    Vec2::new(418.0, 32.0),
                );
                let tp = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("ka_draw_tip"),
                ));
                tp.rect_filled(tip_r, 5.0, Color32::from_rgb(35, 34, 32));
                tp.text(
                    tip_r.center(),
                    egui::Align2::CENTER_CENTER,
                    "그리기 — 조사구역·유구 면과 선을 그립니다. 다시 누르면 도구를 닫고 이동합니다",
                    FontId::proportional(11.5),
                    Color32::WHITE,
                );
            }

            if is_measure {
                let mr = Rect::from_min_size(
                    rect.min + Vec2::new(284.0, 54.0),
                    Vec2::new(280.0, 178.0),
                );
                p.rect_filled(
                    mr,
                    8.0,
                    Color32::from_rgba_unmultiplied(248, 246, 240, 246),
                );
                p.rect_stroke(
                    mr,
                    8.0,
                    Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                    egui::StrokeKind::Middle,
                );
                let t1 = Rect::from_min_size(mr.min + Vec2::new(12.0, 12.0), Vec2::new(56.0, 28.0));
                let t2 = Rect::from_min_size(mr.min + Vec2::new(72.0, 12.0), Vec2::new(56.0, 28.0));
                let area_mode = self.home_map.real_survey.measure_area_tab;
                p.rect_filled(
                    t1,
                    3.0,
                    if !area_mode {
                        Color32::from_rgb(214, 208, 194)
                    } else {
                        Color32::WHITE
                    },
                );
                p.rect_filled(
                    t2,
                    3.0,
                    if area_mode {
                        Color32::from_rgb(214, 208, 194)
                    } else {
                        Color32::WHITE
                    },
                );
                p.text(
                    t1.center(),
                    egui::Align2::CENTER_CENTER,
                    "거리",
                    FontId::proportional(12.5),
                    Color32::BLACK,
                );
                p.text(
                    t2.center(),
                    egui::Align2::CENTER_CENTER,
                    "면적",
                    FontId::proportional(12.5),
                    Color32::from_rgb(80, 79, 74),
                );
                if ui
                    .interact(t1, ui.id().with("m_tab_dist"), egui::Sense::click())
                    .clicked()
                {
                    self.home_map.real_survey.measure_area_tab = false;
                }
                if ui
                    .interact(t2, ui.id().with("m_tab_area"), egui::Sense::click())
                    .clicked()
                {
                    self.home_map.real_survey.measure_area_tab = true;
                }

                // KaMeasureMapTool.cpp:331-355,400 — 좌표계 이름은 작업 좌표계에서 그대로 가져온다
                let auth_id = self.work_crs.auth_id();
                let readout = if let Some(m) = &self.home_map.cad.last_measure {
                    if area_mode {
                        format!("{:.2} m²", m.polygon_area_m2)
                    } else {
                        format!("{:.3} m", m.total_distance_m)
                    }
                } else {
                    "0.000 m".to_owned()
                };
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 58.0),
                    egui::Align2::LEFT_CENTER,
                    &readout,
                    FontId::proportional(19.0),
                    Color32::BLACK,
                );
                let detail = match (&self.home_map.cad.last_measure, area_mode) {
                    (Some(m), true) => format!(
                        "둘레 {:.2} m · 면적만 산출 · 작업좌표\n{auth_id} 평면",
                        m.polygon_perimeter_m
                    ),
                    (Some(m), false) => format!(
                        "{:.3} m · 도북 방위 · 작업좌표\n{auth_id} 평면",
                        m.total_distance_m
                    ),
                    (None, _) => format!(
                        "세 점 이상 찍고 우클릭 → 마침 · 작업좌표\n{auth_id} 평면"
                    ),
                };
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 86.0),
                    egui::Align2::LEFT_CENTER,
                    &detail,
                    FontId::proportional(11.5),
                    Color32::from_rgb(55, 54, 50),
                );
                for (bi, b_txt) in ["마침", "복사", "한 점 취소", "다시"].into_iter().enumerate() {
                    let br = Rect::from_min_size(
                        Pos2::new(mr.left() + 12.0 + (bi as f32) * 64.0, mr.top() + 110.0),
                        Vec2::new(60.0, 28.0),
                    );
                    p.rect_filled(br, 5.0, Color32::WHITE);
                    p.rect_stroke(
                        br,
                        5.0,
                        Stroke::new(1.0, Color32::from_rgb(175, 172, 162)),
                        egui::StrokeKind::Middle,
                    );
                    p.text(
                        br.center(),
                        egui::Align2::CENTER_CENTER,
                        b_txt,
                        FontId::proportional(11.5),
                        Color32::BLACK,
                    );
                    if ui
                        .interact(br, ui.id().with(("m_btn", bi)), egui::Sense::click())
                        .clicked()
                    {
                        match bi {
                            0 => {
                                let _ = self
                                    .home_map
                                    .cad
                                    .on_right_click(MapToolKind::Measure, &mut self.home_map.scene);
                            }
                            1 => {
                                // KaMeasureMapTool::copyText() — 한 줄이 아니라 여러 줄을 복사한다
                                let mut lines = Vec::new();
                                if let Some(m) = &self.home_map.cad.last_measure {
                                    if area_mode {
                                        lines.push(format!(
                                            "면적 {readout}"
                                        ));
                                        lines.push(format!(
                                            "둘레 {:.2} m",
                                            m.polygon_perimeter_m
                                        ));
                                    } else {
                                        lines.push(format!("거리 {readout}"));
                                    }
                                } else if area_mode {
                                    lines.push("면적 0.00 ㎡".to_owned());
                                } else {
                                    lines.push("거리 0.00 m".to_owned());
                                }
                                lines.push(format!("작업좌표 {auth_id} 평면"));
                                self.status_bar_hint =
                                    "줄자 숫자를 복사했습니다.".to_owned();
                                ui.ctx().copy_text(lines.join("\n"));
                            }
                            2 => self.home_map.cad.on_backspace(&mut self.home_map.scene),
                            _ => self.home_map.cad.on_escape(&mut self.home_map.scene),
                        }
                    }
                }
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 154.0),
                    egui::Align2::LEFT_CENTER,
                    "클릭 점 · 우클릭에서 마침 · Backspace 한 점 취소 · Esc\n지우기",
                    FontId::proportional(10.5),
                    Color32::from_rgb(110, 108, 102),
                );
            }

            // KaMapControls.cpp:70-109 — 우상단 + / − / 맞춤 버튼
            let zoom_r = Rect::from_min_size(
                Pos2::new(rect.right() - 44.0, rect.top() + 12.0),
                Vec2::new(32.0, 96.0),
            );
            p.rect_filled(zoom_r, 5.0, Color32::WHITE);
            p.rect_stroke(
                zoom_r,
                5.0,
                Stroke::new(1.0, Color32::from_rgb(190, 188, 178)),
                egui::StrokeKind::Middle,
            );
            for (i, (sym, tip)) in [
                ("+", "확대 (마우스 휠을 앞으로)"),
                ("−", "축소 (마우스 휠을 뒤로)"),
                ("□", "조사구역이 화면에 꽉 차게 봅니다"),
            ]
            .into_iter()
            .enumerate()
            {
                let zr = Rect::from_min_size(
                    Pos2::new(zoom_r.left(), zoom_r.top() + (i as f32) * 32.0),
                    Vec2::new(32.0, 32.0),
                );
                if i > 0 {
                    p.line_segment(
                        [
                            Pos2::new(zr.left() + 4.0, zr.top()),
                            Pos2::new(zr.right() - 4.0, zr.top()),
                        ],
                        Stroke::new(1.0, Color32::from_rgb(220, 218, 210)),
                    );
                }
                p.text(
                    zr.center(),
                    egui::Align2::CENTER_CENTER,
                    sym,
                    FontId::proportional(15.0),
                    Color32::from_rgb(45, 44, 40),
                );
                if ui
                    .interact(zr, ui.id().with(("map_zoom_btn", i)), egui::Sense::click())
                    .on_hover_text(tip)
                    .clicked()
                {
                    let center_px = [
                        f64::from(rect.width()) * 0.5,
                        f64::from(rect.height()) * 0.5,
                    ];
                    match i {
                        0 => self.home_map.viewport.zoom_at(0.8, center_px),
                        1 => self.home_map.viewport.zoom_at(1.25, center_px),
                        _ => self.home_map.fit_to_survey(),
                    }
                }
            }

            // KaMapScaleBar (KaMapControls.cpp:138-260) — 좌하단 도북 화살표 · 1·2·5 decade 축척 막대
            let bl = Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0);
            p.text(
                bl,
                egui::Align2::LEFT_CENTER,
                "▲ 도북",
                FontId::proportional(11.0),
                Color32::from_rgb(45, 44, 40),
            );
            if let Some(round_m) = scale_bar_metres(self.home_map.viewport.meters_per_pixel) {
                let per_pixel = self.home_map.viewport.meters_per_pixel.max(1e-6);
                let pixels = (round_m / per_pixel).clamp(40.0, 220.0) as f32;
                let sb = Rect::from_min_size(
                    Pos2::new(bl.x + 48.0, bl.y - 4.0),
                    Vec2::new(pixels, 6.0),
                );
                // 인쇄 축척 막대처럼 진한 칸·흰 칸을 번갈아 네 칸
                let ink = Color32::from_rgb(35, 35, 32);
                let quarter = sb.width() / 4.0;
                for i in 0..4 {
                    p.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(sb.left() + quarter * i as f32, sb.top()),
                            Vec2::new(quarter, sb.height()),
                        ),
                        0.0,
                        if i % 2 == 0 { ink } else { Color32::WHITE },
                    );
                }
                p.rect_stroke(sb, 0.0, Stroke::new(1.0, ink), egui::StrokeKind::Middle);
                let (start_txt, mid_txt, end_txt) = scale_bar_tick_text(round_m);
                let tick_font = FontId::monospace(10.0);
                p.text(
                    Pos2::new(sb.left(), sb.top() - 7.0),
                    egui::Align2::LEFT_CENTER,
                    start_txt,
                    tick_font.clone(),
                    Color32::from_rgb(45, 44, 40),
                );
                p.text(
                    Pos2::new(sb.center().x, sb.top() - 7.0),
                    egui::Align2::CENTER_CENTER,
                    mid_txt,
                    tick_font.clone(),
                    Color32::from_rgb(45, 44, 40),
                );
                p.text(
                    Pos2::new(sb.right(), sb.top() - 7.0),
                    egui::Align2::RIGHT_CENTER,
                    end_txt,
                    tick_font,
                    Color32::from_rgb(45, 44, 40),
                );
            }

            // 3. KaShellGridControls (KaShellGridControls.cpp:41-193) — 우하단 좌표격자 토글 · 격자 종류 · 간격 선택 · 라벨 표시 · 선 색상
            self.paint_shell_grid_controls(ctx, ui, &p, rect);
        });
    }

    /// `KaDrawGuideBand.cpp:55-69` — `[되돌리기]` · `[다시 실행]` 버튼 쌍 (실제 undo/redo 호출)
    fn paint_undo_redo_pill(&mut self, ui: &mut egui::Ui, p: &egui::Painter, min: Pos2) {
        let ur_r = Rect::from_min_size(min, Vec2::new(146.0, 34.0));
        p.rect_filled(ur_r, 6.0, Color32::WHITE);
        p.rect_stroke(
            ur_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );
        p.line_segment(
            [
                Pos2::new(ur_r.center().x, ur_r.top() + 4.0),
                Pos2::new(ur_r.center().x, ur_r.bottom() - 4.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
        );
        let undo_r = Rect::from_min_max(ur_r.min, Pos2::new(ur_r.center().x, ur_r.max.y));
        let redo_r = Rect::from_min_max(Pos2::new(ur_r.center().x, ur_r.min.y), ur_r.max);
        p.text(
            undo_r.center(),
            egui::Align2::CENTER_CENTER,
            "↺ 되돌리기",
            FontId::proportional(11.5),
            Color32::from_rgb(45, 44, 40),
        );
        p.text(
            redo_r.center(),
            egui::Align2::CENTER_CENTER,
            "↻ 다시 실행",
            FontId::proportional(11.5),
            Color32::from_rgb(45, 44, 40),
        );

        if ui
            .interact(
                undo_r,
                ui.id().with("ka_canvas_undo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("되돌리기 (Ctrl+Z) — 마지막 그리기·정점·삭제를 되돌립니다")
            .clicked()
        {
            if !self.home_map.scene.active_rubberband.is_empty() {
                self.home_map.cad.on_backspace(&mut self.home_map.scene);
            } else if self.home_map.real_survey.survey_rings.len() > 1 {
                if let Some(popped) = self.home_map.real_survey.survey_rings.pop() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("ka_redo_ring"), popped));
                }
            }
            self.home_map.draw_guide_message =
                Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
        }
        if ui
            .interact(
                redo_r,
                ui.id().with("ka_canvas_redo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("다시 실행 (Ctrl+Y) — 되돌린 편집을 다시 적용합니다")
            .clicked()
        {
            let restored = ui
                .ctx()
                .data_mut(|d| d.remove_temp::<Vec<[f64; 2]>>(egui::Id::new("ka_redo_ring")));
            if let Some(ring) = restored {
                self.home_map.real_survey.survey_rings.push(ring);
            }
            self.home_map.draw_guide_message =
                Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
        }
    }

    /// `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
    pub(crate) fn paint_opacity_card(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        card_r: Rect,
        title: &str,
    ) {
        p.rect_filled(
            card_r,
            6.0,
            Color32::from_rgba_unmultiplied(252, 251, 247, 248),
        );
        p.rect_stroke(
            card_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );
        p.text(
            Pos2::new(card_r.left() + 12.0, card_r.top() + 16.0),
            egui::Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0),
            Color32::from_rgb(28, 25, 23),
        );

        let opacity_val = self.home_map.real_survey.layer_opacity.clamp(0.0, 1.0);
        let bright_val = self.home_map.real_survey.layer_brightness.clamp(0.0, 1.0);
        let opacity_pct = (opacity_val * 100.0).round() as i32;
        let bright_int = ((bright_val - 0.5) * 2.0 * 255.0).round() as i32;
        let opacity_txt = format!("{opacity_pct}%");
        // KaLayerOpacityRail.cpp:156-173 — 벡터(지적도·유적 경계)에는 밝기가 없다
        let is_raster = is_raster_image_layer(title);
        let (bright_txt, bright_tip) = brightness_row_text(bright_int, is_raster);

        for (row_i, (lbl, pct_txt, knob_t, col, tip)) in [
            (
                "투명도",
                opacity_txt,
                opacity_val,
                Color32::from_rgb(181, 87, 58),
                "선택한 레이어의 투명도. 오른쪽으로 밀면 진해집니다. 지도에서만 흐려지고 자료는 그대로입니다.",
            ),
            (
                "밝기",
                bright_txt,
                bright_val,
                Color32::from_rgb(92, 90, 84),
                bright_tip,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let is_bright_row = row_i == 1;
            let row_kind = if is_bright_row {
                if is_raster {
                    BrightRow::Raster
                } else {
                    BrightRow::Vector
                }
            } else {
                BrightRow::Opacity
            };
            let y = card_r.top() + 40.0 + (row_i as f32) * 24.0;
            p.text(
                Pos2::new(card_r.left() + 12.0, y),
                egui::Align2::LEFT_CENTER,
                lbl,
                FontId::proportional(11.5),
                Color32::from_rgb(65, 64, 60),
            );
            let (sx0, sx1) = (card_r.left() + 58.0, card_r.right() - 48.0);
            let kx = sx0 + (sx1 - sx0) * knob_t;
            // 벡터 레이어의 밝기 줄은 값 자리에 「그림만」만 두고 손잡이를 없앤다
            if !matches!(row_kind, BrightRow::Vector) {
                p.line_segment([Pos2::new(sx0, y), Pos2::new(kx, y)], Stroke::new(3.5, col));
                p.line_segment(
                    [Pos2::new(kx, y), Pos2::new(sx1, y)],
                    Stroke::new(3.5, Color32::from_rgb(208, 205, 196)),
                );
                p.circle_filled(Pos2::new(kx, y), 6.0, Color32::WHITE);
                p.circle_stroke(Pos2::new(kx, y), 6.0, Stroke::new(2.0, col));
            } else {
                p.line_segment(
                    [Pos2::new(sx0, y), Pos2::new(sx1, y)],
                    Stroke::new(3.5, Color32::from_rgb(208, 205, 196)),
                );
            }
            p.text(
                Pos2::new(card_r.right() - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                &pct_txt,
                FontId::proportional(11.0),
                Color32::from_rgb(45, 44, 40),
            );

            let track_r =
                Rect::from_min_max(Pos2::new(sx0 - 6.0, y - 10.0), Pos2::new(sx1 + 6.0, y + 10.0));
            let resp = if matches!(row_kind, BrightRow::Vector) {
                // 벡터 밝기는 끌 수 없다 (KaLayerOpacityRail.cpp:158-166). 안내말만 남긴다.
                ui.interact(
                    track_r,
                    ui.id().with(("ka_opacity_slider", row_i)),
                    egui::Sense::hover(),
                )
            } else {
                ui.interact(
                    track_r,
                    ui.id().with(("ka_opacity_slider", row_i)),
                    egui::Sense::click_and_drag(),
                )
            }
            .on_hover_text(tip);
            if !matches!(row_kind, BrightRow::Vector) && (resp.clicked() || resp.dragged()) {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let t = ((pos.x - sx0) / (sx1 - sx0).max(1.0)).clamp(0.0, 1.0);
                    if row_i == 0 {
                        self.home_map.real_survey.layer_opacity = t;
                        self.home_map.set_basemap_opacity(t);
                    } else {
                        self.home_map.real_survey.layer_brightness = t;
                        self.home_map.set_basemap_brightness((t - 0.5) * 2.0);
                    }
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            egui::Id::new(("ka_layer_op_br", title)),
                            (
                                self.home_map.real_survey.layer_opacity,
                                self.home_map.real_survey.layer_brightness,
                            ),
                        );
                    });
                }
            }
        }
    }

    /// `KaShellGridControls.cpp:41-193` — 좌표격자 토글 + 2줄 `gridDetail`
    /// (미터·경위도, 간격, 회전 / 굵기, 선 모양, 색 견본 셋과 직접 고르기)
    fn paint_shell_grid_controls(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
    ) {
        let mut cfg = ctx.data_mut(|d| {
            d.get_temp::<ShellGridConfig>(egui::Id::new("ka_shell_grid_cfg"))
                .unwrap_or_default()
        });
        let enabled = self.home_map.grid_enabled;

        // KaShellGridControls.cpp:120-125 — 세부 설정은 「좌표격자」를 켤 때만 보인다
        let detail_h = if enabled { 60.0 } else { 0.0 };
        let bar_h = 28.0 + detail_h;
        let bar_w = 344.0;
        let bar_r = Rect::from_min_size(
            Pos2::new(rect.right() - bar_w - 12.0, rect.bottom() - bar_h - 10.0),
            Vec2::new(bar_w, bar_h),
        );
        p.rect_filled(
            bar_r,
            6.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 246),
        );
        p.rect_stroke(
            bar_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );

        let detail_r = Rect::from_min_size(bar_r.min, Vec2::new(bar_w, detail_h));
        if enabled {
            ui.scope_builder(egui::UiBuilder::new().max_rect(detail_r.shrink2(Vec2::new(8.0, 6.0))), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                // Row 0 — 미터/경위도 · 간격 · 회전 (KaShellGridControls.cpp:113-116)
                ui.horizontal(|ui| {
                    let kind_lbl = if cfg.is_geo { "경위도 격자" } else { "미터 격자" };
                    if ui
                        .selectable_label(cfg.is_geo, kind_lbl)
                        .on_hover_text(
                            "미터 격자는 작업 좌표계(5186·5187) 좌표로, 경위도 격자는 도·분·초로 긋습니다.",
                        )
                        .clicked()
                    {
                        cfg.is_geo = !cfg.is_geo;
                    }
                    ui.add_enabled(
                        !cfg.is_geo,
                        egui::DragValue::new(&mut cfg.step_m)
                            .range(0.1..=10000.0)
                            .speed(1.0)
                            .fixed_decimals(1)
                            .prefix("간격 ")
                            .suffix(" m"),
                    )
                    .on_hover_text("격자 간격(미터). 시굴격자를 옮길 때도 이 간격에 붙습니다.");
                    ui.add_enabled(
                        !cfg.is_geo,
                        egui::DragValue::new(&mut cfg.rotation_deg)
                            .range(0.0..=360.0)
                            .speed(1.0)
                            .fixed_decimals(1)
                            .prefix("회전 ")
                            .suffix("°"),
                    )
                    .on_hover_text("격자 회전 (동쪽 기준 시계 방향)");
                });
                // Row 1 — 굵기 · 선 모양 · 색 견본 (KaShellGridControls.cpp:116-118)
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut cfg.line_width)
                            .range(0.5..=5.0)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .prefix("굵기 "),
                    )
                    .on_hover_text("격자 선 굵기");
                    let dash_lbl = SHELL_GRID_DASH_LABELS[cfg.dash_idx.min(2)];
                    if ui
                        .selectable_label(false, dash_lbl)
                        .on_hover_text("격자 선 모양")
                        .clicked()
                    {
                        cfg.dash_idx = (cfg.dash_idx + 1) % SHELL_GRID_DASH_LABELS.len();
                    }
                    for (si, (scol, stip)) in SHELL_GRID_SWATCHES.into_iter().enumerate() {
                        let (sr, resp) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::click());
                        p.rect_filled(sr, 3.0, scol);
                        if cfg.color_idx == si {
                            p.rect_stroke(
                                sr,
                                3.0,
                                Stroke::new(1.5, Color32::from_rgb(38, 37, 34)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if resp.on_hover_text(stip).clicked() {
                            cfg.color_idx = si;
                        }
                    }
                    // 「…」 그 밖의 색을 직접 고른다 (KaShellGridControls.cpp:101-111)
                    if ui
                        .small_button("…")
                        .on_hover_text("그 밖의 색을 직접 고릅니다")
                        .clicked()
                    {
                        cfg.color_idx = (cfg.color_idx + 1) % SHELL_GRID_SWATCHES.len();
                    }
                });
            });
        }

        // 1) ☐ 좌표격자 체크박스 (KaShellGridControls.cpp:49-51, 124)
        let chk_r = Rect::from_min_size(
            Pos2::new(bar_r.right() - 88.0, bar_r.bottom() - 25.0),
            Vec2::new(82.0, 22.0),
        );
        let chk_lbl = if enabled { "☑ 좌표격자" } else { "☐ 좌표격자" };
        p.text(
            chk_r.center(),
            egui::Align2::CENTER_CENTER,
            chk_lbl,
            FontId::proportional(11.5),
            Color32::from_rgb(28, 25, 23),
        );
        if ui
            .interact(chk_r, ui.id().with("map_grid_toggle"), egui::Sense::click())
            .on_hover_text("지도에 좌표 격자를 켭니다. 켠 뒤 미터·경위도 격자를 고릅니다.")
            .clicked()
        {
            self.home_map.grid_enabled = !self.home_map.grid_enabled;
        }

        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("ka_shell_grid_cfg"), cfg);
        });
    }

    pub(crate) fn paint_gyeongju_heritage_canvas(
        &self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: Rect,
        cadastral_detail: bool,
    ) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        let [cx, cy] = rs.survey_center;
        let sel_layer = self.right_inspector.selected_layer_key.as_deref();

        let layer_color = |name: &str, base: Color32| -> Color32 {
            let (op, br) = if sel_layer == Some(name) {
                (rs.layer_opacity, rs.layer_brightness)
            } else {
                ctx.data_mut(|d| {
                    d.get_temp::<(f32, f32)>(egui::Id::new(("ka_layer_op_br", name)))
                        .unwrap_or((rs.layer_opacity, rs.layer_brightness))
                })
            };
            modulate_color_with_opacity_brightness(base, op, br)
        };

        if rs.is_layer_visible("지형 음영") || rs.is_layer_visible("지표모델(DSM) 30m · Coperni...")
        {
            let tl = w2s(rect, vp, [cx - 1050.0, cy + 720.0]);
            let br = w2s(rect, vp, [cx + 1050.0, cy - 720.0]);
            let dsm_r = Rect::from_min_max(tl, br);
            match self.modals.dem_applied_mode_idx {
                0 => {
                    // 등고 모드: 연한 지형 배경 + 등고선 밴드
                    let base = layer_color("지형 음영", Color32::from_rgb(228, 224, 208));
                    p.rect_filled(dsm_r, 0.0, base);
                    let contour_col = layer_color("지형 음영", Color32::from_rgb(138, 102, 62));
                    for idx in 1..=7 {
                        let ratio = (idx as f32) / 8.0;
                        let r_x = dsm_r.width() * 0.46 * ratio;
                        let r_y = dsm_r.height() * 0.44 * ratio;
                        let mut ring = Vec::with_capacity(25);
                        for step in 0..=24 {
                            let ang = (step as f32) / 24.0 * std::f32::consts::TAU;
                            ring.push(Pos2::new(
                                dsm_r.center().x + ang.cos() * r_x,
                                dsm_r.center().y + ang.sin() * r_y,
                            ));
                        }
                        p.add(egui::Shape::line(
                            ring,
                            Stroke::new(if idx % 2 == 0 { 1.4 } else { 0.8 }, contour_col),
                        ));
                    }
                }
                2 => {
                    // 고도 모드: 선택된 색 표현 프리셋(0: 전국 표준, 1: 저지대 강조, 2: 현재 화면 맞춤)에 따른 단계별 색 띠
                    let bands = 10usize;
                    let bh = dsm_r.height() / (bands as f32);
                    for bi in 0..bands {
                        let t = (bi as f32) / ((bands - 1) as f32);
                        let raw_col = match self.modals.dem_applied_preset_idx {
                            0 => Color32::from_rgb(
                                (90.0 + t * 110.0) as u8,
                                (175.0 - t * 35.0) as u8,
                                (95.0 + t * 60.0) as u8,
                            ),
                            1 => Color32::from_rgb(
                                (58.0 + t * 145.0) as u8,
                                (152.0 - t * 48.0) as u8,
                                (118.0 - t * 55.0) as u8,
                            ),
                            _ => Color32::from_rgb(
                                (68.0 + t * 165.0) as u8,
                                (168.0 - (t - 0.4).abs() * 120.0).clamp(65.0, 210.0) as u8,
                                (74.0 - t * 32.0) as u8,
                            ),
                        };
                        let seg_r = Rect::from_min_size(
                            Pos2::new(dsm_r.left(), dsm_r.top() + (bi as f32) * bh),
                            Vec2::new(dsm_r.width(), bh + 1.0),
                        );
                        p.rect_filled(seg_r, 0.0, layer_color("지형 음영", raw_col));
                    }
                }
                _ => {
                    // 기본 음영 모드
                    let dsm_col = layer_color("지형 음영", Color32::from_rgb(154, 198, 120));
                    p.rect_filled(dsm_r, 0.0, dsm_col);
                }
            }
        }

        if rs.is_layer_visible("지적도 · 조사 주변 5km") {
            let base_cad = if cadastral_detail {
                Color32::from_rgb(38, 37, 34)
            } else {
                Color32::from_rgba_unmultiplied(60, 58, 52, 135)
            };
            let cad_col = layer_color("지적도 · 조사 주변 5km", base_cad);
            let cad_s = Stroke::new(if cadastral_detail { 0.85 } else { 0.55 }, cad_col);
            for parcel in &rs.cadastral_parcels {
                let pts: Vec<Pos2> = parcel.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, cad_s));
            }
        }

        if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
            let green_col = layer_color("지정유산", Color32::from_rgb(34, 148, 52));
            let mag_col = layer_color("지정유산", Color32::from_rgb(176, 28, 128));
            let green_s = Stroke::new(1.3, green_col);
            let mag_s = Stroke::new(1.2, mag_col);
            for h in &rs.heritage_polys {
                if !rs.is_layer_visible(&h.layer_name) {
                    continue;
                }
                let pts: Vec<Pos2> = h.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(
                    pts,
                    if h.is_green { green_s } else { mag_s },
                ));
            }
        }

        if rs.is_layer_visible("조사구역") {
            let base_stroke = if let Some([r, g, b]) = self.modals.applied_line_color_rgb {
                Color32::from_rgb(r, g, b)
            } else {
                self.right_inspector.active_stroke_color()
            };
            let sc = layer_color("조사구역", base_stroke);
            let fill_col = if self.right_inspector.no_fill {
                Color32::TRANSPARENT
            } else {
                let base_fill = Color32::from_rgba_unmultiplied(sc.r(), sc.g(), sc.b(), 95);
                layer_color("조사구역", base_fill)
            };
            let stroke = if self.right_inspector.no_stroke {
                Stroke::NONE
            } else {
                Stroke::new(self.right_inspector.stroke_width_mm * 1.6, sc)
            };
            for ring in &rs.survey_rings {
                let pts: Vec<Pos2> = ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::convex_polygon(
                    pts.clone(),
                    fill_col,
                    Stroke::NONE,
                ));
                p.add(egui::Shape::line(pts, stroke));
            }
            if rs.is_label_visible("조사구역") {
                let sp = w2s(rect, vp, rs.survey_center);
                let lbl_r = Rect::from_center_size(sp, Vec2::new(66.0, 16.0));
                p.rect_filled(
                    lbl_r,
                    3.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
                p.text(
                    lbl_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.2} m²", rs.survey_area_m2),
                    FontId::proportional(10.0),
                    Color32::BLACK,
                );
            }
        }

        if rs.is_layer_visible("시굴격자") {
            let tr_col = layer_color("시굴격자", Color32::from_rgb(156, 62, 32));
            let tr_s = Stroke::new(1.2, tr_col);
            for (ti, tr) in rs.trenches.iter().enumerate() {
                let pts: Vec<Pos2> = tr.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, tr_s));
                if cadastral_detail && rs.is_label_visible("시굴격자") && ti % 14 == 3 {
                    let tp = w2s(rect, vp, tr.center);
                    p.text(
                        tp,
                        egui::Align2::CENTER_CENTER,
                        format!("{}\n({:.1} m²)", tr.name, tr.area_m2),
                        FontId::proportional(8.5),
                        Color32::WHITE,
                    );
                }
            }
        }

        if rs.is_layer_visible("주변 500m") && !rs.buffer_500m.is_empty() {
            let b_col = layer_color("주변 500m", Color32::from_rgb(184, 58, 24));
            let ring_pts: Vec<Pos2> = rs.buffer_500m.iter().map(|&pt| w2s(rect, vp, pt)).collect();
            p.add(egui::Shape::line(ring_pts, Stroke::new(2.6, b_col)));
        }
        if rs.show_buffer_1000m && !rs.buffer_1000m.is_empty() {
            let ring_pts: Vec<Pos2> = rs
                .buffer_1000m
                .iter()
                .map(|&pt| w2s(rect, vp, pt))
                .collect();
            p.add(egui::Shape::line(
                ring_pts,
                Stroke::new(2.2, Color32::from_rgb(148, 42, 18)),
            ));
        }
        if rs.is_layer_visible("주변 500m 거리") && rs.is_label_visible("주변 500m 거리") {
            for (lbl, pt) in &rs.dist_labels_500m {
                let bp = w2s(rect, vp, *pt);
                let b500 = Rect::from_center_size(bp, Vec2::new(36.0, 15.0));
                p.rect_filled(b500, 3.0, Color32::WHITE);
                p.text(
                    b500.center(),
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    FontId::proportional(9.5),
                    Color32::from_rgb(184, 58, 24),
                );
            }
        }

        // KaCanvasGridOverlay.cpp:101-194 — 좌표격자가 켜져 있으면 지도 위에 격자와 도곽 좌표 띠를 그린다
        if self.home_map.grid_enabled {
            let cfg = ctx.data_mut(|d| {
                d.get_temp::<ShellGridConfig>(egui::Id::new("ka_shell_grid_cfg"))
                    .unwrap_or_default()
            });
            let labels = cfg.show_labels;
            paint_projected_grid(p, rect, vp, &cfg, rs.survey_center, cfg.is_geo, labels);
        }

        // T9 KaTopographicImportDialog에서 지도에 추가한 수치지형도 레이어 렌더링 (styleTopographic: 회색 0.2mm 선)
        if self.modals.topo_maps_enabled && self.modals.topo_imported_count > 0 {
            let topo_col = layer_color(
                "수치지형도 1:5,000 (합침)",
                Color32::from_rgba_unmultiplied(115, 115, 115, 190),
            );
            for idx in 0..self.modals.topo_imported_count.min(6) {
                let offset = (idx as f64 - 2.5) * 140.0;
                let p1 = w2s(rect, vp, [cx - 880.0, cy + offset - 90.0]);
                let p2 = w2s(rect, vp, [cx + 880.0, cy + offset + 90.0]);
                p.line_segment([p1, p2], Stroke::new(1.0, topo_col));
            }
        }

        // T6 (1) KaControlPointDialog에서 추가한 GPS 기준점 심볼·라벨 렌더링
        for (cp_id, cp_xy, cp_z) in &self.modals.control_points_world {
            let sp = w2s(rect, vp, *cp_xy);
            if rect.contains(sp) {
                let tri = vec![
                    Pos2::new(sp.x, sp.y - 7.0),
                    Pos2::new(sp.x - 6.5, sp.y + 5.0),
                    Pos2::new(sp.x + 6.5, sp.y + 5.0),
                ];
                p.add(egui::Shape::convex_polygon(
                    tri,
                    Color32::from_rgb(220, 38, 38),
                    Stroke::new(1.5, Color32::WHITE),
                ));
                let lbl = match cp_z {
                    Some(z) => format!("{cp_id} ({z:.2}m)"),
                    None => cp_id.clone(),
                };
                p.text(
                    Pos2::new(sp.x + 9.0, sp.y),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    FontId::proportional(10.5),
                    Color32::from_rgb(185, 28, 28),
                );
            }
        }

        if self.home_map.cad.snap_enabled && self.home_map.cad.snap_preview_enabled {
            for &pt in &self.home_map.cad.preview_markers_world {
                let sp = w2s(rect, vp, pt);
                p.circle_filled(sp, 4.5, Color32::from_rgba_unmultiplied(255, 251, 235, 210));
                p.circle_stroke(
                    sp,
                    4.5,
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(217, 119, 6, 230)),
                );
            }
        }

        if !self.home_map.scene.active_rubberband.is_empty() {
            let mut active_pts: Vec<Pos2> = self
                .home_map
                .scene
                .active_rubberband
                .iter()
                .map(|&pt| w2s(rect, vp, pt))
                .collect();
            for &v in &active_pts {
                p.circle_filled(v, 4.0, Color32::from_rgb(214, 78, 22));
            }
            active_pts.push(w2s(rect, vp, self.home_map.cursor_world_xy));
            p.add(egui::Shape::line(
                active_pts,
                Stroke::new(2.2, Color32::from_rgb(214, 78, 22)),
            ));
        }
    }
}
