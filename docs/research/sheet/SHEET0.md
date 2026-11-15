# SHEET0 — Read-Only Specification of the Drawing Sheet (`도면` Tab, `KaDrawingStudio`) vs. Rust Today

- **Order**: `SHEET0` (`docs/senior/research/sheet/ORDER-SHEET0.yaml`)
- **Workspace `HEAD`**: `a9d24f3` (read-only main checkout)
- **Citation convention**: C++ paths are relative to `target/cpp-ref/` (`src/...:line`, `tests/...:line`); Rust paths are relative to checkout root (`crates/...:line`).

## 0. File Inventory, Scope Boundaries, and Verification of Director's 4 Notes

### 0.1 C++ `KaDrawing*` / `KaLayout*` and Supporting Core Files Found (`ORDER-SHEET0.yaml:40`)
- **App (`src/app/`)**:
  - `src/app/KaDrawingStudio.h:1-290`, `src/app/KaDrawingStudio.cpp:1-3897` (also defines `KaLayoutMenuProvider` at `src/app/KaDrawingStudio.cpp:315-355`)
  - `src/app/KaDrawingStudioSheets.cpp:1-170` (`placeTitleBlock`, `storeCurrentSheet`, `restoreStoredSheet`, `removeStoredSheet`)
  - `src/app/KaDrawingStudioPlate.h:1-26`, `src/app/KaDrawingStudioPlate.cpp:1-207` (`paperLook` / `조용한 도판` chrome for right inspector, bottom strip, scale table, word tiles, `세트 ▾`)
  - `src/app/KaDrawingCoordCallout.h:1-33`, `src/app/KaDrawingCoordCallout.cpp:1-146`
  - `src/app/KaDrawingTools.h:1-58`, `src/app/KaDrawingTools.cpp:1-182` (defines `KaLayoutCoordPointTool` and `KaLayoutMapAdjustTool` — there are no separate `KaLayout*.cpp` files)
  - `src/app/KaDrawingNorthArrow.h:1-31`, `src/app/KaDrawingNorthArrow.cpp:1-209`
  - `src/app/KaDrawingSheetSet.h:1-41`, `src/app/KaDrawingSheetSet.cpp:1-164`
  - `src/app/KaDrawingLayerSets.h:1-25`, `src/app/KaDrawingLayerSets.cpp:1-109`
  - `src/app/KaDrawingPdfProgress.h:1-29`, `src/app/KaDrawingPdfProgress.cpp:1-84`
  - `src/app/KaTitleBlock.h:1-34`, `src/app/KaTitleBlock.cpp:1-118`
  - `src/app/KaPaperDialog.h:1-26`, `src/app/KaPaperDialog.cpp:1-198`
  - `src/app/KaPrintDialog.h:1-91`, `src/app/KaPrintDialog.cpp:1-669`
  - `src/app/KaTilePreview.h:1-48`, `src/app/KaTilePreview.cpp:1-213`
  - Callers in `MainWindow`: `src/app/MainWindowExport.cpp:186-278`, `src/app/MainWindow.cpp:1184-1208`, `src/app/MainWindowUndo.cpp:302-323`, `src/app/MainWindowContextMenus.cpp:254-262`
- **Core (`src/core/`)**:
  - `src/core/LayoutService.h:1-186`, `src/core/LayoutService.cpp:480-1645`
  - `src/core/StandardScales.h:1-32`, `src/core/StandardScales.cpp:1-92`
  - `src/core/PdfExportSettings.h:1-27`, `src/core/PdfExportSettings.cpp:1-37` (`namespace KaPdfExport`)
  - `src/core/TilePrint.h:1-94`, `src/core/TilePrint.cpp:1-160`
  - `src/core/ExportService.cpp:73-241`, `src/core/SubmitReadme.cpp:1-144`
  - `src/core/DemColorRampLegend.cpp:1-226`, `src/core/HeritageSiteLegend.cpp:1-75`
- **Out of scope (`ORDER-SHEET0.yaml:21-25, 68`)**: `KaTerrain3dLayoutStudio`, report documents, and details of `already_done_in_rust` (`레이어를 가운데` at `src/app/KaDrawingStudio.cpp:3708-3756`, survey-area initial centering owner rule, map-frame layer painting, and `HeritageLayoutNumbers.cpp`), which are mentioned below only where `P1`–`P11` directly meet them.

### 0.2 Verification / Correction of Director's 4 Notes (`ORDER-SHEET0.yaml:44`) at `HEAD = a9d24f3`
1. **「항목 이동」 only writes a message (`gui_tab_drawing_toolbar.rs`)** — **CORRECTED (partially implemented)**:
   - Clicking button 4 (`"항목 이동"`) at `crates/strata-app/src/gui_tab_drawing_toolbar.rs:104` calls `self.use_sheet_select_tool()` (`crates/strata-app/src/gui_tab_drawing_items.rs:68-74`), which exits `coord_callout_mode` (when `!map_adjust_active`) and writes `"항목을 눌러 끌면 옮겨집니다."`. Meanwhile `self.move_sheet_items(ui, sheet_rect, px_per_mm)` (`crates/strata-app/src/gui_tab_drawing_sheet.rs:103`, `crates/strata-app/src/gui_tab_drawing_items.rs:47-64`) drags 6 `SheetItem` variants (`Legend`, `TitleBlock`, `NorthArrow`, `ScaleBar`, `ScaleText`, `CrsLabel`) whenever `!map_adjust_active && !coord_callout_mode`.
   - **What is still missing**: item click-selection, resize handles, moving/resizing `ka_map` or `CoordCallout` boxes, arrow-key nudging, `Delete` on `MainTab::Drawing` (`crates/strata-app/src/app_state_layer_remove.rs:109-112` returns immediately), and sheet `Ctrl+Z` / `Ctrl+Y` (`crates/strata-app/src/gui_shell.rs:155-160` calls `undo_map_action` / `redo_map_action`).
2. **「지도 조정」 only shows its bar (`gui_tab_drawing_popups.rs`)** — **CORRECTED (pan + wheel + snap-up implemented, details differ)**:
   - `crates/strata-app/src/gui_tab_drawing_popups.rs:19-65` draws the top bar, AND `crates/strata-app/src/gui_tab_drawing_sheet.rs:102` calls `self.adjust_sheet_map(ui, map_rect, px_per_mm)` (`crates/strata-app/src/gui_tab_drawing_adjust.rs:18-47`), which pans `sheet_center` on primary drag and zooms `scale_denominator` (`10.0..=500_000.0`) around the pointer on mouse wheel, and `end_sheet_map_adjust()` (`crates/strata-app/src/gui_tab_drawing_adjust.rs:64-71`) snaps `scale_denominator` up via `strata_layout::standard_scales::snap_up`. Also `crates/strata-app/src/gui_tab_drawing_toolbar.rs:75` switches the bottom button label to `"조정 끝"` while active.
   - **What still differs / is missing**: wheel factor uses `1.2` (`crates/strata-app/src/gui_tab_drawing_adjust.rs:36`) instead of C++ `1.25` with `220 ms` batching (`src/app/KaDrawingTools.cpp:52, 111-121`), no `0.2 mm` drag threshold (`src/app/KaDrawingTools.cpp:93-94`), no `48.0 DPI` fast preview (`src/app/KaDrawingStudio.cpp:3204, 3569-3588`), no map right-click context menu (`src/app/KaDrawingStudio.cpp:337-350`), and ending does not relayout coordinate callouts (`src/app/KaDrawingStudio.cpp:3605-3606`).
3. **A coordinate point is kept as a place on the SCREEN (`gui_tab_drawing_popups.rs`)** — **CONFIRMED**:
   - `CoordCallout` (`crates/strata-app/src/tab_drawing.rs:46-53`) stores `pub pos: [f32; 2]` in screen pixels alongside `pub world: [f64; 2]`. On click, `crates/strata-app/src/gui_tab_drawing_popups.rs:77-96` stores `[pos.x, pos.y]`; when painting (`crates/strata-app/src/gui_tab_drawing_popups.rs:98-125`), it draws at `let cp = Pos2::new(c.pos[0], c.pos[1]);` without projecting `c.world` through `f.w2m(...)`. Panning/zooming the sheet map, changing paper, or resizing the window leaves every callout frozen at its old screen pixel.
4. **「PDF 내보내기」 is switched off (`gui_tab_drawing_right_b.rs`)** — **CONFIRMED**:
   - `crates/strata-app/src/gui_tab_drawing_right_b.rs:144` renders `ui.add_enabled(false, pdf_btn).on_disabled_hover_text(DEFERRED_R3)` where `DEFERRED_R3 = "아직 Rust 판에서 연결하지 않았습니다 (지시 R3)"` (`crates/strata-app/src/gui_tab_drawing_right_b.rs:10`). Underlying `render_sheet_pdf_bytes` (`crates/strata-layout/src/sheet_pdf_export.rs:27-56`) only writes empty PDF rectangle strokes (`re S`).

---

## P1 — `P1_toolbar`: Seven Buttons Under the Sheet

### A. C++ offered
- **Container**: `QWidget` (`objectName = "layoutBottomTools"`) built in `KaDrawingStudio::buildBottomTools(QWidget* desk)` (`src/app/KaDrawingStudio.cpp:1279-1342`) and wrapped by `KaDrawingStudioPlate::bottomStrip(...)` (`src/app/KaDrawingStudio.cpp:1234`, `src/app/KaDrawingStudioPlate.cpp:105-120`), anchored at `Qt::AlignHCenter | Qt::AlignBottom` over `desk` (`src/app/KaDrawingStudio.cpp:1234`).
- **Enabled / hidden rule**: Always visible and enabled while `KaDrawingStudio` is shown (`src/app/KaDrawingStudio.cpp:1280-1341`; no `setEnabled(false)` or `hide()` on any of the 7 buttons).
- **Checkable state**: **None** of the 7 `QToolButton`s call `setCheckable(true)` (`src/app/KaDrawingStudio.cpp:1286-1339`). Button #3 (`btnMapAdjust`) changes its text and icon when active (`src/app/KaDrawingStudio.cpp:3616-3621`); button #7 (`btnSheetSet`) uses `QToolButton::InstantPopup` (`src/app/KaDrawingStudio.cpp:1328`).
- **Seven buttons in left-to-right order** (`src/app/KaDrawingStudio.cpp:1298-1339`):

| # | `objectName` | Icon ID | Stock text (`paperLook=false`) | `paperLook=true` text (`KaDrawingStudioPlate.cpp:115`) | Tooltip verbatim | Checkable | Slot / popup |
|---|---|---|---|---|---|---|---|
| 1 | *(none)* | `"layout_coord_point"` | `"좌표점"` | `"좌표점"` | `"용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다"` | No | `KaDrawingStudio::beginPlaceCoordPoint` (`:1298-1300`) |
| 2 | *(none)* | `"layout_center"` | `"레이어를 가운데"` | `"레이어를 가운데"` | `"축척은 두고, 고른 레이어를 용지 한가운데로 옮깁니다"` | No | `KaDrawingStudio::centerSurveyInMap` (`:1301-1303`) |
| 3 | `"btnMapAdjust"` | `"layout_activate"` (idle) / `"layout_activate_done"` (active, `:3619-3620`) | `"지도 조정"` (idle) / `"조정 끝"` (active, `:3618`) | `"지도 조정"` / `"조정 끝"` | `"지도 칸 안의 지도를 끌어 옮기고 휠로 확대합니다. 다시 누르면 끝냅니다"` | No | `KaDrawingStudio::toggleMapAdjust` (`:1305-1309`) |
| 4 | *(none)* | `"map"` | `"화면 맞춤"` | `"화면 맞춤"` | `"지도 화면과 같은 범위·축척으로 지도 칸을 다시 맞춥니다"` | No | `KaDrawingStudio::fitToMapCanvas` (`:1310-1312`) |
| 5 | *(none)* | `"layout_select"` | `KaBeginnerRibbon::twoLine("항목 이동")` → `"항목\n이동"` (`:1314`) | `"항목 이동"` (`\n` replaced by `' '` at `KaDrawingStudioPlate.cpp:115`) | `"좌표 상자를 끌어 옮깁니다"` | No | `KaDrawingStudio::useSelectTool` (`:1313-1316`) |
| 6 | *(none)* | `"layout_map_frame"` | `"용지/방향"` | `"용지/방향"` | `"용지 크기(A4/A3) 및 방향(가로/세로)을 전환합니다"` | No | `KaDrawingStudio::openPaperSettingsDialog` (`:1317-1319`) |
| 7 | `"btnSheetSet"` | `"open"` | `"도면 목록"` | `"도면 목록"` | `"지금 도면을 이름 붙여 보관하고, 보관한 도면을 다시 엽니다. 제출 PDF는 지금 열린 도면입니다"` | No (`InstantPopup` `:1328`) | `QMenu::aboutToShow` → `KaDrawingSheetSet::fillMenu` (`:1330-1338`) |

### B. C++ does
1. **Geometry & constants**:
   - Stock look (`src/app/KaDrawingStudio.cpp:1284-1295`): `QHBoxLayout` margins `(14, 6, 14, 6)` px, spacing `22` px, `Qt::ToolButtonTextUnderIcon`, icon size `22 × 22` px, `autoRaise = true`.
   - `paperLook` (`src/app/KaDrawingStudioPlate.cpp:105-120`): margins `(0, 0, 0, 0)` px, spacing `0` px, each `QToolButton` `setFixedSize(96, 52)` px (`7 × 96 = 672` px total), newline `0x0A` replaced by `' '`, `kaCell = "first"` on cell 0 and `"rest"` on cells 1..6.
2. **Start / end lifecycle of each button**:
   - **#1 `좌표점`** (`beginPlaceCoordPoint`, `src/app/KaDrawingStudio.cpp:1662-1675`): calls `endActivateMap()`, checks `m_view && m_toolCoordPoint && mapItem()` (if missing, status `"먼저 지도 칸을 그리세요."`), activates `m_view->setTool(m_toolCoordPoint)` (note: clicking `좌표점` again while already in `m_toolCoordPoint` stays in `m_toolCoordPoint` — it is **not** a toggle). Ends when user clicks `#5 항목 이동` (`useSelectTool`, `:1891-1897`), `#3 지도 조정` (`beginActivateMap`, `:3195-3211`), right-clicks the map frame and chooses `"점찍기 끝"` (`endPlaceCoordPoint`, `:334, 1681-1685`), or clicks a right-panel placement button (`beginPlace*`, `:1606-1660`).
   - **#2 `레이어를 가운데`** (`centerSurveyInMap`, `src/app/KaDrawingStudio.cpp:3708-3756`): one-shot action (`already_done_in_rust`); calls `endActivateMap()` (`:3715`) and pans `ka_map` keeping current scale.
   - **#3 `지도 조정`** (`toggleMapAdjust`, `src/app/KaDrawingStudio.cpp:3609-3614`): if `!m_adjustingMap`, calls `beginActivateMap()` (`:3195-3211`), showing `m_adjustBar`, setting `m_view->setTool(m_toolMoveContent)`, and changing `#3` text to `"조정 끝"` (`syncAdjustButton`, `:3616-3621`). Ends when `#3` is clicked again, `m_adjustBar`'s `"조정끝"` button is clicked (`:839`), right-click menu `"지도조정끝"` is clicked (`:347-348`), or any action calling `endActivateMap()` runs (`좌표점`, `레이어를 가운데`, `화면 맞춤`, `용지/방향`, `도면 목록` store/restore, `표제란`, `범례`, `방위`, `축척자`, `축척 글자`, `좌표계`). Note: clicking `#5 항목 이동` while `m_adjustingMap == true` is a **no-op** (`if (m_adjustingMap) return;` at `:1892`)!
   - **#4 `화면 맞춤`** (`fitToMapCanvas`, `src/app/KaDrawingStudio.cpp:3623-3635`): one-shot; calls `endActivateMap()`, `centerOnMapCanvas()`, `markUserComposed()`.
   - **#5 `항목 이동`** (`useSelectTool`, `src/app/KaDrawingStudio.cpp:1891-1897`): if `m_adjustingMap` is `true`, returns immediately (`:1892`); otherwise sets `m_view->setTool(m_toolSelect)` and status `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."` (`:1896`). Stays active as the default tool whenever no other `QgsLayoutViewTool` is set (`:623-624`).
   - **#6 `용지/방향`** (`openPaperSettingsDialog`, `src/app/KaDrawingStudio.cpp:447-454`): opens modal `KaPaperDialog::prompt`; on accept calls `resetPaper(w, h, true)` (`:474` calls `endActivateMap()`) and `markUserComposed()`.
   - **#7 `도면 목록`** (`src/app/KaDrawingStudio.cpp:1321-1339`): opens `QMenu` immediately via `QToolButton::InstantPopup`; closes when an action is triggered or user clicks outside.
3. **Undo step**: None on toolbar clicks themselves (sub-actions record undo as documented in P2–P9).

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `bottomToolsAreOneStripOfCellsInPaper` | `tests/test_drawing_plate.cpp:67-98` | `QWidget("layoutBottomTools")` with `QHBoxLayout` margins `(14,6,14,6)`, spacing `22`, 3 `QToolButton`s | `KaDrawingStudioPlate::bottomStrip(tools.get())` with `kaPaperLook = true` then `QVariant()` | words `{"좌표점", "항목\n이동", "도면 목록"}` | `paperLook=true`: returns `tools.get()`, `spacing() == 0`, `contentsMargins() == QMargins()`, `cells.size() == 3`, each `minimumSize() == QSize(96, 52)`, `!cell->text().contains(QLatin1Char(0x0A))`, `cells[0]->property("kaCell") == "first"`, `cells[1]->property("kaCell") == "rest"`; stock: `spacing() == 22` |
| `drawingStudioLayoutAndExport` (bottom toolbar check) | `tests/test_workflow.cpp:5499-5509` | `MainWindow` + `KaDrawingStudio` | `findChild<QWidget*>("layoutBottomTools")`, `findChild<QToolButton*>("btnMapAdjust")` | default studio | `layoutBottomTools != nullptr`, `btnMapAdjust != nullptr`, `btnMapAdjust->text() == "지도 조정"` |

### D. Rust today
- **Strip geometry** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:15-38`): **DIFFERENT** — custom painter strip of width `504.0_f32.min((rect.width() - 12.0).max(210.0))` (`~72 px` per cell) and height `34.0` (`compact_v`) or `42.0` px (`:15-16`), rounded `6.0` px, instead of C++ `96 × 52` px (`paperLook`) or `22 px`-spaced `QToolButton`s.
- **Labels & tooltips of buttons #1, #2, #3, #4, #6, #7** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:23-30, 75`): **SAME** — labels `"좌표점"`, `"레이어를 가운데"`, `"지도 조정"` / `"조정 끝"` (`:75`), `"화면 맞춤"`, `"용지/방향"`, `"도면 목록"` and their 6 tooltips match C++ verbatim.
- **Tooltip of button #5 `항목 이동`** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:27`): **DIFFERENT** — Rust has `"범례·표제란·방위표·축척을 끌어 옮깁니다"` instead of C++ `"좌표 상자를 끌어 옮깁니다"` (`src/app/KaDrawingStudio.cpp:1315`).
- **Button #1 `좌표점` click behavior** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:85-94`): **DIFFERENT** — Rust toggles `self.drawing.coord_callout_mode = !self.drawing.coord_callout_mode` (`:88`) with messages `"좌표점 모드 — 용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다."` / `"좌표점 모드를 껐습니다."`, whereas C++ `beginPlaceCoordPoint` (`src/app/KaDrawingStudio.cpp:1662-1675`) always enters `m_toolCoordPoint` (not a toggle) with status `"지도 칸에서 찍으세요. 선·점에 자석이 붙습니다. 우클릭·Delete·Ctrl+Z는 마지막 점 지우기."`.
- **Button #2 `레이어를 가운데` click** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:95-97`): **SAME** (`already_done_in_rust`, calls `self.center_layer_in_sheet()`).
- **Button #3 `지도 조정` click** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:98`): **SAME** entry/exit toggle via `self.toggle_sheet_map_adjust()` (`crates/strata-app/src/gui_tab_drawing_adjust.rs:50-60`).
- **Button #4 `화면 맞춤` click** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:99-103`): **DIFFERENT** — Rust calls `replace_sheet_chrome()` + `recover_orphaned_items()` + `fit_sheet_to_map_view()` without exiting `coord_callout_mode` (see P4).
- **Button #5 `항목 이동` click** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:104`, `crates/strata-app/src/gui_tab_drawing_items.rs:68-74`): **DIFFERENT** — returns early when `map_adjust_active` (matching C++ `:1892`) and clears `coord_callout_mode`, but writes `"항목을 눌러 끌면 옮겨집니다."` (`:73`) instead of C++ `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."` (`src/app/KaDrawingStudio.cpp:1896`).
- **Button #6 `용지/방향` click** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:105-109`): **DIFFERENT** — opens `ActiveModalDialog::PaperSetup`, which resets its draft to A4 Portrait `210 × 297 mm` instead of current sheet size and does not call `end_sheet_map_adjust()` on accept (see P9).
- **Button #7 `도면 목록` popup dismiss** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:110-113`): **DIFFERENT** — toggles `sheet_set_menu_open` boolean instead of a transient `QMenu` (`InstantPopup`), so clicking outside the menu does not close it (`crates/strata-app/src/gui_tab_drawing_popups.rs:129-232`).

---

## P2 — `P2_coordinate_points`: `좌표점` (`KaDrawingCoordCallout` and Callers)

### A. C++ offered
1. **Bottom toolbar button #1**: `"좌표점"`, tooltip `"용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다"`, always enabled (`src/app/KaDrawingStudio.cpp:1298-1300`).
2. **Layout view tool**: `KaLayoutCoordPointTool` (`src/app/KaDrawingTools.h:16-28`, `src/app/KaDrawingTools.cpp:28-46`), cursor `Qt::CrossCursor` (`:30`), left-click emits `pointClicked(event->layoutPoint())` (`:44`), right-click emits `undoRequested()` (`:35-38`).
3. **Right-click context menu on `ka_map` while `isPlacingCoordPoint()` is true** (`KaLayoutMenuProvider::createContextMenu`, `src/app/KaDrawingStudio.cpp:330-336`):
   - `"마지막 점 지우기"` → `KaDrawingStudio::undoLastCoordCallout`
   - `"점찍기 끝"` → `KaDrawingStudio::endPlaceCoordPoint`
4. **Map-tab import bridge**: `KaDrawingStudio::importMapCoordCallouts` (`src/app/KaDrawingStudio.cpp:1405-1444`, called from `MainWindowExport.cpp:253` when opening studio with map coordinate callouts).

### B. C++ does
1. **Entering mode (`beginPlaceCoordPoint`, `src/app/KaDrawingStudio.cpp:1662-1675`)**:
   - Step 1: `endActivateMap()` (`:1663`).
   - Step 2: Guard `!m_view || !m_toolCoordPoint || !mapItem()` → status `"먼저 지도 칸을 그리세요."` and return (`:1664-1668`).
   - Step 3: `m_view->setTool(m_toolCoordPoint)` (`:1671`); status `"지도 칸에서 찍으세요. 선·점에 자석이 붙습니다. 우클릭·Delete·Ctrl+Z는 마지막 점 지우기."` (`:1673-1674`).
2. **Clicking on the sheet (`placeCoordCallout(const QPointF& layoutPt)`, `src/app/KaDrawingStudio.cpp:1687-1804`)**:
   - Step 1: Require `ly = layout()` and `map = mapItem()`; compute `itemPt = map->mapFromScene(layoutPt)` and `ir = map->rect()`; require `ir.width() >= 1.0 && ir.height() >= 1.0` (`:1688-1693`).
   - Step 2 (Inside-map-frame check): `LayoutService::layoutMapItemContains(itemPt, ir, 0.8)` (`src/core/LayoutService.cpp:1017-1020`, `marginMm = 0.8 mm` tolerance around `ir`). If outside, status `"지도 칸 안에서 찍으세요. 흰 용지에는 점이 안 생깁니다."` and return (`src/app/KaDrawingStudio.cpp:1694-1698`).
   - Step 3 (Ground coordinate in map CRS, `f64` Easting `x`, Northing `y`): `click = LayoutService::layoutMapXyFromItem(ext, ir, itemPt)` (`src/app/KaDrawingStudio.cpp:1702`, `src/core/LayoutService.cpp:1006-1015`).
   - Step 4 (18 px vector vertex/segment magnet, `:1704-1779`):
     - `viewPx = max(1e-9, m_view->transform().m11())`; `snapMm = 18.0 / viewPx` (`18.0` screen pixels converted to layout mm, `:1704-1705`).
     - Ground tolerance `tol = max(ext.width() * snapMm / max(1.0, ir.width()), ext.height() * snapMm / max(1.0, ir.height()))` m (`:1706-1707`).
     - Searches checked visible `QgsVectorLayer`s in `map->layers()` plus `ka_map_above->layers()`, **excluding** `LayerOps::isReferenceLayer(vl)` and `LayerOps::isCadastralLayer(vl)` (`:1714-1730`), reprojecting via `QgsCoordinateTransform` when layer CRS differs from `mapCrs` (`:1732-1755`).
     - Tests both `g.closestVertex(...)` and `g.closestSegmentWithContext(...)`; nearest hit `< tol` updates `mapPt` and sets `snapped = true` (`:1756-1778`).
   - Step 5 (Paper tip & letter tag): `tip = map->mapToScene(LayoutService::layoutMapItemFromXy(ext, ir, mapPt.x(), mapPt.y()))` (`:1781-1782`). Counts existing `QgsLayoutItemLabel`s with `id().startsWith("ka_coord_box_")` as `n` (`:1784-1790`), computes `tag = KaDrawingCoordCallout::tagFor(n)` → `'A' + (max(0, index) % 26)` (cycles `A..Z`, then `A..Z` after 26; no hard count cap, `src/app/KaDrawingCoordCallout.cpp:31-33`).
   - Step 6 (Number format — Korean surveying convention): `KaDrawingCoordCallout::xyText(mapPt.x(), mapPt.y())` (`src/app/KaDrawingCoordCallout.cpp:35-37`) formats **two lines** to **3 decimal places** with **`X` = Northing (`mapY`) and `Y` = Easting (`mapX`)**: `"X=%1\nY=%2".arg(mapY, 0, 'f', 3).arg(mapX, 0, 'f', 3)`.
   - Step 7 (Four `QgsLayoutItem`s added by `KaDrawingCoordCallout::add`, `src/app/KaDrawingCoordCallout.cpp:39-134`):
     - Box rect `clickBoxRect(tip, m_paperW)` (`:39-44`): size `26.0 × 7.2 mm`; `placeRight = (tip.x() + 34.0 < paperWidthMm - 3.0)` → `x = placeRight ? tip.x() + 6.0 : tip.x() - 32.0`; `placeUp = (tip.y() - 10.0 > 3.0)` → `y = placeUp ? tip.y() - 9.0 : tip.y() + 3.0`. (For imported points, `freeBoxRect` `:46-69` tries 6 candidate `26.0 × 7.2 mm` rects at `gap = 8.0 mm` with `2 mm` padding against `usedBoxes`.)
     - `ka_coord_box_<tag>` (`QgsLayoutItemLabel`, `:74-86`): `Malgun Gothic 6.0 pt` normal, `AlignLeft | AlignVCenter`, frame enabled, background `rgba(255, 255, 255, 235)`, margins `X = 0.4 mm`, `Y = 0.3 mm`, rect `boxRect`.
     - `ka_coord_let_<tag>` (`QgsLayoutItemLabel`, `:88-97`): text `tag`, `Malgun Gothic 8.0 pt` bold, centered, no frame/background, rect `(tip.x() - 2.4, tip.y() - 5.4, 5.2, 4.2)` mm.
     - `ka_coord_arr_<tag>` (`QgsLayoutItemPolyline`, `:99-122`): attaches at `(placeRight ? boxRect.left() : boxRect.right(), boxRect.center().y())` where `placeRight = boxRect.center().x() >= tip.x()`; shaft runs from `attach` to `neck = tip - dir * 2.4` (`ah = 2.4 mm`), `#1C1917` line width `0.22 mm`.
     - `ka_coord_head_<tag>` (`QgsLayoutItemPolygon`, `:124-133`): solid `#1C1917` triangle with vertices `tip` and `neck ± normal * (2.4 * 0.42)` (`±1.008 mm` half-width), no outline.
   - Step 8 (Connecting orange frame `ka_coord_frame`, `updateCoordFrame()`, `src/app/KaDrawingStudio.cpp:1796-1797, 1841-1889`): appends `QPointF(mapPt.x(), mapPt.y())` to `m_coordMapPts`; when `>= 2` points exist, adds/updates `QgsLayoutItemPolygon` `"ka_coord_frame"` with transparent fill and `#C2410C` outline width `0.45 mm`.
   - Step 9 (Undo & status, `:1794-1804`): calls `markUserComposed()`, pushes `"ka_coord_<tag>"` onto `m_placeUndo`, shows status `"%1점  X=%2  Y=%3%4"` (`3` decimals, `X=mapPt.y()`, `Y=mapPt.x()`, suffix `"  (자석)"` when `snapped`).
