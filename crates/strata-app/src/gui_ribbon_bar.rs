use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::BasemapSource;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{SubToolsMode, PENINSULA_REGION_TILES};
use crate::ribbon_icons::{cpp_ribbon_tooltip, paint_ribbon_icon};

/// UX_LAYOUT_SPEC.md 1·2절: C++ 실측 상수 (창 1600×900 기준)
pub const UX_RIBBON_HEIGHT: f32 = 91.0;
pub const UX_VIEW_TAB_BAR_HEIGHT: f32 = 51.0;
pub const UX_RIBBON_BUTTON_SIZE: Vec2 = Vec2::new(69.0, 100.0);

/// C++ `MainWindowRibbon.cpp:229-235` 7그룹과 각 그룹의 단추 순서 1:1.
///
/// `d7afcc8` 은 「자료 받기」에 다섯 번째 단추 「시추」(`btnBoreholeFetch`)를 더했고,
/// 그 자리를 만들려고 리본 칩 최소 너비를 40 → 36 으로 10% 줄였다. 마스터플랜 Chapter 04
/// 목업도 「자료 받기 (5종 완성)」 = DEM·지적·수치지형·유산·시추, 「배경 지도」 =
/// 지형·등고선·토양·고지형·지질·수계·옛 지도 로 같다.
pub const UX_RIBBON_GROUPS: [(&str, &[&str]); 7] = [
    ("조사", &["새 조사", "열기", "저장", "다른 이름"]),
    ("기록", &["선택", "측거", "그리기", "시굴격자", "버퍼"]),
    ("자료 받기", &["DEM", "지적", "수치지형", "유산", "시추"]),
    (
        "배경 지도",
        &["지형", "등고선", "토양", "고지형", "지질", "수계", "옛 지도"],
    ),
    ("정합", &["정합"]),
    ("내보내기", &["도면", "인쇄", "GeoTIFF", "검수·제출"]),
    ("기타", &["더보기"]),
];

/// `KaTheme.h:97-98` `ribbonChipWidth = 36` / `40` — d7afcc8 의 10% 축소 배율.
/// 칩 폭 표를 이 배율로 줄여 「시추」가 더해져도 1줄 안에 들어오게 한다.
pub const RIBBON_CHIP_SCALE: f32 = 36.0 / 40.0;

/// `439-449` — 한 칩의 기준 폭(px, 12.5px 라벨 실측값). 배율을 곱하기 전 값이다.
#[must_use]
pub fn ribbon_chip_base_width(group_idx: usize, label: &str) -> f32 {
    match (group_idx, label) {
        (0, _) => 53.0,
        (1, "그리기" | "시굴격자") => 51.0,
        (1, _) => 47.0,
        (2, "수치지형") => 60.0,
        (2, _) => 47.0,
        (3, "옛 지도") => 51.0,
        (3, _) => 47.0,
        (4, _) => 47.0,
        (5, "GeoTIFF" | "검수·제출") => 60.0,
        (5, _) => 47.0,
        _ => 48.0,
    }
}

/// 7그룹의 `(시작 x, 폭)` 을 `UX_RIBBON_GROUPS` 에서 계산한다. 그룹 폭은 칩 폭 합 + 좌우 여백 2px.
/// 표를 손으로 고치지 않으므로 단추를 더해도 배치가 어긋나지 않는다.
#[must_use]
pub fn ribbon_group_layout() -> ([f32; 7], [f32; 7]) {
    let mut xs = [0.0_f32; 7];
    let mut widths = [0.0_f32; 7];
    let mut x = 9.0_f32;
    for (g_idx, (_, items)) in UX_RIBBON_GROUPS.iter().enumerate() {
        let sum: f32 = items
            .iter()
            .map(|label| ribbon_chip_base_width(g_idx, label) * RIBBON_CHIP_SCALE)
            .sum();
        xs[g_idx] = x;
        widths[g_idx] = sum + 2.0;
        x += widths[g_idx];
    }
    (xs, widths)
}

/// C++ `KaBeginnerRibbon` 은 폭을 재서 배치하므로 고정 표가 없다.
/// Rust 는 1600×900 목업 기준표(`UX_RIBBON_GROUP_XS`/`_WIDTHS`)를 같은 방식으로 계산해 쓴다.
#[must_use]
pub fn ux_ribbon_group_xs() -> [f32; 7] {
    ribbon_group_layout().0
}

#[must_use]
pub fn ux_ribbon_group_widths() -> [f32; 7] {
    ribbon_group_layout().1
}

/// `KaRegionLocator.cpp:156, 162-163, 192-194` — 동·읍·면·리 / 지번 칸 안내와 맨 아래 규칙 두 줄
pub const REGION_LOT_PLACEHOLDER: &str = "산 12-3";
pub const REGION_LOT_HINT: &str = "번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요.";
pub const REGION_BOTTOM_HINT: &str =
    "번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3";

/// `MainWindowAlign.cpp:721` — 정합 서브툴바 맨 앞 캡션 문구
pub const ALIGN_SUBTOOLBAR_CAPTION: &str = "  맞추기 › ";

/// `MainWindowAlign.cpp:724,726,730,745,793` — 정합 서브툴바 다섯 항목과 그 순서.
/// C++ 에는 이 다섯 개만 있다(기준점 CSV·CAD DXF 가져오기·품질 요약 줄 없음).
pub const ALIGN_SUBTOOLBAR_ITEMS: [&str; 5] =
    ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"];

/// C++ `GeorefService::Pair` — 왼쪽(그림 원본 픽셀) 좌표와 오른쪽(지도) 좌표 한 쌍
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlignPair {
    pub src: [f64; 2],
    pub map: [f64; 2],
}

/// C++ `MainWindow::m_alignTool` + `m_alignApplied` — 정합 세션 상태 1:1.
#[derive(Debug, Clone, Default)]
pub struct AlignSessionState {
    /// 맞출 도면 이름. `None` 이면 C++ `statusText()` 첫 줄(`맞출 도면을 고르세요`).
    pub target_layer: Option<String>,
    pub pairs: Vec<AlignPair>,
    /// C++ `m_haveFrom` — 왼쪽 점만 찍고 오른쪽 점을 기다리는 중
    pub pending_source: Option<[f64; 2]>,
    /// C++ `m_alignApplied` — 「이동」을 이미 눌렀다
    pub applied: bool,
    pub transform: Option<strata_geo::polygon_ops::HelmertTransform2D>,
    /// 「맞추기 저장」이 실제로 쓴 파일 (C++ `saveAligned` 의 `path`)
    pub saved_path: Option<String>,
}

impl AlignSessionState {
    /// C++ `KaAlignMapTool::statusText()` — `KaAlignMapTool.cpp:453-462`
    #[must_use]
    pub fn status_text(&self) -> String {
        if self.target_layer.is_none() {
            return "맞출 도면을 고르세요".to_owned();
        }
        if self.pending_source.is_some() {
            return "오른쪽 지도에서 같은 모서리를 찍으세요".to_owned();
        }
        if self.pairs.is_empty() {
            return "왼쪽 도면 모서리 → 오른쪽 지적 모서리".to_owned();
        }
        format!(
            "왼쪽 → 오른쪽 · {}점{}",
            self.pairs.len(),
            self.quality_summary()
        )
    }

    /// C++ `GeorefQuality::summary()` — `GeorefQuality.cpp:87-94`.
    /// C++ 은 4점부터 아핀 잔차를 재지만 여기서는 같은 점들로 유사변환(헬머트) 잔차를 잰다.
    #[must_use]
    fn quality_summary(&self) -> String {
        match self.pairs.len() {
            2 => " · 한 점 더 찍으면 기울기도 맞습니다".to_owned(),
            3 => " · 한 점 더 찍으면 어긋남을 잽니다".to_owned(),
            n if n < 4 => String::new(),
            _ => {
                let src: Vec<[f64; 2]> = self.pairs.iter().map(|p| p.src).collect();
                let dst: Vec<[f64; 2]> = self.pairs.iter().map(|p| p.map).collect();
                let Ok(transform) = strata_geo::polygon_ops::solve_helmert_georef(&src, &dst)
                else {
                    return String::new();
                };
                let max = self
                    .pairs
                    .iter()
                    .map(|p| {
                        let q = transform.apply(p.src);
                        (q[0] - p.map[0]).hypot(q[1] - p.map[1])
                    })
                    .fold(0.0_f64, f64::max);
                format!(
                    " · 평균 어긋남 {:.2} m · 최대 {:.2} m",
                    transform.rms_error_m, max
                )
            }
        }
    }

    /// C++ `KaAlignMapTool::removeLastPair()` — `KaAlignMapTool.cpp:319-329`.
    /// 왼쪽 점만 찍혀 있으면 그것부터 지우고, 아니면 마지막 점 쌍을 지운다.
    pub fn remove_last_pair(&mut self) -> bool {
        if self.pending_source.take().is_some() {
            return true;
        }
        self.pairs.pop().is_some()
    }

    /// C++ `KaAlignMapTool::endSession()` — `KaAlignMapTool.cpp:177-198`
    pub fn end_session(&mut self) {
        self.pairs.clear();
        self.pending_source = None;
        self.applied = false;
        self.transform = None;
        self.target_layer = None;
        self.saved_path = None;
    }
}

