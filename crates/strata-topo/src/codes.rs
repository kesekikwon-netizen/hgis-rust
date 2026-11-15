//! What a CAD layer of a topographic sheet is: the standard feature codes (수치지도 지형지물 표준코드) and
//! the nine kinds the map tells apart (C++ `TopographicCatalog.cpp:62-103`, `:238-251`). The code book is
//! the file both apps ship, `data/topographic/layer-codes.json`.

use std::collections::HashMap;
use std::sync::LazyLock;

#[cfg(test)]
#[path = "codes_tests.rs"]
mod tests;

/// The kind of thing a layer holds (`TopographicCatalog.h:14-15`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Unknown,
    Contour,
    Road,
    Building,
    Water,
    Vegetation,
    Boundary,
    Facility,
    ElevationPoint,
    PlaceName,
}

/// The kinds in the order the C++ tries their words (`:91-93`).
const NAMED: [Category; 9] = [Category::Contour, Category::Road, Category::Building, Category::Water, Category::Vegetation, Category::Boundary, Category::Facility, Category::ElevationPoint, Category::PlaceName];

impl Category {
    /// `categoryName`, `:238-251`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Contour => "등고선",
            Self::Road => "도로",
            Self::Building => "건물",
            Self::Water => "수계",
            Self::Vegetation => "식생",
            Self::Boundary => "경계",
            Self::Facility => "시설물",
            Self::ElevationPoint => "표고점",
            Self::PlaceName => "지명",
            Self::Unknown => "미분류",
        }
    }

    /// The kinds that are put on the map (`visibleAtScale`, `TopographicCatalog.cpp:351-364`).
    #[must_use]
    pub const fn is_shown(self) -> bool {
        matches!(self, Self::Contour | Self::Road | Self::Building | Self::Water | Self::Boundary)
    }

    /// The kinds whose extent says where a sheet lies (`primary`, `TopographicSourceCrs.cpp:20-27`): the
    /// frame and the texts around it reach past the sheet, these do not.
    #[must_use]
    pub const fn is_primary(self) -> bool {
        matches!(self, Self::Contour | Self::ElevationPoint | Self::Water | Self::Road | Self::Building | Self::Boundary | Self::PlaceName)
    }
}

/// What a layer name was found to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeInfo {
    pub category: Category,
    /// The name of the code in the book, or the kind's own word.
    pub name: String,
}

/// The code book: code -> (kind, name) (`codeBook`, `:63-87`).
static BOOK: LazyLock<HashMap<String, CodeInfo>> = LazyLock::new(|| {
    let kind = |word: &str| match word {
        "Contour" => Category::Contour,
        "Road" => Category::Road,
        "Building" => Category::Building,
        "Water" => Category::Water,
        "Vegetation" => Category::Vegetation,
        "Boundary" => Category::Boundary,
        "Facility" => Category::Facility,
        "ElevationPoint" => Category::ElevationPoint,
        "PlaceName" => Category::PlaceName,
        _ => Category::Unknown,
    };
    let book: serde_json::Value = serde_json::from_str(include_str!("../../../data/topographic/layer-codes.json")).unwrap_or_default();
    let text = |entry: &serde_json::Value, key: &str| entry.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
    let mut entries = HashMap::new();
    for entry in book.get("codes").and_then(serde_json::Value::as_array).map_or(&[][..], Vec::as_slice) {
        let (code, name) = (text(entry, "code"), text(entry, "name"));
        if !code.is_empty() && !name.is_empty() {
            entries.insert(code, CodeInfo { category: kind(&text(entry, "category")), name });
        }
    }
    entries
});

/// The standard codes in a name: a letter A to H and seven digits, with no letter or digit before or after
/// (`(?<![\p{L}\p{N}])([A-H][0-9]{7})(?![\p{L}\p{N}])`, `:94`).
fn codes_in(name: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at + 8 <= chars.len() {
        let token = &chars[at..at + 8];
        let is_code = matches!(token[0].1, 'A'..='H') && token[1..].iter().all(|(_, c)| c.is_ascii_digit());
        let free_before = at == 0 || !chars[at - 1].1.is_alphanumeric();
        let free_after = chars.get(at + 8).is_none_or(|(_, c)| !c.is_alphanumeric());
        if is_code && free_before && free_after {
            let end = chars.get(at + 8).map_or(name.len(), |(byte, _)| *byte);
            found.push(&name[token[0].0..end]);
            at += 8;
        } else {
            at += 1;
        }
    }
    found
}

/// What a layer name says the layer holds (`inferCode`, `:89-103`): the word of a kind, or standard codes
/// that all name the same kind. `None` for anything else: no code, a code that is not in the book, codes of
/// different kinds.
#[must_use]
pub fn infer_code(layer_name: &str) -> Option<CodeInfo> {
    let trimmed = layer_name.trim();
    if let Some(kind) = NAMED.into_iter().find(|kind| kind.name() == trimmed) {
        return Some(CodeInfo { category: kind, name: trimmed.to_owned() });
    }
    let mut result: Option<&CodeInfo> = None;
    for code in codes_in(layer_name) {
        let entry = BOOK.get(code)?;
        if result.is_some_and(|known| known.category != Category::Unknown && known.category != entry.category) {
            return None;
        }
        result = Some(entry);
    }
    result.cloned()
}
