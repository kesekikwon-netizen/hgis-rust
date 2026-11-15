//! Where the number badges of a sheet stand (`target/cpp-ref/src/core/LayoutBadgePlacer.cpp`, the whole
//! file): no two badges cover each other, each lies wholly on the visible map paper and off the legend
//! card. A badge stays on its site; one that cannot moves to the nearest free place on rings around it, a
//! free place inside its own site first. Coordinates are those of the map; the paper is in millimetres.

use std::collections::HashMap;

#[cfg(test)]
#[path = "badge_rings_tests.rs"]
mod tests;
#[path = "badge_rings_site.rs"]
mod site;

use site::Site;

/// Centres closer than this share of a step cover each other (`kClearShare`).
const CLEAR_SHARE: f64 = 0.96;
/// The outline of the circle and a hair of paper stay inside the frame (`kEdgeMarginMm`).
const EDGE_MARGIN_MM: f64 = 0.3;
const BASE_RINGS: i32 = 6;
const MAX_RINGS: i32 = 60;

/// The rings of a polygon, the outer one first.
pub type Rings = Vec<Vec<[f64; 2]>>;

/// Map coordinates to millimetres of paper: `paper = map × scale + shift` on each axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToPaper {
    pub scale: [f64; 2],
    pub shift: [f64; 2],
}

impl ToPaper {
    /// Map units are millimetres of paper.
    pub const IDENTITY: Self = Self { scale: [1.0, 1.0], shift: [0.0, 0.0] };

    fn map(&self, point: [f64; 2]) -> [f64; 2] {
        [point[0] * self.scale[0] + self.shift[0], point[1] * self.scale[1] + self.shift[1]]
    }
}

/// The paper the badges may stand on: the visible map frame, without the legend cards over it
/// (`[min x, min y, max x, max y]` in millimetres).
#[derive(Debug, Clone, PartialEq)]
pub struct Paper {
    pub frame: [f64; 4],
    pub blocks: Vec<[f64; 4]>,
}

