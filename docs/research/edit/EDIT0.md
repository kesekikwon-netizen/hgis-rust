# EDIT0 — Map Editing Tools Specification (C++ Strata vs. Rust Today)

- **Order**: `EDIT0` (`docs/senior/research/edit/ORDER-EDIT0.yaml`)
- **Checkout `HEAD` at inspection**: `f7244bc` (`main`; includes `ea2b3fb` `A2`, `c4da1ba` `F1`, and `f7244bc` `fix(snap): 「가까운 선에 붙이기」 snaps to the received cadastral map (B2)`)
- **C++ citation format**: `<path under target/cpp-ref>:line`
- **Rust citation format**: `crates/<crate>/...:line`
- **Scope note**: Read-only comparison of the 10 map editing tools and the 3 `CLAUDE.md:38` edit-buffer rules. Sketch creation internals, trench grid generation, and layout sheet editing are cited only where these 10 editing tools meet them.

---

## 0. Verification of `CLAUDE.md:38` Product Rules in Source

`CLAUDE.md:38` states three editing rules:
1. **Unsaved features have negative IDs.**
2. **A multipolygon in a single-`POLYGON` GPKG table silently becomes a zero-area ring (pieces are stored as separate features).**
3. **Undo stacks are per layer.**

| Rule | C++ source verification | Rust source verification | Status |
|---|---|---|---|
| **R1. Unsaved features have negative IDs** | QGIS `QgsVectorLayerEditBuffer` assigns `-1, -2, ...` to uncommitted features (`src/core/GeometryEditMerge.cpp:26-30`). Every Strata picker and vertex tool checks `FID_IS_NULL(fid)` (`std::numeric_limits<QgsFeatureId>::min()`), never `fid < 0` (`src/app/KaVertexEditTool.h:56-57`, `src/app/KaVertexEditTool.cpp:150-152`, `src/app/KaFeatureSelectPick.cpp:167-169`, `src/core/FeaturePick.h:29,41-42`, `src/core/FeaturePick.cpp:39-54`). On `commitChanges` / `저장`, `MainWindow::watchUndoFeatureIds` (`src/app/MainWindowUndo.cpp:86-130`) and `remapUndoFeatureIdsAfterSave` (`:132-156`) rewrite negative IDs in `m_undoActions` to positive provider IDs. Tested in `tests/test_save_open.cpp:3509` (`QVERIFY2(inner.id() < 0, ...)`), `tests/test_inner_piece.cpp:108-129`, `tests/test_trench_edit.cpp:158-180`, and `tests/test_layer_edits.cpp:62-63`. | `LayerEditBuffer` (`crates/strata-storage/src/edit_buffer.rs:20,31,37-52`) initializes `next_temp_id: -1`, assigns `let fid = self.next_temp_id; self.next_temp_id -= 1;` in `add_unsaved_feature`, and accepts negative `fid` in `change_geometry` (`:55-63`) and `delete_feature` (`:84-92`). On `저장`, `apply_table` (`crates/strata-storage/src/survey_gpkg_apply.rs:34-36,68-90`) routes `rec.fid < 0` to `INSERT INTO` (SQLite assigns positive `fid`) and reloads `survey_tables` (`crates/strata-app/src/app_state_projects.rs:232-234`). Picking (`crates/strata-app/src/feature_pick.rs:64`) carries `rec.fid: i64` without `< 0` filtering. | `SAME` in buffer/pick/save; `DIFFERENT` across `저장` undo (see `3.B7`). |
| **R2. Multipolygon in a single-`POLYGON` table becomes a zero-area ring; pieces stored as separate features** | Survey polygon tables (`survey_area`, `feature_poly`) are created with single-part `"Polygon"` geometry (`src/core/SurveySchema.cpp:30,34`). Three places enforce single-part storage explicitly: (1) `LayerOps::mergePolygonFeatures` (`src/core/GeometryEditMerge.cpp:130-136`: comment `한 조각 표(조사구역·유구면)에 여러 조각 도형을 넣으면 저장 때 면적 0인 고장 난 도형이 된다.`, refuses `partCount() > 1` when `!QgsWkbTypes::isMultiType(layer->wkbType())` and calls `convertToSingleType()` when `partCount() == 1`); (2) `PolygonErase::cutTarget` (`src/core/PolygonErase.cpp:93-155`: when `!isMultiType(layer->wkbType())`, writes `parts.first()` into the target feature and calls `layer->addFeature` for each remaining part `i >= 1` with copied attributes and cleared PKs); (3) `EditGeometryRepair::closePolygon` (`src/core/EditGeometryRepair.cpp:58-72`: decomposes bowtie output into single polygons and keeps only the largest-area part). Tested in `tests/test_layer_edits.cpp:97-106`, `tests/test_polygon_erase.cpp:227-296`, `tests/test_editing_history.cpp:150-167`. | `edit_geometry_repair::close_polygon` (`crates/strata-app/src/edit_geometry_repair.rs:48-77`) keeps only the largest single `Polygon` on sketch close (`SAME` for sketch repair). However, `gpkg_geom_to_wkt` (`crates/strata-storage/src/gpkg_wkt.rs:65-75`) serializes `Geometry::MultiPolygon` as `MULTIPOLYGON(...)`, `survey_gpkg_apply::insert_feature` (`crates/strata-storage/src/survey_gpkg_apply.rs:146-167`) writes `GeomFromText(?1, srs_id)` into single-`POLYGON` GPKG tables without splitting `MultiPolygon` into separate features, and neither `mergePolygonFeatures` nor `PolygonErase` is wired yet. | `DIFFERENT` (`crates/strata-storage/src/survey_gpkg_apply.rs:146-167` lacks single-`POLYGON` multi-part guard/splitting). |
| **R3. Undo stacks are per layer** | Each `QgsVectorLayer` owns its own `QUndoStack` (`layer->undoStack()`), populated via `LayerOps::runEditCommand` (`src/core/LayerOps.cpp:2652-2679`) and `KaUndoGroup` (`src/app/KaUndoGroup.h:13-36`). Reference and cadastral layers are excluded (`src/app/KaEditUndoOrder.cpp:26-30`). Across layers, `EditHistory` (`src/core/EditHistory.cpp:11-147`) stamps every command on watched per-layer `QUndoStack`s with a global monotonic `m_seq`, and `MainWindow::newestUndoLayer` / `newestRedoLayer` (`src/app/KaEditUndoOrder.cpp:105-140`) plus `KaLinkedEdit` (`src/app/MainWindowErase.cpp:174-244`) coordinate global `Ctrl+Z` / `Ctrl+Y`. Tested in `tests/test_editing_history.cpp:42-148`. | Each `SurveyFileTable` owns its own `LayerEditBuffer` with `undo_stack: Vec<Vec<FeatureRecord>>` and `redo_stack: Vec<Vec<FeatureRecord>>` full-table snapshots (`crates/strata-storage/src/edit_buffer.rs:16-23,94-112`). Global ordering is tracked on `StrataAppState` by `undo_tables: Vec<UndoStep>` and `redo_tables: Vec<String>` (`crates/strata-app/src/app_state.rs:99-100`, `crates/strata-app/src/app_state_draw_commit_b.rs:31-74`). | `SAME` per-layer architecture; `DIFFERENT` in granularity (full-table clone vs `QUndoCommand`), post-save state, and missing `KaLinkedEdit` (see Tool 3). |

---

## 1. 도형 선택

### A. C++ offered
- **A1. Ribbon button (`기록` group)**:
  - Control: checkable `QAction` `m_actSelect` / `QToolButton` on main ribbon (`src/app/MainWindowRibbon.cpp:300-310`).
  - Text: `"선택"` (`src/app/MainWindowRibbon.cpp:301`).
  - Tooltip: initially set to `"그린 도형을 선택합니다. 다시 누르면 이동으로 돌아갑니다"` (`src/app/MainWindowRibbon.cpp:302`), immediately overwritten in `KaFeatureSelectTool::installKeyShortcut` to `"그린 도형을 선택합니다 (A 또는 Ctrl+1). 단추를 다시 누르면 이동으로 돌아갑니다"` (`src/app/KaFeatureSelectPick.cpp:97-98`).
  - Enabled/hidden: always visible on the ribbon; checked iff `m_canvas->mapTool() == m_featureSelectTool` (`src/app/MainWindowRibbon.cpp:306`, `tests/test_save_open.cpp:3881-3884`). Clicking while already active returns to `m_panTool` (`src/app/MainWindowEditing.cpp:224-228`).
- **A2. Draw subtoolbar button (`subToolbar`, `m_subToolsMode == "draw"`)**:
  - Control: `QAction` `selAct` (`kaSubTool = "select"`) right after caption `"  그리기 › "` (`src/app/MainWindowRibbon.cpp:853-865`).
  - Text: `"도형선택"` (`src/app/MainWindowRibbon.cpp:857`).
  - Tooltip (`src/app/MainWindowRibbon.cpp:860-865`):
    `"도형을 클릭하면 수정점이 나옵니다.\n점을 끌면 그 점만 옮겨집니다.\n점 위에서 우클릭하면 그 점을 지우고, 선 위에서 우클릭하면 점을 넣습니다.\nCtrl을 누른 채 끌면 다른 도형의 점·선에 붙습니다. 점을 클릭해 고른 뒤 Delete로 지웁니다.\n겹친 도형은 같은 자리를 다시 누르면 다음 도형이 골라집니다. 단축키 A. 면 안쪽 도형은 클릭 후 Delete."`
  - Enabled/hidden: visible whenever the `그리기` subtoolbar row is open (`src/app/MainWindowRibbon.cpp:945`). Calls `&MainWindow::startSelectTool` (`:858`), so clicking it while active also toggles back to `m_panTool` (`src/app/MainWindowEditing.cpp:224-228`).
- **A3. Keyboard shortcuts (`Ctrl+1` and `A` / physical `VK_A`)**:
  - `Ctrl+1` on `m_actSelect` (`src/app/MainWindowRibbon.cpp:307`) triggers `startSelectTool` (toggles on/off).
  - `A`: `QAction` `"도형선택"` (`objectName = "actSelectShapeKey"`, `Qt::WindowShortcut`, `Qt::Key_A`) installed by `KaFeatureSelectTool::installKeyShortcut` (`src/app/KaFeatureSelectPick.cpp:91-128`), plus Korean-IME fallback in `MainWindow::eventFilter` checking `ke->modifiers() == Qt::NoModifier && !ke->isAutoRepeat() && ke->key() != Qt::Key_A && ke->nativeVirtualKey() == 0x41` (`src/app/MainWindow.cpp:2309-2315`).
  - Guards on `A` in order (`src/app/KaFeatureSelectPick.cpp:104-126`):
    1. Canvas missing, not visible to window, or `!selectAction->isEnabled()` -> no-op (`:105`).
    2. Focus in `QLineEdit`, `QAbstractSpinBox`, `QTextEdit`, `QPlainTextEdit`, or editable `QComboBox` -> no-op (`:107-112`).
    3. `KaFeatureSelectTool` already active -> no-op (`A` **never** toggles select off, `:114`).
    4. Active tool inherits `KaMeasureMapTool` or `KaAlignMapTool` -> forwards `Key_A` to `tool->keyPressEvent(&press)` (`:117-121`).
    5. `keyOwnedElsewhere()` (`m_subToolsMode == "align"`, `src/app/MainWindowRibbon.cpp:310`) -> no-op (`:122`).
    6. Active tool is `KaCaptureMapTool` and `capture->hasSketch()` (`pointCount() > 0`) -> no-op (never drops a half-drawn shape, `:124`). Otherwise triggers `selectAction` (exiting idle capture / split mode cleanly via `stopCaptureTool`, `src/app/MainWindowEditing.cpp:195-196,284`).
- **A4. Right-click canvas context menu while `KaFeatureSelectTool` is active**:
  - Built in `KaFeatureSelectTool::handleContextMenu` (`src/app/KaFeatureSelectTool.cpp:276-415`):
    - If nothing is selected yet, runs non-cycling `selectAtPoint(e->mapPoint(), false, e->pos(), false)`; if still empty, emits `requestMapContextMenu(e->pos())` (`:280-288`).
    - On 1 selected editable Line/Polygon feature: `"점삭제"` (when cursor within `kVertexMenuPx = 12` px of a vertex) or `"점추가"` (when within `kSegmentPx = 10` px of a segment) (`:310-314,355-360`).
    - On 1 selected editable Polygon with an inner piece under cursor: `"안쪽 도형 지우기"` (`:365`) and, when `!piece->isHole()`, `"안쪽 도형만큼 구멍 내기"` (`:366`).
    - When neither vertex nor piece action applies and `anyPolygon && (plan.ready() || all.size() >= 2)`: `"겹친 곳 지우기"` (`objectName = "actSelectEraseOverlap"`, enabled iff `plan.ready()`, disabled tooltip `plan.hint`, `:377-392`).
    - If menu is empty, emits status `"점 위에서 우클릭하면 점삭제, 선 위에서 우클릭하면 점추가입니다."` (`:394-396`).

### B. C++ does
1. **B1. Left-click point picking & same-spot overlap cycling**:
   - Chain: `KaFeatureSelectTool::canvasPressEvent` (`src/app/KaFeatureSelectTool.cpp:174-194`) -> `canvasReleaseEvent` (`:227-274`) -> `selectAtPoint(e->mapPoint(), shift, e->pos(), true)` (`src/app/KaFeatureSelectPick.cpp:130-210`) -> `FeaturePick::candidates(kaPickLayers(mCanvas), mapPt, mupp * kFeaturePickPx, destinationCrs, transformContext)` (`src/core/FeaturePick.cpp:64-151`).
   - Constants (`src/app/KaEditTolerance.h:7,12`): `kFeaturePickPx = 10` px, `kClickSlopPx = 4` px (Manhattan distance `(e->pos() - m_pressPos).manhattanLength()`).
   - Candidate ranking (`Rank = std::tuple<int, int, double, int>`, smaller wins, `src/core/FeaturePick.cpp:29,91-147`):
     1. `tier`: `0` for editable survey layers (`FeaturePick::isSurveyLayer`: not reference/basemap/cadastral, `:58-62`), `1` for reference/cadastral. **If any `tier == 0` hit exists, all `tier != 0` hits are removed** (`:142-144`).
     2. `kind`: `0` for point/line within `tolerance` (`distance / tolerance`); `1` for polygon or inner piece containing `probe` (`abs(area) / tolerance²`, smallest area first); `2` for polygon or inner piece outline within `tolerance` (`distance / tolerance`).
     3. `order`: canvas layer draw order (`kaPickLayers(mCanvas)`, which includes above-labels survey layers).
     4. Also scans `missedBufferedFeatures` (`editBuffer()->changedGeometries()` and `addedFeatures()`, `:39-54,133-137`) so unsaved negative-ID shapes and newly cut holes are picked before save.
   - Same-spot cycling (`src/app/KaFeatureSelectPick.cpp:156-165`): if `allowCycle && !addToSelection && !candidates.isEmpty() && m_lastPickIndex >= 0 && candidates == m_lastPickCandidates && (screenPos - m_lastPickPos).manhattanLength() <= kClickSlopPx`, advances `pick = (m_lastPickIndex + 1) % candidates.size()`. Right-click passes `allowCycle = false` so it never starts/advances a cycle (`:160-164`).
   - Undo step: none (selection is transient).
2. **B2. Inner-piece (`PolygonPieces`) picking & highlight**:
   - Chain: `FeaturePick::candidates` calls `PolygonPieces::innerPieceAt(geometry, point, tolerance)` (`src/core/FeaturePick.cpp:105`, `src/core/PolygonPieces.cpp:40-96`), which inspects interior hole rings (`Piece::Kind::Hole`, `{part, ring}`) and multipart islands contained inside another part (`Piece::Kind::Part`, `{part, 0}`), returning the smallest-area containing/nearby piece.
   - When `all.size() == 1 && !addToSelection && hits.at(pick).piece.isValid()`, `setActivePiece` (`src/app/KaFeatureSelectPick.cpp:36-55,183-185`) draws `m_pieceBand` (`QgsRubberBand` stroke `QColor(220, 38, 38)` width `3`, fill `QColor(220, 38, 38, 70)`). Cycling at the same spot moves from the inner piece to the whole polygon (`tests/test_inner_piece.cpp:194-219`).
3. **B3. Drag rubber-band marquee selection (`selectInRect`)**:
   - Chain: `canvasMoveEvent` (`src/app/KaFeatureSelectTool.cpp:209-225`) starts `m_dragging = true` once `(e->pos() - m_pressPos).manhattanLength() > kClickSlopPx` (`4` px), showing `m_rubberBand` (`QColor(30, 103, 198, 180)` width `1`, fill `QColor(30, 103, 198, 40)`).
   - On release, `selectInRect(QgsRectangle(m_pressMapPt, e->mapPoint()), shift)` (`:263-270,417-484`) transforms the rectangle into each layer's CRS (`setBallparkTransformsAreAppropriate(true)`), tests `f.geometry().intersects(layerGeom)`, and selects `survey` hits if non-empty, else `reference` hits (`:471-476`). Clears `m_lastPickIndex = -1` and `setActivePiece({})`, calls `syncVertexTarget()`, emits `"도형 %1개 선택됨"` (`:480-484`).
4. **B4. `Shift` modifier (multi-selection)**:
   - `shift = (e->modifiers() & Qt::ShiftModifier)` (`src/app/KaFeatureSelectTool.cpp:261`).
   - On click (`src/app/KaFeatureSelectPick.cpp:134-143,170-175`): without `Shift`, clears selection on all project vector layers first; with `Shift`, preserves existing selections across layers and toggles `hitFid` (`deselect` if already selected, else `select`), disabling cycling and inner-piece activation (`:156,183`).
   - On box drag (`src/app/KaFeatureSelectTool.cpp:420-429,474`): without `Shift`, clears all layers first; with `Shift`, adds via `Qgis::SelectBehavior::AddToSelection`.
   - When `allSelectedFeatures(mCanvas).size() != 1`, `syncVertexTarget()` clears vertex handles (`:111-119`).
5. **B5. `Ctrl` modifier (snap-to-other-shapes during vertex drag)**:
   - `Ctrl` has **no** selection-toggle effect on click/marquee (`src/app/KaFeatureSelectTool.cpp:261` checks only `ShiftModifier`).
   - During vertex drag (`src/app/KaFeatureSelectTool.cpp:102-109,197-206`), free cursor position `e->mapPoint()` is used unless `e->modifiers() & Qt::ControlModifier`, which calls `m_vertex->snapMapPointExcludingTarget(e, &snapped)` (`src/app/KaVertexEditTool.cpp:30-40,225-241`, `ExcludeFeatureFilter` ignoring `(m_layer, m_fid)`) and displays a 14 px blue circle `m_snapMark` (`QColor(30, 103, 198)` / fill `QColor(255, 255, 255, 220)`, `src/app/KaFeatureSelectTool.cpp:85-100`).
6. **B6. `Esc` key in `KaFeatureSelectTool`**:
   - `KaFeatureSelectTool::keyPressEvent` (`src/app/KaFeatureSelectTool.cpp:164-170`): if `m_vertex && (m_vertex->activeVertex() >= 0 || m_activePiece.isValid())`, clears `m_vertex->setActiveVertex(-1)`, `setActivePiece({})`, and `m_lastPickIndex = -1`, accepting the event while keeping `KaFeatureSelectTool` active and keeping the feature(s) selected. When neither a vertex nor an inner piece is active, `Esc` does **not** exit `Select` to `Pan` (`:171`).
7. **B7. `Ctrl+C` / `Ctrl+V` parcel copy-paste in `Select` tool**:
   - `KaParcelCopyKeys` (`src/app/KaParcelCopyKeys.cpp:19-95`, `tests/test_parcel_copy_keys.cpp:31-120`) listens only while `KaFeatureSelectTool` is the active canvas tool (ignoring auto-repeat on `Ctrl+V`) to copy a selected cadastral parcel and paste its parts as features into the survey polygon layer.
8. **B8. Activation & selection status messages and guide band**:
   - `MainWindow::startSelectTool` (`src/app/MainWindowEditing.cpp:235-238`):
     `"도형선택 — 도형을 클릭하면 점이 나옵니다. 점을 끌면 옮기고(Ctrl=자석), 점을 클릭한 뒤 Delete로 지웁니다. 같은 자리를 다시 누르면 겹친 도형을 고릅니다."` (10000 ms); toggling off (`:226`): `"도형 선택 종료"` (3000 ms).
   - `KaFeatureSelectTool::activate` (`src/app/KaFeatureSelectTool.cpp:137-139`):
     `"도형선택: 도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮기세요(Ctrl=자석). 점 우클릭은 삭제, 선 우클릭은 점추가입니다."`
   - `KaFeatureSelectTool::selectAtPoint` (`src/app/KaFeatureSelectPick.cpp:188-209`), in exact branch order:
     1. `all.isEmpty()`: `"선택된 도형 없음"`
     2. `m_activePiece.isValid()`: `"안쪽 도형을 골랐습니다. Delete를 누르면 안쪽 도형만 지워집니다(바깥은 그대로). 우클릭하면 구멍으로 뺄 수도 있습니다."` (+ `" 같은 자리를 다시 누르면 도형 전체를 고릅니다."` when `candidates.size() > 1`)
     3. `all.size() == 1 && candidates.size() > 1 && !addToSelection`: `"겹친 도형 %1/%2 — 같은 자리를 다시 누르면 다음 도형을 고릅니다."`
     4. `all.size() == 1 && !(m_vertex && m_vertex->hasTarget())`: `"참고 자료 도형은 고칠 수 없습니다(보기만)."`
     5. `all.size() == 1`: `"수정점이 나왔습니다. 점을 끌어 옮기세요(Ctrl=자석). 점 우클릭은 삭제, 선 우클릭은 추가입니다."`
     6. `all.size() == 2`: `"도형 2개 선택됨 (%1, %2) — [겹친 곳 지우기](우클릭)는 큰 도형에서 작은 도형 자리를 지우고, [폴리곤 나누기]는 겹친 자리를 새 도형으로 나눕니다."`
     7. `all.size() >= 3`: `"도형 %1개 선택됨"`

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestEditingTools::selectPrefersSmallShapeAndCyclesOverlaps` | `tests/test_editing_tools.cpp:199-226` | 640x480 EPSG:5186 canvas `kExtent(200000,450000,200100,450075)`; `reference` polygon covering `kExtent` + survey `"feature_poly"` with `house` `square(200010,450010,60)` and `pit` `square(200030,450030,10)`. | 3x `click(tool, &view.canvas, pitCenter)` + `isPickableLayer` | `pitCenter = view.px(200035, 450035)` | `selectedIds(layer) == {pit}` (1st); `reference->selectedFeatureIds().isEmpty()`; `selectedIds(layer) == {house}` (2nd); `selectedIds(layer) == {pit}` (3rd); `!isPickableLayer(reference)`; `isPickableLayer(layer)`. |
| `TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | `tests/test_editing_tools.cpp:228-267` | 640x480 EPSG:5186 canvas; 5-corner polygon `{(200020,450020),(200060,450020),(200070,450045),(200040,450065),(200015,450045),(200020,450020)}`. | Click interior -> click vertex -> `deleteActiveVertex()` x2 -> drag vertex | `view.px(200040, 450040)`, `view.px(200060, 450020)`, `from = view.px(200070, 450045)`, `to = from + QPoint(30, 12)` | `selectedIds(layer) == {fid}`; `!layer->isModified()`; `undoStack()->count() == 0`; `tool.deleteActiveVertex() == true`; `nCoordinates() == 5`; `isModified() && undoStack()->count() == 1`; 2nd `deleteActiveVertex() == false`; dragged vertex within `1e-6` of `toMapCoordinates(to)`; `undoStack()->count() == 2`. |
| `TestEditingTools::attributeEditingSkipsReferenceAndCadastral` | `tests/test_editing_tools.cpp:269-282` | `survey` (`"feature_poly"`), `reference` (`markReferenceLayer`), `cadastral` (`markCadastralLayer`). | `KaAttributeMapTool::isEditableLayer(...)` | `survey, reference.get(), cadastral.get(), nullptr` | `true`, `false`, `false`, `false`. |
| `PolygonEraseTest::pick_takesTheSmallShapeDrawnOnTopEvenWhenUnsaved` | `tests/test_polygon_erase.cpp:298-315` | EPSG:5187 `"survey_area"` with saved `outer` `rect(0,0,100,100)` and unsaved buffered `inner` `rect(40,40,60,60)` (`inner.id() < 0`). | `FeaturePick::candidates` at `(50,50)` and `(10,10)`, tol `2.0` | `QgsPointXY(50, 50), 2.0` and `QgsPointXY(10, 10), 2.0` | `inner.id() < 0`; `hits.size() == 2`; `hits.at(0).fid == inner.id()`; `hits.at(1).fid == outer`; `at({area}, (10,10), 2.0).fid == outer`. |
| `PolygonEraseTest::pick_findsAShapeInsideAShapeOfTheLayerAbove` | `tests/test_polygon_erase.cpp:317-331` | EPSG:5187 `"survey_area"` `rect(0,0,100,100)` above `"feature_poly"` `pit` `rect(40,40,50,50)`. | `FeaturePick::candidates({area, poly}, (45,45), 2.0, ...)` | `QgsPointXY(45, 45), 2.0` | `hits.size() == 2`; `hits.at(0).layer == poly`; `hits.at(0).fid == pit`; `hits.at(1).layer == area`; `hits.at(1).fid == outer`. |
| `PolygonEraseTest::pick_prefersSurveyShapesAndNearbyLines` | `tests/test_polygon_erase.cpp:333-359` | EPSG:5187 `"survey_area"` `rect(0,0,100,100)`, `"feature_line"` `(20,20)-(80,20)`, cadastral `parcel` `rect(0,0,10,10)` + `(200,200,220,220)`. | `FeaturePick::candidates` at `(50, 20.5)` and `(210, 210)`, tol `2.0` | `QgsPointXY(50, 20.5), 2.0`; `QgsPointXY(210, 210), 2.0` | At `(50,20.5)`: `hits.size() == 2`, `hits[0].layer == line`, `hits[1].layer == area` (cadastral excluded). At `(210,210)`: `onlyParcel.size() == 1`, `onlyParcel[0].fid == farParcel`. |
| `TestPolygonPieces::*` (6 tests) | `tests/test_polygon_pieces.cpp:43-170` | Single/multipart polygons with holes, nested parts, islands inside holes, and unsaved buffered holes in EPSG:5187. | `PolygonPieces::innerPieceAt`, `withoutPiece`, `withPieceCutOut`, `FeaturePick::candidates` | Clicks at `(50,50)` tol `2.0`, `(80,80)` tol `2.0`, etc. | `piece->isHole()`; `filled.area() == 10000.0` (`<1e-6`); `without.area() == 10000.0`; `cut.area() == 8400.0`; stacked smallest `(45..55)` area `100.0` wins; separate parts return `!innerPieceAt`; island inside hole removed with hole -> `10000.0`; unsaved hole picked before whole polygon (`hits.size() == 2`, `hits[0].piece.isHole()`, `!hits[1].piece.isValid()`). |
| `TestInnerPiece::*` (7 tests) | `tests/test_inner_piece.cpp:108-281` | 640x480 EPSG:5187 canvas + `KaFeatureSelectTool` (+ `KaCaptureMapTool` in `:240-281`). | Clicks, right-clicks, `deleteActivePick()`, `cutOutActivePiece()`, `Qt::Key_A` | Unsaved square `(40,40,60,60)`, above-labels layer, hole `(30,30,70,70)` in `(10,10,90,90)`, inner part, `Key_A` with 1 sketch point | Unsaved negative-ID shape gets handles & vertex move (`xMinimum == 35.0`); above-labels shape picked; click inside hole sets `hasActivePiece()`, `deleteActivePick()` fills hole to `6400.0` in 1 undo step; outer click keeps `hasActivePiece() == false`; 2nd click cycles from inner part to whole shape, `cutOutActivePiece()` cuts hole to `4800.0`; right-click on `user_poly_1` shows `"점삭제"` without starting cycle; `Key_A` switches to `Select`, 2nd `Key_A` stays `Select`, `Key_A` during 1-point sketch keeps `capture` and `pointCount() == 1`. |
| `ParcelCopyKeysTest::*` (3 select tests) | `tests/test_parcel_copy_keys.cpp:31-120` | Cadastral parcel layer + `KaFeatureSelectTool` vs `KaCaptureMapTool`. | `Ctrl+C`, `Ctrl+V` (including auto-repeat) | `Qt::Key_C` / `Qt::Key_V` with `Qt::ControlModifier` | Auto-repeat pastes `1` feature, not `2`; `Select` tool pastes at original parcel geometry; outside `Select` tool pastes `0`. |
| `TestSaveOpen::selectTool_picksTheShapeDrawnOnTopAndKeyAStartsIt` | `tests/test_save_open.cpp:3807-3849` | Opened survey (`survey_area` `outer` + unsaved `inner` `rect(190040,560040,190060,560060)`). | `Key_A` x2, `clickAt(190050,560050)`, `clickAt(190010,560010)` | `Qt::Key_A`, `(190050,560050)`, `(190010,560010)` | `canvas->mapTool() == select` after 1st and 2nd `Key_A`; `selectedFeatureIds() == {inner.id()}` and `vertex->hasTarget()`; then `{outer}`. |

