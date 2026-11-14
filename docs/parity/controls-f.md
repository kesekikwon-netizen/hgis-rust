# Controls Parity Audit — Group F (Report Studio, 3D Terrain Tabs, and App-Wide Message Boxes)

Target C++ reference: `target/cpp-ref/src/app/` (read-only).
Target Rust workspace: `crates/strata-app/src/`.

---

## Summary Index

| ID | Window / Box Group | C++ Source | Rust Counterpart | Status |
|---|---|---|---|---|
| D5 | `Strata · 보고서` | `KaHwpReportStudio.cpp:37`, `KaHwpReportStudioActions.cpp` | Not found | **MISSING** |
| D6 | `Strata · 한글(HWPX / HWP) A4 미리보기 창` | `KaHwpLivePaperWindow.cpp:7` | Not found | **MISSING** |
| D7 | Report & assessment message boxes | `KaHwpReportStudioActions.cpp:185, :197, :207`; `KaHwpAssessmentActions.cpp:44, :52, :65` | Not found | **MISSING** |
| D8 | `입체지형` tab | `KaTerrain3dStudio.cpp:77`, `KaStrata3dLink.cpp:15`, `MainWindow.cpp:1170` | `gui_tab_terrain3d.rs:19`, `gui_tab_terrain3d_view.rs:12` | Partial (shell only; `화면을 입체로` disabled, `strata-3d` links missing, blank view) |
| D9 | `입체지형 도면` tab | `KaTerrain3dLayoutStudio.cpp:72`, `MainWindowExport.cpp:373` | Not found | **MISSING** |
| D10 | 3D message boxes | `KaTerrain3dStudio.cpp:466, :477, :543`; `KaTerrain3dLayoutStudio.cpp:385, :478, :491`; `MainWindowExport.cpp:323` | `gui_tab_terrain3d.rs:64, :74` (status bar only for 2; rest missing) | **MISSING** (no modal boxes) |
| D12 | Generic error & information boxes (`KaUserError` / `notify`) | `KaUserError.cpp:65-101`, `MainWindow.cpp:440-445` | `feature_record.rs:133` (struct only; no modal renderer) | **MISSING** |
| D13 | Session confirm boxes | `MainWindowSession.cpp:671, :804, :1059, :1090, :1187`; `MainWindowOffline.cpp:251, :302` | Not found (only `render_close_prompt` in `shell_dialogs.rs:67` for app close) | **MISSING** |
| D14 | Not-built-yet boxes / deferred controls | `MainWindowSession.cpp:1629, :1730` | `아직 Rust 판에서 연결하지 않았습니다 (지시 <id>)` tooltips across 9 modules | Different pattern (C++ 2 stub boxes vs Rust 9 deferred tooltip sites) |

---

## D5. 「Strata · 보고서」 (`KaHwpReportStudio`)

- **C++ title**: `"Strata · 보고서"` (`KaHwpReportStudio.cpp:37`), tab title `"보고서"` (`KaHwpReportStudioActions.cpp:253`).
- **How opened**:
  - **C++**: `"더보기"` menu action `"보고서 작성 (Ctrl+G)"` (`actionReportStudio`, `KaHwpReportStudioActions.cpp:262-270`) or `Ctrl+G`.
  - **Rust**: **MISSING** — not found in `crates/strata-app/src/`.
- **Modality**: C++ `QDialog` embedded as `Qt::Widget` tab inside `viewTabs` (`KaHwpReportStudioActions.cpp:245-256`). Rust: **MISSING**.
- **Size rules**: C++ 3-pane `QSplitter` with stretch factors `4 : 6 : 3` (`KaHwpReportStudio.cpp:205-207`); `m_siteDetailView->setMaximumHeight(135)` (`:78`); `m_stepList->setMaximumHeight(150)` (`:189`). Rust: **MISSING**.

