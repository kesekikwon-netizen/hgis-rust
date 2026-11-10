use super::draw::DrawShape;
use super::rasterize::Raster;

pub(super) fn paint_shape(r: &mut Raster, s: &DrawShape) {
    match s {
        DrawShape::Polygon {
            pts,
            fill,
            stroke,
            stroke_width_px,
        } => {
            fill_polygon(r, pts, *fill);
            if *stroke_width_px > 0.0 {
                stroke_polygon(r, pts, *stroke, *stroke_width_px);
            }
        }
        DrawShape::Line {
            pts,
            color,
            width_px,
        } => {
            for w in pts.windows(2) {
                draw_line(r, w[0], w[1], *color, *width_px);
            }
        }
        DrawShape::Point {
            at,
            radius_px,
            fill,
            stroke,
        } => {
            draw_disc(r, *at, *radius_px, *fill);
            draw_ring(r, *at, *radius_px, *stroke);
        }
        DrawShape::Label { at, size_px, color, .. } => {
            // 글꼴 없이 자리만 표시 — 라벨이 어디에 놓이는지 검증할 수 있게.
            let rr = (*size_px * 0.25).max(1.0);
            draw_disc(r, *at, rr, *color);
        }
    }
}

fn fill_polygon(r: &mut Raster, pts: &[[f32; 2]], c: [u8; 4]) {
    if pts.len() < 3 || c[3] == 0 {
        return;
    }
    let (mut ymin, mut ymax) = (f32::INFINITY, f32::NEG_INFINITY);
    for p in pts {
        ymin = ymin.min(p[1]);
        ymax = ymax.max(p[1]);
    }
    let y0 = ymin.floor().max(0.0) as i64;
    let y1 = ymax.ceil().min(r.height as f32 - 1.0) as i64;
    for y in y0..=y1 {
        let scan = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::with_capacity(8);
        let mut j = pts.len() - 1;
        for i in 0..pts.len() {
            let (xi, yi) = (pts[i][0], pts[i][1]);
            let (xj, yj) = (pts[j][0], pts[j][1]);
            if (yi > scan) != (yj > scan) {
                let t = (scan - yi) / (yj - yi);
                xs.push(xi + (xj - xi) * t);
            }
            j = i;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let xa = pair[0].ceil().max(0.0) as i64;
            let xb = pair[1].floor().min(r.width as f32 - 1.0) as i64;
            for x in xa..=xb {
                r.blend(x as u32, y as u32, c);
            }
        }
    }
}

fn stroke_polygon(r: &mut Raster, pts: &[[f32; 2]], c: [u8; 4], w: f32) {
    if pts.len() < 2 {
        return;
    }
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        draw_line(r, a, b, c, w);
    }
}

fn draw_line(r: &mut Raster, a: [f32; 2], b: [f32; 2], c: [u8; 4], w: f32) {
    if c[3] == 0 {
        return;
    }
    let half = (w * 0.5).max(0.5);
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i64;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        draw_disc(r, [a[0] + dx * t, a[1] + dy * t], half, c);
    }
}

fn draw_disc(r: &mut Raster, at: [f32; 2], radius: f32, c: [u8; 4]) {
    if c[3] == 0 || radius <= 0.0 {
        return;
    }
    let x0 = (at[0] - radius).floor().max(0.0) as i64;
    let x1 = (at[0] + radius).ceil().min(r.width as f32 - 1.0) as i64;
    let y0 = (at[1] - radius).floor().max(0.0) as i64;
    let y1 = (at[1] + radius).ceil().min(r.height as f32 - 1.0) as i64;
    let rr = radius * radius;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - at[0];
            let dy = y as f32 + 0.5 - at[1];
            if dx * dx + dy * dy <= rr {
                r.blend(x as u32, y as u32, c);
            }
        }
    }
}

fn draw_ring(r: &mut Raster, at: [f32; 2], radius: f32, c: [u8; 4]) {
    if c[3] == 0 || radius <= 1.0 {
        return;
    }
    let inner = (radius - 1.0).max(0.0);
    let x0 = (at[0] - radius).floor().max(0.0) as i64;
    let x1 = (at[0] + radius).ceil().min(r.width as f32 - 1.0) as i64;
    let y0 = (at[1] - radius).floor().max(0.0) as i64;
    let y1 = (at[1] + radius).ceil().min(r.height as f32 - 1.0) as i64;
    let (ro, ri) = (radius * radius, inner * inner);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - at[0];
            let dy = y as f32 + 0.5 - at[1];
            let d2 = dx * dx + dy * dy;
            if d2 <= ro && d2 > ri {
                r.blend(x as u32, y as u32, c);
            }
        }
    }
}
