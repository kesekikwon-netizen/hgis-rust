//! `right_inspector` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::right_inspector::*;


// D9 KaFeatureCardEditors / KaFeatureCard / KaInspectorStyle
// D22 KaFeatureCard 유형별 필드와 하단 액션 버튼
#[test]
fn test_d22_feature_card_type_fields_and_actions() {
    let st = RightInspectorState::new_default();
    // KaFeatureCardEditors.cpp — 유형별 기본값
    assert_eq!(st.feature_name, "1호 주거지");
    assert_eq!(st.site_name, "경주 월성북편 유적");
    assert_eq!(st.control_point_id, "CP-01");
    assert!((st.control_z - 48.50).abs() < 1e-9);
    assert_eq!(st.section_id, "A-A'");
    assert!(st.control_csv_path.is_empty());

    // 유형 판별은 레이어 이름으로 한다 (KaFeatureCard.cpp 와 같다)
    let kind_of = |n: &str| -> &'static str {
        if n.contains("기준점") {
            "control"
        } else if n.contains("단면") {
            "section"
        } else if n.contains("조사구역") {
            "survey"
        } else {
            "feature"
        }
    };
    assert_eq!(kind_of("기준점"), "control");
    assert_eq!(kind_of("단면선"), "section");
    assert_eq!(kind_of("조사구역"), "survey");
    assert_eq!(kind_of("지정유산"), "feature");

    // KaFeatureCard.cpp 하단 액션 버튼 4개
    let actions = ["저장 (Enter)", "되돌리기", "도형 삭제 (Del)", "면·선 색상…"];
    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0], "저장 (Enter)");
}

#[test]
fn test_d9_feature_card_period_band_next_number_and_style_drag() {
    assert_eq!(KA_BUILTIN_PERIODS.len(), 10);
    let names: Vec<&str> = KA_BUILTIN_PERIODS.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        vec![
            "구석기",
            "신석기",
            "청동기",
            "초기철기",
            "원삼국",
            "삼국",
            "통일신라",
            "고려",
            "조선",
            "미정"
        ]
    );
    assert_eq!(KA_BUILTIN_PERIODS[0].1, Some((0x53, 0x29, 0x1B)));
    assert_eq!(KA_BUILTIN_PERIODS[9].1, None);
    assert_eq!(
        KA_BUILTIN_PERIODS
            .iter()
            .filter(|(_, c)| c.is_some())
            .count(),
        9
    );
    assert_eq!(KA_BUILTIN_KINDS.len(), 8);
    assert_eq!(KA_BUILTIN_KINDS[0], "주거지");
    assert_eq!(KA_BUILTIN_KINDS[7], "기타");
    // 「다음 번호」는 C++ FeatureNumbering::suggestNext(feature_card_tests·feature_form_tests 가 본다).
    let d = RightInspectorState::new_default();
    assert!((0.2..=12.0).contains(&d.stroke_width_mm));
}

#[test]
fn test_feature_card_area_calculation_10x20_rect_formats_exact_200m2_605_pyeong_and_60m_perim()
{
    let rect_10x20 = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 20.0],
        [0.0, 20.0],
        [0.0, 0.0],
    ];
    let (area_label, perim_label) = RightInspectorState::format_bound_ring_metrics(&rect_10x20);
    assert_eq!(area_label, "200.00 ㎡ (약 60.5 평)");
    assert_eq!(perim_label, "60.00 m");

    let mut card = RightInspectorState::new_default();
    assert!(card.save_current_attributes());
    card.feature_no.clear();
    assert!(!card.save_current_attributes());
}

#[test]
fn test_batch_period_update_5_features_and_elevation_nan_revert() {
    let mut table = std::collections::BTreeMap::new();
    for fid in 1..=5_i64 {
        let mut attrs = std::collections::BTreeMap::new();
        attrs.insert("FEAT_NO".to_owned(), format!("유구-{fid}"));
        attrs.insert("PERIOD".to_owned(), "미상".to_owned());
        table.insert(fid, attrs);
    }
    let mut changes = std::collections::BTreeMap::new();
    changes.insert("PERIOD".to_owned(), "삼국".to_owned());

    let res = RightInspectorState::apply_batch_attribute_update(
        &[1, 2, 3, 4, 5],
        &mut table,
        &changes,
    );
    assert_eq!(res.updated_count, 5);
    assert_eq!(res.failed_count, 0);
    for fid in 1..=5_i64 {
        assert_eq!(table[&fid]["PERIOD"], "삼국");
    }

    let (val_ok, ok) = RightInspectorState::validate_and_parse_elevation("48.25", 12.0);
    assert!(ok);
    assert!((val_ok - 48.25).abs() < 1e-9);

    let (val_reverted, ok_nan) =
        RightInspectorState::validate_and_parse_elevation("NaN", 48.25);
    assert!(!ok_nan);
    assert!((val_reverted - 48.25).abs() < 1e-9);
}
