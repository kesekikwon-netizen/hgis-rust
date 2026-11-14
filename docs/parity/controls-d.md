# Controls Parity Audit — Group D (Drawing Studio, Print, PDF, Grid, Context Menus)

Target C++ reference: `target/cpp-ref/src/app/` (read-only).
Target Rust workspace: `crates/strata-app/src/` and `crates/strata-layout/src/`.

---

## Summary Index

| ID | Window / Menu (C++ Title) | C++ Source | Rust Counterpart | Status |
|---|---|---|---|---|
| D1 | `도면` (Drawing Studio) | `KaDrawingStudio.cpp:374, :649, :1279` | `gui_tab_drawing.rs:38` (+ `gui_tab_drawing_*.rs`, `gui_layer_panel_tree.rs`) | Partial (several missing / stubbed / extra controls) |
| D2 | Drawing message & prompt boxes | `KaDrawingStudio.cpp:3294, :3821, :3841, :3854, :3860, :3866, :3887`; `KaDrawingStudioSheets.cpp:47, :82, :88, :99, :109, :125, :160` | `gui_tab_drawing_popups.rs:200-226` (status-bar only; no modal dialogs) | **MISSING** (all 10+ message/input boxes) |
| D3 | `도면 인쇄` (+ progress, printer-properties, result boxes) | `KaPrintDialog.cpp:74, :578, :614, :637, :654, :667`; `KaPrintDialogPlate.cpp:60` | `dlg_drawing_print.rs:24`, `dlg_drawing_print_options.rs`, `dlg_drawing_print_preview.rs`, `dlg_drawing_print_actions.rs` | Partial (static preview, hardcoded printer, disabled PDF, missing sub-dialogs) |
| D4 | `PDF 내보내기` / `인쇄 준비` (PDF preparation progress) | `KaDrawingPdfProgress.cpp:10-24` | `dlg_pdf_preparing.rs:10` | Partial (hardcoded text & progress bar, no real cancellation) |
| C12 | `격자 설정` | `KaDrawingStudio.cpp:1460-1518` | Not found (`gui_tab_drawing_right_b.rs:127-130` has inline checkbox only) | **MISSING** |
| CTX-L | Drawing Studio Layer Tree Context Menu | `KaDrawingStudio.cpp:734-743`; `MainWindowContextMenus.cpp:176-448` | `gui_layer_panel_context_menu.rs:13`, `layer_menu_model.rs` | Present (shared with Map tab; several actions deferred/missing) |
| CTX-M | Drawing Studio Map Box Context Menu | `KaDrawingStudio.cpp:315-355, :857` (`KaLayoutMenuProvider`) | Not found | **MISSING** |

---

## D1. 「도면」 (Drawing Studio)

- **C++ title**: `"도면"` (`KaDrawingStudio.cpp:374`, tab title `MainWindowExport.cpp:246`).
- **How opened**:
  - **C++**: Clicking ribbon button `"도면"` → `MainWindow::openDrawingStudio()` (`MainWindowExport.cpp:200-255`), which prompts paper via `KaDrawingStudio::promptPaper` (`MainWindowExport.cpp:216`) on first open and embeds `KaDrawingStudio` in `m_viewTabs` (`MainWindowExport.cpp:245`).
  - **Rust**: Clicking ribbon button `"도면"` (`gui_ribbon_click_b.rs:122-131`) opens `ActiveModalDialog::PaperSetup` on first open (or switches to `MainTab::Drawing` if already open); also `Ctrl+P` opens `MainTab::Drawing` + `DrawingPrint` (`gui_shell.rs:141-146`).
- **Modality**:
  - **C++**: Non-modal embedded `QMainWindow` tab inside `m_viewTabs` (`MainWindowExport.cpp:243-247`; standalone constructor `KaDrawingStudio.cpp:369-375`).
  - **Rust**: Non-modal embedded workspace tab (`MainTab::Drawing`, `shell_workspace.rs:121`).
- **Size rules**:
  - **C++**: `resize(1280, 860)` (`KaDrawingStudio.cpp:375`); left column `minimumWidth(160)`, `maximumWidth(720)` (`:657-658`), `m_layerTree->setMinimumWidth(300)` (`:723`); right inspector `minimumWidth(260)`, `maximumWidth(max(420, inspectorMinWidth))` (`:1253-1254`); `m_studioSplit` sizes `{348, 820, 320}` (`:1266`), children non-collapsible (`:1259`).
  - **Rust**: Left panel fixed `exact_width(UX_LEFT_PANEL_WIDTH)` (`gui_layer_panel_tree.rs:17-18`, non-resizable); right panel `SidePanel::right("ka_drawing_right_panel")` `.resizable(false).exact_width(268.0)` (`gui_tab_drawing_right.rs:11-13`); central panel fills remaining space (`gui_tab_drawing.rs:41-43`).

