//! 조사 저장 — C++ `SurveyStorage::persistWorkspace`(`SurveyStorage.cpp:788-1010`)의 조사 데이터 쪽과
//! `publishSurveyGeneration`(:510-519)·`copySurvey`(:456-508)·`finishStagedSurvey`(:420-453).
//!
//! 조사 폴더 안 `.ka-survey-gen-XXXXXX\survey.gpkg` 에 원본의 일관된 사본(VACUUM INTO, WAL 포함)을
//! 만들고, 사본만 이전하고, 편집을 쓰고, 확인한 뒤에야 원본과 바꾼다. 어디서 실패해도 원본 바이트는
//! 그대로다. 원본을 끝내 바꿀 수 없으면(잠김·읽기 전용) 옆 「<이름>-저장.gpkg」에 둔다.
//! 작업공간(.qgz·qgis_projects 쓰기)과 복구 사본은 아직 없다.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::survey_gpkg_apply::apply_table_edits;
use crate::survey_gpkg_create::make_temp_dir;
use crate::survey_gpkg_migrate::migrate_generation_copy;
use crate::survey_gpkg_read::SurveyFileTable;

/// 저장 실패. `Layer` 는 그 표의 편집을 세대 파일에 쓰지 못한 것(앱이 한국어 레이어 이름으로 알린다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveFailure {
    Layer { table: String, detail: String },
    Message(String),
}