### D. Rust today
- **A1**: `DIFFERENT` — Ribbon `"선택"` toggles `MapToolKind::Select` <-> `MapToolKind::Pan` and sets `"도형 선택 종료"` (`crates/strata-app/src/gui_ribbon_click_a.rs:63-72`), **but** its tooltip (`crates/strata-app/src/ribbon_icons_map.rs:49`) is `"그린 도형을 선택합니다. 다시 누르면 이동으로 돌아갑니다"`, omitting `(A 또는 Ctrl+1)` from `KaFeatureSelectPick.cpp:98`.
- **A2**: `DIFFERENT` — Subtoolbar `"도형선택"` (`crates/strata-app/src/gui_ribbon_draw_row.rs:137-142`) has the verbatim tooltip, **but** clicking it when already in `Select` does not toggle back to `Pan`, resets `draw_sketch` directly instead of calling `startSelectTool`, and sets no status bar message.
- **A3**: `DIFFERENT` — `Ctrl+1` switches to `Select` without toggling off (`crates/strata-app/src/gui_shell.rs:136-139`) and `A` switches to `Select` without toggling off (`crates/strata-app/src/gui_shell.rs:78-81`), **but**: (1) `Ctrl+1` does not toggle back to `Pan`; (2) there is no Korean-IME physical `VK_A` (`0x41`) fallback; (3) pressing `A` steals the key even during `Measure` or `Align`; and (4) pressing `A` while a sketch has points switches `active_tool = MapToolKind::Select`, causing `drop_sketch_when_tool_changed` (`crates/strata-app/src/app_state_draw_row.rs:31-35`) to **discard the half-drawn sketch** (violating `KaFeatureSelectPick.cpp:124`).
- **A4**: `MISSING` — Right-clicking the canvas in `Select` mode (`crates/strata-app/src/gui_tab_map_canvas.rs:130-137`, `crates/strata-app/src/gui_tab_map_context.rs:106-152`) always opens the generic map context menu (`전체 보기`, `좌표 복사`, `그리기`, etc.); there is no `handleContextMenu` for `점추가`, `점삭제`, `안쪽 도형 지우기`, `안쪽 도형만큼 구멍 내기`, or `겹친 곳 지우기`.
- **B1**: `DIFFERENT` — `feature_pick::candidates` (`crates/strata-app/src/feature_pick.rs:17-18,58-103`) and `pick_shape_at` (`crates/strata-app/src/gui_tab_map_canvas.rs:107-112,232-279`) match `PICK_PX = 10.0`, `CLICK_SLOP_PX = 4.0`, point/line -> smallest-polygon -> outline ranking, negative-ID support, and same-spot cycling on visible `survey_tables`, **but** do not pick reference or cadastral vector features (`self.ref_vectors`) when no survey shape lies under the click and do not include inner polygon pieces.
- **B2**: `MISSING` — `PolygonPieces::innerPieceAt` / `withoutPiece` / `withPieceCutOut` has **no** Rust port (`crates/strata-storage/src/polygon_pieces.rs:77-134` contains only unwired multipolygon-split and erase-difference helpers, not inner-hole/island picking), and no red piece highlight exists.
- **B3**: `MISSING` — Dragging on the map in `Select` mode pans the viewport (`crates/strata-app/src/gui_tab_map_canvas.rs:113-118`); there is no rubber-band marquee or `selectInRect`.
- **B4**: `MISSING` — `HomeMapState::selected_shape` is `Option<(String, i64)>` (`crates/strata-app/src/gui_tab_map_state.rs:14`), holding at most one feature; `Shift` is ignored (`crates/strata-app/src/gui_tab_map_canvas.rs:232-279`).
- **B5**: `MISSING` — No vertex handles or `Ctrl`-snapped vertex dragging exist (`crates/strata-app/src/gui_tab_map_canvas.rs:101-120`).
- **B6**: `DIFFERENT` — Pressing `Esc` in `Select` mode switches `self.active_tool` to `MapToolKind::Pan` without clearing `selected_shape` (`crates/strata-app/src/gui_shell.rs:34-39`), whereas C++ keeps `KaFeatureSelectTool` active and only clears an active vertex or inner piece (`src/app/KaFeatureSelectTool.cpp:164-170`).
- **B7**: `MISSING` — No `Ctrl+C` / `Ctrl+V` cadastral parcel copy-paste exists in `crates/strata-app/src/gui_shell.rs:29-95`.
- **B8**: `FAKE` — Ribbon `"선택"` (`crates/strata-app/src/gui_ribbon_click_a.rs:70`), subtoolbar `"도형선택"` (`crates/strata-app/src/gui_ribbon_draw_row.rs:137`), and the map HUD pill (`crates/strata-app/src/gui_tab_map_hud.rs:110-115`: `"도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다"`) tell the user that clicking a shape shows draggable vertex handles with `Ctrl` snapping and `Delete` vertex removal, **but** clicking a shape in `crates/strata-app/src/gui_tab_map_canvas.rs:256-277` renders zero vertex handles and sets only `"<표>: 도형 1개 선택됨"` / `"겹친 도형 ..."` / `"선택된 도형 없음"`.

---

## 2. Delete Key on a Selected Shape and on a Selected Legend Row

### A. C++ offered
- **A1. Window `Delete` shortcut & canvas/tree event filter**:
  - Window `QAction` `delAct` (`"선택 도형 삭제"`, `QKeySequence::Delete`, `Qt::WindowShortcut`, `src/app/MainWindow.cpp:256-269`).
  - Also intercepted in `MainWindow::eventFilter` on `m_canvas` and `m_layerTree` (`!measuring && ke->key() == Qt::Key_Delete`, `src/app/MainWindow.cpp:2371-2378`; yields to `KaCaptureMapTool` when capturing on canvas at `:2374`).
  - Guarded by `editingText()` (`QLineEdit`, `QAbstractSpinBox`, `QTextEdit`, `QPlainTextEdit`, `src/app/MainWindowUndo.cpp:51-55,159,487,539`) and `!m_isOpeningSurvey && !m_closingWindow` (`:159`).
- **A2. `Backspace` synonym in canvas tools**:
  - `KaCaptureMapTool::keyPressEvent` (`src/app/KaCaptureMapTool.cpp:388`), `KaFeatureSelectTool::keyPressEvent` (`src/app/KaFeatureSelectTool.cpp:160`), `KaVertexEditTool::keyPressEvent` (`src/app/KaVertexEditTool.cpp:512`), and `KaAlignMapTool::keyPressEvent` (`src/app/KaAlignMapTool.cpp:549`) treat `Qt::Key_Backspace` identically to `Qt::Key_Delete`.
- **A3. Layer tree right-click `"레이어 삭제"` action**:
  - Offered on both layer rows (`src/app/MainWindowContextMenus.cpp:441-445`, tooltip none, disabled only when `busy`) and group header rows including empty groups (`src/app/MainWindowContextMenus.cpp:199-213`, tooltip `"이 묶음과 안의 레이어를 목록과 지도에서 뺍니다. 원본 파일은 보존하며 Ctrl+Z로 복원할 수 있습니다."`), calling `removeLayersFromTree(treeView)`.

### B. C++ does
1. **B1. Focus routing & active-sketch guard**:
   - `delAct` (`src/app/MainWindow.cpp:260-268`):
     1. `routeEditKeyToActiveStudio(true)` (handles layout studio / 3D studio item deletion first).
     2. Focus inside `m_alignPointList` -> `deleteSelectedAlignPoint()`.
     3. Focus inside `m_layerTree` -> `removeSelectedLayers()` -> `removeLayersFromTree(m_layerTree)`.
     4. Otherwise -> `deleteFeaturesOrSelectedReferenceLayers()` (`src/app/MainWindowUndo.cpp:486-536`).
   - Inside `deleteFeaturesOrSelectedReferenceLayers` (`:490-495`): if `m_canvas->mapTool() == m_captureTool`, calls `m_captureTool->undoLastVertex()` on the map tab, shows `"꼭짓점 하나를 되돌렸습니다."` (4000 ms), and **returns immediately** (never deletes shapes or layers while drawing).
2. **B2. Active vertex or inner-piece deletion priority (`Select` tool)**:
   - Next (`src/app/MainWindowUndo.cpp:497-499`): if `m_canvas->mapTool() == m_featureSelectTool && m_featureSelectTool->deleteActivePick()`, returns.
   - `deleteActivePick()` (`src/app/KaFeatureSelectPick.cpp:87`) tries `deleteActiveVertex()` (`src/app/KaFeatureSelectTool.cpp:71-83`: 1 undo step `"꼭짓점 삭제"`, status `"점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다."`, or min-vertex error message while **still returning `true`** so the whole shape is not deleted!) and then `removeActivePiece(false)` (`src/app/KaFeatureSelectPick.cpp:63-85`: `PolygonPieces::withoutPiece`, 1 undo step `"안쪽 도형 삭제"`, status `"안쪽 도형을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다."`, also returning `true` on error).
3. **B3. Active trench deletion (`KaTrenchMoveTool`)**:
   - Next (`src/app/MainWindowUndo.cpp:501-503`): if `m_canvas->mapTool() == m_trenchMoveTool && m_trenchMoveTool->deleteSelectedTrench()`, deletes the picked trench in 1 undo step `"트렌치 삭제"` (`src/core/TrenchLayerEdit.cpp:176`) and returns; right-clicking a trench in `KaTrenchMoveTool` also deletes it (`tests/test_trench_edit.cpp:92-108`).
4. **B4. Deleting selected survey shape(s) (`deleteSelectedFeatures`)**:
   - Triggered when any `KaFeatureSelectTool::allSelectedFeatures(m_canvas)` item is on a non-reference/non-basemap layer (`src/app/MainWindowUndo.cpp:504-510,538-575`).
   - Skips `isReferenceLayer` and `isCadastralLayer` (`:546-548`). Groups selected FIDs per layer and runs `LayerOps::runEditCommand(layer, "도형 삭제", ...)` calling `layer->deleteFeature(fid)` in the edit buffer (nothing written to GPKG until `저장`), followed by `layer->removeSelection()` (`:559-565`).
   - Sets `project->setDirty(true)`, calls `m_featureSelectTool->refreshSelectedGeometry()`, refreshes canvas, and shows status `"도형 %1개를 지웠습니다. Ctrl+Z로 복원할 수 있습니다."` (6000 ms, `:568-573`); on failure warns `"도형 삭제 확인"` (`:574`).
5. **B5. Map-focus fallback to reference/basemap layer removal**:
   - When no survey feature is selected (`src/app/MainWindowUndo.cpp:511-535`): inspects `m_layerTree->selectedLayers()` (or `currentLayer()`). If a selected layer satisfies `LayerOps::isReferenceOrBasemapLayer(layer)`, or `removableLegendLayersFromNode(currentNode())` is non-empty and **contains no survey layer** (`layerKeyOf(layer).isEmpty()`), calls `removeSelectedLayers()`. A survey layer is **never** removed from the project by pressing `Delete` on the map (`:525-533`).
6. **B6. Legend-focus layer removal (`removeLayersFromTree`)**:
   - Collects `tree->selectedLayers()`, `tree->currentLayer()`, and `LayerOps::removableLegendLayersFromNode(node)` for all selected/current tree nodes (`src/app/MainWindowUndo.cpp:160-182`). If nothing is selected, shows `"제거할 레이어를 먼저 클릭하세요."` (4000 ms, `:206-209`).
   - Snapshots `(node->clone(), parent, index)` into `KaRemovedLayers::entries`, stops capture if `m_editLayer == layer`, clears vector selection, detaches the layer from `QgsProject` via `SurveyHandles::release(project->takeMapLayer(layer), m_surveyPath)` (preserving pending edit buffers in memory while releasing OS file locks so `저장` does not fall back to `-저장.gpkg`; `releaseRemovedLayersForSave` at `:72-80` also releases buffered handles on save), and removes the tree node (`:213-237`).
   - Only if a removed layer is `isVworldCadastralPicture`, calls `LayerOps::rememberUserRemovedCadastral(project)` (removing a downloaded cadastral vector layer does **not** suppress the VWorld cadastral basemap, `:267-273`).
   - Pushes `KaUndoAction::LayersRemoved` (capped at `kMaxRemovedLayerUndo = 20`, `src/app/KaEditUndoOrder.cpp:23,61-71`), marks dirty, updates undo actions, and shows `"레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다."` (6000 ms, `src/app/MainWindowUndo.cpp:276-290`).
7. **B7. Legend-focus group removal (empty and non-empty groups)**:
   - Automatically detaches ancestor groups emptied by layer removal (`src/app/MainWindowUndo.cpp:239-252`) **and** detaches explicitly selected top-most `QgsLayerTreeGroup` nodes even when originally empty (`:187-205,255-265`), storing them in `KaRemovedLayers::groups`.
   - `KaLayerTreeUndo::check` (`src/app/KaLayerTreeUndo.cpp:201-203`) ignores row-count changes (`now->rowKeys() == m_baseline->rowKeys()`) so group/layer deletion never creates a spurious reorder undo step on top of `LayersRemoved`.
   - When only empty group(s) were removed (`onlyGroups`), shows `"빈 묶음을 목록에서 지웠습니다. Ctrl+Z로 복원할 수 있습니다."` (6000 ms, `src/app/MainWindowUndo.cpp:266,287-288`).

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | `tests/test_editing_tools.cpp:228-267` | Pentagon on `"feature_poly"`, `KaFeatureSelectTool`. | `tool.deleteActiveVertex()` x2 | Picked vertex at `(200060, 450020)` | 1st `deleteActiveVertex() == true`, `nCoordinates() == 5`, `undoStack()->count() == 1`; 2nd `deleteActiveVertex() == false`. |
| `TestInnerPiece::clickInsideHolePicksItAndDeleteFillsIt` | `tests/test_inner_piece.cpp:159-180` | EPSG:5187 `"survey_area"` polygon `(10,10,90,90)` with hole `(30,30,70,70)` (area `4800`). | Click `(50,50)` -> `tool.deleteActivePick()` -> `undoStack()->undo()` | `QgsPointXY(50, 50)` | `tool.hasActivePiece()`; `deleteActivePick() == true`; `!tool.hasActivePiece()`; `featureCount() == 1`; `area == 6400.0` (`<1e-6`); `dataProvider()->getFeature(fid).area() == 4800.0`; `undoStack()->count() == 1`; after `undo()`, `area == 4800.0`. |
| `TestTrenchEdit::rightClickDeleteIsUndoableAndNotSaved` | `tests/test_trench_edit.cpp:92-108` | 2 saved trenches on `"trial_trench"`, `KaTrenchMoveTool` (`Mode::Whole`). | Right-click `(200011, 450020)` -> `undo()` | `QgsPointXY(200011, 450020), Qt::RightButton` | `layer->featureCount() == 1`; `dataProvider()->featureCount() == 2`; `layer->isModified()`; `edited.count() == 1`; `canUndo()`; after `undo()`, `featureCount() == 2`. |
| `TestTrenchEdit::newUnsavedTrenchesCanBePickedAndDeleted` | `tests/test_trench_edit.cpp:158-180` | 2 unsaved trenches (`replaceWithCells`, negative FIDs) on `"trial_trench"`. | Left-click `(200011, 450015)` -> Right-click `(200011, 450015)` | `QgsPointXY(200011, 450015)` | `!FID_IS_NULL(tool.selectedTrench())`; after right-click `featureCount() == 1` and `FID_IS_NULL(tool.selectedTrench())`. |
| `TestSaveOpen::layerDeleteKeyPreservesSourceAndUndoRestoresPendingEdits` | `tests/test_save_open.cpp:3024-3057` | Opened survey with uncommitted attribute edit `"저장 전 기록"` on `survey_area`; focus on `layerTree`. | `QTest::keyClick(tree, Qt::Key_Delete)` -> `Ctrl+Z` | `Qt::Key_Delete`, `Qt::Key_Z + Qt::ControlModifier` | `!project->mapLayer(id)`; `contents(path) == original`; after `Ctrl+Z`, `project->mapLayer(id) == layer`, `findLayer(id)`, `layer->isModified()`, attribute `== "저장 전 기록"`, `contents(path) == original`. |
| `TestSaveOpen::layerDeleteKeyRemovesReferenceFromMap` | `tests/test_save_open.cpp:3059-3087` | Opened survey + memory reference layer `"현장참고점"` selected in `layerTree`, focus on `canvas`. | `QTest::keyClick(canvas, Qt::Key_Delete)` -> `Ctrl+Z` | `Qt::Key_Delete`, `Qt::Key_Z + Qt::ControlModifier` | `!project->mapLayer(id)`; `!findLayer(id)`; after `Ctrl+Z`, `project->mapLayer(id)` and `findLayer(id)` restored. |
| `TestSaveOpen::layerDeleteKeyRemovesCadastralAndKeepsItGone` | `tests/test_save_open.cpp:3089-3114` | Opened survey + downloaded cadastral layer `"지적도 · 조사 주변 5km"`, focus on `layerTree`. | `QTest::keyClick(tree, Qt::Key_Delete)` | `Qt::Key_Delete` | `!project->mapLayer(id)`; `!LayerOps::userRemovedCadastral(project)`. |
| `TestSaveOpen::cadastralGroupDeleteKeyRemovesChildren` | `tests/test_save_open.cpp:3116-3150` | Downloaded cadastral `"지적도 · 조사 주변 5km"` + separate `"VWorld 지적(본번·부번)"`. | `QTest::keyClick(tree, Qt::Key_Delete)` -> `Ctrl+Z` | `Qt::Key_Delete`, `Qt::Key_Z + Qt::ControlModifier` | `!project->mapLayer(id)`; `project->mapLayer(otherId)` untouched; `Ctrl+Z` restores `id`. |
| `TestSaveOpen::emptyGroupDeleteKeyRemovesRowAndCtrlZRestores` | `tests/test_save_open.cpp:3337-3362` | Empty `QgsLayerTreeGroup` selected in `layerTree`. | `QTest::keyClick(tree, Qt::Key_Delete)` -> `Ctrl+Z` | `Qt::Key_Delete`, `Qt::Key_Z + Qt::ControlModifier` | Group removed on `Delete`; restored by single `Ctrl+Z` (not swallowed by `KaLayerTreeUndo`). |
| `TestSaveOpen::ctrlZRestoresVertexEditsAndGroupedFeatureDeletion` | `tests/test_save_open.cpp:3364-3431` | Multi-feature selection deleted in one `Delete` press + vertex edits. | `Delete` -> `Ctrl+Z` | `Qt::Key_Delete`, `Qt::Key_Z + Qt::ControlModifier` | Multi-shape `Delete` is undone together in 1 `Ctrl+Z` step. |

### D. Rust today
- **A1**: `SAME` — `gui_shell.rs` (`crates/strata-app/src/gui_shell.rs:82-88`) guards with `!ctx.wants_keyboard_input()` and dispatches `egui::Key::Delete` (without Ctrl/Alt/Cmd) to `self.delete_selected()`.
- **A2**: `DIFFERENT` — `Backspace` is handled on the survey sketch only while `capture` (`self.capture_active()`) is true (`crates/strata-app/src/gui_shell.rs:82-91`), not as a synonym for `Delete` in `Select` or `Align` tools.
- **A3**: `DIFFERENT` — Layer rows (`crates/strata-app/src/layer_menu_model.rs:235`, `crates/strata-app/src/gui_layer_panel_menu_b.rs:58-63`) and non-empty group bands (`crates/strata-app/src/gui_layer_panel_rows.rs:224-227`, `crates/strata-app/src/app_state_layer_remove.rs:94-97`) offer right-click `"레이어 삭제"`, **but** empty groups cannot exist in `left_layers.entries` and group bands cannot be selected with left-click (`crates/strata-app/src/gui_layer_panel_rows.rs:180-215`).
- **B1**: `DIFFERENT` — `delete_selected` (`crates/strata-app/src/app_state_layer_remove.rs:106-120`) routes `self.home_map.legend_focus` to `remove_layers` and `self.capture_active()` to `sketch_undo_vertex()` (`"꼭짓점 하나를 되돌렸습니다."`), **but** returns early whenever `self.active_tab != MainTab::Map` (`:107-109`, no sheet/3D studio routing) and has no align-point-list focus branch.
- **B2**: `MISSING` — No active vertex or active inner-piece deletion exists; `delete_selected` (`crates/strata-app/src/app_state_layer_remove.rs:121-122`) deletes the entire selected survey shape.
- **B3**: `MISSING` — No `KaTrenchMoveTool` active-trench `Delete` or right-click delete branch exists in `crates/strata-app/src/app_state_layer_remove.rs:106-126`.
- **B4**: `DIFFERENT` — `delete_shape` (`crates/strata-app/src/app_state_layer_remove.rs:130-141`, `crates/strata-storage/src/edit_buffer.rs:84-92`) deletes the selected feature in `LayerEditBuffer`, pushes `UndoStep::Table`, clears `redo_tables` and `selected_shape`, sets `is_dirty = true`, and displays `"도형 1개를 지웠습니다. Ctrl+Z로 복원할 수 있습니다."`, **but** operates on only 1 feature (`selected_shape: Option<(String, i64)>`) rather than multi-selected features across layers.
- **B5**: `SAME` — When `legend_focus` is false and no survey shape is selected, `delete_selected` (`crates/strata-app/src/app_state_layer_remove.rs:123-125`) removes the selected legend row iff `*group != LegendGroup::SurveyData`, keeping survey layers safe from map-focus `Delete`.
- **B6**: `DIFFERENT` — `remove_layers` (`crates/strata-app/src/app_state_layer_remove.rs:30-61`) detaches `LayerTreeEntry` and `SurveyFileTable` (preserving unsaved edits in `RemovedLayer::table` without modifying the GPKG file on disk and excluding `removed_table_keys()` on save at `crates/strata-app/src/app_state_projects.rs:236-237`), pushes `UndoStep::LayersRemoved`, and shows `"제거할 레이어를 먼저 클릭하세요."` / `"레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다."`, **but** `delete_selected` (`:111-115`) supports only single-row selection, enforces no `kMaxRemovedLayerUndo = 20` cap, and does not distinguish VWorld cadastral picture `userRemovedCadastral` from downloaded cadastral vector removal.
- **B7**: `MISSING` — Group headers are painted as section bands on the first layer row of each group (`crates/strata-app/src/gui_layer_panel_rows.rs:47-53,88-106`), so group headers cannot be selected or deleted with the `Delete` key, empty groups cannot exist, `RemovedLayer` (`crates/strata-app/src/app_state_layer_remove.rs:22-26`) has no group variant, and `"빈 묶음을 목록에서 지웠습니다. Ctrl+Z로 복원할 수 있습니다."` is absent.

