//! The numbers of the heritage sites on a sheet (`target/cpp-ref/src/core/HeritageLayoutNumbers.cpp`): the
//! rows of the 「유산」 button that are checked in the legend are numbered for the map of the sheet, and the
//! sheet draws a badge on every numbered site. The rule is `strata_refmap::heritage_numbers`.

use std::path::PathBuf;
use std::sync::Arc;

use strata_layout::badge_rings::{BadgePlacer, Paper, Rings, ToPaper};
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
/// The paper left between two badges (`kNumberGapMm`).
const NUMBER_GAP_MM: f64 = 0.3;
/// Names take the place of numbers at this scale and larger (`kNamesMaxScale`).
const NAMES_MAX_SCALE: f64 = 5000.0;
/// A site's name on the sheet: black letters of 2.2 mm with a white edge, no box (`kNameSizeMm`,
/// `kNameHaloMm`, `:112-115`). The edge is a stroke of 0.5 mm along the letters: half of it lies outside.
pub(crate) const NAME_MM: f32 = 2.2;
pub(crate) const NAME_HALO_MM: f32 = 0.5;

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
    /// The point of the site the number belongs to, in the work CRS.
    pub(crate) at: [f64; 2],
    /// Where the badge stands: on that point, or the free place it was moved to (`place_badges`).
    pub(crate) placed: [f64; 2],
    /// The ground of the site on the sheet: a badge that must move looks for a free place on it first.
    pub(crate) ground: Vec<Rings>,
}

/// The paper of a sheet for the placing of its badges: how map coordinates become millimetres of paper,
/// the visible map frame and the legend cards over it (millimetres), and the scale of the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SheetPaper {
    pub(crate) to_paper: ToPaper,
    pub(crate) frame: [f64; 4],
    pub(crate) blocks: Vec<[f64; 4]>,
    pub(crate) scale: f64,
    /// With names in place of numbers: how wide the letters of each site's name are, in millimetres.
    pub(crate) names: Option<Vec<f64>>,
}

/// Whether names stand in place of the numbers: the sheet asks for them and its scale is 1:5,000 or
/// larger; a shown 1:5,000 may be 5000.3 in the map (`HeritageLayoutNumbers.h:66`, `.cpp:630-632`).
pub(crate) fn names_shown(scale: f64, wanted: bool) -> bool {
    wanted && scale > 0.0 && scale < NAMES_MAX_SCALE + 0.5
}

/// The paper a name takes, in millimetres, from the width of its letters: the letters and their white edge
/// (`nameBoxMm`, `:117-123`).
pub(crate) fn name_box_mm(letters_mm: f64) -> [f64; 2] {
    [letters_mm + f64::from(NAME_HALO_MM), f64::from(NAME_MM) * 1.15 + f64::from(NAME_HALO_MM)]
}

/// The kinds that have a site on the sheet, each once in the order of the sites, with their colours: with
/// names on the map the legend lists the kinds and no site (`:1207, 1281-1293`). The C++ makes a row for
/// every layer, so two layers of one kind repeat the row; here a kind is listed once, as the check says
/// (「범례에는 유적 종류만 남깁니다」, `KaDrawingStudio.cpp:975-989`).
pub(crate) fn legend_kinds(badges: &[Badge]) -> Vec<(&'static str, [u8; 3])> {
    let mut kinds: Vec<(&'static str, [u8; 3])> = Vec::new();
    for badge in badges {
        if !kinds.iter().any(|(kind, _)| *kind == badge.kind) {
            kinds.push((badge.kind, badge.colour));
        }
    }
    kinds
}

/// How many rows the sites take in the legend: one for each site, or with names one for each kind. The
/// legend and the placing of the badges both count by this (the badges keep off the legend's card).
pub(crate) fn legend_site_rows(badges: &[Badge], names: bool) -> usize {
    if names {
        legend_kinds(badges).len()
    } else {
        badges.len()
    }
}

/// Moves the badges, or the names in their place, that would cover each other, lie off the map frame or
/// under a legend card (`HeritageLayoutNumbers.cpp:968-995`).
pub(crate) fn place_badges(badges: &mut [Badge], paper: &SheetPaper) {
    let scale = if paper.scale > 0.0 { paper.scale } else { 5000.0 };
    let (origins, grounds): (Vec<[f64; 2]>, Vec<Vec<Rings>>) = badges.iter().map(|badge| (badge.at, badge.ground.clone())).unzip();
    let allowed = Paper { frame: paper.frame, blocks: paper.blocks.clone() };
    let placed = if let Some(letters) = paper.names.as_ref().filter(|letters| letters.len() == badges.len()) {
        // Every name keeps off every other by its own box and the gap; names step by their height (`:972-981`).
        let boxes: Vec<[f64; 2]> = letters.iter().map(|wide| name_box_mm(*wide).map(|side| side + NUMBER_GAP_MM)).collect();
        let step = (name_box_mm(0.0)[1] + NUMBER_GAP_MM) / 1000.0 * scale;
        BadgePlacer::new(paper.to_paper, Some(allowed), step).place_all_boxes(&origins, &boxes, &grounds)
    } else {
        let diameters: Vec<f64> = badges.iter().map(|badge| layout_badge_circle_size_mm(badge.number as usize)).collect();
        // Badge centres stay the sheet's widest badge and a gap apart on paper (`:186-192, 983-990`).
        let step = (diameters.iter().copied().fold(0.0, f64::max) + NUMBER_GAP_MM) / 1000.0 * scale;
        BadgePlacer::new(paper.to_paper, Some(allowed), step).place_all(&origins, &diameters, &grounds)
    };
    for (badge, at) in badges.iter_mut().zip(placed) {
        badge.placed = at;
    }
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
    /// The badges as they were numbered, the paper they were placed on, and where they stand.
    placed: Option<(Vec<Badge>, SheetPaper, Vec<Badge>)>,
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
                Some(Badge { layer_key: key.clone(), kind: kind.name, name: site.name, number: site.number, colour: kind.colour, at: site.at, placed: site.at, ground: site.pieces })
            })
            .collect();
        self.sheet_numbers.last = Some((asked, badges.clone()));
        badges
    }

    /// The same badges, each where it stands on the paper of the sheet: placed anew only when the numbers
    /// or the paper changed.
    pub(crate) fn sheet_heritage_badges_placed(&mut self, sheet_box: [f64; 4], paper: SheetPaper) -> Vec<Badge> {
        let numbered = self.sheet_heritage_badges(sheet_box);
        if let Some((known, on, placed)) = &self.sheet_numbers.placed {
            if *known == numbered && *on == paper {
                return placed.clone();
            }
        }
        let mut placed = numbered.clone();
        place_badges(&mut placed, &paper);
        self.sheet_numbers.placed = Some((numbered, paper, placed.clone()));
        placed
    }
}