### D5 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `근거 확인` | none | static | always | Left pane header caption | `KaHwpReportStudio.cpp:49` | — | **MISSING** |
| 2 | toolbutton (check) | `500m` | none | checked=`true` | always | Sets `m_radiusMeters = 500.0`, refreshes project | `KaHwpReportStudio.cpp:51, :235` | — | **MISSING** |
| 3 | toolbutton (check) | `1000m` | none | checked=`false` | always | Sets `m_radiusMeters = 1000.0`, refreshes project | `KaHwpReportStudio.cpp:52, :236` | — | **MISSING** |
| 4 | check box | `보고서 원문` | none | checked=`true` | always | Toggles RAG full-text slot & refreshes | `KaHwpReportStudio.cpp:53-54, :237` | — | **MISSING** |
| 5 | label | `m_summaryLabel` | none | dynamic | always | Word-wrapped evidence summary | `KaHwpReportStudio.cpp:60-61` | — | **MISSING** |
| 6 | table (`0×8`) | `연번`, `유적명`, `거리`, `방위`, `시대`, `유구`, `층위`, `보고서` | none | single row selection, no edit | always | Selecting row calls `onSiteRowSelected(r)` | `KaHwpReportStudio.cpp:64-74, :238` | — | **MISSING** |
| 7 | label + text browser | `선택 유적 원문 및 ⑤ 조사구역과의 연관성` / `m_siteDetailView` | none | `maxHeight(135)` | always | Shows selected site text & relation | `KaHwpReportStudio.cpp:76-79` | — | **MISSING** |
| 8 | label | `m_connStatusLabel` | none | dynamic | always | Connection status text | `KaHwpReportStudio.cpp:81-83` | — | **MISSING** |
| 9 | toolbutton (check) | `고찰` | none | checked=`true` | always | Sets `m_isThesisMode = false` | `KaHwpReportStudio.cpp:91, :244` | — | **MISSING** |
| 10 | toolbutton (check) | `논문` | none | checked=`false` | always | Sets `m_isThesisMode = true` | `KaHwpReportStudio.cpp:92, :244` | — | **MISSING** |
| 11 | toolbutton (check) | `용지` | none | checked=`true` | always | Switches center stack to `m_previewView` (0) | `KaHwpReportStudio.cpp:97, :239-240` | — | **MISSING** |
| 12 | toolbutton (check) | `검수` | none | checked=`false` | always | Switches center stack to `m_verifyView` (1) | `KaHwpReportStudio.cpp:98, :239-240` | — | **MISSING** |
| 13 | toolbutton (check) | `비교` | none | checked=`false` | always | Switches center stack to `m_compareView` (2) | `KaHwpReportStudio.cpp:99, :239-240` | — | **MISSING** |
| 14 | toolbutton (check) | `DBpia 로그인` | none | checked=`false` | always | Switches center stack to `dbpiaPane` (3) | `KaHwpReportStudio.cpp:100, :239-242` | — | **MISSING** |
| 15 | toolbutton (check) | `평가` | none | checked=`false` | always | Switches center stack to `m_assessView` (4) & `refreshAssessmentView()` | `KaHwpReportStudio.cpp:101, :239-241` | — | **MISSING** |
| 16 | combo box | `지역: 자동(조사지역)`, `6대 도: 전국`, `6대 도: 경상도`, `6대 도: 경기도`, `6대 도: 충청도`, `6대 도: 전라도`, `6대 도: 강원도`, `6대 도: 제주도` | none | index `0` (`지역: 자동(조사지역)`) | always | Refreshes project with selected region | `KaHwpReportStudio.cpp:109-113, :245` | — | **MISSING** |
| 17 | combo box | `90%`, `100%`, `120%` | none | index `1` (`100%`) | always | Updates preview zoom (`syncUiFromSession()`) | `KaHwpReportStudio.cpp:115-117, :246` | — | **MISSING** |
| 18 | toolbutton (menu) | `저장 ▾` (menu: `한글 표준(.hwpx) 저장…`, `한글 5.0(.hwp) 저장…`, `평가 자료 저장…`, separator, `기관 한글 양식(.hwpx/.hwp) 열기…`, `별도 창으로 보기…`) | none | `InstantPopup` | always | Triggers HWPX/HWP/assessment save, template open, or `openLivePaperWindow()` | `KaHwpReportStudio.cpp:119-129` | — | **MISSING** |
| 19 | check box (DBpia pane) | `로그인 기억` | `켜면 이 PC에 DBpia 로그인을 저장합니다. 공용 PC에서는 켜지 마세요.` | `false` | always | Sets persistent cookies on `dbpiaProfile()` | `KaHwpReportStudio.cpp:155-159` | — | **MISSING** |
| 20 | 3 buttons (DBpia pane) | `개인 로그인`, `기관 로그인`, `DBpia AI` (plus hidden `자동 문헌수집 실행`) | none | `btnHarvest` hidden (`setVisible(false)`) | always | Opens DBpia URLs in `QWebEngineView` | `KaHwpReportStudio.cpp:150-154, :173-175` | — | **MISSING** |
| 21 | stacked browsers | `m_previewView`, `m_verifyView`, `m_compareView`, `dbpiaPane`, `m_assessView` | none | index `0` | always | Displays paper preview, verify, compare, DBpia web, assessment | `KaHwpReportStudio.cpp:141-181` | — | **MISSING** |
| 22 | label + list | `작업 순서` / `m_stepList` | none | `maxHeight(150)` | always | Displays 4-stage progress rail | `KaHwpReportStudio.cpp:187-190` | — | **MISSING** |
| 23 | label + browser | `진행 기록` / `m_chatView` | none | — | always | Chat/action log | `KaHwpReportStudio.cpp:191-193` | — | **MISSING** |
| 24 | label + table (`0×2`) | `한글 양식 칸 편집` / headers `양식 칸`, `값` | none | col 0 read-only, col 1 editable | always | `onFieldCellChanged` updates session field | `KaHwpReportStudio.cpp:194-200, :247`; `KaHwpReportStudioActions.cpp:224` | — | **MISSING** |
| 25 | line edit | `m_promptEdit` | placeholder: `다차원 분석·집필 지시 (예: 전남 원형주거지와 방형주거지 현황 / 전국 후기구석기 분포 / 지역별 시대별 건물지 크기 변화)` | empty | always | `returnPressed` calls `onSendPromptClicked()` | `KaHwpReportStudio.cpp:211-213, :249` | — | **MISSING** |
| 26 | push button | `작성 시작` | none | `objectName("primary")` | always | Calls `onSendPromptClicked()` | `KaHwpReportStudio.cpp:214-216, :248` | — | **MISSING** |

