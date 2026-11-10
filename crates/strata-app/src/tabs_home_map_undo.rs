//! `KaCommandStack` — 실행취소/다시실행 명령 스택.
//!
//! `tabs_home_map.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 상수·명령 엔트리·스택이 서로 붙어 있어야 해서 한 파일에 모았다.
//! 옮긴 항목의 이름·시그니처·동작은 그대로다.

pub const HGIS_BASEMAP_DEFAULT_OPACITY: f32 = 1.0;
pub const HGIS_BASEMAP_DEFAULT_BRIGHTNESS: f32 = 0.0;

/// C++ `HGIS_UNDO_MAX_DEPTH` (50) — `KaCommandStack` Undo 히스토리 최대 보관 단계
pub const HGIS_UNDO_MAX_DEPTH: usize = 50;

/// C++ `HGIS_CMD_VERTEX_DRAG_ID` (1001) — 연속 정점 드래그 병합 커맨드 ID
pub const HGIS_CMD_VERTEX_DRAG_ID: i32 = 1001;

/// C++ `MoveVertexCommand` / `QUndoCommand` 1:1 대응 명령 엔트리
#[derive(Debug, Clone, PartialEq)]
pub struct UndoCommandEntry {
    pub command_id: i32,
    pub feature_id: i64,
    pub vertex_index: usize,
    pub target_xy: [f64; 2],
}

/// C++ `KaCommandStack` (`QUndoStack` 래퍼) 1:1 대응:
/// - 최대 50단계(`HGIS_UNDO_MAX_DEPTH`) 상한 유지 (초과 시 가장 오래된 명령 즉시 제거)
/// - 연속 정점 드래그(`HGIS_CMD_VERTEX_DRAG_ID`) 시 동일 `feature_id` + `vertex_index`면 `mergeWith` 병합
/// - 백그라운드 파일 저장 중 `locked` 가드로 `undo()`/`redo()` 차단 및 빈 스택 호출 무음 무시
#[derive(Debug, Clone, Default)]
pub struct KaCommandStack {
    pub undo_stack: std::collections::VecDeque<UndoCommandEntry>,
    pub redo_stack: Vec<UndoCommandEntry>,
    pub locked: bool,
}

impl KaCommandStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, cmd: UndoCommandEntry) {
        if self.locked {
            return;
        }
        self.redo_stack.clear();
        if cmd.command_id == HGIS_CMD_VERTEX_DRAG_ID {
            if let Some(last) = self.undo_stack.back_mut() {
                if last.command_id == HGIS_CMD_VERTEX_DRAG_ID
                    && last.feature_id == cmd.feature_id
                    && last.vertex_index == cmd.vertex_index
                {
                    last.target_xy = cmd.target_xy;
                    return;
                }
            }
        }
        if self.undo_stack.len() >= HGIS_UNDO_MAX_DEPTH {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(cmd);
    }

    pub fn undo(&mut self) -> Option<UndoCommandEntry> {
        if self.locked {
            return None;
        }
        let cmd = self.undo_stack.pop_back()?;
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }

    pub fn redo(&mut self) -> Option<UndoCommandEntry> {
        if self.locked {
            return None;
        }
        let cmd = self.redo_stack.pop()?;
        if self.undo_stack.len() >= HGIS_UNDO_MAX_DEPTH {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(cmd.clone());
        Some(cmd)
    }
}

/// 링 전체를 되돌리는 스냅샷 한 장.
///
/// ## 왜 `UndoCommandEntry` 로는 부족한가
///
/// `UndoCommandEntry` 는 `target_xy`(옮긴 뒤 좌표)만 들고 있다. 그래서
/// 실행취소를 해도 **되돌릴 좌표를 모른다** — "무엇을 했는지"는 알지만
/// "하기 전이 어디였는지"가 없다. C++ `QUndoCommand` 는 `undo()` 안에서
/// 이전 값을 들고 되돌리는데, 그 값을 담을 자리가 없었던 것이다.
///
/// 그래서 편집 전 링을 통째로 담는 스냅샷을 따로 둔다. 면 하나가 수십 점이라
/// 50단계를 담아도 메모리 부담이 작고, 어떤 편집이든(이동·삽입·삭제·그리기)
/// 같은 방식으로 되돌릴 수 있다.
#[derive(Debug, Clone, PartialEq)]
pub struct RingSnapshot {
    /// 어느 조사구역인지(`survey_rings` 인덱스).
    pub feature_index: usize,
    /// 편집 **전** 링. `None` 이면 그 전에 링이 없었다(새로 그린 경우).
    pub before: Option<Vec<[f64; 2]>>,
}

/// 링 스냅샷 스택. C++ `QUndoStack` 의 push/undo/redo 에 대응.
#[derive(Debug, Clone, Default)]
pub struct RingUndoStack {
    undo: std::collections::VecDeque<RingSnapshot>,
    redo: Vec<RingSnapshot>,
    pub locked: bool,
}

impl RingUndoStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 편집 전 상태를 넣는다. 새 편집을 하면 redo 는 비워진다(표준 동작).
    pub fn push(&mut self, snap: RingSnapshot) {
        if self.locked {
            return;
        }
        self.redo.clear();
        if self.undo.len() >= HGIS_UNDO_MAX_DEPTH {
            self.undo.pop_front();
        }
        self.undo.push_back(snap);
    }

