# UI Parity Backlog

Single ordered backlog built strictly from the seven parity difference lists:

1. [controls-a.md](controls-a.md) (`## Differences to fix (most visible first)`, 14 items)
2. [controls-b.md](controls-b.md) (`## Differences to fix`, 15 items)
3. [controls-c.md](controls-c.md) (`## Differences to fix`, 13 items)
4. [controls-d.md](controls-d.md) (`## Differences to fix`, 23 items)
5. [controls-e.md](controls-e.md) (`## Differences to fix`, 20 items)
6. [controls-f.md](controls-f.md) (`## Differences to fix`, 10 items)
7. `../../../docs/senior/UI_BUTTONS.md` (`## 3. Differences to fix (most visible first)`, 15 items)

Across those 110 entries, `controls-c.md #5` (`C11 「격자 원점」`) and `controls-e.md #18` (`KaGridOriginButton`) describe the exact same missing button and modal dialog and are merged into row `B034`, leaving **109** backlog rows.

Allowed tag values:
- **class**: `text`, `tooltip`, `default or range`, `enabled rule`, `state or look`, `behaviour`, `missing control`, `missing message`, `missing window`, `extra in Rust`
- **seen**: `1` (every session), `2` (common field task), `3` (rare)
- **risk**: `data`, `result`, `confusion`, `looks`
- **size**: `S` (text or rule in one place), `M` (one control with behaviour), `L` (window or function to build)
- **needs**: prerequisite feature name from the source tables or `docs/PARITY_STATUS.md`, else `—`

---

## 1. Full Backlog