- **Keyboard & closing**: `Enter` in `m_promptEdit` submits prompt (`KaHwpReportStudio.cpp:249`); embedded tab, no separate close box. Rust: **MISSING**.

---

## D6. HWPX / HWP A4 Preview Window (`KaHwpLivePaperWindow`)

- **C++ title**: `"Strata · 한글(HWPX / HWP) A4 미리보기 창"` (`KaHwpLivePaperWindow.cpp:7`).
- **How opened**:
  - **C++**: `KaHwpReportStudio` `"저장 ▾"` → `"별도 창으로 보기…"` → `openLivePaperWindow()` (`KaHwpReportStudio.cpp:128`, `KaHwpReportStudioActions.cpp:129-140`).
  - **Rust**: **MISSING**.
- **Modality**: Non-modal `QDialog` with `Qt::Window | Qt::WindowMinMaxButtonsHint | Qt::WindowCloseButtonHint`, `show()` (`KaHwpLivePaperWindow.cpp:6`, `KaHwpReportStudioActions.cpp:136`). Rust: **MISSING**.
- **Size rules**: `resize(920, 960)` (`KaHwpLivePaperWindow.cpp:8`). Rust: **MISSING**.

### D6 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `한글(HWPX/HWP) A4 미리보기 준비됨` | none | dynamic via `updatePaperHtml` | always | Shows live status text | `KaHwpLivePaperWindow.cpp:15, :88` | — | **MISSING** |
| 2 | progress bar | (progress) | none | `100`, range `0..100`, `fixedWidth(130)` | always | Shows report generation progress | `KaHwpLivePaperWindow.cpp:16-19` | — | **MISSING** |
| 3 | push button | `축소 -` | none | — | always | Decreases `m_zoomPercent` by `15` (min `70`) | `KaHwpLivePaperWindow.cpp:21, :53-57` | — | **MISSING** |
| 4 | label | `100%` | none | `100%` | always | Displays `m_zoomPercent` | `KaHwpLivePaperWindow.cpp:22, :55` | — | **MISSING** |
| 5 | push button | `확대 +` | none | — | always | Increases `m_zoomPercent` by `15` (max `160`) | `KaHwpLivePaperWindow.cpp:23, :58-62` | — | **MISSING** |
| 6 | check box | `양식 칸 표시` | none | `true` | always | Toggles field mark highlights & refreshes | `KaHwpLivePaperWindow.cpp:24-25, :63-65` | — | **MISSING** |
| 7 | push button | `.hwpx 저장` | none | — | always | Invokes `onSaveHwpxClicked` on parent | `KaHwpLivePaperWindow.cpp:26, :74-76` | — | **MISSING** |
| 8 | push button | `.hwp 저장` | none | — | always | Invokes `onSaveHwp5Clicked` on parent | `KaHwpLivePaperWindow.cpp:27, :77-79` | — | **MISSING** |
| 9 | text browser | `m_paperBrowser` | none | `openExternalLinks(false)` | always | Renders A4 HTML preview | `KaHwpLivePaperWindow.cpp:39-41` | — | **MISSING** |
| 10 | line edit | `m_livePromptEdit` | placeholder: `작성·비교·치환 지시를 입력하세요` | empty | always | `returnPressed` submits live prompt | `KaHwpLivePaperWindow.cpp:44-45, :66-73` | — | **MISSING** |
| 11 | push button | `문서 반영` | none | `objectName("primary")` | always | Submits `m_livePromptEdit` text | `KaHwpLivePaperWindow.cpp:46-48, :72` | — | **MISSING** |

- **Keyboard & closing**: `Enter` in `m_livePromptEdit` submits prompt (`:73`); `Esc` / window X closes `QDialog`. Rust: **MISSING**.

---

## D7. Report & Assessment Message Boxes

All 6 C++ boxes are **MISSING** in Rust.

| # | kind | C++ title (verbatim) | C++ text (verbatim) | C++ buttons | C++ condition | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|
| 1 | `QMessageBox::warning` | `양식 열기 실패` | `err` | `OK` | `onOpenTemplateClicked()` when `openTemplateAndFill` fails | `KaHwpReportStudioActions.cpp:185` | — | **MISSING** |
| 2 | `QMessageBox::warning` | `저장 실패` | `err` | `OK` | `onSaveHwpxClicked()` when `saveAs` fails | `KaHwpReportStudioActions.cpp:197` | — | **MISSING** |
| 3 | `QMessageBox::warning` | `저장 실패` | `err` | `OK` | `onSaveHwp5Clicked()` when `saveAs` fails | `KaHwpReportStudioActions.cpp:207` | — | **MISSING** |
| 4 | `QMessageBox::warning` | `저장 실패` | `b.ledger.error` | `OK` | `onSaveAssessmentClicked()` when `!b.ledger.ok()` | `KaHwpAssessmentActions.cpp:44` | — | **MISSING** |
| 5 | `QMessageBox::warning` | `저장 실패` | `err` | `OK` | `onSaveAssessmentClicked()` when `writeBundle` fails | `KaHwpAssessmentActions.cpp:52` | — | **MISSING** |
| 6 | `QMessageBox::question` | `저장 전 확인` | `문서에 남아 있으면 안 되는 글자가 %1건 있습니다.\n\n%2\n\n그래도 저장할까요?` | `Yes \| No` | `confirmExportClean()` when `scanExportText` finds issues | `KaHwpAssessmentActions.cpp:65-67` | — | **MISSING** |