fn draw_buffer_pill(
    ui: &mut egui::Ui,
    icon_key: &str,
    badge_bg: bool,
    text: &str,
    width: f32,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), egui::Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, 6.0, Color32::WHITE);
    p.rect_stroke(
        r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
        egui::StrokeKind::Middle,
    );
    if !icon_key.is_empty() {
        let ir = Rect::from_center_size(
            Pos2::new(r.left() + 16.0, r.center().y),
            Vec2::new(20.0, 20.0),
        );
        if badge_bg {
            p.rect_filled(ir, 4.0, Color32::from_rgb(236, 233, 224));
        }
        paint_ribbon_icon(&p, ir.expand(2.0), icon_key, Color32::from_rgb(38, 37, 34));
        p.text(
            Pos2::new(r.left() + 31.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    } else {
        p.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    }
    resp.clicked()
}

/// C++ `BasemapDsm.h:21` `copernicusTitle()` = `지표모델(DSM) 30m · Copernicus`.
/// Rust 범례(`gui_layer_panel.rs`)는 이 이름을 `지표모델(DSM) 30m · Coperni...` 로 줄여
/// 쓰므로 DEM 토글은 두 키를 함께 맞춘다.
const DEM_LEGEND_KEY: &str = "지표모델(DSM) 30m · Coperni...";

impl StrataAppState {
    /// C++ `MainWindowRibbon.cpp` + `KaBeginnerRibbon.cpp` 1:1 (Images 1, 2, 4, 9, 10, 11, 12, 15)
    pub(crate) fn render_cpp_top_bars(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_home = self.active_tab == MainTab::Home;
        let dirty_save = !is_home && self.is_dirty;
        let [pr, pg, pb, _] = self.theme.chrome;
        ui.spacing_mut().item_spacing.y = 0.0;

        let avail_for_ribbon = (ui.available_width() - 266.0).max(640.0);
        let h_scale = (avail_for_ribbon / 1328.0).clamp(0.72, 1.38);
        let scaled_tile = (39.0 * h_scale).clamp(28.0, 56.0);
        let extra_h = (scaled_tile - 39.0).max(0.0);
        let ribbon_h = UX_RIBBON_HEIGHT + extra_h;

        let (ribbon_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ribbon_h),
            egui::Sense::hover(),
        );
        {
            let rp = ui.painter_at(ribbon_rect);
            rp.rect_filled(ribbon_rect, 0.0, Color32::from_rgb(pr, pg, pb));
            rp.line_segment(
                [
                    Pos2::new(ribbon_rect.left(), ribbon_rect.bottom() - 1.0),
                    Pos2::new(ribbon_rect.right(), ribbon_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(218, 215, 205)),
            );
        }

        let groups: &[(&str, &[&str])] = &UX_RIBBON_GROUPS;
        let group_xs = ribbon_group_layout().0;
        let group_widths = ribbon_group_layout().1;

        let chip_base_w = |g_idx: usize, label: &str| -> f32 {
            ribbon_chip_base_width(g_idx, label) * RIBBON_CHIP_SCALE
        };

        for (g_idx, (g_title, items)) in groups.iter().enumerate() {
            let gx = ribbon_rect.left() + group_xs[g_idx] * h_scale;
            let gw = group_widths[g_idx] * h_scale;
            let g_rect = Rect::from_min_size(
                Pos2::new(gx, ribbon_rect.top() + 1.0),
                Vec2::new(gw, ribbon_h - 2.0),
            );
            let gp = ui.painter_at(ribbon_rect);
            if g_idx > 0 {
                gp.line_segment(
                    [
                        Pos2::new(gx, g_rect.top() + 3.0),
                        Pos2::new(gx, g_rect.bottom() - 3.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(218, 215, 205)),
                );
            }
            gp.text(
                Pos2::new(g_rect.left() + 6.0, g_rect.top() + 9.0),
                egui::Align2::LEFT_CENTER,
                *g_title,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(94, 93, 89),
            );
            let mut bx = g_rect.left() + 2.0 * h_scale;
            for &label in items.iter() {
                let cw = chip_base_w(g_idx, label) * h_scale;
                let btn_rect = Rect::from_min_size(
                    Pos2::new(bx, g_rect.top() + 18.0),
                    Vec2::new(cw, 69.0 + extra_h),
                );
                bx += cw;
                // MainWindowRibbon.cpp:572 / MainWindow.cpp:1086 — GeoTIFF 는 「지도」 탭에서만
                // 켜진다. 다른 탭(홈·도면·고지형·입체지형)에서는 흐리게 그린다.
                let muted = (is_home && *g_title == "기록")
                    || (label == "GeoTIFF" && self.active_tab != MainTab::Map);
                let highlight_save = label == "저장" && dirty_save;
                let dark_active = (label == "선택" && self.active_tool == MapToolKind::Select)
                    || (label == "측거" && self.active_tool == MapToolKind::Measure)
                    || (label == "그리기"
                        && (self.active_tool == MapToolKind::Measure
                            || (self.ribbon.sub_toolbar_visible
                                && self.ribbon.sub_mode == SubToolsMode::Draw)))
                    || (label == "버퍼"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Buffer)
                    || (label == "정합"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Align);
                let warm_active = label == "DEM" && !is_home;
                let mut chip_ui = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
                let (chip_clicked, chip_secondary) = self.draw_ribbon_chip(
                    &mut chip_ui,
                    label,
                    muted,
                    highlight_save,
                    dark_active,
                    warm_active,
                );
                if chip_clicked {
                    if label == "옛 지도" || label == "더보기" {
                        self.open_ribbon_context_menu(label, btn_rect);
                    } else {
                        self.handle_ribbon_click(label);
                    }
                }
                // KaPaleoMenu::install 등 우클릭 팝업
                if chip_secondary {
                    self.open_ribbon_context_menu(label, btn_rect);
                }
            }
        }

        // KaAppBar.cpp:25-58 — [📍 지역] 버튼 + [🔍 주소·지번 찾기] 입력칸
        let right_left = (ribbon_rect.left() + 1334.0 * h_scale).max(ribbon_rect.right() - 264.0);
        let right_rect = Rect::from_min_max(
            Pos2::new(right_left, ribbon_rect.top() + 1.0),
            Pos2::new(ribbon_rect.right() - 8.0, ribbon_rect.bottom() - 1.0),
        );
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let search_placeholder = if self.display_options.paper_look {
                "주소·지번 찾기"
            } else {
                "도로명·지번 (예: 하회종가길 40)  Ctrl+F"
            };
            let resp = ui
                .add_sized(
                    [156.0, 32.0],
                    egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                        .hint_text(format!("     {search_placeholder}"))
                        .vertical_align(egui::Align::Center),
                )
                .on_hover_text("도로명·지번으로 찾습니다 (예: 하회종가길 40) · Ctrl+F");
            // 검색칸 안쪽 왼쪽 돋보기 아이콘(16px, KaAppBar.cpp:41)
            {
                let sp = ui.painter();
                let c = Pos2::new(resp.rect.left() + 12.0, resp.rect.center().y - 1.0);
                let ink = Color32::from_rgb(94, 93, 89);
                sp.circle_stroke(c, 4.8, Stroke::new(1.5, ink));
                sp.line_segment(
                    [
                        Pos2::new(c.x + 3.4, c.y + 3.4),
                        Pos2::new(c.x + 7.2, c.y + 7.2),
                    ],
                    Stroke::new(1.6, ink),
                );
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.run_app_bar_search();
            }
            let (reg_rect, reg_resp) =
                ui.allocate_exact_size(Vec2::new(76.0, 36.0), egui::Sense::click());
            self.ribbon.app_bar.region_button_rect = Some([
                reg_rect.min.x,
                reg_rect.min.y,
                reg_rect.max.x,
                reg_rect.max.y,
            ]);
            {
                let rp = ui.painter_at(reg_rect);
                let active_or_hover = reg_resp.hovered()
                    || self.ribbon.app_bar.region_popup_open
                    || self.ribbon.app_bar.address_popup_open;
                let bg = if active_or_hover {
                    Color32::from_rgb(240, 238, 230)
                } else {
                    Color32::WHITE
                };
                let border_col = if active_or_hover {
                    Color32::from_rgb(181, 87, 58)
                } else {
                    Color32::from_rgb(194, 192, 182)
                };
                rp.rect_filled(reg_rect, 8.0, bg);
                rp.rect_stroke(
                    reg_rect,
                    8.0,
                    Stroke::new(1.0, border_col),
                    egui::StrokeKind::Inside,
                );
                // 지도 핀 아이콘 (KaIcons "gps", 16px, KaAppBar.cpp:32)
                let pin_c = Pos2::new(reg_rect.left() + 19.0, reg_rect.center().y - 2.0);
                let pin_ink = Color32::from_rgb(94, 93, 89);
                rp.circle_stroke(pin_c, 4.5, Stroke::new(1.5, pin_ink));
                rp.circle_filled(pin_c, 1.6, pin_ink);
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x - 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x + 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.text(
                    Pos2::new(reg_rect.left() + 31.0, reg_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "지역",
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(20, 20, 19),
                );
            }
            if reg_resp
                .on_hover_text("시·도를 고른 뒤 시·군·동과 지번으로 찾아갑니다")
                .clicked()
            {
                self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
                if self.ribbon.app_bar.region_popup_open {
                    self.ribbon.app_bar.address_popup_open = false;
                }
            }
        });

        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::Frame::NONE
                .fill(Color32::from_rgb(245, 243, 236))
                .stroke(Stroke::new(1.0, Color32::from_rgb(218, 215, 205)))
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Draw => {
                                let wbtn = |ui: &mut egui::Ui, txt: &str| -> egui::Response {
                                    ui.add(egui::Button::new(egui::RichText::new(txt).size(12.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(5.0).min_size(Vec2::new(0.0, 26.0)))
                                };
                                ui.label(egui::RichText::new("그리기 ›").size(12.5).strong());
                                if wbtn(ui, "▶ 도형선택").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.active_tool = MapToolKind::Select;
                                }
                                ui.separator();
                                // KaSnapSettingsWidget.cpp:16-35 — 자석 / 공유 경계 / 허용치 / 붙을 대상
                                ui.checkbox(&mut self.home_map.cad.snap_enabled, "자석")
                                    .on_hover_text(
                                        "켜면 조사구역·유구·지적 선에 붙습니다. 위성 그림에는 붙지 않습니다",
                                    );
                                ui.checkbox(&mut self.home_map.cad.snap_topological, "공유 경계")
                                    .on_hover_text(
                                        "켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집",
                                    );
                                ui.label("허용치");
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.home_map.cad.snap_tolerance_px,
                                    )
                                    .range(4.0..=64.0)
                                    .speed(1.0)
                                    .fixed_decimals(0)
                                    .suffix(" px"),
                                )
                                .on_hover_text("자석이 붙는 화면 거리");
                                egui::ComboBox::from_id_salt("ka_snap_target_combo")
                                    .selected_text(if self.home_map.cad.snap_target_current_only {
                                        "현재 레이어"
                                    } else {
                                        "모든 조사 레이어"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            false,
                                            "모든 조사 레이어",
                                        );
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            true,
                                            "현재 레이어",
                                        );
                                    })
                                    .response
                                    .on_hover_text(
                                        "조사 도형과 지적 선에 붙습니다. 위성 그림은 제외",
                                    );
                                let assist_resp = ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("▼").size(11.5))
                                            .fill(Color32::WHITE)
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                            .corner_radius(5.0)
                                            .min_size(Vec2::new(31.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기",
                                    );
                                if assist_resp.clicked() {
                                    self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
                                }
                                if self.ribbon.snap_assist_menu_open {
                                    let popup_pos = Pos2::new(assist_resp.rect.left(), assist_resp.rect.bottom() + 4.0);
                                    egui::Area::new(ui.id().with("ka_snap_assist_menu"))
                                        .fixed_pos(popup_pos)
                                        .order(egui::Order::Foreground)
                                        .show(ctx, |ui| {
                                            egui::Frame::popup(ui.style())
                                                .fill(Color32::from_rgb(252, 251, 247))
                                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(188.0);
                                                    let snap_now = ui
                                                        .button("가까운 선에 붙이기")
                                                        .on_hover_text(
                                                            "지적도·사업자 도면 위에 그린 도형(방금 그린 도형 또는 선택한 도형)을\n가장 가까운 지적선·도면선에 자동으로 붙이고 빠진 사이 꺾임점도 채웁니다 (Ctrl+Z 되돌리기 가능).",
                                                        );
                                                    if snap_now.clicked() {
                                                        let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                        if changed > 0 {
                                                            self.is_dirty = true;
                                                        }
                                                        self.ribbon.snap_assist_menu_open = false;
                                                    }
                                                    ui.separator();
                                                    let prev_trace = self.home_map.cad.snap_trace_enabled;
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_trace_enabled, "선 따라 자동채움")
                                                        .on_hover_text(
                                                            "1번 기능(선 따라 빠진 사이 자석점 자동 채우기)\n· 그릴 때 자석점 한두 칸을 건너뛰어도 사이 꼭짓점을 선을 따라 자동으로 채우며,\n  이미 그린 도형이 있을 때 누르면 가장 가까운 선에 바로 붙입니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_trace_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        if on && !prev_trace {
                                                            let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                            if changed == 0 {
                                                                self.home_map.draw_guide_message = Some(
                                                                    "선 따라 자동채움 켜짐 — 자석점을 한두 칸 건너뛰어 찍어도 사이 꼭짓점을 자동으로 채웁니다."
                                                                        .to_owned(),
                                                                );
                                                            } else {
                                                                self.is_dirty = true;
                                                            }
                                                        } else if !on {
                                                            self.home_map.draw_guide_message = Some("선 따라 자동채움 꺼짐".to_owned());
                                                        }
                                                    }
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_preview_enabled, "근접 자석점 표시")
                                                        .on_hover_text(
                                                            "3번 기능(마우스가 가까이 갔을 때만 주변 자석점 미리 표시)\n· 마우스가 선·꼭짓점에 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_preview_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        self.home_map.draw_guide_message = Some(if on {
                                                            "근접 자석점 표시 켜짐 — 마우스가 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다."
                                                                .to_owned()
                                                        } else {
                                                            "근접 자석점 표시 꺼짐".to_owned()
                                                        });
                                                    }
                                                });
                                        });
                                }
                                ui.separator();
                                for (b_lbl, tool) in [("／ 쉽게그리기", MapToolKind::DrawPolygon), ("□ 조사구역 그리기", MapToolKind::DrawPolygon), ("◇ 유구 그리기", MapToolKind::DrawPolygon), ("━ 유구 선", MapToolKind::DrawLine), ("● 유물위치표시", MapToolKind::PlacePoint)] {
                                    if wbtn(ui, b_lbl).clicked() {
                                        self.active_tab = MainTab::Map;
                                        self.active_tool = tool;
                                        self.home_map.update_guide_for_tool(tool);
                                    }
                                }
                                ui.separator();
                                let _ = wbtn(ui, "연속 그리기");
                                ui.separator();
                                for op in ["폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기"] {
                                    if wbtn(ui, op).clicked() {
                                        let msg = match op {
                                            "폴리곤 묶기" => {
                                                self.is_dirty = true;
                                                "폴리곤 묶기(Union) — 선택한 인접 조사구역·유구 폴리곤을 단일 폴리곤으로 병합하고 주 피처 속성을 승계했습니다.".to_owned()
                                            }
                                            "폴리곤 나누기" => {
                                                self.is_dirty = true;
                                                "폴리곤 나누기(Split) — 절단선 관통 검사를 통과하여 대상 폴리곤을 2개 조각으로 분할했습니다.".to_owned()
                                            }
                                            "구간 분리" => {
                                                self.is_dirty = true;
                                                "구간 분리(Isolate Part) — 내부 도넛 홀(수혈·주초)을 독립 유구 폴리곤으로 분리했습니다.".to_owned()
                                            }
                                            _ => {
                                                self.is_dirty = true;
                                                "겹친 곳 지우기(Difference) — 중복 침범 영역을 도려내고 슬라이버(0.05 m² 미만)를 제거했습니다.".to_owned()
                                            }
                                        };
                                        self.home_map.draw_guide_message = Some(msg);
                                    }
                                }
                                if wbtn(ui, "닫기").clicked() { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                                if draw_buffer_pill(ui, "버퍼", true, "500m", 78.0) {
                                    self.home_map.real_survey.toggle_layer_visible("주변 500m");
                                    self.home_map.draw_guide_message = Some("조사구역 주변 500m 경계를 갱신했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "버퍼", true, "1000m", 84.0) {
                                    self.home_map.real_survey.show_buffer_1000m = !self.home_map.real_survey.show_buffer_1000m;
                                    self.home_map.draw_guide_message = Some("조사구역 주변 1,000m 경계를 지도에 표시했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "수치지형", false, "엑셀 내보내기", 118.0) {
                                    if let Some(out) = self.home_map.real_survey.export_nearby_heritage_csv() {
                                        self.home_map.draw_guide_message = Some(format!("주변유적 목록을 내보냈습니다: {}", out.display()));
                                    }
                                }
                                if draw_buffer_pill(ui, "", false, "닫기", 48.0) { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Align => {
                                // MainWindowAlign.cpp:716-797 showSubToolsAlign — C++ 항목은
                                // 이동(:724) · 점 지우기(:726) · 되돌리기(:730) ·
                                // 맞추기 저장(:745) · 닫기(:793) 다섯 개뿐이다.
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(ALIGN_SUBTOOLBAR_CAPTION)
                                            .size(12.5)
                                            .strong(),
                                    )
                                    .selectable(false),
                                );
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[0]).clicked() {
                                    // MainWindowAlign.cpp:627-703 applyAlignMove
                                    match self.apply_align_move() {
                                        Ok(()) => {}
                                        Err(message) => {
                                            // :632 KaUserError::warn({「이동」, 「도면을 옮기지 못했습니다」, err})
                                            self.home_map.draw_guide_message =
                                                Some(format!("도면을 옮기지 못했습니다 — {message}"));
                                        }
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[1]).clicked() {
                                    // MainWindowAlign.cpp:726-729 — 지운 뒤 상태줄에 statusText()
                                    self.align.remove_last_pair();
                                    self.status_bar_hint = self.align.status_text();
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[2]).clicked() {
                                    // MainWindowAlign.cpp:730-744 — 되돌린 결과를 상태줄(성공)이나
                                    // 알림(실패)으로 보여 준다. 문구는 KaAlignMapToolEdit.cpp:132-161.
                                    let (restored, message) = self.restore_align_originals();
                                    self.status_bar_hint = message.clone();
                                    if !restored {
                                        self.home_map.draw_guide_message =
                                            Some(format!("맞추기 — {message}"));
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[3]).clicked() {
                                    // MainWindowAlign.cpp:745-792 맞추기 저장
                                    let saved = self.save_align_result();
                                    self.status_bar_hint = saved.1;
                                    self.home_map.draw_guide_message = Some(saved.0);
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[4]).clicked() {
                                    // MainWindowAlign.cpp:793-794 → hideSubTools(:769-777)
                                    // → :771-772 stopAlignSession
                                    self.hide_sub_tools();
                                }
                            }
                            SubToolsMode::None => {}
                        }
                    });
                });
        }

        let tabbar_h = if ctx.pixels_per_point() > 1.2 {
            34.0
        } else {
            UX_VIEW_TAB_BAR_HEIGHT
        };
        let (tabbar_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), tabbar_h),
            egui::Sense::hover(),
        );
        {
            let tbp = ui.painter_at(tabbar_rect);
            tbp.rect_filled(tabbar_rect, 0.0, Color32::from_rgb(0xF5, 0xF4, 0xED));
            tbp.line_segment(
                [
                    Pos2::new(tabbar_rect.left(), tabbar_rect.bottom() - 1.0),
                    Pos2::new(tabbar_rect.right(), tabbar_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(214, 211, 200)),
            );
        }
        let tabbar_inner = tabbar_rect.shrink2(Vec2::new(12.0, 2.0));
        let mut tabbar_ui = ui.new_child(egui::UiBuilder::new().max_rect(tabbar_inner));
        tabbar_ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (tab, ico, txt) in [
                (MainTab::Home, "유산", "홈"),
                (MainTab::Map, "옛 지도", "지도"),
            ] {
                let sel = self.active_tab == tab;
                let (tr, resp) =
                    ui.allocate_exact_size(Vec2::new(64.0, tabbar_h - 4.0), egui::Sense::click());
                let tp = ui.painter_at(tr);
                paint_ribbon_icon(
                    &tp,
                    Rect::from_center_size(
                        Pos2::new(tr.left() + 12.0, tr.center().y),
                        Vec2::new(26.0, 26.0),
                    ),
                    ico,
                    Color32::from_rgb(45, 44, 40),
                );
                tp.text(
                    Pos2::new(tr.left() + 28.0, tr.center().y),
                    egui::Align2::LEFT_CENTER,
                    txt,
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(30, 30, 28),
                );
                if sel {
                    tp.line_segment(
                        [
                            Pos2::new(tr.left(), tr.bottom()),
                            Pos2::new(tr.right(), tr.bottom()),
                        ],
                        Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                    );
                }
                if resp.clicked() {
                    self.active_tab = tab;
                }
            }
            let open_tabs = self.open_studio_tabs.clone();
            for (extra, ico, lbl) in [
                (MainTab::Drawing, "도면", "도면"),
                (MainTab::Paleo, "고지형", "고지형"),
                (MainTab::Terrain3d, "검수·제출", "입체지형"),
            ] {
                if open_tabs.contains(&extra) || self.active_tab == extra {
                    let sel = self.active_tab == extra;
                    let (tr, resp) =
                        ui.allocate_exact_size(Vec2::new(92.0, 36.0), egui::Sense::click());
                    let tp = ui.painter_at(tr);
                    paint_ribbon_icon(
                        &tp,
                        Rect::from_center_size(
                            Pos2::new(tr.left() + 12.0, tr.center().y),
                            Vec2::new(22.0, 22.0),
                        ),
                        ico,
                        Color32::from_rgb(35, 35, 32),
                    );
                    tp.text(
                        Pos2::new(tr.left() + 28.0, tr.center().y),
                        egui::Align2::LEFT_CENTER,
                        lbl,
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(30, 30, 28),
                    );
                    let close_rect =
                        Rect::from_min_max(Pos2::new(tr.right() - 18.0, tr.top()), tr.max);
                    tp.text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::proportional(13.5),
                        Color32::from_rgb(90, 89, 84),
                    );
                    if sel {
                        tp.line_segment(
                            [
                                Pos2::new(tr.left(), tr.bottom()),
                                Pos2::new(tr.right(), tr.bottom()),
                            ],
                            Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                        );
                    }
                    if resp.clicked() {
                        if resp.interact_pointer_pos().is_some_and(|p| close_rect.contains(p)) {
                            self.close_studio_tab(extra);
                        } else {
                            self.active_tab = extra;
                        }
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if self.theme_menu_open {
                    let popup_pos = Pos2::new(
                        (tabbar_rect.right() - 220.0).max(16.0),
                        tabbar_rect.bottom() + 4.0,
                    );
                    egui::Area::new(ui.id().with("ka_theme_menu_popup"))
                        .fixed_pos(popup_pos)
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style())
                                .fill(Color32::from_rgb(252, 251, 247))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_min_width(198.0);
                                    for ct in strata_types::theme_tokens::ColorTheme::ALL {
                                        let changed = ui
                                            .radio_value(
                                                &mut self.display_options.color_theme,
                                                ct,
                                                ct.label_ko(),
                                            )
                                            .on_hover_text(ct.tooltip_ko())
                                            .changed();
                                        if changed {
                                            self.theme =
                                                strata_types::theme_tokens::ThemeTokens::for_options(
                                                    &self.display_options,
                                                );
                                            self.theme_dirty = true;
                                        }
                                    }
                                    ui.separator();
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.paper_look,
                                            "새 모양 (종이·흙색)",
                                        )
                                        .changed()
                                    {
                                        self.paper_look = self.display_options.paper_look;
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                        self.theme_dirty = true;
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.high_contrast,
                                            "고대비 화면 (햇빛 아래)",
                                        )
                                        .changed()
                                    {
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                        self.theme_dirty = true;
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.large_text,
                                            "큰 글씨",
                                        )
                                        .changed()
                                    {
                                        self.large_font_mode = self.display_options.large_text;
                                        ctx.set_zoom_factor(if self.large_font_mode {
                                            1.15
                                        } else {
                                            1.0
                                        });
                                    }
                                });
                        });
                }
                if is_home {
                    let theme_btn = egui::Button::new(
                        egui::RichText::new("테마 ▼")
                            .size(12.0)
                            .color(Color32::from_rgb(45, 44, 40)),
                    )
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(195, 192, 182)))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(56.0, 24.0));
                    if ui
                        .add(theme_btn)
                        .on_hover_text("화면 테마·모양·글자 크기 설정")
                        .clicked()
                    {
                        self.theme_menu_open = !self.theme_menu_open;
                    }
                } else {
                    if self.is_dirty {
                        ui.label(
                            egui::RichText::new("● 저장 안 됨")
                                .size(12.0)
                                .color(Color32::from_rgb(181, 87, 58)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("저장됨")
                                .size(12.0)
                                .color(Color32::from_rgb(94, 93, 89)),
                        );
                    }
                    ui.label(
                        egui::RichText::new(&self.project_title)
                            .size(12.0)
                            .color(Color32::from_rgb(94, 93, 89)),
                    );
                }
            });
        });

        self.render_region_locator_popup(ctx);
    }

    fn draw_ribbon_chip(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        muted: bool,
        highlight_save: bool,
        dark_active: bool,
        warm_active: bool,
    ) -> (bool, bool) {
        let target_rect = ui.max_rect();
        let (_, resp) = ui.allocate_exact_size(target_rect.size(), egui::Sense::click());
        let rect = target_rect;
        let p = ui.painter_at(rect);
        // KaBeginnerRibbon.cpp:229-250 — 창 너비에 맞춰 tile 크기를 32..56 범위에서 결정.
        let tile_side = (rect.width() - 6.0)
            .min(rect.height() - 28.0)
            .clamp(26.0, 56.0);
        let icon_box = Rect::from_min_size(
            Pos2::new(rect.center().x - tile_side * 0.5, rect.top() + 3.0),
            Vec2::new(tile_side, tile_side),
        );
        let corner = tile_side * 0.25;
        let is_hovered = resp.hovered()
            && ui
                .input(|i| i.pointer.hover_pos())
                .is_none_or(|pos| rect.contains(pos));
        let paper = self.display_options.paper_look;
        if highlight_save {
            let strong_bg = if paper {
                Color32::from_rgb(0xB5, 0x57, 0x3A)
            } else {
                Color32::from_rgb(0x20, 0x6C, 0xB0)
            };
            p.rect_filled(icon_box, corner, strong_bg);
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                4.2,
                Color32::WHITE,
            );
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                2.8,
                Color32::from_rgb(0xF2, 0xA3, 0x3A),
            );
        } else if dark_active {
            let (on_bg, on_fg) = if paper {
                (
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                    Color32::from_rgb(0xFA, 0xF9, 0xF5),
                )
            } else {
                (
                    Color32::from_rgb(0xE0, 0xEC, 0xF8),
                    Color32::from_rgb(0x10, 0x50, 0x88),
                )
            };
            p.rect_filled(icon_box, corner, on_bg);
            if !paper {
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(1.5, Color32::from_rgb(0x1A, 0x68, 0xB0)),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, on_fg);
        } else {
            // KaThemePaper.cpp:60-70 vs KaTheme.cpp:105-113
            let (bg, stroke_col) = if muted {
                if paper {
                    (
                        Color32::from_rgb(0xF5, 0xF4, 0xED),
                        Color32::from_rgb(0x9E, 0x9D, 0x98),
                    )
                } else {
                    (
                        Color32::from_rgb(0xEE, 0xF1, 0xF3),
                        Color32::from_rgb(0x8F, 0x98, 0xA3),
                    )
                }
            } else if warm_active {
                if paper {
                    (
                        Color32::from_rgb(0xE3, 0xDA, 0xCC),
                        Color32::from_rgb(0x14, 0x14, 0x13),
                    )
                } else {
                    (
                        Color32::from_rgb(0xE0, 0xEC, 0xF8),
                        Color32::from_rgb(0x10, 0x50, 0x88),
                    )
                }
            } else if is_hovered {
                if paper {
                    (
                        Color32::from_rgb(0xE8, 0xE6, 0xDC),
                        Color32::from_rgb(0x30, 0x30, 0x2E),
                    )
                } else {
                    (
                        Color32::from_rgb(0xD9, 0xE2, 0xE7),
                        Color32::from_rgb(0x2B, 0x48, 0x58),
                    )
                }
            } else if paper {
                (
                    Color32::from_rgb(0xF0, 0xEE, 0xE6),
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                )
            } else {
                (
                    Color32::from_rgb(0xE4, 0xEA, 0xED),
                    Color32::from_rgb(0x2B, 0x48, 0x58),
                )
            };
            p.rect_filled(icon_box, corner, bg);
            if warm_active {
                let bw = (tile_side * 2.0 / 32.0).max(1.0);
                let border_col = if paper {
                    Color32::from_rgb(0xB0, 0xAE, 0xA5)
                } else {
                    Color32::from_rgb(0x1A, 0x68, 0xB0)
                };
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(bw, border_col),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, stroke_col);
        }
        let txt_col = if muted {
            Color32::from_rgb(0x9E, 0x9D, 0x98)
        } else {
            Color32::from_rgb(0x14, 0x14, 0x13)
        };
        p.text(
            Pos2::new(rect.center().x, rect.top() + tile_side + 15.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            txt_col,
        );
        let resp = resp.on_hover_text(cpp_ribbon_tooltip(label));
        let clicked = resp.clicked()
            && resp
                .interact_pointer_pos()
                .is_none_or(|pos| rect.contains(pos));
        let secondary = resp.secondary_clicked();
        (clicked, secondary)
    }

    /// KaPaleoCrossSectionWidget.cpp:99-158 `KaPaleoMenu::install` +
    /// MainWindowRibbon.cpp:390-711 —
    /// 리본 `고지형`/`지질`/`DEM`/`토양`/`지적`/`수치지형`/`옛 지도`/`더보기` 팝업 메뉴.
    pub(crate) fn open_ribbon_context_menu(&mut self, label: &str, anchor: egui::Rect) {
        self.ribbon.paleo_menu_open = false;
        self.ribbon.geology_menu_open = false;
        self.ribbon.dem_menu_open = false;
        self.ribbon.soil_menu_open = false;
        self.ribbon.cadastral_menu_open = false;
        self.ribbon.topo_menu_open = false;
        self.ribbon.old_map_menu_open = false;
        self.ribbon.more_menu_open = false;
        self.ribbon.context_menu_anchor = Pos2::new(anchor.left(), anchor.bottom() + 2.0);
        match label {
            "고지형" => self.ribbon.paleo_menu_open = true,
            "지질" => self.ribbon.geology_menu_open = true,
            "DEM" => self.ribbon.dem_menu_open = true,
            "토양" => self.ribbon.soil_menu_open = true,
            "지적" => self.ribbon.cadastral_menu_open = true,
            "수치지형" => self.ribbon.topo_menu_open = true,
            "옛 지도" => self.ribbon.old_map_menu_open = true,
            "더보기" => {
                self.ribbon.context_menu_anchor =
                    Pos2::new((anchor.right() - 250.0).max(16.0), anchor.bottom() + 2.0);
                self.ribbon.more_menu_open = true;
            }
            _ => {}
        }
    }

    /// 우클릭 팝업 메뉴를 그린다. 항목을 고르면 닫고 실제 동작을 실행한다.
    pub(crate) fn render_ribbon_context_menus(&mut self, ctx: &egui::Context) {
        let anchor = self.ribbon.context_menu_anchor;
        // ── 고지형 (7항목, KaPaleoMenu::install)
        if self.ribbon.paleo_menu_open {
            let mut open = true;
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_paleo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 7] = [
                        "「고지형분석」 전용 탭 열기",
                        "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 창…",
                        "수치지질도(KIGAM 1:5만) 별도 받기 (반경 5km · 도면 범례)",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km · 도면 범례)",
                        "KIGAM 단층·지하수위·단구연대 별도 받기 (반경 10km)",
                        "국토지반정보 포털 열기 (geoinfo.or.kr)…",
                        "KIGAM 지오빅데이터 포털 열기 (data.kigam.re.kr)…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 2 || i == 5 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [420.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.paleo_menu_open = false;
                self.run_paleo_menu_action(item);
                open = false;
            }
            let _ = open;
        }
        // ── 지질 (6항목, MainWindowRibbon.cpp geoMenu)
        if self.ribbon.geology_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_geology_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 6] = [
                        "조사구역 반경 5km 수치지질도 받기 (1:5만 · 권장)",
                        "조사구역 반경 2km 수치지질도 받기",
                        "조사구역 반경 10km 수치지질도 받기",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km)",
                        "KIGAM 단층·선구조·수문 별도 받기 (반경 10km)",
                        "수치지질도 · 시추주상도 별도 내려받기 창…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 3 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [400.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.geology_menu_open = false;
                self.run_geology_menu_action(item);
            }
        }
        // ── DEM (5항목, MainWindowChrome.cpp:215-223)
        if self.ribbon.dem_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_dem_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 5] = [
                        "국토지리정보원 계정 설정·자동 다운로드…",
                        "국토지리원 DEM 불러오기(.img)…",
                        "DEM 파일로 음영 만들기…",
                        "DEM 표현…",
                        "측량점으로 등고선…",
                    ];
                    for item in ITEMS {
                        if ui
                            .add_sized(
                                [330.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.dem_menu_open = false;
                self.run_dem_menu_action(item);
            }
        }
        // ── 토양 (2항목, MainWindowRibbon.cpp:447-452)
        if self.ribbon.soil_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_soil_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in [
                        "분포지형 내려받기 — 현재 화면 범위",
                        "토양도 파일 불러오기(SHP·GPKG)",
                    ] {
                        if ui
                            .add_sized(
                                [250.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.soil_menu_open = false;
                self.handle_ribbon_click("토양");
            }
        }
        // ── 지적 (2항목, MainWindowRibbon.cpp:477-478)
        if self.ribbon.cadastral_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_cadastral_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["VWorld 계정 설정", "선 색·지번 표시"] {
                        if ui
                            .add_sized(
                                [190.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.cadastral_menu_open = false;
                if item.contains("계정") {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                } else {
                    self.modals.active = Some(ActiveModalDialog::CadastralDownload);
                }
            }
        }
        // ── 수치지형 (2항목, MainWindowRibbon.cpp:391-392)
        if self.ribbon.topo_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_topo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["국토지리정보원에서 내려받기…", "받아 둔 수치지형도 폴더…"]
                    {
                        if ui
                            .add_sized(
                                [230.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.topo_menu_open = false;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
        }
        // ── 옛 지도 (2항목, MainWindowRibbon.cpp:492-500 & 12-oldmap.png / 14-drawing.png)
        if self.ribbon.old_map_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_old_map_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["▤  대동여지도", "▤  1919 조선지형도 1:5만"] {
                        if ui
                            .add_sized(
                                [196.0, 26.0],
                                egui::Button::new(egui::RichText::new(item).size(12.5))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.old_map_menu_open = false;
                self.active_tab = MainTab::Map;
                let map_type = if item.contains("대동여지도") {
                    strata_net::tile_fetcher::HistoricalMapType::Daedongyeojido
                } else {
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000
                };
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(map_type, 0.8);
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
        }
        // ── 더보기 (MainWindowRibbon.cpp:628-710 — 불러오기·배경 / 화면 / 계정과 키 / 상태와 정보)
        if self.ribbon.more_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_more_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    ui.set_min_width(236.0);
                    ui.label(
                        egui::RichText::new("불러오기·배경")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "벡터·도면 불러오기",
                        "참조 벡터를 조사 파일 밖으로…",
                        "조사 폴더 임시 자료 정리…",
                        "토양도 SHP 불러오기",
                        "CSV 기준점",
                        "맞추기",
                        "OSM 배경",
                        "Google 위성",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("화면")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "파일함 보이기/숨기기",
                        "지도 넓게 보기 (F11)",
                        "왼쪽 패널 접기/펴기 (F9)",
                        "화면 보기 (고대비·큰 글씨·테마)…",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("계정과 키 · 상태와 정보")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in ["API 키 입력 · 계정 설정…", "자료 준비 상태", "정보"]
                    {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.more_menu_open = false;
                if item.contains("화면 보기") {
                    self.theme_menu_open = !self.theme_menu_open;
                } else if item.contains("맞추기") {
                    self.handle_ribbon_click("정합");
                } else if item.contains("OSM 배경") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldBase;
                } else if item.contains("Google 위성") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldSatellite;
                } else if item.contains("파일함") {
                    self.home_map.files_open = !self.home_map.files_open;
                } else if item.contains("벡터·도면") {
                    self.modals.active = Some(ActiveModalDialog::CadImportNotice);
                } else {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
            }
        }
        // 바깥을 누르면 닫는다
        if ctx.input(|i| i.pointer.any_click()) {
            let any_open = self.ribbon.paleo_menu_open
                || self.ribbon.geology_menu_open
                || self.ribbon.dem_menu_open
                || self.ribbon.soil_menu_open
                || self.ribbon.cadastral_menu_open
                || self.ribbon.topo_menu_open
                || self.ribbon.old_map_menu_open
                || self.ribbon.more_menu_open;
            if any_open {
                let pos = ctx.input(|i| i.pointer.interact_pos());
                if let Some(p) = pos {
                    let near = anchor.x - 16.0 <= p.x
                        && p.x <= anchor.x + 450.0
                        && anchor.y - 36.0 <= p.y
                        && p.y <= anchor.y + 420.0;
                    if !near {
                        self.ribbon.paleo_menu_open = false;
                        self.ribbon.geology_menu_open = false;
                        self.ribbon.dem_menu_open = false;
                        self.ribbon.soil_menu_open = false;
                        self.ribbon.cadastral_menu_open = false;
                        self.ribbon.topo_menu_open = false;
                        self.ribbon.old_map_menu_open = false;
                        self.ribbon.more_menu_open = false;
                    }
                }
            }
        }
    }

    fn run_paleo_menu_action(&mut self, item: &str) {
        if item.contains("전용 탭 열기") {
            self.open_studio_tab(MainTab::Paleo);
        } else if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 5km · 도면 범례") {
            self.paleo_3d.source_dl_radius_km = 5.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        } else if item.contains("포털 열기") {
            self.home_map.draw_guide_message =
                Some(format!("{item} — 브라우저에서 이어서 받으세요."));
        }
    }

    fn run_geology_menu_action(&mut self, item: &str) {
        if item.contains("별도 내려받기 창") {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
            return;
        }
        if item.contains("반경 2km") {
            self.paleo_3d.source_dl_radius_km = 2.0;
        } else if item.contains("반경 10km") {
            self.paleo_3d.source_dl_radius_km = 10.0;
        } else {
            self.paleo_3d.source_dl_radius_km = 5.0;
        }
        if item.contains("수치지질도") {
            self.active_tab = MainTab::Map;
            let spec = strata_net::tile_fetcher::build_wms_thematic_layer_spec(
                strata_net::tile_fetcher::ThematicLayerType::Geology1to50k,
                self.work_crs.auth_id(),
            );
            self.home_map.draw_guide_message = Some(format!(
                "{} — 반경 {:.0} km 기준으로 켰습니다.",
                spec.layer_title_ko, self.paleo_3d.source_dl_radius_km
            ));
        } else {
            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
        }
    }

    fn run_dem_menu_action(&mut self, item: &str) {
        if item.contains("계정 설정") {
            self.modals.active = Some(ActiveModalDialog::AccountSettings);
        } else if item.contains("DEM 불러오기") {
            self.modals.active = Some(ActiveModalDialog::DemDownload);
        } else if item.contains("음영 만들기") || item.contains("DEM 표현") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::DemClass);
        } else if item.contains("등고선") {
            self.active_tab = MainTab::Map;
            self.modals.active = Some(ActiveModalDialog::SurveyContour);
        }
    }
    /// C++ `MainWindow::applyAlignMove()` — `MainWindowAlign.cpp:627-703`.
    /// 점이 2개 미만이면 C++ `KaAlignMapTool::applyMove` 문구(`KaAlignMapTool.cpp:346-348`)로 거절한다.
    fn apply_align_move(&mut self) -> Result<(), String> {
        if self.align.pairs.len() < 2 {
            return Err("점을 2개 이상 찍은 뒤 「이동」을 누르세요".to_owned());
        }
        let src: Vec<[f64; 2]> = self.align.pairs.iter().map(|p| p.src).collect();
        let dst: Vec<[f64; 2]> = self.align.pairs.iter().map(|p| p.map).collect();
        let transform = strata_geo::polygon_ops::solve_helmert_georef(&src, &dst)
            .map_err(|e| e.to_string())?;
        self.align.transform = Some(transform);
        self.align.applied = true;
        self.is_dirty = true;
        // :695-701 — 좌표 파일을 새로 썼다는 사실과 백업 위치를 함께 알린다.
        // Rust 는 아직 세계파일을 쓰지 않으므로(:694 `backup` 이 빈 경우의 C++ 분기와 같다)
        // 뒷문장 없이 앞문장만 남긴다.
        Ok(())
    }

    /// C++ `KaAlignMapTool::restoreOriginals()` — `KaAlignMapToolEdit.cpp:124-168`.
    /// 돌려주는 값은 (성공 여부, C++ 원문 메시지).
    fn restore_align_originals(&mut self) -> (bool, String) {
        // C++ 은 세션 레이어가 없으면 `되돌릴 맞추기가 없습니다`(:132-133)
        if self.align.target_layer.is_none() {
            return (true, "되돌릴 맞추기가 없습니다".to_owned());
        }
        let applied = self.align.applied;
        self.align.pairs.clear();
        self.align.pending_source = None;
        self.align.transform = None;
        self.align.applied = false;
        if !applied {
            // :147-148 — 아직 세계파일에 쓴 것이 없으면 점만 지운다
            return (
                true,
                "찍은 점을 지웠습니다. 그림 파일은 아직 바뀌지 않았습니다".to_owned(),
            );
        }
        self.is_dirty = true;
        (true, "도면을 맞추기 전 자리로 되돌렸습니다".to_owned())
    }

    /// C++ `MainWindowAlign.cpp:745-792` 「맞추기 저장」.
    /// 돌려주는 값은 (알림 문구, 상태줄 문구).
    fn save_align_result(&mut self) -> (String, String) {
        // :748-752 — 점이 2개 미만이면 저장하지 않고 안내만 한다
        if self.align.pairs.len() < 2 && !self.align.applied {
            return (
                "맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요.".to_owned(),
                "점을 2곳 이상 찍은 뒤 저장하세요.".to_owned(),
            );
        }
        if !self.align.applied && self.apply_align_move().is_err() {
            return (
                "맞추기 — 도면을 옮기지 못했습니다".to_owned(),
                self.align.status_text(),
            );
        }
        // :763 — 저장하며 도면 세션은 끝난다
        self.stop_align_session();
        self.is_dirty = true;
        let path = self.align.saved_path.clone();
        (
            "맞춰 두었습니다. 이 도면은 참고용이니 제출할 구역은 그리기로 직접 그리세요.".to_owned(),
            match path {
                // :791 `맞춤 저장: %1`
                Some(p) => format!("맞춤 저장: {p}"),
                None => self.align.status_text(),
            },
        )
    }

    /// C++ `MainWindow::stopAlignSession()` — `MainWindowAlign.cpp:705-714`.
    /// 도구를 놓고 이동으로 돌아간 뒤 세션을 끝낸다.
    fn stop_align_session(&mut self) {
        self.align.end_session();
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
        if self.active_tool == MapToolKind::GeorefAlign {
            self.active_tool = MapToolKind::Pan;
        }
    }

    /// C++ `GeorefService::isAlignableLayer` — `GeorefService.cpp:502-512`.
    /// 도메인 조사 레이어(`:495-500`)는 맞출 수 없고, GDAL 래스터와 OGR·메모리 벡터만 된다.
    /// 레이어 목록에서 그런 첫 레이어의 표시 이름을 고른다(C++ 은 현재 레이어를 쓴다).
    fn alignable_layer_name(&self) -> Option<String> {
        let current = self.right_inspector.selected_layer_key.as_deref();
        let is_alignable = |key: &str| {
            !matches!(
                key,
                "survey_area"
                    | "feature_poly"
                    | "feature_line"
                    | "section_line"
                    | "control_points"
                    | "artifact_point"
                    | "trial_trench"
            ) && !key.starts_with("vworld_")
        };
        if let Some(key) = current.filter(|k| is_alignable(k)) {
            return self
                .left_layers
                .entries
                .iter()
                .find(|e| e.layer_key == key)
                .map(|e| e.label_ko.clone())
                .or_else(|| Some(key.to_owned()));
        }
        self.left_layers
            .entries
            .iter()
            .find(|e| is_alignable(&e.layer_key))
            .map(|e| e.label_ko.clone())
    }

    /// C++ `MainWindow::startAlignSession(QgsMapLayer*)` — `MainWindowAlign.cpp:799-838`.
    fn start_align_session(&mut self, target_layer: &str) {
        self.align = AlignSessionState {
            target_layer: Some(target_layer.to_owned()),
            ..AlignSessionState::default()
        };
        // :831-832 showAlignSplit + showSubToolsAlign
        self.ribbon.toggle_sub_mode(SubToolsMode::Align);
        self.active_tool = MapToolKind::GeorefAlign;
        // :838 — 세션을 열면 상태줄에 statusText() 를 띄운다
        self.status_bar_hint = self.align.status_text();
    }

    /// C++ `MainWindow::hideSubTools()` — `MainWindowRibbon.cpp:769-777`.
    /// 정합 모드에서 도구 줄을 닫으면 정합 세션도 함께 끝난다.
    fn hide_sub_tools(&mut self) {
        if self.ribbon.sub_mode == SubToolsMode::Align {
            self.stop_align_session();
            return;
        }
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = SubToolsMode::None;
    }

    /// C++ `MainWindowOverlay.cpp:225-240` `toggleExistingOverlay` 그대로.
    /// 레이어가 이미 있으면 켜고 끄기만 하며 `…를 껐습니다.` / `…를 다시 켰습니다.` 를
    /// 남기고 `true`. 없으면 `false` 라 호출한 쪽이 처음 받아 오는 길로 간다.
    /// (`layer_visible` 에 키가 있는지가 C++ 의 `layersMatchingBaseName` 비어 있지 않음에 해당)
    fn toggle_existing_overlay(&mut self, title: &str) -> bool {
        if !self
            .home_map
            .real_survey
            .layer_visible
            .contains_key(title)
        {
            return false;
        }
        let now_on = !self.home_map.real_survey.is_layer_visible(title);
        self.home_map
            .real_survey
            .layer_visible
            .insert(title.to_owned(), now_on);
        self.status_bar_hint = if now_on {
            format!("{title}를 다시 켰습니다.")
        } else {
            format!("{title}를 껐습니다.")
        };
        true
    }

    fn handle_ribbon_click(&mut self, label: &str) {
        match label {
            "새 조사" => self.modals.active = Some(ActiveModalDialog::NewSurvey),
            // C++ MainWindowSession.cpp:1634 openProject — 파일 대화상자로 조사 파일을 고른다
            "열기" => self.open_survey_file_dialog(),
            // C++ MainWindowSession.cpp:1409 saveProject / :1420 saveProjectAs
            "저장" => {
                let _ = self.save_survey_project();
            }
            "다른 이름" => {
                let _ = self.save_survey_project_as();
            }
            // C++ MainWindowEditing.cpp:182 startSelectTool — 같은 도구를 다시 누르면
            // 이동(Pan)으로 돌아가고 상태줄에 「도형 선택 종료」를 띄운다.
            "선택" => {
                self.active_tab = MainTab::Map;
                if self.active_tool == MapToolKind::Select {
                    self.active_tool = MapToolKind::Pan;
                    self.status_bar_hint = "도형 선택 종료".to_owned();
                } else {
                    self.active_tool = MapToolKind::Select;
                    self.status_bar_hint = "도형선택 — 도형을 클릭하면 점이 나옵니다. 점을 끌면 옮기고(Ctrl=자석), 점을 찍은 뒤 Delete로 지웁니다. 같은 자리를 다시 누르면 겹친 도형을 고릅니다.".to_owned();
                }
            }
            // C++ MainWindowEditing.cpp:242 startMeasureTool — 토글 + 두 상태줄 문구
            "측거" => {
                if self.active_tool == MapToolKind::Measure {
                    self.active_tool = MapToolKind::Pan;
                    self.status_bar_hint = "줄자 종료".to_owned();
                } else {
                    self.active_tool = MapToolKind::Measure;
                    self.active_tab = MainTab::Map;
                    self.status_bar_hint =
                        "줄자: 점을 찍고 우클릭에서 마침을 고르세요. 면적은 면적만 나옵니다."
                            .to_owned();
                }
            }
            // C++ MainWindowRibbon.cpp:345 — 다시 누르면 도구를 닫고 이동으로 돌아간다
            "그리기" => {
                self.active_tab = MainTab::Map;
                let was_open =
                    self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw;
                self.ribbon.toggle_sub_mode(SubToolsMode::Draw);
                if was_open {
                    self.active_tool = MapToolKind::Pan;
                    self.status_bar_hint = "그리기 도구 모음을 닫고 지도 이동으로 돌아갑니다"
                        .to_owned();
                } else {
                    self.active_tool = MapToolKind::DrawPolygon;
                }
            }
            // C++ MainWindow.cpp:1834 startTrenchGrid — 조사가 없으면 먼저 안내하고 열지 않는다
            "시굴격자" => {
                if self.survey_path.is_none() {
                    self.home_map.draw_guide_message = Some(
                        "먼저 「새 조사」를 만들거나 「열기」로 조사를 여세요.".to_owned(),
                    );
                } else {
                    self.modals.active = Some(ActiveModalDialog::TrenchGenerator);
                }
            }
            "버퍼" => {
                self.active_tab = MainTab::Map;
                self.ribbon.toggle_sub_mode(SubToolsMode::Buffer);
            }
            // C++ MainWindowOverlay.cpp:303-348 toggleDemMap — DEM 겹침이 이미 있으면
            // 보이기/숨기기만 하고(:315-340), 없을 때만 받으러 간다(:344 startDemDownload).
            // C++ 은 :315 에서 이름 "DEM" 으로 묻지만 이것은 BasemapDsm.h:44-57 이
            // ka_hgis/reference_kind=dem 태그로 찾는 것과 같은 레이어이고, 그 범례 이름은
            // BasemapDsm.h:21 `지표모델(DSM) 30m · Copernicus` 다. Rust 범례는 이 이름을
            // `지표모델(DSM) 30m · Coperni...` 로 줄여 쓰므로 두 키를 같은 값으로 맞춘다.
            "DEM" => {
                self.active_tab = MainTab::Map;
                let present = self
                    .home_map
                    .real_survey
                    .layer_visible
                    .contains_key("DEM")
                    || self
                        .home_map
                        .real_survey
                        .layer_visible
                        .contains_key(DEM_LEGEND_KEY);
                if present {
                    let make_visible = !self.home_map.real_survey.is_layer_visible("DEM");
                    for key in ["DEM", DEM_LEGEND_KEY] {
                        self.home_map
                            .real_survey
                            .layer_visible
                            .insert(key.to_owned(), make_visible);
                    }
                    // MainWindowOverlay.cpp:330-332 — DEM 을 끄는데 지질도가 꺼져 있으면
                    // 같이 올라와 있던 「지형 음영」 도 내린다. C++ `LayerOps::isLayerVisible`
                    // 은 레이어가 없으면 false 이므로(BasemapOps.cpp:2358-2359) Rust 기본값
                    // (없으면 true)을 그대로 쓰지 않고 있는지까지 본다.
                    let geology_on = self
                        .home_map
                        .real_survey
                        .layer_visible
                        .contains_key("지질도(KIGAM 1:5만)")
                        && self
                            .home_map
                            .real_survey
                            .is_layer_visible("지질도(KIGAM 1:5만)");
                    if !make_visible && !geology_on {
                        self.home_map
                            .real_survey
                            .layer_visible
                            .insert("지형 음영".to_owned(), false);
                    }
                    // MainWindowOverlay.cpp:337-340 문구 그대로
                    self.status_bar_hint = if make_visible {
                        "DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요.".to_owned()
                    } else {
                        "DEM 지형을 숨겼습니다.".to_owned()
                    };
                } else {
                    // :344 startDemDownload
                    self.modals.active = Some(ActiveModalDialog::DemDownload);
                }
            }
            "지적" => {
                self.active_tab = MainTab::Map;
                self.left_layers.add_cadastral_wms_layer();
                self.modals.active = Some(ActiveModalDialog::CadastralDownload);
            }
            "수치지형" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
            "유산" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect);
            }
            // C++ `MainWindowRibbon.cpp:532-547` btnBoreholeFetch (`d7afcc8`) —
            // 조사 폴더가 없으면 「먼저 조사 폴더를 준비하세요」 안내만 남기고 창을 열지 않는다
            // (`KaBoreholeDialog.cpp:66-70`).
            "시추" => {
                self.active_tab = MainTab::Map;
                let _ = self.open_borehole_dialog();
            }
            // C++ MainWindowOverlay.cpp:243-271 toggleTerrainMap. 이미 있는 「지형맵」 은
            // :247 toggleExistingOverlay(:225-240)가 껐다 켰다 하고 `지형맵를 껐습니다.` /
            // `지형맵를 다시 켰습니다.` 를 내며, 처음 올릴 때만 :267 의
            // `지형맵을 올렸습니다. 다시 누르면 숨깁니다.` 가 나간다.
            "지형" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("지형맵") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("지형맵".to_owned(), true);
                    self.status_bar_hint =
                        "지형맵을 올렸습니다. 다시 누르면 숨깁니다.".to_owned();
                }
            }
            // C++ MainWindowRibbon.cpp:429-436 — `contour` 단추에는 setCheckable 이 없어
            // 토글이 아니고, 누르면 MainWindowSurveyContour.cpp:104 createSurveyContours 만
            // 돈다. 그 함수는 단면선 같은 도메인 레이어를 만들지 않는다(계산이 끝나면
            // :165 SurveyContourStyle::apply 가 결과 레이어를 올린다).
            "등고선" => {
                self.active_tab = MainTab::Map;
                self.modals.active = Some(ActiveModalDialog::SurveyContour);
            }
            // C++ MainWindowOverlay.cpp:352-401 downloadSoilTerrain — 본체 클릭은
            // :357 toggleExistingOverlay(`토양도(흙토람)`)로 껐다 켰다 하고, 없을 때만
            // 내려받는다(:396 startReferenceDownload → :536,622 `토양도를 추가했습니다. …`).
            "토양" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("토양도(흙토람)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("토양도(흙토람)".to_owned(), true);
                    self.status_bar_hint =
                        "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
            }
            // C++ MainWindowOverlay.cpp:405-472 downloadGeologyMap — :409 toggleExistingOverlay
            // (`지질도(KIGAM 1:5만)`, m_actGeology)로 껐다 켰다 하고, 켤 때는 지형 음영
            // 겹침(GeologyMapService::reliefLayerTitle = `지질 음영`, GeologyMapService.cpp:60)
            // 도 같은 상태로 맞춘다(:422-423). 없을 때만 내려받는다(:467 → :536,622).
            "지질" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("지질도(KIGAM 1:5만)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("지질도(KIGAM 1:5만)".to_owned(), true);
                    self.status_bar_hint =
                        "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
                let now_on = self
                    .home_map
                    .real_survey
                    .is_layer_visible("지질도(KIGAM 1:5만)");
                self.home_map
                    .real_survey
                    .layer_visible
                    .insert("지질 음영".to_owned(), now_on);
            }
            // C++ MainWindowOverlay.cpp:476-527 downloadRiverMap — :479 toggleExistingOverlay
            // (`수계도(하천망)`, m_actRiver)로 껐다 켰다 하고, 없을 때만 내려받는다
            // (:522 → :536,622 `수계도를 추가했습니다. …`).
            "수계" => {
                self.active_tab = MainTab::Map;
                if !self.toggle_existing_overlay("수계도(하천망)") {
                    self.home_map
                        .real_survey
                        .layer_visible
                        .insert("수계도(하천망)".to_owned(), true);
                    self.status_bar_hint =
                        "수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요.".to_owned();
                }
            }
            "옛 지도" => {
                self.active_tab = MainTab::Map;
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000,
                    0.8,
                );
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
            "고지형" => self.open_studio_tab(MainTab::Paleo),
            // C++ MainWindowRibbon.cpp:522-523 — 「정합」 단추는 setCheckable 없이
            // `georefAssistant` 에 연결된다. 다시 눌러도 세션을 끄지 않고 맞출 도면을 고르는
            // 흐름이 다시 돈다. 끄는 길은 서브툴바 「닫기」(:793) → hideSubTools(:769-777)
            // → stopAlignSession(:705) 하나뿐이다.
            "정합" => {
                self.active_tab = MainTab::Map;
                // :846-875 — 맞출 수 있는 도면(도메인 조사 레이어가 아닌 OGR·메모리 벡터와
                // GDAL 래스터, GeorefService.cpp:502-512)이 있는지 본다.
                if let Some(target) = self.alignable_layer_name() {
                    self.start_align_session(&target);
                } else {
                    // :871-874 QMessageBox::information 원문
                    self.home_map.draw_guide_message = Some(
                        "맞추기 — JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요."
                            .to_owned(),
                    );
                }
            }
            // MainWindowRibbon.cpp:552-557 「도면」 → MainWindow::openLayoutDesigner (Ctrl+L).
            // MainWindowExport.cpp:204-213 — the studio tab already exists: go there without
            // asking for the paper again (re-opening would move the user's sheet view).
            // MainWindowExport.cpp:216 — only the first open asks, through KaPaperDialog.
            "도면" => {
                if self.open_studio_tabs.contains(&MainTab::Drawing) {
                    self.open_studio_tab(MainTab::Drawing);
                    // MainWindowExport.cpp:206 `hideSubTools()` — 도면 탭으로 가면 도구 줄을 닫는다.
                    self.hide_sub_tools();
                    self.home_map.draw_guide_message =
                        Some(StrataAppState::DRAWING_STUDIO_WELCOME.to_owned());
                } else {
                    self.modals.active = Some(ActiveModalDialog::PaperSetup);
                }
            }
            "인쇄" => {
                // MainWindowExport.cpp:186-194 — 도면 화면이 앞에 없으면 도면 탭을 열고
                // 인쇄 창을 띄운다. 용지 기본값은 KaPrintDialog.cpp:124 대로 프린터가
                // A3 를 지원할 때만 A3 다.
                self.open_studio_tab(MainTab::Drawing);
                self.modals.print_paper =
                    crate::dialogs_modal::default_print_paper_for_printer();
                self.modals.active = Some(ActiveModalDialog::DrawingPrint);
            }
            // MainWindowRibbon.cpp:566-573 + KaShellUi.cpp:48-50 + MainWindow.cpp:1086 —
            // GeoTIFF 는 「지도」 탭에서만 쓴다. 다른 탭이면 사유만 알리고 끝낸다.
            "GeoTIFF" => {
                if self.active_tab != MainTab::Map {
                    self.home_map.draw_guide_message =
                        Some(crate::dialogs_download_print::MAP_TAB_ONLY_REASON.to_owned());
                    return;
                }
                // MainWindowExport.cpp:63-67 — 저장할 레이어가 없으면 안내만 한다.
                if self.left_layers.entries.is_empty() {
                    self.home_map.draw_guide_message =
                        Some("지도에 저장할 레이어가 없습니다.".to_owned());
                    return;
                }
                // MainWindowExport.cpp:68-75 — 저장 대화상자: 기본 이름 지도_EPSG_5187.tif,
                // 필터 "GeoTIFF (*.tif *.tiff)", 기본 확장자 tif, 시작 폴더는 조사 폴더.
                let default_name =
                    crate::dialogs_download_print::geotiff_default_file_name(self.work_crs.auth_id());
                let Some(path) = ::rfd::FileDialog::new()
                    .set_title("현재 지도 GeoTIFF 저장")
                    .add_filter("GeoTIFF", &["tif", "tiff"])
                    .set_file_name(&default_name)
                    .set_directory(&self.home_map.real_survey.survey_dir)
                    .save_file()
                else {
                    return;
                };
                // MapGeoTiffExport.cpp:94-103 — 화면 캔버스 출력 크기 그대로, 상한만 검사한다.
                let size = match crate::dialogs_download_print::geotiff_canvas_size_px(
                    self.home_map.viewport.screen_size_px,
                ) {
                    Ok(sz) => sz,
                    Err(reason) => {
                        self.home_map.draw_guide_message = Some(reason.to_owned());
                        return;
                    }
                };
                // MapGeoTiffExport.cpp:187-190 — 좌표계는 지도 좌표계 그대로 기록한다.
                let auth_id = self.work_crs.auth_id();
                if strata_layout::pdf_geotiff::export_geotiff_with_cancel(
                    &path,
                    size.0,
                    size.1,
                    self.work_crs.epsg_code(),
                    None,
                )
                .is_ok()
                {
                    // MainWindowExport.cpp:90-91 — "GeoTIFF 저장 완료 · EPSG:5187 · 경로"
                    self.home_map.draw_guide_message =
                        Some(format!("GeoTIFF 저장 완료 · {} · {}", auth_id, path.display()));
                } else {
                    self.home_map.draw_guide_message =
                        Some("GeoTIFF를 저장하지 못했습니다".to_owned());
                }
            }
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.modals.active = Some(ActiveModalDialog::AccountSettings),
            _ => self.active_tab = MainTab::Map,
        }
    }

    /// `KaRegionLocator.cpp:38-286` —
    /// 1단계(`region_popup_open`): 「지역」 버튼 바로 밑에 붙는 17개 광역 시·도 격자 메뉴 (`#appBarRegionMenu`)
    /// 2단계(`address_popup_open`): 시·도 칩을 누르면 열리는 주소 팝업 (`#regionAddressPopup`: 시·도 제목 · ✕ · 시·군·구 · 동·읍·면 · 번지 · 찾기 · 취소 · 힌트)
    fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        let screen_r = ctx.screen_rect();
        let [btn_min_x, _btn_min_y, _btn_max_x, btn_max_y] =
            self.ribbon.app_bar.region_button_rect.unwrap_or([
                screen_r.right() - 252.0,
                34.0,
                screen_r.right() - 176.0,
                70.0,
            ]);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.ribbon.app_bar.region_popup_open = false;
            self.ribbon.app_bar.address_popup_open = false;
        }

        // 1단계: 「지역」 드롭다운 메뉴 (17개 광역 시·도 칩 격자만 표시 — media_1794135047748_487cba62.png 1:1)
        if self.ribbon.app_bar.region_popup_open {
            let menu_w = 214.0_f32;
            let menu_x = btn_min_x.clamp(8.0, (screen_r.right() - menu_w - 8.0).max(8.0));
            let menu_pos = Pos2::new(menu_x, btn_max_y + 1.0);
            let mut clicked_sido: Option<String> = None;

            let area_resp = egui::Area::new(egui::Id::new("ka_app_bar_region_menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(menu_pos)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(118, 118, 118)))
                        .corner_radius(0.0)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            egui::Grid::new("ka_peninsula_17_grid")
                                .spacing([6.0, 6.0])
                                .show(ui, |ui| {
                                    for r in 0..6 {
                                        for c in 0..4 {
                                            if let Some(tile) = PENINSULA_REGION_TILES
                                                .iter()
                                                .find(|t| t.row == r && t.col == c)
                                            {
                                                let (chip_rect, chip_resp) = ui
                                                    .allocate_exact_size(
                                                        Vec2::new(42.0, 26.0),
                                                        egui::Sense::click(),
                                                    );
                                                let is_checked =
                                                    self.ribbon.app_bar.address_popup_open
                                                        && self.ribbon.app_bar.selected_sido
                                                            == tile.name;
                                                let bg = if is_checked {
                                                    Color32::from_rgb(239, 236, 226)
                                                } else if chip_resp.hovered() {
                                                    Color32::from_rgb(245, 244, 237)
                                                } else {
                                                    Color32::WHITE
                                                };
                                                let border_col =
                                                    if is_checked || chip_resp.hovered() {
                                                        Color32::from_rgb(181, 87, 58)
                                                    } else {
                                                        Color32::from_rgb(118, 118, 118)
                                                    };
                                                let cp = ui.painter_at(chip_rect);
                                                cp.rect_filled(chip_rect, 6.0, bg);
                                                cp.rect_stroke(
                                                    chip_rect,
                                                    6.0,
                                                    Stroke::new(1.0, border_col),
                                                    egui::StrokeKind::Inside,
                                                );
                                                cp.text(
                                                    chip_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    tile.name,
                                                    egui::FontId::proportional(12.0),
                                                    Color32::from_rgb(140, 54, 22),
                                                );
                                                if chip_resp
                                                    .on_hover_text(
                                                        "시·도를 누르면 시·동·번지로 찾습니다",
                                                    )
                                                    .clicked()
                                                {
                                                    clicked_sido = Some(tile.name.to_owned());
                                                }
                                            } else {
                                                ui.allocate_space(Vec2::new(42.0, 26.0));
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });

            if let Some(sido_short) = clicked_sido {
                self.ribbon.app_bar.selected_sido = sido_short.clone();
                self.ribbon.app_bar.city_input.clear();
                self.ribbon.app_bar.dong_input.clear();
                self.ribbon.app_bar.lot_input.clear();
                self.ribbon.app_bar.region_popup_open = false;
                self.ribbon.app_bar.address_popup_open = true;
                self.zoom_to_sido_overview(&sido_short);
            } else if ctx.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    let btn_rect =
                        self.ribbon.app_bar.region_button_rect.map(|b| {
                            Rect::from_min_max(Pos2::new(b[0], b[1]), Pos2::new(b[2], b[3]))
                        });
                    let in_btn = btn_rect.is_some_and(|r| r.contains(pos));
                    if !area_resp.response.rect.contains(pos) && !in_btn {
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                }
            }
        }

        // 2단계: 시·도 칩을 누른 뒤 열리는 주소 팝업 (`#regionAddressPopup`, KaRegionLocator.cpp:176-286)
        if self.ribbon.app_bar.address_popup_open {
            let popup_w = 570.0_f32;
            let popup_x = btn_min_x.clamp(8.0, (screen_r.right() - popup_w - 8.0).max(8.0));
            let popup_pos = Pos2::new(popup_x, btn_max_y + 2.0);
            let mut run_search = false;
            let mut close_popup = false;
            let canon_sido = strata_net::address_search::KoreaRegionCatalog::canonical_sido(
                &self.ribbon.app_bar.selected_sido,
            );
            let cities = strata_net::address_search::KoreaRegionCatalog::cities_of(&canon_sido);
            let dongs = strata_net::address_search::KoreaRegionCatalog::dongs_of(
                &canon_sido,
                &self.ribbon.app_bar.city_input,
            );

            egui::Area::new(egui::Id::new("ka_region_address_popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(popup_w);
                            // Row 1: m_sidoLabel (#regionSidoTitle) + closeX (✕)
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&canon_sido)
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::from_rgb(140, 54, 22)),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("✕")
                                            .on_hover_text("닫기 (Esc)")
                                            .clicked()
                                        {
                                            close_popup = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(6.0);
                            // Row 2: 시·군·구 ComboBox | 동·읍·면(법정동·읍·면·리) | 산 12-3 | 찾기 | 취소
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let city_label = if self.ribbon.app_bar.city_input.trim().is_empty()
                                {
                                    "시·군·구".to_owned()
                                } else {
                                    self.ribbon.app_bar.city_input.clone()
                                };
                                egui::ComboBox::from_id_salt("ka_region_city_combo")
                                    .width(120.0)
                                    .selected_text(city_label)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                self.ribbon.app_bar.city_input.is_empty(),
                                                "시·군·구",
                                            )
                                            .clicked()
                                        {
                                            self.ribbon.app_bar.city_input.clear();
                                            self.ribbon.app_bar.dong_input.clear();
                                        }
                                        for c in &cities {
                                            let sel = self.ribbon.app_bar.city_input == *c;
                                            if ui.selectable_label(sel, c).clicked() {
                                                self.ribbon.app_bar.city_input = c.clone();
                                                self.ribbon.app_bar.dong_input.clear();
                                            }
                                        }
                                    });

                                let dong_resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input)
                                        .hint_text("법정동·읍·면·리")
                                        .desired_width(112.0),
                                );
                                if dong_resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    run_search = true;
                                }
                                if !dongs.is_empty() {
                                    egui::ComboBox::from_id_salt("ka_region_dong_combo")
                                        .width(24.0)
                                        .selected_text("")
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.ribbon.app_bar.dong_input.is_empty(),
                                                    "동·읍·면",
                                                )
                                                .clicked()
                                            {
                                                self.ribbon.app_bar.dong_input.clear();
                                            }
                                            for d in &dongs {
                                                let sel = self.ribbon.app_bar.dong_input == *d;
                                                if ui.selectable_label(sel, d).clicked() {
                                                    self.ribbon.app_bar.dong_input = d.clone();
                                                }
                                            }
                                        });
                                }

                                let lot_resp = ui
                                    .add(
                                        egui::TextEdit::singleline(
                                            &mut self.ribbon.app_bar.lot_input,
                                        )
                                        .hint_text(REGION_LOT_PLACEHOLDER)
                                        .desired_width(96.0),
                                    )
                                    .on_hover_text(REGION_LOT_HINT);
                                if lot_resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    run_search = true;
                                }

                                let find_btn = egui::Button::new(
                                    egui::RichText::new("찾기")
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(12.5),
                                )
                                .fill(Color32::from_rgb(181, 87, 58))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(56.0, 28.0));
                                if ui.add(find_btn).clicked() {
                                    run_search = true;
                                }

                                let cancel_btn = egui::Button::new(
                                    egui::RichText::new("취소")
                                        .color(Color32::from_rgb(35, 35, 32))
                                        .size(12.5),
                                )
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(54.0, 28.0));
                                if ui.add(cancel_btn).clicked() {
                                    close_popup = true;
                                }
                            });
                            ui.add_space(6.0);
                            // Row 3: 힌트 문구
                            ui.label(
                                egui::RichText::new(REGION_BOTTOM_HINT)
                                    .size(11.5)
                                    .color(Color32::from_rgb(94, 93, 89)),
                            );
                        });
                });

            if close_popup {
                self.ribbon.app_bar.address_popup_open = false;
            }
            if run_search {
                self.run_region_popup_search();
            }
        }

        // 복수 검색 결과 선택 모달 (`MainWindow::onLocationResults`, MainWindow.cpp:2808-2831)
        if !self.ribbon.app_bar.search_results_modal.is_empty() {
            let mut close_results = false;
            let mut confirm_idx: Option<usize> = None;
            let hits = self.ribbon.app_bar.search_results_modal.clone();
            egui::Window::new("위치 선택")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "검색 결과 {}건 — 이동할 위치를 선택하세요",
                        hits.len()
                    ));
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            for (idx, h) in hits.iter().enumerate() {
                                let line = if h.detail.is_empty() {
                                    h.title.clone()
                                } else {
                                    format!("{}  —  {}", h.title, h.detail)
                                };
                                let sel = self.ribbon.app_bar.selected_hit_idx == idx;
                                if ui.selectable_label(sel, line).clicked() {
                                    self.ribbon.app_bar.selected_hit_idx = idx;
                                }
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("확인").color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .min_size(Vec2::new(72.0, 28.0));
                        if ui.add(ok_btn).clicked() {
                            confirm_idx = Some(self.ribbon.app_bar.selected_hit_idx);
                        }
                        if ui
                            .add(egui::Button::new("취소").min_size(Vec2::new(72.0, 28.0)))
                            .clicked()
                        {
                            close_results = true;
                        }
                    });
                });
            if let Some(idx) = confirm_idx {
                if let Some(hit) = hits.get(idx) {
                    self.zoom_to_location_hit(hit);
                }
                self.ribbon.app_bar.search_results_modal.clear();
            } else if close_results {
                self.ribbon.app_bar.search_results_modal.clear();
            }
        }

        // 검색 실패 안내 모달 (`MainWindow::onLocationFailed`, MainWindow.cpp:2801-2806)
        if let Some(err_msg) = self.ribbon.app_bar.search_error_modal.clone() {
            let mut close_err = false;
            egui::Window::new("위치 검색")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(380.0)
                .show(ctx, |ui| {
                    ui.label(&err_msg);
                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("확인").color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .min_size(Vec2::new(72.0, 28.0));
                        if ui.add(ok_btn).clicked() {
                            close_err = true;
                        }
                    });
                });
            if close_err {
                self.ribbon.app_bar.search_error_modal = None;
            }
        }
    }

    /// `MainWindowRibbon.cpp:600-620` — 시·도 칩을 누르면 해당 시·도 개관 범위(overviewBounds)로 지도 이동
    fn zoom_to_sido_overview(&mut self, sido: &str) {
        if let Some(bounds) = strata_net::address_search::KoreaRegionCatalog::overview_bounds(sido)
        {
            let hit = strata_net::address_search::LocationHit {
                title: strata_net::address_search::KoreaRegionCatalog::canonical_sido(sido),
                detail: String::new(),
                lon: (bounds.west + bounds.east) * 0.5,
                lat: (bounds.south + bounds.north) * 0.5,
                bbox: Some([bounds.west, bounds.south, bounds.east, bounds.north]),
                pnu: String::new(),
            };
            self.zoom_to_location_hit(&hit);
        } else if let Some(tile) = PENINSULA_REGION_TILES.iter().find(|t| t.name == sido) {
            self.home_map.viewport.center_xy = tile.center_5187;
            self.active_tab = MainTab::Map;
        }
    }

    /// `MainWindow::zoomToLocation` (`MainWindow.cpp:2833-2890`) 1:1
    fn zoom_to_location_hit(&mut self, hit: &strata_net::address_search::LocationHit) {
        let mut lon = hit.lon;
        let mut lat = hit.lat;
        if lon.abs() <= 90.0 && lat.abs() > 90.0 {
            std::mem::swap(&mut lon, &mut lat);
        }
        let dst_crs = match self.work_crs {
            strata_types::crs::WorkCrs::Epsg5186 => strata_types::crs::CrsCode::Epsg5186,
            strata_types::crs::WorkCrs::Epsg5187 => strata_types::crs::CrsCode::Epsg5187,
        };
        let xf = strata_geo::crs_transform::CrsTransformer::new(
            strata_types::crs::CrsCode::Epsg4326,
            dst_crs,
        );
        let p = xf.transform_xy(lon, lat);
        self.home_map.viewport.center_xy = p;
        if !hit.pnu.is_empty() {
            // 지번 필지 정확 검색: ~1:2,500 축척 (0.65 m/px)
            self.home_map.viewport.meters_per_pixel = 0.65;
        } else if let Some([west, south, east, north]) = hit.bbox {
            let sw = xf.transform_xy(west, south);
            let ne = xf.transform_xy(east, north);
            let span_m = (ne[0] - sw[0]).abs().max((ne[1] - sw[1]).abs()) * 1.2;
            self.home_map.viewport.meters_per_pixel = (span_m / 900.0).clamp(0.5, 250.0);
        } else {
            // 도로명·장소 점 위치: ~1:5,000 축척 (1.25 m/px)
            self.home_map.viewport.meters_per_pixel = 1.25;
        }
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!("위치 이동: {}", hit.title));
    }

    /// `KaAppBar::returnPressed` (`KaAppBar.cpp:54-57`) -> `MainWindow::searchLocation`
    fn run_app_bar_search(&mut self) {
        let q = self.ribbon.app_bar.query.trim().to_owned();
        if q.is_empty() {
            self.home_map.draw_guide_message = Some("주소·지번·지역·상호를 입력하세요".to_owned());
            return;
        }
        let parcel = strata_net::address_search::looks_like_lot(&q);
        self.execute_interactive_location_search(&q, parcel);
    }

    /// `KaRegionLocator::emitSearch` (`KaRegionLocator.cpp:273-286`) -> `MainWindow::searchLocation`
    fn run_region_popup_search(&mut self) {
        let q = strata_net::address_search::KoreaRegionCatalog::compose_address(
            &self.ribbon.app_bar.selected_sido,
            &self.ribbon.app_bar.city_input,
            &self.ribbon.app_bar.dong_input,
            &self.ribbon.app_bar.lot_input,
        );
        if q.is_empty() {
            return;
        }
        let parcel = !self.ribbon.app_bar.lot_input.trim().is_empty();
        self.search_region_locator_address();
        self.ribbon.app_bar.query = q.clone();
        self.execute_interactive_location_search(&q, parcel);
    }

    fn execute_interactive_location_search(&mut self, query: &str, parcel: bool) {
        let key = self.accounts.vworld_key.clone();
        match strata_net::address_search::execute_location_search(query, parcel, &key) {
            Ok(hits) => {
                if hits.len() == 1 {
                    self.zoom_to_location_hit(&hits[0]);
                } else if hits.len() > 1 {
                    self.ribbon.app_bar.selected_hit_idx = 0;
                    self.ribbon.app_bar.search_results_modal = hits;
                } else {
                    self.home_map.draw_guide_message = Some("검색 결과 없음".to_owned());
                    self.ribbon.app_bar.search_error_modal = Some("검색 결과 없음".to_owned());
                }
            }
            Err(msg) => {
                self.home_map.draw_guide_message = Some(msg.clone());
                self.ribbon.app_bar.search_error_modal = Some(msg);
            }
        }
    }

    /// `KaRegionLocator::emitSearch` 오프라인/단위시험 경로 (`gui_shell.rs:820-842`)
    pub fn search_region_locator_address(&mut self) {
        let full_addr = format!(
            "{} {} {} {}",
            self.ribbon.app_bar.selected_sido,
            self.ribbon.app_bar.city_input.trim(),
            self.ribbon.app_bar.dong_input.trim(),
            self.ribbon.app_bar.lot_input.trim()
        );
        self.ribbon.app_bar.query = full_addr.trim().to_owned();
        if let Some(parcel) = self
            .home_map
            .real_survey
            .cadastral_parcels
            .iter()
            .find(|p| {
                !self.ribbon.app_bar.lot_input.trim().is_empty()
                    && p.jibun.contains(self.ribbon.app_bar.lot_input.trim())
            })
        {
            self.home_map.viewport.center_xy = parcel.center;
            self.home_map.viewport.meters_per_pixel = 1.25;
        } else if let Some(tile) = PENINSULA_REGION_TILES
            .iter()
            .find(|t| t.name == self.ribbon.app_bar.selected_sido)
        {
            self.home_map.viewport.center_xy = tile.center_5187;
        }
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!(
            "지역·지번 「{}」 위치로 이동했습니다.",
            self.ribbon.app_bar.query
        ));
        self.ribbon.app_bar.region_popup_open = false;
        self.ribbon.app_bar.address_popup_open = false;
    }
}