| id | source (table file and the number in its list) | window | control | what differs (C++ value -> Rust value) | C++ file:line | Rust file:line | class | seen | risk | size | needs |
|---|---|---|---|---|---|---|---|---|---|---|---|
| B001 | controls-a.md #1 | `더보기` menu / `자료 준비 상태` (A7) | `자료 준비 상태` menu entry | Opens `ReferenceStatus` -> opens `AccountSettings` | `MainWindowOffline.cpp:342-356` | `crates/strata-app/src/gui_ribbon_menus_b.rs:82,114` | behaviour | 2 | confusion | S | — |
| B002 | controls-a.md #2 | `지적도 선 색·지번` (A11) | `선 색·지번 표시` dialog | Opens `지적도 선 색·지번` (`경계선 0.2mm`, `지번 표시`, notes) -> opens `CadastralDownload`; dialog missing | `MainWindowCadastral.cpp:56-80` | `crates/strata-app/src/gui_ribbon_menus_a.rs:164,183` | missing window | 2 | confusion | M | — |
| B003 | controls-a.md #3 | `토양도 불러오기 설정` (A13) | `토양도 파일 불러오기(SHP·GPKG)` | Opens SHP picker + `토양도 불러오기 설정` -> toggles overlay; dialog missing | `MainWindow.cpp:2643-2737` | `crates/strata-app/src/gui_ribbon_menus_a.rs:136,153` | missing window | 3 | confusion | L | — |
| B004 | controls-a.md #4 | `레이어 추가` (A12) | Entire dialog (`이름`, `5186 중부`/`5187 동부`, fill/stroke colors, `추가`) | Modal `레이어 추가` dialog -> entire dialog missing | `MainWindow.cpp:2441-2530` | not found in `crates/strata-app/src` (`MISSING`) | missing window | 2 | confusion | L | — |
| B005 | controls-a.md #5 | Start notice (A8) | `KaStartupSplash` (`Strata · 필드고고학 GIS 시작 안내`) | Startup attribution & progress splash on launch -> window missing (`main.rs` opens Home directly) | `KaStartupSplash.cpp:104` | `crates/strata-app/src/main.rs` (`MISSING`) | missing window | 1 | looks | L | R10: Attribution credits & startup splash |
| B006 | controls-a.md #6 | `검수·제출` (A6) | `레이어 5179 변환…`, `제출 꾸러미 만들기…`, 5 `KaSubmitFlow` sub-dialogs | Enabled buttons + 5 sub-dialogs (error confirm, unsaved confirm, `SHP 인코딩`, folder picker, progress) -> disabled with R2 tooltip, 5 sub-dialogs missing | `KaSubmitDialog.cpp:89-97`, `KaSubmitFlow.cpp:88-169` | `crates/strata-app/src/modal_export_footer.rs:23-40` | missing control | 2 | data | L | R2: EPSG:5179 SHP export & submit package engine |
| B007 | controls-a.md #7 | `위치 검색` progress (A9) | `KaShellUi::createSearchProgress` (`위치 검색` + `취소`) | Non-modal progress dialog with `취소` during async search -> dialog missing; search blocks UI synchronously | `KaShellUi.cpp:68-77` | `crates/strata-app/src/gui_ribbon_search.rs:90` | missing window | 2 | confusion | M | — |
| B008 | controls-a.md #8 | `조사 닫기` (A2) | Title, `{name}을(를) 저장할까요?`, count/path body, `Save`/`Discard` | Title `조사 닫기`, `{name}을(를) 저장할까요?`, `저장하지 않은 작업이 {N}건 있습니다.\n{folder} › {fileName}`, `Save`/`Discard` -> `저장하지 않은 편집이 있습니다` / `저장하고 끝내기` / `저장하지 않고 끝내기` (`format_close_prompt` unused) | `KaClosePrompt.cpp:26-46` | `crates/strata-app/src/shell_dialogs.rs:83-138` | text | 1 | data | S | — |
| B009 | controls-a.md #9 | `API 키 입력` (A3) & `VWorld 지적도 계정` (A10) | Two separate dialogs vs `AccountSettings` | Two separate dialogs (`API 키 입력`, `VWorld 지적도 계정`) -> merged with NGII/Heritage into `API 키 · 사이트 계정 설정 - Strata` | `KaApiKeyDialog.cpp:18`, `MainWindowCadastral.cpp:36` | `crates/strata-app/src/modal_paper_account.rs:147-292` | behaviour | 2 | confusion | M | — |
| B010 | controls-a.md #10 | `자료 준비 상태` (A7) | Table rows (`m_table`), `online_cnt`, `받기 / 설정` column | Live `ReferenceInventory::collect`, 6 columns, read-only («이 창은 자료를 새로 받거나 지우지 않습니다») -> 7 hardcoded rows (`online_cnt = 2`) + extra 7th `받기 / 설정` column | `KaReferenceStatusDialog.cpp:39-67,92-111` | `crates/strata-app/src/modal_reference.rs:42-209` | behaviour | 3 | result | M | — |
| B011 | controls-a.md #11 | `검수·제출` (A6) | Tree (`QTreeWidget`), package line, `set_work_crs` action | 3-col collapsible tree with target child rows & double-click jump, package line without `README_submit.txt`, `set_work_crs` sets CRS -> flat 27-rule list, adds `README_submit.txt`, routes `set_work_crs` to `AccountSettings` | `KaSubmitDialog.cpp:54-77,153-226` | `crates/strata-app/src/modal_export_table.rs:28-164` | behaviour | 2 | result | M | — |
| B012 | controls-a.md #12 | `새 조사` (A1) | Origin button text, title, ruler tooltip, `maxLength` | `5186  중부원점`/`5187  동부원점`, title `새 조사`, ruler tooltip, `maxLength(100)` -> adds city subtitles & ` - Strata`, omits ruler tooltip & `maxLength` | `KaNewSurveyDialog.cpp:48,59,67-68`, `KaLongitudeRuler.cpp:42-43` | `crates/strata-app/src/dlg_new_survey.rs:28,33,73-95`, `crates/strata-app/src/dlg_new_survey_ruler.rs:12-126` | text | 2 | looks | S | — |
| B013 | controls-a.md #13 | `지역` address popup (A14) | `동·읍·면` combo & outside-click close | Editable `QComboBox`; closes on `WindowDeactivate` (outside click) -> split `TextEdit` + 24 px `ComboBox`; does not close on outside click | `KaRegionLocator.cpp:116-119,146-160` | `crates/strata-app/src/gui_ribbon_region_b.rs:191-222` | behaviour | 2 | confusion | S | — |
| B014 | controls-a.md #14 | A1, A2, A3, A4, A6, A9 dialogs | Modality & `Enter` default-button handling | Modal `QDialog::exec()` (except A7, A8, A9 progress) with `Enter` activating default button -> non-modal `egui::Window`s without `Enter` handler (except A5) | `KaNewSurveyDialog.cpp:146`, `KaClosePrompt.cpp:51`, `KaApiKeyDialog.cpp:36`, `KaPaperDialog.cpp:180`, `KaSubmitDialog.cpp:92`, `MainWindow.cpp:2807` | `crates/strata-app/src/dlg_new_survey.rs`, `crates/strata-app/src/shell_dialogs.rs`, `crates/strata-app/src/modal_paper_account.rs`, `crates/strata-app/src/modal_export.rs`, `crates/strata-app/src/gui_ribbon_region_b.rs` | state or look | 1 | confusion | M | — |
| B015 | controls-b.md #1 | `지형분석` (TG) | Entire dialog & hillshade flow | Opens `지형분석` dialog (`DEM 파일로 음영 만들기…`) -> opens `DemClass`; dialog & flow missing | `MainWindow.cpp:1715-1739` | `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87` (`MISSING` dialog) | missing window | 3 | confusion | L | — |
| B016 | controls-b.md #2 | `GeoTIFF 저장` (GT) | Progress dialog & export flow | Progress dialog & GeoTIFF export flow -> click is no-op; dialog & flow missing | `MainWindowExport.cpp:78-84` | `crates/strata-app/src/gui_ribbon_click_b.rs:145` (`MISSING` dialog) | missing window | 2 | result | L | R7: GeoTIFF raster exporter |
| B017 | controls-b.md #3 | `<title> 내려받기` (DP2) | `토양도`, `지질도`, `수계도`, `오프라인 지도` progress windows | Download progress dialogs & workers -> ribbon clicks only toggle visibility; progress dialogs missing | `MainWindowDownloads.cpp:22-33`, `MainWindowOverlay.cpp:612` | `crates/strata-app/src/gui_ribbon_click_b.rs:38-85` (`MISSING` dialogs) | missing window | 2 | confusion | L | R7: Thematic WFS & offline-map download workers |
| B018 | controls-b.md #4 | `측량 등고선` (SC) | Non-modal `contourProgress` `QProgressDialog` | Shows non-modal `contourProgress` during contour generation -> missing | `MainWindowSurveyContour.cpp:132-137` | `MISSING` (`crates/strata-app/src/dlg_survey_contour.rs`) | missing window | 3 | confusion | M | — |
| B019 | controls-b.md #5 | `측량 등고선` (SC) | `찾기…` button & preview label | `찾기…` picks point file & computes real count/elevation -> dead button; preview fabricates `점 248개, 표고 24.5–48.2 m` | `KaSurveyContourDialog.cpp:31,142-147,227-230` | `crates/strata-app/src/dlg_survey_contour.rs:43,154` | behaviour | 3 | result | L | — |
| B020 | controls-b.md #6 | `주변유적 받기` (HB) | `받기` button & embedded `m_tabs` webview | Embedded webview (`m_tabs`) + active `받기` trigger -> `받기` permanently disabled (`DEFERRED_WB1`), webview missing | `KaHeritageBrowser.cpp:252-257` | `crates/strata-app/src/dlg_heritage_browser_footer.rs:66-78`, `crates/strata-app/src/dlg_heritage_browser.rs:23` | missing control | 2 | confusion | L | DEFERRED_WB1: Embedded WebView & 국가유산청 portal session |
| B021 | controls-b.md #7 | `수치지형도 내려받기` (TB) | `도엽 확인` button, webview, `m_downloads` table | Embedded NGII webview + live `도엽 확인` + real queue -> `도엽 확인` disabled (`DEFERRED_WB2`), webview missing, `m_downloads` shows 3 hardcoded fake rows | `KaTopographicBrowser.cpp:172-200` | `crates/strata-app/src/ngii_topographic_state.rs:30-31`, `crates/strata-app/src/dlg_topo_browser_body.rs:123-140`, `crates/strata-app/src/dlg_topographic_browser.rs:33` | missing control | 2 | result | L | DEFERRED_WB2: Embedded WebView & 국토지리정보원 portal session |
| B022 | controls-b.md #8 | `국가유산 계정` (HA) & `수치지형 계정` (TA) | Separate dialogs, HA description, Cancel text | Separate `KaHeritageAccountDialog` (with description) & `KaTopographicAccountDialog` with `취소` -> merged into `AccountSettings`, lacks HA description, uses `닫기` | `KaHeritageSetupDialogs.cpp:32-34,62` | `crates/strata-app/src/modal_paper_account.rs:252-267` | text | 2 | confusion | M | — |
| B023 | controls-b.md #9 | `주변유적 받을 시·군 선택` (HR) | `addNearby`, `m_reuseWhen`, tooltip, `ka_modal_heritage_bg` | 5 km neighbouring 시·군 group, `m_reuseWhen` timestamp, `m_reuse` tooltip -> missing; decorative `ka_modal_heritage_bg` window has dead `상세 보기`/`취소` | `KaHeritageSetupDialogs.cpp:124-131,174-228` | `crates/strata-app/src/dlg_heritage_region_select.rs:13-70,140-142` | missing control | 2 | confusion | M | — |
| B024 | controls-b.md #10 | `수치지형도 — 좌표계 확인과 지도 추가` (TI) | `QgsMapCanvas` preview, stub actions, cell tooltips | Real `QgsMapCanvas` preview, `선택 도엽 정합 미리보기`/`지도에 추가` actions, cell tooltips -> schematic box, stub buttons, tooltips missing | `KaTopographicImportDialog.cpp:304-306,471-474,498-522` | `crates/strata-app/src/dlg_topo_import_preview.rs:36-105`, `crates/strata-app/src/dlg_topo_import_actions.rs:32-97` | behaviour | 2 | result | L | — |
| B025 | controls-b.md #11 | `DEM 표현` (DC) | DEM guard, range source, tooltip, buttons, EXTRA rows | Requires DEM layer, real range, range tooltip, `[적용][닫기]` -> opens without DEM, fake `32.4 m ~ 74.8 m`, `[적용][확인][취소]`, EXTRA `DEM 모드` + `색 띠 미리보기` | `KaDemClassDialog.cpp:38-39,66-71` | `crates/strata-app/src/modal_style_demclass.rs:44-163,200-254` | behaviour | 2 | result | M | — |
| B026 | controls-b.md #12 | `측량 등고선` (SC) | `의심점 빼기`/`단절선` enabled rule, `파일 좌표계`, 5 tooltips | Two checkboxes disabled until detected, `파일 좌표계` ComboBox, 5 tooltips -> always enabled, cycling button, 5 tooltips missing | `KaSurveyContourDialog.cpp:45-100,190-199` | `crates/strata-app/src/dlg_survey_contour.rs:63-139` | enabled rule | 3 | confusion | S | — |
| B027 | controls-b.md #13 | `지적도 다운로드` (DP1) | `취소` button, hint label, 3 extra buttons | `취소` enabled & closes dialog; shows `다운로드 중에도 지도 작업을 계속할 수 있습니다.` hint -> `취소` disabled when idle & stays open, hint missing, 3 extra buttons | `KaDownloadUi.cpp:51-55` | `crates/strata-app/src/cadastral_download_ui_render.rs:115-156` | enabled rule | 2 | confusion | S | — |
| B028 | controls-b.md #14 | HB, HR, TB, TI, DP1, DP2, SC, DC (12 windows) | Keyboard `Enter` / `Esc` across all 12 windows | `QDialog` handles `Key::Enter` (default) and `Key::Escape` (reject) -> none of the Rust dialogs handle `Enter` or `Esc` | `KaHeritageBrowser.cpp:252`, `KaHeritageSetupDialogs.cpp:215`, `KaTopographicBrowser.cpp:172`, `KaTopographicImportDialog.cpp:498`, `KaDownloadUi.cpp:51`, `KaSurveyContourDialog.cpp:95`, `KaDemClassDialog.cpp:66` | `crates/strata-app/src/dlg_heritage_browser_footer.rs`, `crates/strata-app/src/dlg_heritage_region_select.rs`, `crates/strata-app/src/dlg_topographic_browser.rs`, `crates/strata-app/src/dlg_topo_import_actions.rs`, `crates/strata-app/src/cadastral_download_ui_render.rs`, `crates/strata-app/src/dem_download_ui_render.rs`, `crates/strata-app/src/dlg_survey_contour.rs`, `crates/strata-app/src/modal_style_demclass.rs` | behaviour | 2 | confusion | M | — |
| B029 | controls-b.md #15 | HB, HR, TB, TI, DP1, DP2, SC, DC (12 windows) | Window resizability | All 12 C++ `QDialog`s are resizable -> Rust modal windows use `.resizable(false)` (and fixed sizes on HB, TI, SC, DC) | `KaHeritageBrowser.cpp`, `KaHeritageSetupDialogs.cpp`, `KaTopographicBrowser.cpp`, `KaTopographicImportDialog.cpp`, `KaDownloadUi.cpp`, `KaSurveyContourDialog.cpp`, `KaDemClassDialog.cpp` | `crates/strata-app/src/dlg_heritage_browser.rs`, `crates/strata-app/src/dlg_heritage_region_select.rs`, `crates/strata-app/src/dlg_topographic_browser.rs`, `crates/strata-app/src/dlg_topographic_import.rs`, `crates/strata-app/src/cadastral_download_ui_render.rs`, `crates/strata-app/src/dem_download_ui_render.rs`, `crates/strata-app/src/dlg_survey_contour.rs`, `crates/strata-app/src/modal_style_demclass.rs` | state or look | 2 | looks | S | — |
| B030 | controls-c.md #1 | `기준점 미리보기` (C2) | Entire dialog & `더보기 -> CSV 기준점` action | `CSV 기준점` opens `기준점 미리보기` -> opens `AccountSettings`; dialog missing | `MainWindow.cpp:2557-2563` | `crates/strata-app/src/gui_ribbon_menus_b.rs:114` (`MISSING` dialog) | missing window | 3 | confusion | L | — |
| B031 | controls-c.md #2 | `기준점 축` (C3) | Axis-swap confirmation dialog | Post-add axis-swap confirmation dialog (`기준점 축`) -> entire dialog missing | `MainWindowEditing.cpp:1416-1427` | `crates/strata-app/src/modal_control.rs:248-260` (`MISSING` dialog) | missing window | 3 | data | M | — |
| B032 | controls-c.md #3 | `도형 색` (C8) | Polygon/line/point layer style dialog | Right-click `면·외곽선 색` opens `도형 색` -> opens `선 색`; dialog missing | `MainWindowEditing.cpp:467-610` | `crates/strata-app/src/gui_layer_panel_context_menu.rs:64` (`MISSING` dialog) | missing window | 2 | confusion | L | — |
| B033 | controls-c.md #4 | `겹치는 구간 분리 (클립)` (C10) | Entire dialog & ribbon `구간 분리` button | Ribbon `구간 분리` opens `겹치는 구간 분리 (클립)` -> button disabled; dialog missing | `MainWindowEditing.cpp:1306-1348` | `crates/strata-app/src/gui_ribbon_draw_row.rs:205,210` (`MISSING` dialog) | missing window | 2 | confusion | L | — |
| B034 | controls-c.md #5, controls-e.md #18 | `격자 원점` (C11) / Map overlays | `원점` button (`KaGridOriginButton`) & `격자 원점` modal | `원점` button opens `격자 원점` modal dialog -> button & dialog missing | `KaGridOriginButton.cpp:35-112`, `MainWindow.cpp:1027` | `crates/strata-app/src/gui_tab_map_canvas_overlay.rs:40`, `crates/strata-app/src/gui_tab_map_gridctl.rs:1-155` | missing window | 2 | confusion | M | — |
| B035 | controls-c.md #6 | `선 색` / Colour pickers (C13) | `QColorDialog` color/alpha picker | `클릭해서 색 고르기` opens full color/alpha picker (`QColorDialog`) -> cycles 8 palette colors only | `MainWindowEditing.cpp:89-97`, `KaLineColorDialog.cpp:66-68` | `crates/strata-app/src/modal_style.rs:77-82` | missing window | 2 | looks | M | — |
| B036 | controls-c.md #7 | `시굴격자 속성` (C6) | Trial-trench confirm/prompt message boxes | `confirmReplaceAdjusted` and multi-survey-area prompt boxes -> both missing | `KaTrenchDialogPlan.cpp:200-213`, `MainWindow.cpp:2065-2081` | `MISSING` (`crates/strata-app/src/dlg_trench_generator.rs`) | missing message | 2 | data | S | — |
| B037 | controls-c.md #8 | Map editing boxes (C14) | 11 `MainWindowEditing.cpp` confirm/info boxes | 11 `QMessageBox` prompts (`그린 도형 삭제` `:1491`, `지금 「새 조사」를 만들까요?` `:1090`, etc.) -> 10 missing, 11th (`:689`) status-bar-only | `MainWindowEditing.cpp:339-1496` | `crates/strata-app/src/app_state_feature_record.rs:23-25` (`MISSING` 10 others) | missing message | 1 | data | M | — |
| B038 | controls-c.md #9 | `선 색` (C7) | Non-line-map guard, EXTRA swatches/widths, reset rule | Refuses open with `KaUserError::warn` on non-line layers; no swatches/quick widths; `"원래 색으로"` conditionally enabled -> inline warn, EXTRA rows, reset always enabled | `KaLineColorDialog.cpp:47-53` | `crates/strata-app/src/modal_style.rs:43-52,85-144` | behaviour | 2 | confusion | S | — |
| B039 | controls-c.md #10 | `시굴격자 속성` (C5) | Title, widgets, `m_size`, terrain info, `구역에 깔기` | Title `"시굴격자 속성"`, `ComboBox`es, `"규격"` (`20m`/`10m`), dynamic terrain status, live plan & grid gen -> ` - Strata`, cycling buttons, `"트렌치 길이"` 3 radios, hardcoded `135° · 4.2%`, `구역에 깔기` no-op | `KaTrenchDialog.cpp:27-70`, `KaTrenchDialogPlan.cpp:26-71` | `crates/strata-app/src/dlg_trench_generator.rs:32,56-72`, `crates/strata-app/src/dlg_trench_generator_controls.rs:27-99` | behaviour | 2 | result | L | — |
| B040 | controls-c.md #11 | `GPS 기준점` (C1) | Hint CRS, mutable X/Y labels, combos, OK text, Enter, error box | Dynamic axis hint, fixed X/Y labels, `QLineEdit` for `원점`/`수신상태`, `"확인"`, `Enter` submits, modal error -> hardcoded 5187 hint, X/Y mutate, ComboBoxes, `"지도에 추가 (확인)"`, no Enter, status-bar error | `KaControlPointDialog.cpp:73-77,105-124` | `crates/strata-app/src/modal_control.rs:32-245` | behaviour | 2 | data | M | — |
| B041 | controls-c.md #12 | `이름·번호` (C4) | Width `440`, `kind`/`period` completer, error box | `minWidth` `440`, typing completer on `kind`/`period`, modal `KaUserError::warn` on save error -> `380`, no completer, status-bar error | `KaFeatureFormDialog.cpp:71`, `KaWindowSize.h:15` | `crates/strata-app/src/dlg_feature_form.rs:77`, `crates/strata-app/src/app_state_feature_form.rs:38-40` | behaviour | 2 | confusion | S | — |
| B042 | controls-c.md #13 | `도형 속성 — <layer>` (C9) | Layer menu opener, completer, modal error | Context-menu `"도형 속성"` enabled, typing completer, modal error warning -> opener disabled, no completer, status-bar error | `MainWindowEditing.cpp:332,406,643-776` | `crates/strata-app/src/layer_menu_model.rs:187`, `crates/strata-app/src/app_state_feature_record.rs:41-52`, `crates/strata-app/src/dlg_feature_record.rs` | behaviour | 2 | confusion | M | — |
| B043 | controls-d.md #1 | Drawing Studio (`C12 격자 설정`) | `좌표 격자` button & dialog | Opens `격자 설정` modal dialog (5 controls) -> button toggles `grid_follow_map` directly; dialog missing | `KaDrawingStudio.cpp:1460-1518` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:106-124` (`MISSING` dialog) | missing window | 2 | confusion | M | — |
| B044 | controls-d.md #2 | Drawing Studio (`CTX-M`) | Map box context menu (`KaLayoutMenuProvider`) | 5 actions (`마지막 점 지우기`, `점찍기 끝`, `지도 조정`, `그린 것 가운데로`, `지도조정끝`) -> entire menu missing | `KaDrawingStudio.cpp:315-355` | `MISSING` (`crates/strata-app/src/gui_tab_drawing*.rs`) | missing control | 2 | confusion | M | — |
| B045 | controls-d.md #3 | Drawing Studio (`D1`) | `PDF 내보내기` button | Exports drawing sheet to PDF -> disabled with `아직 Rust 판에서 연결하지 않았습니다 (지시 R3)` | `KaDrawingStudio.cpp:929-932` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:142-144` | enabled rule | 2 | result | L | R3: Drawing PDF exporter |
| B046 | controls-d.md #4 | Drawing Print (`D3`) | `나눈 장을 PDF로 저장…` button | Saves divided print plates to PDF -> disabled with `아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)` | `KaPrintDialog.cpp:204-206,460` | `crates/strata-app/src/dlg_drawing_print_actions.rs:37-45` | enabled rule | 3 | result | L | W-50: OS printing & multi-page PDF exporter |
| B047 | controls-d.md #5 | Drawing Print (`D3`) | `프린터` combo & `프린터 속성…` button | Real OS printers & `프린터 속성…` action -> hardcoded `"ezViewX Virtual Image Printer              ▼"` and dead button | `KaPrintDialog.cpp:88-107,237` | `crates/strata-app/src/dlg_drawing_print_options.rs:22-38` | behaviour | 3 | result | L | W-50: OS printing & multi-page PDF exporter |
| B048 | controls-d.md #6 | Drawing Print (`D3`) | `미리보기` (`KaTilePreview`) & `모두 다시 넣기` | Interactive multi-sheet grid preview, `m_pickHint`, `m_pickAll` (`모두 다시 넣기`), `m_warning`, `m_skipNote`, `m_summary` -> static dummy with `"축척 1 : 6532"`, missing pick/notes | `KaPrintDialog.cpp:177-200`, `KaPrintDialogPlate.cpp:130-160` | `crates/strata-app/src/dlg_drawing_print_preview.rs:17-124` | behaviour | 3 | result | L | W-50: OS printing & multi-page PDF exporter |
| B049 | controls-d.md #7 | Drawing Print (`D3`) | `붙였을 때 크기` combo (`m_output`) | Scale/paper-fit combo box -> entire control missing | `KaPrintDialog.cpp:122-125,255-285` | `MISSING` (`crates/strata-app/src/dlg_drawing_print_options.rs`) | missing control | 3 | result | M | W-50: OS printing & multi-page PDF exporter |
| B050 | controls-d.md #8 | Drawing Print (`D3`) | `인쇄` button & sub-dialogs | Opens `QPrintDialog`, `QProgressDialog` (`멈추기`), result/error `QMessageBox`es -> only sets `draw_guide_message` | `KaPrintDialog.cpp:575-668` | `crates/strata-app/src/dlg_drawing_print_actions.rs:65-83` | behaviour | 3 | result | L | W-50: OS printing & multi-page PDF exporter |
| B051 | controls-d.md #9 | PDF Progress (`D4`) | `KaDrawingPdfProgress` | Real page size/DPI, live progress, `취소` cancels export -> hardcoded `"210x297 mm · 300 DPI"`, static `160.0 px` bar, `취소` only closes modal | `KaDrawingPdfProgress.cpp:15-58` | `crates/strata-app/src/dlg_pdf_preparing.rs:21-55` | behaviour | 2 | confusion | M | R3: Drawing PDF exporter |
| B052 | controls-d.md #10 | Drawing Studio (`D2`) | 14 message & input boxes | Modal `QMessageBox`/`KaUserError::warn`/`QInputDialog` prompts (`표제란`, `도면 보관`, `도면 열기`, `도면 목록`, `레이어 삭제`, `PDF`, `인쇄`) -> status bar only | `KaDrawingStudio.cpp:3294-3887`, `KaDrawingStudioSheets.cpp:47-164` | `crates/strata-app/src/gui_tab_drawing_popups.rs:200-226` | missing message | 2 | data | M | — |
| B053 | controls-d.md #11 | Drawing Studio (`D1`) | `도면 목록` popup | `열기:` prefix, `보관한 도면 지우기` submenu, `제출 PDF는 지금 열린 도면으로 만듭니다` footer -> all 3 missing | `KaDrawingSheetSet.cpp:135-161` | `crates/strata-app/src/gui_tab_drawing_popups.rs:123-226` | missing control | 2 | confusion | M | — |
| B054 | controls-d.md #12 | Drawing Studio (`D1`) | `세트` button | Opens `KaDrawingLayerSets` save/apply/delete menu -> cycles 4 hardcoded presets in status bar | `KaDrawingStudio.cpp:696-709`, `KaDrawingLayerSets.cpp:63-104` | `crates/strata-app/src/gui_layer_panel_tree.rs:101-120` | behaviour | 2 | confusion | M | — |
| B055 | controls-d.md #13 | Drawing Studio (`D1`) | `축척 글자` & `좌표계` buttons | Adds scale-text or CRS-note decoration item on sheet -> no-op on click | `KaDrawingStudio.cpp:1160-1167` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:98-107` | behaviour | 2 | confusion | M | — |
| B056 | controls-d.md #14 | Drawing Studio (`D1`) | `선택 항목 크기` spin box | Disabled when no decoration is selected -> always enabled | `KaDrawingStudio.cpp:1151,3077` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:86-94` | enabled rule | 2 | confusion | S | — |
| B057 | controls-d.md #15 | Drawing Studio (`D1`) | `지도 조정` top banner | Includes `레이어를 가운데` button -> button missing | `KaDrawingStudio.cpp:835` | `crates/strata-app/src/gui_tab_drawing_popups.rs:19-59` | missing control | 2 | confusion | S | — |
| B058 | controls-d.md #16 | Drawing Studio (`D1`) | `범례` & `표제란` defaults & behavior | `표제란` defaults `false` (optional), prompts title input, shows tooltips -> defaults `true`, no title prompt, no tooltips | `KaDrawingStudio.cpp:908-946`, `KaDrawingStudioSheets.cpp:47` | `crates/strata-app/src/gui_tab_drawing_right.rs:31-65`, `crates/strata-layout/src/sheet_drawing.rs:82-83` | default or range | 2 | result | S | — |
| B059 | controls-d.md #17 | Drawing Studio (`D1`) | `도면 정보` default scale & `적용` button | Default scale `1000`, `적용` applies scale, 14 scale chips checkable -> default `5000`, `적용` only sets status text, chips not checkable | `KaDrawingStudio.cpp:1064,1093` | `crates/strata-layout/src/sheet_drawing.rs:72`, `crates/strata-app/src/gui_tab_drawing_right.rs:160-165`, `crates/strata-app/src/gui_tab_drawing_right_b.rs:15-47` | behaviour | 2 | result | M | — |
| B060 | controls-d.md #18 | Drawing Studio (`D1`) | `좌표 격자` default | Defaults `false` -> defaults `true` | `KaDrawingStudio.cpp:1553` | `crates/strata-layout/src/sheet_drawing.rs:73` | default or range | 2 | result | S | — |
| B061 | controls-d.md #19 | Drawing Studio (`D1`) | `범례 제목` line edit placeholder | Placeholder `"제목을 입력하세요"` -> missing placeholder | `KaDrawingStudio.cpp:953` | `crates/strata-app/src/gui_tab_drawing_right.rs:69-72` | tooltip | 2 | looks | S | — |
| B062 | controls-d.md #20 | Drawing Studio (`D1`) | `방위 (도북)` & `전체 켜기/끄기` tooltips | Sets tooltips on `방위 (도북)` & `전체 켜기/끄기` -> missing tooltips | `KaDrawingStudio.cpp:691,1000,1015` | `crates/strata-app/src/gui_layer_panel_tree.rs:88-96`, `crates/strata-app/src/gui_tab_drawing_right.rs:95-132` | tooltip | 2 | looks | S | — |
| B063 | controls-d.md #21 | Drawing Studio (`D1`) | `Delete` / `Backspace` key | Deletes selected drawing items -> does not delete selected drawing items | `KaDrawingStudio.cpp:3561-3565` | `crates/strata-app/src/gui_shell.rs:84-88` | behaviour | 2 | confusion | S | — |
| B064 | controls-d.md #22 | Drawing Print (`D3`) | `방법` default, `600 DPI` text/tip, ledger rows | `print_single_sheet` follows `probe.actualSize`; `600 DPI (가는 선이 많은 실측도)` + tip; `배열` row + `+ 안내 1장` -> hardcoded `true`, `600 DPI (고정밀 제출용)` + no tip, missing `배열`, hardcoded `"실제 크기 100%"`, missing `+ 안내 1장` | `KaPrintDialog.cpp:155,158-160,223,524`, `KaPrintDialogPlate.cpp:178,207` | `crates/strata-app/src/dlg_drawing_print_options.rs:107,171-191`, `crates/strata-app/src/dlg_drawing_print_actions.rs:58`, `crates/strata-app/src/modal_controller_state.rs:176` | text | 3 | confusion | S | — |
| B065 | controls-d.md #23 | Drawing Studio (`D1`) | EXTRA grid sync & opacity pill | No inline sync checkbox or static opacity pill -> inline `지도 좌표격자와 같게 (20 m)` checkbox and static `"투명도·밝기: 조절할 레이어를 고르세요"` pill | none in `KaDrawingStudio.cpp` | `crates/strata-app/src/gui_tab_drawing_right_b.rs:127-130`, `crates/strata-app/src/gui_tab_drawing_sheet.rs:19-36` | extra in Rust | 2 | confusion | S | — |
| B066 | controls-e.md #1 | Home screen | `복구 사본` banner links & table context menu | Banner links (`사본 열기 · 폴더 열기 · 숨기기`) & recent context menu (`복구 사본 열기`, `복구 사본 폴더 열기`, `복구 사본 표시 숨기기`) -> missing | `KaStartPage.cpp:301-303,336-344`, `KaHomeRecentCard.cpp:225-236` | `crates/strata-app/src/gui_tab_home_hero.rs:145-242`, `crates/strata-app/src/gui_tab_home_recent_box.rs:146-151` | missing control | 1 | data | M | Unsaved recovery snapshot writer (복구사본) |
| B067 | controls-e.md #2 | Home screen | `이어서 작업` badge & `상태` column | Reflects dirty / recovery snapshot (`● 저장 안 됨`) -> hardcodes `"저장됨"` on Continue card | `KaHomeTitleBlock.cpp:142`, `KaStartPage.cpp:285` | `crates/strata-app/src/gui_tab_home_hero.rs:155`, `crates/strata-app/src/gui_tab_home_recent_box.rs:29,43,128` | state or look | 1 | confusion | S | — |
| B068 | controls-e.md #3 | Home screen | `작업 순서` (`KaHomeGuideCard`) | Dynamic `SurveyFacts::summaryLine` and `제출 준비:` / `오류 %1 · 주의 %2` chips -> hardcoded steps `1/2/3`, clickable links (`NewSurvey`/`DrawPolygon`/`PaperSetup`), fixed step-3 text | `KaHomeGuideCard.cpp:95-98,138-185` | `crates/strata-app/src/gui_tab_home_recent.rs:72-76,133-151` | behaviour | 1 | result | M | — |
| B069 | controls-e.md #4 | Home & Left panel | `m_filter` / `m_edit` filter inputs | `clearButtonEnabled(true)`, normal placeholder alignment -> no clear `(×)` button, adds 5 leading spaces | `KaHomeRecentCard.cpp:62-63`, `KaLayerListChrome.cpp:31-32` | `crates/strata-app/src/gui_tab_home_recent.rs:36`, `crates/strata-app/src/gui_layer_panel_tree.rs:130` | missing control | 1 | looks | S | — |
| B070 | controls-e.md #5 | Tab bar | `KaSurveyBadge` | Clickable / Tab-focusable badge calling `MainWindow::showMapWorkspace` -> non-clickable `ui.label` | `KaSurveyBadge.cpp:133,142`, `MainWindowChrome.cpp:155` | `crates/strata-app/src/gui_ribbon_tabbar.rs:272-277` | behaviour | 1 | confusion | S | — |
| B071 | controls-e.md #6 | Tab bar | `테마 ▼` menu & tooltip | Tooltip `"화면 색을 고릅니다..."`; color-theme popup separate from display options -> `"화면 테마·모양·글자 크기 설정"`, mixes `새 모양`/`고대비 화면`/`큰 글씨`/sound into popup | `KaThemeOptions.cpp:161,168-181` | `crates/strata-app/src/gui_ribbon_tabbar.rs:171-217,246` | tooltip | 1 | confusion | S | — |
| B072 | controls-e.md #7 | Tab bar | Dynamic closable tabs & `×` tooltip | Has `입체지형 도면` dynamic tab & `"탭 닫기"` tooltip on `×` (no `고지형` tab) -> adds EXTRA `고지형` tab, lacks `입체지형 도면` tab & `"탭 닫기"` tooltip | `KaTabClose.cpp:45`, `MainWindowExport.cpp:373` | `crates/strata-app/src/gui_ribbon_tabbar.rs:81-82,110` | extra in Rust | 1 | confusion | S | — |
| B073 | controls-e.md #8 | Status bar | `snapChip` (`자석 끔` / `자석 켬`) | Clickable snap status chip -> missing | `KaStatusBar.cpp:48-51,147` | `crates/strata-app/src/shell_workspace.rs:238-244` | missing control | 1 | confusion | M | — |
| B074 | controls-e.md #9 | Status bar | `crsButton` (`작업 5187 · 동부원점 ▼`) | Opens 2-item CRS menu + disabled `EPSG:5179` note, has tooltip -> toggles directly on click with no tooltip | `KaStatusBar.cpp:108,195`, `MainWindow.cpp:700-715` | `crates/strata-app/src/shell_workspace.rs:160-174` | behaviour | 1 | data | M | — |
| B075 | controls-e.md #10 | Status bar | `scaleCombo` & `→ 제출 5179` | Inline editable `QComboBox`, tooltip, visible `onMap \|\| onDrawing`; full `→ 제출 5179` tooltip -> popup button with `예: 1000` inside, visible on all non-Home tabs, truncated tooltip | `KaStatusBar.cpp:77-98,204-205`, `MainWindow.cpp:1091-1092` | `crates/strata-app/src/shell_workspace.rs:155,176-228` | behaviour | 1 | result | M | — |
| B076 | controls-e.md #11 | Status bar | Idle message on `홈` tab | Dynamic hint when a survey is open -> hardcodes `"먼저 「새 조사」로 오늘 현장을 만드세요."` unconditionally on Home | `MainWindow.cpp:395-412` | `crates/strata-app/src/shell_workspace.rs:42-51` | text | 1 | confusion | S | — |
| B077 | controls-e.md #12 | Left layer panel | `파일함` toggle (`filesToggle`) | Hides/shows file drawer panel -> sets `files_open` while `paint_file_drawer` runs unconditionally | `MainWindow.cpp:935-938` | `crates/strata-app/src/gui_layer_panel_tree.rs:70,79-85` | behaviour | 1 | confusion | S | — |
| B078 | controls-e.md #13 | Left layer panel | `표시 설정` (`KaLayerInformationPanel`) | `m_title`, `글자 표시` checkbox, `m_reason`, `ComboBox`es for `내용`/`글자 색`, float size `1.0..72.0` -> missing 3 parts, cycling buttons, integer `6..=24` | `KaLayerInformationPanel.cpp:66-91,143-165` | `crates/strata-app/src/gui_layer_panel_card.rs:20-88` | state or look | 1 | result | M | — |
| B079 | controls-e.md #14 | Left layer panel | Header, tree & context-menu gaps | Tooltip on `전체 끄기`/`m_pencil`, drag-drop reorder, `"「%1」에 맞는 레이어가 없습니다."` hint, 8 menu actions, no `세트 ▼` -> missing tips/drag-drop/hint, EXTRA `세트 ▼`, 8 actions disabled | `MainWindow.cpp:839-840`, `KaLayerListChrome.cpp:50,53,126`, `MainWindowContextMenus.cpp:265-439` | `crates/strata-app/src/gui_layer_panel_tree.rs:88-120`, `crates/strata-app/src/gui_layer_panel_rows.rs:39-45`, `crates/strata-app/src/layer_menu_model.rs:182-248` | behaviour | 1 | confusion | L | — |
| B080 | controls-e.md #15 | Left layer panel | `파일함` (`KaFileBrowserPanel`) | `내 PC` lists Windows drives, `상위` disabled at root -> `내 PC` goes to user home dir, `상위` always enabled | `KaFileBrowserPanel.cpp:94,122` | `crates/strata-app/src/gui_layer_panel_card.rs:117,160-171` | behaviour | 2 | confusion | S | — |
| B081 | controls-e.md #16 | Left & Right panels | Panel splitter widths | Resizable `QSplitter` panes (`348` and `272`/`300`) -> fixed widths (`351.0` and `236.0`) | `MainWindow.cpp:1031-1039`, `KaInspectorPanel.cpp:30` | `crates/strata-app/src/gui_layer_panel_consts.rs:6`, `crates/strata-app/src/gui_tab_map_consts.rs:7`, `crates/strata-app/src/right_inspector.rs:249` | state or look | 1 | looks | M | — |
| B082 | controls-e.md #17 | Right panel | Collapse tip, `스타일` warning, `배경 지도` card | Collapse tip `"오른쪽 패널 접기 (F10)"`, `스타일` warning `"지금은 종류·시대별 색입니다..."`, `배경 지도` note/tooltip/synced buttons -> `"패널 접기 (F10)"`, missing warnings & tooltips/sync | `KaInspectorPanel.cpp:56`, `KaInspectorStyle.cpp:160`, `KaBasemapQuickCard.cpp:24,52-63` | `crates/strata-app/src/ri_body.rs:26`, `crates/strata-app/src/ri_layer_card.rs:185-248`, `crates/strata-app/src/ri_body_bottom.rs:13-41` | state or look | 1 | confusion | M | — |
| B083 | controls-e.md #19 | Map overlays | `KaLayerOpacityRail` & `Undo`/`Redo` pill | Draggable opacity rail + tooltip; short Undo/Redo tooltips & disabled when stack empty -> fixed rail without tip, long custom tips, Undo/Redo always clickable | `KaLayerOpacityRail.cpp:100`, `KaDrawGuideBand.cpp:61-62,89` | `crates/strata-app/src/gui_tab_map_opacity.rs:19-29,111-130`, `crates/strata-app/src/gui_tab_map_undo.rs:120-142` | enabled rule | 1 | confusion | M | — |
| B084 | controls-e.md #20 | Map overlays | `KaMapControls`, `KaMapScaleBar`, `KaShellGridControls` | `32x32` `"□"` button, rotated graphic north arrow + `"도북"`, `QComboBox` for `m_kind`/`m_dash`, `QColorDialog` on `…` -> `36x36` fit icon, `"▲ 도북"` literal, cycling labels, 3 presets | `KaMapControls.cpp:58,78-83,216,238`, `KaShellGridControls.cpp:60,73-77,107-109` | `crates/strata-app/src/gui_tab_map_overlay.rs:36,43,85`, `crates/strata-app/src/gui_tab_map_gridctl.rs:50-58,91-97,120` | state or look | 1 | looks | M | — |
| B085 | controls-f.md #1 | `Strata · 보고서` (D5) | Entire studio tab & `Ctrl+G` action | Full `KaHwpReportStudio` tab (26 controls) & `Ctrl+G` -> missing | `KaHwpReportStudio.cpp:37-274`, `KaHwpReportStudioActions.cpp:242-270` | not found in `crates/strata-app/src` (`MISSING`) | missing window | 3 | confusion | L | R8: HWP/HWPX report studio |
| B086 | controls-f.md #2 | HWPX/HWP preview (D6) | `KaHwpLivePaperWindow` A4 preview window | Live A4 HWPX/HWP preview window (11 controls) -> missing | `KaHwpLivePaperWindow.cpp:5-80` | not found in `crates/strata-app/src` (`MISSING`) | missing window | 3 | confusion | L | R8: HWP/HWPX report studio |
| B087 | controls-f.md #3 | `입체지형 도면` (D9) | Entire tab & `placeTerrain3dOnSheet()` | `KaTerrain3dLayoutStudio` tab (15 controls) & `placeTerrain3dOnSheet()` -> missing | `KaTerrain3dLayoutStudio.cpp:72-496`, `MainWindowExport.cpp:318-378` | not found in `crates/strata-app/src` (`MISSING`) | missing window | 3 | confusion | L | R8: 3D terrain engine & studio |
| B088 | controls-f.md #4 | `입체지형` (D8) | `화면을 입체로` & 3D viewport | Interactive 3D terrain viewport & `화면을 입체로` -> button disabled (`지시 W-56`), viewport blank white rect | `KaTerrain3dStudio.cpp:102,127` | `crates/strata-app/src/gui_tab_terrain3d.rs:49-57`, `crates/strata-app/src/gui_tab_terrain3d_view.rs:12-20` | behaviour | 3 | confusion | L | R8: 3D terrain engine & studio |
| B089 | controls-f.md #5 | `입체지형` (D8) | `strata-3d로 열기` & `strata-3d 도면 받기` | External `strata-3d` launcher & sheet importer buttons -> both buttons missing | `KaStrata3dLink.cpp:17-28` | `MISSING` (`crates/strata-app/src/gui_tab_terrain3d.rs`) | missing control | 3 | confusion | M | R8: 3D terrain engine & studio |
| B090 | controls-f.md #6 | Generic error/notify (D12) | `KaUserError` & `MainWindow::notify` modal boxes | Modal `KaUserError` & `MainWindow::notify` box renderer -> struct `feature_record.rs:133` only, no modal renderer | `KaUserError.cpp:65-101`, `MainWindow.cpp:440-445` | `crates/strata-app/src/feature_record.rs:133` (`MISSING` renderer) | missing message | 1 | data | M | — |
| B091 | controls-f.md #7 | Session confirm boxes (D13) | 7 `QMessageBox::question` session prompts | 7 session save/overwrite/exit/reset confirm modals -> all 7 missing | `MainWindowSession.cpp:671,804,1059,1090,1187`, `MainWindowOffline.cpp:251,302` | not found in `crates/strata-app/src` (`MISSING`) | missing message | 1 | data | M | — |
| B092 | controls-f.md #8 | Report boxes (D7) | 6 report & assessment message boxes | 6 report/assessment confirm & info boxes -> all 6 missing | `KaHwpReportStudioActions.cpp:185,197,207`, `KaHwpAssessmentActions.cpp:44,52,65` | not found in `crates/strata-app/src` (`MISSING`) | missing message | 3 | confusion | S | R8: HWP/HWPX report studio |
| B093 | controls-f.md #9 | 3D message boxes (D10) | 7 3D terrain message boxes | 7 modal boxes in `KaTerrain3d*` -> 5 missing, 2 replaced by status bar text | `KaTerrain3dStudio.cpp:466,477,543`, `KaTerrain3dLayoutStudio.cpp:385,478,491`, `MainWindowExport.cpp:323` | `crates/strata-app/src/gui_tab_terrain3d.rs:64,74` (`MISSING` 5 others) | missing message | 3 | confusion | S | R8: 3D terrain engine & studio |
| B094 | controls-f.md #10 | `입체지형` (D8) | `이미지 저장`, `입체지형 도면출력`, `m_status` | `QFileDialog`, `QMessageBox::information`, live DEM status -> only status messages (`NO_SCENE_MESSAGE` / `DEFERRED_W56`) | `KaTerrain3dStudio.cpp:120,464,541` | `crates/strata-app/src/gui_tab_terrain3d.rs:59-75,96-100` | behaviour | 3 | confusion | M | R8: 3D terrain engine & studio |
| B095 | UI_BUTTONS.md #1 | Ribbon (`기록`) | `선택`, `측거`, `그리기`, `시굴격자`, `버퍼` enabled state | Greyed out and non-clickable until survey ready + tooltip reason `먼저 「새 조사」를 만들거나 「열기」로 조사를 여세요.` -> muted only on Home tab, click still runs, no reason line | `MainWindow.cpp:1438-1454`, `KaShellUi.cpp:14-40,44-46` | `crates/strata-app/src/gui_ribbon_top.rs:137,162-168`, `crates/strata-app/src/gui_ribbon_draw.rs:209-217` | enabled rule | 1 | confusion | S | — |
| B096 | UI_BUTTONS.md #2 | Ribbon (`내보내기`) | `GeoTIFF` chip state & tooltip | Active chip disabled only outside Map tab with reason `「지도」 탭에서 쓸 수 있습니다. 지도 탭으로 옮긴 뒤 누르세요.` -> muted on every tab, `지시 R7` tooltip, no-op | `MainWindowRibbon.cpp:566-573`, `MainWindow.cpp:1084`, `KaShellUi.cpp:48-50` | `crates/strata-app/src/gui_ribbon_draw.rs:56-62,210`, `crates/strata-app/src/gui_ribbon_click_b.rs:145`, `crates/strata-app/src/gui_ribbon_top.rs:137-138` | enabled rule | 2 | confusion | S | R7: GeoTIFF raster exporter |
| B097 | UI_BUTTONS.md #3 | Ribbon (`내보내기`) | `검수·제출` tooltip | Long tooltip text at `MainWindowRibbon.cpp:576` -> empty string (`_ => ""`) | `MainWindowRibbon.cpp:576` | `crates/strata-app/src/ribbon_icons_map.rs:44-70`, `crates/strata-app/src/gui_ribbon_draw.rs:209-211` | tooltip | 2 | looks | S | — |
| B098 | UI_BUTTONS.md #4 | Ribbon (`배경 지도`, `기록`, `정합`) | Lit/checked look of 7 chips (`지형`, `토양`, `지질`, `수계`, `DEM`, `그리기`, `정합`) | Legend-synced checkable (`지형`/`토양`/`지질`/`수계`/`DEM`), `그리기` only on draw/capture, `정합` uncheckable -> first 4 never lit, `DEM` lit whenever non-Home, `그리기` lit in Measure, `정합` lit | `MainWindowRibbon.cpp:374,404,442,507,514,522-523`, `MainWindow.cpp:727-731,1415-1419` | `crates/strata-app/src/gui_ribbon_top.rs:140-152` | state or look | 1 | confusion | S | — |
| B099 | UI_BUTTONS.md #5 | Ribbon (`조사`, `내보내기`) | `저장`, `도면`, `인쇄` strong tile & `save_unsaved` | All 3 always `strongIcon`; `저장` swaps to `save_unsaved` + bold label when unsaved -> `저장` strong only when dirty, `도면`/`인쇄` plain, `save_unsaved` unused, no bold | `MainWindowRibbon.cpp:294,557,565`, `MainWindowChrome.cpp:180-190` | `crates/strata-app/src/gui_ribbon_top.rs:139`, `crates/strata-app/src/gui_ribbon_draw.rs:91-108`, `crates/strata-app/src/ribbon_icon_svg_part1.rs:25` | state or look | 1 | confusion | S | — |
| B100 | UI_BUTTONS.md #6 | Ribbon (`배경 지도`) | `고지형` click & EXTRA 7-item menu | Click calls `startPaleoLandform`; no context menu -> click opens Paleo studio tab; adds EXTRA 7-item right-click menu | `MainWindowRibbon.cpp:469`, `MainWindowOverlay.cpp:343` | `crates/strata-app/src/gui_ribbon_click_b.rs:99`, `crates/strata-app/src/gui_ribbon_menus_a.rs:12-53`, `crates/strata-app/src/gui_ribbon_menu_actions.rs:23` | behaviour | 2 | confusion | M | P1: 2D paleo-landform hypothesis seeding |
| B101 | UI_BUTTONS.md #7 | Ribbon (`배경 지도`) | `지질` EXTRA menu & tooltip suffix | No right-click menu; tooltip ends `. 다시 누르면 숨깁니다` -> EXTRA 6-item right-click menu; tooltip lacks suffix | `MainWindowRibbon.cpp:501-507` | `crates/strata-app/src/gui_ribbon_menus_a.rs:55-91`, `crates/strata-app/src/gui_ribbon_menu_actions.rs:24`, `crates/strata-app/src/ribbon_icons_map.rs:62` | extra in Rust | 2 | confusion | S | — |
| B102 | UI_BUTTONS.md #8 | Ribbon (`배경 지도`) | `DEM` menu EXTRA 1st item | 4 menu items -> 5 items (extra 1st `국토지리정보원 계정 설정·자동 다운로드…`) | `MainWindowRibbon.cpp:411-419` | `crates/strata-app/src/gui_ribbon_menus_a.rs:101` | extra in Rust | 2 | confusion | S | — |
| B103 | UI_BUTTONS.md #9 | Ribbon (`자료 받기`, `배경 지도`) | Duplicate-action submenus (`수치지형`, `지적`, `토양`, `DEM`) | Distinct actions per menu item -> duplicate actions on items (`수치지형`, `지적`, `토양`, `DEM` hillshade/DemClass) | `MainWindowRibbon.cpp:391-392,411-419,447-452,477-478` | `crates/strata-app/src/gui_ribbon_menus_a.rs:151-154,182-184,210-213`, `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87` | behaviour | 2 | confusion | M | — |
| B104 | UI_BUTTONS.md #10 | Ribbon (`기타`) | `더보기` menu (`계정`, `웹 자료`, `화면 보기`, captions, shortcuts, 8 wrong routes) | 3 account items, 6-entry `웹 자료`, `화면 보기` submenu, paper-only captions, `Ctrl+F11`/`F9` shortcuts -> merged accounts, missing `웹 자료`, merged/always-visible captions, extra separator, wrong labels, 8 items open `AccountSettings` | `MainWindowRibbon.cpp:649-708`, `KaMenuCaption.cpp:25-37`, `KaShellFocus.h:27-28` | `crates/strata-app/src/gui_ribbon_menus_b.rs:22-115` | behaviour | 1 | confusion | M | — |
| B105 | UI_BUTTONS.md #11 | Ribbon | 5 real tooltip wording differences (`선택`, `유산`, `지질`, `GeoTIFF`, `검수·제출`) | Full wording on `선택` (`(A 또는 Ctrl+1)`, `단추를`), `유산` (2nd line), plus `지질`/`GeoTIFF`/`검수·제출` -> shortened/omitted texts | `KaFeatureSelectPick.cpp:98`, `MainWindowRibbon.cpp:547-548,503,566-576` | `crates/strata-app/src/ribbon_icons_map.rs:44-70`, `crates/strata-app/src/gui_ribbon_draw.rs:209-211` | tooltip | 1 | looks | S | — |
| B106 | UI_BUTTONS.md #12 | Ribbon | `<label> — <tooltip>` prefix on 10 chips | `keepNameInTip` prepends `<label> — ` when label absent from tooltip -> never prepended (`열기`, `측거`, `그리기`, `버퍼`, `고지형`, `수계`, `옛 지도`, `정합`, `인쇄`, `더보기`) | `KaBeginnerRibbon.cpp:54-59` | `crates/strata-app/src/gui_ribbon_draw.rs:209-211` | tooltip | 1 | looks | S | — |
| B107 | UI_BUTTONS.md #13 | Ribbon (`배경 지도`) | `옛 지도` menu (`▤  ` prefix, icons, key check, tooltip) | `대동여지도` & `1919 조선지형도 1:5만` with `old_map`/`old_topo` icons, disabled + tooltip without history key -> `▤  ` prefix, no icon, no key check or tooltip | `MainWindowRibbon.cpp:493-499`, `MainWindow.cpp:1425-1436` | `crates/strata-app/src/gui_ribbon_menus_a.rs:223` | state or look | 2 | confusion | S | — |
| B108 | UI_BUTTONS.md #14 | Ribbon (`내보내기`) / Shell | `도면` shortcut `Ctrl+L` | Both chip & `Ctrl+L` call `openLayoutDesigner` (reuses open Drawing tab) -> `Ctrl+L` always opens paper setup modal | `MainWindowRibbon.cpp:554-555` | `crates/strata-app/src/gui_shell.rs:139-140`, `crates/strata-app/src/gui_ribbon_click_b.rs:122-132` | behaviour | 2 | confusion | S | — |
| B109 | UI_BUTTONS.md #15 | Ribbon | Icon rendering engine (`KaIcons` vs SVG) | Own `KaIcons` artwork across 26 chips -> Lucide-style SVGs (IDs match; drawings not compared) | `MainWindowRibbon.cpp` | `crates/strata-app/src/ribbon_icon_svg_part1.rs`, `crates/strata-app/src/ribbon_icons_map.rs:12-37` | state or look | 1 | looks | L | — |

