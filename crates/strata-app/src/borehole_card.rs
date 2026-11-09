//! C++ `KaFeatureCard.cpp:261-278` + `KaFeatureCardEditors.cpp:202-219` (`d7afcc8`) 1:1.
//!
//! 시추공 레이어(`ka_hgis/geoinfo_borehole`)를 고르면 오른쪽 속성 카드가 주상도 층서를
//! 읽기 전용으로 보여 준다. 유구 카드와 같은 카드 틀을 쓰되 시추 항목만 다르다.

/// C++ `KaFeatureCardEditors.cpp:202-209` — `hole_code` 가 있는 레이어에서만 나오는 여섯 줄.
/// 앞의 문자열은 속성 필드 이름, 뒤는 카드에 보이는 한국어 라벨이다.
pub const BOREHOLE_CARD_ROWS: [(&str, &str); 6] = [
    ("hole_code", "시추공번"),
    ("soil_dominant", "우세 토질"),
    ("alluvial_m", "충적층(m)"),
    ("fill_m", "매립·성토(m)"),
    ("weathered_m", "기반암 GL-(m)"),
    ("water_m", "지하수위(m)"),
];

/// C++ `KaFeatureCardEditors.cpp:218` — `layer_summary` 가 있으면 메모 줄 이름이 「지층 구성」이 된다.
pub const BOREHOLE_SUMMARY_ROW_LABEL: &str = "지층 구성";

/// C++ `KaFeatureCardEditors.cpp:218` — `layer_summary` 가 없을 때의 기본 메모 줄 이름.
pub const FEATURE_NOTE_ROW_LABEL: &str = "메모";

/// C++ `KaFeatureCard.cpp:264` — `hole_code` 가 있는 카드는 이름 없음 대신 공번을 머리글로 쓴다.
pub const BOREHOLE_HEADER_FIELD: &str = "hole_code";

/// 한 시추공의 카드에 들어갈 값. C++ 은 `QgsFeature` 속성에서 읽지만 Rust 는 이 구조체로 받는다.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoreholeCardData {
    pub hole_code: String,
    pub soil_dominant: String,
    pub alluvial_m: f64,
    pub fill_m: f64,
    pub weathered_m: f64,
    pub water_m: f64,
    /// C++ `layer_summary` — `0.0~1.2m 매립토 → 1.2~5.4m 실트질점토 → 5.4~12.0m 풍화암` 같은 한 줄 요약.
    pub layer_summary: String,
    /// 현장 메모. 비어 있으면 `layer_summary` 가 대신 보인다.
    pub note: String,
}

impl BoreholeCardData {
    /// C++ 은 `hole_code` 가 비었는지로 시추 카드를 그릴지 정한다(`KaFeatureCardEditors.cpp:202`).
    #[must_use]
    pub fn has_borehole_fields(&self) -> bool {
        !self.hole_code.trim().is_empty()
    }

    /// C++ `KaFeatureCard.cpp:264` — `header` 가 비어 있으면 `hole_code` 를 머리글로 쓴다.
    #[must_use]
    pub fn header_text(&self) -> String {
        let code = self.hole_code.trim();
        if code.is_empty() {
            "이름 없음".to_owned()
        } else {
            code.to_owned()
        }
    }

    /// C++ `KaFeatureCard.cpp:278` — `note` 가 비어 있으면 `layer_summary` 를 보여 준다.
    #[must_use]
    pub fn note_or_summary(&self) -> &str {
        if self.note.trim().is_empty() {
            &self.layer_summary
        } else {
            &self.note
        }
    }