---

## 3. Ctrl+Z and Ctrl+Y

### A. C++ offered
- **A1. Window shortcuts & canvas event filter**:
  - `m_actUndo`: `QAction` `"되돌리기"`, icon `"undo"`, tooltip `"마지막 그리기·정점·삭제를 되돌립니다 (Ctrl+Z)"`, shortcut `QKeySequence::Undo`, `Qt::WindowShortcut`, calls `routeEditKeyToActiveStudio(false)` then `undoLastAction()` (`src/app/MainWindowRibbon.cpp:311-320`).
  - `m_actRedo`: `QAction` `"다시 실행"`, icon `"redo"`, tooltip `"되돌린 편집을 다시 적용합니다 (Ctrl+Y)"`, shortcut `QKeySequence::Redo`, `Qt::WindowShortcut`, calls `redoLastAction()` (`src/app/MainWindowRibbon.cpp:321-329`).
  - Canvas `eventFilter` (`src/app/MainWindow.cpp:2294-2305`): intercepts `QKeySequence::Undo` or `Ctrl+Z` -> `undoLastAction()`, and `QKeySequence::Redo` or `Ctrl+Y` -> `redoLastAction()`.
  - Guarded by `editingText()` (`src/app/MainWindowUndo.cpp:51-55,303,313,326,470`) and `!m_isOpeningSurvey && !m_closingWindow` (`:326,470`).
- **A2. Map canvas floating Undo/Redo pill & dynamic enablement**:
  - `MainWindow::updateUndoRedoActions` (`src/app/MainWindowUndo.cpp:292-300`) sets `m_actUndo->setEnabled(canUndo)` (`captureVertex || newestUndoLayer() || newestFallbackUndoIndex() >= 0`) and `m_actRedo->setEnabled(canRedo)` (`newestRedoLayer() != nullptr`).
  - Floating map pill (`KaMapUndoPill` / `test_map_polish.cpp:83-115`, `test_shell_chrome.cpp:244-264`) mirrors `m_actUndo` and `m_actRedo` enabled state and tooltips.
- **A3. Draw subtoolbar sketch `"되돌리기"` button**:
  - `QAction` `undoAct`, icon `"undo"`, text `"되돌리기"`, `kaSketch = "undo"`, tooltip `"마지막 점 하나를 지웁니다. Ctrl+Z·Backspace와 같습니다."` (`src/app/KaDrawSketchTools.cpp:102-108`).
  - Enabled and visible **only** while `sketching` (`m_captureTool && m_canvas->mapTool() == m_captureTool && m_captureTool->hasSketch()`, `src/app/KaDrawSketchTools.cpp:144-145,167-168`).

### B. C++ does
1. **B1. Active-tab routing (`undoLastAction` / `redoLastAction`)**:
   - `undoLastAction` (`src/app/MainWindowUndo.cpp:302-310`): if current tab is `m_terrain3dLayoutStudio`, calls `undoLastChange()`; else if `routeEditKeyToActiveStudio(false)` (2D `KaDrawingStudio`), routes there; else calls `undoMapAction()`.
   - `redoLastAction` (`src/app/MainWindowUndo.cpp:312-323`): routes to `m_terrain3dLayoutStudio->redoLastChange()` or `m_drawingStudio->handleRedoKey()`, else `redoMapAction()`.
2. **B2. Active-tool pre-stack undo (`undoMapAction` steps 1–2)**:
   - Step 1 (`src/app/MainWindowUndo.cpp:328-333`): if on the map tab and `m_canvas->mapTool() == m_captureTool && m_captureTool->undoLastVertex()`, removes the last sketch vertex and shows `"꼭짓점 하나를 되돌렸습니다."` (4000 ms). (A sketch left open while viewing another tab is untouched, `:327-328`.)
   - Step 2 (`src/app/MainWindowUndo.cpp:334-337`): if `m_canvas->mapTool() == m_alignTool && m_alignTool->removeLastPair()`, shows `m_alignTool->statusText()` (4000 ms) and returns.
3. **B3. Per-layer `QUndoStack`s ordered on one global timeline (`EditHistory`)**:
   - Architecture (`CLAUDE.md:38`): each editable survey `QgsVectorLayer` has its own `QUndoStack` (`undoableSurveyLayer` excludes reference and cadastral layers, `src/app/KaEditUndoOrder.cpp:26-30`).
   - `EditHistory` (`src/core/EditHistory.cpp:11-147`) watches `QUndoStack::indexChanged` on every vector layer and stamps each command with monotonic `m_seq`: `id` and `recent` when pushed/redone (`:77-85`), and `undone = ++m_seq` in reverse index order when undone so the command undone **last** gets the largest `undone` timestamp and is redone **first** (`:89-93`).
   - `undoMapAction` (`src/app/MainWindowUndo.cpp:341-365`) compares `stackTime = editHistory()->undoTop(stackLayer).recent` (`newestUndoLayer()`, `src/app/KaEditUndoOrder.cpp:105-122`) against `fallbackTime` (`newestFallbackUndoIndex()`, `:83-103`). When `stackLayer` is newer (`stackTime >= fallbackTime`), runs `undoLinkedEdit(stackLayer) || LayerOps::undoLayerEdits(stackLayer)` (`src/core/LayerOps.cpp:2689-2695`), marks project dirty, refreshes selection handles & canvas, and shows `"이전 상태로 되돌렸습니다."` (4000 ms); if neither exists, shows `"되돌릴 것이 없습니다."` (4000 ms, `src/app/MainWindowUndo.cpp:362-364`).
   - `redoMapAction` (`src/app/MainWindowUndo.cpp:469-484`) picks `newestRedoLayer()` (`src/app/KaEditUndoOrder.cpp:124-140`, max `redoTop(vector).undone`), runs `redoLinkedEdit(layer) || LayerOps::redoLayerEdits(layer)` (`src/core/LayerOps.cpp:2697-2703`), and shows `"다시 실행했습니다."` (4000 ms) or `"다시 실행할 것이 없습니다."` (4000 ms).
4. **B4. Single-step macro grouping (`KaUndoGroup`)**:
   - `KaUndoGroup(layer, "도형 그리기")` (`src/app/KaUndoGroup.h:13-36`, `src/app/MainWindowEditing.cpp:828`) opens `QUndoStack::beginMacro(title)` before `layer->addFeature(feat)` and stays open across `KaFeatureFormDialog` `"이름·번호"` (`:860`), so `Ctrl+Z` / `Ctrl+Y` undoes and redoes both the drawn geometry and its entered name/number in **one** step labeled `"도형 그리기"`.
   - If post-sketch `"겹친 곳 지우기"` is chosen, `drawStep.close()` runs **before** the erase (`src/app/MainWindowEditing.cpp:848`), keeping `"도형 그리기"` and `"겹친 곳 지우기"` as **two** separate steps (`tests/test_undo_group.cpp:63-74`).
   - If 0 child commands were pushed inside `KaUndoGroup`, `close()` marks the macro `setObsolete(true)` and calls `stack->undo()` so no no-op step remains (`src/app/KaUndoGroup.h:30-34`).
5. **B5. Cross-layer linked undo/redo (`KaLinkedEdit`)**:
   - When one user action edits `>= 2` layers simultaneously (`PolygonErase::apply` cutting a target on layer A and deleting a cutter on layer B), `MainWindow::rememberLinkedEdit` (`src/app/MainWindowErase.cpp:174-188`) records `(layerId, stack->index(), command*, text)` in `m_linkedEdits`.
   - `undoLinkedEdit` (`:191-216`) verifies all member commands are still on top of their respective stacks, undoes all members together, and pushes the group onto `m_linkedRedos`; `redoLinkedEdit` (`:219-244`) redoes all members together and moves the group back to `m_linkedEdits`. `forgetStaleLinkedEdits` (`:160-170`) prunes invalidated groups and clears `m_linkedRedos` before new edits.
6. **B6. App-level `m_undoActions` timeline (`LayersRemoved`, `LayerTreeChanged`, `LayerAdded`)**:
   - Stored in `m_undoActions` (capped at `kMaxUndoActions = 300`, `kMaxRemovedLayerUndo = 20`, `src/app/KaEditUndoOrder.cpp:23-24,60-73`) with `seq = history->next()` on the **same** `EditHistory` clock (`:58`):
     - `LayersRemoved`: restores detached groups (outer-first) and layers via `SurveyHandles::reattach` (`src/app/MainWindowUndo.cpp:369-390`).
     - `LayerTreeChanged`: `KaLayerTreeUndo` (`src/app/KaLayerTreeUndo.cpp:16,180-205`) captures user-initiated (`kGestureWindowMs = 800` ms) pure visibility/order changes (`now->rowKeys() == m_baseline->rowKeys()`) and restores via `KaLayerTreeUndo::restore` (`src/app/MainWindowUndo.cpp:391-393`).
     - `LayerAdded`: removes `action.layerId` (e.g. `[클립] ...` from `구간 분리`, `src/app/MainWindowUndo.cpp:394-402`).
   - Popped from `m_undoActions` on undo (`:459`) with status `"이전 상태로 되돌렸습니다."` (`:466`).
7. **B7. Undo across `저장` (generation save / `commitChanges`)**:
   - Saving a survey writes pending buffers to the next generation GPKG and calls `rollBack(false)` + `undoStack()->clear()` while `m_isOpeningSurvey == true`; `beforeCommitChanges` / `beforeRollBack` call `editHistory()->commitApplied(layer)` (`src/app/MainWindowUndo.cpp:92-103`, `src/core/EditHistory.cpp:65-73,98-106`), marking applied commands `State::Committed` (keeping `lastRecent`) and undone commands `State::Discarded`.
   - Fallback `m_undoActions` (`FeatureAdded`, `FeatureChanged`, `AttributesChanged`, `FeatureDeleted`) paired with `commandId` are skipped while `Applied`/`Undone`, pruned when `Discarded`, and become active once `Committed` (`src/app/KaEditUndoOrder.cpp:75-103`), with negative FIDs remapped to saved FIDs via `committedFeaturesAdded` and `remapUndoFeatureIdsAfterSave` (`src/app/MainWindowUndo.cpp:104-156,403-453`).
8. **B8. What is NOT undoable (or not redoable)**:
   - **Not redoable by `Ctrl+Y`**: every `m_undoActions` fallback (`LayersRemoved`, `LayerTreeChanged`, `LayerAdded`, post-save `Feature*` fallbacks) is removed on `Ctrl+Z` (`src/app/MainWindowUndo.cpp:459`) and `redoMapAction` (`:469-484`) inspects only `newestRedoLayer()` (`QUndoStack`).
   - **Not undoable**: layer style/color/stroke/label changes (`src/app/MainWindowEditing.cpp:609-610`), layer opacity (`src/app/MainWindowContextMenus.cpp:381`), layer rename, map pan/zoom, reference/cadastral layers (`src/app/KaEditUndoOrder.cpp:28`), and `explodeMultipartFeatures` / `splitPolygonWithLine` when invoked on a layer where `isEditable()` was `false` (because `commitChanges()` clears `QUndoStack` with no `m_undoActions` entry, `src/core/LayerOps.cpp:648,666,822,866`).

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestEditingHistory::stampsOrderEditsAcrossLayers` | `tests/test_editing_history.cpp:42-64` | Layers `"A"`, `"B"` (EPSG:5186); `EditHistory`. | `addSquare(a,0)` -> `addSquare(b,20)` -> `history.next()` -> `addSquare(a,40)` | `0, 20, next(), 40` | `first.id > 0 && first.recent > 0`; `second.recent > first.recent`; `removal > second.recent`; `undoTop(a).recent > removal`; `redoTop(a).command == nullptr`. |
| `TestEditingHistory::commandStateFollowsUndoCommitAndDiscard` | `tests/test_editing_history.cpp:66-95` | Layer `"유구면"`, `EditHistory`. | `addSquare(0)` -> `addSquare(20)` -> `undo()` -> `addSquare(40)` -> `commitChanges(false)` | `0, 20, undo, 40, commitChanges(false), state(0)` | `state(kept) == Applied`; `state(undone) == Undone`; after `addSquare(40)`, `state(undone) == Discarded`; after `commitChanges`, `state(kept) == Committed`, `state(last) == Committed`, `lastRecent(last) > lastRecent(kept)`, `undoTop.command == nullptr`, `state(0) == Unknown`. |
| `TestEditingHistory::saveThatDropsTheBufferKeepsWrittenCommandsCommitted` | `tests/test_editing_history.cpp:97-128` | Layer `"유구면"`, `EditHistory`. | `addSquare(0)`, `addSquare(20)`, `undo()`, `commitApplied`, `rollBack(false)`, `clear()`; then plain `rollBack(false)` | See call | `state(written) == Committed`; `state(undoneBefore) == Discarded`; `lastRecent(written) > 0`; plain rollback -> `state(discarded) == Discarded`. |
| `TestEditingHistory::redoPrefersTheLastUndone` | `tests/test_editing_history.cpp:130-148` | Layers `"A"`, `"B"`, `EditHistory`. | `addSquare(a,0)`, `addSquare(b,20)`, `b->undo()`, `a->undo()`, `a->redo()` | `0, 20` | `redoTop(a).undone > redoTop(b).undone`; after `a->redo()`, `state(undoTop(a).id) == Applied` and `undoTop(a).recent > redoTop(b).undone`. |
| `LayerTreeUndoTest::restorePutsOrderAndChecksBack` | `tests/test_layer_tree_undo.cpp:30-55` | Root `"A"` + group `"참조 지도"` (`"B"`, `"C"`). | Reorder, move `"A"` into group, uncheck `"B"` -> `restoreInto` | `*before` | `names(root) == {"A", "참조 지도"}`; `names(refs) == {"B", "C"}`; `b` checked; `*capture(root) == *before`. |
| `LayerTreeUndoTest::onlyTheUsersChangesBecomeUndoSteps` | `tests/test_layer_tree_undo.cpp:57-90` | Root `"A"`, `"B"`, `KaLayerTreeUndo`. | Programmatic move; `noteUserGesture` + uncheck `"A"`; add `"C"`; remove empty group `"빈 묶음"`; `restore` | See call | Programmatic -> `pushed.isEmpty()`; uncheck -> `pushed.size() == 1`; add `"C"` & remove `"빈 묶음"` -> still `pushed.size() == 1`; `restore` -> `A` checked, `pushed.size() == 1`. |
| `TestUndoGroup::*` (3 tests) | `tests/test_undo_group.cpp:42-82` | Memory Polygon `"유구"` EPSG:5186. | `KaUndoGroup("도형 그리기")` with `addFeature` + `rename("1호 주거지")`; `close()` before `"겹친 곳 지우기"`; empty scope | `"1호 주거지"` | Test 1: `stack->count() == 1`, `text(0) == "도형 그리기"`, `undo()` -> `0` features, `redo()` -> `1` feature `"1호 주거지"`. Test 2: `count() == 2`, 1 `undo()` leaves `1` feature. Test 3: `count() == 0`, `!canRedo()`. |
| `TestEditBufferSummary::*` (5 tests) | `tests/test_edit_buffer_summary.cpp:47-156` | Point layers EPSG:5186 (`3`, `2`, `200` + `9800` features). | `EditBufferSummary::uniqueEditedIds` & `summarize` | Attribute + geom edits, add, delete, `project.setDirty`, 10,000 features | Attr+geom on same feature -> `1`; add+attr+del+geom -> `3`; unedited editable layer -> `layers == 0`; `projectDirty` -> `unsaved() == true, features == 0`; 10,000 edited features best of 20 runs `< 10'000'000` ns (`10 ms`). |
| `TestSaveOpen::eraseOverlap_acrossLayersUndoesAndRedoesTogether` | `tests/test_save_open.cpp:3592-3643` | `survey_area` (`outer` `10000` m²) + `feature_poly` (`cutter` `400` m²). | `eraseOverlapWithShape` -> `Ctrl+Z` -> `Ctrl+Y` -> `Ctrl+Z` | `190040..190060, 560040..560060` | Erase: `area == 9600.0`, `poly->featureCount() == 0`; `Ctrl+Z`: `area == 10000.0 && poly->featureCount() == 1`; `Ctrl+Y`: `9600.0 && 0`; `Ctrl+Z`: `10000.0 && 1`. |

### D. Rust today
- **A1**: `DIFFERENT` — `gui_shell.rs` (`crates/strata-app/src/gui_shell.rs:126,152-157`) handles `Ctrl+Z` (`self.undo_map_action()`) and `Ctrl+Y` (`self.redo_map_action()`) when `!wants_kb`, **but** does not gate out `Shift`/`Alt` (so `Ctrl+Shift+Z` triggers **Undo** instead of Redo!) and skips studio routing (`undoLastAction` / `redoLastAction`).
- **A2**: `DIFFERENT` — Floating map pill (`crates/strata-app/src/gui_tab_map_undo.rs:8-143`) renders `[↶]` and `[↷]` with matching tooltips `"마지막 그리기·정점·삭제를 되돌립니다 (Ctrl+Z)"` and `"되돌린 편집을 다시 적용합니다 (Ctrl+Y)"`, **but** always paints active ink (`INK`) and never dims/disables when `undo_tables` or `redo_tables` is empty.
- **A3**: `SAME` — Subtoolbar sketch `"undo"` button (`crates/strata-app/src/gui_ribbon_draw_row.rs:169-186`) appears only while `sketching` with exact tooltip `"마지막 점 하나를 지웁니다. Ctrl+Z·Backspace와 같습니다."` and calls `self.sketch_undo_vertex()`.
- **B1**: `DIFFERENT` — `undo_map_action` and `redo_map_action` (`crates/strata-app/src/app_state_draw_commit_b.rs:31-74`) always run map undo/redo regardless of `self.active_tab` (no 2D Drawing Studio or 3D Terrain Studio undo/redo routing).
- **B2**: `DIFFERENT` — `undo_map_action` (`crates/strata-app/src/app_state_draw_commit_b.rs:32-34`, `crates/strata-app/src/app_state_draw_commit.rs:106-112`) undoes the last sketch vertex first when `self.capture_active()` (`"꼭짓점 하나를 되돌렸습니다."`), **but** has no `KaAlignMapTool::removeLastPair()` step.
- **B3**: `SAME` — Per-table `LayerEditBuffer` undo/redo stacks (`crates/strata-storage/src/edit_buffer.rs:16-23,94-112`) ordered globally via `undo_tables` / `redo_tables` (`crates/strata-app/src/app_state_draw_commit_b.rs:31-74`), with exact status strings `"이전 상태로 되돌렸습니다."`, `"되돌릴 것이 없습니다."`, `"다시 실행했습니다."`, and `"다시 실행할 것이 없습니다."`.
- **B4**: `DIFFERENT` — Drawing + `FeatureForm` (`crates/strata-app/src/app_state_draw_commit.rs:199-210`, `crates/strata-app/src/app_state_feature_form.rs:53-76`) produces 1 undo step because `add_unsaved_feature` pushes one snapshot and `apply_feature_form` mutates `features` in place without pushing a second snapshot, **but** there is no `KaUndoGroup` macro class or command label (`"도형 그리기"`).
- **B5**: `MISSING` — `UndoStep::Table(String)` (`crates/strata-app/src/app_state_layer_remove.rs:13-18`) tracks a single table name only; `MultiLayerUndoStack` (`crates/strata-storage/src/polygon_pieces.rs:145-180`) is called nowhere in `strata-app`, so no `KaLinkedEdit` multi-layer undo/redo group is active.
- **B6**: `DIFFERENT` — `UndoStep::LayersRemoved(Vec<RemovedLayer>)` and `UndoStep::RowsMoved(Vec<String>)` (`crates/strata-app/src/app_state_layer_remove.rs:13-18`, `crates/strata-app/src/app_state_draw_commit_b.rs:38-47`) share the `undo_tables` timeline with table edits, **but** layer-checkbox visibility undo (`KaLayerTreeUndo`), `kMaxUndoActions = 300` / `kMaxRemovedLayerUndo = 20` caps, and `KaUndoAction::LayerAdded` are missing.
- **B7**: `DIFFERENT` — On `저장`, `write_survey_to` (`crates/strata-app/src/app_state_projects.rs:232-234`) overwrites `self.survey_tables` with fresh `read_survey_tables` whose `LayerEditBuffer` undo/redo stacks are empty, **without** clearing stale `UndoStep::Table` entries from `self.undo_tables` / `self.redo_tables` and with no post-save fallback undo: pressing `Ctrl+Z` right after `Ctrl+S` skips all saved table edits (`t.layer.undo() == false`, `crates/strata-app/src/app_state_draw_commit_b.rs:49-58`) and either undoes an older `LayersRemoved`/`RowsMoved` step or says `"되돌릴 것이 없습니다."`.
- **B8**: `SAME` — `LayersRemoved` and `RowsMoved` are not pushed to `redo_tables` (`crates/strata-app/src/app_state_draw_commit_b.rs:38-47`), and layer style/color/opacity/rename, pan/zoom, and reference/cadastral data are not undoable on layer edit stacks.

---

## 4. 꼭짓점 수정: Adding and Deleting a Vertex, Moving One

### A. C++ offered
- **A1. Automatic vertex handles inside `도형선택` (no separate ribbon tool)**:
  - `KaFeatureSelectTool` owns a private `KaVertexEditTool* m_vertex` (`src/app/KaFeatureSelectTool.h:105`, `src/app/KaFeatureSelectTool.cpp:39-45`; header note `src/app/KaVertexEditTool.h:30,51-53`: there is no separate `"도형수정"` map-tool button).
  - Whenever `allSelectedFeatures(mCanvas).size() == 1` and `isPickableLayer(all[0].layer)`, `syncVertexTarget()` (`src/app/KaFeatureSelectTool.cpp:111-119`) calls `m_vertex->setTarget(layer, fid)` (`src/app/KaVertexEditTool.cpp:60-69`), rendering:
    - Outline `QgsRubberBand` (`QColor(30, 103, 198, 200)` width `2`, fill `QColor(30, 103, 198, 40)`, `:158-167`).
    - One `QgsVertexMarker::ICON_BOX` (`penWidth = 2`) per vertex (`:177-197`): inactive size `14`, stroke `QColor(30, 103, 198)`, fill `QColor(255, 255, 255)`; active (`m_activeIndex`) size `16`, stroke `QColor(220, 38, 38)`, fill `QColor(254, 226, 226)`.
- **A2. Layer tree context menu `"꼭짓점 수정"` (`layer.vertices`)**:
  - Offered on non-Point editable vector layers (`editableKind && vector->geometryType() != Point`, `src/app/MainWindowContextMenus.cpp:263-269,412-418`).
  - Text: `"꼭짓점 수정"` (`:265`). Tooltip when disabled (`vertexReason`, `:226-238`): `"조사 파일 처리가 끝난 뒤 다시 실행하세요."` (busy), `"이 자료가 저장된 조사 파일을 먼저 열어 주세요."` (`domain && !ownSurvey`), `"레이어 파일을 찾을 수 없습니다. 자료를 다시 열어 주세요."` (`!valid`), `"이 레이어에는 도형이 없습니다."` (`!hasFeatures`), or `editUnavailable(vector, ChangeGeometries)`.
  - Action (`:265-268`): activates `startSelectTool()` if not already in `m_featureSelectTool` and shows `"고칠 도형을 클릭한 뒤 꼭짓점을 끌어 옮기세요."` (8000 ms).
- **A3. Right-click canvas menu (`점삭제` / `점추가`), `Delete`/`Backspace`, and edge double-click**:
  - In `KaFeatureSelectTool::handleContextMenu` (`src/app/KaFeatureSelectTool.cpp:306-360,409-415`):
    - Checks `vtxIdx = m_vertex->vertexNear(mapPt, KaEditTolerance::kVertexMenuPx)` (`12` px, `src/app/KaEditTolerance.h:9`); if `< 0`, checks `segAfter = m_vertex->segmentNear(mapPt, &onLine)` (`kSegmentPx = 10` px, `src/app/KaEditTolerance.h:11`).
    - Adds `"점삭제"` when `vtxIdx >= 0`, else `"점추가"` when `segAfter >= 0` (`:357`), disabling with tooltip if `vertexReason` / min-vertex guard fails (`:326-347,358-359`).
  - Pressing `Delete` or `Backspace` while `m_activeIndex >= 0` deletes the active vertex (`src/app/KaFeatureSelectTool.cpp:71-83,160-163`).
  - Standalone `KaVertexEditTool` (`src/app/KaVertexEditTool.cpp:433-447,494-503`, not set as canvas tool in normal flow) also supports right-click `"점추가"`/`"점삭제"` (`kVertexMenuFarPx = 14`, `src/app/KaEditTolerance.h:10`) and left double-click on a segment to insert a vertex (`:494-503`).

### B. C++ does
1. **B1. Picking and preview-dragging a vertex**:
   - Press (`src/app/KaFeatureSelectTool.cpp:177-188`): calls `m_vertex->vertexNear(e->mapPoint())` (`kVertexGrabPx = 10` px, `src/app/KaEditTolerance.h:8`, `src/app/KaVertexEditTool.cpp:243-260`). If `idx >= 0`, sets `m_vertexDragging = true`, `m_vertexMoved = false`, `m_vertexIndex = idx`, `m_vertex->setActiveVertex(idx)`, and clears any active inner piece.
   - Move (`src/app/KaFeatureSelectTool.cpp:197-206`): ignores movement `<= kClickSlopPx` (`4` px Manhattan, `src/app/KaEditTolerance.h:12`); once exceeded, sets `m_vertexMoved = true` and calls `m_vertex->previewVertexMove(m_vertexIndex, m_vertex->toLayer(dragPoint(e)))` (`src/app/KaVertexEditTool.cpp:75-86`), moving both `index` and `LayerOps::ringClosingVertex(geom, index)` in the rubber band without touching the edit buffer.
   - Release without drag (`!moved`, `src/app/KaFeatureSelectTool.cpp:241-246`): commits nothing (0 undo steps) and emits `"점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다."`.
2. **B2. Committing a vertex move (`moveVertexTo`)**:
   - Release after drag (`src/app/KaFeatureSelectTool.cpp:247-258`): computes `target = m_vertex->toLayer(dragPoint(e))` (raw `e->mapPoint()`, or `snapMapPointExcludingTarget` when `Ctrl` is held, `:102-109`) and calls `m_vertex->moveVertexTo(idx, target)` (`src/app/KaVertexEditTool.cpp:334-355`).
   - Runs `LayerOps::runEditCommand(m_layer, "꼭짓점 이동", ...)` -> `LayerOps::applyVertexMove` (`src/core/GeometryEditVertex.cpp:76-156`).
   - Polygon ring-closure rule (`LayerOps::ringClosingVertex`, `src/core/GeometryEditVertex.cpp:65-74`): for `Polygon`, maps flat vertex `index` via `geometry.vertexIdFromVertexNr(vertex, id)`; if `id.vertex == 0`, returns `vertex + count - 1`; if `id.vertex == count - 1`, returns `vertex - (count - 1)` — moving both endpoints together so exterior rings and interior holes remain closed (`tests/test_polygon_erase.cpp:361-388`).
   - Undo step: 1 command `"꼭짓점 이동"` on `m_layer->undoStack()`.
   - Status (`src/app/KaFeatureSelectTool.cpp:253-256`): `"수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다."`, or `m_vertex->lastEditError()` / `"수정점을 옮기지 못했습니다."`.
3. **B3. Topological shared-vertex move (`공유 경계`)**:
   - `topological = m_layer->project() && m_layer->project()->topologicalEditing()` (`src/app/KaVertexEditTool.cpp:343`), toggled by the draw subtoolbar `"공유 경계"` checkbox (`KaSnapSettingsWidget`).
   - When `topological == true`, `LayerOps::applyVertexMove` (`src/core/GeometryEditVertex.cpp:20,114-133`) queries `m_layer` with spatial filter `QgsRectangle(from.x ± 0.001, from.y ± 0.001)` (`kSharedVertexTolerance = 0.001` m = 1 mm, `tol2 = 1e-6` m²) and moves **every** vertex on the same layer within `sqrDist <= tol2` to `(x, y)` inside the **same** `"꼭짓점 이동"` edit command.
4. **B4. Adding a vertex (`insertVertexAt`)**:
   - `KaVertexEditTool::segmentNear` (`src/app/KaVertexEditTool.cpp:262-274`) calls `geom.closestSegmentWithContext(toLayer(mapPt_), closest, afterVertex)` and requires `std::sqrt(sqrDist) <= layerTolerance(kSegmentPx)` (`10` px).
   - `insertVertexAt(segAfter, onLine)` (`:381-390`) calls `geom.insertVertex(at.x(), at.y(), index)` -> `applyGeometryChange(geom, "꼭짓점 추가")` (`:392-416`), which guards `!layer->isEditCommandActive()` (`"다른 도형 편집이 진행 중입니다. 편집을 마친 뒤 다시 시도하세요."`), skips no-op `isExactlyEqual`, and runs `LayerOps::runEditCommand(layer, "꼭짓점 추가", layer->changeGeometry(...))`.
   - Undo step: 1 command `"꼭짓점 추가"`. Status (`src/app/KaFeatureSelectTool.cpp:413`): `"점을 넣었습니다."` (`KaVertexEditTool.cpp:441,501`: `"꼭짓점을 넣었습니다."`).
5. **B5. Deleting a vertex (`deleteVertexAt`)**:
   - Triggered by `Delete`/`Backspace` on `m_activeIndex` (`KaFeatureSelectTool::deleteActiveVertex`, `src/app/KaFeatureSelectTool.cpp:71-83`) or right-click `"점삭제"` (`:410`).
   - Per-ring minimum vertex check (`src/app/KaVertexEditTool.cpp:362-374`): resolves `geom.vertexIdFromVertexNr(index, ring)` and counts `geom.constGet()->vertexCount(ring.part, ring.ring)` (counting **only that ring**, so a 4-point outer ring cannot lose a corner just because the polygon also has a hole!). Requires `count >= minCount`, where `minCount = 5` for `Polygon` (3 corners + closing vertex; deleting 1 leaves 4) and `minCount = 3` for `Line` (deleting 1 leaves 2).
   - Refusal messages (`:370-373`, also in `:344-345` menu tooltip):
     - Polygon: `"면은 꼭짓점 3개보다 줄일 수 없습니다."`
     - Line: `"선은 꼭짓점 2개보다 줄일 수 없습니다."`
   - Calls `geom.deleteVertex(index)` -> `applyGeometryChange(geom, "꼭짓점 삭제")`, resets `m_activeIndex = -1` (`:375-378`).
   - Undo step: 1 command `"꼭짓점 삭제"`. Status: `"점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다."` (`Delete` key, `src/app/KaFeatureSelectTool.cpp:78`) or `"점을 지웠습니다."` (context menu, `:413`).
6. **B6. Map-to-layer CRS transformation (`toLayer`, `toMap`, `layerTolerance`)**:
   - `KaVertexEditTool::toLayer` / `toMap` (`src/app/KaVertexEditTool.cpp:114-122`) convert between canvas CRS (e.g. EPSG:5187) and layer CRS (e.g. EPSG:5186) via `QgsMapTool::toLayerCoordinates` / `toMapCoordinates`; `layerTolerance(px)` (`:125-134`) transforms `(center, center + (d, 0))` at canvas center to scale pixel tolerances into layer units.
7. **B7. Post-edit selection & undo-fallback bookkeeping**:
   - Feature `m_fid` (including negative unsaved IDs) stays selected with refreshed vertex markers (`src/app/KaFeatureSelectTool.cpp:76,251,411`); `featureGeometryEdited(layer, before)` pushes a post-save fallback `KaUndoAction::FeatureChanged` (`src/app/MainWindowEditing.cpp:210-221`).

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestEditingTools::clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex` | `tests/test_editing_tools.cpp:228-267` | Pentagon (6 coords) on `"feature_poly"` EPSG:5186, `KaFeatureSelectTool`. | Click handle -> `deleteActiveVertex()` x2 -> drag vertex | Handle `(200060,450020)`, drag `(200070,450045)` by `+QPoint(30,12)` | Click handle: `!isModified()`, `undoStack()->count() == 0`. `deleteActiveVertex()`: `true`, `nCoordinates() == 5`, `undoStack()->count() == 1`; 2nd call `false`. Drag: vertex within `1e-6`, `undoStack()->count() == 2`. |
| `LayerEditsTest::sharedVertexMoveTouchesOnlyNeighbours` | `tests/test_layer_edits.cpp:108-129` | `"feature_poly"` EPSG:5187: `left` `(0,0,10,10)`, `right` `(10,0,20,10)`, `far` `(500,500,510,510)`. | `LayerOps::applyVertexMove(layer.get(), left, 2, 11, 12, true, &error)` | `vertex=2` at `(10,10)` -> `(11, 12)`, `topological=true` | `hasVertex(left, (11,12))`; `hasVertex(right, (11,12))`; `hasVertex(far, (500,500))`. |
| `PolygonEraseTest::movingTheFirstVertexKeepsAHoleClosed` | `tests/test_polygon_erase.cpp:361-388` | `"survey_area"` EPSG:5187 polygon `(0,0,100,100)` with hole `(40,40,60,60)`. | `LayerOps::moveFeatureVertex` on vertex `0` `(0,0)->(-5,-5)` and vertex `5` (hole start `(40,40)->(35,40)`), `topological=false` | `0 -> (-5,-5)`, `5 -> (35,40)` | `isGeosValid()` after both moves; `vertexAt(0) == vertexAt(4) == (-5,-5)`; `vertexAt(5) == vertexAt(9) == (35,40)`. |
| `TestInnerPiece::unsavedShapeIsPickedAndEdited` | `tests/test_inner_piece.cpp:108-129` | `"feature_poly"` EPSG:5187 with uncommitted square `(40,40,60,60)` (`fid < 0`). | Click `(50,50)` -> drag corner `(40,40)` to `(35,40)` -> `undoStack()->undo()` | `QgsPointXY(50,50)`, `(40,40)->(35,40)` | `fid < 0`; `selectedFeatureIds() == {fid}`; after drag `xMinimum() == 35.0` (`<1e-6`), `undoStack()->count() == 2`; after `undo()`, `xMinimum() == 40.0`. |
| `TestInnerPiece::anySurveyLayerGetsTheMenuAndARightClickStartsNoCycle` | `tests/test_inner_piece.cpp:221-238` | User polygon layer `user_poly_1` (`outer` `(10,10,90,90)` + `inner` `(30,30,50,50)`). | Right-click corner `(30,30)` -> Left-click `(40,40)` | `(30,30)` RightButton, `(40,40)` LeftButton | Popup menu contains enabled `"점삭제"`; subsequent left-click at `(40,40)` still picks `inner` (no cycle started by right-click). |
| `TestBuffer::moveFeatureVertex_updatesPolygonCorner` | `tests/test_buffer.cpp:199-223` | Memory Polygon EPSG:5187 `rect(0,0,10,10)`. | `LayerOps::moveFeatureVertex(vl, got.id(), at, 20.0, 10.0, &err)` | Corner `(10,10)` -> `(20.0, 10.0)` | `true`; `std::abs(ext.xMaximum() - 20.0) < 1e-6`. |
| `TestWorkflow::vertexEdit_snapsAndOffersAddDeleteOnLine` | `tests/test_workflow.cpp:1462-1499` | Line/Polygon survey layer + `KaVertexEditTool`. | Right-click on line segment / vertex + snap check | Segment & vertex positions | Offers `"점추가"` on segment and `"점삭제"` on vertex; respects snap. |
| `TestWorkflow::topologicalVertexMove_movesSharedVertexOnBothFeatures` | `tests/test_workflow.cpp:3179-3251` | Two adjacent survey polygons sharing a vertex, `topologicalEditing() == true`. | Drag shared vertex via select/vertex tool | Shared corner -> moved point | Both features update shared vertex in 1 undo step. |