#[cfg(test)]
mod tests_align {
    use super::*;
    use crate::app_state::StrataAppState;

    /// R9 — 서브툴바 다섯 항목과 순서가 `MainWindowAlign.cpp:724,726,730,745,793` 과 같은지.
    #[test]
    fn test_r9_align_subtoolbar_has_cpp_five_items_in_cpp_order() {
        assert_eq!(
            ALIGN_SUBTOOLBAR_ITEMS,
            ["이동", "점 지우기", "되돌리기", "맞추기 저장", "닫기"]
        );
        assert_eq!(ALIGN_SUBTOOLBAR_ITEMS.len(), 5);
        assert_eq!(ALIGN_SUBTOOLBAR_CAPTION, "  맞추기 › ");
    }

    /// R9 — `KaAlignMapTool::statusText()` `KaAlignMapTool.cpp:453-462` 네 갈래.
    #[test]
    fn test_r9_align_status_text_matches_kaalignmaptool_status_text() {
        let mut s = AlignSessionState::default();
        assert_eq!(s.status_text(), "맞출 도면을 고르세요");

        s.target_layer = Some("사진".to_owned());
        assert_eq!(s.status_text(), "왼쪽 도면 모서리 → 오른쪽 지적 모서리");

        s.pending_source = Some([0.0, 0.0]);
        assert_eq!(s.status_text(), "오른쪽 지도에서 같은 모서리를 찍으세요");
    }