---

## D8. 「입체지형」 Tab (`KaTerrain3dStudio`)

- **C++ title**: Tab title `"입체지형"` (`MainWindow.cpp:1170`).
- **How opened**:
  - **C++**: Ribbon `"입체지형"` button adds `m_terrain3dStudio` to `m_viewTabs` (`MainWindow.cpp:1167-1171`).
  - **Rust**: Tab bar `"입체지형"` (`gui_ribbon_tabbar.rs:82`) opens `MainTab::Terrain3d` (`shell_workspace.rs:122`).
- **Modality**: Non-modal embedded tab on both sides.
- **Size rules**:
  - **C++**: Left `terrain3dPanel` `minimumWidth(200)`, `maximumWidth(260)` (`KaTerrain3dStudio.cpp:86-87`); `m_view` fills remaining space (`:135`).
  - **Rust**: `SidePanel::left("ka_terrain3d_left_panel").resizable(true).default_width(230.0).width_range(200.0..=260.0)` (`gui_tab_terrain3d.rs:20-23`); `CentralPanel` fills remaining space (`gui_tab_terrain3d_view.rs:13`).

### D8 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust text | Rust tooltip / placeholder | Rust default, range, step, suffix | Rust enabled-when rule | Rust what it does | Rust file:line | same? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | label (caption) | `입체지형` | none | static | always | Panel caption | `KaTerrain3dStudio.cpp:91` | `입체지형` | none | static | always | Panel caption | `gui_tab_terrain3d.rs:33` | yes |
| 2 | label | `지금 지도에 보이는 화면을 고해상 입체로 만듭니다. 탭을 열면 자동으로 시작합니다.` | none | visible until `rebuildMesh()` | always | Empty-state hint | `KaTerrain3dStudio.cpp:94-100, :160` | `지금 지도에 보이는 화면을 고해상 입체로 만듭니다. 탭을 열면 자동으로 시작합니다.` | none | always visible | always | Hint label | `gui_tab_terrain3d.rs:39-45` | yes |
| 3 | push button | `화면을 입체로` | none | — | always | Calls `tryAutoFill()` | `KaTerrain3dStudio.cpp:102, :139` | `화면을 입체로` | `아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)` | disabled | `false` (disabled) | Disabled until W-56 | `gui_tab_terrain3d.rs:49-57` | no (enabled, tooltip, what it does) |
| 4 | push button | `이미지 저장` | none | — | always | Calls `saveImage()` (shows `QMessageBox` if no scene) | `KaTerrain3dStudio.cpp:104, :140, :464` | `이미지 저장` | none | enabled | always | Calls `show_status_message("먼저 지도 화면을 입체로 만드세요.")` | `gui_tab_terrain3d.rs:59-65` | no (what it does: status bar vs modal/save) |
| 5 | push button | `입체지형 도면출력` | none | — | always | Calls `runExportSheet()` (shows `QMessageBox` if no scene) | `KaTerrain3dStudio.cpp:106, :141, :541` | `입체지형 도면출력` | none | enabled | always | Calls `show_status_message("먼저 지도 화면을 입체로 만드세요.")` | `gui_tab_terrain3d.rs:66-75` | no (what it does: status bar vs modal/sheet) |
| 6 | label + double spin box | `높이 과장` / `2.0 배` | none | `2.0`, range `0.2..12.0`, step `0.1`, decimals `1`, suffix `" 배"` | always | Calls `rebuildMesh()` | `KaTerrain3dStudio.cpp:111-119, :142` | `높이 과장` / `DragValue` | none | `2.0`, range `0.2..=12.0`, speed `0.1`, decimals `1`, suffix `" 배"` | always | Edits `paleo_3d.z_exaggeration` | `gui_tab_terrain3d.rs:79-92` | yes |
| 7 | label | `지도 화면을 입체로 만드는 중…` | none | dynamic (`DEM %1×%2 · 높이 %3–%4 m · 끌어서 돌리기`) | always | Status text | `KaTerrain3dStudio.cpp:120-123, :161` | `아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)` | none | static | always | Deferred label | `gui_tab_terrain3d.rs:96-100` | no (text) |
| 8 | push button | `strata-3d로 열기` | `열려 있는 조사를 정밀 3D 앱(strata-3d)으로 넘겨 실측 지형·건물·유적 위에서 500 m·1000 m 평가를 받습니다.` | — | `!busy` | Calls `KaStrata3dLink::openSurvey()` | `KaStrata3dLink.cpp:17-19, :27` | — | — | — | — | — | — | **MISSING** |
| 9 | push button | `strata-3d 도면 받기` | `strata-3d 에 떠 있는 장면의 입체지형 도면(PNG·PDF)을 받아 저장합니다.` | — | `!busy` | Calls `KaStrata3dLink::receiveSheet()` | `KaStrata3dLink.cpp:20-22, :28` | — | — | — | — | — | — | **MISSING** |
| 10 | 3D viewport | `KaTerrain3dView` | none | interactive 3D orbit mesh | always | Renders 3D terrain mesh & texture | `KaTerrain3dStudio.cpp:127-128` | `render_terrain3d_view` | none | blank white rect | always | Paints white rectangle only | `gui_tab_terrain3d_view.rs:12-20` | no (blank white rect) |