### D1 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (caption) | `레이어` | none | static | always | Left layer card caption | `KaDrawingStudio.cpp:675` | `레이어` | none | static | always | Left layer card header label | `gui_layer_panel_tree.rs:77` | yes |
| 2 | toolbutton (toggle) | `파일함` | none | checked=`true` | always | Toggles visibility of `filesScroll` (`KaFileBrowserPanel`) | `KaDrawingStudio.cpp:682-686, :804` | `파일함` | none | `files_open` toggle | always | Toggles `self.home_map.files_open` | `gui_layer_panel_tree.rs:79-86` | yes |
| 3 | toolbutton | `전체 켜기` / `전체 끄기` | `레이어 체크를 한 번에 모두 끄거나 켭니다. 하나라도 켜져 있으면 전부 끕니다.` | dynamic based on `anyOn` | `!layers.isEmpty()` (`:2610`) | Calls `toggleAllLayersChecked` | `KaDrawingStudio.cpp:688-693, :2605-2619` | `toggle_all_state()` (`전체 켜기` / `전체 끄기`) | none | dynamic | `has_layers` | Calls `self.toggle_all_layers()` | `gui_layer_panel_tree.rs:87-96` | no (tooltip) |
| 4 | toolbutton (menu) | `세트` (`세트 ▾` in plate) | `켜 둔 레이어 조합을 이름 붙여 저장하고, 고를 때만 다시 적용합니다 (예: 유구 배치, 지형·지적 중첩).` | menu populated on `aboutToShow` | always | Opens `KaDrawingLayerSets` menu (`지금 켜진 레이어를 세트로 저장…`, `적용: <name>`, `세트 지우기`) | `KaDrawingStudio.cpp:696-709`; `KaDrawingStudioPlate.cpp:26` | `세트 ▼` | `레이어 가시성 프리셋 전환 (시굴격자 작업 · 유구 정밀 실측 · 지형·등고 분석 · 지적·주변유산 대조)` | no menu; cycles 4 presets | always | Sets `draw_guide_message` only (no menu, no real layer set save/apply) | `gui_layer_panel_tree.rs:101-120` | no (tooltip, what it does) |
| 5 | tree view | Layer tree (`KaLayerInformationView`) | none | Project layers | visible when `!m_project->mapLayers().isEmpty()` | Checkboxes, drag-reorder, double-click style, right-click context menu | `KaDrawingStudio.cpp:716-767` | Layer tree (`paint_layer_tree`) | none | `left_layers.entries` | visible when `!entries.is_empty()` | Checkboxes, selection, right-click context menu | `gui_layer_panel_tree.rs:146-160` | yes (partial drag-reorder) |
| 6 | label (empty state) | `이 도면에는 레이어가 없습니다.\n필요한 레이어를 켜고\n용지 위에 올려 보세요.` | none | centered | visible when `m_project->mapLayers().isEmpty()` | Empty state message | `KaDrawingStudio.cpp:772-789` | `layer_panel_empty_hint()` | none | centered | visible when `entries.is_empty()` | Empty state message | `gui_layer_panel_tree.rs:40-57` | yes |
| 7 | collapsible panel | `표시 설정` (`KaLayerInformationPanel`: `내용` combo, `면적(㎡) 함께 표시` check, `크기` spin `1..72 pt`, `글자 색` combo, `흰 테두리 (위성·지적 위에서 읽기)` check) | none | collapsed (`RightArrow`) | per selected layer | Edits label field, area, size, color, halo of selected layer | `KaDrawingStudio.cpp:770`; `KaLayerInformationPanel.cpp:43-92` | `paint_layer_display_settings` | none | collapsed | always | Renders `표시 설정` section | `gui_layer_panel_tree.rs:43, :60`; `gui_layer_panel_card.rs:16` | yes |
| 8 | scroll panel | `KaFileBrowserPanel` (`파일함`) | none | visible (`filesToggle` checked) | visible when `filesToggle` checked | Double-click / activate file adds raster/vector to project | `KaDrawingStudio.cpp:795-816` | `paint_file_drawer` | none | `files_open` | visible when `files_open` | File drawer panel | `gui_layer_panel_tree.rs:70`; `gui_layer_panel_card.rs:93` | yes |
| 9 | banner + 2 buttons | `지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」.` / `레이어를 가운데` / `조정끝` | none | hidden (`m_adjustBar->hide()`) | visible when `m_adjustingMap` | `레이어를 가운데` calls `centerSurveyInMap`; `조정끝` calls `endActivateMap` | `KaDrawingStudio.cpp:828-843` | `지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」.` / `조정끝` | `지도 조정을 끝냅니다.` | hidden | visible when `map_adjust_active` | `조정끝` sets `map_adjust_active = false`; **`레이어를 가운데` button is MISSING** | `gui_tab_drawing_popups.rs:19-59` | no (`레이어를 가운데` button MISSING) |
| 10 | bottom toolbutton 1 | `좌표점` | `용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다` | unchecked | always | Calls `beginPlaceCoordPoint()` | `KaDrawingStudio.cpp:1298-1300` | `좌표점` | `용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다` | `coord_callout_mode = false` | always | Toggles `coord_callout_mode` | `gui_tab_drawing_toolbar.rs:23, :83-91` | yes |
| 11 | bottom toolbutton 2 | `레이어를 가운데` | `축척은 두고, 고른 레이어를 용지 한가운데로 옮깁니다` | action | always | Calls `centerSurveyInMap()` | `KaDrawingStudio.cpp:1301-1303` | `레이어를 가운데` | `축척은 두고, 고른 레이어를 용지 한가운데로 옮깁니다` | action | always | Calls `self.center_layer_in_sheet()` | `gui_tab_drawing_toolbar.rs:24, :92-94` | yes |
| 12 | bottom toolbutton 3 | `지도 조정` (toggles to `조정 끝` when active) | `지도 칸 안의 지도를 끌어 옮기고 휠로 확대합니다. 다시 누르면 끝냅니다` | unchecked | always | Calls `toggleMapAdjust()` | `KaDrawingStudio.cpp:1305-1309, :3618` | `지도 조정` (does not change text to `조정 끝`) | `지도 칸 안의 지도를 끌어 옮기고 휠로 확대합니다. 다시 누르면 끝냅니다` | `map_adjust_active = false` | always | Toggles `map_adjust_active` | `gui_tab_drawing_toolbar.rs:25, :95-103` | no (text when active) |
| 13 | bottom toolbutton 4 | `화면 맞춤` | `지도 화면과 같은 범위·축척으로 지도 칸을 다시 맞춥니다` | action | always | Calls `fitToMapCanvas()` | `KaDrawingStudio.cpp:1310-1312` | `화면 맞춤` | `지도 화면과 같은 범위·축척으로 지도 칸을 다시 맞춥니다` | action | always | Resets `sheet_center`, replaces chrome, recovers orphaned items | `gui_tab_drawing_toolbar.rs:26, :104-110` | yes |
| 14 | bottom toolbutton 5 | `항목 이동` (`twoLine` → `항목 이동` in plate) | `좌표 상자를 끌어 옮깁니다` | action | always | Calls `useSelectTool()` | `KaDrawingStudio.cpp:1313-1316`; `KaDrawingStudioPlate.cpp:115` | `항목 이동` | `좌표 상자를 끌어 옮깁니다` | action | always | Shows guide message only (no select tool switch) | `gui_tab_drawing_toolbar.rs:27, :111-113` | no (what it does) |
| 15 | bottom toolbutton 6 | `용지/방향` | `용지 크기(A4/A3) 및 방향(가로/세로)을 전환합니다` | action | always | Calls `openPaperSettingsDialog()` | `KaDrawingStudio.cpp:1317-1319` | `용지/방향` | `용지 크기(A4/A3) 및 방향(가로/세로)을 전환합니다` | action | always | Opens `ActiveModalDialog::PaperSetup` | `gui_tab_drawing_toolbar.rs:28, :114-118` | yes |
| 16 | bottom toolbutton 7 | `도면 목록` | `지금 도면을 이름 붙여 보관하고, 보관한 도면을 다시 엽니다. 제출 PDF는 지금 열린 도면입니다` | popup menu (`지금 도면 보관…`, `보관한 도면이 없습니다` / `열기: <title>`, `보관한 도면 지우기`, `제출 PDF는 지금 열린 도면으로 만듭니다`) | always | Opens `KaDrawingSheetSet` menu | `KaDrawingStudio.cpp:1321-1339`; `KaDrawingSheetSet.cpp:135-161` | `도면 목록` | `지금 도면을 이름 붙여 보관하고, 보관한 도면을 다시 엽니다. 제출 PDF는 지금 열린 도면입니다` | `sheet_set_menu_open = false` | always | Toggles popup (`도면 목록`, `보관한 도면이 없습니다`, `지금 도면 보관…` — missing `열기:` prefix, `보관한 도면 지우기` submenu, and info footer) | `gui_tab_drawing_toolbar.rs:29, :119-122`; `gui_tab_drawing_popups.rs:123-226` | no (popup entries & delete missing) |
| 17 | group caption | `도면 항목` | none | static | always | Right panel card 1 caption | `KaDrawingStudio.cpp:902` | `도면 항목` | none | static | always | Right panel section 1 caption | `gui_tab_drawing_right.rs:26` | yes |
| 18 | tile button | `범례` | `범례를 용지에 넣습니다. 넣은 뒤 끌어 옮깁니다` | checkable in plate (`findItemById(kIdLegend)`) | always | Calls `beginPlaceLegend()` | `KaDrawingStudio.cpp:908-918, :1588` | `≡  범례` | none | `legend_visible = true` | always | Toggles `legend_visible` | `gui_tab_drawing_right.rs:31-47` | no (tooltip) |
| 19 | tile button | `표제란` | `도면명·조사명·유적명·축척·작성일 상자를 넣습니다. 조사구역의 조사명·유적명을 씁니다. 필요할 때만 넣습니다` | checkable in plate (`itemById(KaTitleBlock::itemId())`) | always | Calls `placeTitleBlock()` (prompts `QInputDialog` for title) | `KaDrawingStudio.cpp:939-946, :1589` | `⊞  표제란` | none | `title_block_visible = true` | always | Toggles `title_block_visible` (no title prompt) | `gui_tab_drawing_right.rs:48-65` | no (tooltip, default `true` vs optional, what it does) |
| 20 | button | `PDF 내보내기` | `지금 용지를 PDF 파일로 저장합니다` | primary button | always | Calls `savePdf()` | `KaDrawingStudio.cpp:929-932` | `PDF 내보내기` | `아직 Rust 판에서 연결하지 않았습니다 (지시 R3)` | disabled | `false` (disabled) | Disabled until R3 | `gui_tab_drawing_right_b.rs:133-144` | no (enabled, tooltip, what it does) |
| 21 | button | `인쇄` | `프린터로 찍거나, 큰 도면을 작은 용지 여러 장으로 나눠 찍습니다 (Ctrl+P)` | secondary button | always | Calls `printDrawing()` | `KaDrawingStudio.cpp:933-937` | `인쇄` | none | enabled | always | Opens `ActiveModalDialog::DrawingPrint` directly (skips PDF prep) | `gui_tab_drawing_right_b.rs:145-156` | no (tooltip, what it does) |
| 22 | label + line edit | `범례 제목` / `범례` | placeholder: `제목을 입력하세요` | `"범례"` | always | Updates legend title via `applyLegendSettings()` | `KaDrawingStudio.cpp:952-957` | `범례 제목` / `TextEdit::singleline` | none (no placeholder) | `"범례"` | always | Edits `legend_title` directly | `gui_tab_drawing_right.rs:68-72` | no (placeholder) |
| 23 | label + spin box | `글자` / `10 pt` | none | `10`, range `5..24`, step `1`, suffix `" pt"` | always | Updates legend font size | `KaDrawingStudio.cpp:958-964, :970-971` | `글자` / `DragValue` | none | `10`, range `5..=24`, speed `0.2`, suffix `" pt"` | always | Edits `legend_font_pt` | `gui_tab_drawing_right.rs:75-82` | yes |
| 24 | check box | `B` | none | `true` | always | Toggles legend bold | `KaDrawingStudio.cpp:965-969, :972` | `B` | none | `true` | always | Toggles `legend_bold` | `gui_tab_drawing_right.rs:83` | yes |
| 25 | check box | `I` | none | `false` | always | Toggles legend italic | `KaDrawingStudio.cpp:967-969, :973` | `I` | none | `false` | always | Toggles `legend_italic` | `gui_tab_drawing_right.rs:84` | yes |
| 26 | check box | `1:5,000 이하에서 유적 이름으로 표시` | `켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다` | `false` (`namesPropertyKey`) | always | Sets layout property & calls `syncHeritageNumbers(true)` | `KaDrawingStudio.cpp:975-989` | `1:5,000 이하에서 유적 이름으로 표시` | `켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다` | `false` | always | Toggles `show_site_names_under_5000` | `gui_tab_drawing_right.rs:86-92` | yes |
| 27 | group caption | `방위 (도북)` | `방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다.` | static | always | Right panel card 2 caption | `KaDrawingStudio.cpp:998-1001`; `KaDrawingNorthArrow.cpp:205-207` | `방위 (도북)` | none | static | always | Section 2 caption | `gui_tab_drawing_right.rs:95-99` | no (tooltip) |
| 28 | 4 tile buttons | `북 글자`, `북 화살`, `나침반`, `바람장미` | `<tip> — 방위표는 좌표계의 북쪽(도북)을 가리킵니다. 진북·자북과는 조금 다를 수 있습니다.` | selected style highlighted | always | Calls `beginPlaceNorth(rel)` | `KaDrawingStudio.cpp:1004-1020` | `NorthArrowStyle::ALL` (`북 글자`, `북 화살`, `나침반`, `바람장미`) | none | `SingleArrow` (`북 화살`) | always | Sets `north_arrow_style` | `gui_tab_drawing_right.rs:100-132` | no (tooltip) |
| 29 | label + double spin box | `크기` / `28 mm` | none | `28.0`, range `3.0..80.0`, decimals `0`, suffix `" mm"` | always | Resizes north arrow item on sheet | `KaDrawingStudio.cpp:1022-1041` | `크기` / `DragValue` | none | `28.0`, range `3.0..=80.0`, decimals `0`, suffix `" mm"` | always | Edits `north_arrow_size_mm` | `gui_tab_drawing_right.rs:134-144` | yes |
| 30 | group caption | `도면 정보` | none | static | always | Right panel card 3 caption | `KaDrawingStudio.cpp:1058` | `도면 정보` | none | static | always | Section 3 caption | `gui_tab_drawing_right.rs:147-151` | yes |
| 31 | label + spin box + button | `1 :` / `1000` / `적용` | none | `1000`, range `10..5000000`, step `10` | always | `applyOnScreenScale()` applies scale to map & decorations | `KaDrawingStudio.cpp:1061-1080` | `1 :` / `DragValue` / `적용` | none | `5000` (`sheet_drawing.rs:72`), range `10..=5_000_000`, speed `50.0` | always | `DragValue` edits `scale_denominator`; `적용` only sets `draw_guide_message` | `gui_tab_drawing_right.rs:152-166` | no (default `1000` vs `5000`) |
| 32 | 14 preset buttons | `20`, `30`, `50`, `100`, `200`, `250`, `300`, `400`, `500`, `1000`, `2000`, `5000`, `10000`, `25000` | none | checkable (`denom == current`) | always | Sets `m_scaleSpin` and calls `applyOnScreenScale()` | `KaDrawingStudio.cpp:1084-1123`; `StandardScales.cpp:26-50` | `20`..`25000` (2 rows × 7) | none | non-checkable buttons | always | Sets `scale_denominator = v` | `gui_tab_drawing_right_b.rs:15-47` | no (not checkable/highlighted) |
| 33 | 3 tile buttons | `쌍칸`, `외칸`, `눈금` | none | icon + label | always | Calls `beginPlaceScaleBar(style)` | `KaDrawingStudio.cpp:1126-1138` | `ScaleBarStyle::ALL` (`쌍칸`, `외칸`, `눈금`) | none | `DoubleBox` (`쌍칸`) | always | Sets `scale_bar_style` | `gui_tab_drawing_right_b.rs:49-81` | yes |
| 34 | label + double spin box | `선택 항목 크기` / `100 %` | `용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다.` | `100`, range `25..200`, step `10`, decimals `0`, suffix `" %"` | `editable` (only when a decoration item is selected, `:1151, :3077`) | Calls `resizeSelectedDecoration()` | `KaDrawingStudio.cpp:1141-1157, :3077` | `선택 항목 크기` / `DragValue` | `용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다.` | `100`, range `25..=200`, speed `1.0`, suffix `" %"` | always enabled | Edits `decoration_size_percent` | `gui_tab_drawing_right_b.rs:83-95` | no (enabled-when rule) |
| 35 | toolbutton | `축척 글자` | none | action | always | Calls `beginPlaceScaleLabel()` | `KaDrawingStudio.cpp:1160-1163` | `축척 글자` | none | button | always | No-op on click (`&& i == 2` only handles index 2) | `gui_tab_drawing_right_b.rs:98-107` | no (what it does) |
| 36 | toolbutton | `좌표계` | none | action | always | Calls `beginPlaceCrsLabel()` | `KaDrawingStudio.cpp:1164-1167` | `좌표계` | none | button | always | No-op on click (`&& i == 2` only handles index 2) | `gui_tab_drawing_right_b.rs:98-107` | no (what it does) |
| 37 | toolbutton (toggle) | `좌표 격자` | `지도 안 +와 테두리 좌표 자를 켭니다. 기본은 꺼져 있습니다.` (or `좌표 격자 켜짐 (%1). 눌러서 끄거나 간격을 바꿉니다.`) | `m_gridEnabled` (`false` by default, `:1553`) | always | Opens `focusGridSettings()` (C12) | `KaDrawingStudio.cpp:1168-1177, :1560-1570` | `좌표 격자` | none | `grid_enabled = true` (`sheet_drawing.rs:73`) | always | Toggles `grid_follow_map` directly instead of opening C12 | `gui_tab_drawing_right_b.rs:98-124` | no (tooltip, default, what it does) |
| 38 | floating rail | `KaLayerOpacityRail` (opacity/brightness) | per rail | hidden unless layer supports opacity/brightness | layer selected | Adjusts layer opacity / brightness | `KaDrawingStudio.cpp:1196-1231` | `투명도·밝기: 조절할 레이어를 고르세요` (static pill) | none | always visible | static | Painted pill only (no slider/rail) | `gui_tab_drawing_sheet.rs:19-36` | no (EXTRA static pill / MISSING rail) |
| 39 | check box | (none in C++ right panel — inside C12) | — | — | — | — | — | `지도 좌표격자와 같게 (20 m)` | none | `false` | always | Toggles `grid_follow_map` | `gui_tab_drawing_right_b.rs:127-130` | **EXTRA** |