    /// R9 — `GeorefQuality::summary()` `GeorefQuality.cpp:88-89` 두 점·세 점 안내문.
    #[test]
    fn test_r9_align_pair_summary_matches_georefquality_summary() {
        let mut s = AlignSessionState {
            target_layer: Some("사진".to_owned()),
            ..AlignSessionState::default()
        };
        s.pairs.push(AlignPair {
            src: [0.0, 0.0],
            map: [100.0, 100.0],
        });
        assert_eq!(s.status_text(), "왼쪽 → 오른쪽 · 1점");

        s.pairs.push(AlignPair {
            src: [100.0, 0.0],
            map: [200.0, 100.0],
        });
        assert_eq!(
            s.status_text(),
            "왼쪽 → 오른쪽 · 2점 · 한 점 더 찍으면 기울기도 맞습니다"
        );

        s.pairs.push(AlignPair {
            src: [0.0, 100.0],
            map: [100.0, 200.0],
        });
        assert_eq!(
            s.status_text(),
            "왼쪽 → 오른쪽 · 3점 · 한 점 더 찍으면 어긋남을 잽니다"
        );
    }

    /// R9 — 「이동」을 점 2개 미만에서 누르면 `KaAlignMapTool.cpp:346-348` 문구로 거절.
    #[test]
    fn test_r9_align_move_needs_two_pairs_like_kaalignmaptool() {
        let mut app = StrataAppState::new_startup();
        app.align.target_layer = Some("사진".to_owned());
        assert_eq!(
            app.apply_align_move(),
            Err("점을 2개 이상 찍은 뒤 「이동」을 누르세요".to_owned())
        );
        assert!(!app.align.applied);

        app.align.pairs = vec![
            AlignPair {
                src: [0.0, 0.0],
                map: [100.0, 100.0],
            },
            AlignPair {
                src: [100.0, 0.0],
                map: [200.0, 100.0],
            },
        ];
        assert_eq!(app.apply_align_move(), Ok(()));
        assert!(app.align.applied);
        assert!(app.align.transform.is_some());
    }

