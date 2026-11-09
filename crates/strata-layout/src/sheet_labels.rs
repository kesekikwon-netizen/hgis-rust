//! 라벨 배치 탐색과 도면 번호·격자 간격·표제란 줄 형식.
//!
//! `sheet_composer` 에서 분리했다.

use crate::sheet_geometry::SheetRectMm;

/// 단일 시트 내 최대 배치 뷰포트 수 (C++ `HGIS_MAX_FIGURES_PER_SHEET`)
pub const HGIS_MAX_FIGURES_PER_SHEET: usize = 8;

/// 주 평면도 기본 축척 분모 (1:500, C++ `HGIS_DEFAULT_PLAN_SCALE`)
pub const HGIS_DEFAULT_PLAN_SCALE: f64 = 500.0;

/// 세부 유구도 기본 축척 분모 (1:50, C++ `HGIS_DEFAULT_DETAIL_SCALE`)
pub const HGIS_DEFAULT_DETAIL_SCALE: f64 = 50.0;

/// 다중 뷰포트 프레임 역할 분류 (C++ `ViewportRole`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportRole {
    /// 주 평면도
    MainPlan,
    /// 토층/유구 횡단면도
    CrossSection,
    /// 세부 유구 확대도
    DetailFeature,
}

/// 다중 뷰포트 프레임 설정 (C++ `ViewportFrameConfig`)
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportFrameConfig {
    pub role: ViewportRole,
    pub page_layout_rect_mm: SheetRectMm,
    pub world_extent: [f64; 4],
    pub scale_denominator: f64,
    pub auto_figure_label: String,
}

/// 축척 분모가 0 이하이거나 비정상일 때 기본 축척(`1:500`)으로 폴백
#[must_use]
pub fn sanitize_viewport_scale(scale_denominator: f64) -> f64 {
    if scale_denominator <= 0.0 || !scale_denominator.is_finite() {
        HGIS_DEFAULT_PLAN_SCALE
    } else {
        scale_denominator
    }
}

/// 시트 번호와 서브 프레임 인덱스에 따라 국가유산청 보고서 규격 도면 번호 포맷팅
/// (C++ `KaFigureNumberingEngine::formatFigureNumber` 1:1 대응)
#[must_use]
pub fn format_figure_number(sheet_no: usize, sub_index_1based: Option<usize>) -> String {
    let s = sheet_no.max(1);
    match sub_index_1based {
        Some(sub) => format!("도면 {s}-{}", sub.max(1)),
        None => format!("도면 {s}"),
    }
}

/// 단일 시트 내 다중 뷰포트 목록에 대해 `"도면 {sheet}-{idx}"` 연속 도면 번호 자동 채번
/// (C++ `KaMultiViewportSheet::layoutMultiViewportSheet` & `KaFigureNumberingEngine`)
#[must_use]
pub fn assign_cascading_figure_numbers(
    base_sheet_number: usize,
    roles: &[ViewportRole],
) -> Vec<String> {
    let clamped = roles.len().min(HGIS_MAX_FIGURES_PER_SHEET);
    (0..clamped)
        .map(|idx| format_figure_number(base_sheet_number, Some(idx + 1)))
        .collect()
}

pub const HGIS_LEADER_LINE_MAX_DIST_MM: f64 = 25.0;

