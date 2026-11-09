//! `dem_synthesizer` 는 정밀 DEM 합성의 재수출 창구다.
//!
//! 원래 한 파일(465줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_paleo::dem_synthesizer::<이름>` 경로는 그대로 동작한다.

pub use crate::dem_contour_interp::*;
pub use crate::dem_hydrology::*;
pub use crate::dem_synthesis_io::*;
