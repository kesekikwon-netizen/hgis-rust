//! SVG 한 조각 타입과 아이콘 id 진입점.
//!
//! 아이콘 데이터는 300줄 규칙에 맞춰 `ribbon_icon_svg_part1` / `part2` 로 나눴다.
//! 기존 `crate::ribbon_icon_svg::svg_parts(id)` 경로는 그대로 동작한다.

mod part1 {
    pub use crate::ribbon_icon_svg_part1::*;
}
mod part2 {
    pub use crate::ribbon_icon_svg_part2::*;
}

/// SVG 한 조각 — Lucide 원문 요소 하나.
#[derive(Debug, Clone, Copy)]
pub enum SvgPart {
    /// `<path d="...">`
    Path(&'static str),
    /// `<circle cx|cy|r>`
    Circle(&'static str),
    /// `<line x1|y1|x2|y2>`
    Line(&'static str),
    /// `<rect x|y|width|height|rx>`
    Rect(&'static str),
    /// `<polyline points>`
    Polyline(&'static str),
}

/// 앱 아이콘 id → Lucide 원문 조각들.
pub fn svg_parts(id: &str) -> Option<&'static [SvgPart]> {
    part1::parts_1(id).or_else(|| part2::parts_2(id))
}
