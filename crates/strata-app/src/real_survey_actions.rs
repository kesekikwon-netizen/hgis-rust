//! 시굴 트렌치 이동. (F1: 경주 기본점으로 거리를 잰 주변유적 CSV 와, 쓰이지 않던 기본점 둘레 트렌치 배치를 없앴다.)
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·시그니처·동작은 그대로다.



use super::real_survey_types::RealSurveyData;

impl RealSurveyData {
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