    /// R9 — 「점 지우기」는 `KaAlignMapTool.cpp:319-329` 대로 왼쪽 점만 있으면 그것부터 지운다.
    #[test]
    fn test_r9_align_delete_point_removes_pending_then_last_pair() {
        let mut s = AlignSessionState {
            target_layer: Some("사진".to_owned()),
            ..AlignSessionState::default()
        };
        s.pairs.push(AlignPair {
            src: [0.0, 0.0],
            map: [100.0, 100.0],
        });
        s.pending_source = Some([5.0, 5.0]);
        assert!(s.remove_last_pair());
        assert!(s.pending_source.is_none());
        assert_eq!(s.pairs.len(), 1, "왼쪽 점만 찍힌 상태는 쌍을 지우지 않는다");

        assert!(s.remove_last_pair());
        assert!(s.pairs.is_empty());
        assert!(!s.remove_last_pair(), "지울 것이 없으면 false");
    }

    /// R9 — 「맞추기 저장」 점 2개 미만 안내는 `MainWindowAlign.cpp:750` 원문.
    #[test]
    fn test_r9_align_save_refuses_under_two_points_with_cpp_text() {
        let mut app = StrataAppState::new_startup();
        app.align.target_layer = Some("사진".to_owned());
        let (notice, status) = app.save_align_result();
        assert_eq!(
            notice,
            "맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요."
        );
        assert_eq!(status, "점을 2곳 이상 찍은 뒤 저장하세요.");
        assert!(!app.ribbon.sub_toolbar_visible);
    }

