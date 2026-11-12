//! 좌표 변환 — PROJ. QGIS `QgsCoordinateTransform(src, dst).transform` 과 같은 길
//! (`proj_create_crs_to_crs` + `proj_normalize_for_visualization`, 넓이 지정 없음).
//!
//! proj.db(EPSG 자료)는 실행 파일 안에 들어 있다 — proj-sys 의 PROJ 9.6.2 소스 빌드가
//! EMBED_RESOURCE_FILES=ON 이라 OSGeo4W 없이 다른 PC 에서도 그대로 돈다. PROJ 는 파일을 먼저 찾고
//! (filemanager.cpp `pj_open_lib_internal`: `PROJ_DATA` 환경 변수 → 실행 파일 기준 `..\share\proj` →
//! 빌드 폴더) 없을 때 안에 든 것을 쓴다. `PROJ_DATA` 를 따르는 것은 QGIS 와 같다.

use proj::{Proj, ProjBuilder};

use crate::EngineError;

/// EPSG 좌표계 둘 사이의 변환기. 만들기가 비싸므로 한 번 만들어 여러 점에 쓴다.
#[derive(Debug)]
pub struct Transformer {
    proj: Proj,
}

impl Transformer {
    /// `EPSG:{from}` → `EPSG:{to}`.
    ///
    /// # Errors
    /// PROJ 가 변환을 만들지 못함(좌표계를 모름, proj.db 없음 등).
    pub fn new(from_epsg: u32, to_epsg: u32) -> Result<Self, EngineError> {
        let proj = ProjBuilder::new()
            .proj_known_crs(&format!("EPSG:{from_epsg}"), &format!("EPSG:{to_epsg}"), None)
            .map_err(|e| EngineError(e.to_string()))?;
        Ok(Self { proj })
    }

    /// 점 하나.
    ///
    /// # Errors
    /// PROJ 가 이 점을 바꾸지 못함.
    pub fn transform(&self, xy: [f64; 2]) -> Result<[f64; 2], EngineError> {
        let (x, y) = self.proj.convert((xy[0], xy[1])).map_err(|e| EngineError(e.to_string()))?;
        Ok([x, y])
    }

    /// 점 여러 개(하나라도 실패하면 오류).
    ///
    /// # Errors
    /// PROJ 가 어느 점을 바꾸지 못함.
    pub fn transform_all(&self, points: &[[f64; 2]]) -> Result<Vec<[f64; 2]>, EngineError> {
        points.iter().map(|p| self.transform(*p)).collect()
    }
}

#[cfg(test)]
#[path = "crs_tests.rs"]
mod tests;