3. **How points stay tied to the ground (`relayoutCoordCallouts`, `src/app/KaDrawingStudio.cpp:1823-1839`)**:
   - `m_coordMapPts` stores ground coordinates `(mapX, mapY)` in map CRS (`:1796`).
   - Whenever the map pans, zooms, changes scale, fits to canvas, centers a layer, or changes paper (`resetPaper` `:542-545`, `flushHeavyScaleSync` `:2954-2955`, `endActivateMap` `:3605-3606`, `panLayoutMapTo` `:3660-3661`, `centerOnMapCanvas` `:3699-3700`, `centerSurveyInMap` `:3751-3752`), `relayoutCoordCallouts()` removes all `ka_coord_*` items and rebuilds them from `m_coordMapPts` at updated paper `tip` positions via `applyImportedCoordCallouts()` (`freeBoxRect`) + `updateCoordFrame()`.
4. **Moving, deleting, and saving**:
   - **Moving**: In `m_toolSelect` (`항목 이동`), each `ka_coord_box_<tag>` (and `let`/`arr`/`head`) is a standard unlocked `QgsLayoutItem` and can be dragged freely (`src/app/KaDrawingStudio.cpp:1315, 1893-1896`). Note: dragging `ka_coord_box_<tag>` moves the box item itself (the polyline `ka_coord_arr_<tag>` does not dynamically re-anchor on manual box drag, and the next `relayoutCoordCallouts()` rebuilds boxes at `freeBoxRect`).
   - **Deleting**:
     - While in `m_toolCoordPoint`: right-click (`src/app/KaDrawingTools.cpp:35-38`), `Delete`/`Backspace` (`src/app/KaDrawingStudio.cpp:3466-3467, 3518-3519`), or `Ctrl+Z` (`:3480-3483`) calls `undoLastCoordCallout()` (`:1806-1821`), popping `m_coordMapPts.removeLast()`, relayouting remaining points, and showing `"지울 좌표점이 없습니다."` / `"좌표점을 모두 지웠습니다."` / `"마지막 좌표점을 지웠습니다. 남은 점 %1개."`.
     - In `m_toolSelect`: selecting any `ka_coord_*_<tag>` item and pressing `Delete`/`Backspace` calls `deleteSelectedItems()` (`:3225-3248`), which strips that tag's point from `m_coordMapPts`, re-tags/relayouts remaining points `A, B, ...`, and shows `"선택한 항목 %1개를 지웠습니다."`.
   - **What is saved**: The 4 `QgsLayoutItem`s per point (`ka_coord_box_*`, `ka_coord_let_*`, `ka_coord_arr_*`, `ka_coord_head_*`) and `ka_coord_frame` live inside `QgsPrintLayout` (`user_sheet`), so they serialize into `.qgz` project XML and into `KaDrawingSheetSet` clones (`src/app/KaDrawingSheetSet.cpp:62`). However, `m_coordMapPts` is an in-memory `QVector<QPointF>` on `KaDrawingStudio` (`src/app/KaDrawingStudio.h:261`) and is cleared on `restoreStoredSheet` (`src/app/KaDrawingStudioSheets.cpp:137`) and not reconstructed when reopening a saved `.qgz`.

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `coordCallout_sharedBuilderMakesFourItems` | `tests/test_drawing_studio.cpp:101-133` | `QgsProject` + `QgsPrintLayout` | `tagFor`, `xyText`, `clickBoxRect`, `add`, `freeBoxRect`, `removeAll` | `tagFor(0)`, `tagFor(27)`; `xyText(200123.4, 550987.6)`; `tip(100.0, 80.0)`, `paperWidth=210.0`; `nearEdge(200.0, 80.0)`; `add(..., "A", xyText(1.0, 2.0), box)` | `tagFor(0) == "A"`, `tagFor(27) == "B"`; `xyText(200123.4, 550987.6) == "X=550987.600\nY=200123.400"`; `box.left() > 100.0`; `nearEdge.right() < 200.0`; items `"ka_coord_box_A"`, `"ka_coord_let_A"`, `"ka_coord_arr_A"`, `"ka_coord_head_A"` all non-null; `ka_coord_box_A->text() == "X=2.000\nY=1.000"`; `!second.adjusted(-2,-2,2,2).intersects(used.first())`; after `removeAll`, 0 items start with `"ka_coord_"` |
| `drawingStudioLayoutAndExport` (coord callout section) | `tests/test_workflow.cpp:5511-5555` | `MainWindow` + `KaDrawingStudio` with polygon feature | `beginPlaceCoordPoint()`, synthetic click / `placeCoordCallout`, `undoLastCoordCallout()` | map click inside `ka_map` and outside `ka_map` | outside click creates 0 callouts and status says `"지도 칸 안에서 찍으세요"`; inside click creates `ka_coord_box_A`, status contains `"A점"`, `undoLastCoordCallout()` removes `ka_coord_box_A` |

### D. Rust today
- **Ground-tying on render (`crates/strata-app/src/tab_drawing.rs:46-53`, `crates/strata-app/src/gui_tab_drawing_popups.rs:84-125`)**: **DIFFERENT (broken)** — `CoordCallout` stores screen pixels `pos: [f32; 2]` at click time (`:79, 94`) and paints at `Pos2::new(c.pos[0], c.pos[1])` (`:99`) instead of projecting `c.world` with `f.w2m([c.world[1], c.world[0]])`. Callouts do not move when the sheet map pans, zooms, fits, centers, or changes paper.
- **Inside-map-frame check (`0.8 mm` margin)**: **MISSING** — `crates/strata-app/src/gui_tab_drawing_popups.rs:71-80` interacts over the entire `sheet_rect` (white paper margin included) instead of `f.map_rect` + `0.8 mm`, and never shows `"지도 칸 안에서 찍으세요. 흰 용지에는 점이 안 생깁니다."`.
- **18 px vector vertex/segment magnet**: **MISSING** — `crates/strata-app/src/gui_tab_drawing_popups.rs:89` takes raw `f.m2w(Pos2::new(pos[0], pos[1]))` with zero snapping.
- **Box text, axis order, and decimals**: **DIFFERENT** — `crates/strata-app/src/gui_tab_drawing_popups.rs:100` formats single-line `"A  X {:.2}  Y {:.2}"` (`2` decimals, tag inside box, `X = world[1]` [Easting!] and `Y = world[0]` [Northing!] — look at `:89, 95, 100`: `let [e, n] = f.m2w(...); world: [n, e]; format!("{}  X {:.2}  Y {:.2}", c.tag, c.world[1], c.world[0])` which swaps Korean surveying `X=Northing, Y=Easting`!) instead of C++ 2-line 3-decimal `"X=<Northing .3f>\nY=<Easting .3f>"` (`src/app/KaDrawingCoordCallout.cpp:35-37`).
- **Callout visuals & 4 items + `ka_coord_frame` polygon**: **DIFFERENT / MISSING** — Rust (`crates/strata-app/src/gui_tab_drawing_popups.rs:101-124`) draws an orange dot (`r = 3.2 px`), a 1 px orange line without arrowhead, and a single-line pill at fixed pixel offset `(+8, -22)` with no edge-flip (`clickBoxRect`), no separate bold letter item above tip, and no `#C2410C` `ka_coord_frame` polygon.
- **Moving callout boxes**: **MISSING** — `SheetItem` (`crates/strata-app/src/tab_drawing.rs:35-42`) has no variant for coordinate callouts; `move_sheet_items` cannot drag them.
- **Deleting / undoing callouts (right-click, `Delete`, `Ctrl+Z`, context menu)**: **MISSING** — no right-click handler, no context menu, `Delete` returns early on `MainTab::Drawing` (`crates/strata-app/src/app_state_layer_remove.rs:109-112`), and `Ctrl+Z` calls `undo_map_action()` (`crates/strata-app/src/gui_shell.rs:157`). Once placed in Rust, a coordinate callout can **never** be deleted or moved.
- **Persistence (save/reopen & `도면 목록`)**: **MISSING** — neither project save (`crates/strata-app/src/project_sidecar.rs`) nor `StoredSheet` (`crates/strata-app/src/tab_drawing.rs:57-64`) saves `coord_callouts`.

---

## P3 — `P3_map_adjust`: `지도 조정` (Top Bar and `KaLayoutMapAdjustTool`)

### A. C++ offered
1. **Top bar `m_adjustBar`** (`QFrame`, `objectName = "mapAdjustBar"`, `src/app/KaDrawingStudio.cpp:828-843, 1188-1189`):
   - Placed at `Qt::AlignTop` inside `deskGrid` above `m_view` (`:1188`), hidden initially (`:843`), shown only while `m_adjustingMap == true` (`:3205, 3594`).
   - Margins `(12, 8, 12, 8)` px (`:831`).
   - Label (`barTxt`, `:832-834`): `"지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」."`
   - Button 1 (`btnCenter`, `:835, 838`): `"레이어를 가운데"` → `KaDrawingStudio::centerSurveyInMap`.
   - Button 2 (`btnDone`, `objectName = "btnAdjustDone"`, `:836-839`): `"조정끝"` → `KaDrawingStudio::endActivateMap`.
2. **Bottom toolbar button #3** (`m_adjustBtn`, `objectName = "btnMapAdjust"`, `src/app/KaDrawingStudio.cpp:1305-1309, 3616-3621`): text `"지도 조정"` when idle, `"조정 끝"` (with space) when active.
3. **Right-click context menu on `ka_map`** (`KaLayoutMenuProvider::createContextMenu`, `src/app/KaDrawingStudio.cpp:337-350`):
   - `"지도 조정"` → `beginActivateMap`
   - `"그린 것 가운데로"` → `centerSurveyInMap`
   - Separator + `"지도조정끝"` → `endActivateMap` (only when `m_studio->isMapAdjusting()` is true, `:344-349`).

### B. C++ does
1. **Starting mode (`beginActivateMap()`, `src/app/KaDrawingStudio.cpp:3195-3211`)**:
   - Guard `!map || !ly || !m_view || !m_toolMoveContent` → status `"먼저 지도 칸을 그리세요."` and return (`:3198-3202`).
   - `m_adjustingMap = true`; `setFastLayoutPreview(true)` lowers `ly->renderContext().setDpi(48.0)` (saving previous preview DPI `96.0`, `:172, 3569-3578`); shows `m_adjustBar`; `syncAdjustButton()` sets button text `"조정 끝"`; selects `map`; sets `m_view->setTool(m_toolMoveContent)` (cursor `Qt::OpenHandCursor`, `src/app/KaDrawingTools.cpp:50`); status `"지도 조정: 드래그로 이동, 휠로 확대. 끝나면 「지도조정끝」."` (`:3210`).
2. **Dragging inside `ka_map` (`KaLayoutMapAdjustTool`, `src/app/KaDrawingTools.cpp:56-100`)**:
   - `layoutPressEvent` (`:56-72`): left button only; `mapAt(event->pos())` resolves overlay maps (`ka_map_above`, `ka_map_numbers`) to base `"ka_map"` (`:167-182`); flushes any pending wheel zoom; records `mMoveStart = event->layoutPoint()`; sets cursor `Qt::ClosedHandCursor`.
   - `layoutMoveEvent` (`:74-83`): sets live pixel preview offset `mMoveItem->setMoveContentPreviewOffset(dx, dy)` and calls `mMoveItem->update()`.
   - `layoutReleaseEvent` (`:85-100`): resets preview offset `(0, 0)`; computes `(dx, dy)` in layout mm; **only if `abs(dx) > 0.2 || abs(dy) > 0.2` mm** (`:93`), calls `mMoveItem->moveContent(-dx, -dy)` (which shifts `ka_map` ground extent without changing scale and pushes a `QgsLayoutItemMap` undo command onto `ly->undoStack()`); restores `Qt::OpenHandCursor`; emits `mapViewChanged()` → `syncScaleDecorations()` (`src/app/KaDrawingStudio.cpp:872-873, 2902-2941`).
3. **Turning the mouse wheel (`KaLayoutMapAdjustTool::wheelEvent` & `flushZoom`, `src/app/KaDrawingTools.cpp:102-165`)**:
   - Requires `mapAt(pos)` non-null and `delta = event->angleDelta().y() != 0` (`:105-109`).
   - **Wheel factor**: `LayerOps::kWheelZoomFactor = 1.25` (`src/core/LayerOps.h:29`). Per event: `step = 1.0 + (1.25 - 1.0) * min(1.0, fabs(delta) / 120.0)` (`1.25` for a full `120` notch); if `Qt::ControlModifier` is held, `step = 1.0 + (step - 1.0) * 0.35` (`1.0875` for a full notch, `:111-114`). `factor = (delta > 0) ? step : (1.0 / step)` (`:115`).
   - Accumulates `mPendingFactor *= factor` at cursor anchor `mPendingPoint = map->mapFromScene(scenePt)` and starts `mZoomTimer` (`220 ms` single-shot, `:51-52, 118-121`).
   - `flushZoom()` (`:133-165`): if `fabs(mPendingFactor - 1.0) < 1e-4`, resets to `1.0` and returns. **Scale floor**: checks `cur = mPendingMap->scale() > 10.0` (`:138-139`; note: if `cur <= 10.0`, neither zoom-in nor zoom-out runs; no upper scale cap in `flushZoom`, though `m_scaleSpin` caps at `5,000,000` at `src/app/KaDrawingStudio.cpp:1062`). Calls `LayoutService::zoomExtentAtAnchor(ext, fx, fy, mPendingFactor)` (`src/core/LayoutService.cpp:1022-1037`: `newW = ext.width() / factor`, keeping ground point at `(fx, fy)` fixed), syncs `ka_map_above` and `ka_map_numbers` extents (`:153-161`), resets `mPendingFactor = 1.0`, emits `mapViewChanged()` → `syncScaleDecorations()`.
4. **During adjustment (`syncScaleDecorations`, `src/app/KaDrawingStudio.cpp:2902-2941`)**:
   - `sc = displayScale(map->scale())` (`max(10, lround(raw))`, `:2879-2883`); updates `m_scaleSpin` (unrounded to standard scale yet), updates `ka_scale` text `"축척 1 : %1"`, updates `syncScaleChips()`, syncs `ka_map_above`, and starts `280 ms` `m_scaleSyncTimer` (`:878, 2940`) → `flushHeavyScaleSync()` (`:2943-2957`: updates `ka_scalebar`, `applyCrsGrid`, `relayoutCoordCallouts`, `syncHeritageNumbers`).
5. **Ending mode (`endActivateMap()`, `src/app/KaDrawingStudio.cpp:3590-3607`)**:
   - `m_toolMoveContent->deactivate()` flushes any pending wheel zoom (`src/app/KaDrawingTools.cpp:124-131`).
   - `setFastLayoutPreview(false)` restores `96.0 DPI` and refreshes `ka_map` (`src/app/KaDrawingStudio.cpp:3580-3587`).
   - Hides `m_adjustBar`; `syncAdjustButton()` restores bottom button text `"지도 조정"` (`:3594-3595, 3618`).
   - If `was == true`:
     - Calls **`snapMapScaleToNice()`** (`:2830-2854`): computes `raw = max(ext.width() / (widthMm / 1000.0), ext.height() / (heightMm / 1000.0))`, snaps **up** via `LayoutService::niceScaleDenominator(raw)` → `StandardScales::snapUp(raw)` (`src/core/StandardScales.cpp:25-39` table: `10, 20, 30, 40, 50, 60, 80, 100, 150, 200, 250, 300, 400, 500, 600, 800, 1000, 1200, 1500, 2000, 2500, 3000, 4000, 5000, 6000, 7500, 10000, 15000, 20000, 25000, 30000, 40000, 50000, 75000, 100000, 150000, 200000, 250000, 500000`; above `500000`, rounds up to multiple of `100000`), calls `map->setScale(nice, true)` and updates `m_scaleSpin` (`:2848-2853`).
     - Calls `markUserComposed()` (`:3598`).
   - Calls `refreshScaleWidgets(true)` (updates `m_scaleSpin`, `ka_scale` label `"축척 1 : <nice>"`, and `syncScaleChips()`), `applyNiceScaleBar(sb)`, `useSelectTool()` (if `was`, setting status `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."`), and `relayoutCoordCallouts()` (if `!m_coordMapPts.isEmpty()`, `:3600-3606`).

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `standardScalesAreOneTable` (snapUp checks) | `tests/test_layout_sheet_rules.cpp:155-165` | pure calls on `StandardScales::snapUp` & `LayoutService::niceScaleDenominator` | `StandardScales::snapUp(raw)`, `LayoutService::niceScaleDenominator(raw)` | `23., 55., 520., 1100., 2300., 0.` and `{12., 23., 37., 487., 1847., 21000.}` | `snapUp(23.) == 30`, `snapUp(55.) == 60`, `snapUp(520.) == 600`, `snapUp(1100.) == 1200`, `snapUp(2300.) == 2500`, `snapUp(0.) == 0`; `nice == snapUp(raw)`, `nice >= raw && nice % 10 == 0` |
| `drawingStudioLayoutAndExport` (map adjust bar & snap) | `tests/test_workflow.cpp:5557-5592` | `MainWindow` + `KaDrawingStudio` | `toggleMapAdjust()`, `map->setScale(1100.)`, `endActivateMap()` | `1100.` during adjust | `isMapAdjusting() == true`, `mapAdjustBar->isVisible() == true`, `btnMapAdjust->text() == "조정 끝"`; after `endActivateMap()`: `isMapAdjusting() == false`, `mapAdjustBar->isVisible() == false`, `btnMapAdjust->text() == "지도 조정"`, `qRound(map->scale()) == 1200` |