### D1 Keyboard & Closing Behaviour
- **Default button**: None (embedded tab).
- **Enter / Esc**: C++ `QMainWindow` tab; `Esc` does not close the studio. In Rust, `Esc` closes modals/sub-toolbars (`gui_shell.rs:34-39`).
- **Delete / Backspace**: C++ `KaDrawingStudio::keyPressEvent` (`KaDrawingStudio.cpp:3561-3565`) calls `handleDeleteKey()` to delete selected layout item; `Ctrl+Z` / `Ctrl+Y` call `handleUndoKey()` / `handleRedoKey()` (`:3549-3560`). Rust `gui_shell.rs:84-88` only calls `home_map.cad.on_backspace` and does not handle Drawing Studio item deletion.
- **Ctrl+P**: C++ triggers main window print action (`KaDrawingStudio.cpp:936`); Rust opens Drawing tab + `DrawingPrint` (`gui_shell.rs:141-146`).
- **Closing by X**: Tab cannot be closed independently in C++ `m_viewTabs`; in Rust, studio tabs can be closed via tab X (`app_state_tests.rs:79`).

---

## D2. Drawing Studio Message & Input Boxes

All message boxes and input prompts below are in `KaDrawingStudio.cpp` and `KaDrawingStudioSheets.cpp`. In Rust, **none** of these modal message/input boxes exist (`MISSING`); Rust only sets `self.home_map.draw_guide_message` (status bar text) for some actions (`gui_tab_drawing_popups.rs:209-223`) or does not implement the operation.