- **Keyboard & closing**: Embedded tab, no default button / Enter / Esc handler.

---

## D9. 「입체지형 도면」 Tab (`KaTerrain3dLayoutStudio`)

- **C++ title**: Tab title `"입체지형 도면"` (`MainWindowExport.cpp:373`).
- **How opened**:
  - **C++**: `KaTerrain3dStudio` `"입체지형 도면출력"` → `MainWindow::placeTerrain3dOnSheet()` → `openTerrain3dLayout()` (`MainWindowExport.cpp:318-378`).
  - **Rust**: **MISSING** — not found in `crates/strata-app/src/`.
- **Modality**: Non-modal embedded `QWidget` tab in `m_viewTabs` (`MainWindowExport.cpp:370-374`). Rust: **MISSING**.
- **Size rules**: C++ left column `minWidth(220)`, `maxWidth(280)` (`KaTerrain3dLayoutStudio.cpp:82-83`); right column `minWidth(260)`, `maxWidth(300)` (`:171-172`). Rust: **MISSING**.

### D9 Controls Table

| # | kind | C++ text (verbatim) | C++ tooltip / placeholder (verbatim) | C++ default, range, step, suffix | C++ enabled-when rule | C++ what it does | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|---|
| 1 | label | `레이어` | none | static | always | Left card caption | `KaTerrain3dLayoutStudio.cpp:93` | — | **MISSING** |
| 2 | tree view | `m_layerTree` (`QgsLayerTreeView`) | none | project layers | visible when `!mapLayers().isEmpty()` | Layer visibility checkboxes | `KaTerrain3dLayoutStudio.cpp:97-116, :128-129` | — | **MISSING** |
| 3 | label (empty) | `이 도면에는 레이어가 없습니다.\n지도에서 켠 레이어가\n여기에 보입니다.` | none | centered | visible when `mapLayers().isEmpty()` | Empty state | `KaTerrain3dLayoutStudio.cpp:117-127` | — | **MISSING** |
| 4 | layout view | `m_view` (`QgsLayoutView`) | none | interactive sheet | always | Displays 3D layout sheet | `KaTerrain3dLayoutStudio.cpp:143-152` | — | **MISSING** |
| 5 | label | `입체지형 도면 — 범례는 「범례」로` | none | dynamic | always | Status line below view | `KaTerrain3dLayoutStudio.cpp:154-156` | — | **MISSING** |
| 6 | label | `도면 항목` | none | static | always | Card 1 caption | `KaTerrain3dLayoutStudio.cpp:182` | — | **MISSING** |
| 7 | toolbutton | `범례` | `범례` | — | always | Calls `addLegend()` | `KaTerrain3dLayoutStudio.cpp:187-189` | — | **MISSING** |
| 8 | toolbutton | `PDF 내보내기` | `PDF 내보내기` | `objectName("btnPrimary")` | always | Calls `exportPdf()` | `KaTerrain3dLayoutStudio.cpp:190-193` | — | **MISSING** |
| 9 | label + line edit | `제목` / `범례` | placeholder: `제목을 입력하세요` | `"범례"` | always | Calls `applyLegendStyle()` | `KaTerrain3dLayoutStudio.cpp:197-202` | — | **MISSING** |
| 10 | label + spin + 2 checks | `글자` / `10 pt` / `B` / `I` | none | `10` (`5..24`, `" pt"`), `B=true`, `I=false` | always | Calls `applyLegendStyle()` | `KaTerrain3dLayoutStudio.cpp:203-219` | — | **MISSING** |
| 11 | label + 4 tiles + spin | `방위` / `북 글자`, `북 화살`, `나침반`, `바람장미` / `크기` `28 mm` | tile text | `28.0` (`12.0..80.0`, decimals `0`, `" mm"`) | always | Calls `applyNorthKind(i)` | `KaTerrain3dLayoutStudio.cpp:227-249` | — | **MISSING** |
| 12 | label + spin + button | `도면 정보` / `1 :` `1000` / `적용` | none | `1000` (`10..5000000`, step `10`) | always | Calls `applyScaleNow()` | `KaTerrain3dLayoutStudio.cpp:257-271` | — | **MISSING** |
| 13 | 9 preset buttons | `100`, `200`, `250`, `300`, `400`, `500`, `1000`, `2000`, `5000` | none | — | always | Sets `m_scaleSpin` & calls `applyScaleNow()` | `KaTerrain3dLayoutStudio.cpp:272-298` | — | **MISSING** |
| 14 | 3 tile buttons | `쌍칸`, `외칸`, `눈금` | tile text | — | always | Calls `applyBarStyle(style)` | `KaTerrain3dLayoutStudio.cpp:304-311` | — | **MISSING** |
| 15 | 2 tile buttons | `축척 글자`, `좌표계` | tile text | — | always | Calls `addScaleLabel()` / `addCrsLabel()` | `KaTerrain3dLayoutStudio.cpp:314-322` | — | **MISSING** |

