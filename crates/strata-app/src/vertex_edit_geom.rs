//! The vertices of a shape, for the handles of 「도형선택」 (C++ `KaVertexEditTool`, `GeometryEditVertex.cpp`).
//! Vertices are numbered as QGIS numbers them: over the parts, their rings and their points in order; a
//! polygon ring ends on its first point again, and that closing point has a number of its own.

use strata_storage::wkb_codec::WkbGeometry;

/// 「면은 꼭짓점 3개보다 줄일 수 없습니다.」 · 「선은 꼭짓점 2개보다 줄일 수 없습니다.」 (KaVertexEditTool.cpp:368-373)
pub(crate) const FEWEST_OF_A_POLYGON: &str = "면은 꼭짓점 3개보다 줄일 수 없습니다.";
pub(crate) const FEWEST_OF_A_LINE: &str = "선은 꼭짓점 2개보다 줄일 수 없습니다.";

/// The runs of points of a shape in the order of their numbers, each with whether it is a ring.
fn runs(parts: &[WkbGeometry]) -> Vec<(&[[f64; 2]], bool)> {
    let mut out: Vec<(&[[f64; 2]], bool)> = Vec::new();
    for part in parts {
        match part {
            WkbGeometry::Point(point) => out.push((std::slice::from_ref(point), false)),
            WkbGeometry::LineString(line) => out.push((line, false)),
            WkbGeometry::Polygon(rings) => out.extend(rings.iter().map(|ring| (ring.as_slice(), true))),
        }
    }
    out
}

/// The run a vertex is in (counted over all runs) and its place there.
fn place_of(parts: &[WkbGeometry], index: usize) -> Option<(usize, usize)> {
    let mut first = 0;
    for (run, (points, _)) in runs(parts).into_iter().enumerate() {
        if index < first + points.len() {
            return Some((run, index - first));
        }
        first += points.len();
    }
    None
}

/// The shape with one of its runs changed.
fn with_run(parts: &[WkbGeometry], run: usize, change: impl FnOnce(&mut Vec<[f64; 2]>)) -> Vec<WkbGeometry> {
    let (mut out, mut at, mut change) = (parts.to_vec(), 0, Some(change));
    for part in &mut out {
        let lines: Vec<&mut Vec<[f64; 2]>> = match part {
            WkbGeometry::Point(_) => {
                at += 1;
                continue;
            }
            WkbGeometry::LineString(line) => vec![line],
            WkbGeometry::Polygon(rings) => rings.iter_mut().collect(),
        };
        for line in lines {
            if at == run {
                if let Some(change) = change.take() {
                    change(line);
                }
            }
            at += 1;
        }
    }
    out
}

/// Every vertex of the shape in the order of its numbers.
pub(crate) fn vertices(parts: &[WkbGeometry]) -> Vec<[f64; 2]> {
    runs(parts).into_iter().flat_map(|(points, _)| points.iter().copied()).collect()
}

/// The other number of a point that closes a polygon ring (`ringClosingVertex`): the last for the first
/// and the first for the last; `None` for every other vertex and for lines.
pub(crate) fn ring_closing_vertex(parts: &[WkbGeometry], index: usize) -> Option<usize> {
    let (run, at) = place_of(parts, index)?;
    let (points, ring) = runs(parts)[run];
    match (ring && points.len() >= 2, at) {
        (true, 0) => Some(index + points.len() - 1),
        (true, last) if last == points.len() - 1 => Some(index + 1 - points.len()),
        _ => None,
    }
}

/// The shape with a vertex at another place; the point that closes its ring goes with it.
pub(crate) fn with_vertex_moved(parts: &[WkbGeometry], index: usize, to: [f64; 2]) -> Option<Vec<WkbGeometry>> {
    let (run, at) = place_of(parts, index)?;
    if matches!(parts.first(), Some(WkbGeometry::Point(_))) && parts.len() == 1 {
        return Some(vec![WkbGeometry::Point(to)]);
    }
    let twin = ring_closing_vertex(parts, index).and_then(|twin| place_of(parts, twin));
    Some(with_run(parts, run, |points| {
        points[at] = to;
        if let Some((_, twin)) = twin {
            points[twin] = to;
        }
    }))
}

/// The vertex nearest to a place and no further than `tolerance` (`vertexNear`: of equally near ones the
/// last, so the closing point of a ring).
pub(crate) fn vertex_near(parts: &[WkbGeometry], place: [f64; 2], tolerance: f64) -> Option<usize> {
    let mut best = (None, tolerance);
    for (index, point) in vertices(parts).into_iter().enumerate() {
        let away = (point[0] - place[0]).hypot(point[1] - place[1]);
        if away <= best.1 {
            best = (Some(index), away);
        }
    }
    best.0
}

/// The segment nearest to a place and no further than `tolerance` (`segmentNear`): the number of the vertex
/// at its end, and the point on it.
pub(crate) fn segment_near(parts: &[WkbGeometry], place: [f64; 2], tolerance: f64) -> Option<(usize, [f64; 2])> {
    let (mut best, mut first) = (None::<(usize, [f64; 2], f64)>, 0);
    for (points, _) in runs(parts) {
        for (at, pair) in points.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let along = if dx == 0.0 && dy == 0.0 { 0.0 } else { (((place[0] - a[0]) * dx + (place[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0) };
            let on = [a[0] + dx * along, a[1] + dy * along];
            let away = (on[0] - place[0]).hypot(on[1] - place[1]);
            if away <= tolerance && best.is_none_or(|(_, _, nearest)| away < nearest) {
                best = Some((first + at + 1, on, away));
            }
        }
        first += points.len();
    }
    best.map(|(end, on, _)| (end, on))
}

/// The shape with a new vertex before the vertex `before` (`insertVertexAt`).
pub(crate) fn with_vertex_inserted(parts: &[WkbGeometry], before: usize, at: [f64; 2]) -> Option<Vec<WkbGeometry>> {
    let (run, place) = place_of(parts, before).filter(|(_, place)| *place > 0)?;
    Some(with_run(parts, run, |points| points.insert(place, at)))
}

/// The shape without a vertex (`deleteVertexAt`). A polygon ring keeps three corners and a line two points:
/// below that the words say so. Only the ring of the vertex is counted. When the point that closes a ring
/// goes, the ring closes on the next corner.
pub(crate) fn without_vertex(parts: &[WkbGeometry], index: usize) -> Result<Vec<WkbGeometry>, &'static str> {
    let Some((run, at)) = place_of(parts, index) else { return Err("") };
    let (points, ring) = runs(parts)[run];
    if points.len() < if ring { 5 } else { 3 } {
        return Err(if ring { FEWEST_OF_A_POLYGON } else { FEWEST_OF_A_LINE });
    }
    Ok(with_run(parts, run, |points| {
        if ring && (at == 0 || at == points.len() - 1) {
            points.pop();
            points.remove(0);
            points.push(points[0]);
        } else {
            points.remove(at);
        }
    }))
}

#[cfg(test)]
#[path = "vertex_edit_geom_tests.rs"]
mod tests;
