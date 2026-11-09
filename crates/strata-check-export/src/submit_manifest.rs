//! 제출 결과·매니페스트 타입과 SHA-256 스트리밍 계산.
//!
//! `submit_package` 에서 분리했다.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use strata_types::crs::SubmitCrs;
use strata_types::error::StrataResult;

/// C++ `SubmitPackageExporter::exportSubmissionZip` 반환 상태 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportResult {
    Success,
    HasBlockingErrors,
    IoError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionManifest {
    pub crs: SubmitCrs,
    pub files_sha256: Vec<(String, String)>,
    pub readme_path: PathBuf,
}

pub const HASH_CHUNK_SIZE: usize = 65536;

/// C++ `ManifestBuilder::generateSha256Manifest` — 64KB 청크 단위 스트리밍 SHA-256 체크섬 계산
pub fn stream_file_sha256_hex(path: &Path) -> StrataResult<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; HASH_CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