| # | kind | C++ title (verbatim) | C++ text / prompt (verbatim) | C++ buttons & default | C++ condition | C++ file:line | Rust counterpart | same? |
|---|---|---|---|---|---|---|---|---|
| 1 | `QMessageBox::question` | `레이어 삭제` | `선택한 레이어 %1개를 도면/프로젝트에서 삭제할까요?` | `Yes \| No` (default `Yes`) | `removeSelectedLayers()` when `ids` non-empty | `KaDrawingStudio.cpp:3294-3298` | Direct removal without confirmation (`gui_layer_panel_context_menu.rs:102-105`) | **MISSING** |
| 2 | `QMessageBox::information` | `PDF` | `용지가 없습니다.` | `OK` | `savePdf()` when `!layout()` | `KaDrawingStudio.cpp:3821` | None (`PDF 내보내기` disabled, `gui_tab_drawing_right_b.rs:144`) | **MISSING** |
| 3 | `KaUserError::warn` | `PDF` | `PDF를 저장하지 못했습니다` + `pdfError` | `OK` | `savePdf()` when `!exportDrawingPdf(...)` | `KaDrawingStudio.cpp:3841` | None | **MISSING** |
| 4 | `QMessageBox::information` | `PDF` | `저장했습니다.\n%1` (`path`) | `OK` | `savePdf()` after export succeeds | `KaDrawingStudio.cpp:3854` | None | **MISSING** |
| 5 | `QMessageBox::information` | `인쇄` | `용지가 없습니다.` | `OK` | `printDrawing()` when `!layout()` | `KaDrawingStudio.cpp:3860` | None | **MISSING** |
| 6 | `KaUserError::warn` | `인쇄` | `인쇄를 시작하지 못했습니다` / `인쇄용 임시 폴더를 만들지 못했습니다.` | `OK` | `printDrawing()` when `!temporary.isValid()` | `KaDrawingStudio.cpp:3866` | None | **MISSING** |
| 7 | `KaUserError::warn` | `인쇄` | `인쇄하지 못했습니다` + `error` | `OK` | `printDrawing()` when `!exportDrawingPdf(...)` | `KaDrawingStudio.cpp:3887` | None | **MISSING** |
| 8 | `QInputDialog::getText` | `표제란` | `도면명 (예: 조사구역도, 유적위치도, 유구배치도)` (default `title` or `"조사도면"`) | `OK \| Cancel` | `placeTitleBlock()` | `KaDrawingStudioSheets.cpp:47-50` | None (toggles `title_block_visible` directly, `gui_tab_drawing_right.rs:62-65`) | **MISSING** |
| 9 | `QInputDialog::getText` | `도면 보관` | `도면 이름 (예: 조사구역도 1:1000, 1호 주거지 실측도)` (default `suggestion`) | `OK \| Cancel` | `storeCurrentSheet()` | `KaDrawingStudioSheets.cpp:82-85` | None (stores `self.drawing.title_ko` directly, `gui_tab_drawing_popups.rs:200-214`) | **MISSING** |
| 10 | `QMessageBox::question` | `도면 보관` | `「%1」 도면이 이미 있습니다. 지금 도면으로 바꿀까요?` | `Yes \| No` (default `No`) | `storeCurrentSheet()` when `KaDrawingSheetSet::contains(title)` | `KaDrawingStudioSheets.cpp:88-90` | None (pushes duplicate) | **MISSING** |
| 11 | `KaUserError::warn` | `도면 보관` | `도면을 보관하지 못했습니다` + `error` | `OK` | `storeCurrentSheet()` on store failure | `KaDrawingStudioSheets.cpp:99` | None | **MISSING** |
| 12 | `QMessageBox::question` | `도면 열기` | `지금 용지를 보관한 도면 「%1」로 바꿉니다. 지금 도면을 보관하지 않았다면 사라집니다. 레이어 켜짐도 그 도면의 세트로 바뀝니다. 바꿀까요?` | `Yes \| No` (default `No`) | `restoreStoredSheet(title)` | `KaDrawingStudioSheets.cpp:109-114` | None (restores immediately with status bar text, `gui_tab_drawing_popups.rs:215-225`) | **MISSING** |
| 13 | `KaUserError::warn` | `도면 열기` | `도면을 열지 못했습니다` + `error` | `OK` | `restoreStoredSheet(title)` on restore failure | `KaDrawingStudioSheets.cpp:125` | None | **MISSING** |
| 14 | `QMessageBox::question` | `도면 목록` | `보관한 도면 「%1」과 같은 이름의 레이어 세트를 지울까요? 지금 열린 도면과 레이어는 그대로입니다.` | `Yes \| No` (default `No`) | `removeStoredSheet(title)` | `KaDrawingStudioSheets.cpp:160-164` | None (no delete action in Rust) | **MISSING** |

---

## D3. 「도면 인쇄」 (`KaPrintDialog`)

- **C++ title**: `"도면 인쇄"` (`KaPrintDialog.cpp:74`).
- **How opened**:
  - **C++**: `KaDrawingStudio::printDrawing()` (`KaDrawingStudio.cpp:3894-3895`) after `KaDrawingPdfProgress` finishes exporting temp PDF.
  - **Rust**: Clicking `"인쇄"` in Drawing Studio right panel (`gui_tab_drawing_right_b.rs:151-156`), clicking `"인쇄"` in ribbon (`gui_ribbon_click_b.rs:133-141`), or `Ctrl+P` (`gui_shell.rs:141-146`).
- **Modality**:
  - **C++**: Modal `QDialog::exec()` (`KaDrawingStudio.cpp:3895`).
  - **Rust**: `egui::Window::new("ka_modal_drawing_print")` via `ActiveModalDialog::DrawingPrint` (`dlg_drawing_print.rs:29`, `dialogs_modal.rs:95`).
- **Size rules**:
  - **C++**: In plate mode (`KaWindowSize::width(Large)`): `resize(large, 640)` (`860 × 640`, `KaPrintDialog.cpp:241`, `KaPrintDialogPlate.cpp:1`); left settings column `setFixedWidth(std::max(380, 96 + widest))` (`KaPrintDialogPlate.cpp:119`).
  - **Rust**: `.resizable(false).anchor(CENTER_CENTER, [0,0]).default_width(760.0)` (`dlg_drawing_print.rs:31-33`).

