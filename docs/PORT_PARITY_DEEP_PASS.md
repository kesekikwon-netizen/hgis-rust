# C++ to Rust deep parity pass

Date: 2026-11-18. Rust tip read: `f1835fa` (`main`). C++ pin: `kesekikwon-netizen/gogohak1` `b036c0905f6ac1e97b6d142a5e101ac5d30ca7c7`, cloned read-only at `/tmp/cpp-ref`. No product code was changed. No Linux build was run.

This file lists gaps that are not already rows in [PR #1](https://github.com/kesekikwon-netizen/hgis-rust/pull/1) `docs/PORT_PARITY_CHECKLIST.md` and `docs/STATUS_ASSESSMENT.md`. Those two files are not on `main` yet. Work units here start at WU-13 so they do not collide with WU-01 through WU-12 in that checklist.

Status words match that checklist: `missing`, `partial`, `divergent`.

Severity:

| Word | Meaning |
|---|---|
| `blocker` | A field result cannot be produced, or the app claims a result it did not make. None of the new rows are blockers. The false success paths (trench lay, contour 248, submit package) are already WU-04, WU-05, and WU-06. |
| `major` | A named command, shortcut, or file the field uses does the wrong thing or nothing. |
| `minor` | Wording, a second entrance to a command that already exists, or a helper that the running app does not call. |

## How the list was made

Every `QStringLiteral` in `/tmp/cpp-ref/src` was taken with the call beside it (`addAction`, `setToolTip`, `setWindowTitle`, `QMessageBox`, `showMessage`, `QFileDialog`). `QKeySequence::` names and `Qt::Key_` names were taken from the same tree. Each Korean action string was then searched in `crates/**/*.rs`.

Counts from that pass:

| What was counted | Count |
|---|---|
| Action-label occurrences | 113 |
| Unique action labels | 108 |
| Unique action labels with no equal string in Rust | 21 |
| Distinct shortcut tokens | 34 |
| Short tooltip, title, message, status, and file-dialog strings with no equal string in Rust | 617 |

Most of the 617 are `%1` templates, home cards the folder explorer replaced, or the old topographic browser. They were not copied here. The 21 missing action labels were opened one by one. Labels that PR #1 already names (한글 보고서 buttons, 복구 사본, 웹 자료, 전체 화면 as a bare missing command, 안쪽 도형만큼 구멍 내기, the work-CRS menu sentences) stayed out. What remained was read in both trees before it was written down.

The decision trail is [PORT_PARITY_DEEP_PASS_LOG.tsv](PORT_PARITY_DEEP_PASS_LOG.tsv).

## Checked, and not a new gap

- The tape HUD is present. Distance, area, finish, copy, undo one point, and again are drawn and wired (`gui_tab_map_measure.rs:157-228`). The snap source under that HUD is DP-16.
- Legend drag is present (`gui_layer_panel_rows.rs:57`, `app_state_layer_order.rs:22`). The checklist still repeats the 2026-11-14 sentence that drag is missing. That sentence is stale. It is not filed again as a gap.
- Domain default colours for the seven survey keys match `LayerStyleDefaults::forKey` (`survey_style.rs:106-139`, `LayerStyleDefaults.cpp:42-77`). The extra looks on top of those defaults are DP-13.
- Delete on a legend row removes the row (`app_state_layer_remove.rs`). F9 and F10 fold the side panels (`gui_shell.rs:43-63`). Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S, Ctrl+1, Ctrl+D, Ctrl+L, Ctrl+P, Ctrl+E, Ctrl+Z, and Ctrl+Y are in `gui_shell.rs:91-124`.

## New items

| Id | Area | One line | Status | Severity | C++ | Rust |
|---|---|---|---|---|---|---|
| DP-01 | Shortcuts | The search field says Ctrl+F. The window shortcut that focuses it is not bound. | missing | major | `MainWindowRibbon.cpp:240-244` (`QKeySequence::Find`) | `gui_ribbon_appbar.rs:30-39` shows the chord. `gui_shell.rs:91-124` has no `Key::F`. |
| DP-02 | Shortcuts | F11 is full screen in C++. Rust F11 toggles map focus, and the message tells the user to press Ctrl+F11, which does nothing. | divergent | major | `MainWindow.cpp:270-278` (`Qt::Key_F11`). Map focus is `KaShellFocus.h:27` `Ctrl+F11`. | `gui_shell.rs:65-74` |
| DP-03 | Shortcuts | F2 on the layer list renames the selected layer. | missing | minor | `MainWindow.cpp:2379-2381` | none |
| DP-04 | Shortcuts | While 도형선택 is on, Ctrl+C copies cadastral parcels and Ctrl+V pastes them on a layer named 지적도 · 복사한 필지. | missing | major | `KaParcelCopyKeys.cpp:23-61` | `submit_audit.rs:64-66` writes a TSV for the checklist only. No map copy or paste. |
| DP-05 | Menus | The sheet map's context menu offers 지도 조정, 그린 것 가운데로, and 지도조정끝, and while points are placed it also offers 점찍기 끝. Rust has no sheet context menu. A right click while placing points only drops the last point, which matches the coordinate tool. | divergent | minor | Menu: `KaDrawingStudio.cpp:330-349`, installed at `:857`. Tool right click: `KaDrawingTools.cpp:35-38`. | `gui_tab_drawing_coords.rs:73-74`. The same commands are toolbar buttons at `gui_tab_drawing_toolbar.rs:23-27`. |
| DP-06 | Menus | 깨진 한글 바로잡기 is drawn as a live submenu (CP949, EUC-KR, UTF-8, 이 컴퓨터 설정). The click handler has no arm for it. | partial | major | `MainWindowContextMenus.cpp:350-352` | Shown at `layer_menu_model.rs:190-198`. `run_layer_menu_action` (`gui_layer_panel_context_menu.rs:62-113`) does not match `layer.encodingValue`. |
| DP-07 | Dialogs | Finishing a sketch on top of another shape of the same layer opens 겹친 곳 지우기 and 그대로 두기. Closing the menu keeps the new shape. | missing | major | `MainWindowErase.cpp:120-147` | `app_state_draw_commit.rs:4` says the offer is not there. The later button path is `app_state_erase.rs`. |
| DP-08 | Error messages | Field errors use a three-part box, 무엇이 안 됐나, 왜 그런가, 어떻게 하나, with 닫기 and 로그 폴더 열기. | missing | major | `KaUserError.cpp:34-40`, `:89` | none |
| DP-09 | Dialogs | The CAD notice offers 다른 위치로 바꾸기 with a ranked CRS list, 좌표 없는 도면으로 보기, and 직접 맞추기. Rust shows the work CRS and two other button labels, then says the drawing was not added. | divergent | major | `KaCadImportNotice.cpp:40-68`. Candidates and names: `CadCrsGuess.cpp:52-84`. | `dlg_cad_import_notice.rs:43-71` |
| DP-10 | Status bar | The submit chip tooltip in C++ says the files always leave as the upload CRS and that a click opens 검수·제출. Rust says the package is converted to EPSG:5179. | divergent | minor | `KaStatusBar.cpp:202-205` | `shell_workspace.rs:236-238` |
| DP-11 | Undo | On the sheet, Ctrl+Z undoes a layout move, then a just-added item, and says 한 단계 되돌렸습니다, 방금 넣은 항목을 되돌렸습니다, or 되돌릴 것이 없습니다. Rust on the sheet undoes map edits, except while coordinate points are being placed. | missing | major | `KaDrawingStudio.cpp:3476-3507` | `gui_shell.rs:116-123` |
| DP-12 | Layer order | Without the history-GIS key, C++ leaves the 1919 layer in the project and disables the menu row. Rust drops the row when the list is restored, so a later save forgets it. | divergent | major | `MainWindow.cpp:1425-1434` greys `actionMap1919`. | `survey_map_list.rs:160-168` returns false and skips the row. |
| DP-13 | Symbols | 유구 면 can be 종류별 자동 색 (stable colour per kind, plus 미분류) or 시대색·종류 무늬 (period colour and kind hatch from `data/styles/feature_presets.json`). The map paints one green for every feature polygon. The legend command 면·외곽선 색 opens the line-colour dialog, which has no 점선, no 종류별 자동 색, and no 시대색·종류 무늬. The inspector says 채우기 없음 (선만) and 선 없음. C++ says 채우기 없음 (외곽선만) and 외곽선 없음. | missing | major | `MainWindowEditing.cpp:485-544`. Kind colours: `LayerStyleKinds.h:9-36`. Presets: `data/styles/feature_presets.json`. | One fill at `survey_style.rs:120-124`, painted per table at `survey_paint_layers.rs:183`. Dialog: `modal_style.rs:11-35`. Inspector words: `ri_layer_card.rs:215-221`. |
| DP-14 | Layer order | `enforce_standard_layer_z_order` sorts ten invented roles and its comment names `LayerTreePolicy::getTargetZOrderIndex` and `enforceStandardHierarchy`. C++ `LayerTreePolicy` only writes the fixed-order note. The app does not call the sorter. Wiring it would reorder the flat list. | divergent | minor | `LayerTreePolicy.h:16-19`, `LayerTreePolicy.cpp:8-20` | `layer_policy.rs:7-45`. Call sites are tests only. |
| DP-15 | Import | A topographic NGI file passes the suffix check. The engine GDAL build has no NGI driver, so the file cannot be opened. Enabled drivers are shape, gpkg, dxf, hfa, gtiff, jpeg, png. | partial | major | `TopographicCatalog.cpp:41-44` treats `.ngi` as a sheet, same as `.shp`. | Accepted at `strata-topo/src/archive.rs:176`. Drivers: `strata-engine/Cargo.toml:30`. |
| DP-16 | Measure | The tape clicks snap to `scene.polygons` plus cadastral and heritage rings whose first vertex is within 300 m, and a snap marker is stored on every click. C++ snaps with the canvas snapping utils and shows the marker only on a hit. | partial | major | `KaMeasureMapTool.cpp:226-235` | `tabs_home_map_state.rs:213-234`, `cad_session.rs:110-121` and `:162-164` |
| DP-17 | Tools | The trench move tool nudges the selected trench, or the whole grid, by one grid step with the arrow keys (1 m when the grid is off) and records undo. Arrow keys on the map move nothing. | missing | major | `KaTrenchMoveToolKeys.cpp:45-67` | `gui_shell_keys_tests.rs:9-11` states the tool is absent. |
| DP-18 | Tools | The 3D studio panel has strata-3d로 열기 and strata-3d 도면 받기. They hand the open survey to the external strata-3d app and bring a PNG or PDF back. | missing | major | `KaTerrain3dStudio.cpp:125`, `KaStrata3dLink.cpp:17-28` | none |

Eighteen new items. By area: shortcuts 4, menus 2, dialogs 2, error messages 1, status bar 1, undo 1, layer order 2, symbols 1, import 1, measure 1, tools 2. By severity: major 14, minor 4, blocker 0.

## Work units

Each unit is one change. A test should fail before the change and pass after it. Do not write the user's survey files. Do not log in to a live site. Do not pull LibreDWG in. Items already numbered WU-01 through WU-12 stay in the PR #1 checklist.

### WU-13 Bind the four shortcuts

- DP-01. Ctrl+F focuses the address field, as `QKeySequence::Find`.
- DP-02. F11 is full screen. Map focus stays Ctrl+F11, and the status sentence names that chord only when that chord was used.
- DP-03. F2 renames the legend row when the list was clicked last, the same focus rule as Delete on the list.
- DP-04. Ctrl+C and Ctrl+V during 도형선택 copy and paste cadastral parcels. A non-parcel selection leaves the previous copy in place and says 지적 필지를 도형선택으로 고른 뒤 Ctrl+C 하세요.
- Check: the four chords in a headless egui frame. Text fields still receive Ctrl+C, Ctrl+V, and Ctrl+F.

### WU-14 Sheet map menu and sheet undo

- DP-05. Right click on a sheet map frame opens the C++ menu, with the C++ labels. While points are being placed, the menu includes 점찍기 끝. The existing right click that drops the last point stays available as 마지막 점 지우기.
- DP-11. Ctrl+Z on the sheet undoes a decoration move before it undoes map edits. The three status sentences are the C++ ones.
- Check: move the legend, Ctrl+Z, and the legend is back. Place two coordinate points, open the menu, and both 마지막 점 지우기 and 점찍기 끝 are present.

### WU-15 The two automatic looks on 유구 면

- DP-13. 면·외곽선 색 on a survey polygon opens the vector style dialog, not the line-colour dialog. The dialog has 채우기 없음 (외곽선만), 외곽선 없음, 점선, 종류별 자동 색, and 시대색·종류 무늬. The two automatic looks are exclusive. The map painter reads the saved mode. Kind hatch and period colour come from `data/styles/feature_presets.json`. A kind with no period stays visible as 미분류.
- The line-colour dialog stays the dialog for line reference maps.
- Check: two polygons with kinds 주거지 and 수혈 do not share one green after 시대색·종류 무늬. Cancel restores the previous look.

### WU-16 CAD CRS candidates

- DP-09. After a CAD file is chosen, the notice lists `CadCrsGuess` candidates other than the one in use, then 좌표 없는 도면으로 보기, and the button 직접 맞추기. Labels are the C++ strings in `CadCrsGuess.cpp:62-84`, starting with GRS80 중부원점(2010).
- This unit does not implement DXF drawing. Until that load exists, choosing a candidate must not say the file was added.
- Check: a fixture header that names 베셀 중부원점 offers EPSG:5174, and the work CRS is not the only line.

### WU-17 Tape snap

- DP-16. A measure click uses the same snap candidates as the draw tool (survey vertices and segments that snap is allowed to use, cadastral lines, no satellite picture). The marker appears only when the point moved to a hit.
- Check: a click 2 m from a survey vertex, inside the pixel tolerance, lands on that vertex. A click in empty ground keeps the cursor point and shows no marker.

### WU-18 Trench move

- DP-17. Arrow keys move the selected trench by one grid step, or by 1 m when the grid is off. Shift is not a second speed. The status sentence is `%1 한 칸 옮김 — Ctrl+Z로 되돌립니다.` The move is one undo step. Arrow keys do nothing in every other tool.
- Check: the existing test that an arrow moves no trench stays true outside the tool, and a new test shows one cell of movement inside it.

### WU-19 strata-3d handoff

- DP-18. The two buttons use the C++ labels and tooltips. They refuse to start when no survey file is open, with 열려 있는 조사 파일(.gpkg)이 없습니다. 조사를 저장하거나 연 뒤에 눌러 주세요.
- The external process is not started in tests. A fake outcome covers success and failure sentences.
- This is not the in-app 화면을 입체로 button. That one stays the disabled entry already on the checklist.

### WU-20 Shapefile code page

- DP-06. Choosing CP949, EUC-KR, UTF-8, or 이 컴퓨터 설정 rewrites the layer's text encoding the way `MainWindowContextMenus.cpp:350-357` does, and the map labels use the new text.
- Check: a fixture `.dbf` that is CP949 shows the Korean name after the menu pick and the previous bytes before it.

### WU-21 Ask before erasing a same-layer overlap

- DP-07. When a finished sketch overlaps another polygon of the same layer, the menu is 겹친 곳 지우기 and 그대로 두기. Escape or 그대로 두기 keeps the new shape. A sketch over a different layer does not ask.
- Check: two polygons on `feature_poly`. The menu appears. 그대로 두기 leaves both. 겹친 곳 지우기 matches the current erase status sentence.

### WU-22 Field error box

- DP-08. One box, used by the existing warn and critical call sites: the three headings, the optional 자세히 text, 닫기, and 로그 폴더 열기 when a log folder is set. The button opens that folder. Tests replace the opener so no file window appears.
- DP-10. The submit chip tooltip is the C++ sentence with the upload CRS filled in.
- Check: a forced save failure shows 무엇이 안 됐나 and does not show a one-line status instead.

### WU-23 Keep the 1919 row

- DP-12. Restoring the map list without a history-GIS key keeps the 1919 row. The menu entry that would add it stays disabled, with the tooltip 더보기 → API 키 입력에 역사지리정보DB 키를 저장하면 켤 수 있습니다. Saving again does not delete the row.
- Check: a temp map list that contains the 1919 key, an empty key store, reopen, and the row is still in the file.

### WU-24 Read NGI sheets

- DP-15. An `.ngi` file that `TopographicCatalog` would accept is read into the same sheet tables as a shapefile of the same geometry. If the GDAL build cannot grow an NGI driver without a new copyleft component, the picker must refuse `.ngi` with the C++ sentence that names the formats it can open, and it must not list NGI among them.
- Check: either a tiny NGI fixture round-trips, or the picker error no longer offers NGI.

### WU-25 Do not sort by the invented roles

- DP-14. Delete `enforce_standard_layer_z_order` and `LayerRole`, or stop describing them as `LayerTreePolicy`. The fixed-order notes in `forced_layer_order_note_ko` stay. They match `LayerTreePolicy.cpp:11-19`.
- Check: the legend order test that only calls `move_layer_row` still passes, and nothing in `strata-app` sorts by those ten roles.

WU-13, WU-20, WU-21, and WU-23 touch different files and can run together. WU-15 should land before anyone treats the CAD notice as a finished import. WU-16 should use the snap set WU-13 does not change. WU-24 is the one that might add a GDAL driver. Check the licence before adding it.
