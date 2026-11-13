//! 검수 「위치 보기」 — C++ `MainWindow::goToCheckTargets`(KaSubmitFlow.cpp:191-240): 대상 레이어의 해당 도형들
//! (fid 가 없으면 레이어 전체) 바깥 상자로 지도를 옮기고 골라 둔다. 대상을 못 찾으면 그렇다고 알리고 옮기지 않는다.
//! (Rust 지도는 도형 하나만 골라 둘 수 있어 첫 도형을 고른다 — C++ 은 모두 고른다.)

use strata_check_export::checklist_engine::CheckResult;

use crate::app_state::{MainTab, StrataAppState};
use crate::survey_paint_layers::table_extent_in;

impl StrataAppState {
    pub(crate) fn go_to_check_targets(&mut self, result: &CheckResult) {
        let mut layer: Option<(usize, String)> = None;
        let (mut ids, mut whole) = (Vec::new(), false);
        for t in &result.targets {
            let Some(idx) = self.survey_tables.iter().position(|s| s.table == t.layer_id) else { continue };
            if layer.as_ref().is_some_and(|(i, _)| *i != idx) {
                continue;
            }
            layer.get_or_insert_with(|| (idx, if t.layer_name.is_empty() { t.layer_id.clone() } else { t.layer_name.clone() }));
            if t.feature_id >= 0 {
                ids.push(t.feature_id);
            } else {
                whole = true;
            }
        }
        let Some((idx, name)) = layer else {
            self.home_map.draw_guide_message = Some("해당 도형을 지도에서 찾지 못했습니다. 「다시 검수」로 확인하세요.".to_owned());
            return;
        };
        self.active_tab = MainTab::Map;
        let table = &self.survey_tables[idx];
        let table_key = table.table.clone();
        let bbox = if !ids.is_empty() {
            let mut picked = table.clone();
            picked.layer.features.retain(|f| ids.contains(&f.fid));
            table_extent_in(&picked, self.work_crs)
        } else if whole {
            table_extent_in(table, self.work_crs)
        } else {
            None
        };
        if let Some(&first) = ids.first() {
            self.home_map.selected_shape = Some((table_key, first));
        }
        let Some(b) = bbox.filter(|b| b.iter().all(|v| v.is_finite())) else {
            self.home_map.draw_guide_message = Some(format!(
                "「{name}」에서 {}개를 골라 두었습니다. 도형이 비어 지도에 보이지 않으니 속성표에서 지우세요.",
                ids.len()
            ));
            return;
        };
        let grow = (b[2] - b[0]).max(b[3] - b[1]).max(20.0) * 0.3;
        self.home_map.set_view_extent([b[0] - grow, b[1] - grow, b[2] + grow, b[3] + grow]);
        self.home_map.draw_guide_message = Some(if ids.is_empty() {
            format!("「{name}」 레이어로 이동했습니다: {}", result.message_ko)
        } else {
            format!("「{name}」의 해당 도형 {}개로 이동해 골라 두었습니다: {}", ids.len(), result.message_ko)
        });
    }
}

#[cfg(test)]
#[path = "app_state_check_targets_tests.rs"]
mod tests;