- **Keyboard & closing**: `Ctrl+Z` (`undoLastChange`) and `Delete` (`deleteSelectedItems`) (`KaTerrain3dLayoutStudio.cpp:158-167`). Rust: **MISSING**.

---

## D10. 3D Message Boxes

| # | kind | C++ title (verbatim) | C++ text (verbatim) | C++ condition | C++ file:line | Rust counterpart | same? |
|---|---|---|---|---|---|---|---|
| 1 | `QMessageBox::information` | `입체지형` | `먼저 지도 화면을 입체로 만드세요.` | `saveImage()` when `!hasScene()` | `KaTerrain3dStudio.cpp:466-467` | `show_status_message("먼저 지도 화면을 입체로 만드세요.")` (`gui_tab_terrain3d.rs:64`) | no (status bar vs modal box) |
| 2 | `KaUserError::warn` | `입체지형` | `입체지형 그림을 저장하지 못했습니다` + `err` | `saveImage()` when `!exportImage(...)` | `KaTerrain3dStudio.cpp:477` | None | **MISSING** |
| 3 | `QMessageBox::information` | `입체지형 도면출력` | `먼저 지도 화면을 입체로 만드세요.` | `runExportSheet()` when `!hasScene()` | `KaTerrain3dStudio.cpp:543-544` | `show_status_message("먼저 지도 화면을 입체로 만드세요.")` (`gui_tab_terrain3d.rs:74`) | no (status bar vs modal box) |
| 4 | `QMessageBox::information` | `입체지형 도면` | `err` | `KaTerrain3dLayoutStudio::warn(err)` | `KaTerrain3dLayoutStudio.cpp:385` | None | **MISSING** |
| 5 | `QMessageBox::information` | `입체지형 도면` | `먼저 입체지형 도면출력을 하세요.` | `exportPdf()` when `!ly \|\| !m_project` | `KaTerrain3dLayoutStudio.cpp:478-479` | None | **MISSING** |
| 6 | `KaUserError::warn` | `입체지형 도면` | `PDF를 저장하지 못했습니다` + `err` | `exportPdf()` when `saved.isEmpty()` | `KaTerrain3dLayoutStudio.cpp:491` | None | **MISSING** |
| 7 | `QMessageBox::information` | `입체지형 도면출력` | `먼저 「화면을 입체로」로 지금 지도를 만드세요.` | `placeTerrain3dOnSheet()` when `!hasScene()` | `MainWindowExport.cpp:323-324` | None | **MISSING** |

---

## D12. Generic Error & Information Boxes (`KaUserError` / `MainWindow::notify`)

- **C++ `KaUserError::dress`** (`KaUserError.cpp:65-101`):
  - **Title**: `spec.title` (`KaUserError.cpp:67`).
  - **Icon & Text**: In `kaPaperLook` (`:69-77`), `QMessageBox::NoIcon`, plain heading `spec.what`, informative text `spec.why` + `spec.how`; in stock look (`:66, :78-81`), `Warning` or `Critical` icon, HTML with bold headings `무엇이 안 됐나`, `왜 그런가`, `어떻게 하나`. Optional `spec.details` via `setDetailedText` (`:82-83`).
  - **Buttons**: Optional `spec.actionLabel` (`AcceptRole`, `:85-86`), optional `"로그 폴더 열기"` (`ActionRole`, tooltip `"개발자에게 보낼 session.log가 있는 폴더를 엽니다: %1"`, `:88-95`), `"닫기"` (`RejectRole`, `:97`).
  - **Keyboard**: Default button = `action ? action : close` (`:99`); Escape button = `close` (`:100`).
- **C++ `MainWindow::notify`** (`MainWindow.cpp:440-445`):
  - Fallback when no `m_messageBar`: `QMessageBox::warning(this, title, body)` (`Notice::Warning` / `Critical`) or `QMessageBox::information(this, title, body)` (`Notice::Info` / `Success`), where `body = details.isEmpty() ? text : text + '\n' + details`.
- **Rust**: **MISSING** — `feature_record.rs:133` only defines a data struct comment referencing `KaUserError::warn`; no generic modal renderer exists (status bar is used instead, e.g. `app_state_layer_menu.rs:103`).

---

## D13. Session Confirm Boxes

All 7 C++ `QMessageBox::question` confirmation dialogs are **MISSING** in Rust.

