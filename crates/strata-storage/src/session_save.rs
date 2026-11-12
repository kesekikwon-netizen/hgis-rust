//! 조사 파일 지문·세대 보관·원자적 바이트 저장. 조사 저장 자체는 `survey_gpkg_save` 하나로 한다
//! (예전 Rust 형식 persist_workspace·save_as_copy 는 지시 2C 에서 그 길로 모았다).

use sha2::{Digest, Sha256};
use std::path::Path;
use strata_types::error::StrataResult;

pub fn sha256_fingerprint(path: &Path) -> StrataResult<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    Ok(hex)
}

/// C++ `SessionSave::createGenerationSnapshot(gpkgPath, maxGenerations = 5) -> bool` — `.v1` ~ `.vN` 롤링 시프트 보관 및 초과 세대 자동 정리 1:1
pub fn create_generation_snapshot(gpkg_path: &Path, max_generations: usize) -> bool {
    if !gpkg_path.exists() || max_generations == 0 {
        return false;
    }
    let base = gpkg_path.to_string_lossy();
    let stale_over = std::path::PathBuf::from(format!("{base}.v{}", max_generations + 1));
    let _ = std::fs::remove_file(&stale_over);

    for gen_idx in (1..max_generations).rev() {
        let src = std::path::PathBuf::from(format!("{base}.v{gen_idx}"));
        let dst = std::path::PathBuf::from(format!("{base}.v{}", gen_idx + 1));
        if src.exists() {
            let _ = std::fs::remove_file(&dst);
            if std::fs::rename(&src, &dst).is_err() {
                return false;
            }
        }
    }

    let tmp_v1 = std::path::PathBuf::from(format!("{base}.v1.tmp"));
    let v1 = std::path::PathBuf::from(format!("{base}.v1"));
    for attempt in 0..5 {
        if std::fs::copy(gpkg_path, &tmp_v1).is_ok() {
            let _ = std::fs::remove_file(&v1);
            if std::fs::rename(&tmp_v1, &v1).is_ok() {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10 * (attempt + 1)));
    }
    let _ = std::fs::remove_file(&tmp_v1);
    false
}

/// 백그라운드 자동 저장 주기 (3분 = 180,000ms, C++ `HGIS_AUTOSAVE_INTERVAL_MS`)
pub const HGIS_AUTOSAVE_INTERVAL_MS: u64 = 180_000;
/// 위생 정리 대상 임시 파일 기본 경과 시간 (24시간, C++ `HGIS_STALE_TEMP_FILE_HOURS`)
pub const HGIS_STALE_TEMP_FILE_HOURS: u64 = 24;

/// 파일 무결성 지문 레코드 (C++ `FileFingerprint`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFingerprint {
    pub sha256_hex: String,
    pub file_size_bytes: u64,
}

/// 비정상 종료 복구 상태 (C++ `RecoveryStatus`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    NoRecoveryNeeded,
    CrashDetectedRecoverable,
    JournalCorrupted,
}

/// 임시 파일 쓰기 + `sync_all()`(`fsync`/`FlushFileBuffers`) + 원자적 `rename` 교체 및 SHA-256 지문 반환
/// (C++ `SurveyDurability::safeAtomicSave` & `SurveyFileFingerprint::computeFingerprint` 1:1 대응)
pub fn safe_atomic_save_bytes(target_path: &Path, bytes: &[u8]) -> StrataResult<FileFingerprint> {
    use std::io::Write;
    let tmp_path = target_path.with_extension("hgis.atomic-tmp");
    {
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, target_path)?;
    let sha256_hex = sha256_fingerprint(target_path)?;
    Ok(FileFingerprint {
        sha256_hex,
        file_size_bytes: bytes.len() as u64,
    })
}

/// 파일의 현재 SHA-256 지문 및 바이트 크기가 기대 지문(`expected`)과 일치하는지 검증
/// (C++ `SurveyFileFingerprint::verifyIntegrity` 1:1 대응)
#[must_use]
pub fn verify_file_fingerprint(path: &Path, expected: &FileFingerprint) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if meta.len() != expected.file_size_bytes {
        return false;
    }
    sha256_fingerprint(path).is_ok_and(|hex| hex == expected.sha256_hex)
}

/// 이전 편집 서명과 현재 편집 서명을 비교하여 복구 스냅샷 갱신 필요 여부 판정
/// (C++ `SurveyRecovery::snapshotNeeded` 1:1 대응)
#[must_use]
pub fn recovery_snapshot_needed(last_signature: &str, current_signature: &str) -> bool {
    last_signature.is_empty() || current_signature.is_empty() || last_signature != current_signature
}