### D. Rust today
- **A1**: `MISSING` — Selecting a shape (`crates/strata-app/src/gui_tab_map_canvas.rs:232-279`) stores `selected_shape = Some((table, fid))` for outline tinting only; no vertex box markers (`QgsVertexMarker`) or `KaVertexEditTool` exist.
- **A2**: `DIFFERENT` — Layer context menu (`crates/strata-app/src/layer_menu_model.rs:219-221`) shows `"꼭짓점 수정"` on non-Point editable layers, **but** unconditionally disables it with `"아직 Rust 판에서 연결하지 않았습니다 (지시 CM2a-2)"`.
- **A3**: `MISSING` — No `"점삭제"` / `"점추가"` canvas right-click menu or edge double-click vertex insertion exists (`crates/strata-app/src/gui_tab_map_context.rs:106-152`).
- **B1**: `MISSING` — No vertex grab (`10 px`), click-to-highlight active vertex, or rubber-band drag preview exists in `crates/strata-app/src/gui_tab_map_canvas.rs:101-120`.
- **B2**: `MISSING` — No `moveVertexTo` / `applyVertexMove` / `ringClosingVertex` edit command exists in `crates/strata-app/` or `crates/strata-storage/`.
- **B3**: `FAKE` — The draw subtoolbar (`crates/strata-app/src/gui_ribbon_draw_row.rs:225,231-234`) renders an interactive `"공유 경계"` checkbox with tooltip `"켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집"` that toggles `self.home_map.cad.snap_topological` and triggers `"자석 켜짐..."` status text (`:281-285`), **but** `snap_topological` (`crates/strata-canvas2d/src/cad_session.rs:20`) is read **nowhere** in the workspace.
- **B4**: `MISSING` — No `segmentNear` or `insertVertexAt` (`"꼭짓점 추가"`) exists in `crates/`.
- **B5**: `MISSING` — `crates/strata-geo/src/snap_vertex_ops.rs:29-54` contains unwired helper functions `delete_polygon_vertex_checked` (`HGIS_MIN_POLYGON_VERTICES = 4`) and `delete_line_vertex_checked` (`HGIS_MIN_LINE_VERTICES = 2`), **but** they are called nowhere in `strata-app`, operate on a single flat `Vec<[f64; 2]>` without hole-ring awareness, and emit no status messages.
- **B6**: `MISSING` — No per-layer vertex-edit CRS transform / pixel-tolerance scaling exists.
- **B7**: `MISSING` — No vertex-edit post-commit marker refresh or undo fallback exists.

---

## 5. 「가까운 선에 붙이기」

### A. C++ offered
- **A1. Draw subtoolbar `▾` (`btnSnapAssist`) popup action**:
  - Trigger button: `QToolButton` `btnSnapAssist` (`Qt::DownArrow`, inserted right before `쉽게그리기` / right after `KaSnapSettingsWidget` on `m_subToolbar`), tooltip `"자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기"` (`src/app/KaDrawSketchTools.cpp:51-59`).
  - Menu action: `QAction` `actSnapToLines`, text `"가까운 선에 붙이기"` (`src/app/KaDrawSketchTools.cpp:60-61,95`).
  - Tooltip (`src/app/KaDrawSketchTools.cpp:62-64`):
    `"지적도·사업자 도면 위에 그린 도형(방금 그린 도형 또는 선택한 도형)을\n가장 가까운 지적선·도면선에 자동으로 붙이고 빠진 사이 꺾임점도 채웁니다 (Ctrl+Z 되돌리기 가능)."`
  - Enabled/hidden: always enabled inside `btnSnapAssist`'s popup whenever the `그리기` subtoolbar is visible.
- **A2. Companion toggles in the same `btnSnapAssist` menu**:
  - `"선 따라 자동채움"` (`actSnapTrace`, checkable, default `true`, `src/app/KaDrawSketchTools.cpp:37-38,86-90`): traces intermediate parcel/reference vertices between consecutive sketch clicks (`KaCaptureMapTool::setSnapTrace`), **and when toggled ON (`isTrace && runSnapToLines()`) immediately snaps any selected/just-drawn shape to nearest lines** (`:82`); otherwise shows `"선 따라 자동채움 켜짐 — 자석점을 한두 칸 건너뛰어 찍어도 사이 꼭짓점을 자동으로 채웁니다."` or `"선 따라 자동채움 꺼짐"` (`:83-90`).
  - `"근접 자석점 표시"` (`actSnapPreview`, checkable, default `true`, `:91-94`): previews up to `12` nearby snap vertices within `48.0` px of cursor (`SnapTraceOps::nearbyVertices`, `src/core/SnapTraceOps.h:20-21`).
- **A3. Companion `KaSnapSettingsWidget` on `m_subToolbar`**:
  - Controls `자석` (`m_snapEnabled`), `공유 경계` (`topologicalEditing`), `허용치` (`4`–`64` px, default `12` px), and scope (`모든 조사 레이어` / `현재 레이어`) (`src/app/MainWindowRibbon.cpp:866-877`), updating `QgsSnappingConfig` via `applySnapConfig()` and showing `"자석 켜짐 — 조사 도형·지적 선에 붙습니다. 위성 그림은 제외"` / `"자석 꺼짐"` (`:872-875`).

### B. C++ does
1. **B1. Target shape resolution (`SnapTraceOps::snapCanvasShapesToNearestLines`)**:
   - Chain: `actSnapToLines` -> `runSnapToLines` (`src/app/KaDrawSketchTools.cpp:42-49`) -> `SnapTraceOps::snapCanvasShapesToNearestLines(m_canvas, m_lastDrawnLayer, m_lastDrawnFid, &msg)` (`src/core/SnapTraceOps.cpp:224-283`).
   - Target search order (`:228-263`):
     1. Every selected feature across all valid survey vector layers (`!layerKeyOf(vl).isEmpty() && !isReferenceLayer(vl) && !isCadastralLayer(vl)`).
     2. If empty, `(m_lastDrawnLayer, m_lastDrawnFid)` if valid and `hasGeometry()` (selecting it via `fallbackLayer->selectByIds({fallbackFid})`).
     3. If still empty, scans `LayerOps::findAllByLayerKey(proj, key)` in order `{"survey_area", "feature_poly", "feature_line", "trial_trench"}` and picks the **last** feature with geometry in the first non-empty layer (selecting it).
     4. If still empty, sets `*statusOut = "먼저 지적도·도면 위에 도형을 그리거나 도형선택으로 도형을 고르세요."` and returns `0` (`:260-263`).
2. **B2. Reference/snap-source layer collection (`eligibleLayers` + `toCanvasAndFetch`)**:
   - `SnapTraceOps::detail::eligibleLayers(canvas)` (`src/core/SnapTraceGraph.h:32-53`) collects visible `LayerOps::isSnapSourceLayer(vl)` vector layers, ordering **cadastral and reference vector layers first** (`cadRef`), followed by survey vector layers (`survey`).
   - `toCanvasAndFetch` (`:56-101`, `src/core/SnapTraceOps.cpp:133-140`) transforms `searchBox = bbox.grown(snapTol * 1.5)` to each source layer's CRS, fetches spatially intersecting features **excluding `(targetLayer, ignoreFid)`** (`:80`), and transforms their geometries into `canvasCrs`.
3. **B3. `BoundaryGraph` construction & T-junction linking**:
   - Constants (`src/core/SnapTraceOps.cpp:130-135`, `src/core/SnapTraceGraph.h:141,190-204`):
     - `mupp = canvas->mapUnitsPerPixel() > 0 ? mapUnitsPerPixel() : 0.08`
     - `mergeTol = std::max(mupp * 0.3, 0.01)` (`mergeSq = mergeTol * mergeTol`)
     - `BoundaryGraph::kMaxNodes = 900`
   - Adds segments whose either endpoint lies in `searchBox`, deduplicating nodes within `mergeTol` via spatial `PointGrid`, then calls `linkTJunctions()` (`:189-204`), splitting any segment `(u, v)` where another node `w` projects within `mergeSq * 4.0` (`2 * mergeTol`).
4. **B4. Vertex snapping + Dijkstra shortest-path intermediate vertex filling (`processRing`)**:
   - Constants (`src/core/SnapTraceOps.cpp:129-132,157`, `src/core/SnapTraceGraph.h:212-297`):
     - `diag = std::hypot(bbox.width(), bbox.height())`
     - `snapTol = std::clamp(std::max(mupp * 48.0, diag * 0.24), 2.5, 25.0)` (metres in metric work CRS)
     - `vTol = snapTol * 0.78`
     - `skipSq = std::pow(mergeTol * 2.5, 2)`
     - `maxMids = 24` per edge
   - Step 1 (`BoundaryGraph::snapPoint`, `src/core/SnapTraceGraph.h:212-236`): finds nearest node `<= vTol` (`bestND`) and nearest segment projection `< snapTol` (`bestSD`). Prefers vertex node when `bestND <= std::max(bestSD * 2.2, vTol * 0.45)`, else segment projection; counts `moved` iff `res.sqrDist(target) > mergeSq`.
   - Step 2 (`BoundaryGraph::trace`, `:264-297`): attaches consecutive snapped vertices `s` and `t` to graph nodes/segments and runs Dijkstra within corridor `projectOnSegment.sqrDist <= std::max(snapTol * 1.5, chord * 0.45)²`, accepting path iff `dist[t] <= chord * 2.1` and `mids.size() <= 24` (filtering endpoints within `skipSq`).
   - Step 3 (`src/core/SnapTraceOps.cpp:162-177`): removes consecutive duplicates (`<= mergeSq`) and immediate A-B-A spikes (`clean[size-2].sqrDist(pt) <= mergeSq`), re-closes polygon rings (`>= 4` pts) or keeps lines (`>= 2` pts).
5. **B5. Geometry reconstruction & CRS back-transform**:
   - Rebuilds outer ring `poly[0] = processRing(poly[0], true)` (preserving interior rings `poly[1..]` of part 0) or polyline `processRing(line, false)`, then transforms from `canvasCrs` back to `targetLayer->crs()` if `needXf` (`src/core/SnapTraceOps.cpp:179-200`).
6. **B6. Edit-buffer commit, undo step, selection & status text**:
   - `snapFeatureToNearestLines` (`src/core/SnapTraceOps.cpp:203-222`): if `(moved == 0 && inserted == 0) || snapped.isEmpty()`, returns `false` (no edit command). Otherwise starts editing if needed, wraps `targetLayer->changeGeometry(targetFid, snapped)` in `beginEditCommand("가까운 선에 붙이기")` / `endEditCommand()` (1 undo step per changed layer), and triggers repaint.
   - `runSnapToLines` refreshes vertex handles (`m_featureSelectTool->refreshSelectedGeometry()`) and canvas (`src/app/KaDrawSketchTools.cpp:45-47`), keeping target feature(s) selected.
   - Status messages (`src/core/SnapTraceOps.cpp:273-281`, shown for 6000 ms):
     - `changedCount > 0`: `"도형을 가장 가까운 선에 붙였습니다 (꼭짓점 %1개 맞춤 · 사이점 %2개 채움). Ctrl+Z로 되돌릴 수 있습니다."`
     - `changedCount == 0`: `"이미 가장 가까운 선에 붙어 있거나 주변에 붙일 기준 선(지적·도면)이 없습니다."`

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `ParcelCopyKeysTest::snapTraceShowsNearbyVerticesOnlyWhenCloseAndFillsSkippedVertex` | `tests/test_parcel_copy_keys.cpp:122-196` | 640x480 EPSG:5187 canvas `(199980,449980,200060,450040)` (`mupp = 0.125`); reference `"사업자도면"` polyline `{(200000,450000),(200010,450001.2),(200020,450000)}`; `"survey_area"` rough polygon `{(200001.2,449999.1),(200019.0,450000.8),(200020.8,450019.2),(199999.3,450020.6),close}`. | `canvasMoveEvent` far/near, `canvasPressEvent` x2, `snapCanvasShapesToNearestLines`, source check | `farMove = px(200050,450035)`, `nearMove = px(200010,450002)`, `p1 = px(200000,450000)`, `p2 = px(200020,450000)` | `previewMarkerCount() == 0` (far); `previewMarkerCount() >= 1` (near); `tool.pointCount() == 3` (auto-fills `(200010,450001.2)`); `snapCanvasShapesToNearestLines == 1`; `snappedPoly[0].size() == 6`; `KaDrawSketchTools.cpp` contains `"가까운 선에 붙이기"`, `"선 따라 자동채움"`, `"근접 자석점 표시"`. |
| `ParcelCopyKeysTest::snapTraceMouseMoveStaysFastOverDenseCadastral` | `tests/test_parcel_copy_keys.cpp:201-251` | 1600x1000 EPSG:5187 canvas; dense 160x160 cadastral grid (25,600 6-vertex parcels, `cell = 12.0`, spatial index). | `nearbyVertices` & `traceBetween` x10 per `mupp` | `mupp` in `{8.0, 2.0, 0.5, 0.1}` around `mid(200960, 450960)` | `perMoveMs <= 70` ms when `mupp > 4.0` (`8.0`), and `perMoveMs <= 20` ms when `mupp <= 4.0` (`2.0, 0.5, 0.1`). |
| `TestEditingTools::easyDrawTakesCornersAtOnceAndEdgesAfterRest` | `tests/test_editing_tools.cpp:159-197` | Line `{(200010,450030),(200090,450030)}`, `QgsSnappingConfig` `12` px `Vertex|Segment`, `KaCaptureMapTool` `easyDraw=true`. | Click, sweep hovers, wait `700 + 300` ms, hover corner | `(200050,450060)`, `(200040,450030)`, `(200050,450030)`, `(200090,450030)` | `pointCount()` = `1` -> `1` (sweep) -> `2` (after dwell) -> `3` (immediate at corner). |
| `TestShellChrome::statusBar_snapChipTextAndTone` | `tests/test_shell_chrome.cpp:51-68` | MainWindow status bar snap chip. | Toggle snap on/off | `true` / `false` | Status chip reflects snap state & px tolerance. |