| # | kind | C++ title (verbatim) | C++ text (verbatim) | C++ buttons & default | C++ condition | C++ file:line | Rust | same? |
|---|---|---|---|---|---|---|---|---|
| 1 | `QMessageBox::question` | `새 조사` | `현재 조사에 저장하지 않은 작업이 있습니다. 새 조사를 만들기 전에 저장할까요?` | `Save \| Discard \| Cancel` (default `Save`) | `newSurvey()` when `surveyHasUnsavedChanges()` | `MainWindowSession.cpp:671-674` | — | **MISSING** |
| 2 | `QMessageBox::question` | `레이어` | `%1\n\n지금 「새 조사」를 만들까요?` (`err` or `먼저 「새 조사」로 저장 경로를 만드세요.`) | `Yes \| No` (default `Yes`) | `ensureDomainLayerForEdit()` when `!vl` | `MainWindowSession.cpp:804-808` | — | **MISSING** |
| 3 | `QMessageBox::question` | `임시 자료 정리` | `저장·내보내기 도중 끊겨 조사 폴더에 남은 임시 자료 %1개(약 %2 MB)를 지울까요?\n\n%3\n\n복구 사본과 저장 실패로 남긴 세대는 지우지 않습니다.` | `Yes \| No` (default `No`) | `cleanStaleSurveyStaging()` when `!stale.isEmpty()` | `MainWindowSession.cpp:1055-1060` | — | **MISSING** |
| 4 | `QMessageBox::question` | `참조 벡터를 조사 파일 밖으로` | `조사 파일 안에 있는 참조 벡터 %1개를 바깥 파일로 옮기고, 조사 파일에서 해당 테이블을 뺍니다.\n\n%2\n\n조사 구역·유구·유물 등 조사 데이터는 그대로 둡니다. 원본 조사 파일은 검증된 다음 세대로 교체됩니다.` | `Yes \| No` (default `No`) | `extractEmbeddedReferenceVectors()` when `!names.isEmpty()` | `MainWindowSession.cpp:1084-1091` | — | **MISSING** |
| 5 | `QMessageBox::question` | `조사 열기` | `현재 조사에 저장하지 않은 작업이 있습니다. 선택한 조사를 열기 전에 저장할까요?` | `Save \| Discard \| Cancel` (default `Save`) | `confirmSaveBeforeOpeningSurvey()` when `surveyHasUnsavedChanges()` | `MainWindowSession.cpp:1187-1190` | — | **MISSING** |
| 6 | `QMessageBox::question` | `오프라인 저장` | `<policy.message>\n계속할까요?` | `Yes \| No` (default `No`) | `saveOfflineTilePack()` when `policy.askFirst` | `MainWindowOffline.cpp:251-253` | — | **MISSING** |
| 7 | `QMessageBox::question` | `오프라인 저장` | `지금 화면 범위를 타일 %1장(줌 %2~%3, 예상 크기 약 %4)으로 받아 둡니다.\n받는 동안 지도 작업을 계속할 수 있습니다. 계속할까요?` | `Yes \| No` (default `Yes`) | `saveOfflineTilePack()` before tile download | `MainWindowOffline.cpp:302-310` | — | **MISSING** |

---

## D14. Boxes / Controls That Only Say a Function Is Not Built Yet

### C++ Stub Boxes
1. `MainWindowSession.cpp:1629` — `QMessageBox::information(this, QStringLiteral("스텁"), QStringLiteral("다른 이름으로 저장 시뮬레이션"));`
2. `MainWindowSession.cpp:1730` — `QMessageBox::information(this, QStringLiteral("스텁"), QStringLiteral("프로젝트 열기 시뮬레이션"));`

### Rust Deferred-Control Tooltips / Labels
Rust has **no** modal boxes that only say a function is not built yet; instead it uses disabled controls with `"아직 Rust 판에서 연결하지 않았습니다 (지시 <id>)"` (and 1 status label in `gui_tab_terrain3d.rs:97`):
1. `dlg_drawing_print_actions.rs:12, :45` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)"` (`나눈 장을 PDF로 저장…`)
2. `dlg_heritage_browser_footer.rs:13` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 WB1)"`
3. `gui_ribbon_draw.rs:56` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 R7)"` (`GeoTIFF`)
4. `gui_ribbon_subtoolbar.rs:41` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 W-H1)"`
5. `gui_tab_drawing_right_b.rs:10, :144` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 R3)"` (`PDF 내보내기`)
6. `gui_tab_terrain3d.rs:11, :57, :97` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)"` (`화면을 입체로` button & status label)
7. `layer_menu_model.rs:109` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 {order})"` (e.g., `CM2a-3`, `R2`, `W-T1`, `R4b`)
8. `modal_export_footer.rs:9` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 R2)"`
9. `ngii_topographic_state.rs:13` — `"아직 Rust 판에서 연결하지 않았습니다 (지시 WB2)"`

---

## Differences to fix

