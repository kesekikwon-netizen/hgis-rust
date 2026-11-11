//! 검증용: 조사 파일에서 제출 꾸러미를 만들어 폴더에 남긴다(GDAL 등으로 열어 보려고 쓴다).
//!
//! `cargo run -p strata-check-export --example make_submit_package -- <조사.gpkg> <출력 폴더>`

use std::path::PathBuf;

use strata_check_export::submit_survey_package::export_survey_package;
use strata_storage::survey_gpkg_read::read_survey_gpkg;
use strata_types::error::{StrataError, StrataResult};

fn main() -> StrataResult<()> {
    let mut args = std::env::args().skip(1);
    let (Some(gpkg), Some(out)) = (args.next(), args.next()) else {
        return Err(StrataError::Storage("인자: <조사.gpkg> <출력 폴더>".into()));
    };
    let doc = read_survey_gpkg(&PathBuf::from(gpkg))?;
    let (manifest, layers) = export_survey_package(&PathBuf::from(out), &doc, &[], None)?;
    for l in layers {
        println!("{} {}", l.file_name, l.features);
    }
    println!("files {}", manifest.files_sha256.len());
    Ok(())
}
