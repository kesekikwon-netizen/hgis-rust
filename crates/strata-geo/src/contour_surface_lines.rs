//! Chain the interval crossings of a height grid into polylines.

use super::{HeightGrid, NODATA};

pub(super) fn chain(grid: &HeightGrid, level: f64) -> Vec<Vec<[f64; 2]>> {
    let mut segs = Vec::new();
    for row in 0..grid.height.saturating_sub(1) {
        for col in 0..grid.width.saturating_sub(1) {
            let Some(nw) = at(grid, col, row) else { continue };
            let Some(ne) = at(grid, col + 1, row) else { continue };
            let Some(sw) = at(grid, col, row + 1) else { continue };
            let Some(se) = at(grid, col + 1, row + 1) else { continue };
            let hits = [cross(nw, ne, level), cross(ne, se, level), cross(se, sw, level), cross(sw, nw, level)];
            let hits: Vec<[f64; 2]> = hits.into_iter().flatten().collect();
            if hits.len() >= 2 {
                segs.push((hits[0], hits[1]));
            }
            if hits.len() == 4 {
                segs.push((hits[2], hits[3]));
            }
        }
    }
    join(segs)
}

fn at(grid: &HeightGrid, col: usize, row: usize) -> Option<(f64, f64, f64)> {
    let z = grid.values[row * grid.width + col];
    if z == NODATA {
        None
    } else {
        let x = grid.west + (col as f64 + 0.5) * grid.cell;
        let y = grid.north - (row as f64 + 0.5) * grid.cell;
        Some((x, y, f64::from(z)))
    }
}

fn cross(a: (f64, f64, f64), b: (f64, f64, f64), level: f64) -> Option<[f64; 2]> {
    let below = (a.2 < level && b.2 >= level) || (b.2 < level && a.2 >= level);
    if !below || (b.2 - a.2).abs() < 1e-12 {
        return None;
    }
    let t = (level - a.2) / (b.2 - a.2);
    Some([a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)])
}

fn key(p: [f64; 2]) -> (i64, i64) {
    ((p[0] * 1.0e4).round() as i64, (p[1] * 1.0e4).round() as i64)
}

fn join(segs: Vec<([f64; 2], [f64; 2])>) -> Vec<Vec<[f64; 2]>> {
    let mut unused: Vec<Option<([f64; 2], [f64; 2])>> = segs.into_iter().map(Some).collect();
    let mut lines = Vec::new();
    while let Some(start) = unused.iter().position(Option::is_some) {
        let (a, b) = unused[start].take().unwrap_or(([0.0, 0.0], [0.0, 0.0]));
        let mut line = vec![a, b];
        extend(&mut line, &mut unused, true);
        extend(&mut line, &mut unused, false);
        if line.len() >= 2 {
            lines.push(line);
        }
    }
    lines
}

fn extend(line: &mut Vec<[f64; 2]>, unused: &mut [Option<([f64; 2], [f64; 2])>], at_end: bool) {
    loop {
        let tip = if at_end { line[line.len() - 1] } else { line[0] };
        let tail = key(tip);
        let Some(hit) = unused.iter().position(|seg| seg.is_some_and(|(p, q)| key(p) == tail || key(q) == tail)) else {
            break;
        };
        let (p, q) = unused[hit].take().unwrap_or(([0.0, 0.0], [0.0, 0.0]));
        let next = if key(p) == tail { q } else { p };
        if at_end {
            line.push(next);
        } else {
            line.insert(0, next);
        }
    }
}