pub const HGIS_LABEL_COLLISION_PENALTY: f64 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelCandidatePos {
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelPlacementTask {
    pub fid: i64,
    pub label_text: String,
    pub feature_anchor_point_mm: [f64; 2],
    pub text_size_mm: [f64; 2],
    pub priority: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLabelResult {
    pub fid: i64,
    pub final_label_rect_mm: SheetRectMm,
    pub needs_leader_line: bool,
    pub leader_line_path_mm: Vec<[f64; 2]>,
}

pub(crate) fn rects_overlap_mm(a: SheetRectMm, b: SheetRectMm) -> f64 {
    let ox = (a.x_mm + a.width_mm).min(b.x_mm + b.width_mm) - a.x_mm.max(b.x_mm);
    let oy = (a.y_mm + a.height_mm).min(b.y_mm + b.height_mm) - a.y_mm.max(b.y_mm);
    if ox > 0.0 && oy > 0.0 {
        ox * oy
    } else {
        0.0
    }
}

/// C++ `KaLabelPlacementEngine::solveLabelPlacement` & `KaLeaderLineRouter` — 8방향 후보 위치 비용 함수 최소화, 장애물 충돌 회피 및 2단 굴절 지시선(`leaderLinePath`) 생성
#[must_use]
pub fn solve_label_placement(
    tasks: &[LabelPlacementTask],
    obstacle_rects_mm: &[SheetRectMm],
    viewport_bounds_mm: SheetRectMm,
) -> Vec<PlacedLabelResult> {
    let dirs: [(LabelCandidatePos, [f64; 2], f64); 8] = [
        (LabelCandidatePos::TopRight, [1.0, 1.0], 0.0),
        (LabelCandidatePos::Right, [1.0, 0.0], 0.2),
        (LabelCandidatePos::Top, [0.0, 1.0], 0.4),
        (LabelCandidatePos::BottomRight, [1.0, -1.0], 0.6),
        (LabelCandidatePos::TopLeft, [-1.0, 1.0], 0.8),
        (LabelCandidatePos::Left, [-1.0, 0.0], 1.0),
        (LabelCandidatePos::Bottom, [0.0, -1.0], 1.2),
        (LabelCandidatePos::BottomLeft, [-1.0, -1.0], 1.4),
    ];
    let radii_mm = [2.0_f64, 6.0, 12.0, 20.0];
    let leader_threshold_mm = 5.0;

    let mut placed_obstacles: Vec<SheetRectMm> = obstacle_rects_mm.to_vec();
    let mut results = Vec::with_capacity(tasks.len());

    for task in tasks {
        let [ax, ay] = task.feature_anchor_point_mm;
        let [tw, th] = [task.text_size_mm[0].max(1.0), task.text_size_mm[1].max(1.0)];
        let mut best_rect: Option<(f64, SheetRectMm, f64)> = None;

        for &r in &radii_mm {
            if r > HGIS_LEADER_LINE_MAX_DIST_MM {
                continue;
            }
            for &(_, dir, dir_bias) in &dirs {
                let norm = dir[0].hypot(dir[1]).max(1.0);
                let cx = ax + (dir[0] / norm) * r;
                let cy = ay + (dir[1] / norm) * r;
                let cand_rect = SheetRectMm {
                    x_mm: if dir[0] >= 0.0 { cx } else { cx - tw },
                    y_mm: if dir[1] >= 0.0 { cy } else { cy - th },
                    width_mm: tw,
                    height_mm: th,
                };
                // Check inside viewport bounds
                let inside = cand_rect.x_mm >= viewport_bounds_mm.x_mm
                    && cand_rect.y_mm >= viewport_bounds_mm.y_mm
                    && (cand_rect.x_mm + cand_rect.width_mm)
                        <= (viewport_bounds_mm.x_mm + viewport_bounds_mm.width_mm)
                    && (cand_rect.y_mm + cand_rect.height_mm)
                        <= (viewport_bounds_mm.y_mm + viewport_bounds_mm.height_mm);
                if !inside {
                    continue;
                }
                let overlap_sum: f64 = placed_obstacles
                    .iter()
                    .map(|&obs| rects_overlap_mm(cand_rect, obs))
                    .sum();
                let cost = if overlap_sum > 0.0 {
                    HGIS_LABEL_COLLISION_PENALTY * (1.0 + overlap_sum)
                } else {
                    0.0
                } + 2.5 * r
                    + dir_bias;

                if best_rect.is_none_or(|(best_cost, _, _)| cost < best_cost) {
                    best_rect = Some((cost, cand_rect, r));
                }
            }
        }

        if let Some((_, rect, dist)) = best_rect {
            placed_obstacles.push(rect);
            let needs_leader_line = dist > leader_threshold_mm;
            let leader_line_path_mm = if needs_leader_line {
                let end_x = if rect.x_mm >= ax {
                    rect.x_mm
                } else {
                    rect.x_mm + rect.width_mm
                };
                let end_y = rect.y_mm + rect.height_mm * 0.5;
                let elbow_x = if rect.x_mm >= ax {
                    end_x - 3.0
                } else {
                    end_x + 3.0
                };
                vec![[ax, ay], [elbow_x, end_y], [end_x, end_y]]
            } else {
                Vec::new()
            };
            results.push(PlacedLabelResult {
                fid: task.fid,
                final_label_rect_mm: rect,
                needs_leader_line,
                leader_line_path_mm,
            });
        }
    }

    results
}

/// QGIS 조판 템플릿(`.qpt` XML) 내 동적 슬롯 플레이스홀더(`[% $KEY %]` 또는 `[%KEY%]`)를 바인딩 값으로 치환
/// (C++ `PrintLayoutTemplate::instantiateFromQpt` 1:1 대응)
#[must_use]
pub fn substitute_qpt_template_slots(template_xml: &str, bindings: &[(&str, &str)]) -> String {
    let mut out = template_xml.to_owned();
    for &(key, val) in bindings {
        let bare = key.trim().trim_start_matches('$');
        for pattern in [
            format!("[% ${bare} %]"),
            format!("[%${bare}%]"),
            format!("[% {bare} %]"),
            format!("[%{bare}%]"),
        ] {
            out = out.replace(&pattern, val);
        }
    }
    out
}