### D. Rust today
- **A1**: `SAME` — Subtoolbar `▾` button (`crates/strata-app/src/gui_ribbon_draw_row.rs:288-294`) and popup item `"가까운 선에 붙이기"` (`crates/strata-app/src/gui_ribbon_snap_assist.rs:14-35`) match C++ text and tooltips verbatim and call `self.snap_shapes_to_nearest_lines()`.
- **A2**: `FAKE` — `"선 따라 자동채움"` and `"근접 자석점 표시"` (`crates/strata-app/src/gui_ribbon_snap_assist.rs:36-76`) toggle `self.home_map.cad.snap_trace_enabled` / `snap_preview_enabled` (and toggling `"선 따라 자동채움"` ON does run `snap_shapes_to_nearest_lines()` at `:50`), **but** survey sketch capture (`crates/strata-app/src/gui_tab_map_capture.rs:19-53`) never reads `self.home_map.cad`, never previews nearby snap points, and never traces intermediate vertices between clicks while drawing.
- **A3**: `FAKE` — `paint_snap_settings` (`crates/strata-app/src/gui_ribbon_draw_row.rs:223-286`) renders `"자석"`, `"허용치"` (`4..=64 px`), and `"모든 조사 레이어"` / `"현재 레이어"` and emits `"자석 켜짐 — 조사 도형·지적 선에 붙습니다. 위성 그림은 제외"` / `"자석 꺼짐"`, **but** survey drawing (`crates/strata-app/src/gui_tab_map_capture.rs:19-53` -> `crates/strata-app/src/app_state_draw_commit.rs:80-103`) pushes raw screen-to-world points `s2w(...)` without consulting `self.home_map.cad` at all.
- **B1**: `DIFFERENT` — `snap_targets` (`crates/strata-app/src/app_state_snap_lines.rs:53-74`, `:122-127`) checks `selected_shape`, then `last_drawn_shape` (`:60-63`), then the last feature in `["survey_area", "feature_poly", "feature_line", "trial_trench"]`, selecting the fallback and emitting the exact empty-target message, **but** snaps at most **1** feature (`selected_shape: Option<(String, i64)>`, no multi-selection loop across survey layers).
- **B2**: `DIFFERENT` — `snap_one` (`crates/strata-app/src/app_state_snap_lines.rs:86-88`) reads `self.home_map.real_survey.cadastral_parcels` and `heritage_polys` (which remain **empty** in normal runs because downloaded 지적도 and imported reference vectors are stored in `self.ref_vectors`, `crates/strata-app/src/app_state.rs:115`) plus `survey_snap_lines` (`:29-50`)! Thus in practice `"가까운 선에 붙이기"` **never sees downloaded 지적도 or reference vector layers**, and `survey_snap_lines` performs no spatial bbox pre-filter before transforming every visible survey feature.
- **B3**: `SAME` — `cad_ring_snap.rs` (`crates/strata-canvas2d/src/cad_ring_snap.rs:38,41-59`) and `BoundaryGraph` (`crates/strata-canvas2d/src/cad_boundary_graph.rs:21,115-137`) use `MAX_NODES = 900`, `merge_tol = (eff_mupp * 0.3).max(0.01)`, `grow = snap_tol * 1.5`, and `link_t_junctions()`.
- **B4**: `SAME` — `snap_ring_to_nearest_lines` (`crates/strata-canvas2d/src/cad_ring_snap.rs:10-113`) and `BoundaryGraph::snap_point` / `trace` (`crates/strata-canvas2d/src/cad_snap_trace.rs:37-196`) use identical tolerances (`(eff_mupp * 48.0).max(diag * 0.24).clamp(2.5, 25.0)`, `v_tol = snap_tol * 0.78`, `skip_sq = (merge_tol * 2.5)²`, `max_mids = 24`) and identical duplicate/spike cleanup.
- **B5**: `DIFFERENT` — `snap_one` (`crates/strata-app/src/app_state_snap_lines.rs:22-26,81-111`) transforms between table CRS and work CRS via PROJ `Transformer` and preserves interior holes of part 0 on `Polygon`, **but** operates only on `in_work.first()` (`:91,108-111`), dropping parts `1..` of `MultiPolygon` / `MultiLineString`.
- **B6**: `SAME` — Commits via `LayerEditBuffer::change_geometry` + `UndoStep::Table` (`crates/strata-app/src/app_state_snap_lines.rs:114-141`), sets `selected_shape` on fallback (`:61,69`), and formats both C++ status messages verbatim.


---

## 6. 폴리곤 묶기

### A. C++ offered
- **A1. Draw subtoolbar button (`subToolbar`, `m_subToolsMode == "draw"`)**:
  - Control: `QAction` `mergeAct` right after the separator following `"연속 그리기"` (`src/app/MainWindowRibbon.cpp:917-923`).
  - Text: `"폴리곤 묶기"` (`src/app/MainWindowRibbon.cpp:918`).
  - Tooltip (`src/app/MainWindowRibbon.cpp:920-923`):
    `"여러 면을 하나로 묶습니다.\n쓰는 법: [도형선택]을 누르고 Shift를 누른 채 지도에서 면을 두 개 이상 고른 뒤\n[폴리곤 묶기]를 누르면 고른 면들이 하나로 합쳐집니다."`
  - Enabled/hidden: always visible and enabled whenever the `그리기` subtoolbar row is open (`src/app/MainWindowRibbon.cpp:945`); connects directly to `&MainWindow::mergeFeaturePolygons` (`:919`).
- **A2. Layer tree context menu `"선택한 면 묶기"` (`layer.merge`)**:
  - Control: `QAction` `"선택한 면 묶기"` on Polygon vector layers (`src/app/MainWindowContextMenus.cpp:419-421`, `:270-275`; see also the comment in `src/app/MainWindow.cpp:2061` explaining why multi-part `survey_area` shapes exist in older surveys).
  - Enabled/hidden: offered only when `editableKind && vector->geometryType() == Polygon` (`src/app/MainWindowContextMenus.cpp:419`). Enabled iff `mergeReason.isEmpty()` (`!busy && ownSurvey && valid && vector->selectedFeatureCount() >= 2 && editUnavailable(vector, ChangeGeometries | DeleteFeatures | AddFeatures).isEmpty()`, `:239-247`); when `< 2` polygons are selected on that layer, disabled with tooltip `"지도에서 [도형선택]으로 면을 2개 이상 고른 뒤 실행하세요."` (`:243-244`).
  - Action (`:272-274`): clears selection on every other vector layer so only `vector`'s selection remains, then calls `mergeFeaturePolygons()`.

### B. C++ does
1. **B1. Selection gathering, cross-layer warning, `< 2` guard & read-only layer refusal**:
   - Chain: `MainWindow::mergeFeaturePolygons` (`src/app/MainWindowExport.cpp:442-510`) collects `KaFeatureSelectTool::allSelectedFeatures(m_canvas)` (`:444`), sets `targetLayer = selected[0].layer.data()`, inserts FIDs on `targetLayer` into `selectedIds`, and collects names of any other selected layers into `otherLayers` (`:448-456`).
   - Condition 1 (`!otherLayers.isEmpty()`, `:459-465`): warns via `KaUserError::warn` (title `"폴리곤 묶기"`, heading `"한 번에 한 레이어의 면만 묶을 수 있습니다"`, detail `"「%1」의 면만 묶고 다음 레이어의 선택은 쓰지 않습니다: %2"`) and continues with `targetLayer`.
   - Condition 2 (`selectedIds.size() < 2`, `:468-474`): shows `QMessageBox::information` (title `"폴리곤 묶기"`, body `"묶을 면을 2개 이상 고른 뒤 누르세요.\n[도형선택]으로 면을 클릭하고, Shift를 누른 채 다른 면을 더 고릅니다."`) and returns (never merges the whole layer).
   - Condition 3 (`refuseReadOnlyLayer("폴리곤 묶기", targetLayer)`, `src/app/MainWindowExport.cpp:481`, `src/app/MainWindowEditing.cpp:1178-1186`): if `LayerOps::isReferenceOrBasemapLayer(targetLayer)`, shows `QMessageBox::information` `"「%1」은(는) 보기만 하는 참고 자료(참조 지도·지적)라 고칠 수 없습니다.\n조사 도형만 골라 다시 누르세요."` and returns.
2. **B2. Keeper record & attribute conflict confirmation (`GeometryEditOps`)**:
   - Keeper rule (`GeometryEditOps::mergeKeeperId`, `src/core/GeometryEditMerge.cpp:26-42`): sorts `ids` in drawing order — saved features (`id >= 0`) ascending first, then unsaved features (`id < 0`, `-1, -2, ...`) descending (`a > b`, so `-1` comes before `-2`) — and keeps the **first** feature's attributes.
   - `GeometryEditOps::mergeConflicts(targetLayer, selectedIds)` (`:44-78`) compares all non-PK, non-`"area_m2"` fields across rows in drawing order; any field with a non-empty value differing from the keeper row produces `{aliasOrName, values}`.
   - If `!conflicts.isEmpty()` (`src/app/MainWindowExport.cpp:484-489`), prompts `QMessageBox::question` (title `"폴리곤 묶기"`, `Yes | No`, default **`QMessageBox::No`**) with `mergeConflictSummary(conflicts) + "\n\n묶을까요?"` (`src/core/GeometryEditMerge.cpp:80-97`, `maxFields = 6`: header `"고른 도형의 속성이 서로 다릅니다. 가장 먼저 그린 도형의 값만 남습니다."`, bullets `"· %1: %2"` with up to 4 values + `"…"`, overflow `"· 그 밖에 %1개 항목"`, footer `"묶은 뒤에도 Ctrl+Z로 되돌릴 수 있습니다."`). Canceling returns with no edit.
3. **B3. Geometry union & single-part GPKG guard (`LayerOps::mergePolygonFeatures`)**:
   - Chain: `LayerOps::mergePolygonFeatures(targetLayer, selectedIds, &err)` (`src/core/GeometryEditMerge.cpp:99-152`).
   - Guards (`:104-125`): invalid layer -> `"올바른 레이어가 아닙니다."`; non-Polygon -> `"폴리곤 레이어만 묶을 수 있습니다"`; `featureIds.size() < 2` -> `"묶을 폴리곤을 2개 이상 고르세요 (선택: %1개)"`; repairs invalid inputs via `makeValid()`, `geoms.size() < 2` -> `"묶을 폴리곤이 2개 이상 필요합니다 (선택: %1개)"`.
   - Geometry rule (`:126-136`): runs GEOS `QgsGeometry::unaryUnion(geoms)`; if empty or `!isGeosValid()`, falls back to `QgsGeometry::collectGeometry(geoms)`; if empty -> `"폴리곤 결합 실패"`; if `!isGeosValid()`, calls `makeValid()`.
   - Single-`POLYGON` GPKG rule (`CLAUDE.md:38`, `:130-136`): if `!QgsWkbTypes::isMultiType(layer->wkbType()) && merged.isMultipart()`:
     - If `merged.constGet()->partCount() > 1` (disjoint polygons), **refuses** with `"떨어진 구역은 하나로 묶을 수 없습니다. 두 구역은 따로 저장되고 제출에도 함께 들어갑니다. 맞닿거나 겹친 구역만 하나로 묶습니다."`
     - Else (`partCount() == 1`), calls `merged.convertToSingleType()`.
   - Copies `mergeKeeperId(ids)` attributes and clears all `primaryKeys(layer)` to `QVariant()` (`:141-147`).
   - On failure (`src/app/MainWindowExport.cpp:492-495`), `KaUserError::warn` shows `"폴리곤 묶기"` / `"면을 묶지 못했습니다"` / `err`.
4. **B4. Edit-buffer undo step**:
   - `LayerOps::runEditCommand(layer, "폴리곤 묶기", [&]() { return layer->deleteFeatures(ids) && layer->addFeature(out); }, errorOut)` (`src/core/GeometryEditMerge.cpp:149-151`): **1** undo step labeled `"폴리곤 묶기"` on `targetLayer->undoStack()`, uncommitted until `저장`.
5. **B5. Post-merge selection & dirty state**:
   - `deleteFeatures(ids)` removes the old selected IDs; the newly added merged feature is **not** selected (`selectedFeatureCount() == 0`). Calls `QgsProject::instance()->setDirty(true)` and `m_canvas->refresh()` (`src/app/MainWindowExport.cpp:497-498`).
6. **B6. Status text & toast notification**:
   - Status (`src/app/MainWindowExport.cpp:499-503`, 10000 ms): `"「%1」의 폴리곤 %2개를 1개로 묶었습니다. Ctrl+Z로 되돌릴 수 있습니다."`
   - Toast (`:504-506`): `Notice::Success`, title `"폴리곤 묶기 완료"`, body `"고른 면을 하나로 합쳤습니다."`, detail `"「검수·제출」의 제출 꾸러미에서 feature_poly.shp 한 파일(EPSG:5179)로 나갑니다."`

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `LayerEditsTest::mergeNeverTakesTheWholeLayer` | `tests/test_layer_edits.cpp:42-52` | `"feature_poly"` EPSG:5187 with 3 squares at `x = 0, 20, 40`. | `LayerOps::mergePolygonFeatures(layer.get(), {}, &error)` and `{first}` | `{}` and `{first}` | Both return `false`; `!error.isEmpty()`; `layer->featureCount() == 3`. |
| `LayerEditsTest::mergeWarnsAboutLostAttributesAndCanBeUndone` | `tests/test_layer_edits.cpp:54-93` | `"feature_poly"` EPSG:5187 with 2 saved touching squares (`first`: `"1호 주거지"`, `"청동기"` at `0..10`; `second`: `"2호 수혈"`, `"삼국"` at `10..20`), plus unsaved `a` (`-1`), `b` (`-2`) check. | `mergeKeeperId`, `mergeConflicts`, `mergeConflictSummary`, `mergePolygonFeatures` -> `undo()` | `{first, second}`; `{a, b}` (`a < 0 && b < 0`) | `mergeKeeperId({first, second}) == first`; `mergeKeeperId({a, b}) == a`; `conflicts.size() >= 2`; `summary` contains `"1호 주거지"`, `"2호 수혈"`, `"Ctrl+Z"`; after merge: `isEditable()`, `featureCount() == 1`, `dataProvider()->featureCount() == 2` (file untouched), `area == 200.0` (`<1e-6`), `feat_no == "1호 주거지"`; after `undo()`: `featureCount() == 2`. |
| `LayerEditsTest::mergeRefusesSeparatePolygonsInASinglePartLayer` | `tests/test_layer_edits.cpp:97-106` | Single-part `"survey_area"` EPSG:5187 with disjoint squares `0..10` and `100..110`. | `LayerOps::mergePolygonFeatures(layer.get(), {east, west}, &error)` | `{east, west}` | Returns `false`; `error.contains("떨어진")`; `layer->featureCount() == 2`. |
| `TestSaveOpen::polygonCommands_leaveReferenceAndCadastralShapesAlone` | `tests/test_save_open.cpp:3922-3982` | Survey + reference polygon (`refA`, `refB`) + cadastral polygon (`cadA`, `cadB`). | Select `{refA, refB}` / `{cadA, cadB}` -> `mergeFeaturePolygons()` | 2 reference or 2 cadastral polygons | Refused by `refuseReadOnlyLayer` (`QMessageBox` auto-closed); `ref->featureCount() == 2 && !ref->isEditable()`; `cad->featureCount() == 2 && !cad->isEditable()`. |
| `TestWorkflow::drawSubToolbarWiresEachDomainSlot` | `tests/test_workflow.cpp:5959-6014` | Source inspection of `MainWindow::showSubToolsDraw`. | Checks toolbar slot order | `"mergeFeaturePolygons"`, `"startSplitPolygonTool"`, `"clipOverlappingLayers"`, `"&MainWindow::hideSubTools"` | No `subToolbarSpacer`; order is `startEditArtifact < mergeFeaturePolygons < startSplitPolygonTool < clipOverlappingLayers < hideSubTools`. |

### D. Rust today
- **A1**: `FAKE` — `paint_draw_edit_buttons` (`crates/strata-app/src/gui_ribbon_draw_row.rs:208,213-216`) draws `"폴리곤 묶기"` as a normal clickable `Look::Plain` button with the verbatim tooltip, **but discards the click response** (`let _ = text_button(...);` — comment at `:214`: `진짜 묶기·나누기는 지시 4`).
- **A2**: `DIFFERENT` — Layer tree context menu (`crates/strata-app/src/layer_menu_model.rs:223-226`) shows `"선택한 면 묶기"` on Polygon survey layers, **but** unconditionally disables it with `"아직 Rust 판에서 연결하지 않았습니다 (지시 CM2a-2)"`.
- **B1**: `MISSING` — No `mergeFeaturePolygons` handler, multi-selection, cross-layer warning, `< 2` dialog, or read-only layer refusal exists in `crates/strata-app/`.
- **B2**: `MISSING` — No `mergeKeeperId`, `mergeConflicts`, `mergeConflictSummary`, or `"묶을까요?"` confirmation exists in `crates/`.
- **B3**: `MISSING` — `crates/strata-geo/src/boolean_split.rs:41-43` has a 2-argument `union_polygons(a, b)` helper, **but** it is called nowhere in `strata-app` or `strata-storage`, and there is no `mergePolygonFeatures` or single-part disjoint refusal.
- **B4**: `MISSING` — No `"폴리곤 묶기"` edit-buffer command exists.
- **B5**: `MISSING` — No post-merge selection update or dirty marking exists.
- **B6**: `MISSING` — Neither the status message nor the `"폴리곤 묶기 완료"` toast exists.

---

## 7. 폴리곤 나누기

### A. C++ offered
- **A1. Draw subtoolbar button (`subToolbar`, `m_subToolsMode == "draw"`)**:
  - Control: `QAction` `splitAct` immediately right of `"폴리곤 묶기"` (`src/app/MainWindowRibbon.cpp:924-929`). (No ribbon shortcut and no layer-tree menu item.)
  - Text: `"폴리곤 나누기"` (`src/app/MainWindowRibbon.cpp:924`).
  - Tooltip (`src/app/MainWindowRibbon.cpp:926-929`):
    `"[폴리곤 묶기]로 하나가 된 면을 다시 나눕니다.\n쓰는 법: 면을 가로지르는 선을 클릭해 그리고 우클릭하면 그 선을 따라 갈라집니다.\n(Shift로 도형 두 개를 고른 뒤 눌러도 겹친 구간에서 나뉩니다.)"`
  - Enabled/hidden: always visible and enabled when the `그리기` subtoolbar is open; connects to `&MainWindow::startSplitPolygonTool` (`:925`).

### B. C++ does
1. **B1. Read-only layer guard**:
   - `MainWindow::startSplitPolygonTool` (`src/app/MainWindowEditing.cpp:1193-1280`) first checks every item in `KaFeatureSelectTool::allSelectedFeatures(m_canvas)` (`:1195-1197`) and later `cur` (`:1263`) with `refuseReadOnlyLayer("폴리곤 나누기", layer)` (`:1178-1186`), refusing reference/cadastral layers with `"「%1」은(는) 보기만 하는 참고 자료(참조 지도·지적)라 고칠 수 없습니다.\n조사 도형만 골라 다시 누르세요."`.
2. **B2. Branch 1 — `selected.size() == 2`: overlap intersection extraction (`splitTwoOverlappingFeatures`)**:
   - Chain (`src/app/MainWindowEditing.cpp:1200-1232`): calls `LayerOps::splitTwoOverlappingFeatures(l1, fid1, l2, fid2, &createdFid, &targetLayer, &err)` (`src/core/LayerOps.cpp:876-1019`).
   - Geometry rule (`src/core/LayerOps.cpp:899-988`): transforms `g2` into `layer1->crs()` if needed (`setBallparkTransformsAreAppropriate(true)`), repairs invalid geometries with `makeValid()`, checks `g1.intersects(g2)` (else `"선택한 두 도형이 서로 겹치지 않습니다."`), and computes `interGeom = g1.intersection(g2)` (if empty -> `"두 도형의 교차 영역을 계산할 수 없습니다."`). Although `diff1` and `diff2` are computed (`:934-951`), lines `:973-975` explicitly leave `f1` and `f2` untouched (`Q_UNUSED(diff1); Q_UNUSED(diff2);`) and add **only** `interGeom` as a new feature on `targetLayer` (`layer2` if `layerKeyOf(layer1) == "survey_area"` or `layer1->name().contains("바운더리")`, else `layer1`, `:953-959`), copying `targetFeat.attributes()`.
   - On failure (`src/app/MainWindowEditing.cpp:1209-1215`): `KaUserError::warn` `"폴리곤 나누기"` / `"두 도형의 겹친 구간을 나누지 못했습니다."` / `err` / `"겹치는 도형 두 개를 Shift+클릭으로 다시 고른 뒤 누르세요."`.
   - Status (`:1227-1230`, 8000 ms): `"선택한 두 도형의 겹치는 구간을 잘라 분할했습니다! (A, B, 중첩부 3개로 분할됨)"` + `Notice::Success` `"폴리곤 중첩 분할 완료"`.
3. **B3. Branch 2 — `selected.size() >= 1` and `selected[0]` is `MultiPolygon` (`explodeMultipartFeatures`)**:
   - Chain (`src/app/MainWindowEditing.cpp:1234-1256`): collects selected FIDs on `l = selected[0].layer` and calls `LayerOps::explodeMultipartFeatures(l, fids, &err)` (`src/core/LayerOps.cpp:599-674`). (If `explodeMultipartFeatures` returns `false`, execution falls through to Branch 3.)
   - Geometry rule (`:622-665`): decomposes each multipart feature via `g.asGeometryCollection()`; if `parts.size() > 1`, deletes `f.id()` and adds each non-empty `part` with `f.attributes()`.
   - Status (`src/app/MainWindowEditing.cpp:1248-1250`, 8000 ms): `"그룹으로 묶여 있던 폴리곤을 개별 폴리곤들로 분리했습니다!"` + `Notice::Success` `"폴리곤 그룹 분리 완료"`.
4. **B4. Branch 3 — Entering interactive split-line capture mode**:
   - Chain (`src/app/MainWindowEditing.cpp:1258-1277`): sets `cur = selected[0].layer` if non-empty, else `m_layerTree->currentLayer()`.
   - If `!cur || !cur->isValid() || cur->geometryType() != Polygon` (`:1264-1267`): shows `QMessageBox::information("폴리곤 나누기", "도형을 Shift+클릭으로 선택하거나, 나눌 폴리곤 레이어를 좌측 목록에서 선택해 주세요.")` and returns.
   - Calls `beginEdit(cur)`, sets `m_isSplittingPolygon = true` and `m_captureTool->setMode(KaCaptureMapTool::Mode::Line)`, updates tool chip to `"나누기 선"` (`src/app/KaDrawSketchTools.cpp:180-181`), and shows `"폴리곤 나누기 모드 — 폴리곤을 가로지르는 선을 클릭하여 그리고 우클릭으로 분할 (또는 Shift로 2개 도형 선택 후 실행)."` (12000 ms, `src/app/MainWindowEditing.cpp:1271-1276`).
5. **B5. Branch 3 — Completing the split line (`onGeometryCaptured` -> `splitPolygonWithLine`)**:
   - Chain (`src/app/MainWindowEditing.cpp:794-816`): when `m_isSplittingPolygon` is true on right-click / `Enter` / double-click finish, extracts polyline vertices into `QVector<QgsPointXY> pts` and calls `LayerOps::splitPolygonWithLine(m_editLayer, pts, &err)` (`src/core/LayerOps.cpp:800-874`).
   - Guards (`:803-820`): invalid layer -> `"올바른 레이어가 아닙니다."`; non-Polygon -> `"폴리곤 레이어만 나눌 수 있습니다."`; `splitLine.size() < 2` -> `"분할선은 최소 2개 이상의 점이어야 합니다."`; empty polyline -> `"유효한 분할선이 아닙니다."`.
   - Geometry rule (`:832-864`): iterates features on `layer` — restricted to `layer->selectedFeatureIds()` if non-empty, else **all** intersecting features on `layer` — and calls QGIS/GEOS `g.splitGeometry(&splitCurve, newGeometries, false, false, topologyTestPoints, true)`. Updates `f.id()` to `g` via `layer->changeGeometry(f.id(), g)` and adds each `newG` with `f.attributes()` via `layer->addFeature(newF)`. If `splitCount == 0`, fails with `"분할선이 관통하는 폴리곤이 없습니다. 폴리곤 양쪽 경계를 완전히 가로질러야 합니다."`
   - On success (`src/app/MainWindowEditing.cpp:803-807`): refreshes canvas and shows `"폴리곤을 성공적으로 나누었습니다."` (5000 ms) while staying in split-line capture mode (`m_isSplittingPolygon` remains `true` until tool exit). On failure (`:809-814`): `KaUserError::warn` `"폴리곤 나누기"` / `"선으로 면을 나누지 못했습니다."` / `err` / `"면 바깥에서 시작해 반대쪽 바깥까지 가로지른 뒤 우클릭하세요."`.
6. **B6. Exiting split-line mode**:
   - `MainWindow::stopCaptureTool()` (`src/app/MainWindowEditing.cpp:274-285`) resets `m_isSplittingPolygon = false` whenever the user presses `A` (with 0 sketch points), clicks `도형선택` / `닫기`, or starts another tool (`tests/test_save_open.cpp:3854-3918`).