    /// R9 — 「닫기」는 `MainWindowAlign.cpp:705-714` 대로 세션을 끝내고 이동으로 돌아간다.
    #[test]
    fn test_r9_align_close_ends_session_and_returns_to_pan() {
        let mut app = StrataAppState::new_startup();
        app.start_align_session("사진");
        assert!(app.ribbon.sub_toolbar_visible);
        assert_eq!(app.ribbon.sub_mode, SubToolsMode::Align);
        assert_eq!(app.active_tool, MapToolKind::GeorefAlign);
        assert_eq!(app.status_bar_hint, "왼쪽 도면 모서리 → 오른쪽 지적 모서리");

        app.stop_align_session();
        assert!(!app.ribbon.sub_toolbar_visible);
        assert_eq!(app.ribbon.sub_mode, SubToolsMode::None);
        assert_eq!(app.active_tool, MapToolKind::Pan);
        assert!(app.align.target_layer.is_none());
    }

    /// R9 — 「정합」은 맞출 도면이 없으면 `MainWindowAlign.cpp:872-874` 원문으로 안내만 한다.
    #[test]
    fn test_r9_align_button_guides_when_no_alignable_layer() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("정합");
        assert!(!app.ribbon.sub_toolbar_visible);
        assert_eq!(
            app.home_map.draw_guide_message.as_deref(),
            Some(
                "맞추기 — JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요."
            )
        );
    }

    /// R9 — 맞출 수 있는 도면이 있으면 세션을 열고 상태줄에 statusText() 를 띄운다
    /// (`MainWindowAlign.cpp:831-838`). 도메인 조사 레이어는 맞출 수 없다(`GeorefService.cpp:504`).
    #[test]
    fn test_r9_align_button_opens_session_for_reference_layer_only() {
        let mut app = StrataAppState::new_startup();
        app.left_layers
            .ensure_domain_layer(strata_types::schema::DomainLayerKey::SurveyArea);
        app.handle_ribbon_click("정합");
        assert!(
            !app.ribbon.sub_toolbar_visible,
            "도메인 조사 레이어는 맞출 수 없으므로 세션이 열리면 안 된다"
        );

        app.left_layers.add_cadastral_wms_layer();
        app.right_inspector.selected_layer_key = Some("vworld_cadastral_wms".to_owned());
        app.handle_ribbon_click("정합");
        assert!(
            !app.ribbon.sub_toolbar_visible,
            "지적 그림(vworld_)도 맞출 대상이 아니다"
        );
    }
}

