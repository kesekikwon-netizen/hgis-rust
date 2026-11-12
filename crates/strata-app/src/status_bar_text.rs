//! 상태줄 글 — C++ `MainWindow::updateNextActionStatus`(MainWindow.cpp:393-413), 도구 칸
//! `currentToolLabel`(KaDrawSketchTools.cpp:173-212)·`updateToolChip`(:239-262), 좌표 칸
//! `KaStatusBar::setCoordinate`·`clearCoordinate`(KaStatusBar.cpp:167-175), 축척(QGIS 96 dpi).

use strata_canvas2d::map_tools::MapToolKind;
use strata_types::schema::{DomainLayerKey, LegendGroup};

use crate::app_state::{MainTab, StrataAppState};

/// 96 dpi 화면에서 1 m 의 점 수(QGIS 축척 = 점당 m × 96 / 0.0254).
pub(crate) const POINTS_PER_METER: f64 = 96.0 / 0.0254;

/// 천 단위 쉼표, 소수 셋째 자리(QLocale 한국어 `toString(x, 'f', 3)`).
fn grouped3(v: f64) -> String {
    let s = format!("{:.3}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((s.as_str(), "000"));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    let sign = if v < 0.0 && s.chars().any(|c| matches!(c, '1'..='9')) { "-" } else { "" };
    format!("{sign}{out}.{frac}")
}

impl StrataAppState {
    /// `updateNextActionStatus` — 다음에 할 일 한 줄.
    pub(crate) fn next_action_status(&self) -> &'static str {
        if self.survey_path.is_none() {
            return "먼저 「새 조사」로 오늘 현장을 만드세요.";
        }
        let rs = &self.home_map.real_survey;
        let has_bg = self.left_layers.entries.iter().any(|e| {
            matches!(e.group, LegendGroup::Basemap | LegendGroup::Cadastral | LegendGroup::ReferenceMap)
                && rs.is_layer_visible(&e.label_ko)
        });
        let has_draw = self.left_layers.entries.iter().any(|e| DomainLayerKey::from_id(&e.layer_key).is_some());
        if !has_bg {
            "위성·지적이 없습니다. 더보기 → API 키 입력에서 VWorld 키를 확인하세요."
        } else if !has_draw {
            "「그리기」로 구역을 그리세요."
        } else {
            "다 그렸으면 리본의 「도면」으로 종이에 옮기세요."
        }
    }

    /// 상태줄 왼쪽 글 — 도면 탭 안내, 잠깐 띄운 알림, 그리기 안내, 없으면 다음 할 일.
    pub(crate) fn status_left_message(&self) -> &str {
        if self.active_tab == MainTab::Drawing {
            // MainWindowExport.cpp:252 — 도면 탭 안내는 C++ 원문 그대로.
            return StrataAppState::DRAWING_STUDIO_WELCOME;
        }
        if !self.status_bar_hint.is_empty() {
            return &self.status_bar_hint;
        }
        self.home_map.draw_guide_message.as_deref().unwrap_or_else(|| self.next_action_status())
    }

    /// `currentToolLabel` — 지도 탭에서만, 지금 켜진 도구 이름.
    pub(crate) fn tool_chip_label(&self) -> String {
        if self.active_tab != MainTab::Map {
            return String::new();
        }
        match self.active_tool {
            MapToolKind::Pan => "이동".to_owned(),
            MapToolKind::Select => "도형선택".to_owned(),
            MapToolKind::Measure => "줄자".to_owned(),
            MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint => {
                let target = match self.edit_layer_key.as_deref() {
                    Some("survey_area") => "조사구역",
                    Some("feature_poly") => "유구 면",
                    Some("feature_line") => "유구 선",
                    Some("section_line") => "단면선",
                    Some("control_points") => "기준점",
                    Some(k) if k.starts_with("artifact") => "유물 위치",
                    Some(k) if k.starts_with("survey_area_") => "조사구역",
                    _ => "",
                };
                let mut text = if target.is_empty() { "그리기".to_owned() } else { format!("그리기 · {target}") };
                // currentToolLabel 의 「점 n」 — 면은 그리던 면(KaCaptureMapTool), 선·점은 아직 예전 도구.
                let points = if self.active_tool == MapToolKind::DrawPolygon {
                    self.draw_sketch.points.len()
                } else {
                    self.home_map.scene.active_rubberband.len()
                };
                if points > 0 {
                    text.push_str(&format!(" · 점 {points}"));
                }
                text
            }
            _ => String::new(),
        }
    }

    /// 좌표 칸 — 마우스가 지도 위에 처음 오기 전에는 「X —    Y —」.
    pub(crate) fn xy_readout(&self) -> String {
        if !self.home_map.cursor_seen {
            return "X —    Y —".to_owned();
        }
        let [x, y] = self.home_map.cursor_world_xy;
        format!("X {}    Y {}", grouped3(x), grouped3(y))
    }

    /// 지도 화면 축척 분모(QGIS: 점당 m × 96 / 0.0254).
    pub(crate) fn map_scale_denominator(&self) -> u64 {
        (self.home_map.viewport.meters_per_pixel * POINTS_PER_METER).round().max(0.0) as u64
    }
}

impl StrataAppState {
    /// 「저장 안 됨」 칩 글(KaStatusBar::setUnsavedCount, KaStatusBar.cpp:152-160). 편집한 도형 수는
    /// `EditBufferSummary::uniqueEditedIds` 처럼 레이어마다 더한·지운·바꾼 도형 id 를 한 번씩 센 합.
    /// 조사가 없거나 저장할 것이 없으면 None(칩 숨김).
    pub(crate) fn unsaved_chip_text(&self) -> Option<String> {
        self.survey_path.as_ref()?;
        let mut features = 0usize;
        for t in &self.survey_tables {
            let base: std::collections::HashMap<i64, &strata_storage::edit_buffer::FeatureRecord> =
                t.baseline.iter().map(|f| (f.fid, f)).collect();
            let mut ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
            for f in &t.layer.features {
                if base.get(&f.fid).is_none_or(|b| b.wkt_geom != f.wkt_geom || b.attributes != f.attributes) {
                    ids.insert(f.fid);
                }
            }
            let now: std::collections::HashSet<i64> = t.layer.features.iter().map(|f| f.fid).collect();
            ids.extend(base.keys().filter(|fid| !now.contains(fid)));
            features += ids.len();
        }
        if features > 0 {
            Some(format!("저장 안 됨 {features}건"))
        } else {
            self.is_dirty.then(|| "저장 안 됨".to_owned())
        }
    }
}

#[cfg(test)]
#[path = "status_bar_text_tests.rs"]
mod tests;
