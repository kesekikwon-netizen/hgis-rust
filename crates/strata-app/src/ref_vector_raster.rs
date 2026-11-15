//! The picture of a reference layer for one view of the map.
//!
//! A sheet layer holds some hundred thousand lines. Handing them to the UI toolkit every frame would redo
//! their triangles sixty times a second; QGIS, which draws the C++ app's map, renders a layer into an image
//! in a job and shows that image. The same is done here: this function runs off the UI thread and returns
//! premultiplied RGBA, the map shows it as one textured rectangle until the next picture is ready.
//! Fills go through the general rasteriser, one polygon at a time; lines through `ref_vector_stroke.rs`.
//! Cost: O(shapes) to pick the visible ones, O(their points) to prepare, then the pixels they cover.
//! Measured on 333 390 real sheet lines, picture 3045 x 1589: see docs/RESULTS.md (REF-VEC1).

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

use crate::ref_vector::clip::{clip_ring, clip_segment, ClipBox};
use crate::ref_vector::data::{Item, RefVectorData};
use crate::ref_vector::stroke::{coverage, lay_over, Polylines};

#[cfg(test)]
#[path = "ref_vector_raster_tests.rs"]
mod tests;

/// A point nearer than this to the last drawn point of its line, in both directions, is left out (pixels).
const THIN: f32 = 0.35;
/// Shapes are cut to the picture grown by this many pixels: more than half of any line width.
const GUARD: f64 = 64.0;
/// A shape smaller than this in both directions is not drawn (pixels).
const SPECK: f64 = 0.3;

/// Which part of the world a picture shows. Work CRS metres; picture y grows downwards from `top`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RasterView {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) metres_per_pixel: f64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Picture pixels to one millimetre of line width.
    pub(crate) pixels_per_mm: f32,
}

impl RasterView {
    /// The very same view, also when it is one no picture can be made for (a scale that is no number).
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        let bits = |v: &Self| (v.left.to_bits(), v.top.to_bits(), v.metres_per_pixel.to_bits(), v.width, v.height, v.pixels_per_mm.to_bits());
        bits(self) == bits(other)
    }

    /// `[min x, min y, max x, max y]` of the world the picture covers.
    pub(crate) fn world(&self) -> [f64; 4] {
        let (w, h) = (f64::from(self.width) * self.metres_per_pixel, f64::from(self.height) * self.metres_per_pixel);
        [self.left, self.top - h, self.left + w, self.top]
    }
}

/// A finished picture: premultiplied RGBA, `view.width` x `view.height`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Raster {
    pub(crate) view: RasterView,
    pub(crate) rgba: Vec<u8>,
}

/// World offsets of the layer to picture pixels, in double precision.
struct Mapper {
    dx: f64,
    dy: f64,
    scale: f64,
}

impl Mapper {
    fn px(&self, c: [f32; 2]) -> [f64; 2] {
        [(self.dx + f64::from(c[0])) * self.scale, (self.dy - f64::from(c[1])) * self.scale]
    }
}

/// Collects lines as picture points, leaving out points that would not show.
struct Pen<'a> {
    out: &'a mut Polylines,
    last: [f32; 2],
    held: Option<[f32; 2]>,
    open: bool,
}

impl<'a> Pen<'a> {
    fn new(out: &'a mut Polylines) -> Self {
        Self { out, last: [0.0; 2], held: None, open: false }
    }

    // Picture coordinates were cut to the guard box before: an f32 holds them to a hundredth of a pixel.
    #[allow(clippy::cast_possible_truncation)]
    fn narrow(p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32, p[1] as f32]
    }

    fn start(&mut self, p: [f64; 2]) {
        self.end();
        self.last = Self::narrow(p);
        self.out.starts.push(u32::try_from(self.out.points.len()).unwrap_or(u32::MAX));
        self.out.points.push(self.last);
        self.open = true;
    }

    fn to(&mut self, p: [f64; 2]) {
        let q = Self::narrow(p);
        if (q[0] - self.last[0]).abs() >= THIN || (q[1] - self.last[1]).abs() >= THIN {
            self.out.points.push(q);
            self.last = q;
            self.held = None;
        } else {
            self.held = Some(q);
        }
    }

    /// Ends the line; the last point given is always kept, however near it lies.
    fn end(&mut self) {
        if let Some(q) = self.held.take() {
            self.out.points.push(q);
        }
        self.open = false;
    }

    /// One line, or one ring with the side that closes it. `cut`: the shape reaches beyond the guard box.
    fn trace(&mut self, points: &[[f64; 2]], closed: bool, cut: Option<&ClipBox>) {
        // A ring stored with its first point again at the end has that point once here.
        let points = if closed && points.len() > 1 && points.first() == points.last() { &points[..points.len() - 1] } else { points };
        let Some(first) = points.first() else {
            return;
        };
        let closing = closed.then_some((points[points.len() - 1], *first));
        let Some(guard) = cut else {
            self.start(*first);
            points[1..].iter().for_each(|p| self.to(*p));
            if let Some((_, back)) = closing {
                self.to(back);
            }
            self.end();
            return;
        };
        for (a, z) in points.windows(2).map(|w| (w[0], w[1])).chain(closing) {
            match clip_segment(guard, a, z) {
                None => self.end(),
                Some((from, to)) => {
                    if !self.open || from != a {
                        self.start(from);
                    }
                    self.to(to);
                    if to != z {
                        self.end();
                    }
                }
            }
        }
        self.end();
    }
}

