# Vertex Editing in the C++ App vs. the Rust App (`VTX0`)

Read-only rule-by-rule audit of vertex editing in `target/cpp-ref` (C++/QGIS Strata) against the Rust port on integration branch `ag/vtx0` (`docs/RUST_PLAN.md` §4, order `3-6`).

Important architectural note: in C++, `KaVertexEditTool` is marked `"standalone tool, no button"` (`KaVertexEditTool.h:51`, `KaVertexEditTool.h:30`). The user never switches the map canvas to `KaVertexEditTool` directly; instead, `KaFeatureSelectTool` owns a `KaVertexEditTool* m_vertex` child (`KaFeatureSelectTool.cpp:39`, `KaFeatureSelectTool.cpp:41-43`) and handles canvas mouse/key/menu events while delegating geometry queries, previews, snaps, and edits to `m_vertex`. Both classes are audited below because `KaVertexEditTool`'s own event methods (`canvasPressEvent`, `canvasMoveEvent`, `canvasReleaseEvent`, `canvasDoubleClickEvent`, `keyPressEvent`, `showLineVertexMenu`) also exist and carry slightly different fallback rules and status strings.

---

## 1. Entering and Leaving

### C++ (`target/cpp-ref/src/`)

#### How the user enters
1. **Ribbon button `선택` (`Ctrl+1`)**: `m_actSelect` (`MainWindowRibbon.cpp:300-307`) triggers `MainWindow::startSelectTool` (`MainWindowEditing.cpp:182`). Shortcut is `QKeySequence(QStringLiteral("Ctrl+1"))` (`MainWindowRibbon.cpp:301`); tooltip is `그린 도형을 선택합니다. 다시 누르면 이동으로 돌아갑니다` (`MainWindowRibbon.cpp:304`).
2. **Single-key shortcut `A` (`actSelectShapeKey`)**: bound at `MainWindowRibbon.cpp:309-310`, dispatched via `KaFeatureSelectPick::shortcutKeyPress` (`KaFeatureSelectPick.cpp:91-127`). Only fires when focus is not in an editable text widget (`KaFeatureSelectPick.cpp:107-112`), not already on `m_featureSelectTool` (`:114`), not in `KaMeasureMapTool`/`KaAlignMapTool` (`:117`), `keyOwnedElsewhere` is false (`:121`), and `!capture->hasSketch()` (`:124`).
3. **Draw sub-toolbar `도형선택`**: added at `MainWindowRibbon.cpp:856-865` calling `startSelectTool`, highlighted by `MainWindowRibbon.cpp:743-744` when `m_canvas->mapTool() == m_featureSelectTool`.
4. **Layer panel right-click menu `꼭짓점 수정`**: shown on editable non-`Point` vector layers (`MainWindowContextMenus.cpp:263-268`). If `m_featureSelectTool` is not active, calls `startSelectTool()` (`:266`) and displays `고칠 도형을 클릭한 뒤 꼭짓점을 끌어 옮기세요.` for `8000` ms (`:267`).
5. **`MainWindow::startSelectTool` setup (`MainWindowEditing.cpp:182-239`)**: lazily creates `m_featureSelectTool`, syncs `m_snapEnabled` (`:230`), pins planar ellipsoid (`:231`), sets `m_canvas->setMapTool(m_featureSelectTool)` (`:232`), focuses canvas (`:233`), sets sub-toolbar chip `도형선택` (`:235`, `KaDrawSketchTools.cpp:200`), and shows `고칠 도형을 클릭한 뒤 꼭짓점을 끌어 옮기세요.` for `10000` ms (`:236-238`).
6. **Clicking a feature to expose handles**: left-click in `KaFeatureSelectTool` runs pick/cycle (`KaFeatureSelectPick.cpp:150-208`), then `syncVertexTarget()` (`KaFeatureSelectTool.cpp:111-119`) calls `m_vertex->setTarget(all[0].layer, all[0].fid)` only when `all.size() == 1` and `isPickableLayer(all[0].layer)` is true. On success (and `!pieceHit`), emits `수정점이 나왔습니다. 점을 끌어 옮기세요(Ctrl=자석). 점 우클릭은 삭제, 선 우클릭은 추가입니다.` (`KaFeatureSelectPick.cpp:201-202`). **Double-click** does *not* enter vertex editing in `KaFeatureSelectTool` (no `canvasDoubleClickEvent` override in `KaFeatureSelectTool.cpp`).

#### How the user leaves
1. **Press `선택` / `도형선택` again (`MainWindowEditing.cpp:224-228`)**: if `m_canvas->mapTool() == m_featureSelectTool`, switches to `m_panTool` and shows `도형 선택 종료` for `3000` ms. Note: pressing `A` does **not** toggle off when already in `KaFeatureSelectTool` (`KaFeatureSelectPick.cpp:114`).
2. **Switch to another map tool**: `QgsMapCanvas::setMapTool(...)` deactivates `KaFeatureSelectTool` and updates checked state via `mapToolSet` (`MainWindow.cpp:722-724`, `:751-752`).
3. **Click elsewhere (empty space or >1 features)**: `syncVertexTarget()` (`KaFeatureSelectTool.cpp:111-119`) calls `m_vertex->clearTarget()`, removing vertex markers/rubber band; empty click leaves `선택된 도형 없음` (`KaFeatureSelectPick.cpp:187`).
4. **Press `Esc`**: in `KaFeatureSelectTool::keyPressEvent` (`KaFeatureSelectTool.cpp:164-170`), when `m_vertex->activeVertex() >= 0 || m_activePiece.isValid()`, `Esc` clears `setActiveVertex(-1)`, `setActivePiece({})`, and `m_lastPickIndex = -1`, keeping the select tool active and the feature's vertex handles visible. (`MainWindow*.cpp` has no separate `Key_Escape` exit handler for `m_featureSelectTool`; standalone `KaVertexEditTool.cpp:507-510` would clear selection with `선택을 풀었습니다.`).

#### Which layers / features refuse and verbatim messages
- **Pick filter for vertex handles (`KaFeatureSelectTool::isPickableLayer`, `KaFeatureSelectTool.cpp:67-69`)**: delegates to `FeaturePick::isSurveyLayer(layer)` (`FeaturePick.cpp:58-62`), which rejects `isReferenceOrBasemapLayer(layer)`, `isCadastralLayer(layer)`, and `isReferenceLayer(layer)`. When 1 feature is selected and `!m_vertex->hasTarget()`, status is `참고 자료 도형은 고칠 수 없습니다(보기만).` (`KaFeatureSelectPick.cpp:199-200`).
- **Editable target filter inside `KaVertexEditTool::selectAt` (`KaVertexEditTool.cpp:276-285`)**: only vector layers with `!layer->readOnly()`, geometry type `Line` or `Polygon` (not `Point` / `Null`), and `LayerOps::layerKey(layer)` in `{survey_area, feature_poly, feature_line, section_line, trial_trench}` accept vertex targets.
- **Read-only layer guard in `LayerOps::refuseReadOnlyKinds` (`GeometryEditVertex.cpp:22-32`)**: cadastral layer fails with `지적도는 고칠 수 없습니다.`; reference/basemap layer fails with `참조 지도는 고칠 수 없습니다.`. (`LayerOps::runEditCommand`, `LayerOps.cpp:2660-2663`, emits `지적도는 편집할 수 없습니다.` and `참조 지도는 편집할 수 없습니다.`).
- **Context-menu guards on invalid/unsupported/multi selections (`KaFeatureSelectTool.cpp:316-347`)**: `같은 레이어의 도형만 선택해 주세요.`, `도형이 있는 레이어를 다시 열어 주세요.`, `조사 데이터의 도형을 선택해 주세요.`, `읽기 전용 레이어는 수정할 수 없습니다.`, `이 자료는 해당 편집 기능을 지원하지 않습니다.`, `꼭짓점을 고칠 도형 하나만 선택해 주세요.`.

