//! Path operators for the map frame, scale bar, north arrow and survey rings.

use crate::sheet_composer::{nice_scale_bar_segment_meters, DrawingSheet, SheetRectMm};
use crate::sheet_pdf_export::mm_to_pdf_points;

use super::SheetVectorRing;

pub(super) fn rect_op(page_h: f64, r: &SheetRectMm, op: &str) -> String {
    format!(
        "{:.2} {:.2} {:.2} {:.2} re {op}\n",
        mm_to_pdf_points(r.x_mm),
        mm_to_pdf_points(page_h - r.y_mm - r.height_mm),
        mm_to_pdf_points(r.width_mm),
        mm_to_pdf_points(r.height_mm)
    )
}

/// Four double-box segments (`LayoutService.cpp:598-627`, style "Double Box").
pub(super) fn scale_boxes(page_h: f64, sheet: &DrawingSheet) -> String {
    let denom = f64::from(sheet.scale_denominator.max(1));
    let seg = nice_scale_bar_segment_meters(sheet.map_frame.width_mm, denom, 4);
    let width = (seg * 4.0 * 1000.0 / denom).min(sheet.scale_bar_rect.width_mm).max(1.0);
    let band = sheet.scale_bar_rect.height_mm / 2.0;
    let mut out = String::new();
    for row in 0..2 {
        for col in 0..4 {
            let cell = SheetRectMm {
                x_mm: sheet.scale_bar_rect.x_mm + width / 4.0 * f64::from(col),
                y_mm: sheet.scale_bar_rect.y_mm + band * f64::from(row),
                width_mm: width / 4.0,
                height_mm: band,
            };
            let op = if (col + row) % 2 == 0 { "f" } else { "S" };
            out.push_str(&rect_op(page_h, &cell, op));
        }
    }
    out
}

pub(super) fn north_path(page_h: f64, r: &SheetRectMm) -> String {
    let (cx, top, wing) = (r.x_mm + r.width_mm * 0.5, r.y_mm + 1.5, r.width_mm * 0.28);
    let pts = [[cx, top], [cx - wing, r.bottom() - 1.5], [cx + wing, r.bottom() - 1.5]];
    path_mm(page_h, &pts, true, "f")
}

pub(super) fn rings_path(sheet: &DrawingSheet, center: [f64; 2], rings: &[SheetVectorRing]) -> String {
    let mut out = String::new();
    let (x, y, w, h) = (
        mm_to_pdf_points(sheet.map_frame.x_mm),
        mm_to_pdf_points(sheet.height_mm - sheet.map_frame.y_mm - sheet.map_frame.height_mm),
        mm_to_pdf_points(sheet.map_frame.width_mm),
        mm_to_pdf_points(sheet.map_frame.height_mm),
    );
    if !rings.is_empty() {
        out.push_str(&format!("q {x:.2} {y:.2} {w:.2} {h:.2} re W n\n"));
    }
    for ring in rings {
        if ring.xy.len() < 2 {
            continue;
        }
        let paper: Vec<[f64; 2]> = ring.xy.iter().map(|xy| sheet.world_to_sheet_mm(center, *xy)).collect();
        out.push_str(&path_mm(sheet.height_mm, &paper, ring.filled, if ring.filled { "f" } else { "S" }));
    }
    if !rings.is_empty() {
        out.push_str("Q\n");
    }
    out
}

fn path_mm(page_h: f64, pts: &[[f64; 2]], close: bool, op: &str) -> String {
    let mut out = String::new();
    for (i, p) in pts.iter().enumerate() {
        let cmd = if i == 0 { "m" } else { "l" };
        out.push_str(&format!("{:.2} {:.2} {cmd}\n", mm_to_pdf_points(p[0]), mm_to_pdf_points(page_h - p[1])));
    }
    if close {
        out.push_str("h\n");
    }
    out.push_str(op);
    out.push('\n');
    out
}