---

## 2. Proposed Fix Orders

All **85** rows with `needs = —`, ordered strictly by best `(seen, risk)` (`1 -> 2 -> 3`, then `data -> result -> confusion -> looks`), capped at 10 rows per group, and partitioned by window or file family with symmetric Rust file conflict declarations.

### `UI3-01` — Fix close prompt texts and add modal box & session confirm renderers
- **Priority**: `seen: 1`, `risk: data` | **Size**: `M`
- **Rows (3)**: `B008`, `B090`, `B091`
- **Rust files touched**: `crates/strata-app/src/shell_dialogs.rs`, `crates/strata-app/src/feature_record.rs` (plus new `MISSING` box renderers)
- **Must not run beside**: `UI3-06` (`shell_dialogs.rs`)

### `UI3-02` — Wire editing message boxes (`C14`) and `이름·번호` / `도형 속성` modals (`C4`, `C9`)
- **Priority**: `seen: 1`, `risk: data` | **Size**: `M`
- **Rows (3)**: `B037`, `B041`, `B042`
- **Rust files touched**: `crates/strata-app/src/app_state_feature_record.rs`, `crates/strata-app/src/dlg_feature_record.rs`, `crates/strata-app/src/dlg_feature_form.rs`, `crates/strata-app/src/app_state_feature_form.rs`, `crates/strata-app/src/layer_menu_model.rs`
- **Must not run beside**: `UI3-05` (`layer_menu_model.rs`)