#[cfg(test)]
mod tests_overlay_toggles {
    use super::*;
    use crate::app_state::StrataAppState;

    /// R8 — C++ `MainWindowOverlay.cpp:225-240` `toggleExistingOverlay`.
    /// 없는 레이어면 `false` 이고 아무 문구도 내지 않는다.
    #[test]
    fn test_r8_toggle_existing_overlay_returns_false_when_absent() {
        let mut app = StrataAppState::new_startup();
        assert!(!app.toggle_existing_overlay("지질도(KIGAM 1:5만)"));
        assert_eq!(app.status_bar_hint, "");
    }

    /// R8 — 「지형」: C++ `MainWindowOverlay.cpp:243-271`.
    /// 처음 누르면 :267 문구(올림), 다음부터는 :231/:236 문구(껐다 켰다).
    #[test]
    fn test_r8_terrain_toggle_uses_cpp_first_and_repeat_texts() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("지형");
        assert!(app.home_map.real_survey.is_layer_visible("지형맵"));
        assert_eq!(app.status_bar_hint, "지형맵을 올렸습니다. 다시 누르면 숨깁니다.");

        app.handle_ribbon_click("지형");
        assert!(!app.home_map.real_survey.is_layer_visible("지형맵"));
        assert_eq!(app.status_bar_hint, "지형맵를 껐습니다.");

