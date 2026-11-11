//! 최근 조사 목록 — 실제로 연 조사 파일을 기억하고 홈 화면 표에 보여 준다.
//!
//! C++ `RecentSurveys.cpp` 는 QSettings 에 경로와 연 시각을 남기고, `KaHomeRecentCard` 가
//! 파일이 아직 있는지·조사구역과 유구가 몇 개인지를 표로 보인다. 여기서는 설정 폴더의
//! `recent-surveys.json` 에 남긴다. 시험 중에는 디스크를 읽지도 쓰지도 않는다
//! (사용자의 실제 목록을 건드리지 않는다).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use strata_storage::survey_gpkg_read::read_survey_gpkg;
use strata_types::schema::DomainLayerKey;

/// C++ `RecentSurveys` 의 상한과 같다.
pub const MAX_RECENT_SURVEYS: usize = 10;

/// 저장되는 한 줄: 경로와 마지막으로 연 시각(유닉스 초).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentSurvey {
    pub path: String,
    pub opened_unix: u64,
}

/// 홈 표의 한 줄(화면에 보일 글자).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentRow {
    pub path: PathBuf,
    pub name: String,
    pub folder: String,
    /// 파일이 지금도 있는지.
    pub exists: bool,
    pub status: String,
    pub areas: String,
    pub features: String,
    pub opened: String,
}

/// 설정 폴더. `KA_HGIS_SETTINGS_DIR`(C++ 의 QA 용 변수와 같은 이름)이 있으면 그곳,
/// 없으면 `%LOCALAPPDATA%\ka-hgis`.
fn settings_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("KA_HGIS_SETTINGS_DIR") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|d| PathBuf::from(d).join("ka-hgis"))
}

fn store_path() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("recent-surveys.json"))
}

/// 저장된 목록을 읽는다. 파일이 없거나 깨졌으면 빈 목록.
#[must_use]
pub fn load_recent_surveys() -> Vec<RecentSurvey> {
    if cfg!(test) {
        return Vec::new();
    }
    store_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_recent_surveys(list: &[RecentSurvey]) {
    if cfg!(test) {
        return;
    }
    let Some(path) = store_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(list) {
        let _ = std::fs::write(path, json);
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 방금 연(또는 저장한) 조사를 목록 맨 위에 올리고 디스크에 남긴다.
pub fn remember_survey(list: &mut Vec<RecentSurvey>, path: &Path) {
    remember_survey_at(list, path, now_unix());
    save_recent_surveys(list);
}

/// 시각을 받아 목록만 고친다(시험에서 쓴다).
pub fn remember_survey_at(list: &mut Vec<RecentSurvey>, path: &Path, opened_unix: u64) {
    let text = path.to_string_lossy().into_owned();
    list.retain(|r| !r.path.eq_ignore_ascii_case(&text));
    list.insert(
        0,
        RecentSurvey {
            path: text,
            opened_unix,
        },
    );
    list.truncate(MAX_RECENT_SURVEYS);
}

/// 유닉스 초를 한국 시각 `YYYY-MM-DD HH:MM` 으로 쓴다(한국은 서머타임이 없어 +9시간 고정).
#[must_use]
pub fn format_kst(unix: u64) -> String {
    let secs = unix + 9 * 3600;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // 달력 계산(Howard Hinnant 의 civil_from_days).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
}

/// 목록을 홈 표의 줄로 바꾼다. 파일을 실제로 열어 조사구역·유구 수를 센다.
#[must_use]
pub fn build_recent_rows(list: &[RecentSurvey]) -> Vec<RecentRow> {
    list.iter()
        .map(|r| {
            let path = PathBuf::from(&r.path);
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("조사")
                .to_owned();
            let folder = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .map_or_else(String::new, |s| format!("… › {s}"));
            let exists = path.is_file();
            let doc = if exists { read_survey_gpkg(&path).ok() } else { None };
            let (status, areas, features) = match (&doc, exists) {
                (Some(d), _) => {
                    let n_feat = d.layer(DomainLayerKey::FeaturePoly).features.len()
                        + d.layer(DomainLayerKey::FeatureLine).features.len();
                    (
                        "저장됨".to_owned(),
                        d.layer(DomainLayerKey::SurveyArea).features.len().to_string(),
                        n_feat.to_string(),
                    )
                }
                (None, true) => ("⊗ 열 수 없음".to_owned(), "—".to_owned(), "—".to_owned()),
                (None, false) => ("⊗ 원본 없음".to_owned(), "—".to_owned(), "—".to_owned()),
            };
            RecentRow {
                path,
                name,
                folder,
                exists,
                status,
                areas,
                features,
                opened: if exists {
                    format_kst(r.opened_unix)
                } else {
                    "찾을 수 없음".to_owned()
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembering_moves_a_survey_to_the_top_without_duplicates_and_caps_at_ten() {
        let mut list = Vec::new();
        for i in 0..12 {
            remember_survey_at(&mut list, Path::new(&format!("C:/조사/{i}.gpkg")), i);
        }
        assert_eq!(list.len(), MAX_RECENT_SURVEYS);
        assert_eq!(list[0].path, "C:/조사/11.gpkg");
        remember_survey_at(&mut list, Path::new("c:/조사/5.GPKG"), 99);
        assert_eq!(list.len(), MAX_RECENT_SURVEYS);
        assert_eq!(list[0].opened_unix, 99);
        assert_eq!(list.iter().filter(|r| r.path.to_lowercase().ends_with("/5.gpkg")).count(), 1);
    }

    #[test]
    fn kst_time_is_written_as_a_plain_date_and_minute() {
        assert_eq!(format_kst(0), "1970-01-01 09:00");
        // 2026-11-11 06:20:00 UTC = 한국 15:20
        assert_eq!(format_kst(1_794_378_000), "2026-11-11 15:20");
        // 윤년 2월 29일
        assert_eq!(format_kst(1_709_164_800), "2024-02-29 09:00");
    }

    #[test]
    fn rows_show_real_counts_for_a_real_file_and_say_so_when_the_file_is_gone() {
        let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
        let list = vec![
            RecentSurvey { path: demo.to_string_lossy().into_owned(), opened_unix: 0 },
            RecentSurvey { path: "Z:/없는폴더/사라진조사.gpkg".to_owned(), opened_unix: 0 },
        ];
        let rows = build_recent_rows(&list);
        assert_eq!(rows[0].name, "demo");
        assert_eq!(rows[0].folder, "… › demo_survey");
        assert_eq!((rows[0].status.as_str(), rows[0].areas.as_str(), rows[0].features.as_str()), ("저장됨", "1", "3"));
        assert!(!rows[1].exists);
        assert_eq!(rows[1].status, "⊗ 원본 없음");
        assert_eq!(rows[1].opened, "찾을 수 없음");
    }
}