### D3 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label + combo | `프린터` / `<system printers>` or `(설치된 프린터 없음)` | none | default system printer | `!printers.isEmpty()` (`:94`) | Selects target printer & updates margins/A3 support | `KaPrintDialog.cpp:88-98, :108` | `프린터` / `ezViewX Virtual Image Printer              ▼` | none | hardcoded button string | always enabled | Non-functional button | `dlg_drawing_print_options.rs:20-29` | no (text, kind, what it does) |
| 2 | button | `프린터 속성…` | `프린터 속성에서 단면 인쇄를 고르세요. 양면으로 나오면 기본 설정에서 단면을 고릅니다.` (dynamic in `rebuild()`: + ` 속성 창에서 「인쇄」를 누르면 바로 찍히고, 「취소」를 누르면 찍지 않습니다.`) | — | `canDraw && !printerName().isEmpty()` (`:451`) | Calls `printNow()` (opens `QPrintDialog`) | `KaPrintDialog.cpp:103-107, :237, :451, :457-459` | `프린터 속성...` | none | — | always enabled | Non-functional button (`let _ = ui.add(...)`) | `dlg_drawing_print_options.rs:32-38` | no (text `…` vs `...`, tooltip, what it does) |
| 3 | radio 1 | `방법` / `여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)` | none | `probe.actualSize ? fit : tiles` (`:223`) | always | Sets tiled mode (`rebuild()`) | `KaPrintDialog.cpp:110, :120, :223` | `방법` / `여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)` | none | `print_single_sheet = true` (`modal_controller_state.rs:176`) | always | Sets `print_single_sheet = false` | `dlg_drawing_print_options.rs:42-48` | no (default always `true` in Rust vs `probe.actualSize` in C++) |
| 4 | radio 2 | `한 장에 인쇄 (용지에 들어가면 실제 크기로)` | none | `probe.actualSize` | always | Sets single-sheet fit mode (`rebuild()`) | `KaPrintDialog.cpp:111, :223` | `한 장에 인쇄 (용지에 들어가면 실제 크기로)` | none | `print_single_sheet = true` | always | Sets `print_single_sheet = true` | `dlg_drawing_print_options.rs:49-53` | yes |
| 5 | label + combo | `붙였을 때 크기` / `도면 크기 그대로 …`, `<scale>으로 키우기 …`, `A0..A3에 맞춤 …` | none | `도면 크기 그대로` | visible only when `tiled` (`:414`) | Selects output scale factor | `KaPrintDialog.cpp:122-125, :255-285, :414` | — | — | — | — | — | — | **MISSING** |
| 6 | label + joined buttons (plate) | `용지` / `A4` \| `A3` | none | `A3` if printer supports A3 else `A4` (`:131`, saved in `QSettings`) | always | Sets sheet size (`SheetA4` / `SheetA3`) | `KaPrintDialog.cpp:127-132, :147`; `KaPrintDialogPlate.cpp:89-109` | `용지` / `A4` \| `A3` | none | `A4` or `A3` via `default_print_paper_for_printer()` | always | Sets `print_paper` | `dlg_drawing_print_options.rs:58-86` | yes |
| 7 | label + double spin box | `겹침` / `10 mm` | `이웃 장과 같은 그림이 겹치는 폭입니다. 붙일 때 맞추기 쉽습니다.` | `10.0`, range `0..30`, decimals `0`, suffix `" mm"` | visible only when `tiled` (`:417-418`) | Sets overlap width (`rebuild()`) | `KaPrintDialog.cpp:133-139, :143-145, :417-418` | `겹침` / `DragValue` | `이웃 장과 같은 그림이 겹치는 폭입니다. 붙일 때 맞추기 쉽습니다.` | `10.0`, range `0.0..=30.0`, speed `1.0`, decimals `0`, suffix `" mm"` | visible only when `!print_single_sheet` | Edits `print_overlap_mm` | `dlg_drawing_print_options.rs:88-101` | yes |
| 8 | label + combo | `해상도` / `300 DPI (보통)`, `600 DPI (가는 선이 많은 실측도)` | `도면 PDF를 이 해상도로 그려 찍습니다. 0.1~0.2 mm 선이 많은 1:20 실측도는 600을 고르세요. 프린터 해상도보다 곱게 그리지는 않습니다.` | `300` (`kDefaultPrintDpi`) | always | Selects print DPI (`300` or `600`) | `KaPrintDialog.cpp:150-161`; `TilePrint.cpp:352-354` | `해상도` / `300 DPI (보통)` or `600 DPI (고정밀 제출용)` | none | `300` | always | Button toggles `300` ↔ `600` | `dlg_drawing_print_options.rs:105-127` | no (600 DPI text, tooltip) |
| 9 | check box | `자르는 점선·장 번호·5 cm 확인선 넣기` | none | `true` | visible only when `tiled` (`:415`) | Toggles cut lines & 5 cm check bar | `KaPrintDialog.cpp:163-165, :415` | `자르는 점선·장 번호·5 cm 확인선 넣기` | none | `true` | visible only when `!print_single_sheet` | Toggles `print_marks` | `dlg_drawing_print_options.rs:133-136` | yes |
| 10 | check box | `맨 앞에 붙이는 순서 안내 한 장 넣기` | none | `true` | visible only when `tiled` (`:415`) | Toggles overview sheet | `KaPrintDialog.cpp:166-168, :415` | `맨 앞에 붙이는 순서 안내 한 장 넣기` | none | `true` | visible only when `!print_single_sheet` | Toggles `print_overview` | `dlg_drawing_print_options.rs:137-140` | yes |
| 11 | ledger (4 rows, plate) | `용지` / `배열` / `붙인 크기` (or `인쇄 크기`) / `축척` | none | dynamic (`m_plan` / `m_fit`) | `배열` visible when `planned`; `축척` visible when `!scale.isEmpty()` | Displays summary ledger | `KaPrintDialogPlate.cpp:171-223` | `용지` / `인쇄 크기` (`실제 크기 100%` hardcoded) / `축척` | none | static `인쇄 크기` | always 3 rows | **`배열` row MISSING**, `인쇄 크기` hardcoded | `dlg_drawing_print_options.rs:171-191` | no (`배열` MISSING, `인쇄 크기` hardcoded) |
| 12 | label | `m_summary` (bold fallback summary) | none | dynamic (`summary()`) | visible only when `m_drawingMm.isEmpty() \|\| (tiled && !m_plan.ok)` (`:229`) | Shows error or plan summary | `KaPrintDialog.cpp:177-183, :464-491`; `KaPrintDialogPlate.cpp:229` | — | — | — | — | — | — | **MISSING** |
| 13 | label (warning) | `m_warning` (edge clip / scale mismatch warning) | none | dynamic (`warning()`) | visible when `!warn.isEmpty()` | Warns on edge clipping or scale change | `KaPrintDialog.cpp:184-188, :493-519` | — | — | — | — | — | — | **MISSING** |
| 14 | preview + header | `미리보기` / `도면 · <sheet> <turn> · <pages> · <scale>` + `KaTilePreview` | none | renders PDF thumb + tile grid | always | Interactive preview; clicking sheet calls `toggleSheet(index)` | `KaPrintDialog.cpp:189-191`; `KaPrintDialogPlate.cpp:130-139, :219-223` | `미리보기` / `도면 · {} · {}` + static dummy painter | none | static green/pink dots + `"축척 1 : 6532"` hardcoded | always | Non-interactive static illustration | `dlg_drawing_print_preview.rs:17-124` | no (hardcoded dummy preview, not interactive) |
| 15 | label | `미리보기에서 장을 누르면 이번 인쇄에서 빼거나 다시 넣습니다.\n잘못 나온 장만 다시 찍을 때 쓰세요.` | none | static | visible when `several` (`tiled && m_plan.ok && sheets.size() > 1`) | Explains sheet toggling | `KaPrintDialog.cpp:193-196, :444-445`; `KaPrintDialogPlate.cpp:141-142` | — | — | — | — | — | — | **MISSING** |
| 16 | label + button | `m_skipNote` / `모두 다시 넣기` | none | hidden | visible when `several && !m_skipped.isEmpty()` | Clears `m_skipped` | `KaPrintDialog.cpp:197-200, :232-235, :446`; `KaPrintDialogPlate.cpp:155-160, :224-227` | — | — | — | — | — | — | **MISSING** |
| 17 | button | `나눈 장을 PDF로 저장…` | `프린터가 없는 PC나 출력소에서 찍을 수 있게 한 장이 한 쪽인 PDF로 저장합니다.` | — | `tiled && canDraw` (`:460`) | Calls `saveTilesAs()` | `KaPrintDialog.cpp:204-206, :460` | `나눈 장을 PDF로 저장…` | `아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)` | disabled | `false` (disabled) | Disabled until W-50 | `dlg_drawing_print_actions.rs:37-45` | no (enabled, tooltip, what it does) |
| 18 | button (default) | `인쇄 · 1장` / `인쇄 · %1장` / `인쇄 · %1장 + 안내 1장` | `프린터 속성이 열립니다. 양면으로 나오면 기본 설정에서 단면을 고르세요.` (or no-printer tip) | `setDefault(true)` | `canDraw && !printerName().isEmpty()` (`:450`) | Calls `printNow()` | `KaPrintDialog.cpp:207-209, :448-455, :521-526` | `인쇄 · {page_cnt}장` | none | always enabled | always enabled | Sets `draw_guide_message` & closes modal (no real print) | `dlg_drawing_print_actions.rs:57-83` | no (text missing `+ 안내 1장`, tooltip, what it does) |
| 19 | button | `닫기` | none | — | always | Calls `QDialog::reject()` | `KaPrintDialog.cpp:210, :239` | `닫기` | none | — | always | Sets `app.modals.active = None` | `dlg_drawing_print_actions.rs:49-56` | yes |

