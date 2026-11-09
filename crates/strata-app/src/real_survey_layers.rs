//! 레이어·라벨 표시 상태와 파일 목록 갱신.
//!
//! `real_survey.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목의 이름·시그니처·동작은 그대로다.

use std::path::PathBuf;

use super::real_survey_types::RealSurveyData;

impl RealSurveyData {
    pub fn is_layer_visible(&self, name: &str) -> bool {
        *self.layer_visible.get(name).unwrap_or(&true)
    }
    /// `BasemapOps::toggleLayerVisibility` & `revealCheckedLegendNode` (`c9e58e3`) — 전체 끄기 후 접힌 하위 레이어 묶음만 체크해도 자식 레이어와 상위 그룹이 함께 켜짐
    pub fn toggle_layer_visible(&mut self, name: &str) {
        const HERITAGE_LEAVES: [&str; 5] = [
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ];
        let next = !self.is_layer_visible(name);
        self.layer_visible.insert(name.to_owned(), next);
        if name == "지정유산" {
            if next {
                let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
                if !any_leaf_on {
                    for k in HERITAGE_LEAVES {
                        self.layer_visible.insert(k.to_owned(), true);
                    }
                }
                self.layer_visible.insert("참조 지도".to_owned(), true);
            } else {
                for k in HERITAGE_LEAVES {
                    self.layer_visible.insert(k.to_owned(), false);
                }
            }
        } else if name == "참조 지도" {
            if next {
                let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
                if !any_leaf_on {
                    self.layer_visible.insert("지정유산".to_owned(), true);
                    for k in HERITAGE_LEAVES {
                        self.layer_visible.insert(k.to_owned(), true);
                    }
                }
            } else {
                self.layer_visible.insert("지정유산".to_owned(), false);
                for k in HERITAGE_LEAVES {
                    self.layer_visible.insert(k.to_owned(), false);
                }
            }
        } else if HERITAGE_LEAVES.contains(&name) {
            let any_leaf_on = HERITAGE_LEAVES.iter().any(|k| self.is_layer_visible(k));
            self.layer_visible
                .insert("지정유산".to_owned(), any_leaf_on);
            self.layer_visible
                .insert("참조 지도".to_owned(), any_leaf_on);
        }
    }

    /// C++ `KaLayerSectionDelegate` (`Qt::CheckState`: 0 = Unchecked, 1 = PartiallyChecked, 2 = Checked)
    pub fn layer_check_state(&self, name: &str) -> u8 {
        const HERITAGE_LEAVES: [&str; 5] = [
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ];
        if name == "지정유산" || name == "참조 지도" {
            if !self.is_layer_visible(name) {
                return 0;
            }
            let on_cnt = HERITAGE_LEAVES
                .iter()
                .filter(|&&k| self.is_layer_visible(k))
                .count();
            if on_cnt == 0 {
                0
            } else if on_cnt < HERITAGE_LEAVES.len() {
                1
            } else {
                2
            }
        } else if self.is_layer_visible(name) {
            2
        } else {
            0
        }
    }
    pub fn is_label_visible(&self, name: &str) -> bool {
        *self.label_visible.get(name).unwrap_or(&true)
    }
    pub fn toggle_label_visible(&mut self, name: &str) {
        let next = !self.is_label_visible(name);
        self.label_visible.insert(name.to_owned(), next);
    }

    pub fn set_all_layers_visible(&mut self, visible: bool) {
        for k in [
            "지적도 · 조사 주변 5km",
            "시굴격자",
            "지형 음영",
            "지표모델(DSM) 30m · Coperni...",
            "조사구역",
            "주변 500m 거리",
            "주변 500m",
            "참조 지도",
            "지정유산",
            "국가등록문화유산",
            "국가지정유산",
            "국가지정유산보호구역",
            "시도지정유산",
            "시도지정유산보호구역",
        ] {
            self.layer_visible.insert(k.to_owned(), visible);
        }
    }

    pub fn refresh_file_browser(&mut self) {
        self.file_browser_entries.clear();
        let Ok(rd) = std::fs::read_dir(&self.file_browser_dir) else {
            return;
        };
        let mut items: Vec<(String, PathBuf, bool)> = rd
            .flatten()
            .map(|e| {
                let p = e.path();
                (
                    e.file_name().to_string_lossy().to_string(),
                    p.clone(),
                    p.is_dir(),
                )
            })
            .filter(|(n, _, _)| !n.starts_with('.'))
            .collect();
        items.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        self.file_browser_entries = items;
    }
}