7. **B7. Undo & selection behavior across the 3 branches**:
   - Selection on existing features is preserved as-is; newly added split pieces are not auto-selected.
   - Undo (`src/core/LayerOps.cpp:648-670,822-870,961-1000`): none of the 3 `LayerOps` split helpers wraps edits in `runEditCommand`:
     - If the target layer was **not** already `isEditable()` (`startedHere == true`), `explodeMultipartFeatures` and `splitPolygonWithLine` call `layer->commitChanges()` immediately (`:666,:866`), leaving **0** `QUndoStack` commands; `splitTwoOverlappingFeatures` also calls `commitChanges()`, while `MainWindow::startSplitPolygonTool` (`src/app/MainWindowEditing.cpp:1217-1225`) pushes `KaUndoAction::FeatureAdded` (`"중첩 분할 도형 생성"`) when `createdFid >= 0`.
     - In Branch 3 via UI, `beginEdit(cur)` starts editing beforehand (`startedHere == false`), so `changeGeometry` and each `addFeature` push separate ungrouped commands onto `cur->undoStack()`.

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestBuffer::splitPolygonWithLine_splitsGeometry` | `tests/test_buffer.cpp:280-314` | Memory Polygon EPSG:5187 `rect(0,0,10,10)`. | `LayerOps::splitPolygonWithLine` with non-crossing line then crossing line | Non-crossing `{(-5,-5),(-1,-1)}`; crossing `{(5,-2),(5,12)}` | Non-crossing returns `false`, `!err.isEmpty()`; crossing returns `true`, `vl->featureCount() == 2`, total area `std::abs(totalArea - 100.0) < 0.1`. |
| `TestBuffer::splitTwoOverlappingFeatures_splitsTargetPolygon` | `tests/test_buffer.cpp:316-376` | `"survey_area"` `rect(0,0,20,20)` (`f1`) + `"feature_poly"` `rect(10,0,30,20)` (`f2`), EPSG:5187; plus non-overlapping `rect(100,100,110,110)` (`f3`). | `LayerOps::splitTwoOverlappingFeatures` on `(f1, f2)` and `(f1, f3)` | `(bLayer, f1.id(), tLayer, f2.id())`; `(bLayer, f1.id(), farLayer, f3.id())` | Overlapping returns `true`, `outLayer == tLayer`, `createdFid >= 0`, `bLayer->featureCount() == 1` (`area == 400.0`), `tLayer->featureCount() == 2` (`hasOriginal` `400.0` + `hasInter` `200.0` within `0.5`); non-overlapping returns `false`, `!err2.isEmpty()`. |
| `TestSaveOpen::selectTool_rightClickErasesAndKeyAEndsSplitMode` | `tests/test_save_open.cpp:3854-3918` | Opened survey on `survey_area`; `startSplitPolygonTool()` called. | Press `Qt::Key_A` -> draw polygon on `survey_area` | `Qt::Key_A`, 4-corner polygon sketch | Before `Key_A`: `mapTool() == capture`; after `Key_A`: `mapTool() == select`, `m_actSelect->isChecked()`; subsequent polygon sketch adds feature (`featureCount() == before + 1`) instead of triggering `"분할선"` error (`!m_isSplittingPolygon`). |
| `TestSaveOpen::polygonCommands_leaveReferenceAndCadastralShapesAlone` | `tests/test_save_open.cpp:3922-3982` | Survey + 2 reference polygons + 2 cadastral polygons. | Select `{refA}` or `{refA, refB}` -> `startSplitPolygonTool()` | Reference / cadastral selections | Refused; `mapTool() != capture`; `ref->featureCount() == 2 && !ref->isEditable()`; `cad->featureCount() == 2 && !cad->isEditable()`. |

### D. Rust today
- **A1**: `FAKE` — `paint_draw_edit_buttons` (`crates/strata-app/src/gui_ribbon_draw_row.rs:209,213-216`) draws `"폴리곤 나누기"` as a clickable `Look::Plain` button with the verbatim tooltip, **but discards the click response** (`let _ = text_button(...);`).
- **B1**: `MISSING` — No `startSplitPolygonTool` or `refuseReadOnlyLayer` exists in `crates/strata-app/`.
- **B2**: `MISSING` — No 2-feature overlap intersection split (`splitTwoOverlappingFeatures`) exists in `crates/`.
- **B3**: `MISSING` — No multipart polygon explosion (`explodeMultipartFeatures`) exists in `crates/`.
- **B4**: `MISSING` — No `m_isSplittingPolygon` state or `"나누기 선"` capture mode exists in `crates/strata-app/src/app_state.rs` or `crates/strata-app/src/draw_sketch.rs`.
- **B5**: `MISSING` — `crates/strata-geo/src/boolean_split.rs:46-90` (`split_polygon_by_line`, 2-point 2 mm blade difference) and `crates/strata-geo/src/snap_vertex_ops.rs:57-81` (`split_convex_ring_by_segment`, 2-point convex-ring cutter) exist as standalone library helpers, **but** neither supports multi-vertex polylines and neither is called anywhere in `strata-app` or `strata-storage`.
- **B6**: `MISSING` — No split-line mode flag exists to clear on tool exit.
- **B7**: `MISSING` — No split undo commands exist.

---

## 8. 구간 분리

### A. C++ offered
- **A1. Draw subtoolbar button (`subToolbar`, `m_subToolsMode == "draw"`)**:
  - Control: `QAction` `clipAct` between `"폴리곤 나누기"` and `"겹친 곳 지우기"` (`src/app/MainWindowRibbon.cpp:930-933`). (No shortcut and no context-menu item.)
  - Text: `"구간 분리"` (`src/app/MainWindowRibbon.cpp:930`).
  - Tooltip (`src/app/MainWindowRibbon.cpp:932-933`):
    `"겹쳐 있는 두 레이어에서 겹치는 구간만 새 레이어로 떼어 냅니다. 원본은 그대로 남습니다."`
  - Enabled/hidden: always visible and enabled when the `그리기` subtoolbar is open; connects to `&MainWindow::clipOverlappingLayers` (`:931`).

### B. C++ does
1. **B1. Layer count & polygon-boundary pre-check**:
   - `MainWindow::clipOverlappingLayers` (`src/app/MainWindowEditing.cpp:1282-1390`) collects all valid `QgsVectorLayer*` in `QgsProject::instance()->mapLayers()` into `allVecs` and all Polygon vector layers into `polyVecs` (`:1287-1297`).
   - If `allVecs.size() < 2 || polyVecs.isEmpty()` (`:1299-1303`), shows `QMessageBox::information("구간 분리 (클립)", "구간을 분리하려면 최소 1개의 폴리곤(바운더리) 레이어와 대상 레이어가 필요합니다.")` and returns.
2. **B2. Modal layer-pair dialog & same-layer refusal**:
   - Opens modal `QDialog` (`src/app/MainWindowEditing.cpp:1305-1348`): window title `"겹치는 구간 분리 (클립)"`, size `420 x 200`, label `"기준 바운더리 레이어와 겹치는 구간만 잘라내어 새 레이어로 분리합니다."`, form rows `"자를 대상 레이어:"` (`targetCombo`, defaulting to `m_layerTree->currentLayer()`) and `"기준 바운더리:"` (`boundaryCombo`, defaulting to the last `polyVecs` layer whose lowercased name contains `"바운더리"`, `"구역"`, or `"허가"`, else `0`), buttons `"분리 실행"` / `"취소"`.
   - If `target == boundary` (`:1354-1362`), warns via `KaUserError::warn` (`"구간 분리"` / `"같은 레이어로는 구간을 나눌 수 없습니다."` / `"자를 대상과 기준 바운더리에 같은 레이어를 골랐습니다."` / `"기준 바운더리에 다른 면 레이어를 고른 뒤 다시 실행하세요."`) and returns.
3. **B3. Geometry clip & memory layer creation (`LayerOps::clipLayerByBoundary`)**:
   - Chain (`src/core/LayerOps.cpp:676-798`):
     1. Transforms every valid boundary feature geometry into `sourceLayer->crs()` (`setBallparkTransformsAreAppropriate(true)`), repairs invalid geometries via `makeValid()`, and unions them via `QgsGeometry::unaryUnion(bGeoms)` (fallback `collectGeometry` -> `makeValid()`; empty -> `"바운더리 레이어에 유효한 지오메트리가 없습니다."` / `"바운더리 결합 지오메트리 생성에 실패했습니다."`, `:694-739`).
     2. Creates a QGIS `"memory"` vector layer named `"[클립] %1"` (`sourceLayer->name()`) with `uri = "%1?crs=%2"` (`displayString(sourceLayer->wkbType())`, `sourceLayer->crs().authid()`) and copies `sourceLayer->fields()` (`:742-754`).
     3. For each feature on `sourceLayer` intersecting `boundaryUnion`, computes `clippedGeom = sg.intersection(boundaryUnion)` (`makeValid()` if needed) and copies `sf.attributes()` (`:756-778`). If `outFeatures.isEmpty()`, deletes `clippedLayer` and fails with `"바운더리와 겹치는 구간(피처)이 없습니다."` (`:780-784`).
     4. Adds `outFeatures` to `clippedLayer->dataProvider()`, clones `sourceLayer->renderer()`, tags `markSurveyLayer(clippedLayer, QStringLiteral("clip_%1").arg(sourceLayer->id()))`, adds to `project`, and places in `kGroupSurveyData` (`"조사 데이터"`, `:786-796`), leaving both `sourceLayer` and `boundaryLayer` unmodified and selection unchanged.
   - On failure (`src/app/MainWindowEditing.cpp:1366-1374`): `KaUserError::warn` `"구간 분리"` / `"겹치는 구간을 새 레이어로 떼어 내지 못했습니다."` / `err` / `"두 레이어가 실제로 겹치는지 지도에서 확인한 뒤 다시 실행하세요."`.
4. **B4. Undo step**:
   - Pushes `KaUndoAction{type = LayerAdded, layerId = clipped->id(), description = "구간 분리 레이어 생성"}` via `pushUndoAction` (`src/app/MainWindowEditing.cpp:1377-1381`). `Ctrl+Z` removes `clipped` from the project (`src/app/MainWindowUndo.cpp:394-402`); not redoable by `Ctrl+Y`.
5. **B5. Status text & toast notification**:
   - Status (`src/app/MainWindowEditing.cpp:1383`, 8000 ms): `"겹치는 구간을 분리하여 「%1」 레이어를 생성했습니다. (Ctrl+Z로 되돌리기 가능)"`
   - Toast (`:1384-1386`): `Notice::Success`, title `"구간 분리 완료"`, body `"「%1」 레이어가 성공적으로 생성되었습니다. (Ctrl+Z로 되돌리기 가능)"`, detail `"기준: %1 | 대상: %2"`.

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestBuffer::clipLayerByBoundary_clipsIntersectingFeatures` | `tests/test_buffer.cpp:241-278` | `src` Polygon EPSG:5187 (`"대상레이어"`): `f1` `rect(0,0,20,10)` (`"1호"`), `f2` `rect(100,100,110,110)` (`"2호"`); `bnd` Polygon EPSG:5187 (`"조사구역"`): `bf` `rect(10,0,30,10)`. | `LayerOps::clipLayerByBoundary(src, bnd, QgsProject::instance(), &err)` | `(src, bnd, project)` | `clipped != nullptr && clipped->isValid()`; `clipped->featureCount() == 1`; `clipped->name().contains("대상레이어")`; clipped `area == 100.0` (`std::abs(area - 100.0) < 0.1`); `src->featureCount() == 2` (source untouched). |
| `TestWorkflow::drawSubToolbarWiresEachDomainSlot` | `tests/test_workflow.cpp:5959-6014` | Source inspection of `MainWindow::showSubToolsDraw`. | Checks toolbar slot order | `"clipOverlappingLayers"` | Wired between `"startSplitPolygonTool"` and `"&MainWindow::hideSubTools"`. |

### D. Rust today
- **A1**: `FAKE` — `paint_draw_edit_buttons` (`crates/strata-app/src/gui_ribbon_draw_row.rs:210,213-216`) draws `"구간 분리"` as a clickable `Look::Plain` button with the verbatim tooltip, **but discards the click response** (`let _ = text_button(...);`).
- **B1**: `MISSING` — No `clipOverlappingLayers` pre-check exists in `crates/strata-app/`.
- **B2**: `MISSING` — No `"겹치는 구간 분리 (클립)"` modal dialog exists in `crates/strata-app/`.
- **B3**: `MISSING` — No `clipLayerByBoundary` or `[클립] %1` layer creation exists in `crates/`.
- **B4**: `MISSING` — No `UndoStep::LayerAdded` exists in `crates/strata-app/src/app_state_layer_remove.rs:13-18`.
- **B5**: `MISSING` — Neither the status message nor the `"구간 분리 완료"` toast exists.


---

## 9. 겹친 곳 지우기

### A. C++ offered
- **A1. Draw subtoolbar button (`subToolbar`, `m_subToolsMode == "draw"`)**:
  - Control: `QAction` `eraseAct` (`objectName = "actEraseOverlap"`) between `"구간 분리"` and `"닫기"` (`src/app/MainWindowRibbon.cpp:934-941`).
  - Text: `"겹친 곳 지우기"` (`src/app/MainWindowRibbon.cpp:934`).
  - Tooltip (`src/app/MainWindowRibbon.cpp:937-941`):
    `"위에 그린 도형 모양대로 아래 도형에서 그 자리만 지웁니다(클립).\n쓰는 법: 같은 레이어의 도형 위에 겹쳐 그리고 우클릭으로 마치면 선택창에서 고를 수 있습니다.\n이미 그린 도형은 [도형선택](A)으로 고른 뒤 이 단추를 누릅니다.\n아래 도형에 구멍이 나고 위에 그린 도형은 없어집니다. Ctrl+Z로 되돌립니다."`
  - Enabled/hidden: always visible and enabled when the `그리기` subtoolbar is open (verified at `1920` px width in `tests/test_save_open.cpp:3516-3537`); connects to `&MainWindow::eraseOverlapWithShape` (`:935`).
- **A2. `Select` tool right-click canvas menu (`actSelectEraseOverlap`)**:
  - Built in `KaFeatureSelectTool::handleContextMenu` (`src/app/KaFeatureSelectTool.cpp:369-392,400-402`):
    - Shown when no vertex (`점삭제`/`점추가`) or inner-piece (`안쪽 도형 지우기`) action applies and `anyPolygon && (plan.ready() || all.size() >= 2)` (`:377,385`).
    - Text: `"겹친 곳 지우기"` (`objectName = "actSelectEraseOverlap"`, `:387-388`).
    - Enabled iff `plan.ready()`; when `!plan.ready()`, disabled with tooltip `plan.hint` (`:389-391`). Triggering emits `requestEraseOverlap()` -> `MainWindow::eraseOverlapWithShape` (`:401`, `src/app/MainWindowRibbon.cpp:309`).
- **A3. Post-sketch popup menu (`drawnShapeEraseMenu`) on same-layer polygon overlap**:
  - Called at the end of polygon capture in `MainWindow::onGeometryCaptured` (`src/app/MainWindowEditing.cpp:844-849`) -> `MainWindow::offerEraseWithDrawnShape(layer, feat.id(), [&] { drawStep.close(); })` (`src/app/MainWindowErase.cpp:124-148`).
  - Shown **only** when `PolygonErase::plan({drawn}, project).ready()` **and** every `target.layer == layer` (drawing inside a polygon of *another* layer, such as a 유구 inside 조사구역, never prompts, `:131-135`).
  - Popup `QMenu` (`objectName = "drawnShapeEraseMenu"`, `:136-144`) at `QCursor::pos()` with two actions:
    1. `"겹친 곳 지우기"` (`objectName = "actEraseDrawnOverlap"`)
    2. `"그대로 두기"` (`objectName = "actKeepDrawnShape"`; dismissing the menu acts identically to `"그대로 두기"`, `:145`).

### B. C++ does
1. **B1. Empty-selection first-press preview of `m_lastDrawnFid`**:
   - In `MainWindow::eraseOverlapWithShape` (`src/app/MainWindowErase.cpp:23-75`): if `selected.isEmpty()` and `(m_lastDrawnLayer, m_lastDrawnFid)` is an editable valid polygon (`:44-53`), selects `m_lastDrawnLayer->selectByIds({m_lastDrawnFid})`, refreshes handles and canvas, shows status + `Notice::Info` `"방금 그린 도형을 골랐습니다. 이 모양대로 아래 도형에서 지우려면 [겹친 곳 지우기]를 한 번 더 누르세요."` (10000 ms), and **returns without erasing** (the 2nd press performs the erase).
2. **B2. Erase planning & all 7 refusal hints (`PolygonErase::plan`)**:
   - Chain: `PolygonErase::plan(selected, project)` (`src/core/PolygonErase.cpp:162-260`).
   - Filters `selected` to `PolygonErase::editable(layer)` (`!readOnly`, `Polygon`, not cadastral/reference/basemap, provider has `ChangeGeometries | DeleteFeatures`, `:133-143`) with valid cleaned polygon geometry (`isGeosValid() ? geom : makeValid()`, `:22-25`).
   - Overlap predicate (`overlapWithArea`, `:53-55`): `a.intersects(b) && !a.touches(b)` (shared boundary alone is not overlap).
   - Hint conditions in exact order (`:178-257`, shown via `statusBar()->showMessage(text, 10000)` + `notify(Notice::Info, "겹친 곳 지우기", text)` at `src/app/MainWindowErase.cpp:28-31,56-59`):
     1. `picked.isEmpty() && selected.isEmpty()`: `"위에 그린 도형을 [도형선택]으로 고른 뒤 누르세요."`
     2. `picked.isEmpty() && !selected.isEmpty()`: `"고른 도형으로는 지울 수 없습니다. 조사 데이터의 면 도형을 고르세요."`
     3. `picked.size() == 1`: searches same layer first (`shapesUnder`), and **only if same layer has 0 overlapping targets and `!covered`** searches other visible editable layers (`moved` via `QgsCoordinateTransform`, `setBallparkTransformsAreAppropriate(true)`, `:35-50,198-214`).
        - Cross-layer `remaining.size() > 1`: `"아래에 겹친 도형이 %1개입니다. Shift를 누른 채 지울 도형도 함께 고른 뒤 다시 누르세요."`
        - `remaining.isEmpty() && covered`: `"고른 도형이 아래 도형을 모두 덮고 있어 남는 부분이 없습니다. 위에 그린 작은 도형을 고르세요."`
        - `remaining.isEmpty() && !covered`: `"고른 도형 아래에 겹친 면이 없습니다."`
     4. `picked.size() >= 2` (`:226-259`): chooses the largest-area shape (`MeasureOps::geometryAreaSquareMeters`) as `target` and every other overlapping shape as `cutters` (non-overlapping selections are ignored, neither cutting nor deleted, `:245`).
        - `out.cutters.isEmpty()`: `"고른 도형들이 서로 겹치지 않습니다."`
        - `rest.isNull()`: `"고른 작은 도형들이 큰 도형을 모두 덮어 남는 부분이 없습니다."`
3. **B3. Multi-cutter confirmation dialog (`plan.cutters.size() > 1`)**:
   - In `MainWindow::eraseOverlapWithShape` (`src/app/MainWindowErase.cpp:62-70`): asks `QMessageBox::question` (title `"겹친 곳 지우기"`, `Yes | No`, default **`QMessageBox::No`**) with `"고른 도형 %1개의 자리를 「%2」의 가장 큰 도형에서 지우고, 그 %1개 도형은 없앱니다.\nCtrl+Z로 되돌릴 수 있습니다. 계속할까요?"`.
4. **B4. Boolean difference, sliver filter & single-`POLYGON` part splitting (`PolygonErase::erased` / `partsFor` / `apply`)**:
   - `PolygonErase::erased(target, cutter)` (`src/core/PolygonErase.cpp:145-160`): cleans both geometries (`makeValid()`), computes `rest = from.difference(with)`, extracts polygon subclass if `GeometryCollection` (`convertGeometryCollectionToSubclass(Polygon)`), and rejects slivers when `rest.area() <= from.area() * 1e-9` (`:158`).
   - `partsFor(layer, rest)` (`:92-110`): if `QgsWkbTypes::isMultiType(layer->wkbType())`, calls `convertToMultiType()` and keeps 1 multipart feature; if single-part (`survey_area`, `feature_poly`), decomposes `asGeometryCollection()` into single `Polygon` parts sorted by `area()` descending so the largest piece updates `target.fid` (`edits[at].changes`) and each smaller piece `i >= 1` is added as a new feature with copied attributes and cleared PKs + cleared `"uid"` (`:288-308`, `CLAUDE.md:38`).
   - All cutters `plan.cutters` are scheduled for deletion (`edits[...].deletions.insert(cutter.fid)`, `:309-312`), pre-validated completely before any layer is mutated (`:273-316`).
5. **B5. Per-layer `"겹친 곳 지우기"` undo commands, atomic rollback & `KaLinkedEdit`**:
   - For each affected layer (`src/core/PolygonErase.cpp:318-342`), executes `LayerOps::runEditCommand(layer, "겹친 곳 지우기", ...)` applying `changeGeometry`, `addFeatures`, and `deleteFeature` in 1 command on that layer's `QUndoStack`. If any layer fails, rolls back already-applied layers in reverse order via `LayerOps::undoLayerEdits(*it)` (`:336-339`).
   - `MainWindow::runErasePlan` (`src/app/MainWindowErase.cpp:93-100`) calls `forgetStaleLinkedEdits()` beforehand and `rememberLinkedEdit(outcome.layers)` afterwards so cross-layer erases undo and redo together on `Ctrl+Z` / `Ctrl+Y`.
   - When invoked from `offerEraseWithDrawnShape`, `beforeErase` calls `drawStep.close()` first (`src/app/MainWindowEditing.cpp:848`, `src/app/MainWindowErase.cpp:146`), keeping `"도형 그리기"` and `"겹친 곳 지우기"` as **2** separate undo steps (and skipping `KaFeatureFormDialog` because `erased == true`, `src/app/MainWindowEditing.cpp:853`).
6. **B6. Post-erase selection, current layer & dirty state**:
   - Cutter deletion clears selection on the cutter(s); if `target` was also selected (2-shape selection), `target` remains selected. Sets `project->setDirty(true)`, `m_layerTree->setCurrentLayer(firstLayer)`, `m_featureSelectTool->refreshSelectedGeometry()`, refreshes canvas, and calls `updateUndoRedoActions()` (`src/app/MainWindowErase.cpp:102-107`).
7. **B7. Status text & toast notification**:
   - Built in `MainWindow::runErasePlan` (`src/app/MainWindowErase.cpp:109-116`, 10000 ms + `Notice::Success` `"겹친 곳 지우기"`):
     - Base: `"「%1」 도형 %2개에서 겹친 자리를 지웠습니다."`
     - If `outcome.erased == 1 && outcome.added == 0 && firstLayer`: appends `" 면적 %L1 ㎡ → %L2 ㎡."` (`before`, `after`, 2 decimals).
     - If `outcome.added > 0`: appends `" 떨어진 조각 %1개는 따로 도형이 되었습니다."`
     - Suffix: `" 위에 그린 도형은 없앴습니다. Ctrl+Z로 되돌립니다."`
   - On failure (`:95-98`): status `"지우지 못했습니다. " + outcome.error` (10000 ms) + `Notice::Warning` `"지우지 못했습니다."`.

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `PolygonEraseTest::erased_cutsOnlyTheOverlappingPlace` | `tests/test_polygon_erase.cpp:70-85` | `outer` `rect(0,0,100,100)`. | `PolygonErase::erased` with interior, edge notch, full cover, disjoint | `(40,40,60,60)`, `(80,40,120,60)`, `(-10,-10,110,110)`, `(200,0,300,100)` | Interior: `Polygon`, `area == 9600.0` (`<1e-6`), `holes == 1`; notch: `9600.0`, `holes == 0`; full cover: `isNull()`; disjoint: `10000.0`. |
| `PolygonEraseTest::oneSelectedShapeCutsTheShapeUnderItAndOneUndoRestoresBoth` | `tests/test_polygon_erase.cpp:87-123` | `"조사구역"` EPSG:5187: saved `outer` `(0,0,100,100)`, `far` `(300,0,400,100)`, unsaved `inner` `(40,40,60,60)` (`inner < 0`). | `plan({inner})` -> `apply` -> `undo()` | `{shape(layer, inner)}` | `plan.ready()`, `cutters == 1`, `targets == {outer}`; `ok`, `erased == 1`, `added == 0`, `layers == 1`, `area == 9600.0`, `holes == 1`, `!getFeature(inner).isValid()`, `featureCount() == 2`, `undoStack()->index() == undoIndex + 1`; after `undo()`, `area == 10000.0`, `inner` valid, `featureCount() == 3`. |
| `PolygonEraseTest::selectingTheBigShapeOrALonelyShapeOnlyExplains` | `tests/test_polygon_erase.cpp:125-145` | `"조사구역"` with `outer` `(0..100)`, `inner` `(40..60)`, `apart` `(300..400, 0..100)`. | `plan({outer})`, `plan({apart})`, `plan({})`, `apply(covering)` | `{outer}`, `{apart}`, `{}` | All `!ready()`; hints contain `"덮고"`, `"겹친 면이 없습니다"`, `"도형선택"`; `!apply(covering).ok`; `featureCount() == 3`, `area == 10000.0`. |
| `PolygonEraseTest::anotherLayerIsCutOnlyWhenExactlyOneShapeLiesUnder` | `tests/test_polygon_erase.cpp:147-168` | `"조사구역"` `(0..100)`, `"쉽게그리기"` cutter `(40..60)`, then `"유구면"` `(30..70)`. | `plan({cutter})` before & after adding 2nd underlying shape | `{shape(easy, cutter)}` | 1 under: `ready()`, `targets == {outer}` on `area`; 2 under: `!ready()`, `hint` contains `"2개"` and `"Shift"`, `area == 10000.0`. |
| `PolygonEraseTest::severalSelectedShapesCutTheLargestAcrossLayers` | `tests/test_polygon_erase.cpp:170-194` | `area` `outer` `(0..100)` + `second` `(60..80)`; `easy` `first` `(10..30)` + `apart` `(300..310,0..10)`. | `plan({first, outer, second, apart})` -> `apply` | 4 selected shapes across 2 layers | `targets == {outer}`, `cutters.size() == 2` (`apart` untouched); `layers.size() == 2`, `area == 9200.0`, `holes == 2`, `first` & `second` deleted, `apart` valid. |
| `PolygonEraseTest::referenceAndCadastralShapesAreNeverCutOrUsed` | `tests/test_polygon_erase.cpp:196-225` | Survey `area`, reference `heritage`, cadastral `parcels`. | `editable(...)`, `plan({cutter})`, `plan({heritage})` | Survey cutter over ref/cad; ref shape selected | `editable`: `true, false, false`; both `!ready()`; `picked.hint` contains `"지울 수 없습니다"`; counts unchanged. |
| `PolygonEraseTest::shapeCutInTwoStaysValidInASinglePolygonFile` | `tests/test_polygon_erase.cpp:227-280` | Real GPKG single-`POLYGON` `"survey_area"` EPSG:5187: `(190000,560000,190100,560100)` (`"바깥"`) cut by band `(190040,559990,190060,560110)`. | `plan` -> `apply` -> `commitChanges()` | `{shape(layer, band)}` | `erased == 1`, `added == 1`; after `commitChanges()`, `count == 2`, each `isGeosValid()`, single-part, `area == 4000.0` (`<1e-3`), `survey_name == "바깥"`, `total == 8000.0`. |
| `PolygonEraseTest::shapeCutInTwoStaysOneShapeInAMultiPolygonLayer` | `tests/test_polygon_erase.cpp:282-296` | Memory `"MultiPolygon"` layer `(0,0,100,100)` cut by `(40,-10,60,110)`. | `apply(plan({band}))` | `{shape(layer, band)}` | `added == 0`, `featureCount() == 1`, `isMultipart()`, `partCount() == 2`, `area == 8000.0`. |
| `TestSaveOpen::eraseOverlap_*` & `drawnShapeOnTopOffersEraseWhenFinished` & `selectTool_rightClickErasesAndKeyAEndsSplitMode` (5 UI tests) | `tests/test_save_open.cpp:3494-3803,3854-3918` | Full `MainWindow` surveys in EPSG:5187 (`outer` `10000` m², cutters `400` / `36` m²). | Toolbar `actEraseOverlap`, `Ctrl+Z`/`Ctrl+Y`, `saveNow`, multi-cutter `No`/`Yes`, 2-press `m_lastDrawnFid`, post-sketch `drawnShapeEraseMenu`, `Select` right-click `actSelectEraseOverlap` | See lines | Same-layer + cross-layer `Ctrl+Z`/`Ctrl+Y` (`9600 <-> 10000`); saved GPKG `9600.0`; `No` keeps `3` shapes, `Yes` leaves `1` (`9200.0`); 1st press selects (`count == 2`), 2nd erases (`9600.0`); cross-layer draw shows no menu, same-layer draw shows `{"겹친 곳 지우기", "그대로 두기"}` and skips form (`864.0`, `Ctrl+Z` -> `3` shapes `900.0`); `Select` right-click triggers `actSelectEraseOverlap`. |

