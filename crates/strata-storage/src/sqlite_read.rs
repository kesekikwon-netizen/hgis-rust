//! Sidecar-free read connection helper for survey SQLite/GeoPackage files (`ORDER.yaml` ST1).
//!
//! Chooses the cheapest safe strategy per file state so reading never creates, modifies, or removes
//! files in the survey's folder and works when write access to the folder is denied:
//! 1. Companion `<file>-wal` or `<file>-journal` exists: stages `<file>` and present companions
//!    (omitting `<file>-shm`) into a private temp directory and opens `SQLITE_OPEN_READ_WRITE` so
//!    SQLite replays `-wal` or hot `-journal` without `SQLITE_READONLY_RECOVERY`
//!    (<https://www.sqlite.org/rescode.html#readonly_recovery>,
//!    <https://www.sqlite.org/walformat.html>).
//! 2. WAL header (`header[18] == 2 || header[19] == 2`,
//!    <https://www.sqlite.org/fileformat2.html#read_and_write_version_numbers>) with no companion
//!    files: opens in place via `file:///...?immutable=1` (`SQLITE_OPEN_READ_ONLY |
//!    SQLITE_OPEN_URI`, <https://www.sqlite.org/uri.html#uriimmutable>,
//!    <https://www.sqlite.org/wal.html#read_only_databases>).
//! 3. Rollback-journal (or non-WAL) file with no companion files: opens in place with
//!    `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` (<https://www.sqlite.org/c3ref/open.html>).

use std::fmt::{self, Write as _};
use std::fs::File;
use std::io::{self, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

/// Korean message returned when temporary staging runs out of disk space (`ORDER.yaml` ST1).
pub const TEMP_SPACE_FULL_KO: &str =
    "임시 폴더가 있는 드라이브의 남은 공간이 부족해 조사 파일을 읽을 수 없습니다. 남은 공간을 확보한 뒤 다시 시도해 주세요.";

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Guard holding an open read connection and any private temporary staging directory.
/// `conn` is declared before `_staging` so SQLite closes handles before `TempDir` deletes files.
pub struct SurveyReadConn {
    conn: Connection,
    _staging: Option<tempfile::TempDir>,
}

impl fmt::Debug for SurveyReadConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SurveyReadConn")
            .field("staged", &self._staging.is_some())
            .finish_non_exhaustive()
    }
}

impl Deref for SurveyReadConn {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl SurveyReadConn {
    #[cfg(test)]
    pub(crate) fn staging_dir_path(&self) -> Option<&Path> {
        self._staging.as_ref().map(tempfile::TempDir::path)
    }
}

/// Error returned when opening a survey SQLite/GeoPackage file for reading.
#[derive(Debug)]
pub enum SqliteReadError {
    NotARegularFile,
    TempSpaceFull,
    Io(io::Error),
    Sqlite(rusqlite::Error),
}

impl SqliteReadError {
    /// Short ASCII English reason for callers whose contract requires ASCII errors (`user_sheet`).
    #[must_use]
    pub fn english_reason(&self) -> String {
        match self {
            Self::NotARegularFile => "cannot open survey file: not a regular file".to_owned(),
            Self::TempSpaceFull => {
                "insufficient disk space in temporary folder to read survey file".to_owned()
            }
            Self::Io(err) => format!("cannot read survey file ({:?})", err.kind()),
            Self::Sqlite(rusqlite::Error::SqliteFailure(ffi_err, _)) => {
                format!("cannot open survey file: {ffi_err}")
            }
            Self::Sqlite(rusqlite::Error::InvalidPath(_)) => {
                "cannot open survey file: invalid path".to_owned()
            }
            Self::Sqlite(_) => "cannot open survey file: sqlite error".to_owned(),
        }
    }
}

impl fmt::Display for SqliteReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARegularFile => f.write_str("읽을 수 있는 일반 파일이 아닙니다."),
            Self::TempSpaceFull => f.write_str(TEMP_SPACE_FULL_KO),
            Self::Io(err) => write!(f, "{err}"),
            Self::Sqlite(err) => write!(f, "{err}"),
        }
    }
}

/// Opens `path` for reading without modifying `path` or leaving companion files beside it.
///
/// # Errors
/// Returns [`SqliteReadError`] when `path` cannot be opened or temporary staging fails.
pub fn open_survey_read_only(path: &Path) -> Result<SurveyReadConn, SqliteReadError> {
    open_survey_read_only_inner(path, None, None)
}

pub(crate) fn open_survey_read_only_inner(
    path: &Path,
    max_stage_bytes: Option<u64>,
    record_temp_dir: Option<&mut Option<PathBuf>>,
) -> Result<SurveyReadConn, SqliteReadError> {
    let mut src_file = File::open(path).map_err(SqliteReadError::Io)?;
    let src_meta = src_file.metadata().map_err(SqliteReadError::Io)?;
    if !src_meta.is_file() {
        return Err(SqliteReadError::NotARegularFile);
    }

    let wal_file = open_optional_companion(&sidecar_path(path, "-wal"))?;
    let journal_file = open_optional_companion(&sidecar_path(path, "-journal"))?;

    if wal_file.is_some() || journal_file.is_some() {
        return stage_with_companions(
            &mut src_file,
            src_meta.len(),
            wal_file,
            journal_file,
            max_stage_bytes,
            record_temp_dir,
        );
    }

    let (header, n) = read_prefix_20(&mut src_file)?;
    drop(src_file);

    if n >= 20 && &header[..16] == SQLITE_MAGIC && (header[18] == 2 || header[19] == 2) {
        let uri = sqlite_immutable_uri(path)?;
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(uri, flags).map_err(SqliteReadError::Sqlite)?;
        return Ok(SurveyReadConn { conn, _staging: None });
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags).map_err(SqliteReadError::Sqlite)?;
    Ok(SurveyReadConn { conn, _staging: None })
}