### `UI3-03` — Fix Status bar CRS menu, scale combo, snap chip & Home idle text
- **Priority**: `seen: 1`, `risk: data` | **Size**: `M`
- **Rows (4)**: `B073`, `B074`, `B075`, `B076`
- **Rust files touched**: `crates/strata-app/src/shell_workspace.rs`
- **Must not run beside**: none

### `UI3-04` — Drive Home guide card & status badge from real `SurveyFacts` and clean filter boxes
- **Priority**: `seen: 1`, `risk: result` | **Size**: `M`
- **Rows (3)**: `B067`, `B068`, `B069`
- **Rust files touched**: `crates/strata-app/src/gui_tab_home_hero.rs`, `crates/strata-app/src/gui_tab_home_recent.rs`, `crates/strata-app/src/gui_tab_home_recent_box.rs`, `crates/strata-app/src/gui_layer_panel_tree.rs`
- **Must not run beside**: `UI3-05`, `UI3-17` (`gui_layer_panel_tree.rs`)

### `UI3-05` — Fix Left layer panel `표시 설정`, `파일함` toggle/drives, tree chrome & menu actions
- **Priority**: `seen: 1`, `risk: result` | **Size**: `L`
- **Rows (4)**: `B077`, `B078`, `B079`, `B080`
- **Rust files touched**: `crates/strata-app/src/gui_layer_panel_tree.rs`, `crates/strata-app/src/gui_layer_panel_card.rs`, `crates/strata-app/src/gui_layer_panel_rows.rs`, `crates/strata-app/src/layer_menu_model.rs`
- **Must not run beside**: `UI3-02` (`layer_menu_model.rs`), `UI3-04`, `UI3-17` (`gui_layer_panel_tree.rs`)

