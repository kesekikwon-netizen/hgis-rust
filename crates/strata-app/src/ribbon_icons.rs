//! `ribbon_icons` 는 리본 아이콘 드로잉·아이콘 id 표·툴팁의 재수출 창구다.
//!
//! 원래 398줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::ribbon_icons::<이름>` 경로는 그대로 동작한다.

#[path = "ribbon_icons_logo.rs"]
mod ribbon_icons_logo;
#[path = "ribbon_icons_map.rs"]
mod ribbon_icons_map;
#[path = "ribbon_icons_paint.rs"]
mod ribbon_icons_paint;

pub use ribbon_icons_logo::*;
pub use ribbon_icons_map::*;
pub use ribbon_icons_paint::*;
