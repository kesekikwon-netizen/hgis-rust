//! Letters and survey geometry for the vector sheet PDF.
//!
//! The page is the sheet the studio shows (`KaDrawingStudio::savePdf`,
//! `KaDrawingStudio.cpp:3819-3855`), not a picture of that sheet. Rings are work-CRS
//! metres (EPSG:5186 or EPSG:5187).

use strata_layout::sheet_pdf_vector::{
    render_vector_sheet_pdf, SheetPdfLabel, SheetVectorRing, VectorSheetPdf,
};
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::survey_paint_layers::{build_paint_layers_with, PaintPart};

impl StrataAppState {
    /// The PDF 「PDF 내보내기」 writes.
    pub(crate) fn vector_sheet_pdf(&mut self, map_jpeg: Option<&[u8]>, map_jpeg_px: [u32; 2]) -> Result<Vec<u8>, String> {
        let center = self.sheet_map_center();
        let lines = self.sheet_title_block_lines_on(chrono::Local::now().date_naive());
        let scale =
            super::gui_tab_drawing_decoration::scale_text(self.drawing.sheet.scale_denominator);
        let crs = match self.work_crs {
            WorkCrs::Epsg5186 => "중부원점(GRS80) · EPSG:5186",
            WorkCrs::Epsg5187 => "동부원점(GRS80) · EPSG:5187",
        };
        let rows = self.sheet_legend_pdf_names();
        let (rings, labels) = self.sheet_pdf_marks();
        let sheet = self.drawing.sheet.clone();
        let title = self.drawing.title_ko.clone();
        let legend = sheet.legend_title.clone();
        render_vector_sheet_pdf(&VectorSheetPdf {
            sheet: &sheet,
            title: &title,
            lines: &lines,
            scale_label: &scale,
            crs_label: crs,
            legend_title: &legend,
            legend_rows: &rows,
            north_label: "N",
            map_center_xy: center,
            rings: &rings,
            labels: &labels,
            map_jpeg,
            map_jpeg_px,
        })
        .map_err(|err| err.to_string())
    }

    /// Rings and the names C++ prints on them (`LabelOps.cpp:67-130`, area text `:217-245`,
    /// cadastral `JIBUN` while the scale is closer than 1:10000, `CadastralImport.cpp:250-263`).
    fn sheet_pdf_marks(&self) -> (Vec<SheetVectorRing>, Vec<SheetPdfLabel>) {
        let mut rings = Vec::new();
        let mut labels = Vec::new();
        let rs = &self.home_map.real_survey;
        if rs.is_layer_visible("조사구역") {
            for ring in &rs.survey_rings {
                push_ring(&mut rings, ring, true);
            }
        }
        if rs.is_layer_visible("시굴격자") {
            for trench in &rs.trenches {
                push_ring(&mut rings, &trench.ring, true);
            }
        }
        if rs.is_layer_visible("지적도 · 조사 주변 5km") {
            let letters = self.drawing.sheet.scale_denominator <= 10_000 && rs.is_label_visible("지적도 · 조사 주변 5km");
            for parcel in &rs.cadastral_parcels {
                push_ring(&mut rings, &parcel.ring, false);
                if letters && !parcel.jibun.is_empty() {
                    labels.push(SheetPdfLabel { text: parcel.jibun.clone(), xy: parcel.center, size_pt: 8.0 });
                }
            }
        }
        if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
            for site in &rs.heritage_polys {
                if rs.is_layer_visible(&site.layer_name) {
                    push_ring(&mut rings, &site.ring, true);
                }
            }
        }
        for (meters, ring) in &rs.distance_rings {
            if rs.is_layer_visible(&format!("주변 {meters}m")) {
                for line in &ring.lines {
                    push_ring(&mut rings, line, false);
                }
            }
        }
        for poly in &self.home_map.scene.polygons {
            push_ring(&mut rings, &poly.world_pts, true);
        }
        let layers =
            build_paint_layers_with(&self.survey_tables, self.work_crs.into(), &rs.label_views);
        for layer in &layers {
            let title = self
                .left_layers
                .entries
                .iter()
                .find(|entry| entry.layer_key == layer.table)
                .map_or(layer.title.as_str(), |entry| entry.label_ko.as_str());
            if !rs.is_layer_visible(title) {
                continue;
            }
            let letters = rs.is_label_visible(title);
            for feature in &layer.features {
                if letters {
                    if let (Some(text), Some(anchor)) = (&feature.label, feature.anchor) {
                        if !text.is_empty() {
                            labels.push(SheetPdfLabel { text: text.clone(), xy: anchor.xy, size_pt: 8.0 });
                        }
                    }
                }
                for part in &feature.parts {
                    match part {
                        PaintPart::Point(xy) => {
                            push_ring(
                                &mut rings,
                                &[[xy[0] - 1.0, xy[1]], [xy[0] + 1.0, xy[1]]],
                                false,
                            );
                            push_ring(
                                &mut rings,
                                &[[xy[0], xy[1] - 1.0], [xy[0], xy[1] + 1.0]],
                                false,
                            );
                        }
                        PaintPart::Line(pts) => push_ring(&mut rings, pts, false),
                        PaintPart::Polygon(parts) => {
                            for ring in parts {
                                push_ring(&mut rings, ring, true);
                            }
                        }
                    }
                }
            }
        }
        (rings, labels)
    }
}

fn push_ring(out: &mut Vec<SheetVectorRing>, xy: &[[f64; 2]], filled: bool) {
    if xy.len() >= 2 {
        out.push(SheetVectorRing {
            xy: xy.to_vec(),
            filled,
        });
    }
}