### D3 Keyboard & Closing Behaviour
- **Default button**: C++ `m_print->setDefault(true)` (`KaPrintDialog.cpp:209`), `A4`/`A3` buttons have `setAutoDefault(false)` so `Enter` stays on `인쇄` (`KaPrintDialogPlate.cpp:97`). Rust has no `Enter` key handler on `DrawingPrint`.
- **Esc**: C++ `QDialog` `Esc` triggers `reject()`. Rust `gui_shell.rs:34-36` `Esc` sets `self.modals.active = None`.
- **Tab order**: C++ `KaPrintDialogPlate.cpp:103-105` sets `m_output -> A4 -> A3 -> m_overlap`. Rust uses default egui focus order.
- **Button box order**: Left `나눈 장을 PDF로 저장…`, stretch, right `인쇄 · …`, `닫기` (`KaPrintDialog.cpp:211-214`). Rust matches (`dlg_drawing_print_actions.rs:36-86`).
- **Window X**: C++ `reject()` (`QDialog`). Rust `render_strata_modal_header` sets `*open = false` → `app.modals.active = None` (`dialogs_modal.rs:59-61, :97-99`).

### D3 Sub-dialogs & Messages (`KaPrintDialog.cpp:578, :614, :637, :654, :667`)

| # | kind | C++ title (verbatim) | C++ text (verbatim) | C++ condition | C++ file:line | Rust counterpart | same? |
|---|---|---|---|---|---|---|---|
| 1 | `QProgressDialog` | `도면 인쇄` (`windowTitle()`) | `<verb>…` / `%1… %2 / %3쪽`, cancel button `멈추기` | `renderWithProgress()` during tiled print or tiled PDF save | `KaPrintDialog.cpp:575-590` | None | **MISSING** |
| 2 | `QMessageBox::information` | `나눈 장 PDF` | `%1쪽을 저장했습니다.\n%2\n\n다른 PC나 출력소에서 찍을 때는 꼭 「실제 크기(100%)」로 찍으세요. 「용지에 맞춤」으로 찍으면 축척이 틀어집니다. 각 장 아래 5 cm 확인선을 자로 재어 확인할 수 있습니다.` | `saveTilesAs()` succeeds | `KaPrintDialog.cpp:614-619` | None | **MISSING** |
| 3 | `KaUserError::warn` | `나눈 장 PDF` | `나눈 장을 PDF로 저장하지 못했습니다` + `error` | `saveTilesAs()` fails | `KaPrintDialog.cpp:610` | None | **MISSING** |
| 4 | `QPrintDialog` | `프린터 속성` | Native OS print dialog (`DuplexNone`, `PrintToFile` etc. hidden) | `printNow()` | `KaPrintDialog.cpp:636-642` | None | **MISSING** |
| 5 | `KaUserError::warn` | `인쇄` | `이 용지로는 인쇄할 수 없습니다` / `이 프린터는 %1 용지를 지원하지 않습니다.` / `다른 용지를 고르세요.` | `printNow()` when `!printerSupports(sheetSize)` | `KaPrintDialog.cpp:626-627` | `verify_requested_device_paper` (silent if err, no dialog) | **MISSING** |
| 6 | `QMessageBox::information` | `인쇄` | `인쇄를 멈췄습니다. 이미 넘어간 쪽은 프린터에 따라 나올 수 있습니다.` | `printNow()` when `cancelled` | `KaPrintDialog.cpp:654-655` | None | **MISSING** |
| 7 | `KaUserError::warn` | `인쇄` | `인쇄하지 못했습니다` + `error` | `printNow()` when `!ok` | `KaPrintDialog.cpp:659` | None | **MISSING** |
| 8 | `QMessageBox::information` | `인쇄` | `프린터로 %1쪽을 보냈습니다.` (+ optional `\n다 나오면 첫 장(붙이는 순서)을 보며 붙이세요.` or `\n다 나오면 장 번호 순서대로 붙이세요.`) | `printNow()` succeeds | `KaPrintDialog.cpp:663-667` | `format_print_dispatched_message` in `draw_guide_message` only | **MISSING** (no modal box) |

---

## D4. PDF Export / Print Preparation Progress (`KaDrawingPdfProgress`)

- **C++ title**: Passed by caller: `"PDF 내보내기"` (`KaDrawingStudio.cpp:3832`) or `"인쇄 준비"` (`KaDrawingStudio.cpp:3877`).
- **How opened**:
  - **C++**: `KaDrawingStudio::savePdf()` (`:3832`) and `KaDrawingStudio::printDrawing()` (`:3877`).
  - **Rust**: Only via `debug_control.rs:88` (`ActiveModalDialog::PdfPreparing`); neither `PDF 내보내기` (disabled) nor `인쇄` (opens `DrawingPrint` directly) opens it.
- **Modality**:
  - **C++**: `Qt::ApplicationModal` (`KaDrawingPdfProgress.cpp:14`).
  - **Rust**: `egui::Window::new("ka_modal_pdf_prep")` (`dlg_pdf_preparing.rs:11`).
- **Size rules**:
  - **C++**: `setMinimumWidth(380)` (`KaDrawingPdfProgress.cpp:21`); `KaProgressPlate::dress` (440 px).
  - **Rust**: `.resizable(false).fixed_size([360.0_f32.min(...), 128.0_f32.min(...)])` (`dlg_pdf_preparing.rs:13-18`).

### D4 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `준비하는 중…` (initial), then `renderNote`: `PDF를 그리는 중… (용지 %1×%2 mm · %3 DPI)\n취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.` (or `큰 용지라 시간이 걸립니다. 취소하면...` for > A3), `파일을 저장하는 중…` | none | dynamic | always | Shows current export step | `KaDrawingPdfProgress.cpp:20, :36, :47-58`; `KaDrawingStudio.cpp:3813` | `PDF를 그리는 중... (용지 210x297 mm · 300 DPI)` + `취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.` | none | hardcoded A4 300 DPI string | always | Static labels (ellipsis `...` vs `…`, `x` vs `×`) | `dlg_pdf_preparing.rs:21-29` | no (text, dynamic values) |
| 2 | progress bar | (indeterminate `QProgressBar`) | none | `setRange(0, 0)` | always | Animated busy indicator | `KaDrawingPdfProgress.cpp:15` | Custom painted bar | none | static `160.0 px` filled rect | always | Static painted bar | `dlg_pdf_preparing.rs:31-45` | no (static vs indeterminate) |
| 3 | button | `취소` | none | — | always | Sets `m_cancelled = true` on click | `KaDrawingPdfProgress.cpp:19, :41` | `취소` | none | — | always | Sets `app.modals.active = None` | `dlg_pdf_preparing.rs:48-55` | no (what it does) |

