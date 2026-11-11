//! ZIP 풀기 — 받은 연속지적도·수치지형도 꾸러미를 폴더에 푼다.
//!
//! C++ 은 `CadastralImport`·`TopographicImport` 가 GDAL 의 `/vsizip/` 로 ZIP 안의 SHP 를 읽는다.
//! 여기서는 폴더에 풀어 놓고 SHP 읽기에 넘긴다. 국내 자료의 ZIP 은 파일 이름이 CP949 인
//! 경우가 많아, UTF-8 이 아니면 CP949 로 푼다. ZIP 밖으로 나가는 경로(`..`·절대 경로)는 버린다.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use strata_types::error::{StrataError, StrataResult};

/// 한 파일이 이보다 크면 풀지 않는다(압축 폭탄 방지, 2 GB).
const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn entry_name(raw: &[u8]) -> String {
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_owned(),
        Err(_) => encoding_rs::EUC_KR.decode(raw).0.into_owned(),
    }
}

/// ZIP 안 이름을 `dest` 아래의 안전한 경로로 바꾼다. 밖으로 나가면 `None`.
fn safe_target(dest: &Path, name: &str) -> Option<PathBuf> {
    let rel = Path::new(&name.replace('\\', "/")).to_path_buf();
    let mut out = dest.to_path_buf();
    for part in rel.components() {
        match part {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (out != dest).then_some(out)
}

/// `zip_path` 를 `dest` 폴더에 풀고, 푼 파일 경로를 돌려준다.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> StrataResult<Vec<PathBuf>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| StrataError::Storage(format!("ZIP 을 열지 못했습니다: {e}")))?;
    std::fs::create_dir_all(dest)?;
    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| StrataError::Storage(format!("ZIP 항목을 읽지 못했습니다: {e}")))?;
        let name = entry_name(entry.name_raw());
        if name.ends_with('/') || entry.is_dir() {
            continue;
        }
        let Some(target) = safe_target(dest, &name) else {
            continue;
        };
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(StrataError::Storage(format!("ZIP 안의 「{name}」이 너무 큽니다")));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_ENTRY_BYTES)
            .read_to_end(&mut bytes)
            .map_err(|e| StrataError::Storage(format!("「{name}」을 풀지 못했습니다: {e}")))?;
        std::fs::write(&target, bytes)?;
        written.push(target);
    }
    Ok(written)
}

/// 푼 파일 가운데 `.shp` 만 고른다.
#[must_use]
pub fn shapefiles_in(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("shp"))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_zip_is_unpacked_and_paths_that_escape_the_folder_are_dropped() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let zip_path = dir.path().join("지적도.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path)?);
            let opt = zip::write::SimpleFileOptions::default();
            let fail = |e: zip::result::ZipError| StrataError::Storage(e.to_string());
            w.start_file("LSMD_CONT_LDREG_47130.shp", opt).map_err(fail)?;
            w.write_all(b"shp-bytes")?;
            w.start_file("sub/LSMD_CONT_LDREG_47130.dbf", opt).map_err(fail)?;
            w.write_all(b"dbf-bytes")?;
            w.start_file("../outside.txt", opt).map_err(fail)?;
            w.write_all(b"escape")?;
            w.finish().map_err(fail)?;
        }
        let dest = dir.path().join("풀기");
        let files = extract_zip(&zip_path, &dest)?;
        assert_eq!(files.len(), 2);
        assert_eq!(std::fs::read(dest.join("LSMD_CONT_LDREG_47130.shp"))?, b"shp-bytes");
        assert_eq!(std::fs::read(dest.join("sub").join("LSMD_CONT_LDREG_47130.dbf"))?, b"dbf-bytes");
        assert!(!dir.path().join("outside.txt").exists(), "폴더 밖으로 쓰지 않는다");
        assert_eq!(shapefiles_in(&files), vec![dest.join("LSMD_CONT_LDREG_47130.shp")]);
        Ok(())
    }

    #[test]
    fn a_file_that_is_not_a_zip_is_refused() {
        let dir = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
        let p = dir.path().join("x.zip");
        let _ = std::fs::write(&p, b"not a zip");
        assert!(extract_zip(&p, &dir.path().join("out")).is_err());
    }

    #[test]
    fn cp949_file_names_inside_a_zip_are_read_as_korean() {
        let (cp949, _, _) = encoding_rs::EUC_KR.encode("연속지적도.shp");
        assert_eq!(entry_name(&cp949), "연속지적도.shp");
        assert_eq!(entry_name("지적.shp".as_bytes()), "지적.shp");
    }
}
