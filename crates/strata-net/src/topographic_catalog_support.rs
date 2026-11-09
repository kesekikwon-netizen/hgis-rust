//! `topographic_catalog` 시험 공용 도우미 (임시 폴더·가짜 SHP/DXF·PRJ 문자열).

#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::path::PathBuf;

use super::*;

/// 테스트용 임시 폴더. 끝나면 스스로 지운다(C++ `QTemporaryDir` 자리).
pub(crate) struct TempFolder {
    pub(crate) path: PathBuf,
}

impl TempFolder {
    pub(crate) fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = std::env::temp_dir()
            .join(format!("ka_hgis_{tag}_{}_{nanos}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("임시 폴더를 만들지 못했습니다");
        Self { path }
    }

    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let target = self.path.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("상위 폴더를 만들지 못했습니다");
        }
        std::fs::write(&target, bytes).expect("가짜 자료를 쓰지 못했습니다");
        target
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// `.shp` 머리말 100바이트(매직 9994 + 경계 상자)를 만든다.
pub(crate) fn shp_bytes(extent: Extent) -> Vec<u8> {
    let mut header = vec![0_u8; 100];
    header[..4].copy_from_slice(&9994_u32.to_be_bytes());
    for (offset, value) in [
        (36, extent.min_x),
        (44, extent.min_y),
        (52, extent.max_x),
        (60, extent.max_y),
    ] {
        header[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    header
}

/// CAD 레이어 `layer` 에 선 두 개를 둔 최소 DXF.
pub(crate) fn dxf_bytes(layer: &str) -> Vec<u8> {
    format!(
        "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n{layer}\n10\n1000.0\n20\n2000.0\n\
         0\nLINE\n8\n{layer}\n10\n1500.0\n20\n2500.0\n0\nENDSEC\n0\nEOF\n"
    )
    .into_bytes()
}

pub(crate) const PRJ_5186: &str = "PROJCS[\"Korea 2000 / Central Belt\",GEOGCS[\"Korea 2000\",\
    DATUM[\"Korea_2000\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],\
    PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
    PROJECTION[\"Transverse_Mercator\"],UNIT[\"metre\",1],AUTHORITY[\"EPSG\",\"5186\"]]";

pub(crate) const PRJ_5187_NAMED: &str = "PROJCS[\"Korea_2000_Korea_East_Belt\",GEOGCS[\"Korea 2000\",\
    DATUM[\"Korea_2000\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],\
    PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]],\
    PROJECTION[\"Transverse_Mercator\"],UNIT[\"metre\",1]]";

// C++ `TopographicCatalog.cpp:41-45,50-52` — 확장자 네 가지, shp 는 .prj 동반 확인.
