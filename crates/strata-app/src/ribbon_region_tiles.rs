//! 한반도 17개 시·도 타일 지도 (C++ `KaRegionLocator.cpp:432-439` 1:1).
//!
//! `ribbon.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionGridTile {
    pub name: &'static str,
    pub row: usize,
    pub col: usize,
    pub center_5187: [f64; 2],
}

/// `KaRegionLocator.cpp:432-439` — 6행 4열 한반도 모양 17개 시·도 타일 지도
pub const PENINSULA_REGION_TILES: [RegionGridTile; 17] = [
    RegionGridTile {
        name: "서울",
        row: 0,
        col: 1,
        center_5187: [95_300.0, 552_000.0],
    },
    RegionGridTile {
        name: "인천",
        row: 1,
        col: 0,
        center_5187: [70_500.0, 540_200.0],
    },
    RegionGridTile {
        name: "경기",
        row: 1,
        col: 1,
        center_5187: [104_200.0, 520_800.0],
    },
    RegionGridTile {
        name: "강원",
        row: 1,
        col: 2,
        center_5187: [182_000.0, 565_000.0],
    },
    RegionGridTile {
        name: "충남",
        row: 2,
        col: 0,
        center_5187: [72_000.0, 450_000.0],
    },
    RegionGridTile {
        name: "세종",
        row: 2,
        col: 1,
        center_5187: [125_400.0, 432_000.0],
    },
    RegionGridTile {
        name: "충북",
        row: 2,
        col: 2,
        center_5187: [152_000.0, 455_000.0],
    },
    RegionGridTile {
        name: "경북",
        row: 2,
        col: 3,
        center_5187: [219_850.0, 360_120.0],
    },
    RegionGridTile {
        name: "전북",
        row: 3,
        col: 0,
        center_5187: [108_000.0, 368_000.0],
    },
    RegionGridTile {
        name: "대전",
        row: 3,
        col: 1,
        center_5187: [136_000.0, 415_000.0],
    },
    RegionGridTile {
        name: "대구",
        row: 3,
        col: 2,
        center_5187: [195_200.0, 363_800.0],
    },
    RegionGridTile {
        name: "울산",
        row: 3,
        col: 3,
        center_5187: [238_500.0, 338_000.0],
    },
    RegionGridTile {
        name: "광주",
        row: 4,
        col: 0,
        center_5187: [90_200.0, 295_000.0],
    },
    RegionGridTile {
        name: "전남",
        row: 4,
        col: 1,
        center_5187: [95_000.0, 265_000.0],
    },
    RegionGridTile {
        name: "경남",
        row: 4,
        col: 2,
        center_5187: [180_000.0, 305_000.0],
    },
    RegionGridTile {
        name: "부산",
        row: 4,
        col: 3,
        center_5187: [220_000.0, 296_000.0],
    },
    RegionGridTile {
        name: "제주",
        row: 5,
        col: 1,
        center_5187: [66_000.0, 92_000.0],
    },
];
