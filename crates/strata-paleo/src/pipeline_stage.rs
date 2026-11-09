//! 단계별 진행률 상수와 파이프라인 상태.
//!
//! `pipeline` 에서 분리했다.

use serde::{Deserialize, Serialize};

/// C++ `KaPaleoPipelineRunner` 4단계 진행률 상수 (`20% -> 50% -> 80% -> 100%`)
pub const HGIS_STAGE1_PROGRESS_END: u8 = 20;

pub const HGIS_STAGE2_PROGRESS_END: u8 = 50;

pub const HGIS_STAGE3_PROGRESS_END: u8 = 80;

pub const HGIS_STAGE4_PROGRESS_END: u8 = 100;

/// C++ `PaleoPipelineResult` 실행 상태 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoPipelineStatus {
    Completed,
    UserCancelled,
    InvalidInput,
}
