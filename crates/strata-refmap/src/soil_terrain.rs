//! The terrain classes of the NAAS (흙토람) soil map: code, legend name, official colour and the short name
//! written over a unit (C++ `SoilMapService.cpp:48-66` `kTerrain`, `:140-150`, `:173-198`).

#[cfg(test)]
#[path = "soil_terrain_tests.rs"]
mod tests;

/// One terrain class (`TerrainClass`, `:49-53`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terrain {
    pub code: &'static str,
    /// Its name in the legend.
    pub name: &'static str,
    /// The official palette of the 흙토람 legend (`:48`).
    pub rgb: [u8; 3],
    /// Its name over a unit on the map (`terrainLabelExpression`, `:173-198`).
    pub label: &'static str,
}

/// `kTerrain`, `:54-66`, with the label of each code (`:181-195`).
pub const TERRAIN: [Terrain; 11] = [
    Terrain { code: "01", name: "산악지", rgb: [1, 178, 0], label: "산악지" },
    Terrain { code: "02", name: "구릉지", rgb: [177, 210, 93], label: "구릉지" },
    Terrain { code: "03", name: "산록경사지", rgb: [139, 68, 108], label: "산록경사" },
    Terrain { code: "04", name: "곡간지/선상지", rgb: [254, 244, 182], label: "곡간·선상" },
    Terrain { code: "05", name: "해성평탄지", rgb: [132, 186, 230], label: "해성평탄" },
    Terrain { code: "06", name: "하성평탄지", rgb: [192, 239, 245], label: "하성평탄" },
    Terrain { code: "07", name: "고원지", rgb: [78, 78, 78], label: "고원지" },
    Terrain { code: "08", name: "홍적대지", rgb: [254, 160, 0], label: "홍적대지" },
    Terrain { code: "09", name: "용암류대지", rgb: [192, 151, 155], label: "용암대지" },
    Terrain { code: "10", name: "용암류평탄", rgb: [194, 179, 182], label: "용암평탄" },
    Terrain { code: "99", name: "기타", rgb: [255, 255, 255], label: "기타" },
];

/// Name and colour of a code that is none of the eleven (`:143`, `:149`).
pub const UNCLASSED: &str = "미분류";
pub const UNCLASSED_RGB: [u8; 3] = [200, 200, 200];

fn class(code: &str) -> Option<&'static Terrain> {
    TERRAIN.iter().find(|terrain| terrain.code == code)
}

/// `SoilMapService::terrainName`, `:140-144`.
#[must_use]
pub fn terrain_name(code: &str) -> &'static str {
    class(code).map_or(UNCLASSED, |terrain| terrain.name)
}

/// `SoilMapService::terrainColor`, `:146-150`.
#[must_use]
pub fn terrain_colour(code: &str) -> [u8; 3] {
    class(code).map_or(UNCLASSED_RGB, |terrain| terrain.rgb)
}

/// What is written over a unit of this code; nothing for a code that is none of the eleven (`:196`).
#[must_use]
pub fn label_text(code: &str) -> &'static str {
    class(code).map_or("", |terrain| terrain.label)
}