        app.handle_ribbon_click("지형");
        assert!(app.home_map.real_survey.is_layer_visible("지형맵"));
        assert_eq!(app.status_bar_hint, "지형맵를 다시 켰습니다.");
    }

    /// R8 — 「토양」·「수계」는 `MainWindowOverlay.cpp:357,479` 가
    /// `토양도(흙토람)` / `수계도(하천망)` 이름으로 토글한다.
    #[test]
    fn test_r8_soil_and_river_toggle_with_cpp_layer_names() {
        for (button, title) in [("토양", "토양도(흙토람)"), ("수계", "수계도(하천망)")] {
            let mut app = StrataAppState::new_startup();
            // 없는 상태: 받아 오는 길(여기서는 안내 문구)로 간다.
            app.handle_ribbon_click(button);
            assert!(app.home_map.real_survey.is_layer_visible(title), "{button}");
            assert!(
                app.status_bar_hint.contains("추가했습니다"),
                "{button}: {}",
                app.status_bar_hint
            );

            // 있는 상태: 끄고 켜며 C++ toggleExistingOverlay 문구를 쓴다.
            app.handle_ribbon_click(button);
            assert!(!app.home_map.real_survey.is_layer_visible(title), "{button}");
            assert_eq!(app.status_bar_hint, format!("{title}를 껐습니다."));

            app.handle_ribbon_click(button);
            assert!(app.home_map.real_survey.is_layer_visible(title), "{button}");
            assert_eq!(app.status_bar_hint, format!("{title}를 다시 켰습니다."));
        }
    }

    /// R8 — 「지질」은 `MainWindowOverlay.cpp:409,422` 처럼 `지질도(KIGAM 1:5만)` 을
    /// 토글하고 `지질 음영` 을 같은 상태로 맞춘다.
    #[test]
    fn test_r8_geology_toggle_also_sets_relief_layer() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("지질");
        assert!(app
            .home_map
            .real_survey
            .is_layer_visible("지질도(KIGAM 1:5만)"));
        app.handle_ribbon_click("지질");
        assert!(!app
            .home_map
            .real_survey
            .is_layer_visible("지질도(KIGAM 1:5만)"));
        assert_eq!(app.status_bar_hint, "지질도(KIGAM 1:5만)를 껐습니다.");
        // :422-423 — 지질도를 끄면 지형 음영 겹침도 내린다.
        assert!(!app.home_map.real_survey.is_layer_visible("지질 음영"));
    }

    /// R8 — 「DEM」은 `MainWindowOverlay.cpp:303-348`. 없으면 받는 창(:344),
    /// 있으면 :337-340 두 문구로 켜고 끈다.
    #[test]
    fn test_r8_dem_opens_download_when_absent_and_toggles_when_present() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("DEM");
        assert_eq!(app.modals.active, Some(ActiveModalDialog::DemDownload));

        app.modals.active = None;
        app.home_map
            .real_survey
            .layer_visible
            .insert("DEM".to_owned(), true);
        app.handle_ribbon_click("DEM");
        assert!(!app.home_map.real_survey.is_layer_visible("DEM"));
        assert_eq!(app.status_bar_hint, "DEM 지형을 숨겼습니다.");
        // :330-332 — 지질도가 꺼져 있으면 지형 음영도 함께 내린다.
        assert!(!app.home_map.real_survey.is_layer_visible("지형 음영"));

        app.handle_ribbon_click("DEM");
        assert!(app.home_map.real_survey.is_layer_visible("DEM"));
        assert_eq!(
            app.status_bar_hint,
            "DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."
        );
    }

    /// R8 — 「등고선」은 토글이 아니다(`MainWindowRibbon.cpp:429-436` 에 setCheckable 없음).
    /// `MainWindowSurveyContour.cpp:104` 도 도메인 레이어를 만들지 않는다 — 빈 단면선
    /// 레이어를 자동으로 넣으면 「빈 도메인 레이어 자동 추가 금지」 규칙에 어긋난다.
    #[test]
    fn test_r8_contour_opens_dialog_without_creating_a_domain_layer() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("등고선");
        assert_eq!(app.modals.active, Some(ActiveModalDialog::SurveyContour));
        assert!(
            app.left_layers.entries.is_empty(),
            "등고선은 빈 도메인 레이어를 만들지 않는다: {:?}",
            app.left_layers.entries.len()
        );
    }
}
