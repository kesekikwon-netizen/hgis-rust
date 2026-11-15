//! The fit of 맞추기 (정합): from point pairs — a spot on the drawing (the picture's pixels, or a CAD drawing's
//! units) and the same spot on the map — to the transform that carries the drawing onto the map, and the words
//! that say how well the points agree. C++ `GeorefService` (`target/cpp-ref/src/core/GeorefService.cpp:43-176`)
//! and `GeorefQuality` (`GeorefQuality.cpp:8-94`, constants `GeorefQuality.h:14-18`): two pairs give a
//! similarity (Helmert) — with the picture's rows turned upside down, since they grow downwards — three or more an
//! affine by least squares on centred coordinates, falling back to the two-pair similarity for points in a line.

#[cfg(test)]
#[path = "align_fit_tests.rs"]
mod tests;

/// A residual above this is pointed at (`kResidualWarn`).
pub const RESIDUAL_WARN_M: f64 = 1.0;
/// Scales of the two axes differing by more than this are warned about (`kAnisotropyWarn`).
pub const ANISOTROPY_WARN: f64 = 1.05;
/// Axes of the drawing more than this many degrees off square are warned about (`kShearWarnDeg`).
pub const SHEAR_WARN_DEG: f64 = 3.0;
/// A flat transform has no inverse (`kDetEps`).
const DET_EPS: f64 = 1e-18;
/// Points in a line: the centred normal matrix is singular relative to its size (`kRelDetEps`).
const REL_DET_EPS: f64 = 1e-10;

/// One pair: the spot on the drawing and the same spot on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignPair {
    pub src: [f64; 2],
    pub map: [f64; 2],
}

/// `map = (a·sx + b·sy + c, d·sx + e·sy + f)`, with the number of pairs it was fitted to and their RMS (map units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignAffine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    pub pair_count: usize,
    pub rms_m: f64,
}

impl AlignAffine {
    /// Where a spot of the drawing lies on the map.
    #[must_use]
    pub fn apply(&self, s: [f64; 2]) -> [f64; 2] {
        [self.a * s[0] + self.b * s[1] + self.c, self.d * s[0] + self.e * s[1] + self.f]
    }

    /// The spot of the drawing under a point of the map; none for a flat transform.
    #[must_use]
    pub fn invert(&self, m: [f64; 2]) -> Option<[f64; 2]> {
        let det = self.a * self.e - self.b * self.d;
        if det.abs() < DET_EPS {
            return None;
        }
        let (x, y) = (m[0] - self.c, m[1] - self.f);
        Some([(self.e * x - self.b * y) / det, (-self.d * x + self.a * y) / det])
    }
}

fn helmert_from_two(p0: &AlignPair, p1: &AlignPair, source_y_down: bool) -> Option<AlignAffine> {
    let ky = if source_y_down { -1.0 } else { 1.0 };
    let (dsx, dsy) = (p1.src[0] - p0.src[0], ky * (p1.src[1] - p0.src[1]));
    let (dmx, dmy) = (p1.map[0] - p0.map[0], p1.map[1] - p0.map[1]);
    let (len_s, len_m) = (dsx.hypot(dsy), dmx.hypot(dmy));
    if len_s < 1e-12 || len_m < 1e-12 {
        return None;
    }
    let scale = len_m / len_s;
    let angle = dmy.atan2(dmx) - dsy.atan2(dsx);
    let (cos, sin) = (angle.cos(), angle.sin());
    // The rotation works on (sx, ky·sy); ky is folded into the matrix.
    let (a, b, d, e) = (scale * cos, -scale * sin * ky, scale * sin, scale * cos * ky);
    let (c, f) = (p0.map[0] - a * p0.src[0] - b * p0.src[1], p0.map[1] - d * p0.src[0] - e * p0.src[1]);
    Some(AlignAffine { a, b, c, d, e, f, pair_count: 2, rms_m: 0.0 })
}