### Rust (`ag/vtx0`)

- **Entering select**: `선택` ribbon button (`gui_ribbon_click_a.rs:63-72`), `도형선택` sub-toolbar button (`gui_ribbon_draw_row.rs:138-142`), `A` key (`gui_shell.rs:78-80`), and `Ctrl+1` (`gui_shell.rs:133-135`) all set `active_tool = MapToolKind::Select`.
- **Differences on entry**: `A` fires unconditionally in `gui_shell.rs:78-80` even if a sketch is open; `도형선택` in `gui_ribbon_draw_row.rs:138-142` only sets `Select` and never toggles back to `Pan` (whereas `선택` in `gui_ribbon_click_a.rs:63-66` toggles to `Pan` with `도형 선택 종료`); `gui_ribbon_click_a.rs:70` sets `status_bar_hint` with `"점을 찍은 뒤 Delete로 지웁니다."` instead of C++ `MainWindowEditing.cpp:237` `"점을 클릭한 뒤 Delete로 지웁니다."` (`도형선택` / `A` / `Ctrl+1` omit this status text altogether); `"꼭짓점 수정"` (`layer.vertices`) in `layer_menu_model.rs:220` is disabled (`deferred(..., "CM2a-2")`) with no handler in `gui_layer_panel_context_menu.rs`.
- **Feature pick (`feature_pick.rs:1-104`)**: picks only `self.survey_tables` (`survey: true`), cycles overlaps, and skips vertex handles (`feature_pick.rs:101`: `// 한 개만 골랐을 때 C++ 은 「수정점이 나왔습니다…」라고 하지만 꼭짓점 편집이 아직 없어 말하지 않는다.`). Reference/cadastral layers are never pickable in `feature_pick.rs` (`selected_shape` stays `None` -> `선택된 도형 없음`), so `"참고 자료 도형은 고칠 수 없습니다(보기만)."` never appears.
- **Leaving**: `Esc` in `gui_shell.rs:34-39` unconditionally forces `self.active_tool = MapToolKind::Pan` and hides `sub_toolbar_visible` instead of clearing only an active vertex.
- **Vertex handles / edits**: NOT BUILT (`snap_vertex_ops.rs:9-54` has unused polygon/line vertex deletion helpers with zero callers in `strata-app`).

---

## 2. Rule-by-Rule Table

> Note on columns: `C++ quote` quotes a short piece of the cited C++ line in backticks. When C++ quotes Korean UI strings, the verbatim Korean string also appears in `what happens` so both C++ and Rust line checkers verify directly against their respective files. Rows follow C++ event order: Press -> Move -> Release -> Double click -> Key press -> Context menu.