impl Paper {
    /// Whether a square of half side `half` around `centre` lies wholly on the paper.
    fn holds(&self, centre: [f64; 2], half: [f64; 2]) -> bool {
        let [left, top, right, bottom] = [centre[0] - half[0], centre[1] - half[1], centre[0] + half[0], centre[1] + half[1]];
        let [min_x, min_y, max_x, max_y] = self.frame;
        left >= min_x && top >= min_y && right <= max_x && bottom <= max_y && self.blocks.iter().all(|block| left >= block[2] || right <= block[0] || top >= block[3] || bottom <= block[1])
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The placer of one sheet.
#[derive(Debug)]
pub struct BadgePlacer {
    to_paper: ToPaper,
    allowed: Option<Paper>,
    step: f64,
    units_per_mm: f64,
    reach_rings: i32,
    cells: HashMap<(i64, i64), Vec<[f64; 2]>>,
    count: i32,
    /// Name boxes (millimetres), when names are placed instead of circles; `current` is the one in hand.
    boxes_mm: Vec<[f64; 2]>,
    placed_boxes: Vec<[f64; 4]>,
    current: usize,
}

impl BadgePlacer {
    /// `step` is the distance of two badge centres in map units; `allowed: None` is paper without limit.
    #[must_use]
    pub fn new(to_paper: ToPaper, allowed: Option<Paper>, step: f64) -> Self {
        let step = if step.is_finite() && step > 0.0 { step } else { 1.0 };
        let step_mm = distance(to_paper.map([0.0, 0.0]), to_paper.map([step, 0.0]));
        let units_per_mm = if step_mm > 1e-9 { step / step_mm } else { 1.0 };
        // On a limited paper the search reaches across it (`:30-34`).
        let reach_rings = allowed.as_ref().map_or(0, |paper| {
            let reach_mm = (paper.frame[2] - paper.frame[0]).hypot(paper.frame[3] - paper.frame[1]);
            // A count of rings: small and never negative.
            #[allow(clippy::cast_possible_truncation)]
            let rings = if step_mm > 1e-9 && reach_mm.is_finite() { (reach_mm / step_mm).ceil() as i32 + 1 } else { 0 };
            if rings == 0 { 0 } else { rings.clamp(1, MAX_RINGS) }
        });
        Self { to_paper, allowed, step, units_per_mm, reach_rings, cells: HashMap::new(), count: 0, boxes_mm: Vec::new(), placed_boxes: Vec::new(), current: 0 }
    }

    /// The places on ring `ring` whose neighbours are one step apart (four at least).
    #[must_use]
    pub fn slots_on_ring(ring: i32) -> i32 {
        let half = (1.0 / (2.0 * f64::from(ring.max(1)))).min(0.999);
        // About six places a ring: far from the limits of i32.
        #[allow(clippy::cast_possible_truncation)]
        let count = (std::f64::consts::PI / half.asin() + 1e-6).floor() as i32;
        count.max(4)
    }

    // The cell of a point: coordinates of a map divided by a step of metres stay far inside i64.
    #[allow(clippy::cast_possible_truncation)]
    fn cell_of(&self, point: [f64; 2]) -> (i64, i64) {
        ((point[0] / self.step).floor() as i64, (point[1] / self.step).floor() as i64)
    }

    fn box_at(&self, point: [f64; 2]) -> [f64; 4] {
        let [width, height] = self.boxes_mm.get(self.current).map_or([0.0, 0.0], |size| [size[0] * self.units_per_mm, size[1] * self.units_per_mm]);
        [point[0] - width * 0.5, point[1] - height * 0.5, point[0] + width * 0.5, point[1] + height * 0.5]
    }

    fn is_free(&self, point: [f64; 2]) -> bool {
        if !self.boxes_mm.is_empty() {
            let mine = self.box_at(point);
            return self.placed_boxes.iter().all(|other| mine[0] >= other[2] || other[0] >= mine[2] || mine[1] >= other[3] || other[1] >= mine[3]);
        }
        let (cx, cy) = self.cell_of(point);
        let clear = self.step * CLEAR_SHARE;
        (-1..=1).all(|dx| (-1..=1).all(|dy| self.cells.get(&(cx + dx, cy + dy)).is_none_or(|placed| placed.iter().all(|other| distance(point, *other) >= clear))))
    }

    fn fits(&self, point: [f64; 2], diameter_mm: f64) -> bool {
        let Some(paper) = &self.allowed else {
            return true;
        };
        let centre = self.to_paper.map(point);
        if !self.boxes_mm.is_empty() {
            let [width, height] = self.boxes_mm.get(self.current).copied().unwrap_or([0.0, 0.0]);
            return paper.holds(centre, [width * 0.5 + EDGE_MARGIN_MM, height * 0.5 + EDGE_MARGIN_MM]);
        }
        let radius = (diameter_mm * 0.5).max(0.5) + EDGE_MARGIN_MM;
        paper.holds(centre, [radius, radius])
    }

    fn occupy(&mut self, point: [f64; 2]) {
        let cell = self.cell_of(point);
        self.cells.entry(cell).or_default().push(point);
        if !self.boxes_mm.is_empty() {
            let placed = self.box_at(point);
            self.placed_boxes.push(placed);
        }
        self.count += 1;
    }

    /// The centre chosen for a badge; it is occupied afterwards (`place`, `:92-169`).
    fn place(&mut self, origin: [f64; 2], diameter_mm: f64, site: Option<&Site>) -> [f64; 2] {
        // With a site area, the nearest free place off the site is only kept until one inside turns up;
        // a free place off the paper is the last resort.
        let (mut first_free, mut nearest_outside): (Option<[f64; 2]>, Option<[f64; 2]>) = (None, None);
        let mut chosen = None;
        let mut consider = |placer: &Self, candidate: [f64; 2], off_site: &mut Option<[f64; 2]>| {
            if !placer.is_free(candidate) {
                return false;
            }
            if !placer.fits(candidate, diameter_mm) {
                first_free.get_or_insert(candidate);
                return false;
            }
            if site.is_none_or(|site| site.holds(candidate)) {
                return true;
            }
            off_site.get_or_insert(candidate);
            false
        };
        if consider(self, origin, &mut nearest_outside) {
            chosen = Some(origin);
        }
        // Each placed badge blocks a few places of a ring, and ring n holds about 2πn: this many rings
        // always have a free place.
        let mut rings = MAX_RINGS.min((BASE_RINGS + self.count).max(self.reach_rings));
        let site_reach = site.map_or(0.0, |site| site.reach_from(origin));
        if site.is_some() {
            // A count of rings: small and never negative.
            #[allow(clippy::cast_possible_truncation)]
            let to_the_far_corner = (site_reach / self.step).ceil() as i32;
            rings = MAX_RINGS.min(rings.max(to_the_far_corner));
        }
        let mut ring = 1;
        while chosen.is_none() && ring <= rings {
            // Places off the site are only worth taking near it.
            if site.is_some() && nearest_outside.is_some() && self.step * f64::from(ring) > site_reach {
                break;
            }
            let (count, radius) = (Self::slots_on_ring(ring), self.step * f64::from(ring));
            let mut slot = 0;
            while slot < count && chosen.is_none() {
                let angle = f64::from(slot) * (2.0 * std::f64::consts::PI / f64::from(count));
                let candidate = [origin[0] + angle.cos() * radius, origin[1] + angle.sin() * radius];
                if consider(self, candidate, &mut nearest_outside) {
                    chosen = Some(candidate);
                }
                slot += 1;
            }
            ring += 1;
        }
        // No place is both clear and wholly on the paper: the circles stay apart all the same.
        let chosen = chosen.or(nearest_outside).or(first_free).unwrap_or([origin[0], origin[1] + self.step * f64::from(rings + 1)]);
        self.occupy(chosen);
        chosen
    }

    /// Places every badge of the sheet at once, in the order given (the order of the numbers). Lone sites
    /// go first, so that no crowd takes a site's own place; of the crowded ones those with the smallest
    /// areas choose first; then badges trade places until no two lines from site to badge cross
    /// (`placeAll`, `:171-253`). `site_areas` holds the pieces of each site, or nothing for a site.
    pub fn place_all(&mut self, origins: &[[f64; 2]], diameters_mm: &[f64], site_areas: &[Vec<Rings>]) -> Vec<[f64; 2]> {
        let count = origins.len();
        let sites: Vec<Option<Site>> = (0..count).map(|i| site_areas.get(i).and_then(|pieces| Site::of(pieces))).collect();
        let diameter = |i: usize| diameters_mm.get(i).copied().unwrap_or(0.0);
        // Crowds: sites chained closer than the clear distance.
        let mut root: Vec<usize> = (0..count).collect();
        fn find(root: &mut [usize], mut i: usize) -> usize {
            while root[i] != i {
                root[i] = root[root[i]];
                i = root[i];
            }
            i
        }
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for i in 0..count {
            let (cx, cy) = self.cell_of(origins[i]);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for j in cells.get(&(cx + dx, cy + dy)).cloned().unwrap_or_default() {
                        if distance(origins[i], origins[j]) < self.step * CLEAR_SHARE {
                            let (mine, theirs) = (find(&mut root, i), find(&mut root, j));
                            root[mine] = theirs;
                        }
                    }
                }
            }
            cells.entry((cx, cy)).or_default().push(i);
        }
        let mut crowd_size = vec![0_usize; count];
        for i in 0..count {
            crowd_size[find(&mut root, i)] += 1;
        }
        let lone: Vec<bool> = (0..count).map(|i| crowd_size[find(&mut root, i)] == 1).collect();
        let mut placed = origins.to_vec();
        for i in (0..count).filter(|i| lone[*i]) {
            self.current = i;
            placed[i] = self.place(origins[i], diameter(i), sites[i].as_ref());
        }
        // Small areas have the fewest places on their site, so they choose first; sites without an area
        // before those with one.
        let mut crowded: Vec<usize> = (0..count).filter(|i| !lone[*i]).collect();
        crowded.sort_by(|a, b| match (&sites[*a], &sites[*b]) {
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.area.total_cmp(&b.area),
            (None, None) => std::cmp::Ordering::Equal,
        });
        for i in crowded {
            self.current = i;
            placed[i] = self.place(origins[i], diameter(i), sites[i].as_ref());
        }
        // The length of the line to a place: none inside the badge's own area.
        let cost = |i: usize, at: [f64; 2]| if sites[i].as_ref().is_some_and(|site| site.holds(at)) { 0.0 } else { distance(origins[i], at) };
        // Two badges trade places whenever that shortens their lines; crossing lines always get shorter
        // by trading. Name boxes differ in width and keep their places.
        let mut traded = self.boxes_mm.is_empty();
        let mut pass = 0;
        while traded && pass < MAX_RINGS {
            traded = false;
            for a in 0..count {
                for b in a + 1..count {
                    let now = cost(a, placed[a]) + cost(b, placed[b]);
                    if now <= self.step * 1e-6 {
                        continue;
                    }
                    let then = cost(a, placed[b]) + cost(b, placed[a]);
                    if then >= now - self.step * 1e-6 || !self.fits(placed[b], diameter(a)) || !self.fits(placed[a], diameter(b)) {
                        continue;
                    }
                    placed.swap(a, b);
                    traded = true;
                }
            }
            pass += 1;
        }
        placed
    }

    /// The same for name labels: each box (width, height in millimetres) is kept off every other box
    /// instead of a circle step (`placeAllBoxes`, `:255-262`).
    pub fn place_all_boxes(&mut self, origins: &[[f64; 2]], boxes_mm: &[[f64; 2]], site_areas: &[Vec<Rings>]) -> Vec<[f64; 2]> {
        self.boxes_mm = boxes_mm.to_vec();
        let widest: Vec<f64> = boxes_mm.iter().map(|size| size[0].max(size[1])).collect();
        self.place_all(origins, &widest, site_areas)
    }
}