fn affine_least_squares(pairs: &[AlignPair]) -> Option<AlignAffine> {
    if pairs.len() < 3 {
        return None;
    }
    // A count of clicked points: far below where f64 loses whole numbers.
    #[allow(clippy::cast_precision_loss)]
    let n = pairs.len() as f64;
    let mean = |pick: fn(&AlignPair) -> f64| pairs.iter().map(pick).sum::<f64>() / n;
    let (mx, my, m_x, m_y) = (mean(|g| g.src[0]), mean(|g| g.src[1]), mean(|g| g.map[0]), mean(|g| g.map[1]));
    let (mut sxx, mut sxy, mut syy, mut sx_x, mut sy_x, mut sx_y, mut sy_y) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for g in pairs {
        let (x, y, big_x, big_y) = (g.src[0] - mx, g.src[1] - my, g.map[0] - m_x, g.map[1] - m_y);
        sxx += x * x;
        sxy += x * y;
        syy += y * y;
        sx_x += x * big_x;
        sy_x += y * big_x;
        sx_y += x * big_y;
        sy_y += y * big_y;
    }
    let det = sxx * syy - sxy * sxy;
    if !(sxx > 0.0 && syy > 0.0) || det.abs() <= REL_DET_EPS * sxx * syy {
        return None;
    }
    let (a, b) = ((sx_x * syy - sy_x * sxy) / det, (sy_x * sxx - sx_x * sxy) / det);
    let (d, e) = ((sx_y * syy - sy_y * sxy) / det, (sy_y * sxx - sx_y * sxy) / det);
    let (c, f) = (m_x - a * mx - b * my, m_y - d * mx - e * my);
    Some(AlignAffine { a, b, c, d, e, f, pair_count: pairs.len(), rms_m: 0.0 })
}

fn rms_of(a: &AlignAffine, pairs: &[AlignPair]) -> f64 {
    if pairs.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = pairs.len() as f64;
    let sum: f64 = pairs.iter().map(|p| off(a.apply(p.src), p.map).powi(2)).sum();
    (sum / n).sqrt()
}

fn off(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// `fromPairs`: none for fewer than two pairs or a placement no transform can be made from.
#[must_use]
pub fn align_from_pairs(pairs: &[AlignPair], source_y_down: bool) -> Option<AlignAffine> {
    let fit = match pairs {
        [] | [_] => None,
        [p0, p1] => helmert_from_two(p0, p1, source_y_down),
        // Three or more: the affine carries the flip in the data itself; points in a line fall back to two.
        [p0, p1, ..] => affine_least_squares(pairs).or_else(|| helmert_from_two(p0, p1, source_y_down)),
    };
    fit.map(|mut a| {
        a.pair_count = pairs.len();
        a.rms_m = rms_of(&a, pairs);
        a
    })
}

/// The scales of the drawing's two axes on the map and the angle by which they leave square (`Shape`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignShape {
    pub scale_major: f64,
    pub scale_minor: f64,
    pub anisotropy: f64,
    pub shear_deg: f64,
}

/// What the pairs say (`Report`): residuals only mean something with more pairs than the fit needs (4 and more).
#[derive(Debug, Clone, PartialEq)]
pub struct AlignReport {
    pub affine: Option<AlignAffine>,
    pub pair_count: usize,
    pub measurable: bool,
    pub residuals: Vec<f64>,
    pub rms: f64,
    pub max_residual: f64,
    pub max_index: Option<usize>,
    pub shape: Option<AlignShape>,
}

