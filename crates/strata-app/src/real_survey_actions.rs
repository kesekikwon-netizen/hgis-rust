//! 주변유적 CSV·시굴 트렌치 배치/이동.
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·시그니처·동작은 그대로다.

use std::path::PathBuf;

use strata_geo::trench_grid::{build_lshape_ring, TrenchShapeKind, TrenchStandardLength};

use super::real_survey_types::{ring_centroid, RealSurveyData, RealTrenchPoly};

impl RealSurveyData {
    pub fn export_nearby_heritage_csv(&self) -> Option<PathBuf> {
        let out_path = self.survey_dir.join("주변유적_목록.csv");
        let [cx, cy] = self.survey_center;
        let mut csv = String::from(
            "\u{feff}연번,분류,유적명,중심X(EPSG:5187),중심Y(EPSG:5187),이격거리(m)\n",
        );
        for (i, h) in self.heritage_polys.iter().enumerate() {
            let dist = ((h.center[0] - cx).powi(2) + (h.center[1] - cy).powi(2)).sqrt();
            csv.push_str(&format!(
                "{},{},{},{:.2},{:.2},{:.1}\n",
                i + 1,
                h.layer_name,
                h.site_name,
                h.center[0],
                h.center[1],
                dist
            ));
        }
        std::fs::write(&out_path, csv.as_bytes()).ok()?;
        Some(out_path)
    }

    /// `KaTrenchDialogPlan.cpp` & `TrenchGridGenerator.cpp` & `TrenchLShape.cpp` — 고른 길이(20·15·10 m) 또는 ㄱ·ㄴ자(15 m + 15 m) 10% 배치
    pub fn apply_trench_plan(
        &mut self,
        length: TrenchStandardLength,
        shape: TrenchShapeKind,
        grid_snap: bool,
    ) -> (usize, f64, f64) {
        let [cx, cy] = self.survey_center;
        let area_m2 = self.survey_area_m2.max(1000.0);
        let target_area = area_m2 * 0.10;
        let grid_step = if grid_snap { Some(5.0) } else { None };
        let mut built = Vec::new();

        if let Some(bend) = shape.arm_bend() {
            let one_area = 56.0;
            let count = ((target_area / one_area).ceil() as usize).max(1);
            let cols = ((count as f64).sqrt().ceil() as usize).max(1);
            let step = 25.0;
            for idx in 0..count {
                let r = idx / cols;
                let c = idx % cols;
                let u0 = cx - 90.0 + (c as f64) * step;
                let v0 = cy - 120.0 + (r as f64) * step;
                let ring = build_lshape_ring(u0, v0, 15.0, 2.0, bend, grid_step);
                let center = ring_centroid(&ring);
                built.push(RealTrenchPoly {
                    name: format!("Tr-{}", idx + 1),
                    ring,
                    area_m2: one_area,
                    center,
                });
            }
        } else {
            let len_m = length.meters();
            let wid_m = 2.0;
            let one_area = len_m * wid_m;
            let count = ((target_area / one_area).round() as usize).max(1);
            let cols = 8usize;
            let step_x = 12.0;
            let step_y = len_m + 10.0;
            for idx in 0..count {
                let r = idx / cols;
                let c = idx % cols;
                let mut x0 = cx - 42.0 + (c as f64) * step_x;
                let mut y0 = cy - 380.0 + (r as f64) * step_y;
                if let Some(st) = grid_step {
                    x0 = (x0 / st).round() * st;
                    y0 = (y0 / st).round() * st;
                }
                let ring = vec![
                    [x0, y0],
                    [x0 + wid_m, y0],
                    [x0 + wid_m, y0 + len_m],
                    [x0, y0 + len_m],
                    [x0, y0],
                ];
                let center = ring_centroid(&ring);
                built.push(RealTrenchPoly {
                    name: format!("Tr-{}", idx + 1),
                    ring,
                    area_m2: one_area,
                    center,
                });
            }
        }
        let total_area: f64 = built.iter().map(|t| t.area_m2).sum();
        let ratio_pct = (total_area / area_m2) * 100.0;
        let n = built.len();
        self.trenches = built;
        (n, total_area, ratio_pct)
    }

    /// `KaTrenchMoveToolKeys.cpp` — 화살표 키 또는 드래그로 트렌치를 통째로 이동 (좌표격자 켜짐 시 교점 스냅)
    pub fn translate_all_trenches(&mut self, dx: f64, dy: f64, snap_step: Option<f64>) {
        for tr in &mut self.trenches {
            let mut eff_dx = dx;
            let mut eff_dy = dy;
            if let (Some(step), Some(first)) = (snap_step.filter(|&s| s > 0.0), tr.ring.first()) {
                let target_x = ((first[0] + dx) / step).round() * step;
                let target_y = ((first[1] + dy) / step).round() * step;
                eff_dx = target_x - first[0];
                eff_dy = target_y - first[1];
            }
            for pt in &mut tr.ring {
                pt[0] += eff_dx;
                pt[1] += eff_dy;
            }
            tr.center[0] += eff_dx;
            tr.center[1] += eff_dy;
        }
    }
}
