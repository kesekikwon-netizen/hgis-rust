use strata_check_export::checklist_engine::{ChecklistEngine, ChecklistFinding};
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

#[derive(Debug, Clone)]
pub struct DrawingTabState {
    pub title_ko: String,
    pub sheet: DrawingSheet,
    pub last_findings: Vec<ChecklistFinding>,
    /// KaDrawingStudio.cpp:233-252 decorationSizePercent (75/100/125/150)
    pub decoration_size_percent: u32,
    /// KaDrawingStudio.cpp:822-838 m_adjustBar — 지도 조정 모드
    pub map_adjust_active: bool,
    /// KaDrawingCoordCallout.cpp — 용지 위 좌표점 콜아웃
    pub coord_callout_mode: bool,
    pub coord_callouts: Vec<CoordCallout>,
    /// 도면 지도 칸의 가운데(작업 좌표계) — 없으면 지도 화면 가운데(C++ studioMapExtent), 「레이어를 가운데」가 정한다.
    pub sheet_center: Option<[f64; 2]>,
    /// Where the user put the items of the sheet: the way from where the sheet places them itself, in
    /// millimetres of paper.
    pub item_offsets_mm: std::collections::BTreeMap<SheetItem, [f32; 2]>,
    /// The items as they were painted last: where the sheet places each (screen points) and how far it is
    /// moved. For the mouse of the next frame.
    pub(crate) item_rects: Vec<(SheetItem, eframe::egui::Rect, eframe::egui::Vec2)>,
    /// The scale box (KaDrawingStudio.cpp:1061-1082): a number typed and not applied yet, with the sheet's
    /// scale it was typed against.
    pub(crate) scale_entry: Option<(u32, u32)>,
    /// The item the user clicked last (KaDrawingStudio's select tool), and the items Delete took off the paper
    /// (the legend and the title block are off while `sheet.legend_visible` / `title_block_visible` are false).
    pub(crate) selected_item: Option<SheetItem>,
    pub(crate) removed_items: std::collections::BTreeSet<SheetItem>,
    /// The last words the sheet gave while its tab was shown (C++ `kaLastStatus`), and the guide message as
    /// it was at the last frame, to see new words come.
    pub(crate) status: Option<String>,
    pub(crate) seen_guide: Option<String>,
    /// How many screen points one millimetre of paper had when the sheet was painted last.
    pub(crate) screen_px_per_mm: Option<f32>,
    /// The PDF of the sheet being made (「PDF 내보내기」).
    pub(crate) pdf: crate::gui_tab_drawing::gui_tab_drawing_print::SheetPdfSlot,
    /// KaDrawingSheetSet.cpp — 보관한 도면 목록
    pub stored_sheets: Vec<StoredSheet>,
    /// KaDrawingSheetSet.cpp:135 팝업 열림 상태
    pub sheet_set_menu_open: bool,
    /// KaTerrain3dLayoutStudio.cpp — 3D 입체지형 조판 모드
    pub terrain3d_layout_active: bool,
}

/// An item of a sheet that the user can take with the mouse and put somewhere else on the paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SheetItem {
    Legend,
    TitleBlock,
    NorthArrow,
    ScaleBar,
    ScaleText,
    CrsLabel,
}

/// KaDrawingCoordCallout.cpp — 용지 위 좌표점 하나
#[derive(Debug, Clone)]
pub struct CoordCallout {
    /// A, B, C … 태그
    pub tag: String,
    /// 용지 위 픽셀 위치
    pub pos: [f32; 2],
    /// 작업 좌표 N(북), E(동)
    pub world: [f64; 2],
}

/// KaDrawingSheetSet.cpp — 보관한 도면 한 벌
#[derive(Debug, Clone)]
pub struct StoredSheet {
    pub title: String,
    pub scale_denominator: u32,
    pub paper_w_mm: f64,
    pub paper_h_mm: f64,
    pub legend_visible: bool,
    pub title_block_visible: bool,
}

impl Default for DrawingTabState {
    fn default() -> Self {
        let sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        let last_findings = ChecklistEngine::evaluate_sheet(&sheet, SubmitCrs::Epsg5179, true);
        Self {
            title_ko: "조사도면".to_owned(),
            sheet,
            last_findings,
            decoration_size_percent: 100,
            map_adjust_active: false,
            coord_callout_mode: false,
            coord_callouts: Vec::new(),
            sheet_center: None,
            item_offsets_mm: std::collections::BTreeMap::new(),
            item_rects: Vec::new(),
            scale_entry: None,
            selected_item: None,
            removed_items: std::collections::BTreeSet::new(),
            status: None,
            seen_guide: None,
            screen_px_per_mm: None,
            pdf: crate::gui_tab_drawing::gui_tab_drawing_print::SheetPdfSlot::default(),
            stored_sheets: Vec::new(),
            sheet_set_menu_open: false,
            terrain3d_layout_active: false,
        }
    }
}

impl DrawingTabState {
    pub fn set_paper_and_orientation(&mut self, size: SheetPaperSize, ori: SheetOrientation) {
        self.sheet.change_paper(size, ori);
        self.last_findings =
            ChecklistEngine::evaluate_sheet(&self.sheet, SubmitCrs::Epsg5179, true);
    }

    pub const HGIS_DPI_DRAFT: i32 = 150;
    pub const HGIS_DPI_STANDARD: i32 = 300;
    pub const HGIS_DPI_HIGH: i32 = 600;

    /// C++ `KaDrawingStudio::validateExportDpi` — 150 / 300 / 600 DPI 표준 규격 검증 및 클램핑 1:1
    #[must_use]
    pub const fn validate_export_dpi(requested_dpi: i32) -> i32 {
        if requested_dpi <= 175 {
            Self::HGIS_DPI_DRAFT
        } else if requested_dpi <= 450 {
            Self::HGIS_DPI_STANDARD
        } else {
            Self::HGIS_DPI_HIGH
        }
    }

    /// C++ `KaDrawingStudio::syncMapExtentToFrame(preserveScale)` — 축척 고정 잠금 시 프레임 크기 기준으로 지상 BBOX 재계산 1:1
    #[must_use]
    pub fn sync_map_extent_to_frame(
        center_xy: [f64; 2],
        current_scale_denom: f64,
        frame_w_mm: f64,
        frame_h_mm: f64,
        preserve_scale: bool,
    ) -> ([f64; 4], f64) {
        let scale = if preserve_scale && current_scale_denom > 0.0 {
            current_scale_denom
        } else {
            1_000.0
        };
        let half_w_m = scale * (frame_w_mm.max(1.0) / 1000.0) * 0.5;
        let half_h_m = scale * (frame_h_mm.max(1.0) / 1000.0) * 0.5;
        (
            [
                center_xy[0] - half_w_m,
                center_xy[1] - half_h_m,
                center_xy[0] + half_w_m,
                center_xy[1] + half_h_m,
            ],
            scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawing_studio_dpi_validation_72_200_1200_and_preserve_scale_sync() {
        assert_eq!(DrawingTabState::validate_export_dpi(72), 150);
        assert_eq!(DrawingTabState::validate_export_dpi(200), 300);
        assert_eq!(DrawingTabState::validate_export_dpi(1200), 600);

        let (bbox, scale) = DrawingTabState::sync_map_extent_to_frame(
            [200_000.0, 500_000.0],
            2_500.0,
            200.0,
            100.0,
            true,
        );
        assert!((scale - 2_500.0).abs() < 1e-6);
        assert!((bbox[2] - bbox[0] - 500.0).abs() < 1e-6);
        assert!((bbox[3] - bbox[1] - 250.0).abs() < 1e-6);
    }
}