    /// C++ `KaFeatureCardEditors.cpp:218` — 메모 줄 이름. `layer_summary` 가 있으면 「지층 구성」.
    #[must_use]
    pub fn note_row_label(&self) -> &'static str {
        if self.layer_summary.trim().is_empty() {
            FEATURE_NOTE_ROW_LABEL
        } else {
            BOREHOLE_SUMMARY_ROW_LABEL
        }
    }

    /// 카드에 그릴 `(라벨, 값)` 여섯 줄. 값은 C++ `showText` 처럼 소수 한 자리로 자른다.
    /// `weathered_m < 0` 은 C++ `-1` 관례대로 「미출현」이다.
    #[must_use]
    pub fn formatted_rows(&self) -> Vec<(&'static str, String)> {
        BOREHOLE_CARD_ROWS
            .iter()
            .map(|(field, label)| {
                let value = match *field {
                    "hole_code" => self.hole_code.clone(),
                    "soil_dominant" => self.soil_dominant.clone(),
                    "alluvial_m" => format!("{:.1}", self.alluvial_m),
                    "fill_m" => format!("{:.1}", self.fill_m),
                    "weathered_m" => {
                        if self.weathered_m < 0.0 {
                            "미출현".to_owned()
                        } else {
                            format!("{:.1}", self.weathered_m)
                        }
                    }
                    "water_m" => format!("{:.1}", self.water_m.abs()),
                    _ => String::new(),
                };
                (*label, value)
            })
            .collect()
    }

    /// 마스터플랜 Chapter 01 목업의 카드 머리글:
    /// `[공번] 충적층 4.2m / 기반암 GL-6.5m (세립질 점토 우세)` 꼴의 한 줄 요약.
    /// C++ `GeoInfoBoreholeService` 의 `map_label` 과 같은 규칙이다.
    #[must_use]
    pub fn map_label(&self) -> String {
        let bed = if self.weathered_m >= 0.0 {
            format!("{:.1}", self.weathered_m)
        } else {
            format!("{:.1}", self.fill_m + self.alluvial_m)
        };
        let soil = self.soil_dominant.trim();
        let suffix = if soil.is_empty() {
            String::new()
        } else {
            format!(" ({soil} 우세)")
        };
        format!(
            "{} 충적층 {:.1}m / 기반암 GL-{}m{}",
            self.hole_code.trim(),
            self.alluvial_m,
            bed,
            suffix
        )
    }
}

