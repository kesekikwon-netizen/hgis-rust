//! The sheets a survey has received: `<survey folder>/지형도/strata-sheets.json`.
//!
//! The C++ keeps an index file and a review receipt for each sheet, with hashes of the source and of the
//! record itself, and a cache of converted SHP files (`KaTopographicScopePanel.cpp:120-340`,
//! `TopographicShapefileCache.cpp`). This app keeps one small list instead (director's decision
//! 2026-11-15: the owner's survey folders hold no C++ library to stay compatible with, and the list is all
//! the two things it is for need): a sheet that is there is not ordered again, and the sheets of the folder
//! can be put on the map again without the portal.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

#[cfg(test)]
#[path = "library_tests.rs"]
mod tests;

/// The list's file in the library folder.
pub const FILE: &str = "strata-sheets.json";

/// What became of a sheet.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetState {
    /// Received, not looked at yet.
    Received,
    /// On the map: the belt it is drawn in.
    Placed { epsg: u32 },
    /// Not put on the map automatically, and why (the C++ 「검토 필요」).
    Review { reason: String },
}

/// One received sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetEntry {
    /// The sheet number (six digits).
    pub num: String,
    pub name: String,
    /// The datum the portal names (GRS80).
    pub projection: String,
    /// The sheet's rectangle in the portal's index, EPSG:5179.
    pub official: Option<[f64; 4]>,
    /// The DXF file, relative to the library folder.
    pub dxf: PathBuf,
    pub state: SheetState,
}

/// The list of a library folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Library {
    root: PathBuf,
    pub sheets: Vec<SheetEntry>,
}

impl Library {
    /// The list of this folder; empty when there is none or it cannot be read.
    #[must_use]
    pub fn open(root: &Path) -> Self {
        let listed = std::fs::read(root.join(FILE)).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let sheets = listed.as_ref().filter(|list| list["schema"].as_u64() == Some(1)).and_then(|list| list["sheets"].as_array()).map(|sheets| sheets.iter().filter_map(read_entry).collect()).unwrap_or_default();
        Self { root: root.to_path_buf(), sheets }
    }

    /// Where a sheet's DXF file is.
    #[must_use]
    pub fn dxf_of(&self, entry: &SheetEntry) -> PathBuf {
        self.root.join(&entry.dxf)
    }

    /// The numbers of the sheets whose file is still there: they are not ordered again.
    #[must_use]
    pub fn numbers(&self) -> Vec<String> {
        self.sheets.iter().filter(|entry| self.dxf_of(entry).is_file()).map(|entry| entry.num.clone()).collect()
    }

    /// Puts a sheet into the list, in the place of an older entry of the same number.
    pub fn upsert(&mut self, entry: SheetEntry) {
        match self.sheets.iter_mut().find(|known| known.num == entry.num) {
            Some(known) => *known = entry,
            None => self.sheets.push(entry),
        }
    }

    /// Writes the list.
    ///
    /// # Errors
    /// The folder cannot be written.
    pub fn save(&self) -> std::io::Result<()> {
        let sheets: Vec<Value> = self.sheets.iter().map(write_entry).collect();
        std::fs::create_dir_all(&self.root)?;
        std::fs::write(self.root.join(FILE), json!({"schema": 1, "sheets": sheets}).to_string())
    }
}

fn write_entry(entry: &SheetEntry) -> Value {
    let mut out = json!({"num": entry.num, "name": entry.name, "projection": entry.projection, "dxf": entry.dxf.to_string_lossy().replace('\\', "/")});
    if let Some(official) = entry.official {
        out["official"] = json!(official);
    }
    match &entry.state {
        SheetState::Received => out["state"] = json!("received"),
        SheetState::Placed { epsg } => (out["state"], out["epsg"]) = (json!("placed"), json!(epsg)),
        SheetState::Review { reason } => (out["state"], out["reason"]) = (json!("review"), json!(reason)),
    }
    out
}

/// An entry of the list; one without a number or a file is none.
fn read_entry(listed: &Value) -> Option<SheetEntry> {
    let text = |key: &str| listed[key].as_str().unwrap_or_default().to_owned();
    let (num, dxf) = (text("num"), text("dxf"));
    if num.is_empty() || dxf.is_empty() {
        return None;
    }
    let official = listed["official"].as_array().filter(|a| a.len() == 4).and_then(|a| Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?, a[3].as_f64()?]));
    let state = match (listed["state"].as_str(), listed["epsg"].as_u64().and_then(|e| u32::try_from(e).ok())) {
        (Some("placed"), Some(epsg)) => SheetState::Placed { epsg },
        (Some("review"), _) => SheetState::Review { reason: text("reason") },
        _ => SheetState::Received,
    };
    Some(SheetEntry { num, name: text("name"), projection: text("projection"), official, dxf: PathBuf::from(dxf), state })
}

/// The rectangle a portal record gives (`minx`, `miny`, `maxx`, `maxy`, as numbers or as digits in a text),
/// when it is one (`officialBounds`, `KaTopographicScopePanel.cpp`: finite, max above min).
#[must_use]
pub fn official_bounds(record: &Map<String, Value>) -> Option<[f64; 4]> {
    let number = |key: &str| match record.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    };
    let bounds = [number("minx")?, number("miny")?, number("maxx")?, number("maxy")?];
    (bounds.iter().all(|v| v.is_finite()) && bounds[2] > bounds[0] && bounds[3] > bounds[1]).then_some(bounds)
}