    /// 되돌릴 스냅샷을 꺼낸다. 호출부가 링을 `before` 로 되돌린다.
    pub fn undo(&mut self) -> Option<RingSnapshot> {
        if self.locked {
            return None;
        }
        let s = self.undo.pop_back()?;
        self.redo.push(s.clone());
        Some(s)
    }

    /// 다시 실행 스냅샷을 꺼낸다. 호출부가 링을 `after` 로 되돌린다.
    ///
    /// `redo` 스택에는 "그 편집을 한 뒤의 모습"이 필요하므로 호출부가
    /// 현재 링을 넘겨 준다.
    pub fn pop_redo(&mut self) -> Option<RingSnapshot> {
        if self.locked {
            return None;
        }
        self.redo.pop()
    }

    /// 다시 실행용으로 현재 상태를 되돌림 스택에 되돌려 놓는다.
    pub fn push_redo(&mut self, snap: RingSnapshot) {
        self.redo.push(snap);
    }

    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.locked && !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.locked && !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(i: usize, pts: Vec<[f64; 2]>) -> RingSnapshot {
        RingSnapshot {
            feature_index: i,
            before: Some(pts),
        }
    }

    #[test]
    fn push_and_undo_return_previous_state() {
        let mut st = RingUndoStack::new();
        assert!(!st.can_undo());
        st.push(snap(0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]));
        assert!(st.can_undo());
        let got = st.undo().expect("되돌릴 것이 있다");
        assert_eq!(got.feature_index, 0);
        assert_eq!(
            got.before.expect("이전 상태").len(),
            3,
            "편집 전 점 3개를 돌려줘야"
        );
        assert!(!st.can_undo(), "한 번 되돌리면 비어야");
        assert!(st.can_redo(), "다시 실행 가능");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut st = RingUndoStack::new();
        st.push(snap(0, vec![]));
        st.undo();
        assert!(st.can_redo());
        st.push(snap(1, vec![]));
        assert!(!st.can_redo(), "새 편집을 하면 다시 실행은 사라진다");
    }

    #[test]
    fn depth_is_capped_at_50() {
        let mut st = RingUndoStack::new();
        for i in 0..60 {
            st.push(snap(i, vec![]));
        }
        assert_eq!(st.undo_len(), HGIS_UNDO_MAX_DEPTH, "50단계까지만 보관");
        // 가장 오래된 것은 밀려났고 최근 것이 남는다.
        let last = st.undo().expect("가장 최근");
        assert_eq!(last.feature_index, 59);
    }

    #[test]
    fn locked_blocks_undo_and_push() {
        let mut st = RingUndoStack::new();
        st.push(snap(0, vec![]));
        assert_eq!(st.undo_len(), 1, "잠기기 전 push 는 들어간다");
        st.locked = true;
        assert!(!st.can_undo(), "잠기면 못 되돌린다");
        assert!(st.undo().is_none());
        st.push(snap(1, vec![]));
        assert_eq!(st.undo_len(), 1, "잠긴 동안 push 는 무시된다(늘지 않음)");
    }

    #[test]
    fn new_ring_has_no_before() {
        let mut st = RingUndoStack::new();
        st.push(RingSnapshot {
            feature_index: 3,
            before: None,
        });
        let got = st.undo().expect("되돌릴 것");
        assert!(got.before.is_none(), "새로 그린 링은 '전'이 없다(되돌리면 지운다)");
    }
}
