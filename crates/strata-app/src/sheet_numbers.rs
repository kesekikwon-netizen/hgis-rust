//! The numbers of the heritage sites on a sheet (`target/cpp-ref/src/core/HeritageLayoutNumbers.cpp`): the
//! rows of the 「유산」 button that are checked in the legend are numbered for the map of the sheet, and the
//! sheet draws a badge on every numbered site. The rule is `strata_refmap::heritage_numbers`.

use std::path::PathBuf;
use std::sync::Arc;

use strata_layout::sheet_heritage::{ink_for_badge_rgb, layout_badge_circle_size_mm};
use strata_refmap::heritage_numbers::{number_sites, numbered_area, read_sites, SheetLayer, Site};
use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::heritage_run::{dataset_of_key, KEY_PREFIX};

#[cfg(test)]
#[path = "sheet_numbers_tests.rs"]
mod tests;

/// The height of the number in a badge, in millimetres of paper (`:1064-1083`: Malgun Gothic Bold 1.82 mm).
const LETTER_MM: f32 = 1.82;

/// A numbered site of the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Badge {
    /// The legend row of its layer.
    pub(crate) layer_key: String,
    /// The name of its kind (「지정유산」 …).
    pub(crate) kind: &'static str,
    pub(crate) name: String,
    pub(crate) number: u32,
    pub(crate) colour: [u8; 3],
    /// Where the badge stands, in the work CRS.
    pub(crate) at: [f64; 2],
}

/// How a badge is drawn, in points of the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BadgeLook {
    pub(crate) diameter: f32,
    pub(crate) ink: [u8; 3],
    /// The height of the number.
    pub(crate) letter: f32,
}

/// The look of the badge of a number on a sheet shown at `px_per_mm` points for a millimetre of paper: a
/// circle in the colour of the kind, the number black on a light colour and white on a dark one
/// (`circleSize`, `inkFor`, `:103-110`).
pub(crate) fn badge_look(number: u32, colour: [u8; 3], px_per_mm: f32) -> BadgeLook {
    // A diameter of a few millimetres loses nothing as f32.
    #[allow(clippy::cast_possible_truncation)]
    let diameter_mm = layout_badge_circle_size_mm(number as usize) as f32;
    BadgeLook { diameter: diameter_mm * px_per_mm, ink: ink_for_badge_rgb(colour), letter: LETTER_MM * px_per_mm }
}

/// The sites of a layer's copy: read once, kept while the row draws that file.
type Sites = Arc<Vec<Site>>;

/// What the numbers were asked for: the checked rows with their files in the order of the legend, the map
/// of the sheet, the survey areas and the work CRS.
#[derive(Debug, Clone, PartialEq)]
struct Asked {
    rows: Vec<(String, PathBuf)>,
    sheet_box: [f64; 4],
    survey: Vec<String>,
    work_epsg: u32,
}

/// What the sheet remembers between frames.
#[derive(Debug, Clone, Default)]
pub(crate) struct SheetNumbers {
    /// The sites of the copies read so far, with the CRS of their shapes.
    read: Vec<(PathBuf, u32, Sites)>,
    last: Option<(Asked, Vec<Badge>)>,
}

impl SheetNumbers {
    /// The sites of a copy; a copy that cannot be read has none (its row is drawn, its sites not numbered).
    fn sites_of(&mut self, file: &PathBuf) -> (u32, Sites) {
        if let Some((_, epsg, sites)) = self.read.iter().find(|(known, ..)| known == file) {
            return (*epsg, Arc::clone(sites));
        }
        let (epsg, sites) = read_sites(file).unwrap_or_default();
        let sites = Arc::new(sites);
        self.read.push((file.clone(), epsg, Arc::clone(&sites)));
        (epsg, sites)
    }
}

impl StrataAppState {
    /// The badges of a sheet whose map shows `sheet_box` (`[min x, min y, max x, max y]`, work CRS): the
    /// checked rows of the 「유산」 button, in the order of the legend (the top row first, `:701, 725`).
    pub(crate) fn sheet_heritage_badges(&mut self, sheet_box: [f64; 4]) -> Vec<Badge> {
        let visible = &self.home_map.real_survey;
        let rows: Vec<(String, PathBuf)> = self
            .left_layers
            .entries
            .iter()
            .filter(|entry| entry.layer_key.starts_with(KEY_PREFIX) && visible.is_layer_visible(&entry.label_ko))
            .filter_map(|entry| Some((entry.layer_key.clone(), self.ref_vectors.file_of(&entry.layer_key)?.to_path_buf())))
            .collect();
        if rows.is_empty() {
            self.sheet_numbers.last = None;
            return Vec::new();
        }
        let survey = crate::ref_vector::map::survey_area_wkts(self).unwrap_or_default();
        let asked = Asked { rows, sheet_box, survey, work_epsg: CrsCode::from(self.work_crs).epsg_number() };
        if let Some((known, badges)) = &self.sheet_numbers.last {
            if *known == asked {
                return badges.clone();
            }
        }
        // Copies that are no longer drawn are let go.
        self.sheet_numbers.read.retain(|(file, ..)| asked.rows.iter().any(|(_, used)| used == file));
        let held: Vec<(usize, u32, Sites)> = asked.rows.iter().enumerate().map(|(row, (_, file))| (row, self.sheet_numbers.sites_of(file))).map(|(row, (epsg, sites))| (row, epsg, sites)).collect();
        let layers: Vec<SheetLayer<'_>> = held.iter().filter_map(|(row, epsg, sites)| Some(SheetLayer { kind: dataset_of_key(&asked.rows[*row].0)?.name, epsg: *epsg, sites })).collect();
        let numbered = numbered_area(&asked.survey, sheet_box).ok().flatten().and_then(|area| number_sites(&layers, &area, asked.work_epsg).ok()).unwrap_or_default();
        let badges: Vec<Badge> = numbered
            .into_iter()
            .filter_map(|site| {
                let (key, _) = &asked.rows[held.get(site.layer)?.0];
                let kind = dataset_of_key(key)?;
                Some(Badge { layer_key: key.clone(), kind: kind.name, name: site.name, number: site.number, colour: kind.colour, at: site.at })
            })
            .collect();
        self.sheet_numbers.last = Some((asked, badges.clone()));
        badges
    }
}
