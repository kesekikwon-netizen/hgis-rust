//! The coordinate points of a sheet — C++ `KaDrawingCoordCallout` (KaDrawingCoordCallout.cpp:31-134) and the
//! magnet of `placeCoordCallout` (KaDrawingStudio.cpp:1704-1779): the geometry on the paper in millimetres
//! (from its top-left corner, y down) and the words of a point.

/// A box on the paper, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BoxMm {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}

impl BoxMm {
    fn grown(self, by: f64) -> Self {
        Self { x: self.x - by, y: self.y - by, w: self.w + 2.0 * by, h: self.h + 2.0 * by }
    }

    fn meets(self, other: Self) -> bool {
        self.x < other.x + other.w && other.x < self.x + self.w && self.y < other.y + other.h && other.y < self.y + self.h
    }
}

/// The arrow of a point (a shaft and a head) and the place of its letter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CalloutShape {
    pub(crate) shaft: [[f64; 2]; 2],
    pub(crate) head: [[f64; 2]; 3],
    pub(crate) letter: BoxMm,
}

/// The size of a box (`:39-44`, `:46-49`).
const BOX_W: f64 = 26.0;
const BOX_H: f64 = 7.2;
/// The length of the head of an arrow, and how wide it is on each side of its axis (`:113-114`, `:131-133`).
const HEAD: f64 = 2.4;

/// The letter of the point at `index`: A to Z, and again (`tagFor`, `:31-33`).
pub(crate) fn tag_for(index: usize) -> char {
    // `index % 26` is below 26: it fits a u8.
    #[allow(clippy::cast_possible_truncation)]
    let step = (index % 26) as u8;
    char::from(b'A' + step)
}

/// The words of a point: X the northing and Y the easting, three decimals (`xyText`, `:35-37`).
pub(crate) fn xy_text(map_x: f64, map_y: f64) -> String {
    format!("X={map_y:.3}\nY={map_x:.3}")
}

/// The box of a point among the boxes `used`: the first of six places that keeps 2 mm off them, else the
/// first (`freeBoxRect`, `:46-69`). The points are laid out again in every frame from their ground, as the
/// C++ lays them out again whenever the map changes (`relayoutCoordCallouts`).
pub(crate) fn free_box_rect(tip: [f64; 2], used: &[BoxMm]) -> BoxMm {
    let gap = 8.0;
    let [x, y] = tip;
    let places = [
        BoxMm { x: x + gap, y: y - BOX_H - 4.0, w: BOX_W, h: BOX_H },
        BoxMm { x: x + gap, y: y + 4.0, w: BOX_W, h: BOX_H },
        BoxMm { x: x - gap - BOX_W, y: y - BOX_H - 4.0, w: BOX_W, h: BOX_H },
        BoxMm { x: x - gap - BOX_W, y: y + 4.0, w: BOX_W, h: BOX_H },
        BoxMm { x: x - BOX_W * 0.5, y: y - BOX_H - gap, w: BOX_W, h: BOX_H },
        BoxMm { x: x - BOX_W * 0.5, y: y + gap, w: BOX_W, h: BOX_H },
    ];
    places.into_iter().find(|place| !used.iter().any(|other| place.grown(2.0).meets(*other))).unwrap_or(places[0])
}

/// The arrow from the near side of the box to the point, and the letter above the point (`add`, `:88-133`).
pub(crate) fn callout_shape(tip: [f64; 2], box_mm: BoxMm) -> CalloutShape {
    let right = box_mm.x + box_mm.w * 0.5 >= tip[0];
    let attach = [if right { box_mm.x } else { box_mm.x + box_mm.w }, box_mm.y + box_mm.h * 0.5];
    let (dx, dy) = (tip[0] - attach[0], tip[1] - attach[1]);
    let length = dx.hypot(dy);
    let dir = if length > 0.4 { [dx / length, dy / length] } else { [if right { -1.0 } else { 1.0 }, 0.0] };
    let normal = [-dir[1], dir[0]];
    let neck = [tip[0] - dir[0] * HEAD, tip[1] - dir[1] * HEAD];
    let side = HEAD * 0.42;
    CalloutShape {
        shaft: [attach, neck],
        head: [tip, [neck[0] + normal[0] * side, neck[1] + normal[1] * side], [neck[0] - normal[0] * side, neck[1] - normal[1] * side]],
        letter: BoxMm { x: tip[0] - 2.4, y: tip[1] - 5.4, w: 5.2, h: 4.2 },
    }
}

/// Whether a place on the paper is on the map frame, 0.8 mm around it counting (`layoutMapItemContains`,
/// LayoutService.cpp:1017-1020).
pub(crate) fn inside_map_frame(at: [f64; 2], frame: BoxMm) -> bool {
    let grown = frame.grown(0.8);
    at[0] >= grown.x && at[0] <= grown.x + grown.w && at[1] >= grown.y && at[1] <= grown.y + grown.h
}

/// The nearest place to `click` on the `lines` (a ring is a closed line) or among the `points`, nearer than
/// `reach`; all in metres of the work CRS (`closestVertex` / `closestSegmentWithContext`, `:1756-1778`).
pub(crate) fn nearest_on(click: [f64; 2], lines: &[Vec<[f64; 2]>], points: &[[f64; 2]], reach: f64) -> Option<[f64; 2]> {
    let mut best: Option<([f64; 2], f64)> = None;
    let mut consider = |at: [f64; 2]| {
        let away = (at[0] - click[0]).hypot(at[1] - click[1]);
        if away < reach && best.is_none_or(|(_, nearest)| away < nearest) {
            best = Some((at, away));
        }
    };
    for line in lines {
        for side in line.windows(2) {
            let ([ax, ay], [bx, by]) = (side[0], side[1]);
            let (sx, sy) = (bx - ax, by - ay);
            let squared = sx * sx + sy * sy;
            let along = if squared > 0.0 { (((click[0] - ax) * sx + (click[1] - ay) * sy) / squared).clamp(0.0, 1.0) } else { 0.0 };
            consider([ax + sx * along, ay + sy * along]);
        }
    }
    points.iter().for_each(|point| consider(*point));
    best.map(|(at, _)| at)
}

#[cfg(test)]
#[path = "sheet_coord_points_tests.rs"]
mod tests;
