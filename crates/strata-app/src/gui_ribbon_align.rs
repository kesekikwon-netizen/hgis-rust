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
    /// 맞출 도면 이름. `None` 이면 C++ `statusText()` 첫 줄(`맞출 도면을 고르세요`).
    pub target_layer: Option<String>,
    pub pairs: Vec<AlignPair>,
    /// C++ `m_haveFrom` — 왼쪽 점만 찍고 오른쪽 점을 기다리는 중
    pub pending_source: Option<[f64; 2]>,
    /// C++ `m_alignApplied` — 「이동」을 이미 눌렀다
    pub applied: bool,
    pub transform: Option<strata_geo::polygon_ops::HelmertTransform2D>,
    /// 「맞추기 저장」이 실제로 쓴 파일 (C++ `saveAligned` 의 `path`)
    pub saved_path: Option<String>,
}

impl AlignSessionState {
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
        format!(
            "왼쪽 → 오른쪽 · {}점{}",
            self.pairs.len(),
            self.quality_summary()
        )
    }

    /// C++ `GeorefQuality::summary()` — `GeorefQuality.cpp:87-94`.
    /// C++ 은 4점부터 아핀 잔차를 재지만 여기서는 같은 점들로 유사변환(헬머트) 잔차를 잰다.
    #[must_use]
    fn quality_summary(&self) -> String {
        match self.pairs.len() {
            2 => " · 한 점 더 찍으면 기울기도 맞습니다".to_owned(),
            3 => " · 한 점 더 찍으면 어긋남을 잽니다".to_owned(),
            n if n < 4 => String::new(),
            _ => {
                let src: Vec<[f64; 2]> = self.pairs.iter().map(|p| p.src).collect();
                let dst: Vec<[f64; 2]> = self.pairs.iter().map(|p| p.map).collect();
                let Ok(transform) = strata_geo::polygon_ops::solve_helmert_georef(&src, &dst)
                else {
                    return String::new();
                };
                let max = self
                    .pairs
                    .iter()
                    .map(|p| {
                        let q = transform.apply(p.src);
                        (q[0] - p.map[0]).hypot(q[1] - p.map[1])
                    })
                    .fold(0.0_f64, f64::max);
                format!(
                    " · 평균 어긋남 {:.2} m · 최대 {:.2} m",
                    transform.rms_error_m, max
                )
            }
        }
    }

    /// C++ `KaAlignMapTool::removeLastPair()` — `KaAlignMapTool.cpp:319-329`.
    /// 왼쪽 점만 찍혀 있으면 그것부터 지우고, 아니면 마지막 점 쌍을 지운다.
    pub fn remove_last_pair(&mut self) -> bool {
        if self.pending_source.take().is_some() {
            return true;
        }
        self.pairs.pop().is_some()
    }

    /// C++ `KaAlignMapTool::endSession()` — `KaAlignMapTool.cpp:177-198`
    pub fn end_session(&mut self) {
        self.pairs.clear();
        self.pending_source = None;
        self.applied = false;
        self.transform = None;
        self.target_layer = None;
        self.saved_path = None;
    }
}
