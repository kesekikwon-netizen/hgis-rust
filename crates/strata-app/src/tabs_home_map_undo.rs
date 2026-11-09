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