### `UI3-06` — Add `Enter` default-button handling and modal behavior to `controls-a` dialogs
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `M`
- **Rows (1)**: `B014`
- **Rust files touched**: `crates/strata-app/src/dlg_new_survey.rs`, `crates/strata-app/src/shell_dialogs.rs`, `crates/strata-app/src/modal_paper_account.rs`, `crates/strata-app/src/modal_export.rs`, `crates/strata-app/src/gui_ribbon_region_b.rs`
- **Must not run beside**: `UI3-01` (`shell_dialogs.rs`), `UI3-21` (`modal_paper_account.rs`), `UI3-22` (`gui_ribbon_region_b.rs`), `UI3-30` (`dlg_new_survey.rs`)

### `UI3-07` — Fix Tab bar survey badge click, `테마 ▼` popup separation & dynamic closable tabs
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `S`
- **Rows (3)**: `B070`, `B071`, `B072`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_tabbar.rs`
- **Must not run beside**: none

### `UI3-08` — Fix Right inspector panel controls, quick basemap sync & resizable pane widths
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `M`
- **Rows (2)**: `B081`, `B082`
- **Rust files touched**: `crates/strata-app/src/ri_body.rs`, `crates/strata-app/src/ri_layer_card.rs`, `crates/strata-app/src/ri_body_bottom.rs`, `crates/strata-app/src/right_inspector.rs`, `crates/strata-app/src/gui_layer_panel_consts.rs`, `crates/strata-app/src/gui_tab_map_consts.rs`
- **Must not run beside**: none

### `UI3-09` — Fix Map overlays (`OpacityRail`, `Undo`/`Redo` pill, `MapControls`, `ScaleBar`, `ShellGrid`) and add `격자 원점` (`C11`)
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `M`
- **Rows (3)**: `B034`, `B083`, `B084`
- **Rust files touched**: `crates/strata-app/src/gui_tab_map_opacity.rs`, `crates/strata-app/src/gui_tab_map_undo.rs`, `crates/strata-app/src/gui_tab_map_overlay.rs`, `crates/strata-app/src/gui_tab_map_gridctl.rs`, `crates/strata-app/src/gui_tab_map_canvas_overlay.rs`
- **Must not run beside**: none

### `UI3-10` — Fix Ribbon chip enabled guards, lit/strong states, `save_unsaved` and chip tooltips
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `M`
- **Rows (6)**: `B095`, `B097`, `B098`, `B099`, `B105`, `B106`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_top.rs`, `crates/strata-app/src/gui_ribbon_draw.rs`, `crates/strata-app/src/ribbon_icons_map.rs`, `crates/strata-app/src/ribbon_icon_svg_part1.rs`
- **Must not run beside**: `UI3-12` (`ribbon_icon_svg_part1.rs`, `ribbon_icons_map.rs`), `UI3-20` (`ribbon_icons_map.rs`)

