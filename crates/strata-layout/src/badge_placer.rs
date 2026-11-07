#[derive(Debug, Clone, PartialEq)]
pub struct HeritageBadge {
    pub number: u32,
    pub x_mm: f64,
    pub y_mm: f64,
    pub radius_mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeritageNameBox {
    pub number: u32,
    pub name: String,
    pub center_x_mm: f64,
    pub center_y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl HeritageNameBox {
    #[must_use]
    pub fn overlaps(&self, other: &Self, margin_mm: f64) -> bool {
        let half_w = (self.width_mm + other.width_mm) * 0.5 + margin_mm;
        let half_h = (self.height_mm + other.height_mm) * 0.5 + margin_mm;
        (self.center_x_mm - other.center_x_mm).abs() < half_w
            && (self.center_y_mm - other.center_y_mm).abs() < half_h
    }
}

pub const HGIS_LEADER_MIN_DISTANCE_MM: f64 = 15.0;
pub const HGIS_BADGE_NAMEBOX_SCALE_LIMIT: u32 = 5_000;

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBadgeWithLeader {
    pub box_item: HeritageNameBox,
    pub origin_mm: [f64; 2],
    pub displacement_mm: f64,
    pub leader_line: Option<[[f64; 2]; 3]>,
}

#[derive(Debug, Default, Clone)]
pub struct LayoutBadgePlacer;

impl LayoutBadgePlacer {
    /// `HeritageLayoutNumbers.cpp` (`bfe8dd9`) — 1:5,000 이하 대축척에서 체크 켜짐 시에만 유적 이름으로 표시
    #[must_use]
    pub const fn should_render_site_names(
        scale_denominator: u32,
        prefer_names_under_5000: bool,
    ) -> bool {
        prefer_names_under_5000 && scale_denominator <= HGIS_BADGE_NAMEBOX_SCALE_LIMIT
    }

    #[must_use]
    pub fn place_non_overlapping_badges(
        points_mm: &[(u32, f64, f64)],
        radius_mm: f64,
    ) -> Vec<HeritageBadge> {
        let mut placed: Vec<HeritageBadge> = Vec::with_capacity(points_mm.len());
        let min_dist = radius_mm * 2.2;
        for &(number, mut x, mut y) in points_mm {
            for existing in &placed {
                let dx = x - existing.x_mm;
                let dy = y - existing.y_mm;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    x += min_dist - dist + 0.5;
                    y -= 0.5;
                }
            }
            placed.push(HeritageBadge {
                number,
                x_mm: x,
                y_mm: y,
                radius_mm,
            });
        }
        placed
    }

    /// `LayoutBadgePlacer::placeAllBoxes` (`bfe8dd9`) — 가로로 긴 유적 이름 상자끼리 AABB가 겹치지 않게 8방향 링 탐색 배치 (지시선 없음, 슬롯 교환 없음)
    #[must_use]
    pub fn place_all_boxes(
        sites: &[(u32, String, f64, f64, f64, f64)],
        allowed_rect_mm: Option<(f64, f64, f64, f64)>,
    ) -> Vec<HeritageNameBox> {
        const EDGE_MARGIN_MM: f64 = 0.6;
        const DIAG: f64 = std::f64::consts::FRAC_1_SQRT_2;
        const DIRS: [(f64, f64); 8] = [
            (1.0, 0.0),
            (0.0, -1.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (DIAG, -DIAG),
            (-DIAG, -DIAG),
            (DIAG, DIAG),
            (-DIAG, DIAG),
        ];
        let mut placed: Vec<HeritageNameBox> = Vec::with_capacity(sites.len());

        for (number, name, ox, oy, w, h) in sites {
            let width_mm = w.max(6.0);
            let height_mm = h.max(3.5);
            let fits_bounds = |cx: f64, cy: f64| -> bool {
                if let Some((min_x, min_y, max_x, max_y)) = allowed_rect_mm {
                    let hw = width_mm * 0.5 + EDGE_MARGIN_MM;
                    let hh = height_mm * 0.5 + EDGE_MARGIN_MM;
                    cx - hw >= min_x && cy - hh >= min_y && cx + hw <= max_x && cy + hh <= max_y
                } else {
                    true
                }
            };

            let mut chosen = (*ox, *oy);
            let initial_candidate = HeritageNameBox {
                number: *number,
                name: name.clone(),
                center_x_mm: *ox,
                center_y_mm: *oy,
                width_mm,
                height_mm,
            };
            if !fits_bounds(*ox, *oy)
                || placed
                    .iter()
                    .any(|p| initial_candidate.overlaps(p, EDGE_MARGIN_MM))
            {
                'search: for ring in 1..=20 {
                    let step_x = (width_mm * 0.65 + 1.2) * f64::from(ring);
                    let step_y = (height_mm * 0.85 + 1.0) * f64::from(ring);
                    for (dx, dy) in DIRS {
                        let cx = *ox + dx * step_x;
                        let cy = *oy + dy * step_y;
                        if !fits_bounds(cx, cy) {
                            continue;
                        }
                        let cand = HeritageNameBox {
                            number: *number,
                            name: name.clone(),
                            center_x_mm: cx,
                            center_y_mm: cy,
                            width_mm,
                            height_mm,
                        };
                        if !placed.iter().any(|p| cand.overlaps(p, EDGE_MARGIN_MM)) {
                            chosen = (cx, cy);
                            break 'search;
                        }
                    }
                }
            }

            placed.push(HeritageNameBox {
                number: *number,
                name: name.clone(),
                center_x_mm: chosen.0,
                center_y_mm: chosen.1,
                width_mm,
                height_mm,
            });
        }
        placed
    }

    /// C++ `DrawingBadgePlacer::placeAllBoxes(mapItem, badges, scaleDenom)` 축척 분모 필터링 통합 배치 1:1
    #[must_use]
    pub fn place_badges_for_scale(
        sites: &[(u32, String, f64, f64, f64, f64)],
        scale_denominator: u32,
        prefer_names_under_5000: bool,
        allowed_rect_mm: Option<(f64, f64, f64, f64)>,
    ) -> (Vec<HeritageBadge>, Vec<HeritageNameBox>) {
        let points_mm: Vec<(u32, f64, f64)> = sites
            .iter()
            .map(|(n, _, x, y, _, _)| (*n, *x, *y))
            .collect();
        let badges = Self::place_non_overlapping_badges(&points_mm, 2.2);
        let name_boxes =
            if Self::should_render_site_names(scale_denominator, prefer_names_under_5000) {
                Self::place_all_boxes(sites, allowed_rect_mm)
            } else {
                Vec::new()
            };
        (badges, name_boxes)
    }

    /// C++ `KaBadgePlacer::placeBadgesAndLeaders` — 8방향 나선형 충돌 회피 배치 및 이격 거리 `>= 15.0mm`일 때 꺾인 지시선(Elbow Leader Line) 자동 생성
    #[must_use]
    pub fn place_boxes_with_leaders(
        sites: &[(u32, String, f64, f64, f64, f64)],
        allowed_rect_mm: Option<(f64, f64, f64, f64)>,
    ) -> Vec<PlacedBadgeWithLeader> {
        let boxes = Self::place_all_boxes(sites, allowed_rect_mm);
        boxes
            .into_iter()
            .zip(sites.iter())
            .map(|(b, &(_, _, ox, oy, _, _))| {
                let dist = (b.center_x_mm - ox).hypot(b.center_y_mm - oy);
                let leader_line = if dist >= HGIS_LEADER_MIN_DISTANCE_MM {
                    let elbow_x = 0.5 * (ox + b.center_x_mm);
                    Some([
                        [ox, oy],
                        [elbow_x, b.center_y_mm],
                        [b.center_x_mm, b.center_y_mm],
                    ])
                } else {
                    None
                };
                PlacedBadgeWithLeader {
                    box_item: b,
                    origin_mm: [ox, oy],
                    displacement_mm: dist,
                    leader_line,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_boxes_never_cover_each_other_and_respect_5000_scale_rule() {
        assert!(LayoutBadgePlacer::should_render_site_names(5_000, true));
        assert!(LayoutBadgePlacer::should_render_site_names(2_500, true));
        assert!(!LayoutBadgePlacer::should_render_site_names(5_001, true));
        assert!(!LayoutBadgePlacer::should_render_site_names(2_500, false));

        let sites = vec![
            (1, "경주 황남동 고분군".to_owned(), 50.0, 50.0, 24.0, 5.0),
            (2, "경주 인왕동 사지".to_owned(), 51.0, 50.5, 22.0, 5.0),
            (3, "경주 월성 해자".to_owned(), 50.5, 50.2, 20.0, 5.0),
        ];
        let boxes = LayoutBadgePlacer::place_all_boxes(&sites, Some((10.0, 10.0, 190.0, 260.0)));
        assert_eq!(boxes.len(), 3);
        for i in 0..boxes.len() {
            for j in (i + 1)..boxes.len() {
                assert!(
                    !boxes[i].overlaps(&boxes[j], 0.5),
                    "boxes {} and {} overlapped: {:?} vs {:?}",
                    i,
                    j,
                    boxes[i],
                    boxes[j]
                );
            }
        }
    }

    #[test]
    fn test_badge_placer_scale_filtering_1000_emits_name_boxes_and_10000_emits_only_numbered_circles(
    ) {
        let sites = vec![
            (1, "경주 황남동 고분군".to_owned(), 50.0, 50.0, 24.0, 5.0),
            (2, "경주 인왕동 사지".to_owned(), 55.0, 60.0, 22.0, 5.0),
        ];
        let (badges_1k, names_1k) =
            LayoutBadgePlacer::place_badges_for_scale(&sites, 1_000, true, None);
        assert_eq!(badges_1k.len(), 2);
        assert_eq!(names_1k.len(), 2);

        let (badges_10k, names_10k) =
            LayoutBadgePlacer::place_badges_for_scale(&sites, 10_000, true, None);
        assert_eq!(badges_10k.len(), 2);
        assert!(names_10k.is_empty());
    }

    #[test]
    fn test_overlapping_points_zero_collisions_and_leader_generation_over_15mm() {
        let overlapping_sites = vec![
            (1, "경주 월성 1지점".to_owned(), 100.0, 100.0, 26.0, 18.0),
            (2, "경주 월성 2지점".to_owned(), 100.0, 100.0, 26.0, 18.0),
            (3, "경주 월성 3지점".to_owned(), 100.0, 100.0, 26.0, 18.0),
        ];
        let placed = LayoutBadgePlacer::place_boxes_with_leaders(&overlapping_sites, None);
        assert_eq!(placed.len(), 3);
        for i in 0..placed.len() {
            for j in (i + 1)..placed.len() {
                assert!(!placed[i].box_item.overlaps(&placed[j].box_item, 0.5));
            }
        }
        let leader_count = placed.iter().filter(|p| p.leader_line.is_some()).count();
        assert!(
            leader_count >= 2,
            "expected >= 2 displaced boxes to generate leader lines, got {leader_count}"
        );
    }
}
