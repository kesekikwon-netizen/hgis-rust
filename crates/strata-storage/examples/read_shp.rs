//! 검증용: SHP 를 읽어 도형 수·부분 수·속성을 찍는다(GDAL 이 쓴 파일을 읽을 수 있는지 확인).
//!
//! `cargo run -p strata-storage --example read_shp -- <파일.shp>`

use std::path::PathBuf;

use strata_storage::shp_file_reader::read_shapefile;
use strata_types::error::{StrataError, StrataResult};

fn main() -> StrataResult<()> {
    let Some(path) = std::env::args().nth(1) else {
        return Err(StrataError::Storage("인자: <파일.shp>".into()));
    };
    let ds = read_shapefile(&PathBuf::from(path))?;
    println!("type={} shapes={} fields={:?}", ds.shape_type, ds.shapes.len(), ds.fields);
    for s in &ds.shapes {
        let pts: usize = s.parts.iter().map(Vec::len).sum();
        let first = s.parts.first().and_then(|p| p.first()).copied().unwrap_or([0.0, 0.0]);
        println!("parts={} points={} first=({:.3}, {:.3}) attrs={:?}", s.parts.len(), pts, first[0], first[1], s.attrs);
    }
    Ok(())
}
