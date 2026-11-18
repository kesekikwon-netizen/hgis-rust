//! 정합(맞추기) 세션 상태 — C++ `MainWindow::m_alignTool` + `m_alignApplied` 1:1.
/// C++ `GeorefService::Pair` — 왼쪽(그림 원본 픽셀) 좌표와 오른쪽(지도) 좌표 한 쌍
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlignPair {
    pub src: [f64; 2],
    pub map: [f64; 2],
}

/// C++ `MainWindow::m_alignTool` + `m_alignApplied` — 정합 세션 상태 1:1.
#[derive(Debug, Clone, Default)]
pub struct AlignSessionState {
    pub(crate) pending_view: Option<(u32, [f64; 4])>,
    pub(crate) picker: super::align_picker::PickerSlot,
    pub(crate) raster: Option<raster::RasterSession>,
    /// 맞출 도면 이름. `None` 이면 C++ `statusText()` 첫 줄(`맞출 도면을 고르세요`).
    pub target_layer: Option<String>,
    pub pairs: Vec<AlignPair>,
    /// C++ `m_haveFrom` — 왼쪽 점만 찍고 오른쪽 점을 기다리는 중
    pub pending_source: Option<[f64; 2]>,
    /// C++ `m_alignApplied` — 「이동」을 이미 눌렀다
    pub applied: bool,
    pub transform: Option<strata_geo::align_fit::AlignAffine>,
    /// 「맞추기 저장」이 실제로 쓴 파일 (C++ `saveAligned` 의 `path`)
    pub saved_path: Option<String>,
}

impl AlignSessionState {
    pub(crate) fn cancel_target_action(&mut self) {
        // Cancel transient input only; keep source pixels, completed pairs and original data.
        // target/cpp-ref/src/app/KaAlignMapTool.cpp:538-576.
        if let Some(session) = &mut self.raster {
            session.snap.cancel();
            session.drag = None;
            session.target_cursor = None;
        }
    }
    pub(crate) fn set_source(&mut self, xy: [f64; 2]) {
        self.cancel_target_action();
        self.pending_source = Some(xy);
    }
    pub(crate) fn accept_map(&mut self, xy: [f64; 2]) -> bool {
        let Some(src) = self.pending_source.take() else { return false };
        self.pairs.push(AlignPair { src, map: xy });
        self.invalidate();
        true
    }
    pub(crate) fn move_pair(&mut self, index: usize, source: bool, xy: [f64; 2]) {
        if let Some(pair) = self.pairs.get_mut(index) {
            if source {
                pair.src = xy;
            } else {
                pair.map = xy;
            }
            self.invalidate();
        }
    }
    pub(crate) fn remove_pair_at(&mut self, index: usize) -> bool {
        if index >= self.pairs.len() {
            return false;
        }
        self.pairs.remove(index);
        self.pending_source = None;
        self.invalidate();
        true
    }
    fn invalidate(&mut self) {
        self.cancel_target_action();
        self.applied = false;
        self.transform = None;
    }
    /// C++ `KaAlignMapTool::statusText()` — `KaAlignMapTool.cpp:453-462`
    #[must_use]
    pub fn status_text(&self) -> String {
        if self.target_layer.is_none() {
            return "맞출 도면을 고르세요".to_owned();
        }
        if self.pending_source.is_some() {
            return "오른쪽 지도에서 같은 모서리를 찍으세요".to_owned();
        }
        if self.pairs.is_empty() {
            return "왼쪽 도면 모서리 → 오른쪽 지적 모서리".to_owned();
        }
        format!("왼쪽 → 오른쪽 · {}점{}", self.pairs.len(), self.quality_summary())
    }

    // Raster source pixels grow downwards (target/cpp-ref/src/core/GeorefService.cpp:160-176).
    pub(crate) fn report(&self) -> strata_geo::align_fit::AlignReport {
        let pairs: Vec<_> = self.pairs.iter().map(|p| strata_geo::align_fit::AlignPair { src: p.src, map: p.map }).collect();
        strata_geo::align_fit::assess(&pairs, true)
    }

    fn quality_summary(&self) -> String {
        strata_geo::align_fit::summary(&self.report())
    }

    /// C++ `KaAlignMapTool::removeLastPair()` — `KaAlignMapTool.cpp:319-329`.
    /// 왼쪽 점만 찍혀 있으면 그것부터 지우고, 아니면 마지막 점 쌍을 지운다.
    pub fn remove_last_pair(&mut self) -> bool {
        self.cancel_target_action();
        if self.pending_source.take().is_some() {
            return true;
        }
        let removed = self.pairs.pop().is_some();
        if removed {
            self.invalidate();
        }
        removed
    }

    /// C++ `KaAlignMapTool::endSession()` — `KaAlignMapTool.cpp:177-198`
    pub fn end_session(&mut self) {
        self.pairs.clear();
        self.pending_source = None;
        self.applied = false;
        self.transform = None;
        self.target_layer = None;
        self.saved_path = None;
        self.raster = None;
        self.picker.0 = None;
        self.pending_view = None;
    }
}

#[cfg(test)]
#[path = "align_session_tests.rs"]
mod tests;

#[path = "align_raster_task.rs"]
pub(crate) mod raster;