1. **D5 `Strata · 보고서` (`KaHwpReportStudio`)** — entire studio tab & `Ctrl+G` action missing (`KaHwpReportStudio.cpp:37-274`, `KaHwpReportStudioActions.cpp:242-270` → not found in Rust).
2. **D6 HWPX/HWP A4 preview window (`KaHwpLivePaperWindow`)** — entire window missing (`KaHwpLivePaperWindow.cpp:5-80` → not found in Rust).
3. **D9 `입체지형 도면` (`KaTerrain3dLayoutStudio`)** — entire tab & `placeTerrain3dOnSheet()` missing (`KaTerrain3dLayoutStudio.cpp:72-496`, `MainWindowExport.cpp:318-378` → not found in Rust).
4. **D8 `입체지형` `화면을 입체로` & 3D viewport** — `화면을 입체로` disabled with `지시 W-56` and `render_terrain3d_view` is a blank white rect (`KaTerrain3dStudio.cpp:102, :127` → `gui_tab_terrain3d.rs:49-57`, `gui_tab_terrain3d_view.rs:12-20`).
5. **D8 `strata-3d로 열기` & `strata-3d 도면 받기`** — both buttons missing (`KaStrata3dLink.cpp:17-28` → not found in Rust).
6. **D12 Generic `KaUserError` & `MainWindow::notify` modal boxes** — no modal renderer (`KaUserError.cpp:65-101`, `MainWindow.cpp:440-445` → `feature_record.rs:133` struct only).
7. **D13 Session confirm boxes (7 `QMessageBox::question` boxes)** — all 7 missing (`MainWindowSession.cpp:671, :804, :1059, :1090, :1187`, `MainWindowOffline.cpp:251, :302` → not found in Rust).
8. **D7 Report & assessment message boxes (6 boxes)** — all 6 missing (`KaHwpReportStudioActions.cpp:185, :197, :207`, `KaHwpAssessmentActions.cpp:44, :52, :65` → not found in Rust).
9. **D10 3D message boxes (7 boxes)** — 5 missing, 2 replaced by status bar text (`KaTerrain3dStudio.cpp:466, :477, :543`, `KaTerrain3dLayoutStudio.cpp:385, :478, :491`, `MainWindowExport.cpp:323` → `gui_tab_terrain3d.rs:64, :74`).
10. **D8 `이미지 저장` & `입체지형 도면출력` & `m_status`** — only show status messages (`NO_SCENE_MESSAGE` / `DEFERRED_W56`) instead of `QMessageBox::information` / `QFileDialog` / live DEM status (`KaTerrain3dStudio.cpp:120, :464, :541` → `gui_tab_terrain3d.rs:59-75, :96-100`).

---

## Could not determine

- None: every window/box group in `ORDER.yaml` was located in C++ and checked across `crates/strata-app/src/`.

---

## Counts

- **Windows / box groups audited**: 9 (`D5`, `D6`, `D7`, `D8`, `D9`, `D10`, `D12`, `D13`, `D14`)
- **Controls / boxes counted**:
  - `D5`: 26 C++ controls, 0 Rust (26 `MISSING`)
  - `D6`: 11 C++ controls, 0 Rust (11 `MISSING`)
  - `D7`: 6 C++ boxes, 0 Rust (6 `MISSING`)
  - `D8`: 10 C++ controls, 8 Rust (3 `yes`, 5 `no`, 2 `MISSING`)
  - `D9`: 15 C++ controls, 0 Rust (15 `MISSING`)
  - `D10`: 7 C++ boxes, 2 status-bar equivalents (`no`), 5 `MISSING`
  - `D12`: 2 C++ generic box patterns (`KaUserError::dress`, `MainWindow::notify`), 0 Rust (`MISSING`)
  - `D13`: 7 C++ confirm boxes, 0 Rust (7 `MISSING`)
  - `D14`: 2 C++ stub boxes, 9 Rust deferred-tooltip/label sites
- **Differences by column**:
  - `text`: 1
  - `tooltip / placeholder`: 1
  - `enabled-when`: 1
  - `what it does`: 5
  - `MISSING` (controls + boxes): 72 (26 + 11 + 6 + 2 + 15 + 5 + 7)
  - `EXTRA`: 0

---

## Verification (Recount & 30 Citations)

- **Recount**: Re-verified C++ source counts for D5 (`KaHwpReportStudio.cpp:49-219`, 26 controls), D6 (`KaHwpLivePaperWindow.cpp:15-50`, 11 controls), D7 (6 boxes), D8 (`KaTerrain3dStudio.cpp:91-128` + `KaStrata3dLink.cpp:17-22`, 10 controls), D9 (`KaTerrain3dLayoutStudio.cpp:93-322`, 15 controls), D10 (7 boxes), D12 (2 patterns), D13 (7 boxes), D14 (2 C++ stub boxes, 9 Rust sites).
- **30 Citations Checked**: Verified exact file:line and verbatim text for 30 sampled citations across all sections (`KaHwpReportStudio.cpp:37, :49, :65, :76, :110, :120, :156, :213, :214`, `KaHwpLivePaperWindow.cpp:7, :15, :24, :45, :46`, `KaHwpReportStudioActions.cpp:185, :263`, `KaHwpAssessmentActions.cpp:65`, `KaTerrain3dStudio.cpp:95, :102, :117, :466, :543`, `KaStrata3dLink.cpp:17, :20`, `KaTerrain3dLayoutStudio.cpp:154, :478`, `KaUserError.cpp:67, :89`, `MainWindowSession.cpp:673, :1189`, `MainWindowOffline.cpp:304`) — **30/30 exact**.
