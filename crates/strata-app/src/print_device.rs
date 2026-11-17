//! OS printer boundary, target/cpp-ref/src/app/KaPrintDialog.cpp:83-124,620-669.
use super::print_plan::DevicePaper;
use super::print_plan::PrintPlan;
use serde::{Deserialize, Serialize};
#[path = "print_device_process.rs"]
mod print_device_process;

pub(crate) fn discover() -> Result<Vec<Printer>, String> {
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let bytes = print_device_process::run("Catalog", None, &cancelled)?;
    parse_catalog(&bytes)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Printer {
    pub name: String,
    pub is_default: bool,
    pub file_output: bool,
    pub papers: Vec<DevicePaper>,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    version: u32,
    printers: Vec<Printer>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PrintRequest {
    pub printer: String,
    pub title: String,
    pub paper: DevicePaper,
    pub plan: PrintPlan,
    pub drawing_mm: [f64; 2],
    pub dpi: u32,
    pub output: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PrintReceipt {
    pub version: u32,
    pub printer: String,
    pub pages: usize,
    pub paper_mm: [f64; 2],
}

pub(crate) fn validate_receipt(request: &PrintRequest, bytes: &[u8]) -> Result<PrintReceipt, String> {
    let receipt: PrintReceipt = serde_json::from_slice(bytes).map_err(|why| format!("Invalid printer receipt: {why}"))?;
    let pages = request.plan.pages.len() + usize::from(request.plan.overview);
    if receipt.version != 1
        || receipt.printer != request.printer
        || receipt.pages != pages
        || receipt.paper_mm.iter().any(|v| !v.is_finite())
        || receipt.paper_mm.iter().zip(request.plan.paper_mm).any(|(got, want)| (*got - want).abs() > 2.0)
    {
        return Err("Printer returned a different device, physical paper or page count".to_owned());
    }
    Ok(receipt)
}

#[path = "print_device_output.rs"]
mod print_device_output;
pub(crate) use print_device_output::dispatch;

pub(crate) fn parse_catalog(bytes: &[u8]) -> Result<Vec<Printer>, String> {
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|why| format!("Invalid printer response: {why}"))?;
    if catalog.version != 1 || catalog.printers.iter().any(|printer| printer.name.trim().is_empty() || printer.papers.iter().any(|paper| !valid_paper(paper))) {
        return Err("Invalid printer capabilities".to_owned());
    }
    Ok(catalog.printers)
}

fn valid_paper(paper: &DevicePaper) -> bool {
    if !paper.width_mm.is_finite() || !paper.height_mm.is_finite() || paper.width_mm <= 0.0 || paper.height_mm < paper.width_mm {
        return false;
    }
    [false, true].into_iter().all(|landscape| {
        let (size, rect) = paper.oriented(landscape);
        rect.iter().all(|v| v.is_finite())
            && rect[0] >= 0.0
            && rect[1] >= 0.0
            && rect[2] > 0.0
            && rect[3] > 0.0
            && rect[0] + rect[2] <= size[0] + 2.0
            && rect[1] + rect[3] <= size[1] + 2.0
    })
}

#[cfg(all(test, windows))]
#[path = "print_device_probe_tests.rs"]
mod probe_tests;
#[cfg(test)]
#[path = "print_device_tests.rs"]
mod tests;