/// `shapeOf`: closed-form singular values of the linear part, and the angle between the images of the axes.
#[must_use]
pub fn shape_of(a: &AlignAffine) -> Option<AlignShape> {
    let (big_e, big_f, big_g, big_h) = (0.5 * (a.a + a.e), 0.5 * (a.a - a.e), 0.5 * (a.d + a.b), 0.5 * (a.d - a.b));
    let (q, r) = (big_e.hypot(big_h), big_f.hypot(big_g));
    let (scale_major, scale_minor) = (q + r, (q - r).abs());
    if scale_minor.is_nan() || scale_minor <= 0.0 || !scale_major.is_finite() {
        return None;
    }
    let (lx, ly) = (a.a.hypot(a.d), a.b.hypot(a.e));
    let shear_deg = if lx > 0.0 && ly > 0.0 { ((a.a * a.b + a.d * a.e).abs() / (lx * ly)).min(1.0).asin().to_degrees() } else { 0.0 };
    Some(AlignShape { scale_major, scale_minor, anisotropy: scale_major / scale_minor, shear_deg })
}

/// `assess`: the fit, its shape and, from four pairs on, every pair's residual.
#[must_use]
pub fn assess(pairs: &[AlignPair], source_y_down: bool) -> AlignReport {
    let mut report = AlignReport { affine: align_from_pairs(pairs, source_y_down), pair_count: pairs.len(), measurable: false, residuals: Vec::new(), rms: 0.0, max_residual: 0.0, max_index: None, shape: None };
    let Some(affine) = report.affine else { return report };
    report.shape = shape_of(&affine);
    report.measurable = pairs.len() >= 4;
    if !report.measurable {
        return report;
    }
    for (i, p) in pairs.iter().enumerate() {
        let d = off(affine.apply(p.src), p.map);
        report.residuals.push(d);
        if d > report.max_residual || report.max_index.is_none() {
            report.max_residual = d;
            report.max_index = Some(i);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let n = pairs.len() as f64;
    report.rms = (report.residuals.iter().map(|d| d * d).sum::<f64>() / n).sqrt();
    report
}

/// The row of a pair in the list (`rowText`).
#[must_use]
pub fn row_text(r: &AlignReport, index: usize) -> String {
    let mut s = format!("{}번  왼쪽 → 오른쪽", index + 1);
    if let (true, Some(d)) = (r.measurable, r.residuals.get(index)) {
        s += &format!(" · 어긋남 {d:.2} m");
        if r.max_index == Some(index) && r.max_residual > RESIDUAL_WARN_M {
            s += " (가장 큼)";
        }
    }
    s
}

/// The notes under the list (`notes`).
#[must_use]
pub fn notes(r: &AlignReport) -> Vec<String> {
    let mut out = Vec::new();
    if r.affine.is_none() {
        return out;
    }
    if r.pair_count == 3 {
        out.push("점 4개부터 점마다 어긋남을 잽니다".to_owned());
    }
    if let (true, Some(i)) = (r.measurable && r.max_residual > RESIDUAL_WARN_M, r.max_index) {
        out.push(format!("주의: {}번 점이 {:.2} m 어긋납니다. 두 쪽 점을 다시 확인하세요.", i + 1, r.max_residual));
    }
    if let Some(shape) = r.shape.filter(|_| r.pair_count >= 3) {
        if shape.anisotropy > ANISOTROPY_WARN {
            out.push(format!("주의: 가로·세로 배율이 {:.0}% 다릅니다. 잘못 찍은 점이 있으면 그림이 늘어납니다.", (shape.anisotropy - 1.0) * 100.0));
        }
        if shape.shear_deg > SHEAR_WARN_DEG {
            out.push(format!("주의: 그림 축이 {:.1}° 비뚤어집니다. 잘못 찍은 점이 있는지 보세요.", shape.shear_deg));
        }
    }
    out
}

/// What the status line adds after the number of points (`summary`).
#[must_use]
pub fn summary(r: &AlignReport) -> String {
    match r.pair_count {
        2 => " · 한 점 더 찍으면 기울기도 맞습니다".to_owned(),
        3 => " · 한 점 더 찍으면 어긋남을 잽니다".to_owned(),
        _ if !r.measurable => String::new(),
        _ => format!(" · 평균 어긋남 {:.2} m · 최대 {:.2} m", r.rms, r.max_residual),
    }
}