| # | event | condition | what happens | number with its unit | C++ file:line | C++ quote | Rust: SAME, DIFFERENT or NOT BUILT | Rust file:line | Rust quote | C++ test | Rust test |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | press (left) | `m_vertex->hasTarget()` and `idx = m_vertex->vertexNear(e->mapPoint()) >= 0` | Starts vertex drag without re-picking feature: sets `m_vertexDragging = true`, `m_vertexMoved = false`, `m_vertexIndex = idx`, `m_vertexPressPos = e->pos()`, clears `m_activePiece`, calls `setActiveVertex(idx)` | `20 px` (`kVertexGrabPx` via `toLayer`+`layerTolerance`) | KaFeatureSelectTool.cpp:177-187, KaEditTolerance.h:9 | `const int idx = m_vertex->vertexNear(e->mapPoint());` | NOT BUILT | feature_pick.rs:3 | `수정점(꼭짓점 편집)` | test_editing_tools.cpp:228-267 `clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | none |
| 2 | press (left) | `m_vertex` has no hit (`idx < 0` or `!hasTarget()`) | Clears active vertex `setActiveVertex(-1)` and stores `m_pressPos` / `m_pressMapPt` for feature click/box selection | `0` handles hit (`> 20 px`) | KaFeatureSelectTool.cpp:190-193 | `m_vertex->setActiveVertex(-1);` | DIFFERENT (always starts feature pick/box path, no active vertex) | gui_tab_map_canvas.rs:140-155 | `MapToolKind::Select` | test_editing_tools.cpp:199-226 `selectPrefersSmallShapeAndCyclesOverlaps` | feature_pick_tests.rs:69 `select_on_demo_cycles_overlaps_like_cpp` |
| 3 | press (left, standalone `KaVertexEditTool`) | Standalone tool active, vertex hit (`idx >= 0`) | Records `m_dragging = true`, `m_moved = false`, `m_dragIndex = idx`, `m_pressPos = e->pos()`, calls `setActiveVertex(idx)` (does not delete on press) | `20 px` (`kVertexHitTolerancePx`) | KaVertexEditTool.cpp:450-459, KaVertexEditTool.h:61 | `setActiveVertex(idx);` | NOT BUILT | none | — | test_workflow.cpp:1476-1481 | none |
| 4 | press (left, standalone `KaVertexEditTool`) | Standalone tool active, `idx < 0` | Calls `selectAt(e->mapPoint())` to target editable feature (`survey_area`, `feature_poly`, `feature_line`, `section_line`, `trial_trench`) | `10 px` (`kFeatureSelectPx` via `selectTolerance`) | KaVertexEditTool.cpp:271-290, :460-467 | `if (selectAt(e->mapPoint()))` | NOT BUILT | none | — | none | none |
| 5 | press (left, pick result) | Left release click (`<= 4 px`) picks 1 non-pickable reference/cadastral shape (`!m_vertex->hasTarget()`) | Emits `참고 자료 도형은 고칠 수 없습니다(보기만).` (`8000` ms) and shows no vertex handles | `4 px` (`kClickSlopPx`), `10 px` (`kFeaturePickPx`), `8000` ms | KaFeatureSelectPick.cpp:158-200, KaFeatureSelectTool.cpp:67-69 | `isPickableLayer(all[0].layer)` | DIFFERENT (filters to `survey_tables` only; reference click reports `선택된 도형 없음`) | feature_pick.rs:58-65, :88-100 | `survey_tables` | none | none |
| 6 | press (left, pick result) | Left release click (`<= 4 px`) picks 1 editable survey shape (`!pieceHit`) | Sets `m_vertex->setTarget(all[0].layer, all[0].fid)` and emits `수정점이 나왔습니다. 점을 끌어 옮기세요(Ctrl=자석). 점 우클릭은 삭제, 선 우클릭은 추가입니다.` | `1` shape, `<= 4 px` slop (`kClickSlopPx`), `8000` ms | KaFeatureSelectTool.cpp:111-119, KaFeatureSelectPick.cpp:158, :201-202 | `m_vertex->setTarget(all[0].layer, all[0].fid);` | DIFFERENT (selects feature and shows yellow fill, omits vertex target & hint) | feature_pick.rs:88-104 | `selected_shape = Some` | none | feature_pick_tests.rs:69 `select_on_demo_cycles_overlaps_like_cpp` |
| 7 | move | `m_vertexDragging` and Manhattan distance from `m_vertexPressPos` `<= 4 px` (`kClickSlopPx`) | Ignores micro-jitter before drag begins; does not preview or mark moved | `4 px` (`kClickSlopPx`) Manhattan distance | KaFeatureSelectTool.cpp:200-202, KaEditTolerance.h:13 | `if (!m_vertexMoved && (e->pos() - m_vertexPressPos).manhattanLength() <= KaEditTolerance::kClickSlopPx)` | NOT BUILT | none | — | none | none |
| 8 | move | `m_vertexDragging` and Manhattan distance `> 4 px` | Sets `m_vertexMoved = true` and previews move `previewVertexMove(m_vertexIndex, m_vertex->toLayer(dragPoint(e)))` (moves vertex + closing partner in rubber band & marker only; no layer write) | `> 4 px` slop; vertex `14`/`16 px` | KaFeatureSelectTool.cpp:203-206, KaVertexEditTool.cpp:75-86 | `m_vertex->previewVertexMove(m_vertexIndex, m_vertex->toLayer(dragPoint(e)));` | NOT BUILT | none | — | none | none |
| 9 | move (drag snap) | While dragging (`m_vertexDragging`), `Ctrl` modifier is **not** held | `dragPoint(e)` hides snap marker (`m_snapMark->hide()`) and returns raw `e->mapPoint()` without snapping | `0` snap | KaFeatureSelectTool.cpp:102-109 | `if (!(e->modifiers() & Qt::ControlModifier))` | NOT BUILT | none | — | none | none |
| 10 | move (drag snap) | While dragging (`m_vertexDragging`), `Ctrl` modifier **is** held and `m_snapEnabled` is `true` | Calls `snapMapPointExcludingTarget(e, &snapped)` with `ExcludeFeatureFilter(m_layer, m_fid)` (`Vertex | Segment`); if snapped, shows `14 px` blue `ICON_CIRCLE` snap marker and returns `snapped` | `16.0 px` default snap tolerance (`tolerancePx`), `14 px` `ICON_CIRCLE` | KaFeatureSelectTool.cpp:90-109, KaVertexEditTool.cpp:225-241, LayerOps.cpp:2889 | `m_vertex->snapMapPointExcludingTarget(e, &snapped)` | NOT BUILT on `ag/vtx0` (and `3-7` rejected because it snapped on Ctrl even with magnet off) | cad_session.rs:30-35 | `snap_tolerance_px: 12.0` | none | none |
| 11 | move (drag snap) | While dragging (`m_vertexDragging`), `Ctrl` modifier is held and `m_snapEnabled` is `false` (magnet off) | `snapMapPointExcludingTarget` returns raw `pt` immediately (`if (!m_snapEnabled || !mCanvas->snappingUtils()) return pt;`), so Ctrl drag does **not** snap | `0` snap | KaVertexEditTool.cpp:235 | `if (!m_snapEnabled \|\| !mCanvas->snappingUtils()) return pt;` | NOT BUILT on `ag/vtx0` (`3-7` `app_state_draw_snap.rs:119-127` differed and was sent back as `3-7b`) | cad_session.rs:110-120 | `if !self.snap_enabled` | none | none |
| 12 | release (left) | `m_vertexDragging && !m_vertexMoved` (click on handle without drag) | Hides snap mark, clears `m_vertexDragging`, edits nothing (`0` undo entries), keeps handle active, emits `점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다.` | `<= 4 px` (`kClickSlopPx`), `6000` ms | KaFeatureSelectTool.cpp:237-246 | `emit statusMessage(QStringLiteral(...));` | NOT BUILT | none | — | test_editing_tools.cpp:228-246 `clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | none |
| 13 | release (left) | `m_vertexDragging && m_vertexMoved` (drag ended) | Computes `target = m_vertex->toLayer(dragPoint(e))`, hides snap mark, calls `moveVertexTo(idx, target)`, redraws markers & canvas; on success emits `수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다.` | `> 4 px` move; `0.001` map units (`1 mm`) topological partner tolerance | KaFeatureSelectTool.cpp:247-257, GeometryEditVertex.cpp:20, :114-137 | `const bool ok = m_vertex->moveVertexTo(idx, target);` | NOT BUILT | edit_buffer.rs:47-63 | `pub fn change_geometry` | test_inner_piece.cpp:124, test_save_open.cpp:3386, test_buffer.cpp:199, test_layer_edits.cpp:108 | none |
| 14 | release (left, standalone `KaVertexEditTool`) | Standalone drag moved beyond `4 px` (`> kClickSlopPx`) | Calls `moveVertexTo(idx, target)` and emits `꼭짓점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다.` (`8000` ms) | `> 4 px`, `8000` ms | KaVertexEditTool.cpp:478-491 | `if (moveVertexTo(idx, target))` | NOT BUILT | none | — | none | none |
| 15 | double click (left) | `KaFeatureSelectTool` active on canvas | No `canvasDoubleClickEvent` handler; canvas processes normal click/release (no vertex insertion on double-click in `도형선택`) | `0` edge inserts | KaFeatureSelectTool.cpp:1-499 | `canvasReleaseEvent` | SAME (no vertex-insert on double-click in `Select`; `double_clicked()` only finishes capture) | gui_tab_map_capture.rs:45 | `response.double_clicked()` | none | none |
| 16 | double click (left, standalone `KaVertexEditTool`) | Standalone tool active, `segmentNear(mapPt)` returns `afterIndex >= 0` | Inserts vertex via `insertVertexAt(afterIndex, snapMapPoint(e))` and emits `꼭짓점을 넣었습니다.` (`8000` ms) | `16 px` (`kSegmentTolerancePx`), `8000` ms | KaVertexEditTool.cpp:262, :494-503, KaVertexEditTool.h:62 | `if (insertVertexAt(after, onLine))` | NOT BUILT | none | — | none | none |
| 17 | key press (`Delete` / `Backspace`) | `KaFeatureSelectTool` active, `!m_vertexDragging`, `m_vertex->activeVertex() >= 0` | MainWindow canvas filter routes to `deleteActivePick()` -> `deleteActiveVertex()`: calls `deleteVertexAt(idx)`, refreshes markers & canvas, emits `점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다.` (`8000` ms). Consumed before feature deletion | `1` vertex removed | MainWindow.cpp:2372-2377, MainWindowUndo.cpp:497-499, KaFeatureSelectTool.cpp:71-83, :160-163 | `m_vertex->deleteVertexAt(idx)` | NOT BUILT (`Delete`/`Backspace` only removes sketch point in capture/align) | gui_shell.rs:82-88 | `if capture && (i.key_pressed(egui::Key::Delete)` | test_editing_tools.cpp:228-267 `clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | none (only `polygon_ops_tests.rs:106` `test_delete_vertex_triangle_blocked_and_ortho_45_snap`) |
| 18 | key press (`Delete` / `Backspace`, standalone `KaVertexEditTool`) | Standalone tool active, no `activeVertex()` picked (`idx < 0`) | Falls back to `vertexNear(mapPt)` at current global cursor position and deletes that hovered vertex (`꼭짓점을 지웠습니다.` if succeeded) | `20 px` (`kVertexHitTolerancePx`) | KaVertexEditTool.cpp:512-523 | `idx = vertexNear(mCanvas->getCoordinateTransform()->toMapCoordinates(cursor));` | NOT BUILT | none | — | none | none |
| 19 | key press (`Esc`) | `KaFeatureSelectTool` active and `m_vertex->activeVertex() >= 0 || m_activePiece.isValid()` | Clears active vertex (`setActiveVertex(-1)`), active piece (`setActivePiece({})`), and `m_lastPickIndex = -1`; keeps `KaFeatureSelectTool` and feature selection/handles active | `0` edits | KaFeatureSelectTool.cpp:164-170 | `m_vertex->setActiveVertex(-1);` | DIFFERENT (`Esc` unconditionally resets `active_tool = MapToolKind::Pan` and closes sub-toolbar) | gui_shell.rs:34-39 | `self.active_tool = MapToolKind::Pan;` | none | none |
| 20 | key press (`Esc`, standalone `KaVertexEditTool`) | Standalone tool active | Calls `clearTarget()`, clears layer selection (`m_layer->removeSelection()`), emits `선택을 풀었습니다.` (`4000` ms) | `4000` ms | KaVertexEditTool.cpp:507-510 | `clearSelection();` | NOT BUILT | none | — | none | none |
| 21 | context menu (right-click release) | Right-click release (`button == Qt::RightButton`), not dragging, selection is empty | Auto-selects topmost pickable/candidate shape at cursor (`allSelectedFeatures(cursorPt)`); if still empty, delegates to `requestMapContextMenu(e->globalPos(), e->mapPoint())` | `<= 4 px` slop; `10 px` pick (`kFeaturePickPx`) | KaFeatureSelectTool.cpp:228-231, :276-288 | `emit requestMapContextMenu(e->globalPos(), e->mapPoint());` | DIFFERENT (does not auto-pick on right-click; opens `map_context_menu` directly, only suppresses on legacy `selected_feature_idx`) | gui_tab_map_context.rs:124-135 | `open_map_context_menu` | none | none |
| 22 | context menu (vertex hit) | 1 editable survey `Line`/`Polygon` shape selected and `vertexNear(mapPt, 24 px) >= 0` | Enables `점삭제` if ring/line `> minCount`, disables with exact limit reason (`면은 꼭짓점 3개보다 줄일 수 없습니다.` / `선은 꼭짓점 2개보다 줄일 수 없습니다.`) if at limit; triggering emits `점을 지웠습니다.` | `24 px` (`kVertexMenuPx`), min `5` ring vertices / `3` line vertices | KaFeatureSelectTool.cpp:309-313, :333-339, :355-414, KaEditTolerance.h:10 | `KaEditTolerance::kVertexMenuPx` | NOT BUILT | gui_tab_map_context.rs:64-105 | `open_map_context_menu` | test_inner_piece.cpp:231-235, test_save_open.cpp:3395 | none |
| 23 | context menu (edge hit) | 1 editable survey `Line`/`Polygon` shape selected, `vertexNear(24 px) < 0`, `segmentNear(16 px, &onLine) >= 0` | Adds `점추가`; triggering inserts vertex at projected `onLine` after `segAfter`, clears active piece, refreshes markers & canvas, emits `점을 넣었습니다.` | `16 px` (`kSegmentPx`), `0` auto-edits until clicked | KaFeatureSelectTool.cpp:312-313, :355-414, KaEditTolerance.h:12 | `m_vertex->insertVertexAt(segAfter, onLine)` | NOT BUILT | gui_tab_map_context.rs:64-105 | `open_map_context_menu` | test_save_open.cpp:3391, test_workflow.cpp:1468-1475 | none |
| 24 | context menu (vertex/edge menu guards) | Right-click on selected feature(s) that fail editable single-feature checks | Refuses `점추가`/`점삭제` with disabled action tooltip or warning: `같은 레이어의 도형만 선택해 주세요.`, `도형이 있는 레이어를 다시 열어 주세요.`, `조사 데이터의 도형을 선택해 주세요.`, `읽기 전용 레이어는 수정할 수 없습니다.`, `이 자료는 해당 편집 기능을 지원하지 않습니다.`, `꼭짓점을 고칠 도형 하나만 선택해 주세요.`, `꼭짓점을 넣을 선이나 면의 가장자리에서 우클릭하세요.`, `지울 꼭짓점 가까이에서 우클릭하세요.` | `1` editable shape required | KaFeatureSelectTool.cpp:316-347 | `disableReason = QStringLiteral("같은 레이어의 도형만 선택해 주세요.");` | NOT BUILT | none | — | none | none |
| 25 | context menu (standalone `KaVertexEditTool::showLineVertexMenu`) | Standalone tool right-click | Offers **both** `점추가` (`28 px` fallback if `segmentNear` >= 0) and `점삭제` (`28 px` `kFarTolerancePx`) in one menu; shows `꼭짓점을 추가하거나 삭제할 수 없습니다.` if both disabled | `28 px` (`kFarTolerancePx`) fallback search | KaVertexEditTool.cpp:418-448 | `showLineVertexMenu` | NOT BUILT | none | — | none | none |

---

## 3. What an Edit Does to the Data

### Call Chains: Move, Insert, Delete

1. **Move (`KaVertexEditTool::moveVertexTo`, `KaVertexEditTool.cpp:334`)**
   - `KaFeatureSelectTool::canvasReleaseEvent` (`KaFeatureSelectTool.cpp:250`) -> `KaVertexEditTool::moveVertexTo(index, target)` -> `LayerOps::runEditCommand(m_layer, QStringLiteral("꼭짓점 이동"), ...)` (`LayerOps.cpp:2652-2679`) -> `LayerOps::applyVertexMove(m_layer, m_fid, index, target, m_topologicalEditing, err)` (`GeometryEditVertex.cpp:76`).
   - Standalone programmatic helper: `LayerOps::moveFeatureVertex(layer, fid, vertexIndex, x, y, err)` (`GeometryEditVertex.cpp:35`) starts editing if `!layer->isEditable()` (`startEditing()`, `rollBack()` on error) and calls `applyVertexMove` (`topologicalEditing = false`).
   - Rust today: `LayerEditBuffer::change_geometry(&mut self, fid, wkt_geom) -> bool` (`crates/strata-storage/src/edit_buffer.rs:47-63`) exists and works for `Polygon`/`LineString` edits (`app_state_snap_lines.rs:114`), but has **zero** callers from `Select` or any interactive vertex tool; `snap_vertex_ops.rs` has no move or insert functions.

2. **Insert (`KaVertexEditTool::insertVertexAt`, `KaVertexEditTool.cpp:340-361`)**
   - Finds insertion `QgsVertexId beforeId` via `geom.vertexIdFromVertexNr(afterIndex + 1, beforeId)` -> inserts point with `geom.insertVertex(beforeId, QgsPoint(mapPoint.x(), mapPoint.y()))` -> `applyGeometryChange(geom, QStringLiteral("꼭짓점 추가"))` (`KaVertexEditTool.cpp:392`) -> `LayerOps::runEditCommand(m_layer, title, layer->changeGeometry(before.id(), geom))`.
   - Rust today: NOT BUILT.

3. **Delete (`KaVertexEditTool::deleteVertexAt`, `KaVertexEditTool.cpp:363-390`)**
   - Resolves `QgsVertexId vid` via `geom.vertexIdFromVertexNr(index, vid)` -> checks ring minimum count -> `geom.deleteVertex(vid)` -> `applyGeometryChange(geom, QStringLiteral("꼭짓점 삭제"))` -> `LayerOps::runEditCommand` -> `layer->changeGeometry(...)`.
   - Rust today: `crates/strata-geo/src/snap_vertex_ops.rs:29` (`delete_polygon_vertex_checked`) and `:48` (`delete_line_vertex_checked`) exist as pure `&[[f64; 2]]` helpers, never called in `crates/strata-app`.

### Ring-Closing Partner (`ringClosingVertex`)
- **C++**: `LayerOps::ringClosingVertex(geom, vertexIndex)` (`GeometryEditVertex.cpp:65-74`) resolves `geom.vertexIdFromVertexNr(vertexIndex, id)`. For `Qgis::GeometryType::Polygon` only, reads `count = geom.constGet()->vertexCount(id.part, id.ring)`. If `id.vertex == 0`, partner is `vertexIndex + count - 1`; if `id.vertex == count - 1`, partner is `vertexIndex - (count - 1)`; otherwise `-1`.
- Both live preview (`KaVertexEditTool::previewVertexMove`, `KaVertexEditTool.cpp:80-82`) and committed move (`LayerOps::applyVertexMove`, `GeometryEditVertex.cpp:135-136`) move **both** `vertexIndex` and its partner so the ring stays closed. Proven by `test_polygon_erase.cpp:361-374` (`movingTheFirstVertexKeepsAHoleClosed`).
- **Rust**: NOT BUILT for interactive edits or holes (`snap_vertex_ops.rs:9-54` operates only on flat open slices `&[[f64; 2]]` without ring-closing synchronization).

### Minimum Vertex Count & At-Limit Behaviour
- **C++ (`KaVertexEditTool::deleteVertexAt`, `KaVertexEditTool.cpp:363-374`)**:
  - Polygon ring (`geom.type() == Qgis::GeometryType::Polygon`): `vertexCount(vid.part, vid.ring) < 5` (i.e., `<= 4` including closing vertex = `3` unique corners) refuses deletion with `면은 꼭짓점 3개보다 줄일 수 없습니다.`.
  - Line (`geom.type() == Qgis::GeometryType::Line`): `vertexCount(vid.part, vid.ring) < 3` (i.e., `<= 2` endpoints) refuses deletion with `선은 꼭짓점 2개보다 줄일 수 없습니다.`.
  - Context menu (`KaFeatureSelectTool.cpp:333-339`) also disables `점삭제` with those same strings as tooltips when at the floor.
- **Rust**: `crates/strata-geo/src/snap_vertex_ops.rs:9-12` defines `HGIS_MIN_POLYGON_VERTICES = 4` (closed ring 4 points = 3 unique vertices + closing? Wait: `delete_polygon_vertex_checked` checks `ring.len() <= HGIS_MIN_POLYGON_VERTICES`, meaning `<= 4` stops delete -> leaves at least `4` in an open slice? Wait: `polygon_ops_tests.rs:106` tests `delete_vertex` on a triangle!). Never wired to UI, no Korean error strings in `strata-app`, and unchecked on line vs multi-ring geometries.

### Multi-Part Features and Inner Hole Rings
- **C++**: `KaVertexEditTool::vertexCoords` (`KaVertexEditTool.cpp`) iterates every `(part, ring)` of multi-part and holed geometries; `geom.vertexIdFromVertexNr(index, vid)` (`GeometryEditVertex.cpp:66-74`, `KaVertexEditTool.cpp:344-370`) maps flat vertex indices to `(part, ring, vertex)` so moving/inserting/deleting works independently per outer shell, inner hole ring (`test_polygon_erase.cpp:368-373`), or sub-part, and `minCount` checks the specific `(vid.part, vid.ring)` ring count.
- **Rust**: NOT BUILT (`snap_vertex_ops.rs:29-54` takes a single flat `&[[f64; 2]]`; `app_state_snap_lines.rs:91-101` only touches `parts.first()` -> `rings.first()`).

### Invalid-Geometry & Edit-Command Guards
- **C++**:
  - `LayerOps::refuseReadOnlyKinds` (`GeometryEditVertex.cpp:22-32`) blocks cadastral/reference layers.
  - `KaVertexEditTool::applyGeometryChange` (`KaVertexEditTool.cpp:392`) checks `if (m_layer->isEditCommandActive())` -> `다른 도형 편집이 진행 중입니다. 편집을 마친 뒤 다시 시도하세요.`, and skips writing an undo step if `before.geometry().isExactlyEqual(geom)`.
  - Whether `GeometryEditVertex.cpp:76-156` or `applyGeometryChange` also validates self-intersection via `isGeosValid()` before `changeGeometry()`: **see `Could not determine`** (not observed in the lines opened in this session).
- **Rust**: NOT BUILT in interactive vertex edit (note `edit_geometry_repair.rs` exists for bow-tie repair after polygon capture, `edit_geometry_repair_tests.rs:50`, not connected to vertex editing).

### Undo Entry (`Ctrl+Z`) & Unsaved Mark
- **C++**:
  - `LayerOps::runEditCommand(layer, title, fn)` (`LayerOps.cpp:2652-2679`) starts layer editing (`layer->startEditing()` if not editable) and wraps the mutation in `layer->beginEditCommand(title)` ... `endEditCommand()` (or `destroyEditCommand()` if `fn()` returns false). One drag release, one insert, or one delete creates **one** edit command on `layer->undoStack()` (`"꼭짓점 이동"`, `"꼭짓점 추가"`, `"꼭짓점 삭제"`).
  - `KaFeatureSelectTool::featureGeometryEdited(layer, before)` (`MainWindowEditing.cpp:209-221`) pushes `KaUndoAction::FeatureChanged` fallback, sets `QgsProject::instance()->setDirty(true)`, and refreshes undo/redo UI (`updateUndoRedoActions()`).
  - `Ctrl+Z` (`MainWindowUndo.cpp:350-360`, `LayerOps::undoLayerEdits` `LayerOps.cpp:2689-2695`) pops **one** edit command from the active undo stack (`layer->undoStack()->undo()`), calls `refreshSelectedGeometry()`, sets `setDirty(true)`, and shows `이전 상태로 되돌렸습니다.`.
  - **Nothing is written to `.gpkg` on disk** during vertex edits (`test_save_open.cpp:3379-3425`); disk write happens only on `저장` (`Ctrl+S`).
- **Rust**:
  - `LayerEditBuffer` (`crates/strata-storage/src/edit_buffer.rs:15-101`) keeps edits in memory (`dirty` flag, `undo_stack`, `redo_stack`) without touching `.gpkg` until save; `undo_map_action` / `redo_map_action` (`crates/strata-app/src/app_state_draw_commit_b.rs:31-79`) pops `UndoStep::Table(key)` (`app_state_layer_remove.rs:12`), calls `t.layer.undo()`, sets `self.is_dirty = true`, and runs `after_layer_edit(&key)`.
  - Gap for topological multi-feature vertex drag: `UndoStep::Table(String)` (`app_state_layer_remove.rs:13`) records one table string per undo press (`LayerEditBuffer::undo()` pops one `change_geometry` entry). Moving a shared vertex across 2 neighboring polygons (`layer` + neighbor, or two tables) currently cannot be grouped into a single `Ctrl+Z` step without extending `LayerEditBuffer` / `UndoStep`.

---

## 4. Snapping While a Vertex Is Dragged

### C++ (`target/cpp-ref/src/`)
1. **Default vs `Ctrl` modifier (`KaFeatureSelectTool::dragPoint`, `KaFeatureSelectTool.cpp:102-109`)**:
   - Without `Qt::ControlModifier`: hides `m_snapMark` and returns raw `e->mapPoint()`.
   - With `Qt::ControlModifier`: calls `m_vertex->snapMapPointExcludingTarget(e, &snapped)` (`KaVertexEditTool.cpp:225-241`), shows `m_snapMark` at `snapped` if `snappedPoint` is true, and returns `snapped`.
2. **Magnet switch rule (`KaVertexEditTool::snapMapPointExcludingTarget`, `KaVertexEditTool.cpp:235`)**:
   - `if (!m_snapEnabled || !mCanvas->snappingUtils()) return pt;` — when the magnet toggle is **OFF**, holding `Ctrl` during drag still does **not** snap.
3. **Self-exclusion (`ExcludeFeatureFilter`, `KaVertexEditTool.cpp:28-40`, `:237`)**:
   - `snapMapPointExcludingTarget` passes `ExcludeFeatureFilter(m_layer.data(), m_fid)` into `snapToMap`, rejecting matches where `m.layer() == m_layer && m.featureId() == m_fid` so a dragged vertex never sticks to its own adjacent edges/vertices. (By contrast, plain `snapMapPoint` at `KaVertexEditTool.cpp:206-223`, used by standalone insert/capture paths, does *not* filter out the target feature.)
4. **Snap config & tolerance (`LayerOps::applySnapConfig` / settings, `LayerOps.cpp:2847-2897`)**:
   - Magnet default: `true` (`LayerOps.cpp:2888`, `snap_chosen` empty).
   - Default tolerance: `settings.tolerancePx = cfg.tolerance() > 0.0 ? cfg.tolerance() : 16.0;` (`LayerOps.cpp:2889`).
   - Types: `Vertex | Segment` (`Vertex` and `Segment`, matching `점·선에 붙습니다` in `MainWindowRibbon.cpp:862`).
   - Target scope: `SurveyLayers` vs `CurrentLayer` (`LayerOps.cpp:2890-2893`).
5. **Topological shared-vertex drag (`topologicalEditing`, `GeometryEditVertex.cpp:20, 114-137`)**:
   - Controlled by `m_topologicalEditing` (`QgsProject` topological editing, default on in survey workflow per `LayerOps.cpp:2895`).
   - `kSharedVertexTolerance = 0.001` map units (`1 mm`, `GeometryEditVertex.cpp:20`). When moving vertex `index` of `(m_layer, m_fid)` from `oldPt` to `target`, neighboring survey features with a vertex within `0.001` map units (`1 mm`) of `oldPt` move that shared vertex to `target` in the same edit (`test_layer_edits.cpp:108-140`, `sharedVertexMoveTouchesOnlyNeighbours`).

### Rust (`ag/vtx0` + note on `3-7` -> `3-7b`)
- **On integration branch `ag/vtx0`**:
  - `CadSessionController` (`crates/strata-canvas2d/src/cad_session.rs:13-39`) stores `snap_enabled: true`, `snap_trace_enabled: true`, `snap_preview_enabled: true`, `snap_tolerance_px: 12.0` (C++ default is `16.0 px`, whereas `gui_ribbon_draw_row.rs` UI slider supports `4..=64 px`), `snap_topological: true`, `snap_target_current_only: false`.
  - Ribbon already renders `자석`, `공유 경계` (`snap_topological`), and `허용치` (`4..=64 px`) in `gui_ribbon_draw_row.rs:223-255`.
  - `snap_index.rs` exists in `crates/strata-geo` and `survey_snap_lines(Some((table, fid)))` in `app_state_snap_lines.rs:88` already excludes `(table, fid)` when snapping a shape to lines.
  - Interactive vertex drag snapping, `ExcludeFeatureFilter` on drag, and `topologicalEditing` (`0.001` m shared-vertex drag) are **NOT BUILT** on `ag/vtx0`.
- **Why `3-7` was sent back (now `3-7b`, separate worktree)**:
  - `app_state_draw_snap.rs` is not on `ag/vtx0`; on the `3-7` branch (`docs/senior/orders/queue/3-7b.yaml` citing `app_state_draw_snap.rs:119-127`), vertex-drag snapping ran whenever `Ctrl` was held even when `snap_enabled` (the magnet switch) was `false`, violating `KaVertexEditTool.cpp:235`.

---

## 5. What the User Sees

| visual element | C++ rule & values | C++ file:line | Rust state on `ag/vtx0` | Rust file:line |
|---|---|---|---|---|
| **Vertex handle (normal)** | `QgsVertexMarker::ICON_BOX`, size `14 px`, `penWidth = 2 px`, outline `QColor(30, 103, 198)`, fill `QColor(255, 255, 255)` | KaVertexEditTool.cpp:180-197 | **NOT BUILT** | — |
| **Vertex handle (hover)** | No separate hover style; `styleMarker(m, active)` checks only `index == m_activeIndex` | KaVertexEditTool.cpp:180-197 | **NOT BUILT** | — |
| **Vertex handle (active / clicked / dragged)** | `QgsVertexMarker::ICON_BOX`, size `16 px`, `penWidth = 2 px`, outline `QColor(220, 38, 38)`, fill `QColor(254, 226, 226)` | KaVertexEditTool.cpp:180-197 | **NOT BUILT** | — |
| **Vertex-drag snap marker** | `QgsVertexMarker::ICON_CIRCLE`, size `14 px`, `penWidth = 2 px`, outline `QColor(30, 103, 198)`, fill `QColor(255, 255, 255, 220)`; shown only while dragging with `Ctrl` + magnet on and snap hit | KaFeatureSelectTool.cpp:90-99, :105-108 | **DIFFERENT / NOT BUILT for vertex drag**: `gui_tab_map_canvas_overlay.rs:75-85` paints amber `4.5 px` radius (`2.0 px` stroke `(217, 119, 6, 230)`) CAD `preview_markers_world` only | gui_tab_map_canvas_overlay.rs:75-85 |
| **Vertex-drag preview rubber band** | `QgsRubberBand`, width `2 px`, outline `QColor(30, 103, 198, 200)`, fill `QColor(30, 103, 198, 40)` | KaVertexEditTool.cpp:160-166 | **NOT BUILT**: `active_rubberband` (`2.2 px` orange `(214, 78, 22)`) is capture-only | gui_tab_map_canvas_overlay.rs:87-103 |
| **Selected feature highlight** | `QColor(255, 235, 59, 45)` on canvas & project | MainWindowChrome.cpp:89-90 | **SAME**: `Color32::from_rgba_unmultiplied(255, 235, 59, 45)` | gui_tab_map_capture.rs:93 |
| **Canvas cursor** | `Qt::ArrowCursor` set in `activate()` and `KaVertexEditTool` ctor | KaFeatureSelectTool.cpp:133, KaVertexEditTool.cpp:44 | **DIFFERENT / implicit**: no `CursorIcon` set for `MapToolKind::Select` | gui_tab_map_canvas.rs:140-155 |
| **Tool guide hint** | Guide band `도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다`; status bar start `도형선택 — 도형을 클릭하면 점이 나옵니다. 점을 끌면 옮기고(Ctrl=자석), 점을 클릭한 뒤 Delete로 지웁니다. 같은 자리를 다시 누르면 겹친 도형을 고릅니다.` | MainWindowChrome.cpp:77, MainWindowEditing.cpp:235-238 | **MIXED**: `gui_tab_map_hud.rs:114` matches guide band; `gui_ribbon_click_a.rs:70` differs by 1 word (`찍은` vs `클릭한`, only on `선택` click); `ribbon_state.rs:119` differs (`피처를 클릭하여 선택하거나...`) | gui_tab_map_hud.rs:114, gui_ribbon_click_a.rs:70, ribbon_state.rs:119 |
| **Sub-toolbar tooltip (`도형선택`)** | 5-line verbatim help (`도형을 클릭하면 수정점이 나옵니다.\n...`) | MainWindowRibbon.cpp:859-864 | **SAME** verbatim string in `select_tip` | gui_ribbon_draw_row.rs:137 |
| **Status bar messages** | Tool start `고칠 도형을 클릭한 뒤 꼭짓점을 끌어 옮기세요.`; exit `도형 선택 종료`; non-editable `참고 자료 도형은 고칠 수 없습니다(보기만).`; target shown `수정점이 나왔습니다. 점을 끌어 옮기세요(Ctrl=자석). 점 우클릭은 삭제, 선 우클릭은 추가입니다.`; click handle `점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다.`; drag done `수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다.`; Delete/menu delete `점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다.` / `점을 지웠습니다.`; menu insert `점을 넣었습니다.` | MainWindowContextMenus.cpp:267, MainWindowEditing.cpp:226, KaFeatureSelectPick.cpp:200, :202, KaFeatureSelectTool.cpp:81, :244, :253, :410 | **ONLY** `도형 선택 종료` exists (`gui_ribbon_click_a.rs:67`); all vertex-edit status texts are **NOT BUILT** (`feature_pick.rs:101`) | gui_ribbon_click_a.rs:67, feature_pick.rs:101 |

---

## 6. Proposed Orders

Cut `3-6` into five sequential/layered small orders (`3-6a` .. `3-6e`) on top of `feature_pick.rs` and `LayerEditBuffer`:

### Order `3-6a` (Size: `S`) — Vertex target, handles overlay, handle pick, and `Esc` fix
- **Covers**: §1 entering/leaving fixes (`도형선택` toggle, `A` sketch guard, `Esc` clears active vertex instead of resetting to `Pan`), §2 rows **1, 2, 5, 6, 12, 15, 19**, §5 vertex markers (`14 px` blue / `16 px` red), verbatim selection status strings.
- **Traced Rust files & functions**:
  - `crates/strata-app/src/feature_pick.rs:67-105` (`select_at` — emit target status strings & editable-key check `survey_area | feature_poly | feature_line | section_line | trial_trench`)
  - `crates/strata-app/src/gui_tab_map_canvas.rs:140-185` (pointer press/release routing in `MapToolKind::Select`: check `vertex_near(..., 20.0 px)` before `select_at`)
  - `crates/strata-app/src/gui_tab_map_canvas_overlay.rs:75-105` (`paint_heritage_overlays` — draw `14 px` blue / `16 px` red square vertex boxes)
  - `crates/strata-app/src/gui_shell.rs:34-40, 78-80` (`Esc` active-vertex clear vs `Pan`, `A` sketch guard)
  - `crates/strata-app/src/gui_ribbon_draw_row.rs:138-142` (`도형선택` toggle to `Pan` like `gui_ribbon_click_a.rs:63-66`)
  - `crates/strata-app/src/ribbon_state.rs:119` & `gui_ribbon_click_a.rs:70` (unify status hint)
- **C++ tests to port**: `test_editing_tools.cpp:228-267` (`clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` — click-pick part, `0` edits on handle click without drag).
- **`not_beside`**: `MAP1e` (map pointer routing), `UI3-27a` & `UI3-29` & `UI3-01` & `UI3-10` (`gui_shell.rs`), `UI3-06` & `UI3-13` (`gui_tab_map_canvas.rs`, `gui_ribbon_click_a.rs`), `UI3-09` & `UI3-14` (`gui_tab_map_canvas_overlay.rs`), `UI3-24` (`gui_ribbon_draw_row.rs`), `3-7b`.

### Order `3-6b` (Size: `M`) — Drag preview, ring-closing move, and `LayerEditBuffer` undo/dirty
- **Covers**: §2 rows **7, 8, 13**, §3 move call chain, `ringClosingVertex` (outer & inner hole rings, multi-part via `vertexIdFromVertexNr`), no-op `isExactlyEqual` skip, CRS transform (`table_transformers`), `LayerEditBuffer::change_geometry` + `UndoStep::Table` + `is_dirty = true` (no GPKG write until `저장`), §5 `2 px` blue drag rubber band.
- **Traced Rust files & functions**:
  - `crates/strata-geo/src/snap_vertex_ops.rs:1-82` (add `ring_closing_vertex`, `move_vertex_in_geometry` across `Polygon`/`MultiPolygon`/`LineString`/`MultiLineString`)
  - `crates/strata-app/src/gui_tab_map_canvas.rs:140-210` (drag threshold `> 4 px` `kClickSlopPx`, move preview, left-release commit)
  - `crates/strata-app/src/gui_tab_map_canvas_overlay.rs:87-105` (draw `2 px` `(30, 103, 198, 200)` / fill alpha `40` drag rubber band)
  - `crates/strata-app/src/app_state_draw_commit_b.rs:31-79` (`undo_map_action` / `redo_map_action` refresh vertex handles after undo)
  - `crates/strata-storage/src/edit_buffer.rs:47-85` (`change_geometry`, `undo`)
- **C++ tests to port**: `test_buffer.cpp:199-225` (`moveFeatureVertex_updatesPolygonCorner`), `test_polygon_erase.cpp:361-374` (`movingTheFirstVertexKeepsAHoleClosed`), `test_inner_piece.cpp:124` (`moveVertexTo` in `KaFeatureSelectTool`), `test_save_open.cpp:3379-3425` (vertex move + undo + dirty without GPKG write until save).
- **`not_beside`**: `MAP1e`, `3-7b`, `UI3-06` (`gui_tab_map_canvas.rs`), `UI3-09` (`gui_tab_map_canvas_overlay.rs`, `app_state_draw_commit_b.rs`), `UI3-14` (`gui_tab_map_canvas_overlay.rs`).

### Order `3-6c` (Size: `M`) — Vertex-drag snapping (`Ctrl` + magnet + self-exclusion) and topological shared-vertex move
- **Covers**: §2 rows **9, 10, 11**, §4 snapping while dragging (`ExcludeFeatureFilter`, `!m_snapEnabled` -> no snap on `Ctrl`, `16.0 px` C++ default tolerance, `Vertex | Segment`), `kSharedVertexTolerance = 0.001 m` shared-vertex drag with grouped single `Ctrl+Z` undo, §5 `14 px` blue `ICON_CIRCLE` snap marker.
- **Traced Rust files & functions**:
  - `crates/strata-canvas2d/src/cad_session.rs:13-39` (align C++ default `snap_tolerance_px` `16.0` vs `12.0` if applicable, target-exclusion drag snap helper)
  - `crates/strata-app/src/app_state_snap_lines.rs:76-119` (reuse `survey_snap_lines(Some((table, fid)))` pattern + vertex/segment pick excluding target feature)
  - `crates/strata-app/src/gui_tab_map_canvas.rs:160-210` (`Ctrl` + `cad.snap_enabled` gate during drag)
  - `crates/strata-app/src/gui_tab_map_canvas_overlay.rs:75-86` (render `14 px` blue `ICON_CIRCLE` at drag snap hit)
  - `crates/strata-app/src/app_state_layer_remove.rs:12-15` & `app_state_draw_commit_b.rs:31-79` (compound multi-feature/multi-table `UndoStep` for one shared-vertex move)
- **C++ tests to port**: `test_layer_edits.cpp:108-140` (`sharedVertexMoveTouchesOnlyNeighbours`), `test_dem_trench.cpp:719` (`applySnapConfig_vertexAndSegmentNotWmsPromise` partial snap tolerance/scope assertions).
- **`not_beside`**: `3-7b` (**must land after or share design with `3-7b`**), `MAP1e`, `UI3-06` (`gui_tab_map_canvas.rs`), `UI3-09` & `UI3-14` (`gui_tab_map_canvas_overlay.rs`).

### Order `3-6d` (Size: `M`) — Right-click vertex/edge context menu (`점추가`, `점삭제`) and edge insert
- **Covers**: §2 rows **21, 22, 23, 24** (auto-pick on empty right-click, `24 px` vertex search / `16 px` segment search, exact disabled reasons and guards, insert/delete dispatch), §3 `insertVertexAt` + `점추가` undo step.
- **Traced Rust files & functions**:
  - `crates/strata-geo/src/snap_vertex_ops.rs` (add `segment_near` + `insert_vertex_in_geometry`)
  - `crates/strata-app/src/gui_tab_map_context.rs:64-135` (`open_map_context_menu` — check selected/auto-picked editable survey `Line`/`Polygon`, show `점삭제` / `점추가` with tooltips)
  - `crates/strata-app/src/gui_tab_map_canvas.rs:185-230` (right-click release routing in `Select`)
- **C++ tests to port**: `test_inner_piece.cpp:231-235` (`점삭제` on vertex right-click), `test_save_open.cpp:3391-3395` (`insertVertexAt` + `deleteVertexAt`), `test_workflow.cpp:1468-1475`.
- **`not_beside`**: `MAP1e`, `UI3-02` (`gui_tab_map_context.rs`), `UI3-06` (`gui_tab_map_canvas.rs`).

### Order `3-6e` (Size: `S`) — `Delete`/`Backspace` active-vertex deletion, min-vertex guards, and `layer.vertices` entry
- **Covers**: §1 `"꼭짓점 수정"` layer context menu entry, §2 rows **17** (`Delete`/`Backspace` deletes active vertex first, falling through to piece/feature delete only when no vertex is active), §3 `deleteVertexAt` across holes/multiparts with `5`/`3` vertex-count limits and verbatim Korean refusal messages.
- **Traced Rust files & functions**:
  - `crates/strata-geo/src/snap_vertex_ops.rs:9-54` (`delete_polygon_vertex_checked`, `delete_line_vertex_checked` — update for multi-part/hole ring counts `5` / `3` and verbatim error messages)
  - `crates/strata-app/src/gui_shell.rs:82-89` (dispatch `Delete`/`Backspace` to `delete_active_vertex()` when `MapToolKind::Select` has an active handle)
  - `crates/strata-app/src/layer_menu_model.rs:220` & `crates/strata-app/src/gui_layer_panel_context_menu.rs:56-113` (remove `"CM2a-2"` deferral on `layer.vertices` and enter `Select` with `"고칠 도형을 클릭한 뒤 꼭짓점을 끌어 옮기세요."`)
- **C++ tests to port**: `test_editing_tools.cpp:228-267` (`clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex`), `test_inner_piece.cpp:125` (`deleteVertexAt(1)`), `test_save_open.cpp:3395`.
- **`not_beside`**: `UI3-01`, `UI3-10`, `UI3-11`, `UI3-27a`, `UI3-29` (`gui_shell.rs`), `UI3-02`, `UI3-05`, `UI3-23`, `UI3-28b` (`gui_layer_panel_context_menu.rs`, `layer_menu_model.rs`).

---

## Could Not Determine

1. **GEOS geometry validity guard inside `KaVertexEditTool::applyGeometryChange` / `LayerOps::applyVertexMove`**: checked `KaVertexEditTool.cpp:334-416` and `GeometryEditVertex.cpp:76-137` via session summaries; exact lines `KaVertexEditTool.cpp:392-415` and `GeometryEditVertex.cpp:85-115` were not re-read verbatim for whether `isGeosValid()` / self-intersection warning is checked on vertex move/delete before `layer->changeGeometry`. Marked unknown here rather than guessing.
2. **Exact C++ `KaFeatureSelectTool::deactivate()` body lines**: confirmed `KaFeatureSelectTool.cpp:133` (`setCursor(Qt::ArrowCursor)`) and `KaFeatureSelectTool.cpp:111-119` (`syncVertexTarget`), `MainWindowEditing.cpp:224-228` (`startSelectTool` toggle to `m_panTool`); the exact line numbers of `KaFeatureSelectTool::deactivate()` were not printed verbatim in this session's summary.

---

## Counts

- **Rules in §2**: `25` (`SAME`: `1`, `DIFFERENT`: `5`, `NOT BUILT`: `19`)
- **C++ tests found**: `8` test functions across `5` files (`test_editing_tools.cpp`, `test_inner_piece.cpp`, `test_polygon_erase.cpp`, `test_layer_edits.cpp`, `test_buffer.cpp`, plus undo/dirty sequence in `test_save_open.cpp` and source checks in `test_workflow.cpp`)
- **Proposed orders**: `5` (`3-6a` `S`, `3-6b` `M`, `3-6c` `M`, `3-6d` `M`, `3-6e` `S`)

## 30-Citation Verification

- Re-opened `30` citations across §§1–6 (`18` C++ source/test ranges, `12` Rust source ranges): `26/30` matched verbatim on first check; `4` line/wording citations (`KaEditTolerance.h` constant lines `:9,:10,:12,:13`, `LayerOps.cpp:2889` for `16.0 px`, `MainWindowEditing.cpp:235-238` vs `gui_ribbon_click_a.rs:70` 1-word hint difference, and `KaFeatureSelectTool.cpp:253`) were corrected and re-verified (`30/30` exact in the committed document).
