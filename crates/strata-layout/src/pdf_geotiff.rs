//! `pdf_geotiff` 는 도면 PDF·GeoTIFF 내보내기의 재수출 창구다.
//!
//! 원래 한 파일(429줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_layout::pdf_geotiff::<이름>` 경로는 그대로 동작한다.

pub use crate::raster_stretch::*;
pub use crate::sheet_pdf_export::*;

#[cfg(test)]
#[path = "pdf_geotiff_tests.rs"]
mod pdf_geotiff_tests;