/// C++ `KaFeatureCard` 의 시추공 카드 부분을 egui 로 그린다.
///
/// `KaFeatureCardEditors.cpp:202-209` 여섯 줄을 읽기 전용으로, `KaFeatureCard.cpp:278` 의
/// 「지층 구성」 줄을 그 아래에 둔다. 시추공 자료는 참조 지도라 고칠 수 없다
/// (`KaFeatureCardEditors.cpp:220` `isReferenceLayer` → `setEnabled(false)`).
pub fn render_borehole_card(ui: &mut egui::Ui, data: &BoreholeCardData) {
    use eframe::egui::{self, Color32, Stroke};

    egui::Frame::NONE
        .fill(Color32::from_rgb(0xFA, 0xF9, 0xF5))
        .stroke(Stroke::new(1.0, Color32::from_rgb(0xDE, 0xDC, 0xD1)))
        .corner_radius(5.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(data.header_text())
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(0x14, 0x14, 0x13)),
            );
            ui.add_space(3.0);
            egui::Grid::new("ka_borehole_card_rows")
                .num_columns(2)
                .spacing([6.0, 3.0])
                .show(ui, |ui| {
                    for (label, value) in data.formatted_rows() {
                        ui.label(
                            egui::RichText::new(label)
                                .size(11.5)
                                .color(Color32::from_rgb(0x5E, 0x5D, 0x59)),
                        );
                        ui.label(
                            egui::RichText::new(value)
                                .size(11.5)
                                .color(Color32::from_rgb(0x30, 0x30, 0x2E)),
                        );
                        ui.end_row();
                    }
                });
            let summary = data.note_or_summary();
            if !summary.trim().is_empty() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(data.note_row_label())
                        .size(11.5)
                        .strong()
                        .color(Color32::from_rgb(0x9C, 0x4A, 0x2F)),
                );
                ui.label(
                    egui::RichText::new(summary)
                        .size(11.5)
                        .color(Color32::from_rgb(0x30, 0x30, 0x2E)),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> BoreholeCardData {
        BoreholeCardData {
            hole_code: "BH-07".to_owned(),
            soil_dominant: "세립질 점토".to_owned(),
            alluvial_m: 4.2,
            fill_m: 1.2,
            weathered_m: 5.4,
            water_m: -2.1,
            layer_summary: "0.0~1.2m 매립토 → 1.2~5.4m 실트질점토 → 5.4~12.0m 풍화암".to_owned(),
            note: String::new(),
        }
    }

    /// C++ `KaFeatureCardEditors.cpp:202-209` 여섯 줄의 라벨과 순서 1:1.
    #[test]
    fn test_borehole_card_rows_match_cpp_order() {
        let d = sample();
        assert!(d.has_borehole_fields());
        let rows = d.formatted_rows();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], ("시추공번", "BH-07".to_owned()));
        assert_eq!(rows[1], ("우세 토질", "세립질 점토".to_owned()));
        assert_eq!(rows[2], ("충적층(m)", "4.2".to_owned()));
        assert_eq!(rows[3], ("매립·성토(m)", "1.2".to_owned()));
        assert_eq!(rows[4], ("기반암 GL-(m)", "5.4".to_owned()));
        assert_eq!(rows[5], ("지하수위(m)", "2.1".to_owned()));
    }

    /// C++ `KaFeatureCard.cpp:264` — 머리글은 공번, 비면 「이름 없음」.
    #[test]
    fn test_borehole_header_uses_hole_code_like_cpp() {
        assert_eq!(sample().header_text(), "BH-07");
        let mut d = sample();
        d.hole_code = "  ".to_owned();
        assert_eq!(d.header_text(), "이름 없음");
        assert!(!d.has_borehole_fields());
    }

    /// C++ `KaFeatureCard.cpp:278` — 메모가 비면 `layer_summary` 가 보이고, 줄 이름이 「지층 구성」이다.
    #[test]
    fn test_borehole_note_falls_back_to_layer_summary_like_cpp() {
        let d = sample();
        assert_eq!(d.note_or_summary(), d.layer_summary);
        assert_eq!(d.note_row_label(), "지층 구성");

        let mut with_note = sample();
        with_note.note = "서쪽 3m 지점에서 물이 샙".to_owned();
        assert_eq!(with_note.note_or_summary(), "서쪽 3m 지점에서 물이 샙");

        let mut no_summary = sample();
        no_summary.layer_summary = String::new();
        assert_eq!(no_summary.note_row_label(), "메모");
    }

    /// C++ `-1` 관례: `weathered_m < 0` 은 기반암 미출현이다.
    #[test]
    fn test_borehole_weathered_minus_one_is_not_reached() {
        let mut d = sample();
        d.weathered_m = -1.0;
        let rows = d.formatted_rows();
        assert_eq!(rows[4], ("기반암 GL-(m)", "미출현".to_owned()));
        // 미출현이면 GL 은 fill + alluvial 이다(GeoInfoBoreholeMaps.cpp:216).
        assert_eq!(d.map_label(), "BH-07 충적층 4.2m / 기반암 GL-5.4m (세립질 점토 우세)");
    }

    /// Chapter 01 목업의 마커 라벨 형식.
    #[test]
    fn test_borehole_map_label_matches_mockup_format() {
        assert_eq!(
            sample().map_label(),
            "BH-07 충적층 4.2m / 기반암 GL-5.4m (세립질 점토 우세)"
        );
        let mut no_soil = sample();
        no_soil.soil_dominant = String::new();
        assert_eq!(
            no_soil.map_label(),
            "BH-07 충적층 4.2m / 기반암 GL-5.4m"
        );
    }
}