### D. Rust today
- **A1**: `FAKE` — `paint_draw_edit_buttons` (`crates/strata-app/src/gui_ribbon_draw_row.rs:211,213-216`) draws `"겹친 곳 지우기"` as a clickable `Look::Plain` button with the verbatim tooltip, **but discards the click response** (`let _ = text_button(...);`).
- **A2**: `MISSING` — No `"겹친 곳 지우기"` item exists in the canvas right-click menu (`crates/strata-app/src/gui_tab_map_context.rs:106-152`).
- **A3**: `MISSING` — `sketch_finish` (`crates/strata-app/src/app_state_draw_commit.rs:4,200-211`) records `self.home_map.last_drawn_shape = Some((key.clone(), fid))` for polygons, **but** explicitly notes `겹친 곳 지우기 제안(지시 4)·고친 면 알림 창은 아직 없다` (`:4`) and never offers `drawnShapeEraseMenu`.
- **B1**: `MISSING` — `last_drawn_shape` is stored (`crates/strata-app/src/app_state_draw_commit.rs:202`), **but** no `eraseOverlapWithShape` 1st-press preview handler exists.
- **B2**: `MISSING` — No `PolygonErase::plan` or any of the 7 refusal hints exists in `crates/`.
- **B3**: `MISSING` — No multi-cutter `"계속할까요?"` confirmation dialog exists.
- **B4**: `MISSING` — `crates/strata-geo/src/boolean_split.rs:37-39` (`erase_polygon`) and `crates/strata-storage/src/polygon_pieces.rs:109-134` (`erase_polygon_area_difference`, with `1e-9` sliver filter and descending area sort) exist as standalone library helpers, **but** neither is called anywhere in `strata-app` or `survey_gpkg_apply.rs`, and single-`POLYGON` multi-part feature splitting with cleared PKs/`uid` + cutter deletion is unwired.
- **B5**: `MISSING` — No `"겹친 곳 지우기"` edit command, multi-layer rollback, `KaLinkedEdit`, or `drawStep.close()` split exists.
- **B6**: `MISSING` — No post-erase selection/layer/dirty update exists.
- **B7**: `MISSING` — No `"「%1」 도형 %2개에서 겹친 자리를 지웠습니다..."` status or toast exists.

---

## 10. 연속 그리기 and 닫기 (of the Same Toolbar Row)

### A. C++ offered
- **A1. Draw subtoolbar `"연속 그리기"` toggle (`kaSketch = "continuous"`)**:
  - Control: checkable `QAction` `continuousAct` (`kaToggle = true`, oat background when checked) placed after sketch `완료`/`되돌리기`/`취소` and before the separator preceding `"폴리곤 묶기"` (`src/app/KaDrawSketchTools.cpp:115-124,160-163`).
  - Text: `"연속 그리기"` (`:115`).
  - Tooltip (`:120-123`):
    `"켜면 도형을 마칠 때마다 이름·번호 창을 띄우지 않고 바로 다음 도형을 그립니다.\n이름·번호는 나중에 도형을 우클릭해 「이 도형 기록 입력」으로 넣습니다.\n끄면 도형을 마친 뒤 창이 뜨고, Esc로 건너뛸 수 있습니다."`
  - Enabled/hidden: always visible and enabled whenever the `그리기` subtoolbar is open (`syncDrawSketchButtons` keeps `"continuous"` visible regardless of `sketching`, `:160-163`); `m_continuousDraw` defaults to `false` (`src/app/MainWindow.h:609`) and persists across opening/closing the subtoolbar within the session (`:162`).
- **A2. Draw subtoolbar `"닫기"` button**:
  - Control: `QAction` `closeAct` at the right end of the `그리기` subtoolbar row (`src/app/MainWindowRibbon.cpp:942-944`).
  - Text: `"닫기"` (`:942`).
  - Tooltip: `"그리기 도구 모음을 닫고 지도 이동으로 돌아갑니다"` (`:943`).
  - Enabled/hidden: always visible and enabled when the `그리기` subtoolbar is open.
- **A3. Companion sketch-only buttons (`완료`, `되돌리기`, `취소`) on the same row**:
  - Added in `MainWindow::addDrawSketchButtons` (`src/app/KaDrawSketchTools.cpp:96-114`):
    1. `"완료"` (icon `"check"`, `kaSketch = "finish"`, tooltip `"그리던 도형을 마칩니다. 우클릭·Enter·더블클릭과 같습니다."`)
    2. `"되돌리기"` (icon `"undo"`, `kaSketch = "undo"`, tooltip `"마지막 점 하나를 지웁니다. Ctrl+Z·Backspace와 같습니다."`)
    3. `"취소"` (icon `"stop"`, `kaSketch = "cancel"`, tooltip `"그리던 도형을 지웁니다. Esc와 같습니다. 이미 넣은 도형은 그대로입니다."`)
  - Enabled and visible **only** while `sketching` (`m_captureTool && m_canvas->mapTool() == m_captureTool && m_captureTool->hasSketch()`, `:144-145,167-168`).
- **A4. Responsive width fitting of the row (`KaDrawRowRules` / `subToolbarFitsAt1904WhileSketching`)**:
  - `KaDrawRowRules::apply(m_subToolbar)` (`src/app/KaDrawSketchTools.cpp:130`) adapts `QToolBar` button style (`ToolButtonTextBesideIcon` at 20 px -> `ToolButtonIconOnly` at 20 px while sketching at 1904 px -> `ToolButtonIconOnly` at 16 px on narrower windows) so **every** visible action — including `"겹친 곳 지우기"` and `"닫기"` — fits without overflow (`»`) at `1904` and `1920` px (`tests/test_save_open.cpp:1332-1419,3533-3537`).

### B. C++ does
1. **B1. Toggling `"연속 그리기"`**:
   - `connect(continuousAct, &QAction::toggled, ...)` (`src/app/KaDrawSketchTools.cpp:124-129`) sets `m_continuousDraw = on` (no layer mutation, 0 undo steps) and shows status (5000 ms):
     - `on == true`: `"연속 그리기 — 이름·번호 창 없이 이어서 그립니다."`
     - `on == false`: `"도형을 마칠 때마다 이름·번호 창을 띄웁니다(Esc로 건너뜀)."`
2. **B2. Effect of `m_continuousDraw` in `MainWindow::onGeometryCaptured`**:
   - After inserting the captured feature inside `KaUndoGroup drawStep(layer, "도형 그리기")` (`src/app/MainWindowEditing.cpp:828-837`), computes `offerForm = !erased && KaFeatureFormDialog::canOffer(layer)` (`:853`; `canOffer` is `false` for `"survey_area"`, `true` for feature/user layers with name/number fields).
   - When `offerForm && !m_continuousDraw` (`:854-871`), shows status `"도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요."` and opens modal `KaFeatureFormDialog`; when `m_continuousDraw == true`, **skips** `KaFeatureFormDialog` completely, sets the right-hand `m_featureCard` to the new feature without selecting it on the canvas (`:873`), and keeps `m_captureTool` active for the next shape (`:899-902`).
3. **B3. Post-capture status text & repair toast**:
   - At the end of `onGeometryCaptured` (`src/app/MainWindowEditing.cpp:904-919`), shows status (8000 ms):
     `"도형을 넣었습니다 (%1, %2개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다%3"`
     where `%3` is `" · 이름·번호는 나중에 속성에서"` when `offerForm && m_continuousDraw` (else `""`), followed on `"feature_poly"` by `missingAttributeCounterText()` (`" · 종류·시대 빈 유구 %1개(제출 전 「검수·제출」에서 확인)"`, `src/app/KaSubmitFlow.cpp:285`).
   - If `!repairNotice.isEmpty()` (polygon self-intersection/closure repair), fires a separate warning toast `notify(Notice::Warning, "면을 고쳐 넣었습니다", repairNotice)` (`:918-919`).
4. **B4. Clicking `"닫기"` (`MainWindow::hideSubTools`)**:
   - Chain: `closeAct` -> `MainWindow::hideSubTools()` (`src/app/MainWindowRibbon.cpp:769-781`) -> when `m_subToolsMode == "draw"`, calls `stopCaptureTool()` (`src/app/MainWindowEditing.cpp:274-285`: cancels any in-progress sketch, resets `m_isSplittingPolygon = false`, switches canvas from `m_captureTool` to `m_panTool`), hides `m_subToolbar`, clears `m_subToolsMode`, and unchecks `m_actDraw`. Emits **no** status message (`tests/test_workflow.cpp` / `crates/strata-app/src/app_state_draw_row.rs:63`).
5. **B5. Equivalence with ribbon `"그리기"` button and `Ctrl+D`**:
   - Ribbon `"그리기"` (`m_actDraw`, shortcut `Ctrl+D`, `src/app/MainWindowRibbon.cpp:346-361`) calls `hideSubTools()` when `m_subToolsMode == "draw"`, else `showSubToolsDraw()` (which opens the row in `Pan` mode without starting a tool and shows `"그릴 도구를 고르세요 — 조사구역·유구·유구 선·유물 가운데 하나를 누르면 그리기가 시작됩니다."`, `:947-948`).
6. **B6. Interaction with `Esc`**:
   - While `KaCaptureMapTool` is active (`src/app/KaCaptureMapTool.cpp:381-383`), `Esc` calls `cancel()` (clearing uncommitted sketch vertices while keeping `m_captureTool` and `m_subToolbar` open). When `m_panTool` or `m_featureSelectTool` is active, pressing `Esc` does **not** close `m_subToolbar` (only `"닫기"`, `"그리기"`, `Ctrl+D`, or switching to another subtoolbar closes it).

### C. C++ tests

| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `TestSaveOpen::subToolbarFitsAt1904WhileSketching` | `tests/test_save_open.cpp:1332-1419` | Opened survey in `MainWindow` resized to `1904 x 1000`; `btnDraw->click()`; `startEditFeaturePoly()`. | Idle check -> place 1 vertex at `(190050,560050)` -> resize stages -> `1024 x 768` | `1904x1000`, `(190050,560050)`, `needIcons+25`, `needSmall+5`, `1024x768` | Idle at 1904: `allShown() && !extShown()`, `toolButtonStyle() != IconOnly`, `iconSize() == 20x20`. With 1 vertex (`finish->isVisible()`): `allShown() && !extShown()`, `IconOnly` `20x20`; narrower -> `IconOnly` `16x16`; back to 1904 -> `IconOnly` `20x20`. |
| `TestSaveOpen::eraseOverlap_cutsTheShapeUnderTheSelectedOneAndCtrlZRestores` | `tests/test_save_open.cpp:3516-3537` | `MainWindow` resized to `1920 x 1040`, `btnDraw->click()`. | Inspects `subToolbar` widgets for `actEraseOverlap` and `"닫기"` | `window.resize(1920, 1040)` | Both `widgetForAction(erase)->isVisible()` and `widgetForAction(close)->isVisible()` are `true` (not folded into `»`). |
| `TestWorkflow::drawSubToolbarWiresEachDomainSlot` | `tests/test_workflow.cpp:5959-6014` | Source inspection of `MainWindow::showSubToolsDraw`. | Checks `"&MainWindow::hideSubTools"` wiring | `"&MainWindow::hideSubTools"` | `"닫기"` is wired after `clipOverlappingLayers` with no `subToolbarSpacer`. |

*(Note: No C++ test toggles `continuousAct` to `true` and asserts `!formSeen`; see `Could not determine` / test coverage note below.)*

### D. Rust today
- **A1**: `SAME` — `"연속 그리기"` button (`crates/strata-app/src/gui_ribbon_draw_row.rs:191-200`) uses `Look::Toggled` / `Look::Plain`, matches text and 3-line tooltip verbatim, stays visible regardless of `sketching`, and persists `self.ribbon.continuous_draw` across closing/reopening the row.
- **A2**: `SAME` — `"닫기"` button (`crates/strata-app/src/gui_ribbon_draw_row.rs:217-219`) matches text and tooltip `"그리기 도구 모음을 닫고 지도 이동으로 돌아갑니다"` verbatim and calls `self.hide_sub_tools()`.
- **A3**: `DIFFERENT` — Sketch-only `check` / `undo` / `stop` buttons (`crates/strata-app/src/gui_ribbon_draw_row.rs:169-190`) appear only while `sketching` with verbatim tooltips and matching actions, **but** are rendered as 55x55 icon-only buttons (`icon_button`) and never display the Korean text captions `"완료"`, `"되돌리기"`, `"취소"` even on wide screens.
- **A4**: `DIFFERENT` — `gui_ribbon_draw_row.rs` (`crates/strata-app/src/gui_ribbon_draw_row.rs:193,208-219`) places controls at hardcoded logical x-coordinates: when idle, `"겹친 곳 지우기"` spans `x = 1879..2051` and `"닫기"` spans `x = 2056..2135` (and `+ 180` px when `sketching`, reaching `2315` px!), **clipping `"겹친 곳 지우기"` and hiding `"닫기"` completely off-screen at `1904` and `1920` logical px** (violating `tests/test_save_open.cpp:1332-1419,3533-3537`).
- **B1**: `SAME` — Clicking `"연속 그리기"` (`crates/strata-app/src/gui_ribbon_draw_row.rs:193-200`) flips `self.ribbon.continuous_draw` and emits the exact `"연속 그리기 — 이름·번호 창 없이 이어서 그립니다."` / `"도형을 마칠 때마다 이름·번호 창을 띄웁니다(Esc로 건너뜀)."` status messages.
- **B2**: `SAME` — `after_capture` (`crates/strata-app/src/app_state_feature_form.rs:12-26`) skips `FeatureForm` when `self.ribbon.continuous_draw` is `true` (or `!can_offer`, e.g. `survey_area`), updates the right-hand card unselected (`set_card_target(..., false)`), and keeps the capture tool active.
- **B3**: `DIFFERENT` — `captured_status` (`crates/strata-app/src/app_state_draw_commit_b.rs:12-27`, `:87-103`) formats the exact C++ status string including `" · 이름·번호는 나중에 속성에서"` and `" · 종류·시대 빈 유구 {missing}개(제출 전 「검수·제출」에서 확인)"`, **but** appends `" · 면을 고쳐 넣었습니다: {notice}"` inline to the status bar instead of showing a warning toast (`MainWindowEditing.cpp:918-919`), and `save_feature_form` (`crates/strata-app/src/app_state_feature_form.rs:37-40`) writes form errors to the status bar instead of `KaUserError::warn`.
- **B4**: `SAME` — `hide_sub_tools` (`crates/strata-app/src/gui_ribbon_align_actions.rs:143-153`) calls `self.stop_capture_tool()`, sets `sub_toolbar_visible = false` and `sub_mode = SubToolsMode::None`, and leaves the status message unchanged (`crates/strata-app/src/app_state_draw_row.rs:59-63`).
- **B5**: `SAME` — `toggle_draw_sub_tools` (`crates/strata-app/src/app_state_draw_row.rs:18-27`) and `Ctrl+D` (`crates/strata-app/src/gui_shell.rs:139-141`) open the row in `Pan` mode with exact `DRAW_ROW_OPEN_TEXT` and close it via `hide_sub_tools()`.
- **B6**: `DIFFERENT` — While `capture_active()` is true, `Esc` (`crates/strata-app/src/gui_shell.rs:32-33`, `crates/strata-app/src/app_state_draw_commit.rs:115-119`) calls `sketch_cancel()` and keeps both the capture tool and subtoolbar open (`SAME`), **but** when `capture_active()` is false (e.g. right after opening the `그리기` row in `Pan` mode or while in `도형선택`), pressing `Esc` hits `gui_shell.rs:34-39`, which hides `self.ribbon.sub_toolbar_visible = false` (without resetting `sub_mode`) and forces `self.active_tool = MapToolKind::Pan`.


---

## All Differences (Ordered by Tool 1–10)

