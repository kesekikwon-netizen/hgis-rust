//! C++ `SurveyProjectFactory::createNewSurvey`(`SurveyProjectFactory.cpp:27-170`)의 GPKG 쪽.
//!
//! 저장 폴더 안 `.ka-new-survey-XXXXXX` 임시 폴더에 C++(GDAL)과 같은 표·트리거·좌표계·
//! 판 번호를 가진 조사 파일을 다 쓰고 검사한 뒤, 기존 파일을 덮어쓰지 않는 방법으로 제자리에
//! 놓는다. 작업 화면 파일(.qgz)은 아직 만들지 않는다(지시 2A 범위 밖).

use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};

use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::survey_gpkg_ddl::{
    gpkg_base_sql, layer_sql, GPKG_METADATA_DDL, SURVEY_LAYERS, SURVEY_SCHEMA_VERSION, WKT_5186,
    WKT_5187,
};

/// C++ `SurveyProjectFactory.cpp:27-170` — 새 조사 GPKG 를 만든다. 같은 이름 거절은 부르는 쪽
/// (`GpkgRepository::create_new_survey`)이 먼저 한다.
pub(crate) fn create_cpp_survey_file(path: &Path, work_crs: WorkCrs) -> StrataResult<()> {
    let storage = |msg: &str| StrataError::Storage(msg.to_owned());
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    // C++ :34-37
    if !dir.exists() && std::fs::create_dir_all(dir).is_err() {
        return Err(storage("저장 폴더를 만들 수 없습니다. 쓰기 가능한 다른 폴더를 선택해 주세요."));
    }
    // C++ :61-67
    let staging = make_staging_dir(dir).ok_or_else(|| {
        storage("새 조사 파일을 준비할 수 없습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인해 주세요.")
    })?;
    let staged = staging.join(path.file_name().unwrap_or_default());
    let result = write_survey_gpkg(&staged, work_crs)
        .map_err(|e| {
            StrataError::Storage(format!(
                "새 조사 파일을 기록하지 못했습니다. 남은 공간과 쓰기 권한을 확인한 뒤 다시 시도해 주세요.\n{e}"
            ))
        })
        .and_then(|()| check_integrity(&staged))
        .and_then(|()| install_without_overwrite(&staged, path));
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// C++ `QTemporaryDir(dir.filePath(".ka-new-survey-XXXXXX"))`.
fn make_staging_dir(dir: &Path) -> Option<PathBuf> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos())
        ^ std::process::id();
    (0..16_u32).find_map(|i| {
        let candidate = dir.join(format!(".ka-new-survey-{:06x}", (seed.wrapping_add(i * 7919)) & 0xFF_FFFF));
        std::fs::create_dir(&candidate).ok().map(|()| candidate)
    })
}

/// GDAL 이 같은 조사를 만들 때와 같은 순서로 표·행을 쓴다(`survey_gpkg_ddl` 참고).
fn write_survey_gpkg(path: &Path, work_crs: WorkCrs) -> rusqlite::Result<()> {
    let conn = Connection::open(path)?;
    register_gpkg_sql_functions(&conn)?;
    let epsg = work_crs.epsg_code();
    let (srs_name, wkt) = match work_crs {
        WorkCrs::Epsg5186 => ("KGD2002 / Central Belt 2010", WKT_5186),
        WorkCrs::Epsg5187 => ("KGD2002 / East Belt 2010", WKT_5187),
    };
    let mut sql = String::from("PRAGMA application_id = 1196444487;\nPRAGMA user_version = 10400;\nBEGIN;\n");
    sql.push_str(&gpkg_base_sql());
    sql.push_str(&format!(
        "INSERT INTO gpkg_spatial_ref_sys VALUES ('{srs_name}', {epsg}, 'EPSG', {epsg}, '{wkt}', NULL);\n"
    ));
    for (i, layer) in SURVEY_LAYERS.iter().enumerate() {
        sql.push_str(&layer_sql(layer, epsg, i == 0));
    }
    // C++ `SurveySchema.cpp:146-162` writeVersion — GDAL 데이터셋 메타데이터 한 항목.
    sql.push_str(GPKG_METADATA_DDL);
    sql.push_str(&format!(
        "INSERT INTO gpkg_extensions VALUES ('gpkg_metadata', NULL, 'gpkg_metadata', 'http://www.geopackage.org/spec120/#extension_metadata', 'read-write');
INSERT INTO gpkg_extensions VALUES ('gpkg_metadata_reference', NULL, 'gpkg_metadata', 'http://www.geopackage.org/spec120/#extension_metadata', 'read-write');
INSERT INTO gpkg_metadata (id, md_scope, md_standard_uri, mime_type, metadata) VALUES (1, 'dataset', 'http://gdal.org', 'text/xml', '<GDALMultiDomainMetadata>
  <Metadata>
    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">{SURVEY_SCHEMA_VERSION}</MDI>
  </Metadata>
</GDALMultiDomainMetadata>
');
INSERT INTO gpkg_metadata_reference (reference_scope, md_file_id) VALUES ('geopackage', 1);
COMMIT;
"
    ));
    conn.execute_batch(&sql)
}

/// C++ :120-127 `validateForOpen` 자리 — 다 쓴 임시 파일이 SQLite 로 온전한지.
fn check_integrity(path: &Path) -> StrataResult<()> {
    let check = Connection::open(path)
        .and_then(|c| c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0)));
    match check {
        Ok(s) if s == "ok" => Ok(()),
        other => Err(StrataError::Storage(format!(
            "새 조사 파일을 확인하지 못해 생성을 중단했습니다. 다시 시도해 주세요.\n{other:?}"
        ))),
    }
}

/// C++ :140-150 — 같은 볼륨 rename 은 기존 파일을 덮어쓰지 않는다. Rust `rename` 은 Windows 에서
/// 덮어쓰므로 하드링크(대상이 있으면 실패)로 놓는다. 하드링크를 못 쓰는 저장 장치(FAT·exFAT
/// USB 등)에서는 「새 파일로만」 열어 복사한다.
fn install_without_overwrite(staged: &Path, path: &Path) -> StrataResult<()> {
    let refuse = || {
        StrataError::Storage(
            "새 조사 파일을 저장 폴더에 놓을 수 없습니다. 같은 이름의 파일이 생겼는지 확인하고 다른 이름으로 다시 시도해 주세요."
                .to_owned(),
        )
    };
    match std::fs::hard_link(staged, path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Err(refuse()),
        Err(_) => {
            let mut out = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|_| refuse())?;
            let copied = File::open(staged)
                .and_then(|mut src| std::io::copy(&mut src, &mut out))
                .and_then(|_| out.sync_all());
            if copied.is_err() {
                drop(out);
                let _ = std::fs::remove_file(path);
                return Err(refuse());
            }
            Ok(())
        }
    }
}
