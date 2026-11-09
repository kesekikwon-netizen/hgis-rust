//! 도엽 번호 격자·범위 계산 (C++ `core/TopographicSheets.cpp` `numberForCell` / `resolve`).
//!
//! `ngii_topographic` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

/// 지리 좌표 범위(경도/위도)
#[derive(Debug, Clone, PartialEq)]
pub struct SheetScope {
    pub wkt: String,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// 공식 `suchiQuery.do` 가 돌려준 1:25,000 DXF 도엽 한 장
#[derive(Debug, Clone, PartialEq)]
pub struct SheetProduct {
    pub num: String,
    pub name: String,
    pub scale: String,
    pub file_ext: String,
    pub projection: String,
    pub minx: f64,
    pub miny: f64,
    pub maxx: f64,
    pub maxy: f64,
}

/// 도엽 질의 결과 — 받을 수 있는 도엽과, 요청했지만 공식 목록에 없는 도엽
#[derive(Debug, Clone, PartialEq)]
pub struct SheetQuery {
    pub products: Vec<SheetProduct>,
    pub unavailable: Vec<String>,
}

impl SheetQuery {
    /// `KaTopographicBrowser.cpp:868-871` — 신청할 도엽 목록을 8장까지 보여 주고 나머지는 "외 n장"
    #[must_use]
    pub fn order_sheets_text(&self) -> String {
        if self.products.is_empty() {
            return String::new();
        }
        let total = self.products.len();
        let mut listed: Vec<String> = self
            .products
            .iter()
            .take(8)
            .map(|p| format!("{} {}", p.num, p.name).trim().to_owned())
            .collect();
        if total > 8 {
            listed.push(format!("외 {}장", total - 8));
        }
        format!(
            "공식 신청할 도엽 {}장 · {} — 중지하려면 「취소」를 누르세요.",
            total,
            listed.join(", ")
        )
    }
}

/// C++ `TopographicSheets` — 1:25,000 도엽 번호 격자(경위도 7분 30초 = 0.125°)
#[derive(Debug, Clone, Copy)]
pub struct TopographicSheets;

impl TopographicSheets {
    /// C++ `TopographicSheets.cpp:10` `kGridSize` — 공식 7분 30초
    pub const GRID_SIZE: f64 = 0.125;
    /// C++ `TopographicSheets.cpp:11-12`
    pub const WEST: f64 = 124.0;
    pub const EAST: f64 = 132.0;

    /// C++ `TopographicSheets.cpp:24-31` `numberForCell`
    #[must_use]
    pub fn number_for_cell(x: i32, y: i32) -> String {
        let base_longitude = x.div_euclid(8);
        let base_latitude = y.div_euclid(8);
        let north_row = 7 - y.rem_euclid(8);
        let map50k = (north_row / 2) * 4 + x.rem_euclid(8) / 2 + 1;
        let quarter = (north_row % 2) * 2 + x.rem_euclid(2) + 1;
        format!(
            "{:02}{}{:02}{}",
            base_latitude,
            base_longitude % 10,
            map50k,
            quarter
        )
    }

    /// C++ `TopographicSheets.cpp:34-60` `resolve` — 6자리 숫자만 받는다.
    #[must_use]
    pub fn resolve(number: &str) -> Option<TopographicSheet> {
        if number.len() != 6 || !number.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let base_latitude: i32 = number[0..2].parse().ok()?;
        let longitude_digit: i32 = number[2..3].parse().ok()?;
        let map50k: i32 = number[3..5].parse().ok()?;
        let quarter: i32 = number[5..6].parse().ok()?;
        if base_latitude >= 90
            || longitude_digit == 2
            || longitude_digit == 3
            || !(1..=16).contains(&map50k)
            || !(1..=4).contains(&quarter)
        {
            return None;
        }
        let base_longitude: i32 = (if longitude_digit >= 4 { 120 } else { 130 }) + longitude_digit;
        let index50k = map50k - 1;
        let index25k = quarter - 1;
        let west = f64::from(base_longitude)
            + f64::from(index50k % 4) * 0.25
            + f64::from(index25k % 2) * Self::GRID_SIZE;
        let north = f64::from(base_latitude) + 1.0
            - f64::from(index50k / 4) * 0.25
            - f64::from(index25k / 2) * Self::GRID_SIZE;
        let center_longitude = west + Self::GRID_SIZE / 2.0;
        let zone = if center_longitude < 126.0 {
            TopographicZone::West
        } else if center_longitude < 128.0 {
            TopographicZone::Central
        } else if center_longitude < 130.0 {
            TopographicZone::East
        } else {
            TopographicZone::EastSea
        };
        Some(TopographicSheet {
            number: number.to_owned(),
            west,
            south: north - Self::GRID_SIZE,
            east: west + Self::GRID_SIZE,
            north,
            zone,
        })
    }

    /// 지리 좌표 범위가 걸치는 도엽 번호(정렬·중복 제거) — C++ `select` 의 후보 격자 훑기와 같은 규칙.
    #[must_use]
    pub fn numbers_in_geographic_box(box_wgs84: [f64; 4]) -> Vec<String> {
        let min_x = box_wgs84[0].min(box_wgs84[2]);
        let max_x = box_wgs84[0].max(box_wgs84[2]);
        let min_y = box_wgs84[1].min(box_wgs84[3]);
        let max_y = box_wgs84[1].max(box_wgs84[3]);
        if !(min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite()) {
            return Vec::new();
        }
        // C++ `TopographicSheets.cpp:113-117`
        let lo_x = ((min_x.clamp(Self::WEST, Self::EAST)) * 8.0).floor() as i32;
        let hi_x = 1055.min(((max_x.clamp(Self::WEST, Self::EAST)) * 8.0).floor() as i32);
        let lo_y = ((min_y.clamp(0.0, 90.0)) * 8.0).floor() as i32;
        let hi_y = 719.min(((max_y.clamp(0.0, 90.0)) * 8.0).floor() as i32);
        let mut out = Vec::new();
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                let number = Self::number_for_cell(x, y);
                if Self::resolve(&number).is_some() && !out.contains(&number) {
                    out.push(number);
                }
            }
        }
        out.sort();
        out
    }
}

/// 도엽이 속한 원점대(C++ `TopographicSheets::Zone`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopographicZone {
    West,
    Central,
    East,
    EastSea,
}

impl TopographicZone {
    /// C++ `TopographicSheets.cpp:62-71` `zoneName`
    #[must_use]
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::West => "서부원점",
            Self::Central => "중부원점",
            Self::East => "동부원점",
            Self::EastSea => "동해원점",
        }
    }

    /// C++ `TopographicSheets.cpp:53-57` — 5185/5186/5187/5188
    #[must_use]
    pub const fn epsg(self) -> u32 {
        match self {
            Self::West => 5185,
            Self::Central => 5186,
            Self::East => 5187,
            Self::EastSea => 5188,
        }
    }
}

/// 한 장의 1:25,000 도엽 — 지리(경위도) 도곽
#[derive(Debug, Clone, PartialEq)]
pub struct TopographicSheet {
    pub number: String,
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
    pub zone: TopographicZone,
}

impl TopographicSheet {
    #[must_use]
    pub const fn extent(&self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }

    /// C++ `TopographicSheets.cpp:58` — `World2010` 근거가 확립된 경우에만 채운다.
    #[must_use]
    pub fn crs_auth_id(&self, world_2010: bool) -> Option<String> {
        world_2010.then(|| format!("EPSG:{}", self.zone.epsg()))
    }
}