| tool | what | C++ file:line | Rust file:line or `none` | kind |
|---|---|---|---|---|
| 1 (`1.A1`) | Ribbon `"선택"` tooltip omits `(A 또는 Ctrl+1)` | `src/app/KaFeatureSelectPick.cpp:97-98` | `crates/strata-app/src/ribbon_icons_map.rs:49` | `DIFFERENT` |
| 1 (`1.A2`) | Subtoolbar `"도형선택"` does not toggle back to `Pan` or show activation status | `src/app/MainWindowRibbon.cpp:857-858`, `src/app/MainWindowEditing.cpp:224-238` | `crates/strata-app/src/gui_ribbon_draw_row.rs:137-142` | `DIFFERENT` |
| 1 (`1.A3`) | `Ctrl+1` does not toggle off; `A` lacks Korean-IME `VK_A` (`0x41`), steals `Measure`/`Align`, and drops active sketch | `src/app/KaFeatureSelectPick.cpp:104-126`, `src/app/MainWindow.cpp:2309-2315` | `crates/strata-app/src/gui_shell.rs:78-81,136-139`, `crates/strata-app/src/app_state_draw_row.rs:31-35` | `DIFFERENT` |
| 1 (`1.A4`) | Right-click context menu in `Select` (`점삭제`, `점추가`, `안쪽 도형 지우기`, `안쪽 도형만큼 구멍 내기`, `겹친 곳 지우기`) | `src/app/KaFeatureSelectTool.cpp:276-415` | `none` (`crates/strata-app/src/gui_tab_map_context.rs:106-152` opens generic map menu) | `MISSING` |
| 1 (`1.B1`) | Point pick skips reference/cadastral fallback tier and inner pieces | `src/core/FeaturePick.cpp:91-147` | `crates/strata-app/src/feature_pick.rs:58-103`, `crates/strata-app/src/gui_tab_map_canvas.rs:232-279` | `DIFFERENT` |
| 1 (`1.B2`) | Inner-piece (`PolygonPieces`) picking and red rubber-band highlight (`m_pieceBand`) | `src/core/PolygonPieces.cpp:40-96`, `src/app/KaFeatureSelectPick.cpp:36-55,183-185` | `none` | `MISSING` |
| 1 (`1.B3`) | Drag rubber-band marquee selection (`selectInRect`) | `src/app/KaFeatureSelectTool.cpp:209-225,417-484` | `none` (`crates/strata-app/src/gui_tab_map_canvas.rs:113-118` pans instead) | `MISSING` |
| 1 (`1.B4`) | `Shift` multi-selection across features and layers | `src/app/KaFeatureSelectPick.cpp:134-143,170-175`, `src/app/KaFeatureSelectTool.cpp:420-429` | `none` (`crates/strata-app/src/gui_tab_map_state.rs:14` stores single `Option<(String, i64)>`) | `MISSING` |
| 1 (`1.B5`) | `Ctrl` snap-to-other-shapes during vertex drag (`snapMapPointExcludingTarget`, `m_snapMark`) | `src/app/KaFeatureSelectTool.cpp:85-109`, `src/app/KaVertexEditTool.cpp:225-241` | `none` | `MISSING` |
| 1 (`1.B6`) | `Esc` exits `Select` to `Pan` in Rust instead of staying in `Select` and clearing active vertex/piece | `src/app/KaFeatureSelectTool.cpp:164-171` | `crates/strata-app/src/gui_shell.rs:34-39` | `DIFFERENT` |
| 1 (`1.B7`) | `Ctrl+C` / `Ctrl+V` cadastral parcel copy-paste while `Select` is active | `src/app/KaParcelCopyKeys.cpp:19-95` | `none` | `MISSING` |
| 1 (`1.B8`) | Tooltip & HUD promise draggable vertex handles (`"수정점이 나옵니다. 점을 끌어 옮깁니다"`) that do not exist | `src/app/MainWindowEditing.cpp:235-238`, `src/app/KaFeatureSelectPick.cpp:188-209` | `crates/strata-app/src/gui_tab_map_hud.rs:110-115`, `crates/strata-app/src/gui_tab_map_canvas.rs:256-277` | `FAKE` |
| 2 (`2.A2`) | `Backspace` not accepted as `Delete` synonym in `Select` / `Align` | `src/app/KaFeatureSelectTool.cpp:160`, `src/app/KaVertexEditTool.cpp:512`, `src/app/KaAlignMapTool.cpp:549` | `crates/strata-app/src/gui_shell.rs:82-91` | `DIFFERENT` |
| 2 (`2.A3`) | Legend group headers cannot be selected with left-click and empty groups cannot exist | `src/app/MainWindowContextMenus.cpp:199-213` | `crates/strata-app/src/gui_layer_panel_rows.rs:180-232` | `DIFFERENT` |
| 2 (`2.B1`) | `Delete` does not route to 2D/3D studio items or align-point list | `src/app/MainWindow.cpp:260-268` | `crates/strata-app/src/app_state_layer_remove.rs:106-120` | `DIFFERENT` |
| 2 (`2.B2`) | `Delete` on active vertex or active inner piece (`deleteActivePick`) before whole-shape deletion | `src/app/MainWindowUndo.cpp:497-499`, `src/app/KaFeatureSelectPick.cpp:63-88` | `none` (`crates/strata-app/src/app_state_layer_remove.rs:121-122` deletes whole shape) | `MISSING` |
| 2 (`2.B3`) | `Delete` / right-click on picked trench in `KaTrenchMoveTool` | `src/app/MainWindowUndo.cpp:501-503`, `src/core/TrenchLayerEdit.cpp:176` | `none` | `MISSING` |
| 2 (`2.B4`) | `deleteSelectedFeatures` deletes only 1 feature instead of multi-selected features grouped per layer | `src/app/MainWindowUndo.cpp:538-575` | `crates/strata-app/src/app_state_layer_remove.rs:130-141` | `DIFFERENT` |
| 2 (`2.B6`) | Legend `Delete` removes single row only, lacks `kMaxRemovedLayerUndo = 20` cap and `isVworldCadastralPicture` distinction | `src/app/MainWindowUndo.cpp:160-290`, `src/app/KaEditUndoOrder.cpp:23,61-71` | `crates/strata-app/src/app_state_layer_remove.rs:30-61,111-115` | `DIFFERENT` |
| 2 (`2.B7`) | Deleting selected/empty legend groups (`KaRemovedLayers::groups`, `"빈 묶음을 목록에서 지웠습니다..."`) | `src/app/MainWindowUndo.cpp:187-205,239-266,287-288` | `none` (`crates/strata-app/src/app_state_layer_remove.rs:22-26`) | `MISSING` |
| 3 (`3.A1`) | `Ctrl+Z` / `Ctrl+Y` ignores `Shift`/`Alt` (`Ctrl+Shift+Z` triggers Undo) and skips studio routing | `src/app/MainWindowRibbon.cpp:311-329`, `src/app/MainWindow.cpp:2294-2305` | `crates/strata-app/src/gui_shell.rs:126,152-157` | `DIFFERENT` |
| 3 (`3.A2`) | Floating map Undo/Redo pill never dims/disables when undo/redo stacks are empty | `src/app/MainWindowUndo.cpp:292-300` | `crates/strata-app/src/gui_tab_map_undo.rs:8-143` | `DIFFERENT` |
| 3 (`3.B1`) | `undoLastAction` / `redoLastAction` do not route to 2D Drawing Studio or 3D Terrain Studio | `src/app/MainWindowUndo.cpp:302-323` | `crates/strata-app/src/app_state_draw_commit_b.rs:31-74` | `DIFFERENT` |
| 3 (`3.B2`) | `undoMapAction` lacks `KaAlignMapTool::removeLastPair()` step | `src/app/MainWindowUndo.cpp:334-337` | `crates/strata-app/src/app_state_draw_commit_b.rs:31-35` | `DIFFERENT` |
| 3 (`3.B4`) | Draw + form attribute edit lacks `KaUndoGroup` macro class and command label `"도형 그리기"` | `src/app/KaUndoGroup.h:13-36`, `src/app/MainWindowEditing.cpp:828,860` | `crates/strata-app/src/app_state_draw_commit.rs:199-210`, `crates/strata-app/src/app_state_feature_form.rs:53-76` | `DIFFERENT` |
| 3 (`3.B5`) | Cross-layer linked undo/redo (`KaLinkedEdit` / `m_linkedEdits` / `m_linkedRedos`) | `src/app/MainWindowErase.cpp:160-244` | `none` (`crates/strata-app/src/app_state_layer_remove.rs:13-18`; `crates/strata-storage/src/polygon_pieces.rs:145-180` unwired) | `MISSING` |
| 3 (`3.B6`) | App-level undo lacks layer-checkbox visibility undo (`KaLayerTreeUndo`), `LayerAdded`, and `300`/`20` caps | `src/app/KaLayerTreeUndo.cpp:16,180-205`, `src/app/MainWindowUndo.cpp:391-402`, `src/app/KaEditUndoOrder.cpp:23-24,60-73` | `crates/strata-app/src/app_state_layer_remove.rs:13-18`, `crates/strata-app/src/app_state_draw_commit_b.rs:38-47` | `DIFFERENT` |
| 3 (`3.B7`) | Saving reloads empty `LayerEditBuffer` stacks without post-save fallback undo or clearing stale `UndoStep::Table` entries | `src/app/MainWindowUndo.cpp:86-156,403-453`, `src/core/EditHistory.cpp:65-73,98-106` | `crates/strata-app/src/app_state_projects.rs:232-234`, `crates/strata-app/src/app_state_draw_commit_b.rs:49-58` | `DIFFERENT` |
| 4 (`4.A1`) | Automatic `QgsVertexMarker` box handles and rubber-band outline on single selected survey shape | `src/app/KaFeatureSelectTool.cpp:111-119`, `src/app/KaVertexEditTool.cpp:60-69,158-197` | `none` (`crates/strata-app/src/gui_tab_map_canvas.rs:232-279`) | `MISSING` |
| 4 (`4.A2`) | Layer context menu `"꼭짓점 수정"` is unconditionally disabled (`CM2a-2`) | `src/app/MainWindowContextMenus.cpp:226-238,263-269,412-418` | `crates/strata-app/src/layer_menu_model.rs:219-221` | `DIFFERENT` |
| 4 (`4.A3`) | Canvas right-click `"점삭제"` / `"점추가"` and segment double-click vertex insert | `src/app/KaFeatureSelectTool.cpp:306-360,409-415`, `src/app/KaVertexEditTool.cpp:494-503` | `none` (`crates/strata-app/src/gui_tab_map_context.rs:106-152`) | `MISSING` |
| 4 (`4.B1`) | Vertex grab (`10 px`), click-to-activate handle, and rubber-band drag preview (`4 px` slop) | `src/app/KaFeatureSelectTool.cpp:177-206,241-246`, `src/app/KaVertexEditTool.cpp:75-86,243-260` | `none` | `MISSING` |
| 4 (`4.B2`) | `moveVertexTo` / `applyVertexMove` with exterior & hole `ringClosingVertex` closure (`"꼭짓점 이동"`) | `src/app/KaVertexEditTool.cpp:334-355`, `src/core/GeometryEditVertex.cpp:65-156` | `none` | `MISSING` |
| 4 (`4.B3`) | Subtoolbar `"공유 경계"` checkbox toggles `snap_topological` + status text, but `snap_topological` is never read | `src/app/KaVertexEditTool.cpp:343`, `src/core/GeometryEditVertex.cpp:20,114-133` | `crates/strata-app/src/gui_ribbon_draw_row.rs:225,231-234`, `crates/strata-canvas2d/src/cad_session.rs:20` | `FAKE` |
| 4 (`4.B4`) | `segmentNear` (`10 px`) and `insertVertexAt` (`"꼭짓점 추가"`) | `src/app/KaVertexEditTool.cpp:262-274,381-416` | `none` | `MISSING` |
| 4 (`4.B5`) | Per-ring `deleteVertexAt` (`minCount = 5` Polygon / `3` Line, `"꼭짓점 삭제"`) | `src/app/KaVertexEditTool.cpp:362-379`, `src/app/KaFeatureSelectTool.cpp:71-83` | `none` (`crates/strata-geo/src/snap_vertex_ops.rs:29-54` unwired & single-ring only) | `MISSING` |
| 4 (`4.B6`) | Map-to-layer CRS vertex transform and pixel-tolerance scaling (`toLayer`, `layerTolerance`) | `src/app/KaVertexEditTool.cpp:114-134` | `none` | `MISSING` |
| 4 (`4.B7`) | Post-vertex-edit marker refresh and post-save `FeatureChanged` undo fallback | `src/app/KaFeatureSelectTool.cpp:76,251,411`, `src/app/MainWindowEditing.cpp:210-221` | `none` | `MISSING` |
| 5 (`5.A2`) | `"선 따라 자동채움"` and `"근접 자석점 표시"` toggle flags/messages, but survey sketch capture never previews or traces | `src/app/KaDrawSketchTools.cpp:37-38,80-94`, `src/app/KaCaptureMapTool.cpp:60-120` | `crates/strata-app/src/gui_ribbon_snap_assist.rs:36-76`, `crates/strata-app/src/gui_tab_map_capture.rs:19-53` | `FAKE` |
| 5 (`5.A3`) | `"자석"` / `"허용치"` / scope controls toggle state/messages, but survey sketch capture pushes raw `s2w` points | `src/app/MainWindowRibbon.cpp:866-877` | `crates/strata-app/src/gui_ribbon_draw_row.rs:223-286`, `crates/strata-app/src/gui_tab_map_capture.rs:19-53` | `FAKE` |
| 5 (`5.B1`) | `snap_targets` snaps at most 1 feature (`Option<(String, i64)>`) instead of all multi-selected survey features | `src/core/SnapTraceOps.cpp:228-263` | `crates/strata-app/src/app_state_snap_lines.rs:53-74` | `DIFFERENT` |
| 5 (`5.B2`) | `snap_one` omits `self.ref_vectors` (downloaded 지적도 & imported reference vectors) and bbox pre-filtering | `src/core/SnapTraceGraph.h:32-101` | `crates/strata-app/src/app_state_snap_lines.rs:29-50,86-88`, `crates/strata-app/src/app_state.rs:115` | `DIFFERENT` |
| 5 (`5.B5`) | `snap_one` snaps only `in_work.first()`, dropping parts `1..` of `MultiPolygon` / `MultiLineString` | `src/core/SnapTraceOps.cpp:179-200` | `crates/strata-app/src/app_state_snap_lines.rs:91-111` | `DIFFERENT` |
| 6 (`6.A1`) | Subtoolbar `"폴리곤 묶기"` button is drawn clickable, but `let _ = text_button(...)` discards clicks | `src/app/MainWindowRibbon.cpp:918-923` | `crates/strata-app/src/gui_ribbon_draw_row.rs:208,213-216` | `FAKE` |
| 6 (`6.A2`) | Layer context menu `"선택한 면 묶기"` is unconditionally disabled (`CM2a-2`) | `src/app/MainWindowContextMenus.cpp:239-247,270-275,419-421` | `crates/strata-app/src/layer_menu_model.rs:223-226` | `DIFFERENT` |
| 6 (`6.B1`) | `mergeFeaturePolygons` selection check, cross-layer warning, `< 2` dialog & `refuseReadOnlyLayer` | `src/app/MainWindowExport.cpp:442-482`, `src/app/MainWindowEditing.cpp:1178-1186` | `none` | `MISSING` |
| 6 (`6.B2`) | `mergeKeeperId`, `mergeConflicts`, `mergeConflictSummary` & `"묶을까요?"` confirmation (default `No`) | `src/core/GeometryEditMerge.cpp:26-97`, `src/app/MainWindowExport.cpp:484-489` | `none` | `MISSING` |
| 6 (`6.B3`) | `LayerOps::mergePolygonFeatures` (`unaryUnion`, single-`POLYGON` disjoint refusal `"떨어진 구역은..."`, keeper attrs) | `src/core/GeometryEditMerge.cpp:99-147` | `none` (`crates/strata-geo/src/boolean_split.rs:41-43` unwired) | `MISSING` |
| 6 (`6.B4`) | Single `"폴리곤 묶기"` edit-buffer undo command (`deleteFeatures` + `addFeature`) | `src/core/GeometryEditMerge.cpp:149-151` | `none` | `MISSING` |
| 6 (`6.B5`) | Post-merge cleared selection, `setDirty(true)`, and canvas refresh | `src/app/MainWindowExport.cpp:497-498` | `none` | `MISSING` |
| 6 (`6.B6`) | Merge status message & `"폴리곤 묶기 완료"` toast | `src/app/MainWindowExport.cpp:499-506` | `none` | `MISSING` |
| 7 (`7.A1`) | Subtoolbar `"폴리곤 나누기"` button is drawn clickable, but `let _ = text_button(...)` discards clicks | `src/app/MainWindowRibbon.cpp:924-929` | `crates/strata-app/src/gui_ribbon_draw_row.rs:209,213-216` | `FAKE` |
| 7 (`7.B1`) | `refuseReadOnlyLayer("폴리곤 나누기", ...)` guard | `src/app/MainWindowEditing.cpp:1196-1197,1263` | `none` | `MISSING` |
| 7 (`7.B2`) | Branch 1 (`selected.size() == 2`): `LayerOps::splitTwoOverlappingFeatures` (`g1 ∩ g2` added to `targetLayer`) | `src/app/MainWindowEditing.cpp:1200-1232`, `src/core/LayerOps.cpp:876-1019` | `none` | `MISSING` |
| 7 (`7.B3`) | Branch 2 (`isMultipart()`): `LayerOps::explodeMultipartFeatures` | `src/app/MainWindowEditing.cpp:1234-1256`, `src/core/LayerOps.cpp:599-674` | `none` | `MISSING` |
| 7 (`7.B4`) | Branch 3 entry: `m_isSplittingPolygon = true`, `Mode::Line`, `"나누기 선"` chip & status | `src/app/MainWindowEditing.cpp:1258-1277`, `src/app/KaDrawSketchTools.cpp:180-181` | `none` | `MISSING` |
| 7 (`7.B5`) | Branch 3 finish: `onGeometryCaptured` -> `LayerOps::splitPolygonWithLine` (`QgsGeometry::splitGeometry`) | `src/app/MainWindowEditing.cpp:794-816`, `src/core/LayerOps.cpp:800-874` | `none` (`crates/strata-geo/src/boolean_split.rs:46-90`, `crates/strata-geo/src/snap_vertex_ops.rs:57-81` unwired & 2-pt only) | `MISSING` |
| 7 (`7.B6`) | `stopCaptureTool` clearing `m_isSplittingPolygon = false` on `A` / tool exit | `src/app/MainWindowEditing.cpp:284` | `none` | `MISSING` |
| 7 (`7.B7`) | Split undo steps (`FeatureAdded` `"중첩 분할 도형 생성"` / edit-buffer commands) | `src/app/MainWindowEditing.cpp:1217-1225`, `src/core/LayerOps.cpp:648-670,822-870,961-1000` | `none` | `MISSING` |
| 8 (`8.A1`) | Subtoolbar `"구간 분리"` button is drawn clickable, but `let _ = text_button(...)` discards clicks | `src/app/MainWindowRibbon.cpp:930-933` | `crates/strata-app/src/gui_ribbon_draw_row.rs:210,213-216` | `FAKE` |
| 8 (`8.B1`) | `allVecs.size() < 2 || polyVecs.isEmpty()` pre-check & info dialog | `src/app/MainWindowEditing.cpp:1287-1303` | `none` | `MISSING` |
| 8 (`8.B2`) | Modal `"겹치는 구간 분리 (클립)"` dialog (`420x200`) & `target == boundary` refusal | `src/app/MainWindowEditing.cpp:1305-1362` | `none` | `MISSING` |
| 8 (`8.B3`) | `LayerOps::clipLayerByBoundary` (`boundaryUnion`, memory layer `"[클립] %1"`, `clip_%1` in `조사 데이터`) | `src/core/LayerOps.cpp:676-798` | `none` | `MISSING` |
| 8 (`8.B4`) | `KaUndoAction::LayerAdded` (`"구간 분리 레이어 생성"`) undo step | `src/app/MainWindowEditing.cpp:1377-1381`, `src/app/MainWindowUndo.cpp:394-402` | `none` (`crates/strata-app/src/app_state_layer_remove.rs:13-18`) | `MISSING` |
| 8 (`8.B5`) | Clip status message & `"구간 분리 완료"` toast | `src/app/MainWindowEditing.cpp:1383-1386` | `none` | `MISSING` |
| 9 (`9.A1`) | Subtoolbar `"겹친 곳 지우기"` button is drawn clickable, but `let _ = text_button(...)` discards clicks | `src/app/MainWindowRibbon.cpp:934-941` | `crates/strata-app/src/gui_ribbon_draw_row.rs:211,213-216` | `FAKE` |
| 9 (`9.A2`) | `Select` right-click `"겹친 곳 지우기"` (`actSelectEraseOverlap`) with disabled `plan.hint` tooltip | `src/app/KaFeatureSelectTool.cpp:369-392,400-402` | `none` (`crates/strata-app/src/gui_tab_map_context.rs:106-152`) | `MISSING` |
| 9 (`9.A3`) | Post-sketch `drawnShapeEraseMenu` (`"겹친 곳 지우기"` / `"그대로 두기"`) on same-layer overlap | `src/app/MainWindowEditing.cpp:844-849`, `src/app/MainWindowErase.cpp:124-148` | `none` (`crates/strata-app/src/app_state_draw_commit.rs:4,200-211`) | `MISSING` |
| 9 (`9.B1`) | Empty-selection 1st press selecting `m_lastDrawnFid` with 2-press prompt | `src/app/MainWindowErase.cpp:44-53` | `none` (`crates/strata-app/src/app_state_draw_commit.rs:202` stores ID only) | `MISSING` |
| 9 (`9.B2`) | `PolygonErase::plan` (`editable`, `overlapWithArea`, same-layer-first search, largest-area target, 7 hints) | `src/core/PolygonErase.cpp:53-55,133-143,162-260` | `none` | `MISSING` |
| 9 (`9.B3`) | Multi-cutter (`plan.cutters.size() > 1`) confirmation dialog (default `No`) | `src/app/MainWindowErase.cpp:62-70` | `none` | `MISSING` |
| 9 (`9.B4`) | `PolygonErase::erased` (`1e-9` sliver filter), `partsFor` single-`POLYGON` split (clearing PKs & `uid`), cutter deletion | `src/core/PolygonErase.cpp:92-110,145-160,274-312` | `none` (`crates/strata-geo/src/boolean_split.rs:37-39`, `crates/strata-storage/src/polygon_pieces.rs:109-134` unwired) | `MISSING` |
| 9 (`9.B5`) | Per-layer `"겹친 곳 지우기"` undo commands, partial-failure rollback, `rememberLinkedEdit`, and `drawStep.close()` | `src/core/PolygonErase.cpp:318-345`, `src/app/MainWindowErase.cpp:93-100,146,174-244` | `none` | `MISSING` |
| 9 (`9.B6`) | Post-erase selection, `setCurrentLayer(firstLayer)`, handle refresh & `setDirty(true)` | `src/app/MainWindowErase.cpp:102-107` | `none` | `MISSING` |
| 9 (`9.B7`) | Area `before -> after` m² & split-piece count status text + `"겹친 곳 지우기"` toast | `src/app/MainWindowErase.cpp:95-98,109-117` | `none` | `MISSING` |
| 10 (`10.A3`) | Sketch-only `완료`/`되돌리기`/`취소` buttons are icon-only (`55x55`) without text captions | `src/app/KaDrawSketchTools.cpp:96-114` | `crates/strata-app/src/gui_ribbon_draw_row.rs:169-190` | `DIFFERENT` |
| 10 (`10.A4`) | Hardcoded x-positions clip `"겹친 곳 지우기"` (`1879..2051`) and hide `"닫기"` (`2056..2135`, `+180` sketching) at `1904`/`1920` px | `src/app/KaDrawSketchTools.cpp:130,165-169`, `tests/test_save_open.cpp:1332-1419,3533-3537` | `crates/strata-app/src/gui_ribbon_draw_row.rs:193,208-219` | `DIFFERENT` |
| 10 (`10.B3`) | `repairNotice` and form errors are appended to the status bar instead of toast / `KaUserError::warn` | `src/app/MainWindowEditing.cpp:863-869,918-919` | `crates/strata-app/src/app_state_draw_commit_b.rs:22-25`, `crates/strata-app/src/app_state_feature_form.rs:37-40` | `DIFFERENT` |
| 10 (`10.B6`) | Pressing `Esc` when `capture_active()` is false hides `sub_toolbar_visible` (leaving `sub_mode == Draw`) and forces `Pan` | `src/app/KaCaptureMapTool.cpp:381-383`, `src/app/MainWindowRibbon.cpp:942-944` | `crates/strata-app/src/gui_shell.rs:34-39` | `DIFFERENT` |

---

## Could not determine

- **Runtime-only platform metrics (`not in the source`)**: Exact pixel widths `needText`, `needIcons`, and `needSmall` of `m_subToolbar` in `TestSaveOpen::subToolbarFitsAt1904WhileSketching` (`tests/test_save_open.cpp:1391-1408`) are computed at runtime from Qt font/QSS layout metrics (`sub->sizeHint().width()`), not hardcoded in source.
- **C++ test coverage gap on `연속 그리기` (`not in the source`)**: Every behavioral branch of `m_continuousDraw` is clear in `src/app/KaDrawSketchTools.cpp:115-129` and `src/app/MainWindowEditing.cpp:853-856,910-911`, but no C++ test under `target/cpp-ref/tests/` toggles `continuousAct` ON and asserts `!formSeen` (verified by searching all occurrences of `continuous`, `kaSketch`, and `연속 그리기` across `tests/`).
- All 10 tools' C++ offered controls, function chains, geometry rules, tolerances, undo commands, Korean UI strings, and Rust counterparts were traced directly in source with zero `not traced` items.

---

## Counts

| Tool | Items (`A` + `B`) | `SAME` | `DIFFERENT` | `MISSING` | `FAKE` |
|---|---:|---:|---:|---:|---:|
| 1. 도형 선택 | 12 (`A1..A4`, `B1..B8`) | 0 | 5 | 6 | 1 |
| 2. Delete key (shape & legend) | 10 (`A1..A3`, `B1..B7`) | 2 | 5 | 3 | 0 |
| 3. Ctrl+Z and Ctrl+Y | 11 (`A1..A3`, `B1..B8`) | 3 | 7 | 1 | 0 |
| 4. 꼭짓점 수정 | 10 (`A1..A3`, `B1..B7`) | 0 | 1 | 8 | 1 |
| 5. 「가까운 선에 붙이기」 | 9 (`A1..A3`, `B1..B6`) | 4 | 3 | 0 | 2 |
| 6. 폴리곤 묶기 | 8 (`A1..A2`, `B1..B6`) | 0 | 1 | 6 | 1 |
| 7. 폴리곤 나누기 | 8 (`A1`, `B1..B7`) | 0 | 0 | 7 | 1 |
| 8. 구간 분리 | 6 (`A1`, `B1..B5`) | 0 | 0 | 5 | 1 |
| 9. 겹친 곳 지우기 | 10 (`A1..A3`, `B1..B7`) | 0 | 0 | 9 | 1 |
| 10. 연속 그리기 and 닫기 | 10 (`A1..A4`, `B1..B6`) | 6 | 4 | 0 | 0 |
| **Total** | **94** | **15** | **26** | **45** | **8** |


---

## Verification (30 Random Citations — `verification-before-completion`)

30 unique citations were sampled at random from this document (`Get-Random -Count 30 -SetSeed 20261116` over 478 unique citations) and re-opened directly in the source files before writing `EDIT0.done`:

- **Initial exactness**: **27 / 30** exact on initial check (`90.0%`).
- **Corrected (3 / 30)** — all 3 fixed in place above:
  1. `#6 tests/test_parcel_copy_keys.cpp:122-196` — line range `122-196` was exact, but row `5.C` summarized an `800x800 EPSG:5186` L-parcel instead of the actual `640x480 EPSG:5187` 3-point reference polyline `(200000,450000)-(200010,450001.2)-(200020,450000)` (`previewMarkerCount`, `pointCount() == 3`, `snappedPoly[0].size() == 6`); corrected row `5.C` (and adjacent `:201-251` `160x160 = 25,600` parcel grid bounds).
  2. `#15 crates/strata-storage/src/polygon_pieces.rs:64-129` — `1.B2` attributed `polygon_pieces.rs` to `PolygonPieces::innerPieceAt` / `withoutPiece` / `withPieceCutOut`; re-opening `polygon_pieces.rs:60-261` showed it actually defines `split_polygon_pieces` (`:77-81`), `erase_polygon_area_difference` (`:109-134`), and `MultiLayerUndoStack` (`:145-180`), while `PolygonPieces::innerPieceAt` has no Rust port; corrected `1.B2`, `3.B5`, `9.B4`, and the `All Differences` table.
  3. `#26 src/app/MainWindowUndo.cpp:91-129` — started inside the body of `MainWindow::watchUndoFeatureIds` (`auto pending = ...`) rather than its function signature at `:86-130`; updated to `src/app/MainWindowUndo.cpp:86-130` (`:86-156,403-453`).

| # | Sampled citation | Initial check | Verified content |
|---:|---|---|---|
| 1 | `src/app/MainWindowUndo.cpp:91-156,403-453` | Exact | `beforeCommitChanges` / `beforeRollBack` / `committedFeaturesAdded` / `remapUndoFeatureIdsAfterSave` + post-save `Feature*` undo branches |
| 2 | `src/app/MainWindowEditing.cpp:1200-1232` | Exact | `selected.size() == 2` -> `splitTwoOverlappingFeatures` + `FeatureAdded` undo + status/toast |
| 3 | `crates/strata-app/src/gui_ribbon_draw_row.rs:211,213-216` | Exact | `"겹친 곳 지우기"` spec + `let _ = text_button(...)` loop discarding clicks |
| 4 | `src/app/MainWindowUndo.cpp:538-575` | Exact | `MainWindow::deleteSelectedFeatures()` full body |
| 5 | `crates/strata-app/src/app_state_snap_lines.rs:53-74` | Exact | `StrataAppState::snap_targets(&mut self)` full body |
| 6 | `tests/test_parcel_copy_keys.cpp:122-196` | **Corrected** | Function range exact; updated `5.C` inputs & expected values to match source verbatim |
| 7 | `crates/strata-app/src/gui_shell.rs:136-139` | Exact | `Ctrl+1` (`Key::Num1`) setting `MapToolKind::Select` without toggle-off |
| 8 | `src/core/SnapTraceGraph.h:32-53` | Exact | `SnapTraceOps::detail::eligibleLayers` ordering `cadRef` before `survey` |
| 9 | `tests/test_layer_tree_undo.cpp:30-55` | Exact | `LayerTreeUndoTest::restorePutsOrderAndChecksBack()` full body |
| 10 | `src/app/KaVertexEditTool.h:56-57` | Exact | Negative-ID comment + `hasTarget()` checking `!FID_IS_NULL(m_fid)` |
| 11 | `src/app/KaDrawSketchTools.cpp:124-129` | Exact | `continuousAct` `toggled` handler setting `m_continuousDraw` & 5000 ms status messages |
| 12 | `src/app/MainWindowEditing.cpp:274-285` | Exact | `MainWindow::stopCaptureTool()` resetting `m_isSplittingPolygon = false` |
| 13 | `src/core/LayerOps.cpp:2697-2703` | Exact | `LayerOps::redoLayerEdits(QgsVectorLayer*)` full body |
| 14 | `src/app/KaFeatureSelectTool.cpp:209-225` | Exact | `kClickSlopPx` rubber-band marquee creation & update |
| 15 | `crates/strata-storage/src/polygon_pieces.rs:64-129` | **Corrected** | Reassigned to `9.B4` (`erase_polygon_area_difference` `:109-134`) & `3.B5` (`MultiLayerUndoStack` `:145-180`); `1.B2` marked `none` |
| 16 | `src/app/KaFeatureSelectTool.cpp:85-100` | Exact | `KaFeatureSelectTool::showSnapMark` (14 px blue circle `m_snapMark`) |
| 17 | `src/app/MainWindowRibbon.cpp:866-877` | Exact | `KaSnapSettingsWidget` wiring & `"자석 켜짐..."` / `"자석 꺼짐"` status |
| 18 | `src/core/PolygonErase.cpp:53-55,133-143,162-260` | Exact | `overlapWithArea`, `PolygonErase::editable`, and `PolygonErase::plan` |
| 19 | `src/app/MainWindowContextMenus.cpp:199-213` | Exact | Group row `"레이어 삭제"` action & tooltip |
| 20 | `src/core/LayerOps.cpp:648-670,822-870,961-1000` | Exact | `startedHere` / `started1` / `started2` + `commitChanges()` in all 3 split functions |
| 21 | `src/app/KaEditUndoOrder.cpp:75-103` | Exact | `MainWindow::pruneUndoActions()` & `newestFallbackUndoIndex()` |
| 22 | `src/app/MainWindowUndo.cpp:302-310` | Exact | `MainWindow::undoLastAction()` full body |
| 23 | `src/app/MainWindowEditing.cpp:863-869,918-919` | Exact | Form error `KaUserError::warn` & `"면을 고쳐 넣었습니다"` warning toast |
| 24 | `src/app/MainWindowUndo.cpp:469-484` | Exact | `MainWindow::redoMapAction()` full body |
| 25 | `src/app/KaEditUndoOrder.cpp:23-24,60-73` | Exact | `kMaxRemovedLayerUndo = 20`, `kMaxUndoActions = 300`, and cap pruning |
| 26 | `src/app/MainWindowUndo.cpp:91-129` | **Corrected** | Expanded to `:86-130` to include `void MainWindow::watchUndoFeatureIds` signature |
| 27 | `src/app/KaDrawSketchTools.cpp:115-124,160-163` | Exact | `continuousAct` setup/tooltip & `syncDrawSketchButtons` `"continuous"` sync |
| 28 | `src/app/MainWindowUndo.cpp:391-402` | Exact | `KaUndoAction::LayerTreeChanged` & `LayerAdded` undo branches |
| 29 | `src/app/MainWindowRibbon.cpp:924-929` | Exact | `splitAct` `"폴리곤 나누기"` setup & tooltip |
| 30 | `src/core/SnapTraceOps.cpp:228-263` | Exact | Target resolution order in `SnapTraceOps::snapCanvasShapesToNearestLines` |