### `UI3-11` — Rebuild `더보기` menu captions, `웹 자료` / `화면 보기` submenus, shortcuts & routing
- **Priority**: `seen: 1`, `risk: confusion` | **Size**: `M`
- **Rows (2)**: `B001`, `B104`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_menus_b.rs`
- **Must not run beside**: `UI3-35` (`gui_ribbon_menus_b.rs`)

### `UI3-12` — Match `KaIcons` SVG artwork across the 26 ribbon chips
- **Priority**: `seen: 1`, `risk: looks` | **Size**: `L`
- **Rows (1)**: `B109`
- **Rust files touched**: `crates/strata-app/src/ribbon_icon_svg_part1.rs`, `crates/strata-app/src/ribbon_icons_map.rs`
- **Must not run beside**: `UI3-10` (`ribbon_icon_svg_part1.rs`, `ribbon_icons_map.rs`), `UI3-20` (`ribbon_icons_map.rs`)

### `UI3-13` — Wire trial-trench generator widgets, real terrain readout, plan/grid output & confirm boxes (`C5`, `C6`)
- **Priority**: `seen: 2`, `risk: data` | **Size**: `L`
- **Rows (2)**: `B036`, `B039`
- **Rust files touched**: `crates/strata-app/src/dlg_trench_generator.rs`, `crates/strata-app/src/dlg_trench_generator_controls.rs`
- **Must not run beside**: none

### `UI3-14` — Fix `GPS 기준점` dialog parity and add `기준점 축` confirmation dialog (`C1`, `C3`)
- **Priority**: `seen: 2`, `risk: data` | **Size**: `M`
- **Rows (2)**: `B031`, `B040`
- **Rust files touched**: `crates/strata-app/src/modal_control.rs`
- **Must not run beside**: none

### `UI3-15` — Wire Drawing studio modal prompts (`D2`), `도면 목록` popup, `레이어를 가운데`, `CTX-M` & `Delete` key
- **Priority**: `seen: 2`, `risk: data` | **Size**: `M`
- **Rows (5)**: `B044`, `B052`, `B053`, `B057`, `B063`
- **Rust files touched**: `crates/strata-app/src/gui_tab_drawing_popups.rs`, `crates/strata-app/src/gui_shell.rs`, `crates/strata-app/src/gui_tab_drawing*.rs` (`MISSING` `CTX-M`)
- **Must not run beside**: `UI3-17` (`gui_tab_drawing*.rs`), `UI3-29` (`gui_shell.rs`)

### `UI3-16` — Convert `검수·제출` (`A6`) checklist into collapsible 3-group tree and fix `set_work_crs` routing
- **Priority**: `seen: 2`, `risk: result` | **Size**: `M`
- **Rows (1)**: `B011`
- **Rust files touched**: `crates/strata-app/src/modal_export_table.rs`
- **Must not run beside**: none

### `UI3-17` — Fix Drawing studio right panel, `C12` grid dialog, sheet defaults & remove EXTRA controls
- **Priority**: `seen: 2`, `risk: result` | **Size**: `M`
- **Rows (10)**: `B043`, `B054`, `B055`, `B056`, `B058`, `B059`, `B060`, `B061`, `B062`, `B065`
- **Rust files touched**: `crates/strata-app/src/gui_tab_drawing_right.rs`, `crates/strata-app/src/gui_tab_drawing_right_b.rs`, `crates/strata-app/src/gui_tab_drawing_sheet.rs`, `crates/strata-layout/src/sheet_drawing.rs`, `crates/strata-app/src/gui_layer_panel_tree.rs`
- **Must not run beside**: `UI3-04`, `UI3-05` (`gui_layer_panel_tree.rs`), `UI3-15` (`gui_tab_drawing*.rs`)

### `UI3-18` — Wire real map preview, align/add actions & cell tooltips in NGII topographic import (`TI`)
- **Priority**: `seen: 2`, `risk: result` | **Size**: `L`
- **Rows (1)**: `B024`
- **Rust files touched**: `crates/strata-app/src/dlg_topo_import_preview.rs`, `crates/strata-app/src/dlg_topo_import_actions.rs`
- **Must not run beside**: `UI3-27` (`dlg_topo_import_actions.rs`)

### `UI3-19` — Enforce DEM layer guard, real raster min/max & `[적용][닫기]` buttons in `DEM 표현` (`DC`)
- **Priority**: `seen: 2`, `risk: result` | **Size**: `M`
- **Rows (1)**: `B025`
- **Rust files touched**: `crates/strata-app/src/modal_style_demclass.rs`
- **Must not run beside**: `UI3-27` (`modal_style_demclass.rs`)

### `UI3-20` — Fix ribbon chip submenus (`수치지형`, `지적`, `토양`, `DEM`, `지질`, `옛 지도`) and add `A11` dialog
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `M`
- **Rows (5)**: `B002`, `B101`, `B102`, `B103`, `B107`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_menus_a.rs`, `crates/strata-app/src/gui_ribbon_menu_actions.rs`, `crates/strata-app/src/ribbon_icons_map.rs`
- **Must not run beside**: `UI3-10`, `UI3-12` (`ribbon_icons_map.rs`), `UI3-33` (`gui_ribbon_menus_a.rs`), `UI3-34` (`gui_ribbon_menu_actions.rs`)

