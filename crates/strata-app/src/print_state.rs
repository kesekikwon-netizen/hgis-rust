//! Print tab state, target/cpp-ref/src/app/KaPrintDialog.cpp:83-166.
use super::print_device::PrintRequest;
use super::print_device::Printer;
use super::print_job::PrintJobSlot;
use super::print_plan::{build_plan, DevicePaper, PrintMode, PrintPlan};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use strata_layout::sheet_composer::SheetPaperSize;

#[derive(Debug, Default)]
pub(crate) enum CatalogState {
    #[default]
    Unrequested,
    Loading(Receiver<Result<Vec<Printer>, String>>),
    Ready(Vec<Printer>),
    Failed(String),
}

impl Clone for CatalogState {
    fn clone(&self) -> Self {
        match self {
            Self::Ready(printers) => Self::Ready(printers.clone()),
            Self::Failed(why) => Self::Failed(why.clone()),
            Self::Unrequested | Self::Loading(_) => Self::Unrequested,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PrintTabState {
    pub single_sheet: bool,
    pub paper: SheetPaperSize,
    pub dpi: u32,
    pub overlap_mm: f64,
    pub marks: bool,
    pub overview: bool,
    pub selected: Option<String>,
    pub output_path: String,
    pub catalog: CatalogState,
    pub job: PrintJobSlot,
}

impl Default for PrintTabState {
    fn default() -> Self {
        Self {
            single_sheet: true,
            paper: SheetPaperSize::A4,
            dpi: 300,
            overlap_mm: 10.0,
            marks: true,
            overview: true,
            selected: None,
            output_path: String::new(),
            catalog: CatalogState::Unrequested,
            job: PrintJobSlot::default(),
        }
    }
}

impl PrintTabState {
    pub(crate) fn ensure_catalog(&mut self) {
        if !matches!(self.catalog, CatalogState::Unrequested) {
            return;
        }
        let (send, receive) = channel();
        // Unit tests use a deterministic empty device boundary, never the owner's printers.
        let read: fn() -> Result<Vec<Printer>, String> = if cfg!(test) { || Ok(Vec::new()) } else { super::print_device::discover };
        std::thread::spawn(move || {
            drop(send.send(read()));
        });
        self.catalog = CatalogState::Loading(receive);
    }

    pub(crate) fn poll_catalog(&mut self) {
        let CatalogState::Loading(receive) = &self.catalog else { return };
        match receive.try_recv() {
            Ok(Ok(printers)) => self.install_catalog(printers),
            Ok(Err(why)) => self.catalog = CatalogState::Failed(why),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.catalog = CatalogState::Failed("Printer discovery stopped".to_owned()),
        }
    }

    pub(crate) fn install_catalog(&mut self, printers: Vec<Printer>) {
        let first = self.selected.is_none();
        if !printers.iter().any(|p| Some(&p.name) == self.selected.as_ref()) {
            self.selected = printers.iter().find(|p| p.is_default).or_else(|| printers.first()).map(|p| p.name.clone());
        }
        self.catalog = CatalogState::Ready(printers);
        if first && self.selected_printer().is_some_and(|p| paper_for(p, SheetPaperSize::A3).is_some()) {
            self.paper = SheetPaperSize::A3;
        }
    }

    pub(crate) fn selected_printer(&self) -> Option<&Printer> {
        let CatalogState::Ready(printers) = &self.catalog else { return None };
        printers.iter().find(|p| Some(&p.name) == self.selected.as_ref())
    }

    pub(crate) fn plan(&self, drawing: [f64; 2]) -> Result<PrintPlan, String> {
        let printer = self.selected_printer().ok_or("(설치된 프린터 없음)")?;
        let paper = paper_for(printer, self.paper).ok_or("이 용지로는 인쇄할 수 없습니다")?;
        let mode = if self.single_sheet { PrintMode::Single } else { PrintMode::Tiled { overlap_mm: self.overlap_mm, marks: self.marks, overview: self.overview } };
        build_plan(drawing, paper, mode)
    }

    pub(crate) fn request(&self, title: &str, drawing_mm: [f64; 2]) -> Result<PrintRequest, String> {
        let plan = self.plan(drawing_mm)?;
        let printer = self.selected_printer().ok_or("(설치된 프린터 없음)")?;
        let paper = paper_for(printer, self.paper).ok_or("이 용지로는 인쇄할 수 없습니다")?;
        let output = if printer.file_output {
            if self.output_path.trim().is_empty() {
                return Err("도면 PDF 저장".to_owned());
            }
            Some(std::path::PathBuf::from(self.output_path.trim()))
        } else {
            None
        };
        Ok(PrintRequest { printer: printer.name.clone(), title: title.to_owned(), paper: paper.clone(), plan, drawing_mm, dpi: self.dpi, output })
    }
}

pub(crate) fn paper_for(printer: &Printer, paper: SheetPaperSize) -> Option<&DevicePaper> {
    let (width, height) = paper.portrait_dimensions_mm();
    printer.papers.iter().find(|p| (p.width_mm - width).abs() <= 2.0 && (p.height_mm - height).abs() <= 2.0)
}