/// C++ copySurvey 의 사본 — 원본을 읽기 전용으로 열어 `VACUUM INTO` 한다(WAL 의 행까지, 원본은
/// 바꾸지 않는다). `dst` 가 이미 있으면(비어 있지 않으면) SQLite 가 거절한다.
pub fn copy_survey_snapshot(src: &Path, dst: &Path) -> Result<(), String> {
    let fail = |e: rusqlite::Error| format!("조사 파일 사본을 만들지 못했습니다: {e}");
    let conn = Connection::open_with_flags(src, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .map_err(fail)?;
    conn.execute("VACUUM INTO ?1", [dst.to_string_lossy()]).map_err(fail)?;
    Ok(())
}

/// 지금 메모리의 조사 표를 `path` 에 저장하고, 실제로 쓴 파일 경로를 돌려준다(원본이 잠겨 있었으면
/// 옆 「-저장.gpkg」).
pub fn save_survey(path: &Path, tables: &[SurveyFileTable]) -> Result<PathBuf, SaveFailure> {
    let msg = |s: &str| SaveFailure::Message(s.to_owned());
    let target = std::path::absolute(path).map_err(|e| SaveFailure::Message(e.to_string()))?;
    let dir = target.parent().ok_or_else(|| msg("저장 경로가 없습니다."))?;
    let gen_dir = make_temp_dir(dir, ".ka-survey-gen-").ok_or_else(|| msg("다음 세대 조사 파일을 만들 공간이 없습니다."))?;
    let generation = gen_dir.join("survey.gpkg");
    let built = copy_survey_snapshot(&target, &generation)
        .map_err(SaveFailure::Message)
        .and_then(|()| build_generation(&generation, tables));
    let result = match built {
        Ok(()) => publish(&generation, &target),
        Err(e) => Err((e, false)),
    };
    match result {
        Ok(written) => {
            let _ = std::fs::remove_dir_all(&gen_dir);
            Ok(written)
        }
        // keep = 교체도 옆 파일도 못 해 새 세대를 안내한 자리에 남긴다(C++ :449-452).
        Err((e, keep)) => {
            if !keep {
                let _ = std::fs::remove_dir_all(&gen_dir);
            }
            Err(e)
        }
    }
}

/// 세대 사본을 이전하고(실패는 적어 두고 계속) 편집을 쓰고 확인한다. 연결은 끝날 때 닫는다.
fn build_generation(generation: &Path, tables: &[SurveyFileTable]) -> Result<(), SaveFailure> {
    let db = |e: rusqlite::Error| SaveFailure::Message(format!("다음 세대 조사 파일을 만들지 못했습니다: {e}"));
    let conn = Connection::open(generation).map_err(db)?;
    register_gpkg_sql_functions(&conn).map_err(db)?;
    if let Err(e) = migrate_generation_copy(&conn) {
        eprintln!("[save] 기록 항목 추가 건너뜀: {e}");
    }
    let tx = conn.unchecked_transaction().map_err(db)?;
    let mut expected = Vec::with_capacity(tables.len());
    for t in tables {
        let n = apply_table_edits(&tx, t)
            .map_err(|detail| SaveFailure::Layer { table: t.table.clone(), detail })?;
        expected.push((t.table.as_str(), n));
    }
    tx.commit().map_err(db)?;
    // C++ publishSurveyGeneration -> validateForOpen(:511). 도형 수는 메모리와 같아야 한다.
    let ok: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(db)?;
    if ok != "ok" {
        return Err(SaveFailure::Message(
            "조사 파일의 무결성을 확인하지 못했습니다. 현재 작업은 유지됩니다.".to_owned(),
        ));
    }
    for (table, n) in expected {
        let count: i64 = conn
            .query_row(&format!("SELECT count(*) FROM \"{}\"", table.replace('"', "\"\"")), [], |r| r.get(0))
            .map_err(db)?;
        if usize::try_from(count).ok() != Some(n) {
            return Err(SaveFailure::Message(
                "저장할 조사 파일의 도형 수가 편집과 맞지 않아 저장을 멈췄습니다. 다른 곳에서 이 조사 파일을 고쳤을 수 있습니다. 원본 조사 파일은 그대로입니다."
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

/// C++ `SurveyDurability.cpp:64-68` — 백신·색인·다른 앱이 잠깐 잡고 있을 때의 오류.
fn is_transient_lock(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(5 | 32 | 33))
}

/// C++ dropIdleJournals — 아무도 쓰지 않는 -wal·-shm·-journal 만 지운다(쓰는 중이면 지우기가 실패한다).
fn drop_idle_journals(target: &Path) {
    for ext in ["-wal", "-shm", "-journal"] {
        let mut p = target.as_os_str().to_owned();
        p.push(ext);
        let _ = std::fs::remove_file(PathBuf::from(p));
    }
}

/// C++ siblingSurveyPath(:409-418) — 「<이름>-저장.gpkg」, 있으면 「-저장2」, 「-저장3」 ….
fn sibling_survey_path(target: &Path) -> PathBuf {
    let stem = target.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = target.extension().map_or_else(|| "gpkg".to_owned(), |e| e.to_string_lossy().into_owned());
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let mut path = dir.join(format!("{stem}-저장.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-저장{n}.{ext}"));
        n += 1;
    }
    path
}

/// 검증된 세대로 원본을 바꾼다. 잠금이면 C++ 처럼 40번(100 ms×10, 이후 300 ms) 다시 하고, 끝내
/// 안 되면 옆 「-저장.gpkg」. 그것도 안 되면 세대 파일을 남기고 알린다(keep = true).
fn publish(generation: &Path, target: &Path) -> Result<PathBuf, (SaveFailure, bool)> {
    let _ = std::fs::OpenOptions::new().write(true).open(generation).and_then(|f| f.sync_all());
    drop_idle_journals(target);
    let mut last = None;
    for attempt in 0..40_u64 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(if attempt < 10 { 100 } else { 300 }));
        }
        match std::fs::rename(generation, target) {
            Ok(()) => {
                drop_idle_journals(target);
                return Ok(target.to_path_buf());
            }
            Err(e) => {
                let transient = is_transient_lock(&e);
                last = Some(e);
                if !transient {
                    break;
                }
            }
        }
    }
    eprintln!("[save] 이름 교체 거부 — 원본은 두고 옆 파일에 저장: {last:?}");
    let alt = sibling_survey_path(target);
    let moved = std::fs::rename(generation, &alt).or_else(|_| std::fs::copy(generation, &alt).map(|_| ()));
    match moved {
        Ok(()) => Ok(alt),
        Err(e) => Err((
            SaveFailure::Message(format!(
                "저장 파일을 교체하지 못했습니다: {e} · 원본 조사 파일은 그대로이고, 새로 만든 조사 파일은 {} 에 남겨 두었습니다.",
                generation.display()
            )),
            true,
        )),
    }
}