### `UI3-21` — Split `API 키 입력` (`A3`), `VWorld 지적도` (`A10`), `HA` & `TA` into dedicated account dialogs
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `M`
- **Rows (2)**: `B009`, `B022`
- **Rust files touched**: `crates/strata-app/src/modal_paper_account.rs`
- **Must not run beside**: `UI3-06` (`modal_paper_account.rs`)

### `UI3-22` — Add async `위치 검색` progress modal (`A9`) and fix region popup (`A14`) combo & outside-click dismiss
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `M`
- **Rows (2)**: `B007`, `B013`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_search.rs`, `crates/strata-app/src/gui_ribbon_region_b.rs`
- **Must not run beside**: `UI3-06` (`gui_ribbon_region_b.rs`)

### `UI3-23` — Build missing `레이어 추가` modal dialog (`A12`)
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `L`
- **Rows (1)**: `B004`
- **Rust files touched**: not found in `crates/strata-app/src` (`MISSING`)
- **Must not run beside**: none

### `UI3-24` — Build missing `겹치는 구간 분리 (클립)` dialog and enable ribbon `구간 분리` button (`C10`)
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `L`
- **Rows (1)**: `B033`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_draw_row.rs`
- **Must not run beside**: none

### `UI3-25` — Add 5 km nearby group, timestamp & tooltip and remove decorative background window in `HR`
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `M`
- **Rows (1)**: `B023`
- **Rust files touched**: `crates/strata-app/src/dlg_heritage_region_select.rs`
- **Must not run beside**: `UI3-27` (`dlg_heritage_region_select.rs`)

### `UI3-26` — Fix `취소` enabled/close rule, hint label & extra buttons in `지적도 다운로드` (`DP1`)
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `S`
- **Rows (1)**: `B027`
- **Rust files touched**: `crates/strata-app/src/cadastral_download_ui_render.rs`
- **Must not run beside**: `UI3-27` (`cadastral_download_ui_render.rs`)