### D4 Keyboard & Closing Behaviour
- **Default button / Enter / Esc**: C++ `QProgressDialog` `Esc` cancels (`wasCanceled()`). Rust `Esc` sets `app.modals.active = None` (`gui_shell.rs:36`).
- **Window X**: C++ cancels; Rust `render_strata_modal_header` sets `*open = false` → `app.modals.active = None`.
- **Messages**: C++ `KaDrawingPdfProgress::cancelledText()` → `"PDF 만들기를 취소했습니다. 파일은 만들지 않았습니다."` (`KaDrawingPdfProgress.cpp:82-84`) shown in status bar (`KaDrawingStudio.cpp:3837`). Rust: `MISSING`.

---

## C12. 「격자 설정」 (`focusGridSettings`)

- **C++ title**: `"격자 설정"` (`KaDrawingStudio.cpp:1462`).
- **How opened**:
  - **C++**: Clicking `"좌표 격자"` button (`m_gridBtn`) in Drawing Studio right panel (`KaDrawingStudio.cpp:1168-1175`).
  - **Rust**: **MISSING** — clicking `"좌표 격자"` in `gui_tab_drawing_right_b.rs:106-124` toggles `grid_follow_map` directly; no dialog exists.
- **Modality**: C++ `QDialog::exec()` (`KaDrawingStudio.cpp:1516`). Rust: **MISSING**.
- **Size rules**: C++ `setMinimumWidth(280)` (`KaDrawingStudio.cpp:1463`). Rust: **MISSING**.

### C12 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|---|
| 1 | check box | `좌표 격자` | `끄면 도면 안 +와 바깥 테두리 좌표 자가 사라집니다.` | `m_gridEnabled` (default `false`, `:1553`) | always | Calls `setGridOptions(...)` live | `KaDrawingStudio.cpp:1467-1469, :1490-1499` | — | **MISSING** |
| 2 | label + double spin box | `간격` / `0 m` (`자동(축척에 맞춤)`) | none | `m_gridIntervalM` (`0.0`), range `0.0..100000.0`, decimals `0`, step `10.0`, suffix `" m"`, specialValueText `"자동(축척에 맞춤)"` | `!follow->isChecked()` (`:1489, :1491`) | Sets grid interval live | `KaDrawingStudio.cpp:1472-1479, :1489-1508` | — | **MISSING** |
| 3 | check box | `지도 좌표격자와 같게 (%1 m)` (if `mapGrid`) or `지도 좌표격자와 같게` | `지도 화면의 좌표격자와 같은 간격·같은 선으로 도면에 격자선을 긋습니다.` (or `지도에 미터 좌표격자를 켜면 쓸 수 있습니다(회전한 격자는 안 됩니다).`) | `m_gridFollowMap && mapGrid != nullptr` | `mapGrid != nullptr` (`:1485`) | Syncs sheet grid to map grid live | `KaDrawingStudio.cpp:1481-1488, :1501` | Inline checkbox `지도 좌표격자와 같게 (20 m)` in right panel (`gui_tab_drawing_right_b.rs:127-130`) | **MISSING** (in dialog) |
| 4 | check box | `칸 밖에 좌표 숫자` | none | `m_gridShowNums` (default `false`, `:1554`) | always | Toggles outer coordinate numbers live | `KaDrawingStudio.cpp:1470-1471, :1500` | — | **MISSING** |
| 5 | button (`QDialogButtonBox::Close`) | `닫기` | none | — | always | Calls `dlg.reject()` | `KaDrawingStudio.cpp:1512-1514` | — | **MISSING** |

---

## Context Menus of the Drawing Studio

### CTX-L. Layer Tree Context Menu (`m_layerTree`)
- **C++**: `KaDrawingStudio.cpp:734-743` delegates to `MainWindow::showLayerTreeContextMenu(m_layerTree, pt)` (`MainWindowContextMenus.cpp:176-448`), or fallback single-item `"레이어 삭제"` if no `MainWindow`.
- **Rust**: `gui_layer_panel_rows.rs:201, :229` opens `render_layer_context_menu` (`gui_layer_panel_context_menu.rs:13`), which uses `layer_menu_model.rs`.
- **Note**: Both Map tab and Drawing Studio share `MainWindow::showLayerTreeContextMenu` in C++ and `render_layer_context_menu` in Rust (`layer_menu_model.rs`). In Drawing Studio specifically, C++ passes `layoutLabels = (treeView->objectName() == "layoutLayerTree")` (`MainWindowContextMenus.cpp:223`), which disables `"글자 크기"` for heritage layers with tooltip `"도면의 유적 번호는 정해진 크기로 그립니다. 글자 크기는 지도 탭에서 바꿉니다."` (`:335-337`).

### CTX-M. Map Box Context Menu (`KaLayoutMenuProvider`)
- **C++**: `KaDrawingStudio.cpp:315-355, :857` (`m_view->setMenuProvider(new KaLayoutMenuProvider(this))`). Right-clicking on the map item (`QgsLayoutItemMap`) opens a `QMenu`:
  - When `isPlacingCoordPoint()` (`KaDrawingStudio.cpp:330-336`):
    1. `마지막 점 지우기` → `undoLastCoordCallout` (`:332-333`)
    2. `점찍기 끝` → `endPlaceCoordPoint` (`:334`)
  - Otherwise (`KaDrawingStudio.cpp:337-350`):
    1. `지도 조정` → `beginActivateMap` (`:338-340`)
    2. `그린 것 가운데로` → `centerSurveyInMap` (`:341-343`)
    3. Separator + `지도조정끝` → `endActivateMap` (when `isMapAdjusting()`, `:344-349`)
- **Rust**: **MISSING** — no `secondary_clicked` or context menu exists in `gui_tab_drawing*.rs`.

---

## Differences to fix

