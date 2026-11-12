//! 지도 범위 이력 — QGIS `QgsMapCanvas` 의 mLastExtent·mLastExtentIndex(qgsmapcanvas.cpp setExtent
//! :1629-1650, zoomToPreviousExtent :1745-1758, zoomToNextExtent :1760-1774, clearExtentHistory
//! :1776-1784)와 같은 규칙. C++ 「이전·다음 지도 범위」(MainWindow.cpp:686-689)가 이 상태를 쓴다.
//!
//! 이력은 작업 파일(.qgz·내장 프로젝트)을 읽을 때만 시작된다(QgsMapCanvas::readProject 만
//! clearExtentHistory 를 부른다). 그 전에는 범위를 바꿔도 아무 것도 쌓이지 않는다 — qgis-dev
//! 4.3.0-Master 로 확인했다(`extent_history_tests.rs`). Rust 에는 아직 작업공간 복원(지시 2D)이 없어
//! `seed` 를 부르는 곳이 없다.

/// 범위 [xmin, ymin, xmax, ymax](작업 좌표계).
pub(crate) type Extent = [f64; 4];

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Default)]
pub(crate) struct ExtentHistory {
    list: Vec<Extent>,
    /// 지금 범위의 자리. 비었으면 0 이고 `list` 도 비어 있다(QGIS 는 -1).
    index: usize,
}

impl ExtentHistory {
    /// `clearExtentHistory` — 지금 범위 하나로 다시 시작한다(작업 파일을 읽은 뒤). 작업공간 복원(지시 2D)이 부른다.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn seed(&mut self, current: Extent) {
        self.list = vec![current];
        self.index = 0;
    }

    /// `setExtent` 의 뒷부분 — 지금 자리 뒤 이력을 지우고, 마지막과 다르면 쌓는다(비었으면 쌓지 않는다).
    pub(crate) fn note_extent(&mut self, current: Extent) {
        if self.list.is_empty() {
            return;
        }
        self.list.truncate(self.index + 1);
        if self.list.last() != Some(&current) {
            self.list.push(current);
        }
        if self.list.len() > MAX_HISTORY {
            self.list.remove(0);
        }
        self.index = self.list.len() - 1;
    }

    /// `zoomLastStatusChanged` 의 값.
    pub(crate) fn can_previous(&self) -> bool {
        !self.list.is_empty() && self.index > 0
    }

    /// `zoomNextStatusChanged` 의 값.
    pub(crate) fn can_next(&self) -> bool {
        !self.list.is_empty() && self.index + 1 < self.list.len()
    }

    /// `zoomToPreviousExtent` — 돌아갈 범위(이력은 쌓지 않는다).
    pub(crate) fn previous(&mut self) -> Option<Extent> {
        if !self.can_previous() {
            return None;
        }
        self.index -= 1;
        Some(self.list[self.index])
    }

    /// `zoomToNextExtent`.
    pub(crate) fn next(&mut self) -> Option<Extent> {
        if !self.can_next() {
            return None;
        }
        self.index += 1;
        Some(self.list[self.index])
    }
}

#[cfg(test)]
#[path = "extent_history_tests.rs"]
mod tests;