### D. Rust today
- **Top bar `m_adjustBar` text and 2 buttons (`"레이어를 가운데"`, `"조정끝"`)** (`crates/strata-app/src/gui_tab_drawing_popups.rs:19-65`): **SAME** — shows `"지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」."`, `"레이어를 가운데"`, and `"조정끝"` only while `map_adjust_active` is `true`.
- **Bottom button label toggle (`"지도 조정"` ↔ `"조정 끝"`) and start status** (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:75`, `crates/strata-app/src/gui_tab_drawing_adjust.rs:58`): **SAME**.
- **End status message** (`crates/strata-app/src/gui_tab_drawing_adjust.rs:53`): **DIFFERENT** — Rust sets `"지도 조정을 끝냈습니다."` whereas C++ `endActivateMap()` calls `useSelectTool()` (`src/app/KaDrawingStudio.cpp:3603-3604, 1896`), setting `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."`.
- **Drag pan & `0.2 mm` threshold** (`crates/strata-app/src/gui_tab_drawing_adjust.rs:30-34`): **DIFFERENT** — pans `sheet_center` immediately on every pixel drag delta without C++'s `0.2 mm` deadzone (`src/app/KaDrawingTools.cpp:93`) and pushes no layout undo command.
- **Wheel zoom factor, batching, and upper cap** (`crates/strata-app/src/gui_tab_drawing_adjust.rs:11-12, 36-46`): **DIFFERENT** — uses `map_wheel_zoom::wheel_mpp_factor_of_frame` (`1.2` per notch, `1/3` with `Ctrl`) applied per frame and clamped to `10.0..=500_000.0`, whereas C++ uses `1.25` per notch (`0.35` with `Ctrl`), `220 ms` timer batching, and no `500,000` hard cap (`m_scaleSpin` allows up to `5,000,000`).
- **Snap-up on mode end (`snap_up`)** (`crates/strata-app/src/gui_tab_drawing_adjust.rs:64-71`, `crates/strata-layout/src/standard_scales.rs:26-44`): **SAME** — snaps `scale_denominator` up to the standard scale table on `end_sheet_map_adjust()`.
- **Map right-click context menu (`"지도 조정"`, `"그린 것 가운데로"`, `"지도조정끝"`)**: **MISSING** — no right-click menu on the sheet map frame.
- **Relayout coordinate callouts on adjust / end**: **MISSING** — coordinate callouts remain stuck at screen pixels (see P2).

---

## P4 — `P4_fit_to_screen`: `화면 맞춤` (`fitToMapCanvas`, `centerOnMapCanvas`, `applyCanvasViewToLayoutMap`)

### A. C++ offered
1. **Bottom toolbar button #4**: `"화면 맞춤"`, tooltip `"지도 화면과 같은 범위·축척으로 지도 칸을 다시 맞춥니다"`, always enabled (`src/app/KaDrawingStudio.cpp:1310-1312`).
2. **Automatic triggers**:
   - Initial sheet creation: `KaDrawingStudio::createOrResizeMap` schedules a `0 ms` `QTimer::singleShot` calling `centerOnMapCanvas()` when `!m_holdTerrainExtent && m_mapCanvas` (`src/app/KaDrawingStudio.cpp:2287-2298`), and `MainWindow::openLayoutDesigner` also calls `m_drawingStudio->centerOnMapCanvas()` on first creation (`src/app/MainWindowExport.cpp:251`, unless `m_hasCoordCallout` is true).
   - Map layer context menu `"이 레이어로 이동"` while studio exists: `MainWindowContextMenus.cpp:258` calls `m_drawingStudio->centerOnMapCanvas()`.
   - **Tab re-activation**: When switching back to an **already-created** `도면` tab (`MainWindow::openLayoutDesigner`, `src/app/MainWindowExport.cpp:204-213`, or clicking the `QTabWidget` tab), `centerOnMapCanvas()` does **NOT** run again — the user's sheet view and scale are preserved.

### B. C++ does
1. **`fitToMapCanvas()` (`src/app/KaDrawingStudio.cpp:3623-3635`)**:
   - Step 1: Guard `!mapItem() || !m_mapCanvas` → status `"먼저 지도 칸을 그리세요."` and return (`:3624-3628`).
   - Step 2: `endActivateMap()` (`:3629`).
   - Step 3: `centerOnMapCanvas()` (`:3630`).
   - Step 4: `markUserComposed()` (`:3631`).
   - Step 5: Overwrites status with `"지도 화면과 같은 범위·축척으로 맞췄습니다. 축척 1 : %1".arg(displayScale(mapItem()->scale()))` (`:3633-3634`).
2. **`centerOnMapCanvas()` (`src/app/KaDrawingStudio.cpp:3664-3706`)**:
   - Step 1: Reads `ext = m_mapCanvas->extent()` and `src = m_mapCanvas->mapSettings().destinationCrs()`, transforming `ext` to `dst` (`map->crs()`, fallback `studioMapCrs`) via `QgsCoordinateTransform` if `src != dst` (`:3667-3679`).
   - Step 2 (Idempotence / no-op guard, `:3682-3691`):
     - `tolerance = max(1.0, max(current.width(), current.height()) * 1e-6)` m.
     - `sameCenter = abs(current.center().x() - ext.center().x()) <= tolerance && abs(current.center().y() - ext.center().y()) <= tolerance`.
     - `sameScale = !(targetScale > 1.0) || abs(map->scale() - targetScale) <= max(1.0, targetScale * 1e-4)` where `targetScale = m_mapCanvas->scale()`.
     - If `sameCenter && sameScale`, returns early without touching QGIS cache or legend.
   - Step 3 (`LayoutService::applyCanvasViewToLayoutMap(map, ext, m_mapCanvas->scale())`, `src/core/LayoutService.cpp:968-978`):
     - Validates `canvasExtent` non-null, finite, `width > 0 && height > 0`.
     - Calls `map->zoomToExtent(canvasExtent)` (which centers on `canvasExtent.center()` while keeping `ka_map`'s paper `rect()`), then if `canvasScale > 1.0 && std::isfinite(canvasScale)` calls `map->setScale(canvasScale, true)`. **Note**: does **NOT** snap `canvasScale` to a standard scale!
   - Step 4 (What else it resets/refreshes, `src/app/KaDrawingStudio.cpp:3694-3706`):
     - `map->invalidateCache()`, `map->refresh()`.
     - `refreshScaleWidgets(true)` → sets `m_scaleSpin` to `displayScale(map->scale())`, updates `ka_scale` text `"축척 1 : %1"`, updates `syncScaleChips()` (all chips uncheck if scale is non-preset).
     - `applyNiceScaleBar(sb)` on `ka_scalebar`.
     - If `!m_coordMapPts.isEmpty()`, `relayoutCoordCallouts()`.
     - Emits `drawingScaleChanged(map->scale())` (updates main window bottom status bar scale box).
     - Status (when called directly, before `fitToMapCanvas` overwrites it): `"지도 화면과 같은 범위·축척으로 맞췄습니다. 표준 축척은 오른쪽 숫자 단추로 고르세요."` (`:3705`).
   - **What it does NOT reset**: Does **not** move or reset `ka_legend`, `ka_title_block`, `ka_north`, `ka_scalebar`, `ka_scale`, `ka_crs`, or paper size! (And does **not** record a layout undo command.)

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `drawingStudioFollowsMapCanvasView` | `tests/test_workflow.cpp:4425-4498` | `MainWindow` + `QgsMapCanvas` (`EPSG:5186`) + `KaDrawingStudio` | `openLayoutDesigner()`, `fitToMapCanvas()`, re-open tab after moving map canvas | canvas extent around `(200000, 500000)` at custom scale | initial studio map center & scale match `canvas->extent().center()` and `canvas->scale()` within tolerance; moving canvas and re-activating existing `도면` tab does **not** overwrite studio map extent; clicking `fitToMapCanvas()` updates studio map center & scale to new canvas view without snapping to preset |

### D. Rust today
- **Initial sheet centering (`already_done_in_rust`, `ORDER-SHEET0.yaml:22`)**: Intentional owner decision — new sheet centers on the survey area (`crates/strata-app/src/gui_tab_drawing_frame.rs:34-49`, `crates/strata-app/src/modal_paper_account.rs:132`) rather than `home_map.viewport`, and re-opening an already-open `도면` tab (`crates/strata-app/src/gui_ribbon_click_b.rs:68-73`) keeps `sheet_center` untouched (**SAME** tab-reactivation preservation).
- **Button #4 `화면 맞춤` (`crates/strata-app/src/gui_tab_drawing_toolbar.rs:99-103`, `crates/strata-app/src/gui_tab_drawing_adjust.rs:75-82`)**:
  - **SAME**: Calls `end_sheet_map_adjust()`, copies `home_map.viewport.center_xy` into `self.drawing.sheet_center`, sets un-snapped `scale_denominator = (view.meters_per_pixel * POINTS_PER_METER).round().max(10.0) as u32`, and shows `"지도 화면과 같은 범위·축척으로 맞췄습니다. 축척 1 : {scale:.0}"`.
  - **DIFFERENT**:
    1. `gui_tab_drawing_toolbar.rs:100-101` unnecessarily calls `self.drawing.sheet.replace_sheet_chrome()` and `self.drawing.sheet.recover_orphaned_items()` on click (C++ `fitToMapCanvas` never touches chrome or legend rects).
    2. Does **not** exit `coord_callout_mode` if active (`gui_tab_drawing_adjust.rs:75-82`).
    3. Does **not** transform `home_map.viewport.center_xy` if CRS ever differed (both use `work_crs` in Rust), has no `5_000_000` upper clamp on `scale_denominator` (`u32`), and does **not** relayout `coord_callouts` (see P2).

---

## P5 — `P5_moving_items`: Moving, Selecting, Resizing, Deleting, Undo/Redo, Limits, `선택 항목 크기 %`, and Automatic Placement Rules *(Owner Priority)*

### A. C++ offered
1. **Selection & move/resize tool (`m_toolSelect = new QgsLayoutViewToolSelect(m_view)`, `src/app/KaDrawingStudio.cpp:865`)**:
   - Active **by default** as soon as a layout is attached (`src/app/KaDrawingStudio.cpp:618-625`, `createOrResizeMap` `:2303`, `finishPlace` `:2063-2065`, `endActivateMap` `:3603-3604`), and re-activated by bottom toolbar button #5 `"항목 이동"` (`useSelectTool`, `:1313-1316, 1891-1897`), whenever neither `m_toolMoveContent` (`지도 조정`) nor `m_toolCoordPoint` (`좌표점`) is active.
2. **Which layout items the user can select, move, and resize**:
   - **Every unlocked `QgsLayoutItem` on `user_sheet`** (none are locked — `applyStandardChromePositions` explicitly calls `setLocked(false)` on `ka_scalebar` `:2788`, `ka_scale` `:2805`, `ka_crs` `:2812`, `ka_north` `:2819`):
     1. `ka_map` (`QgsLayoutItemMap`, `:2268-2279`) — **movable AND resizable** via corner/edge handles (`syncAboveLabelsMap` `:2195-2201, 2239` and `m_heritageNumbers` follow `base->pos()` and `base->rect()` on release `:3411`).
     2. `ka_legend` (`QgsLayoutItemLegend`, `:2353-2363`) — **movable AND width-resizable** (changing width triggers `120 ms` `ka_legend_flow_timer` → `LayoutService::flowSheetLegend`, which reflows columns and fits height while keeping user's `pos()` and `width`, `src/core/LayoutService.cpp:1467-1471, 1533-1535`).
     3. `ka_title_block` (`QgsLayoutItemLabel`, `src/app/KaTitleBlock.cpp:31, 85-108`) — **movable AND resizable**.
     4. `ka_north` (`QgsLayoutItemPicture` or fallback `QgsLayoutItemLabel`, `src/app/KaDrawingStudio.cpp:2429-2457`) — **movable AND resizable**.
     5. `ka_scalebar` (`QgsLayoutItemScaleBar`, `:2480-2496`) — **movable AND resizable**.
     6. `ka_scale` (`QgsLayoutItemLabel`, `:2538-2555`) — **movable AND resizable**.
     7. `ka_crs` (`QgsLayoutItemLabel`, `:2513-2530`) — **movable AND resizable**.
     8. `ka_coord_box_<tag>`, `ka_coord_let_<tag>`, `ka_coord_arr_<tag>`, `ka_coord_head_<tag>`, and `ka_coord_frame` (`src/app/KaDrawingCoordCallout.cpp:74-133`, `src/app/KaDrawingStudio.cpp:1874-1885`) — each is an unlocked `QgsLayoutItem` selectable and movable individually.
3. **Keyboard shortcuts on `KaDrawingStudio`**:
   - **Arrow keys (`Left`/`Right`/`Up`/`Down`)**: Handled natively by QGIS `QgsLayoutView` / `QgsLayoutViewToolSelect` (`m_view` has `Qt::StrongFocus`, `src/app/KaDrawingStudio.cpp:850`), nudging selected `QgsLayoutItem`s on the paper (`Shift` = larger step, `Ctrl` = fine step in QGIS `QgsLayoutView`) and pushing undo commands onto `ly->undoStack()`.
   - **`Delete` / `Backspace`**: Routed from `MainWindow::routeEditKeyToActiveStudio(true)` (`src/app/MainWindow.cpp:1184-1208`), `KaDrawingStudio::eventFilter` (`src/app/KaDrawingStudio.cpp:3357-3360, 3465-3471`), and `KaDrawingStudio::keyPressEvent` (`:3561-3565`) to `KaDrawingStudio::handleDeleteKey()` (`:3512-3522`).
   - **`Ctrl+Z` (Undo)**: Routed from `MainWindow::undoLastAction()` → `routeEditKeyToActiveStudio(false)` (`src/app/MainWindowUndo.cpp:302-309`, `src/app/MainWindow.cpp:1184-1208`), `eventFilter` (`src/app/KaDrawingStudio.cpp:3345-3350, 3460-3464`), and `keyPressEvent` (`:3549-3554`) to `KaDrawingStudio::handleUndoKey()` (`:3524-3532`).
   - **`Ctrl+Y` (Redo)**: Routed from `MainWindow::redoLastAction()` (`src/app/MainWindowUndo.cpp:312-323`), `eventFilter` (`src/app/KaDrawingStudio.cpp:3351-3355`), and `keyPressEvent` (`:3555-3560`) to `KaDrawingStudio::handleRedoKey()` (`:3534-3546`).
   - **Middle-mouse button drag on `m_view`** (`src/app/KaDrawingStudio.cpp:3425-3456`): pans the layout viewport (`panLayoutView`, `m_keepPaperCentered = false`, `:3443-3444`, `2 px` Manhattan threshold `:3442`).
4. **Right-panel control `선택 항목 크기`** (`m_decorationSize`, `QDoubleSpinBox`, `objectName = "decorationSize"`, `src/app/KaDrawingStudio.cpp:1141-1156`):
   - In card 3 (`도면 정보`), row `"선택 항목 크기"` + `QDoubleSpinBox` (`25..200`, `0` decimals, step `10`, suffix `" %"`, default `100`, `keyboardTracking = false`).
   - Tooltip verbatim (`:1152`): `"용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다."`
   - **Enabled rule** (`updateInspector`, `:226-231, 3074-3080`): **Disabled (`false`) initially and whenever selection count != 1 or selected item is not one of the 4 sheet decorations** (`ka_scalebar`, `ka_scale`, `ka_crs`, `ka_north`). When enabled, displays `decorationSizePercent(item)` (`customProperty("ka_hgis/chrome_size_percent", 100.0)`).

### B. C++ does
1. **Dragging / resizing items & paper limits / snapping / guides**:
   - `QgsLayoutViewToolSelect` moves and resizes items in layout mm (`QgsLayout` scene units) and pushes `QUndoCommand`s onto `ly->undoStack()->stack()` (`src/app/KaDrawingStudio.cpp:3364-3367, 3386-3397`).
   - **Limits (does an item stay on the paper?)**: **NO clamp during drag!** The desk scene rect is `(-1.5 W, -1.5 H, 4.0 W, 4.0 H)` around the page (`src/app/KaDrawingStudio.cpp:1918-1920`). The user can drag **any** item — specifically `ka_legend` (`tests/test_layout_sheet_rules.cpp:252-269`) — partially or completely outside the white paper onto the desk! When the mouse button is released after moving/resizing (`src/app/KaDrawingStudio.cpp:3385-3398`), if `ly->undoStack()->stack()->index() != pressUndoIndex`, `KaDrawingStudio` calls `markUserComposed()`, and if `ka_legend` was selected or its center is outside `(0, 0, m_paperW, m_paperH)`, sets `legend->setCustomProperty("ka_hgis/legend_kept_on_page", true)` (`:3392-3396`) so `relinkDecorations()` will **never** pull it back onto the page (`:2690-2696`)!
   - **Snapping / guides**: Uses QGIS `QgsLayout` default snapper (`QgsLayout::snapper()`, item bounding-box/page-edge alignment guides native to `QgsLayoutViewToolSelect`; `KaDrawingStudio` adds no custom snap grid for layout items).
2. **`Delete` / `Backspace` chain (`handleDeleteKey()`, `src/app/KaDrawingStudio.cpp:3512-3522`)**:
   - Step 1: In `MainWindow::routeEditKeyToActiveStudio` (`src/app/MainWindow.cpp:1184-1208`), if `editingText()` is true (`QLineEdit`/`QTextEdit`/`QPlainTextEdit`/`QAbstractSpinBox` or editable `QComboBox` has focus), returns `false` so text editing gets the key.
   - Step 2: If `m_layerTree` (or its viewport) has focus (`src/app/KaDrawingStudio.cpp:3513-3516`), calls `removeSelectedLayers()` (`:3259-3306`; delegates to `MainWindow::removeLayersFromTree(m_layerTree)` when inside `MainWindow`, else prompts `"선택한 레이어 %1개를 도면/프로젝트에서 삭제할까요?"` or shows `"삭제할 레이어를 먼저 선택하세요."`).
   - Step 3: Else if `isPlacingCoordPoint()` (`:3518-3519`), calls `undoLastCoordCallout()`.
   - Step 4: Else calls `deleteSelectedItems()` (`:3213-3257`):
     - Guards `!focusWidget is QLineEdit/QAbstractSpinBox` (`:3214-3216`).
     - Iterates `ly->selectedLayoutItems(false)`, skipping `QgsLayoutItemPage` (`:3219-3223`).
     - Any `ka_coord_*_<tag>` collects `<tag>` into `coordTags`, removes matching ground points from `m_coordMapPts`, and calls `relayoutCoordCallouts()` (`:3225-3248`).
     - If `id == "ka_legend"`, sets **`m_legendRemoved = true`** (`:3233-3234`) so editing legend title/font controls will **not** recreate `ka_legend` until `[범례]` is clicked again (`:2352`)!
     - Calls `ly->removeLayoutItem(it)` for every other selected item (`ka_title_block`, `ka_north`, `ka_scalebar`, `ka_scale`, `ka_crs`, even `ka_map`, `:3235`).
     - Calls `updateInspector(nullptr)` and `syncSheetTiles()` (`:3249-3250`).
     - Status: if `n > 0`, `"선택한 항목 %1개를 지웠습니다."`; else `"지울 항목을 먼저 선택하세요."` (`:3252-3256`).
3. **`Ctrl+Z` / `Ctrl+Y` chain (`handleUndoKey` / `undoLastChange` / `handleRedoKey`, `src/app/KaDrawingStudio.cpp:3476-3546`)**:
   - `handleUndoKey()` (`:3524-3532`): if `m_layerTree` has focus, delegates to `MainWindow::undoMapAction()`; else calls `undoLastChange()`.
   - `undoLastChange()` (`:3476-3508`):
     - Step 1: If `QLineEdit` or `QAbstractSpinBox` has focus, returns (`:3477-3479`).
     - Step 2: If `isPlacingCoordPoint()` or `m_placeUndo.last()` starts with `"ka_coord_"`, calls `undoLastCoordCallout()` and returns (`:3480-3484`).
     - Step 3: If `ly->undoStack()->stack()->canUndo()` (covers item moves, resizes, map content drags, north-size spinbox `"방위표 크기"`, decoration-size spinbox `"도면 항목 크기"`), calls `undo()`, `updateInspector(nullptr)`, `syncSheetTiles()`, and shows status `"한 단계 되돌렸습니다."` (`:3486-3493`).
     - Step 4: Else pops `m_placeUndo` (`ka_legend`, `ka_title_block`, `ka_north`, `ka_scalebar`, `ka_scale`, `ka_crs`, `ka_map`, `ka_coord_frame`), removes that item from `ly`, updates inspector/tiles, and shows `"방금 넣은 항목을 되돌렸습니다."` (`:3494-3505`).
     - Step 5: Else shows `"되돌릴 것이 없습니다."` (`:3506-3508`).
   - `handleRedoKey()` (`:3534-3546`): if `m_layerTree` has focus (`eventFilter` `:3351-3355`), calls `MainWindow::redoMapAction()`; else if `ly->undoStack()->stack()->canRedo()`, calls `redo()`, updates inspector/tiles, and shows `"다시 실행했습니다."`; else shows `"다시 실행할 것이 없습니다."`.
4. **`선택 항목 크기 %` (`resizeSelectedDecoration(double percent)`, `src/app/KaDrawingStudio.cpp:3032-3060`)**:
   - Requires `selected.size() == 1 && isSheetDecoration(selected.first())` (`ka_scalebar`, `ka_scale`, `ka_crs`, `ka_north`, `:226-231, 3035-3036`).
   - Reads `previous = decorationSizePercent(item)` (default `100.0`), computes `ratio = percent / previous`, records `position = item->pos()` and `size = item->rect().size()` (`:3038-3041`).
   - Opens undo command `item->beginCommand("도면 항목 크기")`, stores `ka_hgis/chrome_size_percent = percent` (`:3042-3043`):
     - If `ka_scalebar`: calls `applyNiceScaleBar(bar)` — which multiplies **`unitsPerSegment`** by `percent / 100.0` (`:2874`), scales bar height `2.4 * factor` mm, `labelBarSpace = 1.0 * factor` mm, `boxContentSpace = 0.6 * factor` mm, `Malgun Gothic 7.0 * factor pt` (`compactSheetScaleBar`, `:237-252`), and `resizeToMinimumWidth()` (`:2876`) — then resizes height to `size.height() * ratio` (`:3044-3047`).
     - Else (`ka_scale`, `ka_crs`, `ka_north`): if `QgsLayoutItemLabel`, multiplies `font.pointSizeF()` by `ratio`; resizes item `size * ratio` (`:3048-3055`).
   - Restores `item->attemptMove(QgsLayoutPoint(position))`, ends undo command `"도면 항목 크기"`, updates item and inspector (note: if `ka_north` was resized, `m_northSize` updates to `item->rect().width()` at `:3081-3085`!).
5. **Automatic placement (`applyStandardChromePositions`, `relinkDecorations`, `resetPaper`) — WHEN it runs and WHETHER it overrides user moves (`src/app/KaDrawingStudio.cpp:473-554, 2680-2828`)**:
   - **Standard chrome (`ka_map`, `ka_scalebar`, `ka_scale`, `ka_crs`, `ka_north`)**:
     - Geometry (`defaultMapRect` `:1350-1356` and `LayoutService::standardSheetChrome` `src/core/LayoutService.cpp:895-966`):
       - `kFieldEdgeMm = 10.0 mm`, `kFieldBottomChromeMm = 38.0 mm` (`src/app/KaDrawingStudio.h:68-71`): `defaultMapRect = (10.0, 10.0, max(40.0, W - 20.0), max(40.0, H - 48.0))` mm. (If user narrowed `ka_map` below `0.65 * page.width()`, `equalFullSheetMapRect` `src/core/LayoutService.cpp:876-893` equalizes side/top margins while keeping `bottomKeep = max(38.0, page.bottom() - requestedMap.bottom())`.)
       - `standardSheetChrome` (`src/core/LayoutService.cpp:901-939`): `kChromeH = 33.0 mm`, `kGap = 7.0 mm`, `kMargin = 8.0 mm`, `kMinMapH = 40.0 mm`, `kBarH = 12.0 mm`, `kLabelH = 7.0 mm`, `kLabelGap = 2.5 mm`, `kNorth = 20.0 mm`, `kCrsH = 7.0 mm`, `kCrsW = 58.0 mm`, `kCrsGap = 6.0 mm`. `rowTop = map.bottom() + 7.0`; `north = (map.right() - 20.0, rowTop, 20.0, 20.0)`; `crs = (north.left() - 64.0, rowTop + 2.5, 58.0, 7.0)`; `barW = min(88.0, max(48.0, map.width() * 0.38), max(32.0, crs.left() - 8.0 - map.left()))` (then refined to actual `scaleBarWidthMm(segM, segs, denom)` clamped to `32.0..maxW` at `src/app/KaDrawingStudio.cpp:2772-2775`); `scaleBar = (map.left(), rowTop, barW, 12.0)`; `scaleLabel = (bar.left(), bar.bottom() + 2.5, max(36.0, bar.width()), 7.0)`.
     - **When `applyStandardChromePositions()` runs**: inside `ensureStandardDecorations()` (`:2725`), which is called from `createOrResizeMap` (`:2301`), `placeTerrain3dPicture` (`:1401`), `applyFieldEdge` (`:444`), and `resetPaper` (`:551`).
     - **Does it overrule user moves?** Checked per item by `needsChromePlace(it)` (`src/app/KaDrawingStudio.cpp:2760-2763`):
       `!it->customProperty("ka_hgis/chrome_placed", false).toBool() || mapChanged || !pageBounds.contains(itemPaperRect(it))` (where `pageBounds = page.adjusted(-1.0, -1.0, 1.0, 1.0)`).
       - Once initially placed, `ka_hgis/chrome_placed` is `true` (`:2781, 2803, 2810, 2817`), so **ordinary layer toggles, scale changes, legend edits, north style switches (`placeNorth` preserves `placed` at `:2417, 2461`), and `화면 맞춤` NEVER move an item the user kept within `pageBounds`!**
       - It **DOES** re-place chrome items when: (a) the user changes paper size/orientation (`resetPaper` explicitly resets `ka_hgis/chrome_placed = false` on all 4 decorations at `:518-520`), (b) `mapChanged` is true (`ka_map` differed from `equalized` by `> 0.5 mm`, `:2739-2743`), or (c) that chrome item's rect went outside `pageBounds` (`-1 mm` margin around paper, `:2762`).
   - **Non-chrome items (`ka_legend`, `ka_title_block`, `ka_coord_*`) on `resetPaper(..., preserveExisting=true)` (`src/app/KaDrawingStudio.cpp:514-545`)**:
     - If an item's center was inside `oldPageRect.adjusted(-2, -2, 2, 2)`, it shifts relatively (`if pos.y > oldH * 0.55` adds `dh`; `if pos.x > oldW * 0.5` adds `dw`) and clamps onto `[4.0, max(4.0, newPaper - size - 4.0)]` (`:523-539`).
     - If `ka_legend`'s center was intentionally parked **outside** `oldPageRect`, loop `:524-525` skips it (`continue`), leaving it outside! Only if `legend`'s top ended below `page.bottom() + 2.0` after `mapChanged || anyChromeOffPage` does `:2821-2826` reset `legend_kept_on_page = false` and pull it to `defaultItemRect(kIdLegend)`.

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `legendFlowLeavesAnOutsidePlaceWhereItIs` | `tests/test_layout_sheet_rules.cpp:254-269` | `QgsProject` + `QgsPrintLayout` (`297 × 210 mm`) + `QgsLayoutItemLegend` (`60 × 40 mm`) | `legend->attemptMove(at)` then `LayoutService::flowSheetLegend(legend)` | `at` in `{(280., 195.), (320., 40.), (-90., -30.)}` | `legend->pos() == at`, `legend->positionWithUnits().x() == at.x()`, `legend->positionWithUnits().y() == at.y()` |
| `drawingStudioLayoutAndExport` (delete / undo / decoration resize) | `tests/test_workflow.cpp:5594-5675` | `MainWindow` + `KaDrawingStudio` | select `ka_legend` → `handleDeleteKey()` → `applyLegendSettings()` → `beginPlaceLegend()`; select `ka_scalebar` → `m_decorationSize->setValue(150.)` → `handleUndoKey()` → `handleRedoKey()` | `Delete` on `ka_legend`; `150.0 %` on `ka_scalebar` | `ka_legend` removed; editing `m_legendTitle` while `m_legendRemoved == true` does **not** recreate `ka_legend`; `beginPlaceLegend()` recreates it; `m_decorationSize` disabled when nothing selected, enabled when `ka_scalebar` selected; `150 %` multiplies `unitsPerSegment` by `1.5`; `handleUndoKey()` restores `100 %`; `handleRedoKey()` re-applies `150 %` |

### D. Rust today
- **Draggable items (`crates/strata-app/src/tab_drawing.rs:35-42`, `crates/strata-app/src/gui_tab_drawing_items.rs:47-64`)**: **DIFFERENT** — drags the 6 `SheetItem`s (`Legend`, `TitleBlock`, `NorthArrow`, `ScaleBar`, `ScaleText`, `CrsLabel`) whenever `!map_adjust_active && !coord_callout_mode` (hover draws `#B5573A` outline and sets `CursorIcon::Move`), storing `item_offsets_mm: BTreeMap<SheetItem, [f32; 2]>`. **Cannot** drag `ka_map` or `CoordCallout` boxes.
- **Click-to-select & corner/edge resize handles**: **MISSING** — `move_sheet_items` uses `Sense::drag()` (`crates/strata-app/src/gui_tab_drawing_items.rs:54`) with zero selection state (`selected_item: None`) and zero resize handles on any item (including `Legend` width and `ka_map`).
- **Paper boundary limit (`kept_on_paper`, `crates/strata-app/src/gui_tab_drawing_items.rs:14-17, 28, 60`)**: **DIFFERENT** — Rust clamps **every** dragged item (including `SheetItem::Legend`) strictly inside `f.sheet_rect`, making it impossible to place the legend beside/outside the paper as required by `legendFlowLeavesAnOutsidePlaceWhereItIs` (`tests/test_layout_sheet_rules.cpp:254-269`).
- **Arrow keys nudge**: **MISSING** — no arrow-key handler for sheet items (`crates/strata-app/src/gui_shell.rs`, `crates/strata-app/src/gui_tab_drawing_items.rs`).
- **`Delete` / `Backspace` on `MainTab::Drawing`**: **MISSING** — `StrataAppState::delete_selected()` (`crates/strata-app/src/app_state_layer_remove.rs:109-112`) immediately executes `if self.active_tab != MainTab::Map { return; }`. Neither sheet items, nor `ka_legend`, nor `coord_callouts`, nor selected layers in the drawing tab's left tree can be deleted with `Delete`.
- **`Ctrl+Z` / `Ctrl+Y` on `MainTab::Drawing`**: **DIFFERENT (wrong target)** — `crates/strata-app/src/gui_shell.rs:155-160` unconditionally calls `self.undo_map_action()` / `self.redo_map_action()` even when `active_tab == MainTab::Drawing`, undoing map geometry edits instead of sheet item moves/placements/resizes!
- **`선택 항목 크기 %` (`crates/strata-app/src/gui_tab_drawing_right_b.rs:84-95`, `crates/strata-app/src/gui_tab_drawing_chrome.rs:31-37`)**: **DIFFERENT / partially FAKE**:
  1. Always enabled (`gui_tab_drawing_right_b.rs:86-91`) instead of disabled until 1 decoration is selected (`src/app/KaDrawingStudio.cpp:1151, 3076-3078`).
  2. One global `self.drawing.decoration_size_percent: u32` (`crates/strata-app/src/tab_drawing.rs:11`) instead of per-item `ka_hgis/chrome_size_percent`.
  3. **Only read by `north_rect`** (`crates/strata-app/src/gui_tab_drawing_chrome.rs:31-36`, clamped to `12.0..=36.0 px`)! Changing `decoration_size_percent` has **zero effect** on `ScaleBar`, `ScaleText`, or `CrsLabel` (`crates/strata-app/src/gui_tab_drawing_decoration.rs:35-165` never reads `decoration_size_percent` — **FAKE** for 3 of the 4 items named in its own tooltip!).
- **Automatic placement vs. user offsets (`item_offsets_mm`)**: **DIFFERENT** — `item_offsets_mm` (`crates/strata-app/src/tab_drawing.rs:21`) is a relative mm delta added on top of the dynamic auto-computed rect (`gui_tab_drawing_items.rs:27-43`), so:
  1. When `legend_rect` or `chrome` shifts (e.g., legend width/height changes as layers are checked, or scale bar width changes with scale), a user-moved item shifts by the auto-layout delta instead of staying at fixed paper `(x_mm, y_mm)`!
  2. When paper size/orientation changes (`set_paper_and_orientation` / `reset_paper_mm`, `crates/strata-app/src/tab_drawing.rs:89-93`, `crates/strata-layout/src/sheet_drawing.rs:144-178`), `item_offsets_mm` is **not** reset for chrome items nor quadrant-shifted for `Legend`/`TitleBlock` (and `DrawingSheet` has two competing chrome rect calculators: `replace_sheet_chrome()` vs `standard_chrome()`, `crates/strata-layout/src/sheet_drawing.rs:204-236`).

---

## P6 — `P6_legend`: The Legend (`ka_legend`, `LayoutService::tuneSheetLegend` & `flowSheetLegend`) *(Owner Priority)*

### A. C++ offered
1. **Right-panel card 1 (`도면 항목`) controls** (`src/app/KaDrawingStudio.cpp:902-990`):
   - **Tile button `[범례]`** (`m_legendTile`, `objectName = "legendTile"`, `:908-918`): text `"범례"`, icon `"layout_legend"`, tooltip verbatim `"범례를 용지에 넣습니다. 넣은 뒤 끌어 옮깁니다"`. In stock look, non-checkable; in `paperLook`, `syncSheetTiles()` (`:1572-1590`) marks it checked (oat highlight) while `ka_legend` exists on `user_sheet`, but clicking it always calls `beginPlaceLegend()`.
   - **Tile button `[표제란]`** (`m_titleTile`, `objectName = "btnTitleBlock"`, `:939-948`): text `"표제란"`, icon `"new"`, tooltip verbatim `"도면명·조사명·유적명·축척·작성일 상자를 넣습니다. 조사구역의 조사명·유적명을 씁니다. 필요할 때만 넣습니다"` → `placeTitleBlock()` (detailed in P7; in `paperLook`, oat-highlighted while `ka_title_block` exists, `:1589`).
   - **Label `"범례 제목"` + `QLineEdit` `m_legendTitle`** (`:952-957`): placeholder `"제목을 입력하세요"`, default text `"범례"`, no tooltip, `textChanged` → `applyLegendSettings`.
   - **Font row (`:958-974`)**: label `"글자"`, `QSpinBox` `m_legendFont` (`5..24`, suffix `" pt"`, default `10`, `valueChanged` → `applyLegendSettings`), `QCheckBox` `m_legendBold` (`"B"`, default **checked** `true`, `toggled` → `applyLegendSettings`), `QCheckBox` `m_legendItalic` (`"I"`, default **unchecked** `false`, `toggled` → `applyLegendSettings`). All always enabled.
   - **Heritage names checkbox `m_heritageNames`** (`objectName = "heritageNamesCheck"`, `:975-989`): text `"1:5,000 이하에서 유적 이름으로 표시"`, tooltip verbatim `"켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다"`, default `false` (stored on layout custom property `HeritageLayoutNumbers::namesPropertyKey()`).

### B. C++ does
1. **Default sheet vs. `[범례]` click (`autoPlaceDefaultSheet` vs `beginPlaceLegend`, `src/app/KaDrawingStudio.cpp:1358-1361, 1606-1616, 2342-2390`)**:
   - **C++ default**: `autoPlaceDefaultSheet()` calls `createOrResizeMap` → `ensureStandardDecorations()` (`:2705-2727`), which places `ka_north`, `ka_scalebar`, `ka_scale`, `ka_crs` — **NOT** `ka_legend`! In C++, `ka_legend` is created only when the user clicks `[범례]` (`beginPlaceLegend`) or edits `m_legendTitle`/`m_legendFont`/`m_legendBold`/`m_legendItalic` while `m_legendRemoved == false` (`:2350-2363`). *(Note owner's 2026-11-15 instruction `ORDER-SHEET0.yaml:10`: 「범례는 레이어가 선택일때 자동으로 배치되게하라」.)*
   - **What clicking `[범례]` does (`beginPlaceLegend`, `:1606-1616`)**:
     - Sets `m_legendRemoved = false`, calls `endActivateMap()`, sets `m_placeKind = PlaceKind::Legend`, calls `applyLegendSettings()` (creates `ka_legend` if absent, or updates title/fonts and reflows if already present — **never removes/toggles it!**), pushes `"ka_legend"` onto `m_placeUndo`, calls `markUserComposed()`, calls `finishPlace()` (switches to `m_toolSelect` and **selects `ka_legend`** on a `0 ms` timer, `:2061-2099`), and shows status `"범례를 넣었습니다. 끌어 옮기세요."` (`:1615`).
     - Removing `ka_legend` is done by selecting it and pressing `Delete`/`Backspace` (`deleteSelectedItems` sets `m_legendRemoved = true`, `:3233-3235`).
2. **Default legend placement & automatic re-placement rules**:
   - **Default rect** (`defaultItemRect(kIdLegend)`, `src/app/KaDrawingStudio.cpp:2327-2328`): `(mapR.right() - 52.0, mapR.top() + 6.0, 48.0, 74.0)` mm (i.e. `4 mm` inside top-right corner of `ka_map`, initial width `48.0 mm`, initial height `74.0 mm` before `flowSheetLegend` shrinks/grows height to content). Frame enabled, white opaque background enabled (`:2355-2356`).
   - **When legend updates / moves**:
     - **Layer checked or unchecked**: `QgsLayerTree` visibility signal → `syncMapFromLayers()` (`80 ms` `m_layerSyncTimer`, `:880-883, 2621-2625`) → `flushLayerSync()` (`:2627-2678`): removes unchecked nodes from `legend->model()`, or if a newly checked non-omitted layer is missing calls `LayoutService::tuneSheetLegend(legend)` + `m_heritageNumbers.applyLegend(legend)`, else calls `LayoutService::flowSheetLegend(legend)`. **Position `legend->pos()` and width are preserved!**
     - **Scale or map extent changed**: `syncScaleDecorations()` → `280 ms` `m_scaleSyncTimer` (`:876-879, 2939-2940`) → `flushHeavyScaleSync()` (`:2943-2957`) → `syncHeritageNumbers()` (`:2959-3014`) → `applyHeritageNumberChrome()` (`:2992-3008`), updating numbered heritage rows and reflowing height while **preserving `pos()` and width**.
     - **Paper changed (`resetPaper`, `:523-540, 2688-2696, 2821-2827`)**: if `ka_legend` center was on the old page, shifts right/down with `(dw, dh)` when in right/bottom half and clamps to `4.0 mm` page margin; if user parked `ka_legend` outside the page (`ka_hgis/legend_kept_on_page == true`), keeps it outside unless its top exceeds `page.bottom() + 2.0 mm` after a map/chrome change.
3. **Which layers get a row, in what order, and with what symbols (`LayoutService::tuneSheetLegend`, `src/core/LayoutService.cpp:1370-1455, 1566-1627`, and `HeritageLayoutNumbers::applyLegend`)**:
   - **Source & order**: `legend->setSyncMode(Qgis::LegendSyncMode::Manual)` + `resetManualLayers(VisibleLayers)` (`:1586-1589`) clones the project layer tree order (top-to-bottom as shown in the left layer panel), plus `addMissingSheetLegendLayers` (`:1425-1455`) appends any checked layers on `ka_map`, `ka_map_above`, `ka_map_numbers`, or visible heritage vector layers. Group headers from the project tree keep `Group`/`Subgroup` fonts unless flattened by QGIS/heritage rules.
   - **Checked vs. inside map frame**:
     - Must be **checked** in `project->layerTreeRoot()` (`projectLayerChecked`, `:1398-1405`) and on `ka_map` / `ka_map_above` / `ka_map_numbers` / heritage (`onLinkedMap`, `:1407-1423`).
     - **Survey & user layers**: Shown whenever **checked** (even if empty or currently outside the map frame).
     - **Geology / soil / categorized layers & heritage sites**: Filtered to **features inside the map frame** (`hitTest` / `HeritageLayoutNumbers::applyLegend`: `LayoutService.cpp:1473-1484, 1568-1570`; numbered heritage badges `1..N` list only sites visible inside `ka_map` in badge-number order, or when `1:5,000 이하에서 유적 이름으로 표시` is active at `scale <= 5000`, list only `3.0 × 3.0 mm` kind squares for datasets present in `ka_map`, `already_done_in_rust`).
   - **Omitted layers (`omitSheetLegendLayer` & `sheetLegendOmitsLayerName`, `src/core/LayoutService.cpp:1370-1396`)**:
     1. `"흙토람 그림"` (`name.contains("흙토람 그림")`)
     2. `"지질 음영"` (`name == "지질 음영"`)
     3. `"위성"`, `*VWorld 위성*`, `Google 위성*`
     4. **All cadastral layers**: `name.startsWith("지적")` or contains `"지적 본번"` / `"지적 부번"` (`:1377-1378`)
     5. Geology structure classes omitted by `GeologyMapService::omitFromSheetLegend(name)` and `pruneStructureLegend(vl)` (`:1388, 1577`)
     6. Any layer with `customProperty("ka_hgis/omit_sheet_legend") == true` (`:1389`)
     7. Any live tile/WMS layer with `providerType()` `"wms"` or `"xyz"` (`:1392-1393`)
     8. `m_blankMapLayer` (`"layout_blank"`, `src/app/KaDrawingStudio.cpp:2105, 2662`)
   - **Symbols of each row**:
     - Vector survey/reference layers: QGIS `QgsSymbol` of the layer renderer (polygon fill+stroke box, line stroke, marker symbol, or categorized sub-nodes for geology/soil).
     - DEM / DSM elevation raster (`DemColorRampLegend`, `src/core/DemColorRampLegend.cpp:1-226`, `src/core/LayoutService.cpp:1610-1625`): horizontal elevation colour-ramp bar with min/max metre labels (if legend width collapsed `<= 1.1 mm`, repaired to `55.0 × 40.0 mm` at `:1617-1621`).
     - Heritage sites (`HeritageLayoutNumbers::applyLegend`, `src/core/HeritageSiteLegend.cpp:1-75`): circular badge with dataset colour, `0.15 mm` black border, bold number `1..N` + site name (or `3.0 mm` dataset colour square + dataset kind name when names mode is active at `<= 1:5,000`).
4. **Fonts, columns, wrapping, and size limits (`applyLegendSettings` `src/app/KaDrawingStudio.cpp:2365-2376` & `LayoutService::flowSheetLegend` `src/core/LayoutService.cpp:1459-1537`)**:
   - **Title & fonts**:
     - `Title`: `Malgun Gothic`, size `pt` (`5..24`, default `10 pt`), weight **`Bold` if `B` checked else `Normal`**, italic = `I` checked (`:2368-2369, 2372`).
     - `Group` and `Subgroup`: `Malgun Gothic`, size `pt`, weight **always `Normal`**, italic = `I` checked (`:2370-2371, 2373-2374`).
     - `SymbolLabel` (each row's label): `Malgun Gothic`, size **`max(5, pt - 1)` pt**, weight **`Normal`**, italic **`false`** (`:2375-2376` — neither `B` nor `I` is applied to `SymbolLabel`!).
   - **Columns & wrapping (`flowSheetLegend`, `src/core/LayoutService.cpp:1491-1536`)**:
     - Debounced by `120 ms` single-shot `ka_legend_flow_timer` when `legend->rect().width()` changes by `> 0.01 mm` or model rows/hit-test change (`:1463-1485`).
     - `resizeToContents = false`, `splitLayer = true`, `equalColumnWidth = true` (`:1496-1498`).
     - Inner margin `boxSpace = 1.0 mm`, inter-column gap `columnSpace = 1.5 mm`, symbol-label margins `Left = 1.0 mm`, `Right = 0.4 mm` (`:1501-1504`).
     - `points = max(5.0, styleFont(SymbolLabel).pointSizeF())`; **`minColumn = 32.0 * points / 9.0` mm** (`:1506-1509`, i.e. `32.0 mm` at `pt = 10` since `SymbolLabel = 9 pt`).
     - `available = max(1.0, width - 2.0 * boxSpace)` (`width - 2.0 mm`); `rows` = total `layerLegendNodes` across all layers (`:1510-1513`).
     - Initial `columns = clamp(int((available + 1.5) / (minColumn + 1.5)), 1, max(1, rows))` (`:1514-1515`).
     - Loop (`:1519-1528`): `columnWidth = (available - (columns - 1) * 1.5) / columns`; **`wrap = max(1.0, columnWidth - symbolSize.width() - 1.5)` mm** (`setAutoWrapLinesAfter(wrap)`); measures `QgsLegendRenderer(model, settings).minimumSize(&context)` and decrements `columns` until `measured.width() <= width + 0.1` or `columns == 1`.
     - **Size limits**: Keeps user's `width` (`> 0 mm`, default `48.0 mm`, user-resizable via handles with no upper `0.42` cap!) and user's `position = legend->pos()`, and sets height to **`measured.height() + 0.5` mm** (`:1533-1535`). Before PDF export, `settleSheetLegendsForExport` (`:1539-1564`) paints a scratch `8 × 8` image, runs `waitForHitTestBlocking()`, and runs `flowSheetLegend(legend)`.

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `legendFlowLeavesAnOutsidePlaceWhereItIs` | `tests/test_layout_sheet_rules.cpp:254-269` | `QgsPrintLayout` (`297 × 210 mm`) + `QgsLayoutItemLegend` (`60 × 40 mm`) | `LayoutService::flowSheetLegend(legend)` | positions `(280, 195)`, `(320, 40)`, `(-90, -30)` | `legend->pos() == at` for all 3 outside/edge positions |
| `numberBadgesStayOnPaperAndOffLegend` | `tests/test_layout_sheet_rules.cpp:185-227` | `153` heritage points + `ka_map` `(10,10,160,160)` + `QgsLayoutItemLegend` `(110,110,55,55)` | `HeritageLayoutNumbers::update(map, true)` | `153` sites at `1:50,000` | `entries().size() == 153`, `legendKeys().size() == 153`, every badge inside `QRectF(10,10,160,160).adjusted(r,r,-r,-r)` and outside `legendRect.adjusted(-r,-r,r,r)` |
| `demLegendRendersHorizontalBarInSheet` | `tests/test_dem_legend.cpp:45-165` | DEM raster + `DemColorRampLegend` + `QgsLayoutItemLegend` | `LayoutService::tuneSheetLegend(legend)`, `LayoutService::flowSheetLegend(legend)` | collapsed width `1.0 mm` and normal width `60.0 mm` | `sheetLegendOmitsLayerName("지질 음영") == true`, `sheetLegendOmitsLayerName("지적도") == true`, `sheetLegendOmitsLayerName("VWorld 위성") == true`; collapsed `1.0 mm` legend repaired to `width >= 55.0 mm`; user `60.0 mm` width preserved; rendered legend image contains horizontal elevation ramp |
| `drawingStudioLegendAndLayers` | `tests/test_workflow.cpp:5677-5790` | `MainWindow` + `KaDrawingStudio` + survey/cadastral/satellite/geology layers | `beginPlaceLegend()`, uncheck/check layers, resize `ka_legend` width to `95 mm` | checked survey + cadastral + satellite + 12 heritage sites | `ka_legend` omits cadastral & satellite, lists checked survey + numbered heritage rows; unchecking survey removes its row without moving `ka_legend->pos()`; widening to `95 mm` increases `columnCount() >= 2` and reduces height |

### D. Rust today
- **Auto-placement on sheet open vs. when a layer is checked (`crates/strata-layout/src/sheet_drawing.rs:82`, `crates/strata-app/src/gui_tab_drawing_legend.rs:56-73`)**: **DIFFERENT** — `legend_visible` defaults to `true` (`sheet_drawing.rs:82`), so even when **zero** layers are checked (`rows == 0`), `sheet_legend_card` (`gui_tab_drawing_legend.rs:70-92`) draws an empty `24.0 * s` tall `"범례"` box on the map (violating both C++ and owner's 2026-11-15 rule 「범례는 레이어가 선택일때 자동으로 배치되게하라」 when no layer is checked).
- **`[범례]` button click (`crates/strata-app/src/gui_tab_drawing_right.rs:31-47`)**: **DIFFERENT** — toggles `self.drawing.sheet.legend_visible = !self.drawing.sheet.legend_visible`, shows no tooltip (C++ has `"범례를 용지에 넣습니다. 넣은 뒤 끌어 옮깁니다"`), sets no status message, does not end `map_adjust_active`, and does not select the legend.
- **Omitted layers (`crates/strata-app/src/gui_tab_drawing_legend.rs:60-66`)**: **DIFFERENT** — Rust skips `heritage:*`, `user:buffer_label_*`, `SwatchKind::RasterChecker`, and `SwatchKind::HeritageSub`, which **fails to omit checked cadastral vector layers (`지적*`) and geology structure classes** (C++ omits all `지적*` at `src/core/LayoutService.cpp:1377-1378` and geology structures at `:1388`), AND **omits DEM elevation colour-ramp rasters** (`RasterChecker` skips all rasters; `DemColorRampLegend` is **MISSING**).
- **Categorized sub-rows & map-extent filtering for geology/soil**: **MISSING** — `sheet_legend_survey_rows` (`crates/strata-app/src/gui_tab_drawing_legend.rs:60-66`) emits one flat row per checked layer using the left panel's `SwatchKind`, with no geology/soil class swatches and no map-extent hit-test filtering (heritage site badges/kinds at `:101-115` DO filter to the map frame, `already_done_in_rust`).
- **Legend title placeholder, `B` (bold), and `I` (italic)**:
  - `TextEdit` (`crates/strata-app/src/gui_tab_drawing_right.rs:70-72`): **DIFFERENT** — missing `.hint_text("제목을 입력하세요")` (`src/app/KaDrawingStudio.cpp:953`).
  - `B` and `I` checkboxes (`crates/strata-app/src/gui_tab_drawing_right.rs:83-84` vs `crates/strata-app/src/gui_tab_drawing_legend.rs:116-126`): **FAKE** — `self.drawing.sheet.legend_bold` and `self.drawing.sheet.legend_italic` are toggled by the checkboxes in `gui_tab_drawing_right.rs:83-84`, **never read anywhere in `gui_tab_drawing_legend.rs`**! Toggling `B` or `I` changes nothing on the sheet.
- **Columns, wrapping, and width resize (`crates/strata-app/src/gui_tab_drawing_legend_flow.rs:15-33`, `crates/strata-app/src/gui_tab_drawing_legend.rs:79-91, 133-135`)**:
  - `legend_columns` formula (`gui_tab_drawing_legend_flow.rs:15-22`): **SAME** `32.0 * pt / 9.0`, `boxSpace = 1.0`, `columnSpace = 1.5` math (using `legend_font_pt` instead of `max(5, pt - 1)`).
  - Width auto-growth, `0.42 * map_rect.width()` cap, no width resize handle, and clipping instead of wrapping (`gui_tab_drawing_legend.rs:79-89, 134`): **DIFFERENT** — user cannot resize legend width; instead `sheet_legend_card` auto-widens `leg_w` up to `map_rect.width() * 0.42` (anchoring right edge to `map_rect.right() - 5.0 * s` at `:89`, which shifts `placed.left()` whenever layer names change!) and **clips** long labels at cell edge (`:134`) instead of wrapping lines (`setAutoWrapLinesAfter`).
- **Heritage names checkbox (`1:5,000 이하에서 유적 이름으로 표시`)** (`crates/strata-app/src/gui_tab_drawing_right.rs:86-92`, `crates/strata-app/src/gui_tab_drawing_legend.rs:104-115`): **SAME** (`already_done_in_rust`).


---

## P7 — `P7_title_block`: `표제란` (`KaTitleBlock` and `KaDrawingStudio::placeTitleBlock`)

### A. C++ offered
1. **Right-panel card 1 (`도면 항목`) tile button** (`m_titleTile`, `objectName = "btnTitleBlock"`, `src/app/KaDrawingStudio.cpp:939-948`):
   - Built via `makeRailTile` with icon `"new"`, text `"표제란"`, tooltip verbatim `"도면명·조사명·유적명·축척·작성일 상자를 넣습니다. 조사구역의 조사명·유적명을 씁니다. 필요할 때만 넣습니다"`.
   - Always visible and enabled. In stock look, non-checkable; in `paperLook`, `syncSheetTiles()` (`src/app/KaDrawingStudio.cpp:1572-1590`) marks it checked (oat highlight) while `ka_title_block` exists on `user_sheet`, and clicking it always invokes `KaDrawingStudio::placeTitleBlock()`.
2. **Title input dialog** (`src/app/KaDrawingStudioSheets.cpp:44-50`):
   - `QInputDialog::getText(this, "표제란", "도면명 (예: 조사구역도, 유적위치도, 유구배치도)", QLineEdit::Normal, title, &ok)` where `title` defaults to `KaTitleBlock::drawingTitleOf(ly)` (`customProperty("ka_hgis/title_block_name")`, `src/app/KaTitleBlock.cpp:21, 113-116`) or `"조사도면"` if empty (`src/app/KaDrawingStudioSheets.cpp:45`).

### B. C++ does
1. **Optional by default**: `autoPlaceDefaultSheet()` (`src/app/KaDrawingStudio.cpp:938, 1358-1361`) and `LayoutService::createBlankSheet` do **not** create `ka_title_block`; the sheet has no title block until the user clicks `[표제란]`.
2. **Fields and value sources (`KaTitleBlock::collect` & `KaTitleBlock::text`, `src/app/KaTitleBlock.cpp:23-65`)**:
   - **5 lines** formatted verbatim (`src/app/KaTitleBlock.cpp:62-64`):
     `"도면명   %1\n조사명   %2\n유적명   %3\n축  척   %4\n작성일   %5"`
   - Empty / whitespace-only values become `"―"` (U+2015 horizontal bar, `orDash`, `:23-26`).
   - Line 1 `도면명`: user-entered `drawingTitle` from `QInputDialog::getText` (`orDash(drawingTitle)`), saved on the label as `customProperty("ka_hgis/title_block_name", drawingTitle)` (`:21, 104`).
   - Line 2 `조사명` & Line 3 `유적명`: `KaTitleBlock::collect(m_project)` (`:33-55`) iterates `LayerOps::domainLayersForKey(project, "survey_area")` (`NoGeometry` request) and takes the first non-empty `.trimmed()` string from fields `"survey_name"` and `"site_name"`, stopping as soon as both are non-empty (`:46-52`).
   - Line 4 `축  척`: dynamic QGIS expression `"1 : [% round(map_get(item_variables('%1'), 'map_scale')) %]".arg(mapItemId)` (`mapItemId = "ka_map"`, `:60-61`), evaluated live whenever `ka_map` scale changes.
   - Line 5 `작성일`: `QDate::currentDate()` formatted as `"yyyy-MM-dd"` (or `"―"` if `!date.isValid()`, `:64`, `src/app/KaDrawingStudioSheets.cpp:60`).
3. **Size & default placement (`KaTitleBlock::defaultRect`, `src/app/KaTitleBlock.cpp:67-77`)**:
   - Computes `chrome = LayoutService::standardSheetChrome(page, mapRect)`.
   - **Primary slot (under `ka_crs`, left of `ka_north`)**: `top = chrome.crs.bottom() + 1.5` mm, `height = min(18.0, page.bottom() - 3.0 - top)` mm, `width = max(0.0, chrome.north.left() - 2.0 - chrome.crs.left())` mm. Used whenever **`height >= 13.0 && width >= 50.0`** → `QRectF(chrome.crs.left(), top, width, height)` (`:69-73`). *(On A4 Portrait `210 × 297 mm` with `map = (10, 10, 190, 249)`: `chrome.crs = (116, 268.5, 58, 7)` and `chrome.north = (180, 266, 20, 20)`, giving `top = 277.0`, `height = 17.0 >= 13.0`, `width = 62.0 >= 50.0` → `QRectF(116.0, 277.0, 62.0, 17.0)` mm.)*
   - **Fallback slot (inside bottom-right of `ka_map`)**: `w = min(64.0, max(40.0, map.width() * 0.4))` mm → `QRectF(map.right() - w - 2.0, map.bottom() - 19.0, w, 17.0)` (`:74-76`).
4. **Creating vs. editing (`KaDrawingStudio::placeTitleBlock` `src/app/KaDrawingStudioSheets.cpp:35-69` & `KaTitleBlock::place` `src/app/KaTitleBlock.cpp:79-111`)**:
   - Step 1: Guard `!ly || !map` → status `"먼저 지도 칸을 그리세요."`, `syncSheetTiles()`, return (`src/app/KaDrawingStudioSheets.cpp:38-42`).
   - Step 2: `endActivateMap()`; prompt `QInputDialog::getText`; if cancelled (`!ok`), `syncSheetTiles()` and return without changing the sheet (`:43-54`).
   - Step 3 (`KaTitleBlock::place`): looks up `itemById("ka_title_block")`.
     - **If newly created (`created == true`)**: creates `QgsLayoutItemLabel` (`id = "ka_title_block"`), font `Malgun Gothic 6.5 pt`, text color `QColor(17, 24, 39)`, `Qt::AlignLeft | Qt::AlignTop`, margins `X = 1.2 mm`, `Y = 0.8 mm`, frame `0.25 mm` `QColor(17, 24, 39)`, white opaque background, sets text and `ka_hgis/title_block_name`, calls `label->attemptSetSceneRect(rect)`, adds to layout (`src/app/KaTitleBlock.cpp:84-108`), and appends `"ka_title_block"` to `m_placeUndo` (`src/app/KaDrawingStudioSheets.cpp:61`).
     - **If already on the sheet (`created == false`)**: updates `label->setText(text)` and `ka_hgis/title_block_name` **without calling `attemptSetSceneRect`** (`src/app/KaTitleBlock.cpp:103-108`), **preserving the user's moved position and resized box**!
   - Step 4: Sets `m_placeKind = PlaceKind::TitleBlock`, calls `markUserComposed()`, `finishPlace()` (switches to `m_toolSelect` and selects `ka_title_block`), `syncSheetTiles()`, and shows status:
     - If `info.surveyName.isEmpty() && info.siteName.isEmpty()`: `"표제란을 넣었습니다. 조사구역에 조사명·유적명이 없어 ―로 두었습니다."` (`src/app/KaDrawingStudioSheets.cpp:65-66`).
     - Otherwise: `"표제란을 넣었습니다. 끌어 옮기세요. 필요 없으면 Delete로 지웁니다."` (`:67`).
   - Removing `ka_title_block` is done by selecting it and pressing `Delete`/`Backspace` (`deleteSelectedItems`, `src/app/KaDrawingStudio.cpp:3225-3248`), or `Ctrl+Z` via `m_placeUndo` (`:3484-3507`).

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `titleBlock_isOptionalAndUsesSurveyFields` | `tests/test_drawing_studio.cpp:205-246` | `QgsProject` + polygon layer marked `"survey_area"` with `survey_name = "왕복조사"`, `site_name = "테스트유적"` + `user_sheet` (`210 × 297 mm`) | `KaTitleBlock::collect`, `KaTitleBlock::text`, `KaTitleBlock::defaultRect`, `KaTitleBlock::place` (create, `attemptMove`, re-place) | `title = "유구배치도"`, `QDate(2026, 9, 29)`, `"ka_map"`; empty `text(QString(), {}, QDate(), "ka_map")`; `page(0,0,210,297)`, `map(10,10,190,249)`; `moved(30.0, 40.0)`; second `place(..., "조사구역도", "x")` | `info.surveyName == "왕복조사"`, `info.siteName == "테스트유적"`; `text` contains `"도면명   유구배치도"`, `"조사명   왕복조사"`, `"유적명   테스트유적"`, `"item_variables('ka_map')"`, `"2026-09-29"`; empty `text` contains `"―"`; initial `!layout->itemById("ka_title_block")`; `page.contains(rect)`; `drawingTitleOf(layout) == "유구배치도"`; after move to `(30, 40)` and second `place`, `again == block`, `block->text() == "x"`, `QLineF(block->pos(), moved).length() < 0.01` |

### D. Rust today
- **Optional default (`title_block_visible`)**: **DIFFERENT** — `DrawingSheet::new` initializes `title_block_visible: true` (`crates/strata-layout/src/sheet_drawing.rs:83`), showing the title block on every new sheet by default instead of keeping it optional/off (`src/app/KaDrawingStudio.cpp:938`, `tests/test_drawing_studio.cpp:232`).
- **`[표제란]` button behavior & tooltip (`crates/strata-app/src/gui_tab_drawing_right.rs:48-66`)**: **DIFFERENT** — toggles `self.drawing.sheet.title_block_visible = !self.drawing.sheet.title_block_visible` (`:63-64`) with **no tooltip**, no status message, no `end_sheet_map_adjust()`, and no item selection.
- **Editing `도면명` (`QInputDialog`)**: **MISSING** — Rust has `self.drawing.title_ko` (default `"조사도면"`, `crates/strata-app/src/tab_drawing.rs:10, 69`), and **zero UI dialog or text field** to edit it.
- **5-line text & `"―"` fallback (`crates/strata-layout/src/sheet_decorations.rs:198-225`)**: **SAME** 5-line format (`도면명   ...`, `조사명   ...`, `유적명   ...`, `축  척   1 : N`, `작성일   yyyy-MM-dd`) and `"―"` fallback.
- **Source of `조사명` and `유적명` (`crates/strata-app/src/gui_tab_drawing_chrome.rs:64-72`, `crates/strata-app/src/real_survey_load.rs:16-17`)**: **DIFFERENT (unwired to `survey_area` attributes)** — `sheet_title_block_lines_on` reads `self.home_map.real_survey.survey_name` and `site_name`, which `RealSurveyData::new_empty_survey` initializes to `""` and nothing in survey open or `feature_tables` populates from the `"survey_area"` layer's `"survey_name"` / `"site_name"` fields!
- **Screen placement (`crates/strata-app/src/gui_tab_drawing_chrome.rs:92-106`)**: **DIFFERENT** — although `default_title_block_rect_mm` (`crates/strata-layout/src/sheet_layout.rs:128-149`) ports `KaTitleBlock::defaultRect` 1:1, `paint_sheet_title_block` (`gui_tab_drawing_chrome.rs:93-106`) ignores `raw_tb_r.top()` and hardcodes `tb_top = map_rect.bottom() + 8.2 * px_per_mm` (pushing `crs_words` up to `map_rect.bottom() + 4.5 * px_per_mm` at `gui_tab_drawing_decoration.rs:155` instead of `chrome.crs`!).

---

## P8 — `P8_north_scale`: North Arrow, Scale Spin/Apply/Chips, Scale Bar, and Grid Controls

### A. C++ offered
1. **Card 2 `방위 (도북)` (`m_cardNorth`, `src/app/KaDrawingStudio.cpp:992-1042`)**:
   - Caption `"방위 (도북)"`, tooltip `KaDrawingNorth::gridNorthNote()` = `"방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."` (`src/app/KaDrawingNorthArrow.cpp:205-207`).
   - **4 style swatch buttons** (`objectName = "northSwatch"`, `makeRailTile`, `:1004-1021`), always enabled (in `paperLook`, active style is oat-highlighted by `syncSheetTiles`, `:1591-1603`):
     1. `"북 글자"` (`rel = ""`, `art = 0`), tooltip `"북 글자 — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."`
     2. `"북 화살"` (`rel = "arrows/NorthArrow_02.svg"`, `art = 1`, **default** `:2710`), tooltip `"북 화살 — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."`
     3. `"나침반"` (`rel = "arrows/NorthArrow_04.svg"`, `art = 2`), tooltip `"나침반 — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."`
     4. `"바람장미"` (`rel = "wind_roses/WindRose_01.svg"`, `art = 3`), tooltip `"바람장미 — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."`
   - **Size row (`:1022-1041`)**: label `"크기"` + `QDoubleSpinBox` `m_northSize` (`3.0..80.0`, `0` decimals, suffix `" mm"`, initial spinbox value `28.0`, always enabled).
2. **Card 3 `도면 정보` (`m_scaleBar`, `src/app/KaDrawingStudio.cpp:1044-1181`)**:
   - Caption `"도면 정보"` (`:1058`).
   - **Scale box & Apply (`:1061-1082`)**: label `"1 :"`, `QSpinBox` `m_scaleSpin` (`objectName = "drawingScale"`, range `10..5,000,000`, step `10`, initial value `1000`, `keyboardTracking = false`, `groupSeparatorShown = false`), `QPushButton` `"적용"` (`objectName = "scaleApply"`). Always enabled.
   - **14 scale preset chips (`StandardScales::StudioChip`, `src/core/StandardScales.cpp:26-50`, `src/app/KaDrawingStudio.cpp:1084-1123`)**:
     - Full list (14): **`20`, `30`, `50`, `100`, `200`, `250`, `300`, `400`, `500`, `1000`, `2000`, `5000`, `10000`, `25000`**.
     - Each `QToolButton` (`objectName = "scaleChip"`) has `checkable = true`, no tooltip, `3` per row in stock look (5 rows, last row padded with 1 spacer) or **`7` per row** in `paperLook` (2 rows, `0 px` shared-edge table, `src/app/KaDrawingStudioPlate.cpp:122-147`).
   - **3 scale bar style tiles (`:1124-1140`)**:
     1. `"쌍칸"` (`style = "Double Box"`, **default** `:2712`), tooltip `"쌍칸"`
     2. `"외칸"` (`style = "Single Box"`), tooltip `"외칸"`
     3. `"눈금"` (`style = "Line Ticks Up"`), tooltip `"눈금"`
   - **`선택 항목 크기` (`:1141-1157`)**: covered in P5 (`25..200 %`, disabled unless 1 decoration is selected).
   - **Bottom 3 tiles (`:1158-1180`; in `paperLook`, text-only `32 px` tall word buttons sharing the row, `src/app/KaDrawingStudioPlate.cpp:149-170`)**:
     1. `"축척 글자"`, tooltip `"축척 글자"` → `beginPlaceScaleLabel`
     2. `"좌표계"`, tooltip `"좌표계"` → `beginPlaceCrsLabel`
     3. `"좌표 격자"` (`m_gridBtn`, `objectName = "btnGridSettings"`, `checkable = true`):
        - Tooltip when off (default): `"지도 안 +와 테두리 좌표 자를 켭니다. 기본은 꺼져 있습니다."` (`:1569`)
        - Tooltip when on: `"좌표 격자 켜짐 (%1). 눌러서 끄거나 간격을 바꿉니다."` (`"%1 m"` or `"간격 자동"`, `:1566-1568`)
        - Clicking opens modal `QDialog` **`"격자 설정"`** (`focusGridSettings`, `:1460-1518`), which contains:
          - `QCheckBox` `"좌표 격자"`, tooltip `"끄면 도면 안 +와 바깥 테두리 좌표 자가 사라집니다."`
          - Label `"간격"` + `QDoubleSpinBox` (`0..100000`, `0` decimals, step `10`, suffix `" m"`, `specialValueText = "자동(축척에 맞춤)"`, disabled while `gridFollowMap` is checked)
          - **`QCheckBox` `gridFollowMap`** (`:1481-1488`): text `"지도 좌표격자와 같게 (%1 m)"` and tooltip `"지도 화면의 좌표격자와 같은 간격·같은 선으로 도면에 격자선을 긋습니다."` when `northUpMapGrid(m_mapCanvas)` is active (enabled); text `"지도 좌표격자와 같게"` and tooltip `"지도에 미터 좌표격자를 켜면 쓸 수 있습니다(회전한 격자는 안 됩니다)."` when map grid is off or rotated (disabled)!
          - `QCheckBox` `"칸 밖에 좌표 숫자"` (no tooltip, default `false`)
          - `QDialogButtonBox::Close` button `"닫기"`

### B. C++ does
1. **North arrow (`beginPlaceNorth` & `placeNorth`, `src/app/KaDrawingStudio.cpp:1618-1627, 2403-2466`, `src/app/KaDrawingNorthArrow.cpp:53-207`)**:
   - Clicking a swatch calls `endActivateMap()`, sets `m_pendingNorthSvg = svgRel`, calls `applyNorthNow()` → `placeNorth`, appends `"ka_north"` to `m_placeUndo`, calls `markUserComposed()`, selects `ka_north` (`finishPlace()`), and shows status `"방위표를 넣었습니다. 끌어 옮기세요."` (`:1626`).
   - `KaDrawingNorth::writeSvg(kind)` writes a `72 × 72` vector SVG `ka-hgis-north-<0..3>.svg` where `"N"` is converted to a `QPainterPath` glyph outline (`drawGlyph`, no `<text>` element, `src/app/KaDrawingNorthArrow.cpp:25-40, 65-169`).
   - `placeNorth` creates `QgsLayoutItemPicture` (`id = "ka_north"`, `SVG` mode, `Zoom`, **`QgsLayoutItemPicture::GridNorth` linked to `ka_map`** so map rotation/convergence rotates the arrow automatically, `:2438-2446`), preserving `itemRotation`, `chrome_size_percent`, and `chrome_placed` (`:2411-2419, 2458-2462`). If SVG creation fails, falls back to `QgsLayoutItemLabel` `"N\n↑\n도북"` (`Malgun Gothic 12 pt Bold`, `:2428-2436`, `KaDrawingNorthArrow.cpp:203`).
   - Initial `standardSheetChrome` rect is `20.0 × 20.0 mm` (`src/core/LayoutService.cpp:908, 930`); changing `m_northSize` (`3..80 mm`) runs layout undo command `"방위표 크기"` resizing `ka_north` to `mm × mm` at its current `pos()` (`src/app/KaDrawingStudio.cpp:1029-1039`), and selecting `ka_north` syncs `m_northSize` to `item->rect().width()` (`:3081-3085`).
2. **Scale spin, `적용`, chips, and intermediate values (`applyOnScreenScale` & `syncScaleChips`, `src/app/KaDrawingStudio.cpp:1065-1105, 3016-3026, 3121-3137`)**:
   - Because `m_scaleSpin->setKeyboardTracking(false)` (`:1065`) and `valueChanged` is **not** connected, typing in `m_scaleSpin` does nothing until `Enter`/focus-out (`editingFinished`) or clicking `"적용"` (`scaleApply`), which calls `applyOnScreenScale()`.
   - Clicking one of the 14 `scaleChip`s sets `m_scaleSpin->setValue(n)` and calls `applyOnScreenScale()` (`:1100-1103`).
   - `applyOnScreenScale()` (`:3121-3137`): guards `map && ly && m_scaleSpin` (`"먼저 지도 칸을 그리세요."`), computes `wanted = max(10, m_scaleSpin->value())`, marks `user_composed` if changed, calls `map->setScale(double(wanted))` (accepting **any** integer `10..5,000,000`, e.g. `750`), calls `syncScaleDecorations()`, and shows status `"축척을 1 : %1 로 맞췄습니다."` (`:3136`).
   - **Intermediate value behavior (`syncScaleChips`, `:3016-3026`)**: compares each chip's `"denom"` property for exact equality `denom == m_scaleSpin->value()`. For any value between presets (e.g. `1:750` or `1:1200`), **all 14 chips uncheck**, while `m_scaleSpin` and `ka_scale` (`"축척 1 : 750"`) display the exact intermediate denominator!
3. **Scale bar (`beginPlaceScaleBar`, `placeScaleBar`, `applyNiceScaleBar`, `src/app/KaDrawingStudio.cpp:237-252, 1629-1638, 2468-2499, 2856-2877`, `src/core/LayoutService.cpp:678-750`)**:
   - Clicking `"쌍칸"` / `"외칸"` / `"눈금"` calls `endActivateMap()`, sets `m_pendingScaleBarStyle`, creates or updates `QgsLayoutItemScaleBar` (`id = "ka_scalebar"`), pushes `"ka_scalebar"` to `m_placeUndo`, `markUserComposed()`, selects `ka_scalebar`, and shows status `"축척자를 넣었습니다. 끌어 옮기세요."` (`:1637`).
   - Configured with `Meters`, unit label `"m"`, `4` segments right / `0` left, `AboveSegment` labels, bar height `2.4 * factor mm`, label-bar space `1.0 * factor mm`, `Malgun Gothic 7.0 * factor pt` with `0.6 mm` white buffer, `#111827` / `#FFFFFF` strokes `0.30 mm`, white opaque background (`:237-252`, `LayoutService.cpp:708-750`).
   - `niceScaleBarSegmentMeters(mapW, scale, 4)` (`src/core/LayoutService.cpp:678-700`) targets `clamp(mapW * 0.35, 40.0, 160.0)` mm (`* factor` in `applyNiceScaleBar`, `:2868`) and picks closest relative match from `19` values: `{0.1, 0.2, 0.4, 0.5, 1, 2, 4, 5, 10, 20, 40, 50, 100, 200, 400, 500, 1000, 2000, 5000}` m.
4. **`축척 글자`, `좌표계`, `좌표 격자`, and `지도 좌표격자와 같게`**:
   - **`축척 글자`** (`beginPlaceScaleLabel` → `placeScaleLabel(..., false)`, `src/app/KaDrawingStudio.cpp:1640-1649, 2533-2560`): creates or updates `QgsLayoutItemLabel` `"ka_scale"` (`"축척 1 : %1"`, `Malgun Gothic 9 pt Bold`, `AlignHCenter | AlignTop`, no frame/bg, keeping existing rect if present else `defaultItemRect("ka_scale")`), pushes `"ka_scale"` to `m_placeUndo`, `markUserComposed()`, status `"축척 글자를 넣었습니다. 끌어 옮기세요."` (note: `beginPlaceScaleLabel` omits `finishPlace()`, unlike `beginPlaceCrsLabel`).
   - **`좌표계`** (`beginPlaceCrsLabel` → `applyCrsLabelNow`, `:197-204, 1651-1660, 2501-2531`, `src/core/LayoutService.cpp:788-797`): creates or updates `QgsLayoutItemLabel` `"ka_crs"` (`Malgun Gothic 8 pt Bold`, `AlignRight | AlignVCenter`, no frame/bg), text `"동부원점(GRS80) · EPSG:5187"` / `"중부원점(GRS80) · EPSG:5186"` (etc.), pushes `"ka_crs"` to `m_placeUndo`, `markUserComposed()`, `finishPlace()`, status `"좌표계를 넣었습니다. 끌어 옮기세요."`.
   - **`좌표 격자` & `지도 좌표격자와 같게`** (`focusGridSettings`, `setGridOptions`, `applyCrsGrid`, `:1460-1570, 1976-2027`, `src/core/LayoutService.cpp:752-874`):
     - Default on a sheet is **OFF** (`m_gridEnabled = false`, `m_gridShowNums = false`, `m_gridIntervalM = 0.0`, `m_gridFollowMap = false`, stored on layout properties `ka_hgis/grid_*`, `:1521-1558`).
     - When `m_gridEnabled == true` and `m_gridFollowMap == false`: `interval = (m_gridIntervalM > 0) ? m_gridIntervalM : LayoutService::niceGridIntervalMeters(denom, mapWmm)` (`clamp(mapW * 0.24, 25.0, 70.0)` mm snapped to 18 steps `0.1..10000 m`, `LayoutService.cpp:766-786`), then `LayoutService::applySurveyFrameGrid(map, interval, true, m_gridShowNums)` (`LayoutService.cpp:799-874`) sets `Qgis::MapGridStyle::LineCrosses` (`2.2 mm` cross length, `0.18 mm` `rgba(17,24,39,160)` stroke, offset `(0, 0)`), `Zebra` border (`1.4 mm` wide, `0.22 mm` pen), and (only when `m_gridShowNums == true`) `OutsideMapFrame` integer TM annotations on all 4 sides (`Horizontal` top/bottom, `Vertical` left/right, `0.8 mm` distance, `Malgun Gothic 6.5 pt` `#1F2937`).
     - When `m_gridEnabled == true` and `m_gridFollowMap == true` and `northUpMapGrid(m_mapCanvas)` is non-null (requires map canvas coordinate grid enabled and unrotated `abs(rotationDeg) <= 1e-6`, `:176-184`): `LayoutService::applyMapCoordinateGrid(map, stepM, originX, originY, m_gridShowNums)` (`LayoutService.cpp:752-764`) switches style to full **`Qgis::MapGridStyle::Lines`** with positive modulo offsets `offset = origin - floor(origin / stepM) * stepM` so sheet lines pass through the exact same ground nodes as the map tab's trench grid!
     - Live status on change (`:1494-1497`): `"좌표 격자를 껐습니다."` / `"좌표 격자 간격 %1 m"` / `"좌표 격자 간격: 자동(축척에 맞춤)"`.

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `northArrow_isVectorAndSaysGridNorth` | `tests/test_drawing_studio.cpp:66-99` | `QgsProject` + `QgsPrintLayout` + `QgsLayoutItemPicture` for kinds `0..3` | `kindFromRel`, `fallbackLabel`, `gridNorthNote`, `writeSvg(0..3)`, `generatedKind`, `relForKind`, `isLegacyRaster`, `previewIcon(1)` | `"arrows/NorthArrow_04.svg"`, kinds `0, 1, 2, 3` | `kindFromRel("arrows/NorthArrow_04.svg") == 2`; `fallbackLabel().contains("도북")`, `!fallbackLabel().contains("진북")`; `gridNorthNote().contains("도북")`; for `kind = 0..3`: `.svg` file valid, contains `"<svg"`, `!svg.contains("<text")`, `QSvgRenderer(path).isValid()`, `!picture->isMissingImage()`, `!pictureNeedsRebuild(picture)`, `generatedKind(picture) == kind`, `kindFromRel(relForKind(kind)) == kind`, `!isLegacyRaster(picture)`; `!previewIcon(1).isNull()` |
| `standardScalesAreOneTable` (chips) | `tests/test_layout_sheet_rules.cpp:144-154` | pure call on `StandardScales::denominators(StandardScales::StudioChip)` | `StandardScales::denominators(StudioChip)`, `isStandard` | `StudioChip`, `25, 100, 750` | `chips == QList<int>{20, 30, 50, 100, 200, 250, 300, 400, 500, 1000, 2000, 5000, 10000, 25000}`; `!isStandard(25, StudioChip)`; `isStandard(100, StudioChip)`; `!isStandard(750, Snap)` |
| `scaleChipsAreOneJoinedTableInPaper` | `tests/test_drawing_plate.cpp:137-163` | `QWidget` with 2 rows × 7 `scaleChip` `QToolButton`s | `chipsPerRow()`, `chipGap(6)`, `joinScaleTable(&bar)` | stock vs `kaPaperLook = true` | stock: `chipsPerRow() == 3`, `chipGap(6) == 6`; `paperLook`: `chipsPerRow() == 7`, `chipGap(6) == 0`, `chips[0]` `kaRow=="first"`, `kaCol=="first"`, `chips[8]` `kaRow=="rest"`, `kaCol=="rest"`, top spacer `10`, between-row spacer `0` |
| `sheetMarksAreWordsSharingTheRowInPaper` | `tests/test_drawing_plate.cpp:166-199` | row of `"축척 글자"`, `"좌표계"`, `"좌표 격자"` (`btnGridSettings`) | `KaDrawingStudioPlate::wordTiles` | `kaPaperLook = true` vs stock | `paperLook`: all 3 `ToolButtonTextOnly`, `maximumHeight() == 32`, `row->stretch(i) == 1`, trailing spacer width `0`; stock: `ToolButtonTextUnderIcon` |
| `gridFollowsTheMapsStepAndNodes` | `tests/test_layout_map_grid.cpp:48-70` | `QgsPrintLayout` + `ka_map` `(10,10,160,160)` `EPSG:5186` extent `(219700,450000..219760,450060)` | `LayoutService::applyMapCoordinateGrid(map, 15.0, 219707.0, 450002.0, true)` | `step=15.0`, `origin=(219707.0, 450002.0)`, `showCoords=true` | `grids()->size() == 1`, `intervalX() == 15.0`, `intervalY() == 15.0`, `offsetX() == 2.0`, `offsetY() == 2.0`, `style() == Qgis::MapGridStyle::Lines`, `annotationEnabled() == true`, `inkAt(page, 219722.0) > 200`, `inkAt(page, 219714.5) < 20` |
| `plainFrameGridStaysOnTheCrsOrigin` | `tests/test_layout_map_grid.cpp:73-84` | same `ka_map` after `applyMapCoordinateGrid` | `LayoutService::applySurveyFrameGrid(map, 20.0, true, true)` | `interval=20.0`, `crosses=true`, `showCoords=true` | `grids()->size() == 1`, `offsetX() == 0.0`, `style() == Qgis::MapGridStyle::LineCrosses` |

### D. Rust today
- **North arrow styles (`crates/strata-app/src/gui_tab_drawing_right.rs:95-132`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:168-257`)**:
  - 4 style names (`북 글자`, `북 화살`, `나침반`, `바람장미`, default `SingleArrow` `북 화살`): **SAME**.
  - Tooltips on `"방위 (도북)"` caption and all 4 buttons, status message, and `GridNorth` map-rotation linkage: **MISSING** (`gui_tab_drawing_right.rs:95-131` has no `.on_hover_text(...)` and sets no status; `gui_tab_drawing_decoration.rs:170-256` draws unrotated egui polygons + font text `"N"`).
  - North arrow `"크기"` `3..=80 mm` (`crates/strata-app/src/gui_tab_drawing_right.rs:138-143` vs `crates/strata-app/src/gui_tab_drawing_chrome.rs:32-36`): **DIFFERENT** — `size_scale = ((north_arrow_size_mm / 28.0) * deco_pct).clamp(0.8 * deco_pct, 1.35 * deco_pct)` and `north_sz = (...).clamp(12.0, max_north_h.min(36.0))`, which clamps `3..80 mm` to **`0.8×..1.35×` (`22.4..37.8 mm` equivalent)** and `<= 36 px`!
- **Scale spinbox, `"적용"` button, and 14 chips (`crates/strata-layout/src/sheet_drawing.rs:72`, `crates/strata-app/src/gui_tab_drawing_right.rs:152-166`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:15-47`)**:
  - Initial scale denominator (`sheet_drawing.rs:72`): **DIFFERENT** — `5_000` instead of C++ `1000` (`src/app/KaDrawingStudio.cpp:1064`).
  - `DragValue` + `"적용"` button (`gui_tab_drawing_right.rs:156-165`): **DIFFERENT / FAKE `"적용"`** — `DragValue` mutates `self.drawing.sheet.scale_denominator` live on every drag/edit, so clicking `"적용"` (`:160-165`) **changes nothing on the sheet** and only writes `draw_guide_message = Some("축척을 1:{} 로 적용했습니다.")` (also wording `"적용했습니다."` differs from C++ `"맞췄습니다."`).
  - 14 preset numbers (`20..25000`, 7 per row, `gui_tab_drawing_right_b.rs:19, 33`): **SAME** 14 numbers; **DIFFERENT** in that buttons have no active/checked highlight when `scale_denominator` matches a preset (and set no status message on click).
- **Scale bar styles (`쌍칸`, `외칸`, `눈금`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:49-81`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:35-136`)**:
  - 3 styles + `nice_scale_bar_segment_meters` table: **SAME**.
  - Tooltips, status message, and segment labels (`gui_tab_drawing_decoration.rs:113`): **DIFFERENT** — no tooltips or status message, and labels only ticks `[0, 2, 4]` instead of every segment `0..=4`.
- **`"축척 글자"` and `"좌표계"` buttons (`crates/strata-app/src/gui_tab_drawing_right_b.rs:98-107`)**: **FAKE** — rendered as enabled clickable buttons, **gated by `&& i == 2` at line 106**, so clicking `"축척 글자"` (`i == 0`) or `"좌표계"` (`i == 1`) is a complete no-op!
- **`"좌표 격자"` button & `"지도 좌표격자와 같게 (20 m)"` checkbox (`crates/strata-app/src/gui_tab_drawing_right_b.rs:98-130`, `crates/strata-app/src/gui_tab_drawing_grid.rs:23-111`, `crates/strata-layout/src/sheet_drawing.rs:73-76`)**:
  - Default state: **DIFFERENT** — `grid_enabled` defaults to **`true`** (`sheet_drawing.rs:73`) with **no UI control to turn the grid off** (clicking `"좌표 격자"` at `gui_tab_drawing_right_b.rs:106-124` only toggles `grid_follow_map`!).
  - `"격자 설정"` dialog (enable toggle, custom interval spinbox `0..100,000 m`, `"칸 밖에 좌표 숫자"` toggle): **MISSING**.
  - `"지도 좌표격자와 같게 (20 m)"` (`gui_tab_drawing_right_b.rs:108-130`, `gui_tab_drawing_grid.rs:24-25`): **DIFFERENT** — always enabled directly on the panel instead of inside `"격자 설정"`, and **hardcodes `step = 20.0 m` and `origin = [0.0, 0.0]`** instead of reading the map tab's actual coordinate grid step, origin `(originX, originY)` modulo offset (`gui_tab_drawing_grid.rs:35-43, 81-82` never applies `grid_offset_xy_m`!), or rotation state.
  - Frame style & annotations (`gui_tab_drawing_grid.rs:54-109`): **DIFFERENT** — no `1.4 mm` Zebra frame; top-only X labels always drawn when `step_px >= 42.0`, bottom/left/right TM labels never drawn, and grid lines capped at `.take(24)`.

---

## P9 — `P9_paper_and_sheet_set`: `용지/방향` (`KaPaperDialog`, `resetPaper`) and `도면 목록` (`KaDrawingSheetSet`)

### A. C++ offered
1. **`용지 설정` dialog (`KaPaperDialog`, `src/app/KaPaperDialog.cpp:43-198`, opened from `openPaperSettingsDialog` `src/app/KaDrawingStudio.cpp:447-454` and `MainWindow::openLayoutDesigner` `src/app/MainWindowExport.cpp:218`)**:
   - Window title `"용지 설정"`, minimum width `400 px` (`:45-46`).
   - Row `"용지"` (`:61-76, 138`): 3 exclusive checkable `QToolButton`s (`minSize 88 × 56 px`): `"A4"` (`paperA4`, default checked), `"A3"` (`paperA3`), `"직접 입력"` (`paperCustom`).
   - Row `"방향"` (`:77-89, 139`): 2 exclusive checkable `QToolButton`s (`minSize 88 × 56 px`, `28 × 28` page icon): `"세로"` (`paperPortrait`, default checked), `"가로"` (`paperLandscape`).
   - Rows `"가로"` and `"세로"` (`:90-134, 140-141, 154-175`):
     - `QDoubleSpinBox` `paperWidth` & `paperHeight` (`50.0..2000.0`, `1` decimal, suffix `" mm"`, initial `210.0` / `297.0`).
     - In stock look: spinboxes always visible, enabled only when `"직접 입력"` is checked (`:154-157`).
     - In `paperLook`: when `"A4"` or `"A3"` is checked, shows dotted-leader `kaLeader` + mono value `paperSpecValue` (`plain(mm)`) + `"mm"`; spinboxes appear only when `"직접 입력"` is checked (`:107-134, 158-160, 172-175`).
   - Note label `paperScaleNote` (`:135-137, 149-153`):
     - When `A4` + `가로`: `"A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다. A4 종이 한 장에 찍으면 약 90%로 줄어 종이 위 축척이 달라집니다. 축척대로 찍으려면 인쇄 창에서 A3 또는 여러 장 나눠 찍기를 고르세요."`
     - All other combinations: `"기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."`
   - Footer buttons (`:180-185`): `"용지 만들기"` (OK) and `"취소"` (Cancel).
2. **`도면 목록` menu (`KaDrawingSheetSet::fillMenu`, `src/app/KaDrawingSheetSet.cpp:131-162`, attached to bottom toolbar button #7 `btnSheetSet` `src/app/KaDrawingStudio.cpp:1321-1339`)**:
   - Action 1: `"지금 도면 보관…"`, tooltip `"지금 용지를 이름 붙여 따로 둡니다. 레이어 켜짐도 같은 이름의 세트로 둡니다."` → `storeCurrentSheet()`
   - Separator
   - If `KaDrawingSheetSet::list(project)` is empty: disabled action `"보관한 도면이 없습니다"` (`:143`).
   - If non-empty (sorted by `localeAwareCompare` on `title`, `:40-42`):
     - One action per stored sheet: `"열기: %1".arg(title)` → `restoreStoredSheet(title)` (`:145-151`)
     - Submenu `"보관한 도면 지우기"` containing one action `<title>` per stored sheet → `removeStoredSheet(title)` (`:152-158`)
   - Separator
   - Disabled footer note action: `"제출 PDF는 지금 열린 도면으로 만듭니다"` (`:161`).
   - **Rename action**: **`not in the source`** — `KaDrawingSheetSet` has no rename action or function (`src/app/KaDrawingSheetSet.h:1-41`, `src/app/KaDrawingSheetSet.cpp:1-164`).

### B. C++ does
1. **Preset dimensions & changing paper (`KaPaperDialog.cpp:161-171`, `KaDrawingStudio::resetPaper`, `src/app/KaDrawingStudio.cpp:58-67, 456-560`)**:
   - Preset sizes: `A4 세로 = 210.0 × 297.0 mm`; **`A4 가로 = 317.0 × 220.0 mm`** (`kFieldPaperWidthMm × kFieldPaperHeightMm`, `297 + 2×10` wide by `210 + 10` high); `A3 세로 = 297.0 × 420.0 mm`; `A3 가로 = 420.0 × 297.0 mm`; `직접 입력 = 50.0..2000.0 × 50.0..2000.0 mm`. (Legacy `297 × 210 mm` reopened projects are upgraded by `applyFieldPageGrow`, `:432-445`.)
   - **What `resetPaper(widthMm, heightMm, preserveExisting = true)` keeps and moves (`:456-560`)**:
     - Clamps `m_paperW = max(20.0, widthMm)`, `m_paperH = max(20.0, heightMm)`, calls `endActivateMap()`, resizes page 0 (`Orientation::Landscape` if `w > h` else `Portrait`).
     - **Kept on `ka_map` (`:491-509`)**: `keepCenter = map->extent().center()`, `keepScale = map->scale()`, `keepCrs = map->crs()`. Resizes `ka_map` to `defaultMapRect()` (`10 mm` left/top/right, `38 mm` bottom chrome) and restores `keepCrs`, `zoomToExtent` around `keepCenter`, and `map->setScale(keepScale)`!
     - **4 bottom chrome items (`ka_scalebar`, `ka_scale`, `ka_crs`, `ka_north`, `:511-521`)**: resets `ka_hgis/chrome_placed = false` on all 4 and runs `applyStandardChromePositions(ly)` so they move cleanly to the new paper's bottom band.
     - **Non-chrome items (`ka_legend`, `ka_title_block`, etc., `:523-540`)**:
       - If item center was **off the old page** (e.g. legend parked outside paper): **left untouched** (`:528-529`).
       - If item center was **on the old page**: shifts `pos.y += (newH - oldH)` if `pos.y > oldH * 0.55`, shifts `pos.x += (newW - oldW)` if `pos.x > oldW * 0.5`, then clamps to `[4.0, max(4.0, page - size - 4.0)]` mm (`:531-539`).
     - **Coordinate callouts (`:542-545`)**: calls `relayoutCoordCallouts()`.
     - Zooms view to fit new paper (`zoomPaperVisible`), refreshes scale widgets & heritage numbers, marks `user_composed` (`:452`), and shows status `"용지 전환 완료: %1×%2 mm — 기존 지도 범위와 배치가 유지되었습니다."` (`:556-559`).
2. **Sheet set (`도면 목록`) operations (`src/app/KaDrawingSheetSet.cpp:51-129`, `src/app/KaDrawingStudioSheets.cpp:71-170`, `src/app/KaDrawingLayerSets.cpp:1-109`)**:
   - **What a kept sheet holds**:
     1. A complete `QgsPrintLayout::clone()` of `"user_sheet"` stored in `QgsProject::layoutManager()` under layout name `"도면 보관: <title>"` (`ka_hgis/saved_sheet = true`, `ka_hgis/saved_sheet_title = <title>`, `ka_hgis/auto_template = false`, `ka_hgis/user_composed = false`, `src/app/KaDrawingSheetSet.cpp:29-31, 62, 76-82`) — preserving paper size, map extent, scale, CRS, grid settings, north arrow, scale bar, legend, title block, coordinate callout items, and every item's position/size, **persisted in the `.qgz` project file**!
     2. A named layer visibility & style preset of the **exact same `<title>`** in `QgsMapThemeCollection` via `KaDrawingLayerSets::save(m_project, title, m_layerModel)` (`src/app/KaDrawingStudioSheets.cpp:103`, `src/app/KaDrawingLayerSets.cpp:42-60`).
   - **Keep (`storeCurrentSheet`, `src/app/KaDrawingStudioSheets.cpp:71-105`)**:
     - Guards `m_project && layout()` (`"보관할 용지가 없습니다."`), `endActivateMap()`, prompts `QInputDialog::getText` (`"도면 보관"`, `"도면 이름 (예: 조사구역도 1:1000, 1호 주거지 실측도)"`, default = title-block title else `"도면 <N+1>"`).
     - If `KaDrawingSheetSet::contains(m_project, title)`, asks `QMessageBox::question` `"「%1」 도면이 이미 있습니다. 지금 도면으로 바꿀까요?"` (default `No`, `:87-91`).
     - Flushes pending layer sync, stores layout clone + layer set, sets `project->setDirty(true)`, status `"도면 「%1」을 보관했습니다. 「도면 목록」에서 다시 엽니다."` (`:104`).
   - **Open (`restoreStoredSheet`, `:107-156`)**:
     - Asks `QMessageBox::question` (`"도면 열기"`, `"지금 용지를 보관한 도면 「%1」로 바꿉니다. 지금 도면을 보관하지 않았다면 사라집니다. 레이어 켜짐도 그 도면의 세트로 바뀝니다. 바꿀까요?"`, default `No`, `:109-115`).
     - Clones `"도면 보관: <title>"` into `"user_sheet"` (keeping the stored copy intact in `도면 목록`), strips `saved_sheet` properties, marks `user_composed = true`, updates `m_paperW`/`m_paperH` from page 0, clears `m_coordMapPts`/`m_placeUndo`, applies `KaDrawingLayerSets::apply(m_project, title, m_layerModel)`, zooms paper visible, refreshes heritage numbers & scale widgets, status `"도면 「%1」을 열고 그때의 레이어 켜짐을 적용했습니다."` (or `"도면 「%1」을 열었습니다."`, `:153-155`).
   - **Delete (`removeStoredSheet`, `:158-170`)**:
     - Asks `QMessageBox::question` (`"도면 목록"`, `"보관한 도면 「%1」과 같은 이름의 레이어 세트를 지울까요? 지금 열린 도면과 레이어는 그대로입니다."`, default `No`), removes `"도면 보관: <title>"` and `KaDrawingLayerSets::remove(m_project, title)`, status `"보관한 도면 「%1」을 지웠습니다."`.

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `lockedPaperSizeIsAReadingLine` & `stockPaperDialogKeepsItsDisabledFields` | `tests/test_drawing_polish.cpp:125-170` | `KaPaperDialog(nullptr, 317.0, 220.0)` in `paperLook=true` and `false` | click `paperLandscape`, `paperA3`, `paperCustom`, `setValue(123.4)` | `fieldWidthMm=317.0`, `fieldHeightMm=220.0`, custom width `123.4` | `paperLook=true`: initial A4 Portrait `paperSpecWidth=="210"`, `paperSpecHeight=="297"`, spin hidden; `paperLandscape` → `"317"` × `"220"`; `paperA3` (+ landscape) → `420.0 × 297.0`; `paperCustom` → spin visible & enabled, `widthMm() == 123.4`; stock: `paperWidth` visible & `!isEnabled()`, `210.0 × 297.0` |
| `sheetSet_storeAndReopenKeepsWorkingSheet` | `tests/test_drawing_studio.cpp:164-203` | `QgsProject` + `"user_sheet"` (`210 × 297`) with label `"marker"` = `"first"` | `KaDrawingSheetSet::store`, mutate working to `"second"`, `restore`, `remove` | `working = "user_sheet"`, `title = "조사구역도"` | `list.size() == 1`, `title == "조사구역도"`, `layoutName != "user_sheet"`, `!stored->customProperty("ka_hgis/user_composed", true).toBool()`; after `restore`: `workingCount == 1`, `marker->text() == "first"`, `restored->customProperty("ka_hgis/user_composed").toBool() == true`, `!restored->customProperty("ka_hgis/saved_sheet", false).toBool()`, `contains("조사구역도") == true`; after `remove`: `list.isEmpty()`, `sheet(project, "user_sheet") != nullptr`, second `restore` returns `nullptr` |
| `layerSets_applyOnlyWhenPicked` | `tests/test_drawing_studio.cpp:135-162` | `QgsProject` + 2 layers `"유구"` (checked), `"지형"` (unchecked) | `KaDrawingLayerSets::save`, flip visibility, `apply`, `remove` | `"유구 배치"`, `"없는 세트"`, `"  "` | `names.contains("유구 배치")`; after `apply`: `nodeA` checked, `nodeB` unchecked; `apply("없는 세트") == false`; `save("  ") == false`; after `remove`: `names.isEmpty()` |

### D. Rust today
- **`용지 설정` dialog (`crates/strata-app/src/modal_paper_account.rs:15-145`)**:
  - Choices (`A4` with `317 × 220` landscape & warning note, `A3`, `직접 입력` `50..=2000 mm`, `세로`/`가로`, `paper_look` dotted leader vs disabled DragValues, `"용지 만들기"` / `"취소"`, status message): **SAME**.
  - On accept (`modal_paper_account.rs:115-137`, `crates/strata-app/src/tab_drawing.rs:89-93`): **DIFFERENT** — does not call `end_sheet_map_adjust()`, does not reset or quadrant-shift user `item_offsets_mm`, and does not relayout `coord_callouts`.
- **`도면 목록` (`crates/strata-app/src/tab_drawing.rs:56-64`, `crates/strata-app/src/gui_tab_drawing_popups.rs:129-232`)**:
  - **Name prompt (`QInputDialog`) & overwrite confirmation**: **MISSING** — clicking `"지금 도면 보관…"` (`gui_tab_drawing_popups.rs:206-220`) pushes `self.drawing.title_ko.clone()` (`"조사도면"`) every time without asking for a name or checking duplicates!
  - **What `StoredSheet` holds & restores (`tab_drawing.rs:57-64`, `gui_tab_drawing_popups.rs:207-230`)**: **DIFFERENT (severely incomplete)** — in-memory `Vec<StoredSheet>` (lost when saving/closing the project!) holding only 6 fields (`title`, `scale_denominator`, `paper_w_mm`, `paper_h_mm`, `legend_visible`, `title_block_visible`), and on click (`:222-229`) it **does not even restore `paper_w_mm` / `paper_h_mm`**! Does **not** store or restore `sheet_center`, `item_offsets_mm`, `coord_callouts`, north/scalebar/grid/legend settings, or named layer visibility sets (`KaDrawingLayerSets` is **MISSING**), and asks no confirmation before overwriting the open sheet.
  - **Delete submenu (`"보관한 도면 지우기"`)**: **MISSING** — zero UI or method to delete a stored sheet.
  - **Footer note (`"제출 PDF는 지금 열린 도면으로 만듭니다"`) & row prefix (`"열기: <title>"`)**: **MISSING / DIFFERENT** — rows show `"<title>  (1:<scale>)"` (`:171`) and omit the submission note.

---

## P10 — `P10_output`: `PDF 내보내기`, `인쇄`, and Submission Package Sheet Export

### A. C++ offered
1. **`PDF 내보내기` button** (`pdfBtn`, `objectName = "btnPrimary"`, `src/app/KaDrawingStudio.cpp:929-932`, moved to `studioOutputFoot` in `paperLook` at `src/app/KaDrawingStudioPlate.cpp:76-102`):
   - Text `"PDF 내보내기"`, icon `"pdf"`, tooltip `"지금 용지를 PDF 파일로 저장합니다"`, always enabled.
   - Save file dialog (`src/app/KaDrawingStudio.cpp:3825-3826`): `QFileDialog::getSaveFileName(this, "도면 PDF 저장", "도면.pdf", "PDF (*.pdf)")` — **default file name `"도면.pdf"`**, **default folder = Qt current/last dialog directory** (no explicit path prefix).
   - Progress dialog (`KaDrawingPdfProgress`, `objectName = "drawingPdfProgress"`, `src/app/KaDrawingPdfProgress.cpp:10-24`): modal `QProgressDialog`, title `"PDF 내보내기"`, cancel button `"취소"`, initial label `"준비하는 중…"`.
2. **`인쇄` button & `Ctrl+P`** (`printBtn`, `objectName = "btnPrint"`, `src/app/KaDrawingStudio.cpp:933-937`, plus window-wide `Ctrl+P` / ribbon `"인쇄"` `src/app/MainWindowExport.cpp:186-194`):
   - Text `"인쇄"`, icon `"print"`, tooltip `"프린터로 찍거나, 큰 도면을 작은 용지 여러 장으로 나눠 찍습니다 (Ctrl+P)"`, always enabled.
   - Opens `KaDrawingPdfProgress(this, "인쇄 준비")`, then modal **`KaPrintDialog`** (`objectName = "printDialog"`, title `"도면 인쇄"`, size `860 × 640 px` in `paperLook`, `src/app/KaPrintDialog.cpp:69-242`), offering:
     - `"프린터"`: `QComboBox` `printPrinter` (real OS printers via `QPrinterInfo::availablePrinterNames()`, default OS printer selected, or disabled `"(설치된 프린터 없음)"`) + `QPushButton` `"프린터 속성…"` (`printProperties`, tooltip `"프린터 속성에서 단면 인쇄를 고르세요. 양면으로 나오면 기본 설정에서 단면을 고릅니다."`).
     - `"방법"`: radios `"여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)"` (`printModeTiles`) and `"한 장에 인쇄 (용지에 들어가면 실제 크기로)"` (`printModeFit`).
     - `"붙였을 때 크기"`: `QComboBox` `printOutput` (`"도면 크기 그대로 …"`, standard enlarged scales from `TilePrint::enlargedScales` up to A0, ISO A2/A1/A0 sizes; enabled in tiled mode).
     - `"용지"`: `QComboBox` `printSheet` (`"A4"`, `"A3"` — defaults to `A3` only when the printer supports A3, `:131`) + `"겹침"` `QDoubleSpinBox` `printOverlap` (`0..30 mm`, `0` decimals, default `10 mm`, tooltip `"이웃 장과 같은 그림이 겹치는 폭입니다. 붙일 때 맞추기 쉽습니다."`).
     - `"해상도"`: `QComboBox` `printDpi` (`"300 DPI (보통)"`, `"600 DPI (가는 선이 많은 실측도)"`, tooltip `"도면 PDF를 이 해상도로 그려 찍습니다. 0.1~0.2 mm 선이 많은 1:20 실측도는 600을 고르세요. 프린터 해상도보다 곱게 그리지는 않습니다."`).
     - Checkboxes (tiled mode): `"자르는 점선·장 번호·5 cm 확인선 넣기"` (`printMarks`, default `true`) and `"맨 앞에 붙이는 순서 안내 한 장 넣기"` (`printOverview`, default `true`).
     - `printSummary` bold label, `printWarning` amber scale-change warning label, interactive **`KaTilePreview`** (real rendered thumbnail of page 0 of `drawing-print.pdf` overlaid with clickable sheet grid to exclude/re-include individual tiles) + hint `"미리보기에서 장을 누르면 이번 인쇄에서 빼거나 다시 넣습니다. 잘못 나온 장만 다시 찍을 때 쓰세요."` + button `"모두 다시 넣기"` (`printPickAll`).
     - Footer buttons: `"나눈 장을 PDF로 저장…"` (`printSaveTiles`, default save name `"<title>-나눠찍기.pdf"` at `:594-596`), `"인쇄"` / `"인쇄 · N장"` / `"인쇄 · N장 + 안내 1장"` (`printStart`), `"닫기"`.

### B. C++ does
1. **`PDF 내보내기` function chain (`savePdf` → `exportDrawingPdf` → `HeritageLayoutNumbers::exportPdf` → `QgsLayoutExporter::exportToPdf`)**:
   - `KaDrawingStudio::savePdf()` (`src/app/KaDrawingStudio.cpp:3819-3855`):
     - Guards `layout()` (`QMessageBox::information(this, "PDF", "용지가 없습니다.")`) and `!m_pdfBusy`; prompts `QFileDialog::getSaveFileName` (`"도면.pdf"`).
     - Opens `KaDrawingPdfProgress(this, "PDF 내보내기")` and calls `exportDrawingPdf(path, &pdfError, &progress)`.
   - `KaDrawingStudio::exportDrawingPdf` (`:3758-3817`):
     - Step `"도면을 정리하는 중…"`: flushes `m_layerSyncPending` (`applyLayersToMap(map, true, false)`), runs synchronous `syncHeritageNumbers(true)` and `applyCrsGrid(map)`.
     - Step `KaDrawingPdfProgress::renderNote(m_paperW, m_paperH, 300.0)` (`src/app/KaDrawingPdfProgress.cpp:47-58`: `"PDF를 그리는 중… (용지 %1×%2 mm · %3 DPI)\n..."`, adding `"큰 용지라 시간이 걸립니다. "` when `w * h > 297 * 420 * 1.05`).
     - Renders to temp draft `draftDir/drawing.pdf` at **`KaPdfExport::kSheetDpi = 300.0` DPI** (`src/core/PdfExportSettings.h:15`, `src/core/PdfExportSettings.cpp:9-26`: `dpi = 300.0`, `forceVectorOutput = true`, `rasterizeWholeImage = false`, `textRenderFormat = Qgis::TextRenderFormat::PreferText`, `exportMetadata = true`, `writeGeoPdf = false`, `DisableTiledRasterLayerRenders = true`) via `m_heritageNumbers.exportPdf(map, legend, target, 300.0, &pdfError)`, then restores preview DPI (`96.0`).
     - Step `"파일을 저장하는 중…"`: if not cancelled, atomically copies draft to `path` via `QSaveFile` (`KaDrawingPdfProgress::commit`, `src/app/KaDrawingPdfProgress.cpp:60-80`). If cancelled, target file is untouched and status shows `"PDF 만들기를 취소했습니다. 파일은 만들지 않았습니다."` (`:82-84`).
     - On success: `markUserComposed()`, status `"저장: %1"` (+ one-time Google basemap notice if applicable), and `QMessageBox::information(this, "PDF", "저장했습니다.\n%1")` (`src/app/KaDrawingStudio.cpp:3844-3854`).
2. **`인쇄` function chain (`printDrawing` → `exportDrawingPdf` → `KaPrintDialog` → `TilePrint::renderTiles` / `renderFit`)**:
   - `KaDrawingStudio::printDrawing()` (`src/app/KaDrawingStudio.cpp:3857-3896`): creates `QTemporaryDir`, status `"인쇄할 도면을 만드는 중…"`, exports `drawing-print.pdf` at **`300.0 DPI`** with `KaDrawingPdfProgress(this, "인쇄 준비")` (cancel → `"인쇄를 취소했습니다."`), `markUserComposed()`, opens `KaPrintDialog(pdf, name.isEmpty() ? "도면" : name, scale, this)` (default `name` = `project->baseName()`, else `"도면"`).
   - Auto-selects `"한 장에 인쇄"` if `TilePrint::fit` (`5 mm` default margin) fits at `1:1` (`actualSize == true`), else `"여러 장으로 나눠 크게 인쇄"` (`src/app/KaPrintDialog.cpp:220-224`).
   - Clicking `"인쇄"` (`KaPrintDialog::printNow`, `:622-669`): checks `printerSupports(sheetSize())`, configures `QPrinter(HighResolution)` with `DuplexNone`, opens native `QPrintDialog` (`"프린터 속성"`), then renders at `300` or `600 DPI` (capped to printer resolution) via `TilePrint::renderTiles` (with progress `"프린터로 보내는 중… %2 / %3쪽"`, optional overview page + cut lines + sheet numbers + `5 cm` scale check bar) or `TilePrint::renderFit` (`1:1` centered when `actualSize`, else scaled to printable rect), and shows `QMessageBox::information("인쇄", "프린터로 %1쪽을 보냈습니다....")`.
   - Clicking `"나눈 장을 PDF로 저장…"` (`KaPrintDialog::saveTilesAs`, `:592-620`): `QFileDialog::getSaveFileName(this, "나눈 장 PDF 저장", base + "-나눠찍기.pdf", "PDF (*.pdf)")`, writes multi-page PDF via `QPdfWriter` at `300` or `600 DPI`, removes partial file on cancel/failure, shows `QMessageBox::information` reminding user to print at `"실제 크기(100%)"` and check the `5 cm` bar.
3. **What the submission package takes from the sheet (`ExportService::exportSubmissionPackage`, `src/core/ExportService.cpp:115, 195-215`, `src/core/LayoutService.cpp:1168-1232, 1256-1312`, `src/core/SubmitReadme.cpp:74-90, 135-140`)**:
   - Checks `LayoutService::isComposedStudioSheet(project, "user_sheet")` (`LayoutService.cpp:1168-1232`): requires layout `"user_sheet"` to exist, **not** be an unedited `auto_template` (`!(auto_template && !user_composed)`), have a `ka_map` with finite `scale() > 0` and finite non-empty `extent()`, and contain at least one valid non-blank, non-reference, non-cadastral, non-basemap vector layer with features (`LayerFeatures::any(vl)`) or non-empty raster. *(Stored sheets in `도면 목록` are named `"도면 보관: <title>"` with `user_composed = false` and are never exported directly.)*
   - If `sheet == true`: `LayoutService::exportLayoutPdf(project, "user_sheet", dir.filePath("조사도면.pdf"), &pdfErr)` (`ExportService.cpp:198-201`, `LayoutService.cpp:1256-1312`) runs `KaPdfExport::prepareLayout` + `HeritageLayoutNumbers::update(map, true)` + `exportPdf(..., 300.0)` (same `300 DPI` `PreferText` vector recipe as `PDF 내보내기`) to write **`조사도면.pdf`** in the package root (in the **work CRS**, NOT reprojected to 5179).
   - Collects `readme.referenceLines = SubmitReadme::referenceLines(project, {"user_sheet"})` (`ExportService.cpp:203`, `SubmitReadme.cpp:74-90`: reference, basemap, cadastral, and heritage layers drawn on `user_sheet`).
   - Records `readme.hasSheetPdf = QFile::exists(dir.filePath("조사도면.pdf"))` (`ExportService.cpp:214`), writing into `README_submit.txt` (`SubmitReadme.cpp:138-140`):
     - Present: `"- 조사도면.pdf (도면만들기 user_sheet)\n"`
     - Absent: `"조사도면.pdf 없음: 도면만들기에서 용지를 만든 뒤 다시 보내기 하세요.\n"`
   - Hashes `조사도면.pdf` into `MANIFEST.sha256` (`ExportService.cpp:226-228`).

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `pdfProgress_cancelKeepsTargetUntouched` | `tests/test_drawing_studio.cpp:248-283` | `QTemporaryDir` with `draft.pdf` (`"%PDF-1.7 draft"`) and `target.pdf` (`"old"`) + `KaDrawingPdfProgress` | `renderNote`, `progress.step`, `dialog->cancel()`, `commit(draft, target)` | `(210, 297, 300)`, `(841, 1189, 300)` | `!renderNote(210,297,300).contains("큰 용지")`, `renderNote(841,1189,300).contains("큰 용지")`, `renderNote(210,297,300).contains("300 DPI")`; after `cancel()`, `step(...) == false`, `cancelled() == true`, `read(target) == "old"`; after `commit`, `read(target) == "%PDF-1.7 draft"` |
| `a0DrawingNeedsNineA3Sheets` & `tilesPutEachPartOfTheDrawingOnItsOwnPage` | `tests/test_tile_print.cpp:71-86, 115-147` | `TilePlan` (`841×1189` on A3; `400×300` 4-color quadrant PDF on A4) | `TilePrint::bestPlan`, `TilePrint::renderTiles` | `841×1189` on A3 (`5 mm` margin, `6 mm` band, `10 mm` overlap); `400×300` on A4 (`100 DPI`) | A0→A3: `cols == 3`, `rows == 3`, `sheets.size() == 9`, `contentMm == QSizeF(287, 404)`, `sheets[1].left == 277.0`, `sheets[3].top == 394.0`; `400×300`→A4: `landscape == true`, `2×2 = 4` pages (`297×210 mm`), quadrant colors match red/green/yellow, beyond `400 mm` is white paper |
| `printDialogCountsSheetsAndWarnsAboutEnlarging` & `overviewAndChosenSheetsDecideThePages` | `tests/test_tile_print.cpp:172-204, 270-305` | `KaPrintDialog` on `297×420` PDF; `renderTiles` on `400×300` PDF with `sheets = {3, 0, 3}` | `setTiled`, `setSheet(SheetA4)`, `setOutput(OutputA0)`, `setOverview(true)`, `saveTilesPdf`, `renderTiles` | `297×420` → `SheetA4` + `OutputA0`; `sheets = {3, 0, 3}`, `overview = true` | `printProperties->text() == "프린터 속성…"`, `warning().contains("283%")`, `pageCount() == count + 1`, `printButtonText() == "인쇄 · %1장 + 안내 1장"`, saved PDF has `count + 1` pages; `OutputDrawing` clears warning; `sheets = {3,0,3}` + overview writes `3` pages (overview + sheet 1 + sheet 4) |
| `sheetSeesSurveyAreaTheLayerCountMissed` & `sheetSeesSurveyAreaWithAnUnsavedDeletion` | `tests/test_submit_package.cpp:98-116, 153-166` | `QgsProject` + `"user_sheet"` (`297×210`) + `ka_map` + `survey_area` | `LayoutService::isComposedStudioSheet(&project)` | stale OGR count `0` with 1 saved feature; 2 features with 1 unsaved deletion | `isComposedStudioSheet(&project) == true` in both cases |

### D. Rust today
- **`PDF 내보내기` (`crates/strata-app/src/gui_tab_drawing_right_b.rs:9-10, 133-144`, `crates/strata-layout/src/sheet_pdf_export.rs:27-56`)**: **MISSING (disabled)** — button is disabled with tooltip `"아직 Rust 판에서 연결하지 않았습니다 (지시 R3)"`; `render_sheet_pdf_bytes` only strokes empty wireframe boxes (`re S`) with zero map imagery, vectors, text, north arrow, legend, or progress/save dialog.
- **`인쇄` modal (`crates/strata-app/src/dlg_drawing_print.rs`, `dlg_drawing_print_options.rs`, `dlg_drawing_print_preview.rs`, `dlg_drawing_print_actions.rs`)**:
  - `Ctrl+P` shortcut on `MainTab::Drawing`: **MISSING** (`crates/strata-app/src/gui_shell.rs:150-169` handles only `Escape`, `Ctrl+Z`/`Y`, `Delete`).
  - `"프린터"` selector (`"ezViewX Virtual Image Printer              ▼"`, `dlg_drawing_print_options.rs:22-28`) & `"프린터 속성..."` button (`:32-38`): **FAKE** — hardcoded string `"ezViewX Virtual Image Printer              ▼"` and `let _ = ui.add(...)` with zero click action and zero OS printer enumeration!
  - `"방법"` radios, `"용지"` `A4`/`A3`, `"겹침"` `0..=30 mm`, `"해상도"` `300`/`600 DPI` toggle (`"600 DPI (고정밀 제출용)"` vs C++ `"600 DPI (가는 선이 많은 실측도)"`), `print_marks` & `print_overview` checkboxes (`dlg_drawing_print_options.rs:44-143`): **MOSTLY SAME** UI state fields, except `"붙였을 때 크기"` output scale/A0 enlargement combo (`printOutput`) and scale-change warning banner (`printWarning`) are **MISSING**, and `"인쇄 크기"` summary hardcodes `"실제 크기 100%"` even in single-sheet shrink mode (`:173`).
  - Right-hand preview (`render_print_preview`, `dlg_drawing_print_preview.rs:12-124`): **FAKE** — ignores the user's survey, layers, paper orientation, and tile grid, and instead paints a **hardcoded green rectangle (`RGB(154,198,120)`), 85 procedural pink/green dots (`(i * 37 + 13) % 100`), an orange bar, `"축척 1 : 6532"`, and `"동부원점(GRS80) · EPSG:5187"`** (`:49-123`)! Interactive tile exclusion (`toggleSheet`) and `"모두 다시 넣기"` (`printPickAll`) are **MISSING**.
  - `"나눈 장을 PDF로 저장…"` (`dlg_drawing_print_actions.rs:12, 37-45`): **MISSING (disabled)** — disabled with `"아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)"`.
  - `"인쇄 · {page_cnt}장"` button (`dlg_drawing_print_actions.rs:57-83`): **FAKE** — calls `verify_requested_device_paper(app.modals.print_paper, ori, app.modals.print_paper, ori)` (comparing `print_paper` to itself so it is unconditionally `Ok(())`!), sets `draw_guide_message`, closes the modal, and **sends nothing to any printer**! (Also `page_cnt` at `:20-29` ignores `print_overview`, never adding `+ 안내 1장`.)
- **Submission package `조사도면.pdf` (`crates/strata-check-export/src/submit_builder.rs:122-137`, `crates/strata-app/src/modal_export_footer.rs:38-40`)**: **MISSING** — UI button `"제출 꾸러미 만들기…"` is disabled (`지시 R2`), and `submit_builder.rs:123-125, 136` steps `"조사도면.pdf"` without writing any PDF file (`readme.has_sheet_pdf` is always `false`).


---

## P11 — `P11_right_panel`: The Right Panel as a Whole, Top to Bottom

### A. C++ offered
One single table in top-to-bottom screen order (`src/app/KaDrawingStudio.cpp:885-1182, 1237-1266`, `src/app/KaDrawingStudioPlate.cpp:47-103, 122-194`).
*(Note on `PDF 내보내기` and `인쇄`: `KaDrawingStudio.cpp:949-950` builds them in Card 1's button row for stock look; when `paperLook == true`, `KaDrawingStudioPlate::inspectorColumn` at `src/app/KaDrawingStudioPlate.cpp:76-102` reparents both out of `drawingInspectorScroll` into `studioOutputFoot` at the non-scrolling bottom of the right column.)*

| # | Group / Card | Widget (`objectName`) | Text verbatim | Tooltip verbatim | Explicit size in code | Enabled rule |
|---|---|---|---|---|---|---|
| 0 | Column shell | `QSplitter` child 2: `studioInspectorColumn` (`paperLook`) wrapping `QScrollArea` (`"drawingInspectorScroll"`) -> `QFrame` (`"studioLeftCol"`) (`:885-893, 1237-1266`, `Plate:70-103`) | *(none)* | *(none)* | Splitter initial sizes `{348, 820, 320}` px (`:1266`); scroll `minWidth = max(260, inspectorMinWidth)`, `maxWidth = max(420, inspectorMinWidth)` px (`:1251-1254`); `sideLay` margins `(12,8,12,8)`, spacing `8` px (`:890-891`; halved when `studioHeight < 860` px, `Plate:175-190`) | Always visible & enabled |
| 1 | Card 1 (`m_cardLegend`, `"itemInspector"`) | `QLabel` (`"cardCaption"`) + `QFrame` (`"studioCaptionRule"`, `paperLook`) (`:902-904`, `Plate:47-65`) | `"도면 항목"` | *(none)* | Card margins `(10,10,10,10)` stock / `(0,4,0,0)` `paperLook`, spacing `6` px; `studioCaptionRule` fixed height `1 px` (`Plate:53, 61`) | Always enabled |
| 2 | Card 1 row 1 | `QToolButton` (`"legendTile"`) via `makeRailTile` (`:908-918`) | `"범례"` | `"범례를 용지에 넣습니다. 넣은 뒤 끌어 옮깁니다"` | Icon `layoutIconSize × layoutIconSize`; `Fixed × Fixed` = `sizeHint()` (`minWidth >= scaleButtonMinWidth`, `fixedHeight >= layoutButtonHeight`, `:292-310, 913-914`) | Always enabled (oat-checked in `paperLook` while `ka_legend` exists, `:1588`) |
| 3 | Card 1 row 1 | `QToolButton` (`"btnTitleBlock"`) via `makeRailTile` (`:939-948`) | `"표제란"` | `"도면명·조사명·유적명·축척·작성일 상자를 넣습니다. 조사구역의 조사명·유적명을 씁니다. 필요할 때만 넣습니다"` | Same as `"legendTile"` (`Fixed × Fixed` = `sizeHint()`, `:944-945`) | Always enabled (oat-checked in `paperLook` while `ka_title_block` exists, `:1589`) |
| 4 | Card 1 row 1 *(stock look only; see #22–#23 in `paperLook`)* | `QToolButton` (`"btnPrimary"`) & `QToolButton` (`"btnPrint"`) (`:919-937, 949-950`) | `"PDF 내보내기"` / `"인쇄"` | `"지금 용지를 PDF 파일로 저장합니다"` / `"프린터로 찍거나, 큰 도면을 작은 용지 여러 장으로 나눠 찍습니다 (Ctrl+P)"` | Icon `20 × 20 px`, `ToolButtonTextBesideIcon`, `Expanding × Fixed`, stretch `1` each | Always enabled |
| 5 | Card 1 | `QLabel` (`:956`) | `"범례 제목"` | *(none)* | Default `QLabel` | Always enabled |
| 6 | Card 1 | `QLineEdit` `m_legendTitle` (`:952-957`) | default `"범례"`, placeholder `"제목을 입력하세요"` | *(none)* | Full card width | Always enabled |
| 7 | Card 1 font row | `QLabel` (`:970`) | `"글자"` | *(none)* | Default `QLabel` | Always enabled |
| 8 | Card 1 font row | `QSpinBox` `m_legendFont` (`:959-964, 971`) | `5..24`, initial `10`, suffix `" pt"` | *(none)* | Row stretch `1` | Always enabled |
| 9 | Card 1 font row | `QCheckBox` `m_legendBold` (`:965-966, 972`) | `"B"` (initial checked) | *(none)* | Default `QCheckBox` | Always enabled |
| 10 | Card 1 font row | `QCheckBox` `m_legendItalic` (`:967-969, 973`) | `"I"` (initial unchecked) | *(none)* | Default `QCheckBox` | Always enabled |
| 11 | Card 1 | `QCheckBox` `m_heritageNames` (`"heritageNamesCheck"`, `:975-989`) | `"1:5,000 이하에서 유적 이름으로 표시"` | `"켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다"` | Full card width | Always enabled |
| 12 | Card 2 (`m_cardNorth`, `"itemInspector"`) | `QLabel` (`"cardCaption"`) + `"studioCaptionRule"` (`:998-1001`, `Plate:47-65`) | `"방위 (도북)"` | `"방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."` | Card margins `(10,10,10,10)` stock / `(0,4,0,0)` `paperLook`, spacing `6` px | Always enabled |
| 13 | Card 2 swatch row | 4 × `QToolButton` (`"northSwatch"`) via `makeRailTile` (`:1002-1021`) | `"북 글자"` \| `"북 화살"` \| `"나침반"` \| `"바람장미"` | `"<text> — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다."` | Row spacing `6 px`; each icon `layoutIconSize × layoutIconSize`, `Expanding × Fixed`, `minWidth >= scaleButtonMinWidth`, `fixedHeight >= layoutButtonHeight` (`:292-310`) | Always enabled (active style oat-checked in `paperLook`, `:1601-1603`) |
| 14 | Card 2 size row | `QLabel` + `QDoubleSpinBox` `m_northSize` (`:1022-1041`) | `"크기"` + `3.0..80.0` (`0` dec, initial `28`, suffix `" mm"`) | *(none)* | Spinbox row stretch `1` | Always enabled |
| 15 | Card 3 (`m_scaleBar`, `"itemInspector"`) | `QLabel` (`"cardCaption"`) + `"studioCaptionRule"` (`:1044-1060`, `Plate:47-65`) | `"도면 정보"` | *(none)* | Card margins `panelMargin` stock / `(0,4,0,0)` `paperLook`; card stretch `1` in `sideLay` (`:1181`) | Always enabled |
| 16 | Card 3 scale row | `QLabel` + `QSpinBox` `m_scaleSpin` (`"drawingScale"`) + `QPushButton` (`"scaleApply"`) (`:1061-1082`) | `"1 :"` + `10..5000000` (step `10`, initial `1000`) + `"적용"` | *(none)* | Spacing `buttonSpacing`; both `drawingScale` (stretch `1`) and `scaleApply` have `fixedHeight = buttonMetrics.scaleButtonHeight` (`:1070, 1073`) | Always enabled |
| 17 | Card 3 preset grid | 14 × `QToolButton` (`"scaleChip"`, `:1084-1123`, `Plate:122-147`) | `"20"`, `"30"`, `"50"`, `"100"`, `"200"`, `"250"`, `"300"`, `"400"`, `"500"`, `"1000"`, `"2000"`, `"5000"`, `"10000"`, `"25000"` | *(none)* | Stock: `3` per row (5 rows + 1 spacer), gap `buttonSpacing`; `paperLook`: **`7` per row** (2 rows), gap `0 px` joined table; each `minWidth = scaleButtonMinWidth`, `fixedHeight = scaleButtonHeight`, `Expanding × Fixed` | Always enabled (`checked` iff `denom == m_scaleSpin->value()`, `:3021`) |
| 18 | Card 3 scale bar row | 3 × `QToolButton` via `makeRailTile` (`:1124-1140`) | `"쌍칸"` \| `"외칸"` \| `"눈금"` | `"쌍칸"` \| `"외칸"` \| `"눈금"` (`:300`) | Spacing `buttonSpacing`; icon `ribbonIconSize × layoutIconSize` (`72×40` pixmap); `Expanding × Fixed`, `fixedHeight >= layoutButtonHeight` | Always enabled |
| 19 | Card 3 item size row | `QLabel` + `QDoubleSpinBox` `m_decorationSize` (`"decorationSize"`, `:1141-1157`) | `"선택 항목 크기"` + `25..200` (`0` dec, step `10`, initial `100`, suffix `" %"`) | Spinbox: `"용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다."` | Spinbox row stretch `1` | **Disabled (`false`) by default**; enabled **only** when `selectedLayoutItems().size() == 1 && isSheetDecoration(item)` (`ka_scalebar`, `ka_scale`, `ka_crs`, `ka_north`, `:3076-3078`) |
| 20 | Card 3 extra row | 2 × `QToolButton` via `makeRailTile` (`:1158-1167`, `Plate:149-170`) | `"축척 글자"` \| `"좌표계"` | `"축척 글자"` \| `"좌표계"` (`:300`) | Stock: `layoutIconSize` icon + text under; `paperLook`: `ToolButtonTextOnly`, `minWidth = 0`, `fixedHeight = 32 px`, stretch `1` each (`Plate:155-163`) | Always enabled |
| 21 | Card 3 extra row | `QToolButton` `m_gridBtn` (`"btnGridSettings"`, `:1168-1180`, `Plate:149-170`) | `"좌표 격자"` | Off: `"지도 안 +와 테두리 좌표 자를 켭니다. 기본은 꺼져 있습니다."`; On: `"좌표 격자 켜짐 (%1). 눌러서 끄거나 간격을 바꿉니다."` (`:1564-1570`) | Same as #20 (`fixedHeight = 32 px`, stretch `1`, trailing stretch collapsed to `0` in `paperLook`, `Plate:155-165`) | Always enabled (`checked == m_gridEnabled`, opens modal `"격자 설정"`, `:1460-1570`) |
| 22 | Non-scrolling foot (`"studioOutputFoot"`, **`paperLook` only**, `Plate:85-99`) | `QToolButton` (`"btnPrimary"`) | `"PDF 내보내기"` | `"지금 용지를 PDF 파일로 저장합니다"` | Foot margins `(16,12,16,12)`, spacing `8 px`; row stretch `1` (takes remaining width, `Plate:97`) | Always enabled |
| 23 | Non-scrolling foot (`"studioOutputFoot"`, **`paperLook` only**, `Plate:85-99`) | `QToolButton` (`"btnPrint"`) | `"인쇄"` | `"프린터로 찍거나, 큰 도면을 작은 용지 여러 장으로 나눠 찍습니다 (Ctrl+P)"` | `QSizePolicy::Minimum × Fixed`, `minWidth = 88 px` (`Plate:95-98`) | Always enabled |

### B. C++ does
1. **Splitter & scroll geometry (`src/app/KaDrawingStudio.cpp:1237-1266`)**:
   - `m_studioSplit` holds 3 panes `[leftCol, right, inspectorColumn]` with stretch factors `(0, 1, 0)`, `childrenCollapsible = false`, initial sizes `{348, 820, 320}` px.
   - `inspectorMinWidth = max(side->sizeHint().width(), side->minimumSizeHint().width()) + PM_ScrollBarExtent` so the vertical scrollbar never clips the 4 north swatches or output buttons (`:1251-1254`).
2. **`paperLook` (`조용한 도판`) transformations (`src/app/KaDrawingStudioPlate.cpp:47-103, 126-194`)**:
   - `captionLeaders` (`:47-65`): removes card boxes (`kaPlate = true`), turns each `"cardCaption"` into a horizontal header followed by a `1 px` `"studioCaptionRule"` hairline extending to the right edge.
   - `joinScaleTable` (`:126-147`): arranges the 14 `scaleChip`s `7` per row with `0 px` spacing and shared borders (`kaRow` / `kaCol`).
   - `wordTiles` (`:149-170`): turns `"축척 글자"`, `"좌표계"`, `"좌표 격자"` into `32 px` text-only buttons sharing the full row width (`1:1:1`).
   - `inspectorColumn` (`:70-103`): reparents `"btnPrimary"` (`PDF 내보내기`) and `"btnPrint"` (`인쇄`, `minWidth 88 px`) into `"studioOutputFoot"` beneath `"drawingInspectorScroll"` so they never scroll out of view.
   - `fitToHeight` (`:172-194`, called from `KaDrawingStudio::resizeEvent`): halves top/bottom layout margins and positive vertical spacings when `studioHeight < 860 px` while keeping row-internal `MinimumExpanding` spacers intact.
3. **Selection-driven inspector sync (`updateInspector`, `src/app/KaDrawingStudio.cpp:3062-3106`)**:
   - All 3 cards remain visible at all times (`:3087-3089`).
   - Selecting 1 decoration item enables `m_decorationSize` and loads its `ka_hgis/chrome_size_percent`; selecting anything else disables `m_decorationSize` and displays `100 %` (`:3074-3080`).
   - Selecting `ka_north` syncs `m_northSize` to `item->rect().width()` (`:3081-3085`); selecting `ka_legend` syncs `m_legendTitle` and `m_legendFont` (`:3091-3103`); selecting `ka_scale` / `ka_scalebar` / `ka_map` calls `syncScaleChips()` (`:3104-3105`).

### C. C++ tests
| name | file:lines | what it builds | the call | inputs verbatim | EVERY expected value verbatim |
|---|---|---|---|---|---|
| `outputButtonsSitAtThePanelFootInPaper` & `stockKeepsTheButtonsInTheirCard` | `tests/test_drawing_plate.cpp:47-63, 201-208` | `QScrollArea("drawingInspectorScroll")` with `"btnPrimary"` (`PDF 내보내기`) and `"btnPrint"` (`인쇄`) | `KaDrawingStudioPlate::inspectorColumn(scroll)` | `kaPaperLook = true` (`320 × 600`) vs `QVariant()` | `paperLook=true`: returns new `column != scroll`, `studioOutputFoot` exists, `pdf` & `print` parented to `foot`, `foot->y() >= scroll->geometry().bottom()`, `pdf->x() < print->x()`; stock: returns `scroll`, `pdf->parentWidget()` unchanged |
| `studioCardsAreCaptionsWithALeader` | `tests/test_drawing_polish.cpp:47-91` | 3 `"itemInspector"` `QFrame`s with `"cardCaption"` (`"도면 항목"`, `"방위 (도북)"`, `"도면 정보"`) | `KaDrawingStudioPlate::inspectorColumn(scroll)` (`kaPaperLook = true`) | `320 × 520` window | all 3 cards have `kaPlate == true`, visible `studioCaptionRule` of `height() == 1` running from `caption->right()` to `>= card->width() - 4`, and card border pixels have `alpha() == 0` |

### D. Rust today
- **Top-to-bottom section & control order (`crates/strata-app/src/gui_tab_drawing_right.rs:23-168`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:14-158`)**: **SAME** overall order (`도면 항목` → `방위 (도북)` → `도면 정보` → `PDF 내보내기` / `인쇄`).
- **Panel width & splitter (`crates/strata-app/src/gui_tab_drawing_right.rs:11-19`)**: **DIFFERENT** — `SidePanel::right("ka_drawing_right_panel").resizable(false).exact_width(268.0)` instead of resizable `QSplitter` column `260..420 px` (`initial 320 px`).
- **Output buttons scroll away (`crates/strata-app/src/gui_tab_drawing_right.rs:15-18`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:132-157`)**: **DIFFERENT** — `"PDF 내보내기"` and `"인쇄"` sit **inside** `ScrollArea::vertical()` instead of a fixed bottom `studioOutputFoot` beneath the scroll area (`tests/test_drawing_plate.cpp:47-63`).
- **Caption hairline leaders & `< 860 px` compact height (`crates/strata-app/src/gui_tab_drawing_right.rs:25-29, 94-99, 146-151`)**: **DIFFERENT** — draws full-width `ui.separator()` above captions instead of inline `1 px` `studioCaptionRule` leaders from the caption's right edge, and has no `< 860 px` `fitToHeight` compaction.
- **Extra checkbox on main panel (`crates/strata-app/src/gui_tab_drawing_right_b.rs:127-130`)**: **DIFFERENT** — places `"지도 좌표격자와 같게 (20 m)"` directly on the right panel instead of inside the `"격자 설정"` dialog (`src/app/KaDrawingStudio.cpp:1482-1510`).
- **Missing tooltips & selection-driven inspector state (`crates/strata-app/src/gui_tab_drawing_right.rs`, `gui_tab_drawing_right_b.rs`)**: **MISSING** — only `1:5,000 이하에서 유적 이름으로 표시` (`gui_tab_drawing_right.rs:90`), `선택 항목 크기` (`gui_tab_drawing_right_b.rs:92`), and disabled `PDF 내보내기` (`:144`) have hover text; all other 13 controls in the right panel have **no tooltips**, and `updateInspector` selection-driven enable/sync does not exist.

---

## All Differences (Consolidated Table, Ordered `P1`–`P11`)

| part | what | C++ file:line | Rust file:line or `none` | kind |
|---|---|---|---|---|
| `P1_toolbar` | Bottom toolbar strip uses ~72×34/42 px rounded custom rect instead of `96×52 px` joined cells (`paperLook`) or `22 px`-spaced `QToolButton`s (stock) | `src/app/KaDrawingStudio.cpp:1284-1295`, `src/app/KaDrawingStudioPlate.cpp:105-120` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:15-38` | `DIFFERENT` |
| `P1_toolbar` | Button #5 (`항목 이동`) tooltip is `"범례·표제란·방위표·축척을 끌어 옮깁니다"` instead of `"좌표 상자를 끌어 옮깁니다"` | `src/app/KaDrawingStudio.cpp:1315` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:27` | `DIFFERENT` |
| `P1_toolbar` | Button #1 (`좌표점`) toggles off on second click and uses a different status string instead of always activating `m_toolCoordPoint` with magnet/shortcut hint | `src/app/KaDrawingStudio.cpp:1662-1675` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:85-94` | `DIFFERENT` |
| `P1_toolbar` | Button #4 (`화면 맞춤`) resets chrome rects via `replace_sheet_chrome()` and does not exit `coord_callout_mode` | `src/app/KaDrawingStudio.cpp:3623-3635` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:99-103` | `DIFFERENT` |
| `P1_toolbar` | Button #5 (`항목 이동`) status message omits corner-resize instruction (`"항목을 눌러 끌면 옮겨집니다."` vs `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."`) | `src/app/KaDrawingStudio.cpp:1896` | `crates/strata-app/src/gui_tab_drawing_items.rs:73` | `DIFFERENT` |
| `P1_toolbar` | Button #6 (`용지/방향`) accept path does not call `end_sheet_map_adjust()` | `src/app/KaDrawingStudio.cpp:447-454, 474` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:105-109`, `crates/strata-app/src/modal_paper_account.rs:115-137` | `DIFFERENT` |
| `P1_toolbar` | Button #7 (`도면 목록`) uses persistent `sheet_set_menu_open` flag instead of transient `InstantPopup` `QMenu` | `src/app/KaDrawingStudio.cpp:1321-1339` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:110-113`, `crates/strata-app/src/gui_tab_drawing_popups.rs:129-232` | `DIFFERENT` |
| `P2_coordinate_points` | Coordinate callout stores screen pixel `pos: [f32; 2]` and paints at `c.pos` instead of projecting ground `(mapX, mapY)` through `f.w2m`, freezing callouts on screen when map pans/zooms/resizes | `src/app/KaDrawingStudio.cpp:1796, 1823-1839` | `crates/strata-app/src/tab_drawing.rs:46-53`, `crates/strata-app/src/gui_tab_drawing_popups.rs:79, 94, 98-100` | `DIFFERENT` |
| `P2_coordinate_points` | Callout text is single-line 2-decimal `"A  X <E>  Y <N>"` (swapping Easting/Northing) instead of 2-line 3-decimal `"X=<Northing .3f>\nY=<Easting .3f>"` | `src/app/KaDrawingCoordCallout.cpp:35-37` | `crates/strata-app/src/gui_tab_drawing_popups.rs:89-95, 100` | `DIFFERENT` |
| `P2_coordinate_points` | Callout draws orange dot + plain line + fixed `(+8,-22)` pill instead of 4 `QgsLayoutItem`s (`box` `26×7.2 mm` with edge-flip, `let`, `arr`, solid `head`) | `src/app/KaDrawingCoordCallout.cpp:39-134` | `crates/strata-app/src/gui_tab_drawing_popups.rs:101-125` | `DIFFERENT` |
| `P2_coordinate_points` | Inside-map-frame check (`layoutMapItemContains` `0.8 mm` margin + `"지도 칸 안에서 찍으세요. 흰 용지에는 점이 안 생깁니다."`) | `src/app/KaDrawingStudio.cpp:1694-1698`, `src/core/LayoutService.cpp:1017-1020` | `none` (`crates/strata-app/src/gui_tab_drawing_popups.rs:71-80` accepts whole `sheet_rect`) | `MISSING` |
| `P2_coordinate_points` | `18 px` vector vertex/segment magnet (`closestVertex` / `closestSegmentWithContext`, `"  (자석)"`) & `importMapCoordCallouts` | `src/app/KaDrawingStudio.cpp:1405-1444, 1704-1779, 1803` | `none` | `MISSING` |
| `P2_coordinate_points` | Connecting `#C2410C` `0.45 mm` polygon `ka_coord_frame` when `>= 2` coordinate points exist | `src/app/KaDrawingStudio.cpp:1841-1889` | `none` | `MISSING` |
| `P2_coordinate_points` | Dragging/moving coordinate callout boxes on the sheet | `src/app/KaDrawingStudio.cpp:1315, 1893-1896` | `none` (`crates/strata-app/src/tab_drawing.rs:35-42`) | `MISSING` |
| `P2_coordinate_points` | Deleting/undoing coordinate callouts (right-click, `Delete`, `Ctrl+Z`, context menu `"마지막 점 지우기"` / `"점찍기 끝"`) & project/sheet-set persistence | `src/app/KaDrawingStudio.cpp:330-336, 1806-1821, 3225-3248, 3466-3483`, `src/app/KaDrawingTools.cpp:35-38` | `none` | `MISSING` |
| `P3_map_adjust` | Ending map adjust writes `"지도 조정을 끝냈습니다."` instead of `useSelectTool()` status `"항목을 눌러 옮기거나, 모서리를 끌어 가로·세로를 바꾸세요."` | `src/app/KaDrawingStudio.cpp:1896, 3603-3604` | `crates/strata-app/src/gui_tab_drawing_adjust.rs:53` | `DIFFERENT` |
| `P3_map_adjust` | Map drag pans immediately without `0.2 mm` deadzone, `48.0 DPI` fast preview, or layout undo command | `src/app/KaDrawingTools.cpp:85-100`, `src/app/KaDrawingStudio.cpp:3569-3588` | `crates/strata-app/src/gui_tab_drawing_adjust.rs:30-34` | `DIFFERENT` |
| `P3_map_adjust` | Wheel zoom uses factor `1.2` (`1/3` Ctrl) per frame capped at `500,000` instead of `1.25` (`0.35` Ctrl) with `220 ms` batching | `src/app/KaDrawingTools.cpp:51-52, 111-165` | `crates/strata-app/src/gui_tab_drawing_adjust.rs:11-12, 36-46` | `DIFFERENT` |
| `P3_map_adjust` | Right-click context menu on `ka_map` (`"지도 조정"`, `"그린 것 가운데로"`, `"지도조정끝"`) | `src/app/KaDrawingStudio.cpp:337-350` | `none` | `MISSING` |
| `P3_map_adjust` | Relayout coordinate callouts during/after `지도 조정` | `src/app/KaDrawingStudio.cpp:2954-2955, 3605-3606` | `none` | `MISSING` |
| `P4_fit_to_screen` | `화면 맞춤` button calls `replace_sheet_chrome()`, lacks no-op tolerance guard, and does not clear `coord_callout_mode` | `src/app/KaDrawingStudio.cpp:3623-3706` | `crates/strata-app/src/gui_tab_drawing_toolbar.rs:99-103`, `crates/strata-app/src/gui_tab_drawing_adjust.rs:77-90` | `DIFFERENT` |
| `P4_fit_to_screen` | `화면 맞춤` button status message uses `"표준 축척은 오른쪽 숫자 단추로 고르세요."` instead of `"지도 화면과 같은 범위·축척으로 맞췄습니다. 축척 1 : %1"` | `src/app/KaDrawingStudio.cpp:3633-3634` | `crates/strata-app/src/gui_tab_drawing_adjust.rs:86-89` | `DIFFERENT` |
| `P4_fit_to_screen` | Initial sheet open centers on survey area (intentional owner rule, `ORDER-SHEET0.yaml:22`) rather than `centerOnMapCanvas()` | `src/app/KaDrawingStudio.cpp:2287-2298`, `src/app/MainWindowExport.cpp:251` | `crates/strata-app/src/gui_tab_drawing_frame.rs:106-112` | `DIFFERENT` |
| `P4_fit_to_screen` | Relayout coordinate callouts after `화면 맞춤` | `src/app/KaDrawingStudio.cpp:3699-3700` | `none` | `MISSING` |
| `P5_moving_items` | Only 6 `SheetItem`s can be dragged; `ka_map` frame and `CoordCallout` boxes cannot be moved | `src/app/KaDrawingStudio.cpp:621-624, 2116` | `crates/strata-app/src/tab_drawing.rs:35-42`, `crates/strata-app/src/gui_tab_drawing_items.rs:47-64` | `DIFFERENT` |
| `P5_moving_items` | `kept_on_paper` clamps every dragged item (including `Legend`) inside `sheet_rect`, preventing parking the legend outside the paper | `src/core/LayoutService.cpp:1533-1535`, `tests/test_layout_sheet_rules.cpp:254-269` | `crates/strata-app/src/gui_tab_drawing_items.rs:14-17, 28, 60` | `DIFFERENT` |
| `P5_moving_items` | `Ctrl+Z` / `Ctrl+Y` on `MainTab::Drawing` runs `undo_map_action` / `redo_map_action` instead of sheet layout/placement undo & redo | `src/app/KaDrawingStudio.cpp:3477-3516` | `crates/strata-app/src/gui_shell.rs:155-160` | `DIFFERENT` |
| `P5_moving_items` | User moves stored as relative `item_offsets_mm` deltas over dynamic auto-rects (drifting when legend/scalebar resizes, not reset/quadrant-shifted on paper change, split between `replace_sheet_chrome` and `standard_chrome`) | `src/app/KaDrawingStudio.cpp:421-430, 511-540, 2729-2828` | `crates/strata-app/src/tab_drawing.rs:21, 89-93`, `crates/strata-app/src/gui_tab_drawing_items.rs:27-43`, `crates/strata-layout/src/sheet_drawing.rs:204-236` | `DIFFERENT` |
| `P5_moving_items` | Click-selection state, selection outline, and corner/edge resize handles (including `Legend` width and `ka_map` frame) | `src/app/KaDrawingStudio.cpp:621-624, 3062-3106` | `none` (`crates/strata-app/src/gui_tab_drawing_items.rs:54`) | `MISSING` |
| `P5_moving_items` | Arrow-key nudging of selected layout items | `src/app/KaDrawingStudio.cpp:621-624` (`QgsLayoutViewToolSelect`) | `none` | `MISSING` |
| `P5_moving_items` | `Delete` / `Backspace` key handling on `MainTab::Drawing` (neither sheet items nor layers delete; `delete_selected` returns if `!= MainTab::Map`) | `src/app/KaDrawingStudio.cpp:3225-3248, 3463-3475, 3518-3522` | `crates/strata-app/src/app_state_layer_remove.rs:109-112` | `MISSING` |
| `P5_moving_items` | `선택 항목 크기 %` (`25..=200 %`) is always enabled without selection and has **zero effect** on `ScaleBar`, `ScaleText`, and `CrsLabel` (3 of 4 items named in its tooltip) | `src/app/KaDrawingStudio.cpp:1143-1155, 3032-3060, 3074-3080` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:86-94`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:35-165` | `FAKE` |
| `P6_legend` | `legend_visible` defaults to `true`, drawing an empty `"범례"` box even when 0 layers are checked (vs C++ optional / owner rule: auto-place when a layer is checked) | `src/app/KaDrawingStudio.cpp:1358-1361, 2350-2363` | `crates/strata-layout/src/sheet_drawing.rs:82`, `crates/strata-app/src/gui_tab_drawing_legend.rs:56-92` | `DIFFERENT` |
| `P6_legend` | `[범례]` button toggles `legend_visible` on/off and lacks tooltip, status message, `end_sheet_map_adjust()`, and item selection | `src/app/KaDrawingStudio.cpp:908-918, 1606-1616` | `crates/strata-app/src/gui_tab_drawing_right.rs:31-47` | `DIFFERENT` |
| `P6_legend` | Legend omission filter fails to omit checked cadastral (`지적*`) & geology-structure layers and skips all rasters | `src/core/LayoutService.cpp:1370-1396, 1577` | `crates/strata-app/src/gui_tab_drawing_legend.rs:60-66` | `DIFFERENT` |
| `P6_legend` | `범례 제목` `TextEdit` omits placeholder `"제목을 입력하세요"` | `src/app/KaDrawingStudio.cpp:953` | `crates/strata-app/src/gui_tab_drawing_right.rs:70-72` | `DIFFERENT` |
| `P6_legend` | Legend width auto-grows up to `0.42 * map_rect.width()` (anchoring right edge so left `pos` shifts) and clips text instead of user-resizable width + `setAutoWrapLinesAfter` wrapping | `src/core/LayoutService.cpp:1491-1536` | `crates/strata-app/src/gui_tab_drawing_legend.rs:79-89, 134` | `DIFFERENT` |
| `P6_legend` | `DemColorRampLegend` horizontal elevation bar & categorized geology/soil sub-rows filtered to map extent | `src/core/DemColorRampLegend.cpp:1-226`, `src/core/LayoutService.cpp:1473-1484, 1568-1625` | `none` | `MISSING` |
| `P6_legend` | Legend `B` (bold) and `I` (italic) checkboxes mutate `legend_bold` / `legend_italic` fields that are never read when rendering the legend | `src/app/KaDrawingStudio.cpp:965-969, 2368-2376` | `crates/strata-app/src/gui_tab_drawing_right.rs:83-84`, `crates/strata-app/src/gui_tab_drawing_legend.rs:116-140` | `FAKE` |
| `P7_title_block` | `title_block_visible` defaults to `true` on new sheets instead of optional/off | `src/app/KaDrawingStudio.cpp:938`, `tests/test_drawing_studio.cpp:232` | `crates/strata-layout/src/sheet_drawing.rs:83` | `DIFFERENT` |
| `P7_title_block` | `[표제란]` button toggles visibility and lacks tooltip, status message, `end_sheet_map_adjust()`, and item selection | `src/app/KaDrawingStudio.cpp:939-948`, `src/app/KaDrawingStudioSheets.cpp:35-69` | `crates/strata-app/src/gui_tab_drawing_right.rs:48-66` | `DIFFERENT` |
| `P7_title_block` | Title block `조사명` & `유적명` read `real_survey.survey_name` / `site_name` (always `""` at runtime) instead of scanning `"survey_area"` features' `"survey_name"` / `"site_name"` fields | `src/app/KaTitleBlock.cpp:33-55` | `crates/strata-app/src/gui_tab_drawing_chrome.rs:64-72`, `crates/strata-app/src/real_survey_load.rs:16-17` | `DIFFERENT` |
| `P7_title_block` | Screen title block rect ignores `default_title_block_rect_mm` Y position and hardcodes `map_rect.bottom() + 8.2 * px_per_mm` (pushing `ka_crs` out of `chrome.crs`) | `src/app/KaTitleBlock.cpp:67-77` | `crates/strata-app/src/gui_tab_drawing_chrome.rs:92-106`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:155` | `DIFFERENT` |
| `P7_title_block` | `QInputDialog` / UI to edit `도면명` (`self.drawing.title_ko`) while keeping user-moved position | `src/app/KaDrawingStudioSheets.cpp:44-54`, `src/app/KaTitleBlock.cpp:103-108` | `none` (`crates/strata-app/src/tab_drawing.rs:10, 69`) | `MISSING` |
| `P8_north_scale` | North arrow `"크기"` (`3..=80 mm`) is clamped to `0.8..=1.35×` and `12..=36 px` on screen | `src/app/KaDrawingStudio.cpp:1024-1039` | `crates/strata-app/src/gui_tab_drawing_chrome.rs:32-36` | `DIFFERENT` |
| `P8_north_scale` | Default `scale_denominator` is `5_000` instead of `1000`, and `DragValue` mutates scale live before `"적용"` / `Enter` | `src/app/KaDrawingStudio.cpp:1061-1075` | `crates/strata-layout/src/sheet_drawing.rs:72`, `crates/strata-app/src/gui_tab_drawing_right.rs:156-159` | `DIFFERENT` |
| `P8_north_scale` | 14 scale preset buttons are plain non-checkable buttons (never highlight matching scale) and set no status message | `src/app/KaDrawingStudio.cpp:1089-1103, 3016-3026` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:19-46` | `DIFFERENT` |
| `P8_north_scale` | Scale bar buttons lack tooltips/status and scale bar labels only ticks `[0, 2, 4]` instead of `0..=4` | `src/app/KaDrawingStudio.cpp:300, 1126-1138, 1629-1638` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:50-80`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:113` | `DIFFERENT` |
| `P8_north_scale` | Coordinate grid defaults to ON (`grid_enabled: true`) with no UI to turn it off, no `1.4 mm` Zebra frame, top-only X labels, and `.take(24)` line cap | `src/app/KaDrawingStudio.cpp:1550-1558`, `src/core/LayoutService.cpp:799-874` | `crates/strata-layout/src/sheet_drawing.rs:73`, `crates/strata-app/src/gui_tab_drawing_grid.rs:23-111` | `DIFFERENT` |
| `P8_north_scale` | `"지도 좌표격자와 같게 (20 m)"` is always enabled and hardcodes `20.0 m` step and `[0.0, 0.0]` origin instead of reading map tab's actual grid step, modulo origin offset, and rotation | `src/app/KaDrawingStudio.cpp:176-184, 1481-1488, 2016-2020`, `src/core/LayoutService.cpp:752-764` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:108-130`, `crates/strata-app/src/gui_tab_drawing_grid.rs:24-43` | `DIFFERENT` |
| `P8_north_scale` | North arrow tooltips (`gridNorthNote`), status text, font-free glyph-outline SVG, and `GridNorth` map-rotation link | `src/app/KaDrawingStudio.cpp:1000, 1015, 1626, 2421-2446`, `src/app/KaDrawingNorthArrow.cpp:25-40, 205-207` | `none` (`crates/strata-app/src/gui_tab_drawing_right.rs:95-132`, `crates/strata-app/src/gui_tab_drawing_decoration.rs:168-256`) | `MISSING` |
| `P8_north_scale` | Modal `"격자 설정"` dialog (enable checkbox, custom interval spinbox `0..100,000 m`, `"칸 밖에 좌표 숫자"` checkbox) | `src/app/KaDrawingStudio.cpp:1460-1518` | `none` | `MISSING` |
| `P8_north_scale` | Scale `"적용"` button only writes `draw_guide_message` without changing the sheet (`DragValue` already mutated `scale_denominator`) | `src/app/KaDrawingStudio.cpp:1065-1075, 3121-3137` | `crates/strata-app/src/gui_tab_drawing_right.rs:156-165` | `FAKE` |
| `P8_north_scale` | `"축척 글자"` and `"좌표계"` buttons are enabled clickable buttons whose click handler is gated out by `&& i == 2` | `src/app/KaDrawingStudio.cpp:1160-1167, 1640-1660` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:98-107` | `FAKE` |
| `P9_paper_and_sheet_set` | Changing paper does not end map adjust, reset/quadrant-shift `item_offsets_mm`, or relayout `coord_callouts` | `src/app/KaDrawingStudio.cpp:474, 511-545` | `crates/strata-app/src/modal_paper_account.rs:115-137`, `crates/strata-app/src/tab_drawing.rs:89-93` | `DIFFERENT` |
| `P9_paper_and_sheet_set` | `StoredSheet` is in-memory only (not persisted), stores only 6 fields, and opening it fails to restore `paper_w_mm`/`paper_h_mm`, map center, item positions, callouts, or decorations | `src/app/KaDrawingSheetSet.cpp:51-121`, `src/app/KaDrawingStudioSheets.cpp:122-155` | `crates/strata-app/src/tab_drawing.rs:57-64`, `crates/strata-app/src/gui_tab_drawing_popups.rs:207-230` | `DIFFERENT` |
| `P9_paper_and_sheet_set` | Name input dialog (`QInputDialog`), duplicate overwrite confirmation, and open confirmation in `도면 목록` | `src/app/KaDrawingStudioSheets.cpp:77-91, 109-115` | `none` (`crates/strata-app/src/gui_tab_drawing_popups.rs:206-231`) | `MISSING` |
| `P9_paper_and_sheet_set` | Named layer visibility/style sets (`KaDrawingLayerSets`) saved, restored, and deleted alongside stored sheets | `src/app/KaDrawingLayerSets.cpp:1-109`, `src/app/KaDrawingStudioSheets.cpp:103, 146-147, 168` | `none` | `MISSING` |
| `P9_paper_and_sheet_set` | `"보관한 도면 지우기"` submenu and confirmation (`removeStoredSheet`) | `src/app/KaDrawingSheetSet.cpp:152-158`, `src/app/KaDrawingStudioSheets.cpp:158-170` | `none` | `MISSING` |
| `P9_paper_and_sheet_set` | Disabled footer note `"제출 PDF는 지금 열린 도면으로 만듭니다"` in `도면 목록` | `src/app/KaDrawingSheetSet.cpp:161` | `none` | `MISSING` |
| `P10_output` | `"PDF 내보내기"` (`300 DPI` `PreferText` vector PDF, `"도면.pdf"` save dialog, `KaDrawingPdfProgress` cancel/atomic commit) | `src/app/KaDrawingStudio.cpp:3758-3855`, `src/core/PdfExportSettings.cpp:9-26`, `src/app/KaDrawingPdfProgress.cpp:10-84` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:144` (disabled `지시 R3`), `crates/strata-layout/src/sheet_pdf_export.rs:27-56` | `MISSING` |
| `P10_output` | `Ctrl+P` shortcut opening `printDrawing()` on `MainTab::Drawing` | `src/app/KaDrawingStudio.cpp:935-937`, `src/app/MainWindowExport.cpp:186-194` | `none` (`crates/strata-app/src/gui_shell.rs:150-169`) | `MISSING` |
| `P10_output` | Print dialog `"붙였을 때 크기"` output scale/ISO enlargement combo (`printOutput`), scale-change warning (`printWarning`), interactive tile skip (`toggleSheet`), and `"모두 다시 넣기"` (`printPickAll`) | `src/app/KaPrintDialog.cpp:122-125, 184-201, 252-305` | `none` (`crates/strata-app/src/dlg_drawing_print_options.rs`) | `MISSING` |
| `P10_output` | `"나눈 장을 PDF로 저장…"` (`<title>-나눠찍기.pdf` multi-page tiled PDF export) | `src/app/KaPrintDialog.cpp:204-206, 592-620` | `crates/strata-app/src/dlg_drawing_print_actions.rs:45` (disabled `지시 W-50`) | `MISSING` |
| `P10_output` | Submission package `조사도면.pdf` (`300 DPI` export of composed `user_sheet` + `README_submit.txt` `hasSheetPdf` + `MANIFEST.sha256`) | `src/core/ExportService.cpp:115, 195-215, 226-228`, `src/core/LayoutService.cpp:1168-1312` | `crates/strata-check-export/src/submit_builder.rs:123-136`, `crates/strata-app/src/modal_export_footer.rs:40` (disabled `지시 R2`) | `MISSING` |
| `P10_output` | Print dialog `"프린터"` (`"ezViewX Virtual Image Printer              ▼"`) and `"프린터 속성..."` are inert hardcoded buttons with no OS printer query or dialog | `src/app/KaPrintDialog.cpp:88-108, 237, 636-642` | `crates/strata-app/src/dlg_drawing_print_options.rs:22-38` | `FAKE` |
| `P10_output` | Print dialog preview draws 85 procedural dots, an orange bar, `"축척 1 : 6532"`, and `"동부원점(GRS80) · EPSG:5187"` instead of the actual sheet & tile grid | `src/app/KaPrintDialog.cpp:78-83, 189-191`, `src/app/KaTilePreview.cpp:1-213` | `crates/strata-app/src/dlg_drawing_print_preview.rs:12-124` | `FAKE` |
| `P10_output` | Print dialog `"인쇄 · {page_cnt}장"` button compares `print_paper` to itself, sets a status string, closes the modal, and sends nothing to any printer | `src/app/KaPrintDialog.cpp:622-669` | `crates/strata-app/src/dlg_drawing_print_actions.rs:57-83` | `FAKE` |
| `P11_right_panel` | Right panel is a fixed non-resizable `268.0 px` `SidePanel` instead of splitter column `260..420 px` (`initial 320 px`) | `src/app/KaDrawingStudio.cpp:1251-1266` | `crates/strata-app/src/gui_tab_drawing_right.rs:11-14` | `DIFFERENT` |
| `P11_right_panel` | `"PDF 내보내기"` and `"인쇄"` scroll inside `ScrollArea::vertical()` instead of sitting in fixed bottom `studioOutputFoot` (`paperLook`) | `src/app/KaDrawingStudioPlate.cpp:76-102` | `crates/strata-app/src/gui_tab_drawing_right.rs:15-18`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:132-157` | `DIFFERENT` |
| `P11_right_panel` | Group headers draw full-width `ui.separator()` above plain labels instead of inline `1 px` `studioCaptionRule` hairline leaders and `< 860 px` `fitToHeight` compaction | `src/app/KaDrawingStudioPlate.cpp:47-65, 172-194` | `crates/strata-app/src/gui_tab_drawing_right.rs:25-29, 94-99, 146-151` | `DIFFERENT` |
| `P11_right_panel` | `"지도 좌표격자와 같게 (20 m)"` checkbox sits directly on the right panel instead of inside the `"격자 설정"` dialog | `src/app/KaDrawingStudio.cpp:1482-1510` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:127-130` | `DIFFERENT` |
| `P11_right_panel` | Tooltips on 13 right-panel controls (`범례`, `표제란`, `방위 (도북)` + 4 swatches, 3 scale bar tiles, `축척 글자`, `좌표계`, `좌표 격자`, `인쇄`) & selection-driven `updateInspector` sync | `src/app/KaDrawingStudio.cpp:300, 912, 931, 935, 943, 1000, 1015, 1564-1570, 3062-3106` | `none` (`crates/strata-app/src/gui_tab_drawing_right.rs`, `crates/strata-app/src/gui_tab_drawing_right_b.rs`) | `MISSING` |

---

## Could not determine
1. **`도면 목록` rename action (`ORDER-SHEET0.yaml:35`)**: **`not in the source`** — `KaDrawingSheetSet` (`src/app/KaDrawingSheetSet.h:1-41`, `src/app/KaDrawingSheetSet.cpp:1-164`) and `src/app/KaDrawingStudioSheets.cpp:71-170` define `store`, `restore`, `remove`, `list`, `contains`, and `fillMenu`, with no rename function or menu entry.
2. **Exact arrow-key nudge distance in millimetres/pixels and internal snap-guide pixel tolerance (`ORDER-SHEET0.yaml:31`)**: **`not in the source`** of Strata (`target/cpp-ref`) — `KaDrawingStudio` delegates arrow-key item nudging, resize-handle hit zones, and item alignment/page-edge snap guides directly to external QGIS library class `QgsLayoutViewToolSelect` (`src/app/KaDrawingStudio.cpp:621-624`), whose implementation lives in the external OSGeo4W QGIS source tree outside `target/cpp-ref`.

---

## Counts

| part | total | `SAME` | `DIFFERENT` | `MISSING` | `FAKE` |
|---|---:|---:|---:|---:|---:|
| `P1_toolbar` | 10 | 3 | 7 | 0 | 0 |
| `P2_coordinate_points` | 9 | 1 | 3 | 5 | 0 |
| `P3_map_adjust` | 8 | 3 | 3 | 2 | 0 |
| `P4_fit_to_screen` | 6 | 2 | 3 | 1 | 0 |
| `P5_moving_items` | 8 | 0 | 4 | 3 | 1 |
| `P6_legend` | 9 | 2 | 5 | 1 | 1 |
| `P7_title_block` | 6 | 1 | 4 | 1 | 0 |
| `P8_north_scale` | 13 | 3 | 6 | 2 | 2 |
| `P9_paper_and_sheet_set` | 7 | 1 | 2 | 4 | 0 |
| `P10_output` | 9 | 1 | 0 | 5 | 3 |
| `P11_right_panel` | 6 | 1 | 4 | 1 | 0 |
| **Total** | **91** | **18** | **41** | **25** | **7** |