1. **C12 「격자 설정」 dialog** — entire dialog missing (`KaDrawingStudio.cpp:1460-1518` → not found in Rust; `좌표 격자` button toggles `grid_follow_map` directly in `gui_tab_drawing_right_b.rs:106-124`).
2. **CTX-M Drawing Studio map box context menu** — entire `KaLayoutMenuProvider` menu (`마지막 점 지우기`, `점찍기 끝`, `지도 조정`, `그린 것 가운데로`, `지도조정끝`) missing (`KaDrawingStudio.cpp:315-355` → not found in `gui_tab_drawing*.rs`).
3. **D1 `PDF 내보내기` button** — disabled with `아직 Rust 판에서 연결하지 않았습니다 (지시 R3)` (`KaDrawingStudio.cpp:929-932` → `gui_tab_drawing_right_b.rs:142-144`).
4. **D3 `나눈 장을 PDF로 저장…` button** — disabled with `아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)` (`KaPrintDialog.cpp:204-206, :460` → `dlg_drawing_print_actions.rs:37-45`).
5. **D3 `프린터` combo & `프린터 속성…` button** — hardcoded non-functional `"ezViewX Virtual Image Printer              ▼"` and non-functional `"프린터 속성..."` (`KaPrintDialog.cpp:88-107, :237` → `dlg_drawing_print_options.rs:22-38`).
6. **D3 `미리보기` (`KaTilePreview`) & `모두 다시 넣기`** — static dummy illustration with hardcoded `"축척 1 : 6532"`, missing interactive sheet toggle, `m_pickHint`, `m_pickAll` (`모두 다시 넣기`), `m_warning`, `m_skipNote`, `m_summary` (`KaPrintDialog.cpp:177-200`; `KaPrintDialogPlate.cpp:130-160` → `dlg_drawing_print_preview.rs:17-124`).
7. **D3 `붙였을 때 크기` combo (`m_output`)** — entire scale/paper-fit combo box missing (`KaPrintDialog.cpp:122-125, :255-285` → not found in `dlg_drawing_print_options.rs`).
8. **D3 `인쇄` button & sub-dialogs** — no `QPrintDialog` (`프린터 속성`), no `QProgressDialog` (`멈추기`), no result/error `QMessageBox`es (`KaPrintDialog.cpp:575-668` → `dlg_drawing_print_actions.rs:65-83` only sets `draw_guide_message`).
9. **D4 `KaDrawingPdfProgress`** — hardcoded `"PDF를 그리는 중... (용지 210x297 mm · 300 DPI)"` and static `160.0 px` bar; `취소` only closes modal without cancelling export (`KaDrawingPdfProgress.cpp:15-58` → `dlg_pdf_preparing.rs:21-55`).
10. **D2 Drawing message & input boxes (14 boxes)** — all `QMessageBox` / `KaUserError::warn` / `QInputDialog` prompts (`표제란`, `도면 보관`, `도면 열기`, `도면 목록`, `레이어 삭제`, `PDF`, `인쇄`) missing (`KaDrawingStudio.cpp:3294-3887`; `KaDrawingStudioSheets.cpp:47-164` → `gui_tab_drawing_popups.rs:200-226` status bar only).
11. **D1 `도면 목록` popup** — missing `열기:` prefix, `보관한 도면 지우기` submenu, and `제출 PDF는 지금 열린 도면으로 만듭니다` footer (`KaDrawingSheetSet.cpp:135-161` → `gui_tab_drawing_popups.rs:123-226`).
12. **D1 `세트` button** — cycles 4 hardcoded presets in status bar instead of opening `KaDrawingLayerSets` save/apply/delete menu (`KaDrawingStudio.cpp:696-709`; `KaDrawingLayerSets.cpp:63-104` → `gui_layer_panel_tree.rs:101-120`).
13. **D1 `축척 글자` & `좌표계` buttons** — no-op on click (`KaDrawingStudio.cpp:1160-1167` → `gui_tab_drawing_right_b.rs:98-107`).
14. **D1 `선택 항목 크기` spin box** — always enabled in Rust instead of disabled when no decoration is selected (`KaDrawingStudio.cpp:1151, :3077` → `gui_tab_drawing_right_b.rs:86-94`).
15. **D1 `지도 조정` top banner** — missing `레이어를 가운데` button (`KaDrawingStudio.cpp:835` → `gui_tab_drawing_popups.rs:19-59`).
16. **D1 `범례` & `표제란` defaults & behavior** — `표제란` defaults to `true` (optional in C++), no title input prompt, no tooltips (`KaDrawingStudio.cpp:908-946`; `KaDrawingStudioSheets.cpp:47` → `gui_tab_drawing_right.rs:31-65`, `sheet_drawing.rs:82-83`).
17. **D1 `도면 정보` default scale & `적용` button** — default scale `1000` in C++ vs `5000` in Rust; `적용` only sets status text; 14 scale chips not checkable (`KaDrawingStudio.cpp:1064, :1093` → `sheet_drawing.rs:72`, `gui_tab_drawing_right.rs:160-165`, `gui_tab_drawing_right_b.rs:15-47`).
18. **D1 `좌표 격자` default** — `false` in C++ vs `true` in Rust (`KaDrawingStudio.cpp:1553` → `sheet_drawing.rs:73`).
19. **D1 `범례 제목` placeholder** — missing `"제목을 입력하세요"` (`KaDrawingStudio.cpp:953` → `gui_tab_drawing_right.rs:69-72`).
20. **D1 `방위 (도북)` & `전체 켜기/끄기` tooltips** — missing tooltips (`KaDrawingStudio.cpp:691, :1000, :1015` → `gui_layer_panel_tree.rs:88-96`, `gui_tab_drawing_right.rs:95-132`).
21. **D1 `Delete`/`Backspace` key** — does not delete selected drawing items (`KaDrawingStudio.cpp:3561-3565` → `gui_shell.rs:84-88`).
22. **D3 `방법` default & `600 DPI` text & ledger** — `print_single_sheet` defaults to `true` (`probe.actualSize` in C++); `600 DPI (가는 선이 많은 실측도)` → `600 DPI (고정밀 제출용)` + missing tooltip; `배열` ledger row missing and `인쇄 크기` hardcoded `"실제 크기 100%"`; `인쇄 · %1장 + 안내 1장` missing `+ 안내 1장` (`KaPrintDialog.cpp:155, :158-160, :223, :524`; `KaPrintDialogPlate.cpp:178, :207` → `dlg_drawing_print_options.rs:107, :171-191`, `dlg_drawing_print_actions.rs:58`, `modal_controller_state.rs:176`).
23. **D1 EXTRA controls** — inline `지도 좌표격자와 같게 (20 m)` checkbox (`gui_tab_drawing_right_b.rs:127-130`) and static `"투명도·밝기: 조절할 레이어를 고르세요"` pill (`gui_tab_drawing_sheet.rs:19-36`).

---

## Could not determine

- **CTX-L (Drawing Studio layer tree context menu) control-by-control diff**: Looked at `MainWindowContextMenus.cpp:176-448` and `gui_layer_panel_context_menu.rs:13-114`; did not enumerate all ~25 rows of `layer_menu_model.rs` here because `layer_menu_model.rs` is shared with the Map tab (covered by the Map/Layer-panel survey).

---

## Counts

- **Windows / menus audited**: 7 (`D1`, `D2` [14 boxes], `D3` [+ 8 sub-dialogs/boxes], `D4`, `C12`, `CTX-L`, `CTX-M`)
- **Controls / boxes counted**:
  - `D1`: 38 C++ controls, 39 Rust rows (37 mapped + 2 EXTRA; 18 `yes`, 19 `no`, 2 `EXTRA`)
  - `D2`: 14 C++ boxes, 0 Rust boxes (14 `MISSING`)
  - `D3`: 19 C++ controls (+ 8 sub-dialogs/boxes), 13 Rust controls (6 `yes`, 7 `no`, 6 `MISSING` main controls + 8 `MISSING` sub-dialogs)
  - `D4`: 3 C++ controls, 3 Rust controls (0 `yes`, 3 `no`)
  - `C12`: 5 C++ controls, 0 Rust dialog controls (5 `MISSING`)
  - `CTX-M`: 5 C++ menu actions, 0 Rust actions (5 `MISSING`)
- **Differences by column**:
  - `text`: 10
  - `tooltip / placeholder`: 11
  - `default / range`: 5
  - `enabled-when`: 4
  - `what it does`: 15
  - `MISSING` (controls + boxes): 42
  - `EXTRA`: 2

---

## Verification (Recount & 30 Citations)

- **Recount**: Re-verified C++ source lines for D1 (`KaDrawingStudio.cpp:649-1342`), D2 (14 boxes in `KaDrawingStudio.cpp` & `KaDrawingStudioSheets.cpp`), D3 (`KaPrintDialog.cpp:69-253`, `KaPrintDialogPlate.cpp:60-230`), D4 (`KaDrawingPdfProgress.cpp:10-84`), C12 (`KaDrawingStudio.cpp:1460-1518`), CTX-M (`KaDrawingStudio.cpp:315-355`). Table row counts match.
- **30 Citations Checked**: Verified exact file:line and verbatim text for 30 sampled citations across all 6 sections (`KaDrawingStudio.cpp:332, :374, :675, :684, :691, :701, :773, :833, :912, :935, :943, :953, :975, :1025, :1062, :1145, :1152, :1299, :1462, :1469, :1477, :3295, :3821, :3854`, `KaDrawingStudioSheets.cpp:47, :88, :109, :160`, `KaPrintDialog.cpp:74, :139, :155`, `KaDrawingPdfProgress.cpp:19, :48`) — **30/30 exact**.
