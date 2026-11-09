//! 속성 고유성·시대 도메인 검증.
//!
//! `checklist_engine` 에서 분리했다.

use crate::check_rules::ChecklistSeverity;

/// C++ `HGIS_VALID_PERIOD_DOMAINS` — 국가유산청 고고학 11대 표준 시대 코드 도메인
pub const HGIS_VALID_PERIOD_DOMAINS: [&str; 11] = [
    "구석기",
    "신석기",
    "청동기",
    "초기철기",
    "원삼국",
    "삼국",
    "통일신라",
    "고려",
    "조선",
    "근현대",
    "미상",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValidationIssue {
    pub fid: i64,
    pub field_name: String,
    pub severity: ChecklistSeverity,
    pub message_ko: String,
}

/// C++ `KaAttributeValidator::validateUniqueIds` — 동일 레이어 내 유구번호/조사구역번호 중복 검출
#[must_use]
pub fn validate_unique_feature_ids(features: &[(i64, &str)]) -> Vec<AttributeValidationIssue> {
    let mut seen: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut issues = Vec::new();
    for &(fid, raw_id) in features {
        let trimmed = raw_id.trim();
        if trimmed.is_empty() {
            issues.push(AttributeValidationIssue {
                fid,
                field_name: "FEAT_NO".to_owned(),
                severity: ChecklistSeverity::Error,
                message_ko: "필수 속성 'FEAT_NO'이(가) 누락되었습니다.".to_owned(),
            });
            continue;
        }
        if let Some(&first_fid) = seen.get(trimmed) {
            issues.push(AttributeValidationIssue {
                fid,
                field_name: "FEAT_NO".to_owned(),
                severity: ChecklistSeverity::Error,
                message_ko: format!(
                    "동일한 유구번호 '{trimmed}'이(가) 이미 존재합니다 (FID: {first_fid}와 중복)."
                ),
            });
        } else {
            seen.insert(trimmed.to_owned(), fid);
        }
    }
    issues
}

/// C++ `KaAttributeValidator` — `PERIOD`(시대) 필드가 11대 표준 코드 도메인(`HGIS_VALID_PERIOD_DOMAINS`)에 속하는지 검증
#[must_use]
pub fn validate_period_domain(fid: i64, period_value: &str) -> Option<AttributeValidationIssue> {
    let trimmed = period_value.trim();
    if HGIS_VALID_PERIOD_DOMAINS.contains(&trimmed) {
        None
    } else {
        Some(AttributeValidationIssue {
            fid,
            field_name: "PERIOD".to_owned(),
            severity: ChecklistSeverity::Warning,
            message_ko: format!(
                "시대 속성 '{trimmed}'은(는) 국가유산청 표준 11대 시대 도메인에 속하지 않습니다."
            ),
        })
    }
}
