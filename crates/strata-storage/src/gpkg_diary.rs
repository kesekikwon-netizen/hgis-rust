//! 조사 일지와 진척률 계산.
//!
//! `gpkg` 에서 분리했다.

use std::path::Path;

use rusqlite::Connection;

use strata_types::error::{StrataError, StrataResult};

/// SQLite 현장 야장 테이블명 (C++ `HGIS_DIARY_TABLE_NAME`)
pub const HGIS_DIARY_TABLE_NAME: &str = "ka_field_diary";

/// 최대 진척률 상한 (%, C++ `HGIS_PROGRESS_CLAMP_MAX`)
pub const HGIS_PROGRESS_CLAMP_MAX: f64 = 100.0;

/// 현장 발굴조사 일지(야장) 엔트리 (C++ `FieldDiaryEntry`)
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiaryEntry {
    pub diary_id: i64,
    pub work_date: String,
    pub weather: String,
    pub worker_count: i32,
    pub active_trenches: String,
    pub discovered_features: String,
    pub completed_area_m2: f64,
    pub notes: String,
}

/// 발굴조사 진척률 및 누적 투입 공수 지표 (C++ `ProgressMetrics`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressMetrics {
    pub total_survey_area_m2: f64,
    pub excavated_area_m2: f64,
    pub progress_percentage: f64,
    pub total_man_days: i32,
}

/// 전체 계획 조사 면적 대비 최신 완료 면적 진척률(`0.0..=100.0%`) 및 누적 공수(Man-Days) 산출
/// (C++ `KaExcavationProgressTracker::calculateProgressMetrics` 1:1 대응)
#[must_use]
pub fn calculate_excavation_progress_metrics(
    total_survey_area_m2: f64,
    diary_history: &[FieldDiaryEntry],
) -> ProgressMetrics {
    let total_man_days: i32 = diary_history.iter().map(|d| d.worker_count.max(0)).sum();
    let excavated_area_m2 = diary_history
        .last()
        .map_or(0.0, |d| d.completed_area_m2.max(0.0));
    let progress_percentage = if total_survey_area_m2 <= 0.0 || !total_survey_area_m2.is_finite() {
        0.0
    } else {
        ((excavated_area_m2 / total_survey_area_m2) * 100.0).clamp(0.0, HGIS_PROGRESS_CLAMP_MAX)
    };
    ProgressMetrics {
        total_survey_area_m2: total_survey_area_m2.max(0.0),
        excavated_area_m2,
        progress_percentage,
        total_man_days,
    }
}

/// SQLite/GeoPackage 내부 `ka_field_diary` 테이블에 현장 야장 레코드 트랜잭션 기록
/// (C++ `KaFieldDiaryManager::recordDiaryEntry` 1:1 대응)
pub fn record_diary_entry_sqlite(db_path: &Path, entry: &FieldDiaryEntry) -> StrataResult<i64> {
    let mut conn = Connection::open(db_path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let tx = conn
        .transaction()
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS ka_field_diary (
            diary_id INTEGER PRIMARY KEY AUTOINCREMENT,
            work_date TEXT NOT NULL,
            weather TEXT NOT NULL,
            worker_count INTEGER NOT NULL,
            active_trenches TEXT NOT NULL,
            discovered_features TEXT NOT NULL,
            completed_area_m2 REAL NOT NULL,
            notes TEXT NOT NULL
        );",
    )
    .map_err(|e| StrataError::Storage(e.to_string()))?;
    tx.execute(
        "INSERT INTO ka_field_diary (
            work_date, weather, worker_count, active_trenches,
            discovered_features, completed_area_m2, notes
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            entry.work_date,
            entry.weather,
            entry.worker_count,
            entry.active_trenches,
            entry.discovered_features,
            entry.completed_area_m2,
            entry.notes
        ],
    )
    .map_err(|e| StrataError::Storage(e.to_string()))?;
    let row_id = tx.last_insert_rowid();
    tx.commit()
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    Ok(row_id)
}

/// SQLite `ka_field_diary` 테이블에서 현장 야장 이력 조회
pub fn query_diary_entries_sqlite(db_path: &Path) -> StrataResult<Vec<FieldDiaryEntry>> {
    let conn = Connection::open(db_path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT diary_id, work_date, weather, worker_count, active_trenches,
                    discovered_features, completed_area_m2, notes
             FROM ka_field_diary ORDER BY diary_id ASC",
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(FieldDiaryEntry {
                diary_id: row.get(0)?,
                work_date: row.get(1)?,
                weather: row.get(2)?,
                worker_count: row.get(3)?,
                active_trenches: row.get(4)?,
                discovered_features: row.get(5)?,
                completed_area_m2: row.get(6)?,
                notes: row.get(7)?,
            })
        })
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| StrataError::Storage(e.to_string()))?);
    }
    Ok(out)
}