/// One view of a layer: how its points map to the picture, the box shapes are cut to, and the shapes that
/// reach into that box, each with whether it reaches beyond it.
struct Frame<'a> {
    data: &'a RefVectorData,
    map: Mapper,
    guard: ClipBox,
    shown: Vec<(&'a Item, bool)>,
}

impl<'a> Frame<'a> {
    fn new(data: &'a RefVectorData, view: &RasterView) -> Self {
        let map = Mapper { dx: data.origin[0] - view.left, dy: view.top - data.origin[1], scale: 1.0 / view.metres_per_pixel };
        let guard: ClipBox = [-GUARD, -GUARD, f64::from(view.width) + GUARD, f64::from(view.height) + GUARD];
        let mut shown = Vec::new();
        for item in &data.items {
            let (a, z) = (map.px([item.bbox[0], item.bbox[3]]), map.px([item.bbox[2], item.bbox[1]]));
            let apart = z[0] < guard[0] || a[0] > guard[2] || z[1] < guard[1] || a[1] > guard[3];
            if !apart && (z[0] - a[0] >= SPECK || z[1] - a[1] >= SPECK) {
                shown.push((item, a[0] < guard[0] || z[0] > guard[2] || a[1] < guard[1] || z[1] > guard[3]));
            }
        }
        Self { data, map, guard, shown }
    }

    fn points_of(&self, run: usize) -> Vec<[f64; 2]> {
        let r = self.data.runs[run];
        self.data.coords[r.start as usize..(r.start + r.len) as usize].iter().map(|c| self.map.px(*c)).collect()
    }

    /// The lines of every shown shape that is drawn with this colour and width, as picture points. All of
    /// them lie inside the guard box: the line drawer walks a line in steps of a few pixels, and a side that
    /// runs on for millions of pixels beside the picture must not cost millions of steps.
    fn lines_of(&self, key: ([u8; 4], u32)) -> Polylines {
        let mut lines = Polylines::default();
        let mut pen = Pen::new(&mut lines);
        for (item, cut) in &self.shown {
            let of = self.data.paints[item.paint as usize];
            if (of.stroke, of.width_mm.to_bits()) == key {
                for run in runs_of(item) {
                    pen.trace(&self.points_of(run), self.data.closed, cut.then_some(&self.guard));
                }
            }
        }
        lines
    }
}

fn runs_of(item: &Item) -> std::ops::Range<usize> {
    item.first_run as usize..(item.first_run + item.run_count) as usize
}

/// Renders the layer for the view. `None` when the view has no pixels or no scale.
pub(crate) fn render(data: &RefVectorData, view: RasterView) -> Option<Raster> {
    if !view.metres_per_pixel.is_finite() || view.metres_per_pixel <= 0.0 {
        return None;
    }
    let mut pixmap = Pixmap::new(view.width, view.height)?;
    let frame = Frame::new(data, &view);

    // Fills first, each polygon on its own: two polygons of one colour that overlap must both be filled.
    if data.closed {
        for (item, cut) in &frame.shown {
            let fill = data.paints[item.paint as usize].fill;
            if fill[3] == 0 {
                continue;
            }
            let mut rings = Polylines::default();
            let mut pen = Pen::new(&mut rings);
            for run in runs_of(item) {
                let ring = if *cut { clip_ring(&frame.guard, &frame.points_of(run)) } else { frame.points_of(run) };
                pen.trace(&ring, false, None);
            }
            let mut path = PathBuilder::new();
            for ring in rings.lines().filter(|ring| ring.len() >= 3) {
                path.move_to(ring[0][0], ring[0][1]);
                ring[1..].iter().for_each(|p| path.line_to(p[0], p[1]));
                path.close();
            }
            if let Some(path) = path.finish() {
                let mut paint = Paint::default();
                paint.set_color_rgba8(fill[0], fill[1], fill[2], fill[3]);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
            }
        }
    }
    let mut rgba = pixmap.take();

    // Then the lines: every look that shares a colour and a width goes into one mask.
    let (width, height) = (view.width as usize, view.height as usize);
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get).min(16);
    let mut done: Vec<([u8; 4], u32)> = Vec::new();
    for look in &data.paints {
        let key = (look.stroke, look.width_mm.to_bits());
        if look.stroke[3] == 0 || done.contains(&key) {
            continue;
        }
        done.push(key);
        let mask = coverage(&frame.lines_of(key), width, height, look.width_mm * view.pixels_per_mm, threads);
        lay_over(&mut rgba, &mask, look.stroke);
    }
    Some(Raster { view, rgba })
}