fn sidecar_path(base: &Path, suffix: &str) -> PathBuf {
    let mut os = base.as_os_str().to_os_string();
    os.push(suffix);
    PathBuf::from(os)
}

fn open_optional_companion(path: &Path) -> Result<Option<(File, u64)>, SqliteReadError> {
    match File::open(path) {
        Ok(file) => {
            let meta = file.metadata().map_err(SqliteReadError::Io)?;
            if !meta.is_file() {
                return Err(SqliteReadError::NotARegularFile);
            }
            Ok(Some((file, meta.len())))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(SqliteReadError::Io(err)),
    }
}

pub(crate) fn classify_stage_io_error(err: io::Error) -> SqliteReadError {
    if err.kind() == io::ErrorKind::StorageFull || matches!(err.raw_os_error(), Some(39 | 112)) {
        SqliteReadError::TempSpaceFull
    } else {
        SqliteReadError::Io(err)
    }
}

fn stage_with_companions(
    src_file: &mut File,
    src_len: u64,
    wal_file: Option<(File, u64)>,
    journal_file: Option<(File, u64)>,
    max_stage_bytes: Option<u64>,
    record_temp_dir: Option<&mut Option<PathBuf>>,
) -> Result<SurveyReadConn, SqliteReadError> {
    let temp_dir = tempfile::tempdir().map_err(classify_stage_io_error)?;
    if let Some(slot) = record_temp_dir {
        *slot = Some(temp_dir.path().to_path_buf());
    }
    let needed = src_len
        .saturating_add(wal_file.as_ref().map_or(0, |(_, len)| *len))
        .saturating_add(journal_file.as_ref().map_or(0, |(_, len)| *len));
    if max_stage_bytes.is_some_and(|limit| needed > limit) {
        return Err(SqliteReadError::TempSpaceFull);
    }

    let staged = temp_dir.path().join("survey.gpkg");
    copy_staged_stream(src_file, &staged)?;
    if let Some((mut wal, _)) = wal_file {
        copy_staged_stream(&mut wal, &sidecar_path(&staged, "-wal"))?;
    }
    if let Some((mut journal, _)) = journal_file {
        copy_staged_stream(&mut journal, &sidecar_path(&staged, "-journal"))?;
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(&staged, flags).map_err(SqliteReadError::Sqlite)?;
    Ok(SurveyReadConn { conn, _staging: Some(temp_dir) })
}

fn copy_staged_stream(src: &mut File, dst_path: &Path) -> Result<(), SqliteReadError> {
    let mut dst = File::create(dst_path).map_err(classify_stage_io_error)?;
    io::copy(src, &mut dst).map_err(classify_stage_io_error)?;
    Ok(())
}

fn read_prefix_20(file: &mut File) -> Result<([u8; 20], usize), SqliteReadError> {
    let mut buf = [0_u8; 20];
    let mut total = 0;
    while total < buf.len() {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(SqliteReadError::Io(e)),
        }
    }
    Ok((buf, total))
}

/// Builds a SQLite `file:` URI with `?immutable=1` per <https://www.sqlite.org/uri.html>.
pub(crate) fn sqlite_immutable_uri(path: &Path) -> Result<String, SqliteReadError> {
    let abs = std::path::absolute(path).map_err(SqliteReadError::Io)?;
    let raw = abs.to_str().ok_or_else(|| {
        SqliteReadError::Io(io::Error::new(io::ErrorKind::InvalidInput, "non-UTF-8 survey path"))
    })?;
    let (prefix, rest) = if let Some(unc) = raw
        .strip_prefix(r"\\?\UNC\")
        .or_else(|| raw.strip_prefix("//?/UNC/"))
    {
        ("file:////", unc)
    } else if let Some(win) = raw.strip_prefix(r"\\?\").or_else(|| raw.strip_prefix("//?/")) {
        ("file:///", win)
    } else if let Some(unc) = raw.strip_prefix(r"\\").or_else(|| raw.strip_prefix("//")) {
        ("file:////", unc)
    } else if let Some(unix) = raw.strip_prefix('/') {
        ("file:///", unix)
    } else {
        ("file:///", raw)
    };

    let mut uri = String::with_capacity(prefix.len() + rest.len() + 16);
    uri.push_str(prefix);
    for ch in rest.chars() {
        match ch {
            '\\' => uri.push('/'),
            '%' | '?' | '#' => {
                let _ = write!(uri, "%{:02X}", ch as u32);
            }
            c if c.is_ascii() && (c as u8 <= 0x20 || c as u8 == 0x7F) => {
                let _ = write!(uri, "%{:02X}", c as u8);
            }
            _ => uri.push(ch),
        }
    }
    uri.push_str("?immutable=1");
    Ok(uri)
}

#[cfg(test)]
pub(crate) fn open_survey_read_only_with_temp_budget(
    path: &Path,
    max_stage_bytes: Option<u64>,
    record_temp_dir: Option<&mut Option<PathBuf>>,
) -> Result<SurveyReadConn, SqliteReadError> {
    open_survey_read_only_inner(path, max_stage_bytes, record_temp_dir)
}

#[cfg(test)]
#[path = "sqlite_read_tests.rs"]
mod sqlite_read_tests;
#[cfg(test)]
#[path = "sqlite_read_more_tests.rs"]
mod sqlite_read_more_tests;
