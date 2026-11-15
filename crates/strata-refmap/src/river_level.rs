//! The classes of the VWorld river network and their water colours (C++ `RiverMapService.cpp:45-65`,
//! `:67-80`, `:205-222`): the higher the class, the deeper the blue.

#[cfg(test)]
#[path = "river_level_tests.rs"]
mod tests;

/// One class of river (`LevelDef`, `:46-50`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// What the `riv_level` text of such a river contains.
    pub keyword: &'static str,
    /// Its name in the legend.
    pub label: &'static str,
    /// The fill (`:51-55`).
    pub fill: [u8; 3],
    /// The outline: the fill made darker by 150 as Qt does it (`waterFillSymbol`, `:70`:
    /// `base.darker(150)`). The numbers are what Qt 6 returns for each fill (asked of the installed Qt on
    /// 2026-11-15), not a formula of this app.
    pub line: [u8; 3],
}

/// `kLevels`, `:51-55`.
pub const LEVELS: [Level; 3] = [
    Level { keyword: "국가", label: "국가하천", fill: [0, 126, 212], line: [0, 84, 141] },
    Level { keyword: "지방1급", label: "지방1급하천", fill: [16, 146, 222], line: [11, 97, 148] },
    Level { keyword: "지방2급", label: "지방2급하천", fill: [40, 166, 232], line: [27, 111, 155] },
];

/// A river of another class or of none (`:214`, `:218-219`).
pub const OTHER: Level = Level { keyword: "", label: "기타 수계", fill: [120, 196, 236], line: [80, 131, 157] };

/// The class a `riv_level` text names: the first whose keyword it contains (`levelRank`, `:57-64`).
#[must_use]
pub fn level_of(level_text: &str) -> &'static Level {
    LEVELS.iter().find(|level| level_text.contains(level.keyword)).unwrap_or(&OTHER)
}
