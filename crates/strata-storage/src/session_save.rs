use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use sha2::{Digest, Sha256};
use std::path::Path;
use strata_types::error::{StrataError, StrataResult};

pub fn persist_workspace(ws: &mut SurveyWorkspace, path: &Path) -> StrataResult<()> {
    let mut gen_copy = ws.clone();
    gen_copy.migrate_generation_copy();
    let gen_tmp = path.with_extension("gpkg.gen-tmp");
    GpkgRepository::write_workspace_file(&gen_tmp, &gen_copy)?;
    let verified = GpkgRepository::open_read_only(&gen_tmp)?;
    if verified.schema_version != 3 {
        let _ = std::fs::remove_file(&gen_tmp);
        return Err(StrataError::Storage(
            "세대 사본 스키마 버전 검증에 실패했습니다".into(),
        ));
    }
    std::fs::rename(&gen_tmp, path)?;
    *ws = gen_copy;
    Ok(())
}

pub fn save_as_copy(ws: &mut SurveyWorkspace, orig: &Path, copy: &Path) -> StrataResult<()> {
    if orig == copy {
        return persist_workspace(ws, copy);
    }
    let mut copy_ws = ws.clone();
    copy_ws.migrate_generation_copy();
    let tmp_copy = copy.with_extension("gpkg.saveas-tmp");
    GpkgRepository::write_workspace_file(&tmp_copy, &copy_ws)?;
    let _verified = GpkgRepository::open_read_only(&tmp_copy)?;
    std::fs::rename(&tmp_copy, copy)?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpkg::GpkgRepository;
    use strata_types::crs::WorkCrs;
    use strata_types::schema::DomainLayerKey;
    use tempfile::TempDir;

    #[test]
    fn persist_workspace_after_layer_delete_never_locks_file_or_falls_back_to_dash_save(
    ) -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("경주시_성동동.gpkg");
        let mut session = GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;
        session.ensure_domain_layer(DomainLayerKey::SurveyArea);
        session.remove_domain_layer(DomainLayerKey::SurveyArea);
        persist_workspace(&mut session, &path)?;
        assert!(path.exists());
        assert!(!tmp.path().join("경주시_성동동-저장.gpkg").exists());
        Ok(())
    }

    #[test]
    fn save_as_copy_leaves_original_survey_fingerprint_untouched() -> StrataResult<()> {
        let tmp = TempDir::new()?;
        let orig = tmp.path().join("orig.gpkg");
        let copy = tmp.path().join("copy.gpkg");
        let mut session = GpkgRepository::create_new_survey(&orig, WorkCrs::Epsg5187)?;
        let hash_before = sha256_fingerprint(&orig)?;
        session.ensure_domain_layer(DomainLayerKey::FeaturePoly);
        save_as_copy(&mut session, &orig, &copy)?;
        let hash_after = sha256_fingerprint(&orig)?;
        assert!(!hash_before.is_empty());
        assert_eq!(hash_before, hash_after);
        assert!(copy.exists());
        Ok(())
    }
}