/// 프로젝트 폴더 내 지정 경과 시간(`max_age_hours`) 이상 방치된 잔류 임시 파일(`.tmp`, `.journal`, `.gen-tmp`, `.saveas-tmp`) 정리
/// (C++ `SurveyFileHygiene::sweepStaleTempFiles` 1:1 대응)
#[must_use]
pub fn sweep_stale_temp_files(project_dir: &Path, max_age_hours: u64) -> usize {
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return 0;
    };
    let threshold = std::time::Duration::from_secs(max_age_hours.saturating_mul(3600));
    let now = std::time::SystemTime::now();
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let is_temp = name.ends_with(".tmp")
            || name.ends_with(".journal")
            || name.ends_with(".gen-tmp")
            || name.ends_with(".saveas-tmp");
        if !is_temp {
            continue;
        }
        let old_enough = if max_age_hours == 0 {
            true
        } else {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|mtime| now.duration_since(mtime).ok())
                .is_some_and(|age| age >= threshold)
        };
        if old_enough && std::fs::remove_file(&p).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpkg::GpkgRepository;
    use strata_types::crs::WorkCrs;
    use strata_types::schema::DomainLayerKey;
    use tempfile::TempDir;

    /// 조사 저장(하나로 모은 길)은 잠기지 않은 파일을 제자리에 저장하고 「-저장」 옆 파일로 빠지지 않는다.
    #[test]
    fn survey_save_never_locks_file_or_falls_back_to_dash_save() -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("경주시_성동동.gpkg");
        GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;
        let tables = crate::survey_gpkg_read::read_survey_tables(&path)?;
        let written = crate::survey_gpkg_save::save_survey(&path, &tables)
            .map_err(|e| strata_types::error::StrataError::Storage(format!("{e:?}")))?;
        assert_eq!(written, std::path::absolute(&path)?);
        assert!(!tmp.path().join("경주시_성동동-저장.gpkg").exists());
        Ok(())
    }

    /// 다른 이름으로 저장(사본 만들고 그 사본에 저장)은 원본 지문을 바꾸지 않는다.
    #[test]
    fn save_as_copy_leaves_original_survey_fingerprint_untouched() -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let orig = tmp.path().join("orig.gpkg");
        let copy = tmp.path().join("copy.gpkg");
        GpkgRepository::create_new_survey(&orig, WorkCrs::Epsg5187)?;
        let hash_before = sha256_fingerprint(&orig)?;
        let mut tables = crate::survey_gpkg_read::read_survey_tables(&orig)?;
        let poly = tables
            .iter_mut()
            .find(|t| t.layer.key == Some(DomainLayerKey::FeaturePoly))
            .ok_or_else(|| strata_types::error::StrataError::Storage("feature_poly".into()))?;
        poly.layer.add_unsaved_feature("POLYGON ((0 0,1 0,1 1,0 0))".to_owned(), Default::default());
        crate::survey_gpkg_save::copy_survey_snapshot(&orig, &copy).map_err(strata_types::error::StrataError::Storage)?;
        crate::survey_gpkg_save::save_survey(&copy, &tables)
            .map_err(|e| strata_types::error::StrataError::Storage(format!("{e:?}")))?;
        let hash_after = sha256_fingerprint(&orig)?;
        assert!(!hash_before.is_empty());
        assert_eq!(hash_before, hash_after);
        assert!(copy.exists());
        Ok(())
    }

    #[test]
    fn test_session_save_rolling_max_generations_3_keeps_v1_to_v3_and_never_creates_v4(
    ) -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("rolling_project.gpkg");
        let _session = GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5186)?;
        for _ in 0..5 {
            assert!(create_generation_snapshot(&path, 3));
        }
        let base = path.to_string_lossy();
        assert!(std::path::Path::new(&format!("{base}.v1")).exists());
        assert!(std::path::Path::new(&format!("{base}.v2")).exists());
        assert!(std::path::Path::new(&format!("{base}.v3")).exists());
        assert!(!std::path::Path::new(&format!("{base}.v4")).exists());
        Ok(())
    }

    #[test]
    fn test_atomic_save_success_and_fingerprint_and_stale_temp_hygiene() -> StrataResult<()> {
        // TestSurveyDurability::testAtomicSaveSuccessAndBackup & SurveyFileFingerprint
        let tmp = TempDir::new()?;
        let target = tmp.path().join("project.hgis");
        std::fs::write(&target, b"initial-v1")?;

        let payload = b"updated-survey-v2-payload";
        let fp = safe_atomic_save_bytes(&target, payload)?;
        assert_eq!(fp.file_size_bytes, payload.len() as u64);
        assert!(verify_file_fingerprint(&target, &fp));

        // SurveyRecovery::snapshotNeeded
        assert!(recovery_snapshot_needed("", &fp.sha256_hex));
        assert!(!recovery_snapshot_needed(&fp.sha256_hex, &fp.sha256_hex));

        // SurveyFileHygiene::sweepStaleTempFiles
        let stale_tmp = tmp.path().join("project.hgis.tmp");
        std::fs::write(&stale_tmp, b"orphan")?;
        let swept = sweep_stale_temp_files(tmp.path(), 0);
        assert_eq!(swept, 1);
        assert!(!stale_tmp.exists());
        Ok(())
    }
}