### `UI3-27` — Add `Enter`/`Esc` keyboard handling and `.resizable(true)` across `controls-b` modal windows
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `M`
- **Rows (2)**: `B028`, `B029`
- **Rust files touched**: `crates/strata-app/src/dlg_heritage_browser.rs`, `crates/strata-app/src/dlg_heritage_browser_footer.rs`, `crates/strata-app/src/dlg_heritage_region_select.rs`, `crates/strata-app/src/dlg_topographic_browser.rs`, `crates/strata-app/src/dlg_topographic_import.rs`, `crates/strata-app/src/dlg_topo_import_actions.rs`, `crates/strata-app/src/cadastral_download_ui_render.rs`, `crates/strata-app/src/dem_download_ui_render.rs`, `crates/strata-app/src/dlg_survey_contour.rs`, `crates/strata-app/src/modal_style_demclass.rs`
- **Must not run beside**: `UI3-18` (`dlg_topo_import_actions.rs`), `UI3-19` (`modal_style_demclass.rs`), `UI3-25` (`dlg_heritage_region_select.rs`), `UI3-26` (`cadastral_download_ui_render.rs`), `UI3-32` (`dlg_survey_contour.rs`)

### `UI3-28` — Add `도형 색` (`C8`) & color/alpha picker (`C13`) and enforce non-line guard in `선 색` (`C7`)
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `L`
- **Rows (3)**: `B032`, `B035`, `B038`
- **Rust files touched**: `crates/strata-app/src/modal_style.rs`, `crates/strata-app/src/gui_layer_panel_context_menu.rs`
- **Must not run beside**: none

### `UI3-29` — Make `Ctrl+L` shortcut reuse open Drawing tab like the `도면` chip
- **Priority**: `seen: 2`, `risk: confusion` | **Size**: `S`
- **Rows (1)**: `B108`
- **Rust files touched**: `crates/strata-app/src/gui_shell.rs`, `crates/strata-app/src/gui_ribbon_click_b.rs`
- **Must not run beside**: `UI3-15` (`gui_shell.rs`)

### `UI3-30` — Match C++ origin texts, title, ruler tooltip & `maxLength(100)` in `새 조사` (`A1`)
- **Priority**: `seen: 2`, `risk: looks` | **Size**: `S`
- **Rows (1)**: `B012`
- **Rust files touched**: `crates/strata-app/src/dlg_new_survey.rs`, `crates/strata-app/src/dlg_new_survey_ruler.rs`
- **Must not run beside**: `UI3-06` (`dlg_new_survey.rs`)

### `UI3-31` — Replace hardcoded `자료 준비 상태` (`A7`) rows with live inventory and remove 7th action column
- **Priority**: `seen: 3`, `risk: result` | **Size**: `M`
- **Rows (1)**: `B010`
- **Rust files touched**: `crates/strata-app/src/modal_reference.rs`
- **Must not run beside**: none

### `UI3-32` — Wire `찾기…`, real point parsing, `contourProgress`, ComboBox & tooltips in `측량 등고선` (`SC`)
- **Priority**: `seen: 3`, `risk: result` | **Size**: `L`
- **Rows (3)**: `B018`, `B019`, `B026`
- **Rust files touched**: `crates/strata-app/src/dlg_survey_contour.rs`
- **Must not run beside**: `UI3-27` (`dlg_survey_contour.rs`)

### `UI3-33` — Build missing `토양도 불러오기 설정` dialog and SHP picker flow (`A13`)
- **Priority**: `seen: 3`, `risk: confusion` | **Size**: `L`
- **Rows (1)**: `B003`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_menus_a.rs`
- **Must not run beside**: `UI3-20` (`gui_ribbon_menus_a.rs`)

### `UI3-34` — Build missing `지형분석` hillshade dialog and route `DEM 파일로 음영 만들기…` to it (`TG`)
- **Priority**: `seen: 3`, `risk: confusion` | **Size**: `L`
- **Rows (1)**: `B015`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_menu_actions.rs`
- **Must not run beside**: `UI3-20` (`gui_ribbon_menu_actions.rs`)

### `UI3-35` — Build missing `기준점 미리보기` (`C2`) dialog and wire `더보기 -> CSV 기준점`
- **Priority**: `seen: 3`, `risk: confusion` | **Size**: `L`
- **Rows (1)**: `B030`
- **Rust files touched**: `crates/strata-app/src/gui_ribbon_menus_b.rs`
- **Must not run beside**: `UI3-11` (`gui_ribbon_menus_b.rs`)

### `UI3-36` — Fix print dialog `방법` default, `600 DPI` label/tooltip & `배열` / `+ 안내 1장` ledger text (`D3`)
- **Priority**: `seen: 3`, `risk: confusion` | **Size**: `S`
- **Rows (1)**: `B064`
- **Rust files touched**: `crates/strata-app/src/dlg_drawing_print_options.rs`, `crates/strata-app/src/dlg_drawing_print_actions.rs`, `crates/strata-app/src/modal_controller_state.rs`
- **Must not run beside**: none

---

## 3. Waiting for a Feature

All **24** rows where `needs != —`, grouped by prerequisite feature:

1. **R8: 3D terrain engine & studio** — **5** rows (`B087`, `B088`, `B089`, `B093`, `B094`)
2. **W-50: OS printing & multi-page PDF exporter** — **5** rows (`B046`, `B047`, `B048`, `B049`, `B050`)
3. **R8: HWP/HWPX report studio** — **3** rows (`B085`, `B086`, `B092`)
4. **R3: Drawing PDF exporter** — **2** rows (`B045`, `B051`)
5. **R7: GeoTIFF raster exporter** — **2** rows (`B016`, `B096`)
6. **R7: Thematic WFS & offline-map download workers** — **1** row (`B017`)
7. **R2: EPSG:5179 SHP export & submit package engine** — **1** row (`B006`)
8. **DEFERRED_WB1: Embedded WebView & 국가유산청 portal session** — **1** row (`B020`)
9. **DEFERRED_WB2: Embedded WebView & 국토지리정보원 portal session** — **1** row (`B021` — note: deleting the 3 hardcoded `m_downloads` rows in `dlg_topo_browser_body.rs:123-140` can be done ahead of WB2)
10. **R10: Attribution credits & startup splash** — **1** row (`B005`)
11. **P1: 2D paleo-landform hypothesis seeding** — **1** row (`B100`)
12. **Unsaved recovery snapshot writer (`복구사본`)** — **1** row (`B066`)

---

## 4. Counts

### Per source table

| source table | items in its list | backlog rows citing it | notes |
|---|---|---|---|
| `controls-a.md` | 14 | 14 | `B001`–`B014` |
| `controls-b.md` | 15 | 15 | `B015`–`B029` |
| `controls-c.md` | 13 | 13 | `B030`–`B042` (`#5` merged into `B034`) |
| `controls-d.md` | 23 | 23 | `B043`–`B065` |
| `controls-e.md` | 20 | 20 | `B066`–`B084` plus `#18` merged into `B034` |
| `controls-f.md` | 10 | 10 | `B085`–`B094` |
| `UI_BUTTONS.md` (§3) | 15 | 15 | `B095`–`B109` |
| **Total** | **110** | **109** unique rows | 1 exact duplicate merged (`controls-c.md #5` + `controls-e.md #18` -> `B034`) |

### Per `class`

| class | count |
|---|---|
| `text` | 5 |
| `tooltip` | 6 |
| `default or range` | 2 |
| `enabled rule` | 8 |
| `state or look` | 11 |
| `behaviour` | 35 |
| `missing control` | 12 |
| `missing message` | 7 |
| `missing window` | 19 |
| `extra in Rust` | 4 |
| **Total** | **109** |

### Per `seen`

| seen | count |
|---|---|
| `1` | 31 |
| `2` | 56 |
| `3` | 22 |
| **Total** | **109** |

### Section 2 & 3 summary

- **Groups in Section 2 (`Proposed fix orders`)**: **36** (`UI3-01`–`UI3-36`), covering **85** rows
- **Waiting for a feature (Section 3)**: **12** prerequisite features, covering **24** rows
- **Total rows covered**: `85 + 24 = 109`

---

## Could not determine

Mistakes and discrepancies noticed in `controls-a..f` (left untouched in the source files per order instructions):

- **`controls-d.md` (`Counts`)**: `D1` states `38 C++ controls` yet the mapped breakdown gives `37 mapped + 2 EXTRA (18 yes, 19 no, 2 EXTRA)`, leaving 1 C++ control unaccounted for (`18 + 19 = 37`). In the same section, `Differences by column` states `MISSING (controls + boxes): 42`, whereas summing the `MISSING` counts across windows (`D2: 14 + D3: 6 main + 8 sub + C12: 5 + CTX-M: 5`) totals `38`.
- **`controls-f.md` (`Counts`)**: `Differences by column` reports `MISSING (controls + boxes): 72 (26 + 11 + 6 + 2 + 15 + 5 + 7)` which leaves `D12` (`2 C++ generic box patterns, 0 Rust MISSING`) and `D14` (`2 C++ stub boxes`) outside the `MISSING` sum.
- **`controls-c.md` (`C5` row #6 & `#5`)**: `controls-c.md` line 174 cites `trench_types.rs:22-28` without a crate path even though `trench_types.rs` lives in `crates/strata-engine/src/trench_types.rs` rather than `crates/strata-app/src/`. `controls-c.md #5` (`C11`) cites `gui_tab_map_canvas_overlay.rs:40` (where only a comment notes the absent button) whereas `controls-e.md #18` cites `gui_tab_map_gridctl.rs:1-155`.
- **Overlapping non-identical entries across the seven lists**: `controls-a.md #1` (`더보기 -> 자료 준비 상태` opens `AccountSettings`) and `controls-a.md #9` (`API 키 입력` merged in `AccountSettings`) overlap with the bundled `더보기` item in `UI_BUTTONS.md #10`; `controls-b.md #2` (`GT` dialog/flow) complements `UI_BUTTONS.md #2` (`GeoTIFF` ribbon chip enabled rule/tooltip); `controls-c.md #3` (`C8 도형 색` missing dialog) complements `controls-e.md #14` (Left panel context-menu gaps). They are kept as separate backlog rows rather than merged because their scopes differ.
- **`UI_BUTTONS.md` §5 notes**: Section 5 (`Other things in the ribbon area`, e.g., `그리기` sub-toolbar label visibility, `버퍼 -> 엑셀 내보내기`, search box width/clear button, `F11` vs `Ctrl+F11`) is outside §3 (`Differences to fix`, 15 items) and therefore not included in this backlog.
