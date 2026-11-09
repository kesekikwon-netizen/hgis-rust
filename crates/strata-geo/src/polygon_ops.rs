//! `polygon_ops` 는 기능별 하위 모듈의 재수출 창구다.
//!
//! 원래 한 파일(1,629줄)이던 것을 300줄 규칙에 맞춰 나눴다.
//! 기존 호출부가 쓰던 `strata_geo::polygon_ops::<이름>` 경로는 그대로 동작한다.

pub use crate::boolean_split::*;
pub use crate::cadastral_parcel::*;
pub use crate::cluster_traverse::*;
pub use crate::gcp_solve::*;
pub use crate::georef_quality::*;
pub use crate::helmert_solve::*;
pub use crate::mbr_catalog::*;
pub use crate::sliver_hole::*;
pub use crate::snap_vertex_ops::*;
