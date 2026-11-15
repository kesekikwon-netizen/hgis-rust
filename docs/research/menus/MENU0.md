# MENU0 — Side-by-Side Survey of the Eleven Ribbon Map Buttons (C++ vs Rust)

- **Order**: `MENU0` (`ORDER-MENU0.yaml:1-80`)
- **Checkout**: `f277ea6` (read-only survey; no code, build, runtime, or network access)
- **C++ reference**: `target/cpp-ref` (cited as `<file name>:line`)
- **Rust source**: `crates/**` (cited as `<file name>:line`)
- **Owner context** (`ORDER-MENU0.yaml:10-11`):
  - 2026-11-15: 「세부메뉴 보강을 하라 제미나이에게 조사를 해서 이세부메뉴가 다른것이 몇가지가있다. 버튼을 누르거나 우클릭 했을때 그리고 도면이 다운로드받아 후처리되어 레이어에 들어오는 이런것을 조사를 하라고하라 그리고 그걸 받아 넌 rust를 이용 구현을 해야한다」
  - 2026-11-15: 「안티그라비티가 할것은 잘하는것을 작게 지정해주라」

## Shared Ribbon Mechanics & Notes on Starting Pointers

1. **Map-tab auto-switch before running a map button**:
   - **C++**: Every button and sub-menu action placed in `mapOnlyRibbonGroups()` (`record`, `fetch`, `basemap`, `align`, `MainWindowRibbon.cpp:579-581`) connects `pressed` / `triggered` to `showMapTabFromStudio()` (`MainWindowRibbon.cpp:584-592`), switching `m_viewTabs` back to the Map tab (`index 0`) before the button slot runs.
   - **Rust**: Ribbon click handlers explicitly set `self.active_tab = MainTab::Map` (or `self.switch_to_map_tab()`) per button (`gui_ribbon_click_a.rs:159, 166`, `gui_ribbon_click_b.rs:15, 23`, `topo_download_run.rs:85`, `ref_map_run.rs:108`), **except** `고지형`, which opens `MainTab::Paleo` (`gui_ribbon_click_b.rs:42`).
2. **Menu arrows vs right-click on ribbon chips**:
   - **C++**: `QWidget#beginnerRibbon QToolButton::menu-button`, `::menu-arrow`, and `::menu-indicator` all have `width: 0px; height: 0px; image: none;` in `ka-hgis.qss:379-393` (noted in `MainWindowRibbon.cpp:394`: `"The ribbon theme hides menu arrows, so the sub-menu opens on right-click like DEM and 토양"`). Thus buttons configured with `QToolButton::MenuButtonPopup` + `Qt::CustomContextMenu` (`수치지형`, `DEM`, `토양`, `지적`) display **no visible arrow** and open their menu by **right click only**; `옛 지도` uses `QToolButton::InstantPopup` (`MainWindowRibbon.cpp:490`) with no `CustomContextMenu`, so its menu opens by **left click only**.
   - **Rust**: `draw_ribbon_chip` (`gui_ribbon_draw.rs:64-218`) draws no arrow glyph on any chip. Among all ribbon chips, only `"더보기"` opens `open_ribbon_context_menu` on **left click** (`gui_ribbon_top.rs:167-168`); all map buttons with menus in Rust (`"고지형"`, `"지질"`, `"DEM"`, `"토양"`, `"지적"`, `"수치지형"`, `"옛 지도"`) open `open_ribbon_context_menu` on **right click only** (`gui_ribbon_top.rs:174-175`, `gui_ribbon_menu_actions.rs:22-35`), while left-clicking `"옛 지도"` directly toggles `"대동여지도"` (`gui_ribbon_click_b.rs:37-43`).
3. **Corrections to starting pointers in `ORDER-MENU0.yaml:24-26` and Rust comments**:
   - `MainWindowRibbon.cpp` line citations in `gui_ribbon_consts.rs:13-15` match `target/cpp-ref/src/app/MainWindowRibbon.cpp`.
   - `MainWindowChrome.cpp` (`ORDER-MENU0.yaml:26`, cited in `gui_ribbon_menus_a.rs:92`) builds the top `QMenuBar`, not the ribbon button `QMenu`s; every ribbon button `QMenu` is defined in `MainWindowRibbon.cpp:382-550`.
   - `gui_ribbon_click_b.rs:30` cites `MainWindowOverlay.cpp:405-472`, which is shifted relative to `target/cpp-ref/src/app/MainWindowOverlay.cpp`: actual ranges are `editDemElevationClasses` (`:269-297`), `toggleDemMap` (`:299-341`), `startPaleoLandform` (`:343-435`), `downloadSoilTerrain` (`:439-488`), `downloadGeologyMap` (`:492-546`), `downloadRiverMap` (`:550-601`), and `startReferenceDownload` (`:603-695`).

---

## 1. `지적` (Group: `자료 받기`)

### Part A — C++ Left Click
- **Ribbon wiring**: `cadastralBtn` in group `fetch` (`자료 받기`), text `"지적"`, tooltip `"조사구역 주변 5km 지적도를 받아 경계선과 지번을 표시합니다. 우클릭: 계정·선 색 설정"`, non-checkable, connects `clicked` to `MainWindow::downloadCadastral` (`MainWindowRibbon.cpp:472-475, 484`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::downloadCadastral()` (`MainWindowCadastral.cpp:82-174`)**:
  1. `if (m_isOpeningSurvey || m_closingWindow) return;` — silent early return (`MainWindowCadastral.cpp:83`).
  2. `if (m_referenceDownload)` — sets status bar `"지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요."` (5,000 ms) and returns (`MainWindowCadastral.cpp:84`).
  3. `if (m_surveyPath.isEmpty() || !QFileInfo::exists(m_surveyPath))` — shows `QMessageBox::information` titled `"지적도"` with `"먼저 조사를 열거나 저장하고 조사구역을 그려 주세요."` and returns (`MainWindowCadastral.cpp:85-86`).
  4. Iterates `LayerOps::surveyAreaLayers(project)` and transforms geometries to `workCrs`; on transform exception calls `KaUserError::warn` titled `"지적도"` (`"조사구역 좌표계를 확인하지 못했습니다."`, `MainWindowCadastral.cpp:94-112`).
  5. `if (parts.isEmpty())` — shows `QMessageBox::information` titled `"지적도"` with `"조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다."` and returns (`MainWindowCadastral.cpp:114`).
  6. Checks `!CadastralSettings::hasCredentials()`; if missing, runs `configureCadastralAccount()` (`MainWindowCadastral.cpp:34-54`); if still missing, returns (`MainWindowCadastral.cpp:117-120`).
- **What it starts**: Opens cancelable `KaDownloadProgressDialog` titled `"지적도 다운로드"` with subtitle `"VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."` (`MainWindowCadastral.cpp:122-123`) and starts background `KaReferenceDownloadJob` running `CadastralPortal::downloadForSurvey` + `CadastralImport::prepare` -> `CadastralImport::addOrUpdateLayer` (`MainWindowCadastral.cpp:124-173`).

### Part B — C++ Menu
- **How it opens**: **Right click only** (`MenuButtonPopup` + `Qt::CustomContextMenu` at `MainWindowRibbon.cpp:480-483`, menu arrow hidden at `0px` by `ka-hgis.qss:379-393`).
- **Rows in order** (`MainWindowRibbon.cpp:476-479`):
  1. `"VWorld 계정 설정"` — calls `MainWindow::configureCadastralAccount` (`MainWindowCadastral.cpp:34-54`: modal dialog `"VWorld 지적도 계정"` saving VWorld ID/password via `CadastralSettings::saveCredentials`). Always enabled, not checkable.
  2. `"선 색·지번 표시"` — calls `MainWindow::configureCadastralStyle` (`MainWindowCadastral.cpp:56-80`: if no `ka_hgis/cadastral` layer is loaded, shows `QMessageBox::information` titled `"지적도"` with `"지적도를 먼저 받아 주세요."`; otherwise opens modal `"지적도 선 색·지번"` with `"경계선 0.2mm"` color button default `#000000` and `"지번 표시"` checkbox default checked, re-applying style via `CadastralImport::applyStyleToProject`). Always enabled, not checkable.

### Part C — C++ from Download to Legend
1. **Area asked for**: Unary union of all `survey_area` features in work CRS (`EPSG:5187` or `EPSG:5186`), buffered by **5,000 m** (24 segments/quadrant, `CadastralPortal.cpp:222-227`). Transformed to `EPSG:4326` (`CadastralPortal.cpp:229-236`); queries VWorld Data API `https://api.vworld.kr/req/data` (`LT_C_ADSIGG_INFO`) for municipality polygons intersecting the 5 km buffer (`CadastralPortal.cpp:231-262`).
2. **Post-processing**:
   - Authenticates to `https://www.vworld.kr/v4po_usrlogin_a004.do` (`CadastralPortal.cpp:264-292`), queries `/dtmk/dtmk_ntads_s002.do` (`ds_id=30563`, `CadastralPortal.cpp:293-323`), and downloads each municipality ZIP via `/dtmk/downloadResourceFile.do` (`CadastralPortal.cpp:322-350`), skipping cached ZIPs whose header starts with `"PK\x03\x04"` (`CadastralPortal.cpp:312-321`).
   - Extracts `.shp`/`.shx`/`.dbf`/`.prj`/`.cpg` (CP949 archive member names), opens each SHP as CP949 (default `EPSG:5174` when `.prj` is missing, `CadastralImport.cpp:150-165`), transforms parcels to work CRS, keeps parcels intersecting the 5 km buffer (`CadastralImport.cpp:176-214`), normalizes `PNU` and derives `JIBUN` (via `JibunFormat` from `JIBUN` or `PNU`, `CadastralImport.cpp:200-209`), and writes a single `MultiPolygon` GPKG (`CadastralImport.cpp:220-241`). Reuse cache hits when SHA-256 over work CRS + scope WKT + sorted source SHP metadata matches `<sha256>.sha256` (`CadastralImport.cpp:74-103, 134-141`).
3. **Files and folders written**:
   - `<surveyDir>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<safeName>` (raw ZIPs, `CadastralPortal.cpp:299-305`)
   - `<surveyDir>/지적도/원본/추출/<zipStem>/...` (extracted shapefiles, `CadastralPortal.cpp:317, 328`)
   - `<surveyDir>/지적도/표시/<sha256>.gpkg` (table `cadastral`, fields `PNU`, `JIBUN`, `CadastralImport.cpp:118-125, 224-239`)
   - `<surveyDir>/지적도/표시/<sha256>.sha256` (cache key sidecar, `CadastralImport.cpp:119, 240`)
4. **Layers added**:
   - `"지적도 · 조사 주변 5km"` (`CadastralImport.cpp:273`) — placed at **root layer tree index `0`** (`LayerOps::insertVectorLayerAtTop`, `CadastralImport.cpp:283`, `LayerOps.cpp:1091-1100`, not inside `"참조 지도"`), hollow polygon with `#000000` (or saved color) `0.2` mm outline + `JIBUN` 8 pt Malgun Gothic labels with 0.8 mm white buffer at scale `1:1`–`1:10,000` (`CadastralImport.cpp:244-266`), **visible**, **read-only** (`LayerOps::markReferenceLayer` + `setReadOnly(true)`, `CadastralImport.cpp:285-286`).
5. **Status texts**:
   - Progress subtitle: `"VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."` (`MainWindowCadastral.cpp:123`).
   - Cancel: `"지적도 받기를 취소했습니다."` (`MainWindowCadastral.cpp:139`).
   - Done: `"조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."` (`MainWindowCadastral.cpp:168`).
6. **Second click when layer is there**: Re-runs `downloadCadastral()`, hits the `<sha256>.gpkg` cache if `survey_area` and ZIPs have not changed (`CadastralImport.cpp:134-141`), removes the existing `ka_hgis/cadastral` layer (`CadastralImport.cpp:275-282`), and re-inserts `"지적도 · 조사 주변 5km"` at root index `0` (does **not** toggle visibility off).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `1-A1` | `click` | Ribbon group (`자료 받기`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:21-22`, `gui_ribbon_click_a.rs:159` — in `자료 받기`, switches `self.active_tab = MainTab::Map`. |
| `1-A2` | `click` | Left-click pre-checks & immediate background download start | `DIFFERENT` | `gui_ribbon_click_a.rs:158-162`, `left_layer_panel.rs:55-73`, `cadastral_download_ui_render.rs:17-170` — skips all pre-checks on click, immediately adds WMS layer `"연속지적도 (VWorld WMS)"` to `left_layers`, and opens modal `ActiveModalDialog::CadastralDownload` where the user must click `"지적도 받기"` (`cadastral_download_ui_render.rs:115-130`). |
| `1-B0` | `menu row` | Right-click-only menu trigger (no arrow) | `SAME` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:32`, `gui_ribbon_draw.rs:64-218` — opens on right click only, no arrow drawn. |
| `1-B1` | `menu row` | Row 1 `"VWorld 계정 설정"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:163, 180-181` — same verbatim label, enabled/unchecked, but opens shared multi-service `ActiveModalDialog::AccountSettings` rather than a dedicated `"VWorld 지적도 계정"` dialog. |
| `1-B2` | `menu row` | Row 2 `"선 색·지번 표시"` | `MISSING` | `gui_ribbon_menus_a.rs:164, 182-184` — row label exists, but clicking it opens `ActiveModalDialog::CadastralDownload` instead of a `"지적도 선 색·지번"` dialog (`none` in Rust). |
| `1-C1` | `post-processing` | 5 km buffered `survey_area` union & `LT_C_ADSIGG_INFO` polygon intersection | `DIFFERENT` | `cadastral_download_ui_worker.rs:161-187`, `cadastral_download_ui_paths.rs:48-84` — expands `survey_area` axis-aligned bounding box by 5,000 m and tests static catalog `center_lon_lat` points against the WGS84 bbox instead of 24-segment polygon buffer intersection against `LT_C_ADSIGG_INFO`. |
| `1-C2` | `post-processing` | VWorld portal login (`POST /v4po_usrlogin_a004.do`) | `MISSING` | `cadastral_download_ui_worker.rs:93-117` — calls `fetch_cadastral_catalog_page` unauthenticated without logging in (comment at `cadastral_download_ui_worker.rs:102-104` notes unauthenticated catalog lacks `fileNo` entries). |
| `1-C3` | `post-processing` | Raw ZIP filename & `"PK\x03\x04"` cache validation | `DIFFERENT` | `cadastral_download_ui_paths.rs:28-33`, `cadastral_download_ui_worker.rs:124-142` — writes `<surveyDir>/지적도/원본/<file_no>.zip` (without `<sizeKb>-<revision>-<safeName>`) and checks only `len() > 0`. |
| `1-C4` | `post-processing` | ZIP extraction (`추출`), CP949 SHP decode, 5 km parcel clip, `<sha256>.gpkg` + `.sha256` | `MISSING` | `none` (`cadastral_download_ui_worker.rs:144-158` returns `WorkerEvent::Downloaded` right after saving raw `.zip` files). |
| `1-C5` | `layer` | Vector layer `"지적도 · 조사 주변 5km"` & completion status | `FAKE` | `cadastral_download_ui_state.rs:106-117`, `cadastral_download_ui_paths.rs:41-45` — sets `status_bar_hint` `"조사 주변 5km 지적도를 추가했습니다..."` (`:109`) and inserts `"지적도" -> true` into `layer_visible` (`:114-115`) without creating or adding any vector layer to `left_layers` or `ref_vectors`. |
| `1-C6` | `click` | Second click when layer is present | `DIFFERENT` | `gui_ribbon_click_a.rs:158-162` — re-ensures `"연속지적도 (VWorld WMS)"` visible and re-opens `ActiveModalDialog::CadastralDownload`. |

---

## 2. `수치지형` (Group: `자료 받기`)

### Part A — C++ Left Click
- **Ribbon wiring**: `topoBtn` in group `fetch` (`자료 받기`), text `"수치지형"`, tooltip `"국토정보플랫폼에서 수치지형도 받기 (우클릭: 받아 둔 폴더 열기)"`, non-checkable, connects `clicked` to `MainWindow::openTopographicDownload` (`MainWindowRibbon.cpp:385-388, 484`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::openTopographicDownload()` (`MainWindowTopographic.cpp:18-72`)**:
  1. `if (!m_viewTabs) return;` — silent early return (`MainWindowTopographic.cpp:19`).
  2. `if (m_surveyPath.isEmpty() || !QFileInfo::exists(m_surveyPath))` — shows `QMessageBox::information` titled `"수치지형도"` with `"먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 지형도 폴더에 저장합니다."` and returns (`MainWindowTopographic.cpp:20-23`).
  3. `if (!QDir().mkpath(topoDir))` (`<surveyDir>/지형도`) — shows `QMessageBox::warning` titled `"수치지형도"` and returns (`MainWindowTopographic.cpp:25-32`).
- **What it starts**: Lazily constructs non-modal `KaTopographicBrowser` + `KaTopographicScopePanel` + hidden `KaTopographicImportDialog` (`MainWindowTopographic.cpp:34-61`), sets library directory to `<surveyDir>/지형도` (`MainWindowTopographic.cpp:62`), shows and raises `m_topographicBrowser` (`MainWindowTopographic.cpp:63-65`), and calls `m_topographicScope->activate()` (`MainWindowTopographic.cpp:70`).

### Part B — C++ Menu
- **How it opens**: **Right click only** (`MenuButtonPopup` + `Qt::CustomContextMenu` at `MainWindowRibbon.cpp:395-397`, menu arrow hidden at `0px` by `ka-hgis.qss:379-393`).
- **Rows in order** (`MainWindowRibbon.cpp:389-393`):
  1. `"국토지리정보원에서 내려받기…"` — calls `MainWindow::openTopographicDownload` (`MainWindowTopographic.cpp:18-72`). Always enabled, not checkable.
  2. `"받아 둔 수치지형도 폴더…"` — calls `MainWindow::importTopographicFolder` (`MainWindowTopographic.cpp:102-120`: opens `QFileDialog::getExistingDirectory` titled `"수치지형도 폴더 선택 — 하위 폴더도 확인합니다"` and, if non-empty, opens `KaTopographicImportDialog::openWithFolder`, which merges selected layers into `<surveyDir>/지형도/합침/수치지형도-합침-<ms>.gpkg` and adds `"수치지형도"` via `publishPrepared`, `KaTopographicImportDialog.cpp:892-995`). Always enabled, not checkable.

### Part C — C++ from Download to Legend
1. **Area asked for**: `KaTopographicScopePanel::refreshScope()` (`KaTopographicScopePanel.cpp:499-537`) takes the **current map canvas centre** (`m_canvas->extent().center()` in work CRS) and selects 1:5,000 sheets within **`kRadiusKm = 5.0` km** (`KaTopographicScopePanel.cpp:38, 507-509`), transforming the centre to `EPSG:5179` (`KaTopographicScopePanel.cpp:520-522`) for the NGII portal radius search and drawing a dashed blue 5 km rubber-band circle on the canvas (`KaTopographicScopePanel.cpp:524-526`).
2. **Post-processing**:
   - `KaTopographicBrowser` logs into NGII 국토정보플랫폼 (`map.ngii.go.kr`), runs the 5 km radius 1:5,000 DXF/SHP order, and downloads ZIPs into `<surveyDir>/지형도/원본` (`KaTopographicScopePanel.cpp:495`).
   - `TopographicLibrary` indexes and extracts archives; `KaTopographicImportDialog` filters/reads line features, reprojects from source CRS (`EPSG:5186`/`5187`/`5179`) to work CRS, clips geometries to the **5,000 m buffer of `survey_area`** when `survey_area` features exist (`KaTopographicImportDialog.cpp:721-726, 814`), and writes merged `MultiLineString` features into a timestamped GeoPackage (`KaTopographicImportDialog.cpp:919-934`).
3. **Files and folders written**:
   - `<surveyDir>/지형도/원본/` (downloaded NGII ZIPs, `KaTopographicScopePanel.cpp:495`)
   - `<surveyDir>/지형도/files/`, `catalog/`, `SHP/`, `index/<sha256>.json`, `receipts/<sha256>.json` (library archive & index directories under `<surveyDir>/지형도`)
   - `<surveyDir>/지형도/합침/수치지형도-합침-<yyyyMMdd-HHmmss-zzz>.gpkg` (table `topo`, `KaTopographicScopePanel.cpp:496`, `KaTopographicImportDialog.cpp:919-937`)
4. **Layers added**:
   - `"수치지형도"` (`KaTopographicImportDialog.cpp:936, 986`) — placed in the **flat root layer list** above basemap rasters and below survey layers (`LayerOps::placeInLegendGroup` at `LayerOps.cpp:1021-1038` ignores `groupName` to keep the list flat; `KaTopographicImportDialog.cpp:992`), single-symbol `#808080` `0.2` mm solid line with labels off (`KaTopographicImportDialog.cpp:200, 205-209, 987`), **visible**, **read-only** (`markReferenceLayer` + `setReadOnly(true)`, `KaTopographicImportDialog.cpp:989-990`).
5. **Status texts**:
   - Scope check: `"현재 화면 중심 반경 5km의 보관 자료와 공식 도엽을 확인하고 있습니다."` (`KaTopographicScopePanel.cpp:531`).
   - Merge done: `"수치지형도를 합쳐 지도에 올렸습니다."` (`MainWindowTopographic.cpp:57`; Rust `topo_post.rs:25, 69` shows `"수치지형도를 준비했습니다. 도엽 {}장을 지도에 올렸습니다."`).
6. **Second click when layer is there**: Raises `m_topographicBrowser` and re-runs `m_topographicScope->activate()` (`MainWindowTopographic.cpp:62-70`); `publishPrepared` removes any previous `ka_hgis/topographic_merged` layer before inserting the updated `"수치지형도"` (`KaTopographicImportDialog.cpp:980-993`). It does **not** toggle `"수치지형도"` off.

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `2-A1` | `click` | Ribbon group (`자료 받기`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:21, 23`, `gui_ribbon_click_a.rs:164`, `topo_download_run.rs:85` — in `자료 받기`, switches `self.active_tab = MainTab::Map`. |
| `2-A2` | `click` | Left-click pre-checks & browser mode | `DIFFERENT` | `topo_download_run.rs:84-111`, `topo_download.rs:101-144, 163-195` — checks busy hidden browser (`topo` or `dem`), `survey_folder` (with verbatim missing-survey text at `topo_download.rs:104`), drawn `survey_area` centre, `online`, and stored `NgiiTopographic` account upfront (opening `AccountSettings` if missing), then drives a **hidden** WebView2 (`NgiiRun`) reporting `[1/6]`–`[6/6]` in the bottom gauge (owner/plan design, `RUST_PLAN.md` §2.8). |
| `2-B0` | `menu row` | Right-click-only menu trigger (no arrow) | `SAME` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:32`, `gui_ribbon_draw.rs:64-218` — opens on right click only, no arrow drawn. |
| `2-B1` | `menu row` | Row 1 `"국토지리정보원에서 내려받기…"` | `SAME` | `gui_ribbon_menus_a.rs:195, 214-215` — verbatim text, always enabled, unchecked, calls `self.open_topographic_download()`. |
| `2-B2` | `menu row` | Row 2 `"받아 둔 수치지형도 폴더…"` | `FAKE` | `gui_ribbon_menus_a.rs:196, 216-218`, `dlg_topographic_import.rs:24-164`, `dlg_topo_import_actions.rs:64-97` — opens `ActiveModalDialog::TopographicImport` where clicking `"좌표계 확인 완료 · 지도에 추가"` (`dlg_topo_import_actions.rs:85-94`) sets `layer_visible.insert("수치지형도 1:5,000 (합침)", true)` and displays `"수치지형도 레이어 N개를 작업 좌표계(EPSG:5187)로 정합 변환하여 지도에 추가했습니다 ([이미 있음] 표시)."` without reprojecting, merging, writing a GPKG, or adding any layer to `left_layers` or `ref_vectors`. |
| `2-C1` | `post-processing` | 5 km search centre & line geometry filtering | `DIFFERENT` | `topo_download.rs:17, 106-110`, `ref_tile_layers.rs:197-199`, `crates/strata-topo/src/merge.rs:62-105` — requires drawn `survey_area`, centres the 5 km portal search on `survey_area` bbox centre (instead of canvas centre), and retains whole unsplit line features intersecting the 5 km `Scope` around `survey_area` (instead of clipping line geometry at the 5 km boundary). |
| `2-C2` | `post-processing` | Files and folders written under `<surveyDir>/지형도/` | `DIFFERENT` | `topo_download.rs:36`, `crates/strata-topo/src/library.rs:19`, `crates/strata-topo/src/post.rs:25-26, 109-110` — writes `<surveyDir>/지형도/원본/`, `<surveyDir>/지형도/strata-sheets.json`, and `<surveyDir>/지형도/합침/수치지형도-합침-<epoch_ms>.gpkg` (table `topo`), unpacking ZIPs in a temporary scratch folder rather than persisting `files/`, `catalog/`, `SHP/`, `index/`, `receipts/`. |
| `2-C3` | `layer` | Layer `"수치지형도"` name, `#808080` 0.2 mm style, visible, read-only | `SAME` | `topo_post.rs:24, 66-67`, `ref_vector_show.rs:23` — verbatim name `"수치지형도"`, `#808080` 0.2 mm line, no labels, visible, read-only (`ReferenceVectorLayer`). |
| `2-C4` | `layer` | Flat layer list placement above basemap tiles | `SAME` | `ref_vector_show.rs:20-26`, `left_layer_panel.rs:93-122` — flat layer list (`LegendGroup::ReferenceMap` tag, no nested subgroup) stacked above basemap tile rows, matching `LayerOps::placeInLegendGroup` (`LayerOps.cpp:1021-1038`). |
| `2-C5` | `click` | Second click when `"수치지형도"` is in legend | `DIFFERENT` | `topo_download_run.rs:84-111`, `topo_download.rs:133-134`, `crates/strata-topo/src/post.rs:81-115` — runs `open_topographic_download()` again, skips sheet numbers already recorded in `strata-sheets.json`, and re-merges all library sheets into a new `수치지형도-합침-<epoch_ms>.gpkg` with status `"수치지형도를 준비했습니다. 도엽 {}장을 지도에 올렸습니다."` (`topo_post.rs:25, 69`). |

---

## 3. `DEM` (Group: `자료 받기` in Rust / `배경 지도` in C++)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_btnDem` in group `basemap` (`배경 지도`), text `"DEM"`, checkable (`setCheckable(true)`), tooltip `"고도를 색으로 보여 줍니다. 받는 자료는 지표모델(DSM) 30m · Copernicus라 나무·건물 높이가 섞여 있습니다.\n미세지형 판독에는 우클릭 → 「국토지리원 DEM 불러오기」를 쓰세요. 다시 누르면 숨깁니다. 우클릭: 상세 메뉴"`, connects `clicked` to `MainWindow::toggleDemMap` (`MainWindowRibbon.cpp:402-409, 427`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::toggleDemMap()` (`MainWindowOverlay.cpp:299-341`)**:
  1. `if (!m_canvas) return;` — silent early return (`MainWindowOverlay.cpp:301-302`).
  2. Looks up existing DEM raster via `BasemapDsm::findDem(proj)` (`MainWindowOverlay.cpp:305`).
  3. **If existing `dem` is found** (`MainWindowOverlay.cpp:307-335`):
     - Reads current layer visibility `wasVisible = LayerOps::isLayerVisible(proj, "DEM")` and computes `makeVisible = !wasVisible` (`MainWindowOverlay.cpp:308-309`).
     - If `makeVisible && !LayerOps::demCoversCanvas(proj, m_canvas)` (layer was hidden and canvas panned outside covered extent), calls `startDemDownload()` to fetch Copernicus tiles for the new view (`MainWindowOverlay.cpp:311-314`).
     - Otherwise toggles `"DEM"` visibility (`MainWindowOverlay.cpp:315`), sets `m_btnDem->setChecked(makeVisible)` (`MainWindowOverlay.cpp:316-317`), and updates `"지형 음영"`: when `makeVisible` is true, ensures `"지형 음영"` (`LayerOps::ensureDemRelief`) and updates viewport follow (`DemPresentation::followCanvas`, `MainWindowOverlay.cpp:319-322`); when `makeVisible` is false, hides `"지형 음영"` **unless** `"지질도"` is visible (`MainWindowOverlay.cpp:323-325`). Shows status `"DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."` or `"DEM 지형을 숨겼습니다."` (4,000 ms, `MainWindowOverlay.cpp:330-333`).
  4. **If no `dem` exists** (`MainWindowOverlay.cpp:337`): calls `startDemDownload()` (`MainWindowDownloads.cpp:118-160`), which checks `m_isOpeningSurvey || m_closingWindow`, busy `m_referenceDownload`, `referenceDownloadFolder("DEM")` (requiring open survey), thematic scale clamp (`<= 1:100,000`), and `EPSG:5186` canvas extent `<= BasemapDsm::maxSpanMeters()`.
- **What it starts (when no DEM exists)**: Opens cancelable progress dialog for `"DEM"` and runs `BasemapDsm::prepareCopernicus` -> `LayerOps::addDemElevationRaster` + `BasemapDsm::labelCopernicus` (`MainWindowDownloads.cpp:131-159`).

### Part B — C++ Menu
- **How it opens**: **Right click only** (`MenuButtonPopup` + `Qt::CustomContextMenu` at `MainWindowRibbon.cpp:421-425`, menu arrow hidden at `0px` by `ka-hgis.qss:379-393`).
- **Rows in order** (`MainWindowRibbon.cpp:410-419`, 4 actions with no separator):
  1. `"국토지리원 DEM 불러오기(.img)…"` — calls `MainWindow::importDemElevationRaster` (`MainWindow.cpp:1686-1705`: opens `QFileDialog::getOpenFileName` titled `"국토지리원 DEM"` filtering `"국토지리원 DEM (*.img *.IMG *.tif *.tiff *.TIF *.TIFF)"`, adds `"DEM"` + `"지형 음영"` via `LayerOps::addDemElevationRaster` at `BasemapOps.cpp:1996-2037`, zooms to layer if `< 20,000 m`, and shows `"국토지리원 DEM을 올렸습니다. 범례에 높이(m)가 표시됩니다."` or `notify(Notice::Warning, "DEM", err)` on failure). Always enabled, not checkable.
  2. `"DEM 파일로 음영 만들기…"` — calls `MainWindow::runDemHillshade` (`MainWindow.cpp:1707-1774`: opens `QFileDialog` titled `"DEM GeoTIFF"`, opens modal `"지형분석"` dialog with `"방식"` (`"다중광원 음영 (유구·분묘)"` / `"단방향 음영"`), `"방위각"` default 315°, `"고도각"` default 45°, `"Z계수"` default 1.0, writes `<surveyDir>/지형분석/<stem>_hillshade.tif` — or prompts `getSaveFileName` when `m_surveyPath` is empty — adds raster `"음영기복"`, and shows `"음영기복을 참조 지도에 올렸습니다."`). Always enabled, not checkable.
  3. `"DEM 표현…"` — calls `MainWindow::editDemElevationClasses` (`MainWindowOverlay.cpp:269-297`: if no DEM raster is loaded, shows `QMessageBox::information` titled `"DEM 표현"` with `"먼저 DEM을 켜 주세요. 표고 자료를 불러오면 색 표현과 음영을 조절할 수 있습니다."`; otherwise opens non-modal `KaDemClassDialog`, `KaDemClassDialog.cpp:19-132`). Always enabled, not checkable.
  4. `"측량점으로 등고선…"` — calls `MainWindow::createSurveyContours` (`MainWindowSurveyContour.cpp:104-197`, identical to the `등고선` button). Always enabled, not checkable.

### Part C — C++ from Download to Legend
1. **Area asked for**: Current map canvas extent transformed to `EPSG:4326` (expanded to cover 1°×1° Copernicus GLO-30 tiles, span capped at 80 km in `BasemapDsm`), storing covered extent for `LayerOps::demCoversCanvas` (`MainWindowOverlay.cpp:311`).
2. **Post-processing**: Downloads intersecting 1°×1° Cloud-Optimized GeoTIFF windows from AWS public bucket `copernicus-dem-30m.s3.amazonaws.com` via GDAL, merges/clips to `<surveyDir>/ka-hgis-reference-XXXXXX/DEM.tif`, applies `DemPresentation::applyDefault` (`national` 0–2,000 m non-linear pseudocolor ramp), and creates companion hillshade layer `"지형 음영"` via `LayerOps::ensureDemRelief` (`BasemapOps.cpp:2009-2090`).
3. **Files and folders written**:
   - `<surveyDir>/ka-hgis-reference-XXXXXX/DEM.tif` (retained GeoTIFF, `MainWindowDownloads.cpp:131`, `BasemapOps.cpp:1996-2037`)
   - *(Menu Row 2 only)*: `<surveyDir>/지형분석/<stem>_hillshade.tif` (`MainWindow.cpp:1744-1755`, `DemHillshadeFile.cpp:18-138`)
4. **Layers added**:
   - `"지표모델(DSM) 30m · Copernicus"` (`BasemapDsm::labelCopernicus`, `MainWindowDownloads.cpp:147`; or `"DEM"` from Menu Row 1, `BasemapOps.cpp:2007`) — placed in flat root layer list (`LayerOps::placeInLegendGroup`, `BasemapOps.cpp:2026`), singleband pseudocolor ramp (`전국 표준 (0–2000m)` default, `DemPresentation`), **visible**, **read-only** (`markReferenceLayer`).
   - `"지형 음영"` (`BasemapOps.cpp:2046, 2065`) — stacked directly **above** the DEM layer (`stackDemReliefOver`, `BasemapOps.cpp:2079`), QGIS hillshade renderer (`azimuth 315°`, `altitude 45°`, `zFactor 1.0`, Multiply blend, opacity `0.30`), **visible**, **read-only** (`markReferenceLayer`, `ka_hgis/omit_sheet_legend = true`).
   - Map-canvas elevation bar (`DemLegendOverlay`): vertical colour ramp titled `"표고 (m)"` with `"비선형 눈금"` note under `"전국 표준 (0–2000m)"`, hidden whenever the DEM layer is hidden.
5. **Status texts**:
   - Download done: `"DEM을 추가했습니다. DEM 버튼의 「DEM 표현」에서 색과 음영을 조절하세요."` (`MainWindowDownloads.cpp:157`; Rust `dem_post.rs:42, 217` shows `"DEM을 준비했습니다. 도엽 {sheets}장을 지도에 올렸습니다."`).
   - Toggle on / off: `"DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."` / `"DEM 지형을 숨겼습니다."` (`MainWindowOverlay.cpp:330-332`).
6. **Second click when layer is there**: Toggles DEM (and `"지형 음영"`, unless `"지질도"` is visible) **off**, unchecks `m_btnDem`, and shows `"DEM 지형을 숨겼습니다."`; a third click turns it back **on** (or re-downloads Copernicus if the canvas moved outside covered bounds, `MainWindowOverlay.cpp:311-314`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `3-A1` | `click` | Ribbon group placement | `DIFFERENT` | `gui_ribbon_consts.rs:22-24` — `DEM` is in `자료 받기` in Rust vs `배경 지도` in C++ (`MainWindowRibbon.cpp:427`), per owner decision 2026-11-15. |
| `3-A2` | `click` | Checkable button active highlight when DEM is visible | `SAME` | `gui_ribbon_top.rs:146-148`, `dem_look.rs:100-112` — chip `dark_active` tracks visible `"DEM"` / `"지표모델"` / `"국토지리원 DEM"` row. |
| `3-A3` | `click` | Left-click download when no DEM layer exists | `DIFFERENT` | `gui_ribbon_click_a.rs:155`, `topo_download_run.rs:90-111`, `topo_download.rs:101-161` — instead of Copernicus GLO-30 over HTTPS for the canvas, Rust runs hidden WebView2 `OrderOf::Dem` on NGII 국토정보플랫폼 for 1:50,000 sheets within 5 km of `survey_area` using stored `NgiiTopographic` credentials (owner decision 2026-11-15). |
| `3-B0` | `menu row` | Right-click-only menu trigger (no arrow) | `SAME` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:32`, `gui_ribbon_draw.rs:64-218` — opens on right click only, no arrow drawn. |
| `3-BX` | `menu row` | Extra Rust Row 1 `"국토지리정보원 계정 설정·자동 다운로드…"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:101`, `gui_ribbon_menu_actions.rs:81-82` — extra row not in C++ `demMenu`; opens `ActiveModalDialog::AccountSettings`. |
| `3-B1` | `menu row` | C++ Row 1 `"국토지리원 DEM 불러오기(.img)…"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:102`, `gui_ribbon_menu_actions.rs:83-85`, `dem_import.rs:15-56` — opens native `rfd::FileDialog` titled `"국토지리원 DEM"` (`img`/`IMG`/`tif`/`tiff`/`TIF`/`TIFF`), validates via GDAL `check_elevation_file`, adds `"DEM"` + `"지형 음영"` (`show_dem_layers`), zooms when `< 20,000 m`, and shows verbatim `"국토지리원 DEM을 올렸습니다. 범례에 높이(m)가 표시됩니다."`; differs only in reporting open errors on the status bar (`dem_import.rs:38`) instead of `notify(Notice::Warning, "DEM", ...)` (`MainWindow.cpp:1695-1696`). |
| `3-B2` | `menu row` | C++ Row 2 `"DEM 파일로 음영 만들기…"` | `MISSING` | `gui_ribbon_menus_a.rs:103`, `gui_ribbon_menu_actions.rs:87-89` — row label exists, but clicking it opens `ActiveModalDialog::DemClass` (`"DEM 표현"`) instead of a GeoTIFF file picker + `"지형분석"` hillshade writer (`none` in Rust). |
| `3-B3` | `menu row` | C++ Row 3 `"DEM 표현…"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:104`, `gui_ribbon_menu_actions.rs:87-89`, `dem_look.rs:117-170`, `modal_style_demclass.rs:28-95` — same 3 presets, `"현재 화면에 색 맞추기"`, `"지형 음영 합성"`, `"수직과장"` 0.1–5.0, `"음영 세기"` 0–80%, `< 100 m` flat-ground hint (`dem_look.rs:133-140`), and 350 ms viewport follow, except when no DEM is loaded Rust writes verbatim `NEEDS_DEM` to the status bar (`dem_look.rs:118-121`) instead of `QMessageBox::information`, and edits apply live without an `"적용"` button. |
| `3-B4` | `menu row` | C++ Row 4 `"측량점으로 등고선…"` | `FAKE` | `gui_ribbon_menus_a.rs:105`, `gui_ribbon_menu_actions.rs:90-93`, `dlg_survey_contour.rs:184-198` — opens `ActiveModalDialog::SurveyContour`, where `"등고선 만들기"` adds an empty `"단면선"` survey layer and shows a fabricated `"측량점 등고선 약 ...줄... 표고점 248개를 생성했습니다."` message. |
| `3-C1` | `post-processing` | Area, post-processing, and files written | `DIFFERENT` | `dem_post.rs:31-45, 79-219` — saves NGII archives in `<surveyDir>/DEM/원본/`, extracts `.img`+`.ige` into `<surveyDir>/DEM/풀림/<sheet>/`, writes `<surveyDir>/DEM/strata-dem.json`, and builds an in-memory 5 km work-CRS grid + Horn hillshade RGBA mesh (instead of Copernicus `<surveyDir>/ka-hgis-reference-XXXXXX/DEM.tif`). |
| `3-C2` | `layer` | DEM & hillshade layers in legend | `DIFFERENT` | `dem_post.rs:18, 20, 141-152, 217`, `left_layer_panel.rs:93-122` — adds `"지형 음영"` above `"DEM"` (instead of `"지표모델(DSM) 30m · Copernicus"`) with done status `"DEM을 준비했습니다. 도엽 {sheets}장을 지도에 올렸습니다."`. |
| `3-C3` | `layer` | Elevation colour bar (`"표고 (m)"`, `"비선형 눈금"`) | `SAME` | `dem_legend.rs:26-116` — renders vertical ramp `"표고 (m)"` on map right edge, `"비선형 눈금"` note on `"전국 표준 (0–2000m)"`, hidden when `"DEM"` is hidden. |
| `3-C4` | `click` | Second click when DEM layer is in legend | `DIFFERENT` | `gui_ribbon_click_a.rs:108-151` — toggles `"DEM"` and `"지형 음영"` (preserving `"지형 음영"` if `"지질도"`/`"지질도(KIGAM 1:5만)"` is visible) with verbatim status `"DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."` / `"DEM 지형을 숨겼습니다."`, but does not re-download when panned outside initial scope. |

---

## 4. `유산` (Group: `자료 받기`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_actHeritage` / `heritageBtn` in group `fetch` (`자료 받기`), text `"유산"` (action text `"주변유적"`, objectName `"actFetchNearbyHeritage"`), tooltip `"조사구역 시·군과 5km 인접 시·군의 국가유산청 GIS 인트라넷 SHP(지정·현상변경·유존·분포·지표·발굴)를 받아 올립니다"`, non-checkable, connects `triggered` to `MainWindow::fetchNearbyHeritage` (`MainWindowRibbon.cpp:542-550`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-592`).
- **Conditions checked in order inside `MainWindow::fetchNearbyHeritage()` (`MainWindow.cpp:3071-3121`)**:
  1. `LayerOps::findByLayerKey(QgsProject::instance(), "survey_area")` — if null, warns `"주변유적 받기"` / `"조사구역 레이어를 찾지 못했습니다."` / `"주변유적은 조사구역이 속한 시·군 기준으로 받습니다."` / `"「새 조사」로 조사를 시작하거나 「조사 열기」로 기존 조사를 여세요."` (`MainWindow.cpp:3072-3081`).
  2. Iterates `areaVl->getFeatures()` to combine geometry; if `area.isEmpty()`, warns `"주변유적 받기"` / `"먼저 조사구역을 그려 주세요."` / `"어느 시·군의 자료를 받을지 조사구역 위치로 판정합니다."` / `"왼쪽의 「구역 그리기」로 조사 범위를 그린 뒤 다시 누르세요."` (`MainWindow.cpp:3083-3098`).
  3. `if (m_surveyPath.isEmpty())` — warns `"주변유적 받기"` / `"열린 조사가 없습니다."` / `"서약서 동의 기록을 남길 조사폴더가 있어야 받기를 시작합니다."` / `"「새 조사」 또는 「조사 열기」를 먼저 실행하세요."` (`MainWindow.cpp:3099-3107`).
- **What it starts**: Calls `ensureHeritageBrowser()`, displays `m_heritageBrowser->showWaiting("조사구역의 시·군과 주변 5km 시·군을 확인합니다…")` + status bar message (`MainWindow.cpp:3109-3111`), and runs `HeritageRegionResolver::resolve` (`MainWindow.cpp:3112-3120`, `HeritageRegionResolver.cpp:221-352`, `HeritageNearbyRegions.cpp:49-63, 186-236`), which queries VWorld reverse geocoding + `LT_C_ADSIGG_INFO` 5 km boundary neighbours, then opens modal `KaHeritageRegionDialog` (`MainWindow.cpp:3153-3166`, `KaHeritageSetupDialogs.cpp:81-150`), checks `HeritageIntranetSettings::hasCredentials()` (opening modal `KaHeritageAccountDialog` if absent, `MainWindow.cpp:3162-3165`), and launches `KaHeritageBrowser::start()` (`MainWindow.cpp:3185-3191`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:541-550`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Municipality (`시·도` + `시·군·구`) containing `pointOnSurface` of `survey_area` (`HeritageRegionResolver.cpp:226-260`), plus user-checked neighbouring municipalities whose `LT_C_ADSIGG_INFO` polygons intersect a **5,000 m** buffer around `survey_area` (`HeritageNearbyRegions.cpp:49-63, 186-236`). After download, features are filtered to those intersecting the **5,000 m** buffer of `survey_area` (`SurveyScopeClip::keepIntersectingIfSurvey`, `SurveyScopeClip.cpp:19-29, 75-100`, `HeritageImport.cpp:260-270`).
2. **Post-processing**:
   - Embedded `QWebEngineView` (`KaHeritageBrowser`) logs into the KHS GIS intranet (`gis-intranet` RSA `goLogin()`, `HeritageIntranetFlow.h:42-50`), agrees to `"원본자료 사용 서약서"` (`KaHeritageBrowser.cpp:957-1000`), and downloads all **6 fixed datasets** (`HeritageStyle::allDatasets()`, `HeritageStyle.cpp:26-33`: `지정유산`, `현상변경허용기준`, `매장유산유존지역`, `문화유적분포지도`, `지표조사구역`, `발굴조사구역`).
   - Unpacks CP949-named ZIPs via `TopographicArchive::prepare` (`HeritageImport.cpp:178-196`), copies each SHP to a scratch folder, converts CP949/EUC-KR attributes into a persistent UTF-8 GPKG (`HeritageImport.cpp:46-125`), clips features to 5 km around `survey_area`, and styles by dataset (`HeritageImport.cpp:232-315`).
3. **Files and folders written**:
   - `<AppLocalDataLocation>/주변유적/<시·도> <시·군>/원본/*.zip` (downloaded intranet archives, `MainWindow.cpp:3171-3175`)
   - `<AppLocalDataLocation>/주변유적/<시·도> <시·군>/receipts/requests-<yyyyMMdd-HHmmss>.txt` (redacted HTTP request log, `KaHeritageBrowser.cpp:420-443`)
   - `<AppLocalDataLocation>/주변유적/SHP/` and `<AppLocalDataLocation>/주변유적/SHP/UTF8/heritage-XXXXXX/heritage.gpkg` (table `heritage`, `MainWindow.cpp:3201-3204`, `HeritageImport.cpp:49-55, 100-108`)
   - `<surveyDir>/주변유적/receipts/서약서-<yyyyMMdd-HHmmss>.txt` (automatic pledge agreement receipt, `MainWindow.cpp:3228-3244`)
4. **Layers added**:
   - Nested under group `"참조 지도"` -> subgroup `"<datasetName>"` (one of `"지정유산"`, `"현상변경허용기준"`, `"매장유산유존지역"`, `"문화유적분포지도"`, `"지표조사구역"`, `"발굴조사구역"`), each shapefile added as `<datasetName>` (if 1 SHP) or `<shpCompleteBaseName>` (if multiple SHPs, `HeritageImport.cpp:280-314`), single-symbol hollow polygon / line / 2.0 mm circle (`0.35` mm outline in dataset hex `#8E44AD`, `#E67E22`, `#2E86C1`, `#27AE60`, `#D4AA00`, `#C2187D`) + site-name labels (`HeritageStyle.cpp:26-74`), **visible**, **read-only** (`markReferenceLayer`, `HeritageImport.cpp:304-312`).
5. **Status texts**:
   - Resolver start: `"조사구역의 시·군과 주변 5km 시·군을 확인합니다…"` (`MainWindow.cpp:3111`).
   - Per dataset imported: `"%1 %2곳을 올렸습니다."` (`MainWindow.cpp:3220-3223`).
   - All finished: `"주변유적 자료 처리를 마쳤습니다. 받은 자료는 「참조 지도」에 있습니다."` (`MainWindow.cpp:3143`).
6. **Second click when layer is there**: Re-runs `fetchNearbyHeritage()` -> `KaHeritageRegionDialog` (offering optional `heritageReuseRecent` checkbox if recent verified downloads exist for that municipality, `KaHeritageSetupDialogs.cpp:121-131`); on import, `HeritageImport::loadDataset` removes existing child layers inside `"참조 지도"` -> `"<datasetName>"` before inserting the newly loaded layers (`HeritageImport.cpp:285-296`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `4-A1` | `click` | Ribbon group (`자료 받기`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:21, 25`, `gui_ribbon_click_a.rs:166` — in `자료 받기`, switches `self.active_tab = MainTab::Map`. |
| `4-A2` | `click` | Pre-checks (`survey_area`, `survey_path`) & municipality / 5 km neighbour auto-resolution | `MISSING` | `gui_ribbon_click_a.rs:165-168` (`none` for resolver) — skips all `survey_area`/`survey_path` checks and VWorld municipality resolution, immediately opening `ActiveModalDialog::HeritageRegionSelect`. |
| `4-A3` | `click` | `"주변유적 받을 시·군 선택"` dialog | `FAKE` | `dlg_heritage_region_select.rs:12-160` — paints a static dummy backdrop window `"ka_modal_heritage_bg"` (`:24-75`) whose `"상세 보기"` and `"취소"` buttons discard clicks (`let _ = ...` at `dlg_heritage_region_select.rs:59-64`), omits 5 km neighbour checkboxes, shows an unconditional `"최근 받은 자료 다시 쓰기"` checkbox (`:140-142`) without checking whether any local archive exists, and `"이 지역 받기"` (`:154-157`) opens `HeritageBrowser` without checking intranet credentials. |
| `4-A4` | `click` | `"주변유적 받기"` browser window & dataset list | `DIFFERENT` | `dlg_heritage_browser.rs:23-154`, `dlg_heritage_browser_footer.rs:13, 77-78` — shows 7 checkboxes splitting `지정유산` into 5 subtypes and omitting `현상변경허용기준`, `지표조사구역`, `발굴조사구역` (`dlg_heritage_browser.rs:127-135`), has no WebView, and disables `"받기"` (`add_enabled(false, fetch_btn)`) with deferred tooltip `"아직 Rust 판에서 연결하지 않았습니다 (지시 WB1)"` (`dlg_heritage_browser_footer.rs:13, 77-78`). |
| `4-B0` | `menu row` | Menu (none) | `SAME` | `gui_ribbon_menu_actions.rs:22-35` — no right-click or arrow menu. |
| `4-C1` | `post-processing` | Intranet login, automatic pledge agreement, and `<surveyDir>/주변유적/receipts/서약서-*.txt` | `MISSING` | `none` — not implemented in Rust. |
| `4-C2` | `post-processing` | `<AppLocalDataLocation>/주변유적/...` download, CP949 SHP -> UTF-8 GPKG conversion, & 5 km clip | `MISSING` | `none` — not implemented in Rust. |
| `4-C3` | `layer` | `"참조 지도"` -> `"<datasetName>"` layers, 6-colour hollow styling, & status texts | `MISSING` | `none` — not implemented in Rust. |
| `4-C4` | `click` | Second click when layers are in legend | `DIFFERENT` | `gui_ribbon_click_a.rs:165-168` — re-opens static `ActiveModalDialog::HeritageRegionSelect` with no recent-download reuse or layer replacement. |

---

## 5. `지형` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_btnTerrain` in group `basemap` (`배경 지도`), text `"지형"`, checkable (`setCheckable(true)`), tooltip `"등고선과 산세가 그려진 지형도를 켜거나 끕니다 (OpenTopoMap)"`, connects `clicked` to `MainWindow::toggleTerrainMap` (`MainWindowRibbon.cpp:370-381`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::toggleTerrainMap()` (`MainWindowOverlay.cpp:239-267`)**:
  1. `if (!m_canvas) return;` — silent early return (`MainWindowOverlay.cpp:241-242`).
  2. Calls `toggleExistingOverlay(QgsProject::instance(), m_canvas, "지형맵", nullptr, statusBar())` (`MainWindowOverlay.cpp:221-235, 243-249`): **if `"지형맵"` exists**, flips visibility (`LayerOps::toggleLayerVisibility`), updates `m_btnTerrain->setChecked(...)`, and shows `"지형맵를 껐습니다."` or `"지형맵를 다시 켰습니다."` (4,000 ms, `MainWindowOverlay.cpp:227, 232`).
  3. **If `"지형맵"` does not exist**: calls `LayerOps::addElevationHillshadeMap(QgsProject::instance(), m_canvas, VworldSettings::loadApiKey(), &err)` (`MainWindowOverlay.cpp:251-252`, `BasemapOps.cpp:1577-1595`); on failure calls `notify(Notice::Warning, "지형맵", ...)` and resets `m_btnTerrain->setChecked(false)` (`MainWindowOverlay.cpp:253-257`); on success syncs canvas, sets `m_btnTerrain->setChecked(true)`, and shows `"지형맵을 올렸습니다. 다시 누르면 숨깁니다."` (6,000 ms, `MainWindowOverlay.cpp:259-263`).
- **What it starts**: Adds live QGIS XYZ raster layer `"지형맵"` (`BasemapOps.cpp:1577-1595`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:369-381`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Live XYZ tiles in `EPSG:3857` (`zmin=5`, `zmax=17`) over the **entire current map canvas** (no `survey_area` required); if current canvas scale is `> 1:80,000` or `< 1:100`, `addXyzBasemap` zooms canvas to **`1:10,000`** (`BasemapOps.cpp:969-970`).
2. **Post-processing**: None beyond QGIS WMS/XYZ tile rendering; if primary `https://tile.opentopomap.org/{z}/{x}/{y}.png` fails layer validation, falls back to `https://a.tile.opentopomap.org/{z}/{x}/{y}.png` (`BasemapOps.cpp:1583-1586`).
3. **Files and folders written**: None in survey folder or app folders (live streamed XYZ tiles).
4. **Layers added**:
   - `"지형맵"` (`BasemapOps.cpp:1578`) — placed at **root layer tree bottom** below survey layers and below any already-loaded basemap rasters (`placeBasemapBelowVectors`, `BasemapOps.cpp:945-967`), full-opacity raster tiles, **visible**, **read-only** (`ka_hgis/basemap = true`, `BasemapOps.cpp:941`).
5. **Status texts**:
   - First add: `"지형맵을 올렸습니다. 다시 누르면 숨깁니다."` (`MainWindowOverlay.cpp:263`).
   - Toggle off: `"지형맵를 껐습니다."` (`MainWindowOverlay.cpp:227`).
   - Toggle on: `"지형맵를 다시 켰습니다."` (`MainWindowOverlay.cpp:232`).
6. **Second click when layer is there**: Unchecks `"지형맵"` in the layer tree (keeps the layer loaded), unchecks `m_btnTerrain`, and shows `"지형맵를 껐습니다."` (`MainWindowOverlay.cpp:224-228, 243-248`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `5-A1` | `click` | Ribbon group (`배경 지도`), checkable highlight, & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28-29`, `gui_ribbon_top.rs:154`, `gui_ribbon_click_b.rs:14-17` — in `배경 지도`, `warm_active` reflects `terrain_tile_visible()`, switches to `MainTab::Map`. |
| `5-A2` | `click` | Pre-check on first click | `DIFFERENT` | `ref_tile_layers.rs:92, 141-147` — requires drawn `survey_area`, otherwise shows `"조사구역을 먼저 그려 주세요. 조사구역에서 반경 100km 안의 지형맵을 올립니다."` (owner decision 2026-11-15). |
| `5-B0` | `menu row` | Menu (none) | `SAME` | `gui_ribbon_menu_actions.rs:22-35` — no right-click or arrow menu. |
| `5-C1` | `post-processing` | OpenTopoMap XYZ URL (`z 5..=17`, `EPSG:3857`) & zero survey files written | `SAME` | `basemap_tiles.rs:69`, `ref_tile_layers.rs:88-90`, `basemap_fetch.rs:129-135` — fetches `https://tile.opentopomap.org/{z}/{x}/{y}.png` (`max_zoom: 17`), writes no survey files. |
| `5-C2` | `post-processing` | 100 km circle clip/feather & zoom behaviour on add | `DIFFERENT` | `ref_tile_layers.rs:53-54, 91, 150-165`, `ref_tile_paint.rs:118-216` — clips and alpha-feathers tiles to a **100 km circle** around `survey_area` centre and keeps current zoom (`ViewOnAdd::Keep`, panning only if view misses the circle) instead of full-world XYZ + `1:10,000` auto-zoom (owner decision 2026-11-15). |
| `5-C3` | `layer` | Layer `"지형맵"` name, visible, read-only | `SAME` | `ref_tile_layers.rs:87, 148-149` — verbatim `"지형맵"`, visible, read-only tile row. |
| `5-C4` | `layer` | Stacking order among basemap/tile rows | `DIFFERENT` | `ref_tile_layers.rs:105-131` — inserts a newly added tile row **above** older tile rows (below vector layers), whereas C++ places a newly added basemap **below** existing basemap rasters (`BasemapOps.cpp:945-967`). |
| `5-C5` | `click` | Verbatim status texts & second-click visibility toggle | `SAME` | `ref_tile_layers.rs:93-95, 178-189` — verbatim `"지형맵을 올렸습니다. 다시 누르면 숨깁니다."`, `"지형맵를 껐습니다."`, `"지형맵를 다시 켰습니다."` and visibility toggle on second click. |

---

## 6. `등고선` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `contourBtn` in group `basemap` (`배경 지도`), text `"등고선"`, non-checkable, tooltip `"측량 DXF·엑셀 좌표로 등고선과 해발 색 구간을 만듭니다"`, connects `clicked` to `MainWindow::createSurveyContours` (`MainWindowRibbon.cpp:431-436`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::createSurveyContours()` (`MainWindowSurveyContour.cpp:104-197`)**:
  1. `if (m_surveyPath.isEmpty())` — warns `"측량점으로 등고선"` / `"먼저 조사를 열거나 새 조사를 만드세요."` / `"등고선과 표고 격자는 조사 폴더의 측량등고선 아래에 저장합니다."` / `"새 조사 또는 조사 열기 뒤에 다시 누르세요."` (`MainWindowSurveyContour.cpp:105-113`).
  2. `if (findChild<QProgressDialog*>("contourProgress"))` — sets status bar `"등고선을 만들고 있습니다. 끝나거나 취소를 누르세요."` (4,000 ms) and returns (`MainWindowSurveyContour.cpp:114-117`).
  3. Opens modal `KaSurveyContourDialog` (`MainWindowSurveyContour.cpp:118-120`); if rejected, returns silently.
- **What it starts**: Builds `SurveyContourJob` staging into `<surveyDir>/측량등고선/<safeStem>_new` (swapped into `<surveyDir>/측량등고선/<safeStem>` on completion) with clip polygon `surveyAreaGeometry(job.workCrsAuthId)` (`MainWindowSurveyContour.cpp:121-130, 164`), opens cancelable `QProgressDialog` (`objectName = "contourProgress"`, label `"측량점으로 등고선을 계산하고 있습니다."`, `MainWindowSurveyContour.cpp:132-139`), and schedules `SurveyContourTask` on `QgsApplication::taskManager()` (`MainWindowSurveyContour.cpp:142-196`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:429-436`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Bounding extent of the user's input survey elevation points (`DXF`/`CSV`/`TXT`/`XLSX`, optionally excluding suspicious elevation outliers, `MainWindowSurveyContour.cpp:69-87`), optionally clipped to the `survey_area` union polygon (`MainWindowSurveyContour.cpp:36-58, 125`, `SurveyContourClip.cpp`).
2. **Post-processing**: Triangulates survey points into a TIN (`SurveyContourTin.cpp`), rasterizes a north-up elevation grid (`surface.tif`, `NoData = -9999.0`, `SurveyContourWrite.cpp:23, 60-66, 79-100`), generates contour lines (`contour_lines` with `index_line` every 5th interval and `elev_label`) and optional elevation bands (`contour_bands` with `elev_mid`), and styles all 3 layers via `SurveyContourStyle::apply` (`SurveyContourStyle.cpp:70-195`).
3. **Files and folders written**:
   - `<surveyDir>/측량등고선/<safeStem>/surface.tif` (`MainWindowSurveyContour.cpp:122-123, 164`, `SurveyContourWrite.cpp:56`)
   - `<surveyDir>/측량등고선/<safeStem>/contours.gpkg` (tables `survey_points`, `contour_lines`, `contour_bands`, `SurveyContourWrite.cpp:53`, `SurveyContourStyle.cpp:90, 111, 152`)
4. **Layers added** (all inside group `"참조 지도"` -> subgroup `"측량 등고선 · <fileBaseName>"`, `MainWindowSurveyContour.cpp:173-174`, `SurveyContourStyle.cpp:79-87`, all **visible**, all **read-only** via `LayerOps::markReferenceLayer` at `SurveyContourStyle.cpp:54`):
   - `"측량점"` (inserted at subgroup top `index 0`, `SurveyContourStyle.cpp:152-192`) — rule-based circle markers (`#2b6cb0` 1.8 mm normal points + `#d9480f` 2.4 mm suspicious points) with elevation labels at `<= 1:1,000`.
   - `"등고선"` (middle, `SurveyContourStyle.cpp:111-150`) — rule-based lines (`#3a2f24` `0.7` mm `"계곡선"` + `#8a7a66` `0.25` mm `"주곡선"`) with curved 8 pt bold `"elev_label"` labels.
   - `"색 구간"` (bottom, when `bandCount > 0`, `SurveyContourStyle.cpp:89-109`) — graduated 16-stop hypsometric fill (`#72c6b5`..`#f4f3ef`, alpha 170, no outline) on `elev_mid`.
5. **Status texts**:
   - Cancel: `"등고선 만들기를 취소했습니다."` (`MainWindowSurveyContour.cpp:155`).
   - Done: `"등고선 %1줄, 색 구간 %2개를 올렸습니다."` (8,000 ms, `MainWindowSurveyContour.cpp:187`).
6. **Second click when layer is there**: Opens `KaSurveyContourDialog` again; if run for the same `<fileBaseName>`, `SurveyContourStyle::removeGroup` removes the previous `"측량 등고선 · <fileBaseName>"` layers before inserting the rebuilt ones (`SurveyContourStyle.cpp:60-68, 73`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `6-A1` | `click` | Ribbon group (`배경 지도`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28, 30`, `gui_ribbon_click_b.rs:23` — in `배경 지도`, switches `self.active_tab = MainTab::Map`. |
| `6-A2` | `click` | Pre-checks (`survey_path` open, no contour job running) | `MISSING` | `gui_ribbon_click_b.rs:22-25` — opens `ActiveModalDialog::SurveyContour` unconditionally even with no open survey. |
| `6-A3` | `click` | Dialog `"찾기…"` file picker & point-count preview | `FAKE` | `dlg_survey_contour.rs:43, 153-156` — `"찾기…"` ignores clicks (`let _ = ui.add_sized(...)` at `:43`), and any non-empty path text displays hardcoded `"점 248개, 표고 24.5–48.2 m. 등고선 약 {line_est}줄 (간격 {:.1} m). 의심점 3개 제외."` (`:153-156`) without reading any file. |
| `6-B0` | `menu row` | Menu (none) | `SAME` | `gui_ribbon_menu_actions.rs:22-35` — no right-click or arrow menu. |
| `6-C1` | `post-processing` | Survey point parsing, outlier detection, TIN, & contour/band generation | `MISSING` | `none` — no survey-point contour builder exists in Rust. |
| `6-C2` | `post-processing` | `<surveyDir>/측량등고선/<stem>/surface.tif` & `contours.gpkg` | `MISSING` | `none` — writes no files or folders. |
| `6-C3` | `layer` | `"등고선 만들기"` confirm action, `"측량 등고선 · <stem>"` layers, & status | `FAKE` | `dlg_survey_contour.rs:184-198` — clicking `"등고선 만들기"` calls `ensure_domain_layer_in_legend(DomainLayerKey::SectionLine)` adding empty survey domain layer `"단면선"` (`:190`) and sets `draw_guide_message` to `"측량점 등고선 약 {line_est}줄(간격 {interval} m)과 표고점 248개를 생성했습니다."` (`:192-195`) without creating any contour layer. |
| `6-C4` | `click` | Second click when layer is there | `DIFFERENT` | `gui_ribbon_click_b.rs:22-25` — re-opens the fake `ActiveModalDialog::SurveyContour` modal. |

---

## 7. `토양` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_btnSoil` in group `basemap` (`배경 지도`), text `"토양"`, checkable (`setCheckable(true)`), tooltip `"흙토람 분포지형을 받거나 켜고 끕니다 (우클릭: 다시 받기·SHP 불러오기)"`, connects `clicked` to `MainWindow::downloadSoilTerrain` (`MainWindowRibbon.cpp:440-446, 461`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::downloadSoilTerrain()` (`MainWindowOverlay.cpp:439-488`)**:
  1. `if (!m_canvas) return;` — silent early return (`MainWindowOverlay.cpp:441`).
  2. Checks `!qobject_cast<QAction*>(sender())` (true for button left click, false for menu action): if `"토양도(흙토람)"` is already in `project`, toggles its visibility via `toggleExistingOverlay(..., "토양도(흙토람)", ...)` (`MainWindowOverlay.cpp:221-235, 443-450`), updates `m_btnSoil->setChecked(...)`, and shows `"토양도(흙토람)를 껐습니다."` or `"토양도(흙토람)를 다시 켰습니다."` (4,000 ms, `MainWindowOverlay.cpp:227, 232`).
  3. Calls `referenceDownloadFolder("토양도")` (`MainWindowOverlay.cpp:454-455`, prompting for an open survey if `m_surveyPath` is empty).
  4. Clamps canvas scale via `LayerOps::clampCanvasToThematicScale(m_canvas)` (`<= 1:100,000`), showing `"축척을 1:100000으로 맞춘 뒤 토양도를 받습니다."` if clamped (`MainWindowOverlay.cpp:456-457`).
  5. Transforms `m_canvas->extent()` to `EPSG:5186` and checks `ext.width() <= SoilMapService::maxSpanMeters() && ext.height() <= SoilMapService::maxSpanMeters()` (`80,000 m`), else warns `"토양도"` / `"범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요."` (`MainWindowOverlay.cpp:459-479`).
  6. `startReferenceDownload` checks `m_isOpeningSurvey || m_closingWindow` and busy `m_referenceDownload` (`"지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요."`, `MainWindowOverlay.cpp:605-609`).
- **What it starts**: Opens cancelable `QProgressDialog` (`createDownloadProgress("토양도")`, `MainWindowOverlay.cpp:610-612`) and runs `SoilMapService::prepare` -> `SoilMapService::addPrepared` (`MainWindowOverlay.cpp:481-483, 617-694`).

### Part B — C++ Menu
- **How it opens**: **Right click only** (`MenuButtonPopup` + `Qt::CustomContextMenu` at `MainWindowRibbon.cpp:456-459`, menu arrow hidden at `0px` by `ka-hgis.qss:379-393`).
- **Rows in order** (`MainWindowRibbon.cpp:447-452`):
  1. `"분포지형 내려받기 — 현재 화면 범위"` — calls `MainWindow::downloadSoilTerrain` via `QAction::triggered`, so `qobject_cast<QAction*>(sender())` is non-null (`MainWindowOverlay.cpp:443`), **bypassing the visibility toggle** and forcing a fresh download for the current canvas extent. Always enabled, not checkable.
  2. `"토양도 파일 불러오기(SHP·GPKG)"` — calls `MainWindow::importSoilShapefile` (`MainWindow.cpp:2643-2741`: opens `QFileDialog::getOpenFileName` titled `"토양도 SHP 불러오기"` filtering `"Shapefile (*.shp)"`, opens modal `"토양도 불러오기 설정"` for `"좌표계"` + `"색 구분 필드"`, adds layer to `"참조 지도"`, and shows `"토양도를 올렸습니다: %1"`). Always enabled, not checkable.

### Part C — C++ from Download to Legend
1. **Area asked for**: Current map canvas extent in `EPSG:5186` (after clamping canvas scale `<= 1:100,000`, `MainWindowOverlay.cpp:456-457`), capped at **`maxSpanMeters() = 80,000 m`** (`SoilMapService.cpp:257-263`), transformed to `EPSG:4326` (`SoilMapService.cpp:264-266`). Polygons returned by WFS are **not** clipped at the bbox edge (`SoilMapService.cpp:320-345`).
2. **Post-processing**: Queries NAAS WFS `http://apis.data.go.kr/1390802/SoilEnviron/SoilMapWfs/getSoilMapWfs` (`soilmap:SOIL_1`, `soilmap:SOIL_2`, `soilmap:SOIL_3`, splitting into 4 quadrants recursively if a request hits the 1,000-feature cap, `SoilMapService.cpp:279-290`), merges into `MultiPolygon` in `EPSG:5186` normalizing `soil_type_geo` codes (`01`..`10`, `99`), and writes `토양도_흙토람.gpkg` (`SoilMapService.cpp:305-350`).
3. **Files and folders written**:
   - `<surveyDir>/ka-hgis-reference-XXXXXX/soil-%1.geojson` (in `result.storage`, `SoilMapService.cpp:303`)
   - `<surveyDir>/ka-hgis-reference-XXXXXX/토양도_흙토람.gpkg` (retained, table `soil_map`, `MainWindowOverlay.cpp:481`, `SoilMapService.cpp:254, 349`)
4. **Layers added**:
   - `"토양도(흙토람)"` (`SoilMapService.cpp:370`) — placed via `LayerOps::placeInLegendGroup` (`SoilMapService.cpp:394`), stacked **above** `"토양도(흙토람 그림)"`, categorized fill on `soil_type_geo` (11 terrain colours alpha `175`, `#282c34` outline `0.28` mm) + Korean terrain labels for polygons `>= 40,000 m²` at `<= 1:40,000` (`SoilMapService.cpp:105-248`), layer visibility capped at `<= 1:100,000` (`applyThematicOverlayScaleRange`, `SoilMapService.cpp:395`), **visible**, **read-only** (`markReferenceLayer`, `SoilMapService.cpp:382`).
   - `"토양도(흙토람 그림)"` (`SoilMapService.cpp:399-419`) — stacked **below** `"토양도(흙토람)"`, live ArcGIS World Hillshade XYZ raster (`z 1..16`, `EPSG:3857`, opacity `0.55`, `ka_hgis/omit_sheet_legend = true`), visibility capped at `<= 1:100,000`, **visible**, **read-only**.
5. **Status texts**:
   - Cancel: `"토양도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."` (`MainWindowOverlay.cpp:635`).
   - Done: `"토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."` (`MainWindowOverlay.cpp:672`).
   - Toggle off / on: `"토양도(흙토람)를 껐습니다."` / `"토양도(흙토람)를 다시 켰습니다."` (`MainWindowOverlay.cpp:227, 232`).
6. **Second click when layer is there**: Toggles `"토양도(흙토람)"` visibility **off** without re-downloading, unchecks `m_btnSoil`, and shows `"토양도(흙토람)를 껐습니다."` (`MainWindowOverlay.cpp:443-450`). (To re-download when already loaded, the user uses right-click Row 1.)

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `7-A1` | `click` | Ribbon group (`배경 지도`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28, 31`, `gui_ribbon_click_b.rs:29`, `ref_map_run.rs:108` — in `배경 지도`, switches `self.active_tab = MainTab::Map`. |
| `7-A2` | `click` | Checkable button active highlight when `"토양도(흙토람)"` is visible | `MISSING` | `gui_ribbon_top.rs:140-156` — neither `dark_active` nor `warm_active` checks `"토양도(흙토람)"`, so the `"토양"` chip never lights up when active. |
| `7-A3` | `click` | Pre-checks on first click | `DIFFERENT` | `ref_map_run.rs:115-154`, `ref_map.rs:106-128` — checks busy reference download and open `survey_path`, plus requires drawn `survey_area` and `online` (instead of canvas extent `<= 80 km`). |
| `7-B0` | `menu row` | Right-click-only menu trigger (no arrow) | `SAME` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:32`, `gui_ribbon_draw.rs:64-218` — opens on right click only, no arrow drawn. |
| `7-B1` | `menu row` | Row 1 `"분포지형 내려받기 — 현재 화면 범위"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:134, 151-154` — calls `self.handle_ribbon_click("토양")`, which **toggles `"토양도(흙토람)"` off** if already loaded instead of forcing a re-download, and fetches the 10 km `survey_area` scope rather than the current screen extent. |
| `7-B2` | `menu row` | Row 2 `"토양도 파일 불러오기(SHP·GPKG)"` | `MISSING` | `gui_ribbon_menus_a.rs:135, 151-154` — row label exists, but `apply_soil_menu_choice` ignores `_item` and calls `self.handle_ribbon_click("토양")`; no SHP/GPKG soil import dialog exists (`none`). |
| `7-C1` | `post-processing` | Scope (10 km `survey_area` buffer), polygon clipping, & files written | `DIFFERENT` | `ref_map.rs:106-128, 218-241`, `crates/strata-refmap/src/soil_prepare.rs:62-173` — fetches NAAS WFS `soilmap:SOIL_1..3` for a **10 km buffer around `survey_area`** and **clips polygons** to that 10 km boundary (owner decision 2026-11-15), writing `<surveyDir>/ka-hgis-reference-XXXXXX/토양도_흙토람.gpkg` (table `soil_map`) + `ka-hgis-scope.json` and reusing the kept folder when `ka-hgis-scope.json` matches. |
| `7-C2` | `layer` | Layer `"토양도(흙토람)"` name, 11-class style & labels, visible, read-only | `SAME` | `ref_map.rs:108-120`, `ref_vector_show.rs:23`, `crates/strata-refmap/src/soil_style.rs` — verbatim `"토양도(흙토람)"`, 11 terrain colours alpha 175, outline 0.28 mm, Korean terrain labels `>= 40,000 m²` at `<= 1:40,000`, visible, read-only. |
| `7-C3` | `layer` | Companion `"토양도(흙토람 그림)"` hillshade layer & `1:100,000` scale cap | `MISSING` | `gui_ribbon_click_b.rs:27-28` (`none`) — `"토양도(흙토람 그림)"` ArcGIS World Hillshade underlay and the `1:100,000` layer visibility cap are not added (noted as deferred `NET-SOIL1b`). |
| `7-C4` | `layer` | Verbatim status bar texts on add, cancel, & toggle | `SAME` | `ref_map.rs:125, 127`, `ref_map_run.rs:112, 224-229` — verbatim `"토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."`, `"토양도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."`, `"토양도(흙토람)를 껐습니다."`, `"토양도(흙토람)를 다시 켰습니다."`. |
| `7-C5` | `click` | Second click on button when `"토양도(흙토람)"` is in legend | `SAME` | `ref_map_run.rs:109-114` — toggles `"토양도(흙토람)"` visibility without re-downloading. |

---

## 8. `고지형` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_btnPaleo` in group `basemap` (`배경 지도`), text `"고지형"`, non-checkable, tooltip `"흙토람에서 선상지·하안단구·구하도·자연제방 가설을 뽑아 「고지형 판독」 레이어에 채우고 다듬기 모드로 엽니다"`, connects `clicked` to `MainWindow::startPaleoLandform` (`MainWindowRibbon.cpp:464-470`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-590`).
- **Conditions checked in order inside `MainWindow::startPaleoLandform()` (`MainWindowOverlay.cpp:343-435`)**:
  1. `if (m_isOpeningSurvey || m_closingWindow) return;` — silent early return (`MainWindowOverlay.cpp:345`).
  2. `if (m_surveyPath.isEmpty())` — shows `QMessageBox::information` titled `"고지형"` with `"먼저 「새 조사」로 저장 위치를 만드세요.\n그다음 조사구역으로 확대한 뒤 다시 「고지형」을 누르면 흙토람 분포지형이 깔립니다."` and returns (`MainWindowOverlay.cpp:346-352`).
  3. `if (!m_canvas) return;`, then clamps canvas scale via `LayerOps::clampCanvasToThematicScale(m_canvas)` (`MainWindowOverlay.cpp:353-355`).
  4. Searches for loaded soil terrain layer via `PaleoLandformService::findSoilTerrainLayer(proj)` (`MainWindowOverlay.cpp:357`, `PaleoLandformService.cpp:259-267`).
  5. **If no soil layer is loaded** (`MainWindowOverlay.cpp:358-386`): transforms canvas extent to `EPSG:5186`, verifies `width <= 80 km && height <= 80 km` (else shows `"지금 화면이 너무 넓습니다. 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 「고지형」을 누르세요.\n전국·시도 화면에는 분포지형을 깔지 않습니다."`), starts `startReferenceDownload(ReferenceMapKind::PaleoSoil, ext, "<surveyDir>/토양도_흙토람.gpkg")`, and upon completion re-invokes `startPaleoLandform()` (`MainWindowOverlay.cpp:675-680`).
- **What it starts (once soil layer is present)**: Calls `PaleoLandformService::applyCandidateEmphasis(soil)` (`MainWindowOverlay.cpp:387`), ensures `<survey.gpkg>|layername=paleo_landform` (`"고지형 판독"`) via `PaleoLandformService::ensureInterpretationLayer` (`MainWindowOverlay.cpp:393-399`), seeds hypothesis polygons via `PaleoLandformService::seedInterpretationFromSoil` (`MainWindowOverlay.cpp:400-401`), enters polygon editing (`beginEdit(layer)`, `MainWindowOverlay.cpp:402`), and shows `Notice::Success` + `QMessageBox::information` + status bar guidance (`MainWindowOverlay.cpp:403-431`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:462-470`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Current canvas extent in `EPSG:5186` (clamped `<= 1:100,000`, span `<= 80 km`, identical to `SoilMapService::prepare`) if `"토양도(흙토람)"` was not yet loaded; otherwise operates on all features of the already-loaded `"토양도(흙토람)"` layer (`PaleoLandformService.cpp:196-242`).
2. **Post-processing**:
   - Re-styles `"토양도(흙토람)"` so candidate codes `04, 05, 06, 08` render at alpha `220` with `0.55` mm outline and `"입지후보 · <terrainName>"` legend labels while non-candidate classes fade to alpha `95` (`PaleoLandformService.cpp:82-108`).
   - Seeds `"고지형 판독"` (`status = "가설"`, `note = "자동: ..."`): maps `04` -> `"선상지"`, `05` -> `"해성평탄"`, `08` -> `"하안단구"`, and for `06` (`하성평탄지`) computes negative inset `clamp(r * 0.22, 8.0, 80.0)` m to split wide polygons into inner `"구하도"` (`"자동: 하성평탄 안쪽"`) + outer rim `"자연제방"` (`"자동: 하성평탄 가장자리"`), or `"미저지"` when narrow (`PaleoLandformService.cpp:139-242`).
3. **Files and folders written**:
   - `<surveyDir>/ka-hgis-reference-XXXXXX/토양도_흙토람.gpkg` (when auto-fetched on step 5 of Part A, `MainWindowOverlay.cpp:383-384`, `SoilMapService.cpp:254`)
   - Table `paleo_landform` (`kind`, `note`, `status`) inside the survey GeoPackage `<m_surveyPath>` (`PaleoLandformService.cpp:325-361`)
4. **Layers added**:
   - `"고지형 판독"` (`PaleoLandformService.cpp:324, 369`) — placed via `LayerOps::placeInLegendGroup` (`PaleoLandformService.cpp:380`), categorized fill on `kind` across 9 classes (`자연제방`, `선상지`, `하안단구`, `구하도`, `미고지`, `미저지`, `해성평탄`, `개변`, `미분류`, labeled `"<kind> · 자동 가설"` or `"<kind> · 가설"`, `0.35` mm outline, `PaleoLandformService.cpp:42-48, 269-306`), visibility capped at `<= 1:100,000` (`PaleoLandformService.cpp:374`), **visible**, **editable** (opened in edit mode via `startEditing()` + `beginEdit(layer)`, `PaleoLandformService.cpp:254-255`, `MainWindowOverlay.cpp:402`).
5. **Status texts**:
   - Seeded `> 0`: `"고지형 가설 %1개 — 확정 아님. 좌클릭으로 추가, 우클릭으로 완료"` (`MainWindowOverlay.cpp:415`).
   - Seeded `0`: `"고지형 가설 — 진한 색이 입지 후보. 좌클릭으로 그리고 우클릭으로 완료"` (`MainWindowOverlay.cpp:430`).
6. **Second click when layer is there**: Deletes previously generated `"자동:"` features while preserving user-edited features (`!note.startsWith("자동:")`, `PaleoLandformService.cpp:175-185`), re-seeds `"자동:"` hypotheses from the current soil layer, and re-enters polygon edit mode (`MainWindowOverlay.cpp:400-402`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `8-A1` | `click` | Ribbon group (`배경 지도`) | `SAME` | `gui_ribbon_consts.rs:28, 32` — placed in `배경 지도`. |
| `8-A2` | `click` | Left-click target view & pre-checks | `DIFFERENT` | `gui_ribbon_click_b.rs:45` — calls `self.open_studio_tab(MainTab::Paleo)` with zero survey/download checks, leaving the Map tab instead of running soil-based landform seeding on the map. |
| `8-A3` | `click` | `"고지형"` studio tab 3D surface & borehole table | `FAKE` | `gui_tab_paleo.rs:20-37`, `tabs_paleo_3d.rs:52, 58-248` — `Paleo3dState::default()` calls `run_sample_10km_analysis()` (`tabs_paleo_3d.rs:52`), generating a mathematical sine/cosine heightfield (`raw_surface_z` at `tabs_paleo_3d.rs:124-136`) and 8 hardcoded boreholes `BH-01`..`BH-08` (`tabs_paleo_3d.rs:64-73`). |
| `8-B0` | `menu row` | Right-click menu presence | `DIFFERENT` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:28`, `gui_ribbon_menus_a.rs:12-53` — C++ has no menu on `고지형`; Rust attaches a 7-row right-click menu. |
| `8-BX1` | `menu row` | Extra Rust Row 1 `"「고지형분석」 전용 탭 열기"` | `DIFFERENT` | `gui_ribbon_menus_a.rs:22`, `gui_ribbon_menu_actions.rs:39-40` — switches to `MainTab::Paleo`; not in C++. |
| `8-BX2` | `menu row` | Extra Rust Rows 2–5 (`"수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 창…"`, `"수치지질도(KIGAM 1:5만) 별도 받기 (반경 5km · 도면 범례)"`, `"국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km · 도면 범례)"`, `"KIGAM 단층·지하수위·단구연대 별도 받기 (반경 10km)"`) | `FAKE` | `gui_ribbon_menus_a.rs:23-26`, `gui_ribbon_menu_actions.rs:41-48`, `modal_downloads.rs:18-129` — opens `ActiveModalDialog::PaleoSourceDownload`, where every download button calls synthetic `run_sample_10km_analysis()` (`modal_downloads.rs:83, 98, 109, 120`) and prints `"적재했습니다"` / `"받았습니다"` without network or file I/O. |
| `8-BX3` | `menu row` | Extra Rust Rows 6–7 (`"국토지반정보 포털 열기 (geoinfo.or.kr)…"`, `"KIGAM 지오빅데이터 포털 열기 (data.kigam.re.kr)…"`) | `FAKE` | `gui_ribbon_menus_a.rs:27-28`, `gui_ribbon_menu_actions.rs:49-52` — sets `draw_guide_message = Some("... — 브라우저에서 이어서 받으세요.")` without launching a browser or opening any URL. |
| `8-C1` | `post-processing` | Soil candidate emphasis (`04, 05, 06, 08` `"입지후보 · ..."`) | `MISSING` | `none` — not implemented in Rust. |
| `8-C2` | `post-processing` | Soil-to-landform hypothesis seeding (`구하도`/`자연제방`/`선상지`/`하안단구`/`해성평탄`/`미저지`) & `paleo_landform` GPKG table | `MISSING` | `none` — not implemented in Rust. |
| `8-C3` | `layer` | Editable `"고지형 판독"` vector layer & polygon edit mode | `MISSING` | `none` — not implemented in Rust. |
| `8-C4` | `click` | Second click when `"고지형 판독"` is present | `DIFFERENT` | `gui_ribbon_click_b.rs:45` — switches to `MainTab::Paleo` instead of refreshing `"자동:"` hypotheses on the map. |

---

## 9. `지질` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_actGeology` / `geologyBtn` in group `basemap` (`배경 지도`), text `"지질"` (action text `"지질도"`, objectName `"actDownloadGeology"`), checkable (`setCheckable(true)`), tooltip `"현재 화면 범위의 1:5만 지질도를 KIGAM에서 내려받아 「참조 지도」에 추가합니다"`, connects `triggered` to `MainWindow::downloadGeologyMap` (`MainWindowRibbon.cpp:501-507, 515-520`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-592`).
- **Conditions checked in order inside `MainWindow::downloadGeologyMap()` (`MainWindowOverlay.cpp:492-546`)**:
  1. `if (!m_canvas) return;` — silent early return (`MainWindowOverlay.cpp:494`).
  2. Calls `toggleExistingOverlay(..., "지질도(KIGAM 1:5만)", m_actGeology, statusBar())` (`MainWindowOverlay.cpp:495-508`): **if found**, toggles `"지질도(KIGAM 1:5만)"`, syncs companion `"지질 음영"` (`GeologyMapService::ensureReliefUnderlay` when turning on, `toggleLayerVisibility(..., false)` when turning off), updates `m_actGeology->setChecked(...)`, and shows `"지질도(KIGAM 1:5만)를 껐습니다."` or `"지질도(KIGAM 1:5만)를 다시 켰습니다."` (4,000 ms, `MainWindowOverlay.cpp:227, 232`).
  3. **If not found**: checks `referenceDownloadFolder("지질도")` (`MainWindowOverlay.cpp:509-513`), clamps scale `<= 1:100,000` (`"축척을 1:100000으로 맞춘 뒤 지질도를 받습니다."`, `MainWindowOverlay.cpp:514-515`), transforms canvas extent to `EPSG:5186` checking `<= 80,000 m` (`MainWindowOverlay.cpp:517-537`), and calls `startReferenceDownload(ReferenceMapKind::Geology, ext, "<dir>/지질도_KIGAM.gpkg")` (`MainWindowOverlay.cpp:539-541, 603-695`).
- **What it starts**: Opens cancelable `QProgressDialog` (`createDownloadProgress("지질도")`) and runs `GeologyMapService::prepare` -> `GeologyMapService::addPrepared` (`MainWindowOverlay.cpp:610-694`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:501-507, 515-520`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Current canvas extent in `EPSG:5186` (after clamping canvas scale `<= 1:100,000`, `MainWindowOverlay.cpp:514-515`), capped at **`maxSpanMeters() = 80,000 m`** (`GeologyMapService.cpp:740-749`), transformed to `EPSG:4326` (`GeologyMapService.cpp:751-760`). Polygons are not clipped at the bbox boundary (`GeologyMapService.cpp:846-894`).
2. **Post-processing**:
   - Selects WFS layer `geoOpen:l_50k_geology_litho_latest` (mainland) or `geoOpen:l_jeju_50k_geology_litho_view` (Jeju, `GeologyMapService.cpp:638-657`). If outside both boxes or WFS returns 0 features, falls back to official KIGAM WMS raster `geoOpen:L_50K_Geology_Map` (`prepareOfficialRaster` / `addOfficialGeologyRaster`, `GeologyMapService.cpp:659-667, 693-725, 762-766, 786-789`).
   - Otherwise fetches GeoJSON (splitting 4-way on 1,000-feature cap), strips HTML tags, computes `era_class`, reprojects to `EPSG:5186`, samples official WMS PNG pixel colours at `pointOnSurface()` of the largest polygon per `lithoidx` (`sampleOfficialColors`, `GeologyMapService.cpp:880-908`), and writes `지질도_KIGAM.gpkg` (`GeologyMapService.cpp:911`).
3. **Files and folders written**:
   - `<surveyDir>/ka-hgis-reference-XXXXXX/geology.geojson` (temporary, removed at `GeologyMapService.cpp:895`)
   - `<surveyDir>/ka-hgis-reference-XXXXXX/지질도_KIGAM.gpkg` (retained, table `geology_map`, `MainWindowOverlay.cpp:539`, `GeologyMapService.cpp:738, 911`)
4. **Layers added**:
   - `"지질도(KIGAM 1:5만)"` (`GeologyMapService.cpp:924-925`) — placed via `LayerOps::placeInLegendGroup` (`GeologyMapService.cpp:946`), stacked **below** `"지질 음영"`, categorized fill by `lithoidx` using sampled official WMS colours (or `era_class` fallback colours) + `lithoidx` labels, visibility capped at `<= 1:100,000` (`GeologyMapService.cpp:947`), **visible**, **read-only** (`markReferenceLayer`, `GeologyMapService.cpp:936`).
   - `"지질 음영"` (`GeologyMapService.cpp:559-636, 948`) — stacked directly **above** `"지질도(KIGAM 1:5만)"` (`stackOver`, `GeologyMapService.cpp:556, 634`), local DEM hillshade if an elevation raster covers the layer else ArcGIS World Hillshade XYZ (`z 1..16`, Multiply blend, opacity `0.48`, `ka_hgis/omit_sheet_legend = true`), visibility capped at `<= 1:100,000`, **visible**, **read-only**.
5. **Status texts**:
   - Cancel: `"지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."` (`MainWindowOverlay.cpp:635`).
   - Done: `"지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."` (`MainWindowOverlay.cpp:672`).
   - Toggle off / on: `"지질도(KIGAM 1:5만)를 껐습니다."` / `"지질도(KIGAM 1:5만)를 다시 켰습니다."` (`MainWindowOverlay.cpp:227, 232`).
6. **Second click when layer is there**: Toggles **both** `"지질도(KIGAM 1:5만)"` and `"지질 음영"` off without re-downloading, unchecks `m_actGeology`, and shows `"지질도(KIGAM 1:5만)를 껐습니다."` (`MainWindowOverlay.cpp:495-508`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `9-A1` | `click` | Ribbon group (`배경 지도`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28, 33`, `gui_ribbon_click_b.rs:33`, `ref_map_run.rs:108` — in `배경 지도`, switches `self.active_tab = MainTab::Map`. |
| `9-A2` | `click` | Checkable button active highlight when `"지질도(KIGAM 1:5만)"` is visible | `MISSING` | `gui_ribbon_top.rs:140-156` — neither `dark_active` nor `warm_active` checks `"지질도(KIGAM 1:5만)"`. |
| `9-A3` | `click` | Pre-checks on first click | `DIFFERENT` | `ref_map_run.rs:115-154`, `ref_map.rs:82-103` — checks busy reference download and open `survey_path`, plus requires drawn `survey_area` and `online` (instead of canvas extent `<= 80 km`). |
| `9-B0` | `menu row` | Right-click menu presence | `DIFFERENT` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menu_actions.rs:28`, `gui_ribbon_menus_a.rs:54-91` — C++ has no menu on `지질`; Rust attaches a 6-row right-click menu. |
| `9-BX1` | `menu row` | Extra Rust Rows 1–3 (`"조사구역 반경 5km 수치지질도 받기 (1:5만 · 권장)"`, `"조사구역 반경 2km 수치지질도 받기"`, `"조사구역 반경 10km 수치지질도 받기"`) | `FAKE` | `gui_ribbon_menus_a.rs:63-65`, `gui_ribbon_menu_actions.rs:54-76` — builds an unused `geology_download_spec` struct and sets `draw_guide_message = Some("수치지질도 (1:50,000) — 반경 N km 기준으로 켰습니다.")` (`gui_ribbon_menu_actions.rs:73-75`) without starting any download or adding any layer. |
| `9-BX2` | `menu row` | Extra Rust Rows 4–6 (`"국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km)"`, `"KIGAM 단층·선구조·수문 별도 받기 (반경 10km)"`, `"수치지질도 · 시추주상도 별도 내려받기 창…"`) | `FAKE` | `gui_ribbon_menus_a.rs:66-68`, `gui_ribbon_menu_actions.rs:77-78`, `modal_downloads.rs:18-129` — opens `ActiveModalDialog::PaleoSourceDownload`, where every download button calls synthetic `run_sample_10km_analysis()` (`modal_downloads.rs:83, 98, 109, 120`). |
| `9-C1` | `post-processing` | Scope (10 km `survey_area` buffer), WFS + WMS colour sampling, polygon clipping, & files written | `DIFFERENT` | `ref_map.rs:82-103, 218-241`, `crates/strata-refmap/src/geology_prepare.rs:109-226` — same KIGAM WFS layers, 4-quadrant split, HTML stripping, `era_class`, and WMS PNG `point_on_surface` colour sampling, except Rust fetches and **clips polygons** to a **10 km buffer around `survey_area`** (owner decision 2026-11-15) and writes `<surveyDir>/ka-hgis-reference-XXXXXX/지질도_KIGAM.gpkg` + `ka-hgis-scope.json`. |
| `9-C2` | `post-processing` | Official KIGAM WMS raster fallback when WFS has 0 features or outside WFS bbox | `MISSING` | `crates/strata-refmap/src/geology_prepare.rs:124-129, 146-148` — returns an error string instead of adding a live KIGAM WMS raster layer. |
| `9-C3` | `layer` | Layer `"지질도(KIGAM 1:5만)"` name, sampled/era colours & `lithoidx` labels, visible, read-only | `SAME` | `ref_map.rs:84-95`, `ref_vector_show.rs:23`, `crates/strata-refmap/src/geology_style.rs` — verbatim `"지질도(KIGAM 1:5만)"`, sampled official / era fallback fill, `lithoidx` labels, visible, read-only. |
| `9-C4` | `layer` | Companion `"지질 음영"` Multiply relief layer & `1:100,000` scale cap | `MISSING` | `gui_ribbon_click_b.rs:31-32` (`none`) — `"지질 음영"` companion layer and the `1:100,000` visibility cap are not added (noted as deferred `NET-GEO1c`). |
| `9-C5` | `layer` | Verbatim status bar texts on add, cancel, & toggle | `SAME` | `ref_map.rs:100, 102`, `ref_map_run.rs:112, 224-229` — verbatim `"지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."`, `"지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."`, `"지질도(KIGAM 1:5만)를 껐습니다."`, `"지질도(KIGAM 1:5만)를 다시 켰습니다."`. |
| `9-C6` | `click` | Second click when `"지질도(KIGAM 1:5만)"` is in legend | `DIFFERENT` | `ref_map_run.rs:109-114` — toggles `"지질도(KIGAM 1:5만)"` visibility without re-downloading, but has no companion `"지질 음영"` to toggle in tandem. |

---

## 10. `수계` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `m_actRiver` / `riverBtn` in group `basemap` (`배경 지도`), text `"수계"` (action text `"수계도"`, objectName `"actDownloadRiver"`), checkable (`setCheckable(true)`), tooltip `"현재 화면 범위의 하천망(국가·지방하천)을 내려받아 「참조 지도」에 추가합니다"`, connects `triggered` to `MainWindow::downloadRiverMap` (`MainWindowRibbon.cpp:508-520`). `showMapTabFromStudio()` runs first (`MainWindowRibbon.cpp:584-592`).
- **Conditions checked in order inside `MainWindow::downloadRiverMap()` (`MainWindowOverlay.cpp:550-601`)**:
  1. `if (!m_canvas) return;` — silent early return (`MainWindowOverlay.cpp:552`).
  2. Calls `toggleExistingOverlay(..., "수계도(하천망)", m_actRiver, statusBar())` (`MainWindowOverlay.cpp:553-555`): **if found**, toggles visibility, updates `m_actRiver->setChecked(...)`, and shows `"수계도(하천망)를 껐습니다."` or `"수계도(하천망)를 다시 켰습니다."` (4,000 ms, `MainWindowOverlay.cpp:227, 232`).
  3. Checks `referenceDownloadFolder("수계도")` (`MainWindowOverlay.cpp:556-560`) and clamps canvas scale `<= 1:100,000` (`"축척을 1:100000으로 맞춘 뒤 수계도를 받습니다."`, `MainWindowOverlay.cpp:561-562`).
  4. Checks `VworldSettings::loadApiKey().trimmed().isEmpty()`; if empty, calls `notify(Notice::Warning, "수계도", "VWorld 인증키가 없습니다. 배경지도 설정에서 키를 먼저 등록하세요.")` and returns (`MainWindowOverlay.cpp:564-570`).
  5. Transforms canvas extent to `EPSG:5186` checking `<= RiverMapService::maxSpanMeters()` (`160,000 m`), then calls `startReferenceDownload(ReferenceMapKind::River, ext, "<dir>/수계도_VWorld.gpkg", key)` (`MainWindowOverlay.cpp:572-596, 603-695`).
- **What it starts**: Opens cancelable `QProgressDialog` (`createDownloadProgress("수계도")`) and runs `RiverMapService::prepare` -> `RiverMapService::addPrepared` (`MainWindowOverlay.cpp:610-694`).

### Part B — C++ Menu
- **Menu**: **None** — no `QMenu`, no `setPopupMode`, no `CustomContextMenu`, and no arrow (`MainWindowRibbon.cpp:508-520`).

### Part C — C++ from Download to Legend
1. **Area asked for**: Current canvas extent in `EPSG:5186` (after clamping scale `<= 1:100,000`, `MainWindowOverlay.cpp:561-562`), capped at **`maxSpanMeters() = 160,000 m`** (`RiverMapService.h:43`, `RiverMapService.cpp:283-292`), transformed to `EPSG:4326` (`RiverMapService.cpp:294-303`). Polygons are not clipped at the bbox edge (`RiverMapService.cpp:342-365`).
2. **Post-processing**: Queries VWorld WFS `https://api.vworld.kr/req/wfs` (`TYPENAME=lt_c_wkmstrm`, `SRSNAME=EPSG:4326`, `Referer: https://localhost`, up to 10 pages × 1,000 features, `RiverMapService.cpp:307-314`), reprojects polygons from `EPSG:4326` to `EPSG:5186` (`MultiPolygon`), writes `수계도_VWorld.gpkg` (`RiverMapService.cpp:324-378`), and deletes temporary `river.geojson` (`RiverMapService.cpp:366`).
3. **Files and folders written**:
   - `<surveyDir>/ka-hgis-reference-XXXXXX/river.geojson` (temporary, removed at `RiverMapService.cpp:318, 366`)
   - `<surveyDir>/ka-hgis-reference-XXXXXX/수계도_VWorld.gpkg` (retained, table `river_map`, `MainWindowOverlay.cpp:594`, `RiverMapService.cpp:276, 377`)
4. **Layers added**:
   - `"수계도(하천망)"` (`RiverMapService.cpp:397-398`) — placed via `LayerOps::placeInLegendGroup` (`RiverMapService.cpp:420`), categorized water-blue fill by `riv_level` (`국가하천`, `지방1급`, `지방2급`, fallback `RGB(120,196,236)`) + bold 8 pt Malgun Gothic `#0048ba` `riv_nm` labels with 0.7 pt white halo (`RiverMapService.cpp:192-261`), visibility capped at `<= 1:100,000` (`RiverMapService.cpp:421`), **visible**, **read-only** (`markReferenceLayer`, `RiverMapService.cpp:409`).
5. **Status texts**:
   - Cancel: `"수계도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."` (`MainWindowOverlay.cpp:635`).
   - Done: `"수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."` (`MainWindowOverlay.cpp:672`).
   - Toggle off / on: `"수계도(하천망)를 껐습니다."` / `"수계도(하천망)를 다시 켰습니다."` (`MainWindowOverlay.cpp:227, 232`).
6. **Second click when layer is there**: Toggles `"수계도(하천망)"` visibility **off** without re-downloading, unchecks `m_actRiver`, and shows `"수계도(하천망)를 껐습니다."` (`MainWindowOverlay.cpp:553-555`).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `10-A1` | `click` | Ribbon group (`배경 지도`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28, 34`, `gui_ribbon_click_b.rs:36`, `ref_map_run.rs:108` — in `배경 지도`, switches `self.active_tab = MainTab::Map`. |
| `10-A2` | `click` | Checkable button active highlight when `"수계도(하천망)"` is visible | `MISSING` | `gui_ribbon_top.rs:140-156` — neither `dark_active` nor `warm_active` checks `"수계도(하천망)"`. |
| `10-A3` | `click` | Pre-checks on first click (busy download, `survey_path`, `survey_area`, VWorld key, `online`) | `DIFFERENT` | `ref_map_run.rs:115-154`, `ref_map.rs:131-155` — checks busy download, open `survey_path`, drawn `survey_area`, VWorld API key (showing verbatim `NO_VWORLD_KEY` + recording `StopCode::Key1` on the gauge at `ref_map_run.rs:144-147` instead of `Notice::Warning`), and `online` (instead of canvas extent `<= 160 km`). |
| `10-B0` | `menu row` | Menu (none) | `SAME` | `gui_ribbon_menu_actions.rs:22-35` — no right-click or arrow menu. |
| `10-C1` | `post-processing` | Scope (10 km `survey_area` buffer), VWorld WFS `lt_c_wkmstrm` paging, polygon clipping, & files written | `DIFFERENT` | `ref_map.rs:131-152, 218-241`, `crates/strata-refmap/src/river_prepare.rs:77-219` — same VWorld WFS `lt_c_wkmstrm`, `Referer: https://localhost`, up to 10 pages × 1,000, and `EPSG:5186` output table `river_map`, except Rust fetches and **clips polygons** to a **10 km buffer around `survey_area`** (owner decision 2026-11-15) instead of 160 km unclipped canvas extent, and writes `<surveyDir>/ka-hgis-reference-XXXXXX/수계도_VWorld.gpkg` + `ka-hgis-scope.json`. |
| `10-C2` | `layer` | Layer `"수계도(하천망)"` style, labels, visible, read-only, & scale cap | `DIFFERENT` | `ref_map.rs:133-143`, `ref_vector_show.rs:23`, `crates/strata-refmap/src/river_style.rs` — verbatim `"수계도(하천망)"`, `riv_level` blue fill, `riv_nm` bold blue halo labels, visible, and read-only match, except Rust omits C++'s `1:100,000` layer visibility cap (`RiverMapService.cpp:421`). |
| `10-C3` | `layer` | Verbatim status bar texts on add, cancel, & toggle | `SAME` | `ref_map.rs:149, 151`, `ref_map_run.rs:112, 224-229` — verbatim `"수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."`, `"수계도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."`, `"수계도(하천망)를 껐습니다."`, `"수계도(하천망)를 다시 켰습니다."`. |
| `10-C4` | `click` | Second click when `"수계도(하천망)"` is in legend | `SAME` | `ref_map_run.rs:109-114` — toggles `"수계도(하천망)"` visibility without re-downloading. |

---

## 11. `옛 지도` (Group: `배경 지도`)

### Part A — C++ Left Click
- **Ribbon wiring**: `oldMaps` in group `basemap` (`배경 지도`), text `"옛 지도"`, non-checkable, `setPopupMode(QToolButton::InstantPopup)`, tooltip `"대동여지도·1919 조선지형도를 배경으로 올립니다"`, `setMenu(oldMenu)` (`MainWindowRibbon.cpp:488-500, 521`).
- **Conditions checked in order**: None on the button click itself — `QToolButton::InstantPopup` immediately opens `oldMenu` (`MainWindowRibbon.cpp:490`). When either menu action is triggered, `showMapTabFromStudio()` switches to the Map tab first (`MainWindowRibbon.cpp:591-592`).
- **What it starts**: Opens the 2-item popup menu `oldMenu` (`MainWindowRibbon.cpp:492-500`).

### Part B — C++ Menu
- **How it opens**: **Left click only** (`QToolButton::InstantPopup` at `MainWindowRibbon.cpp:490` with no `Qt::CustomContextMenu`, and menu arrow hidden at `0px` by `ka-hgis.qss:379-393`).
- **Rows in order** (`MainWindowRibbon.cpp:492-500`):
  1. `"대동여지도"` — calls `MainWindow::addDaedongyeojidoMap` (`MainWindowRibbon.cpp:494`, `MainWindowOffline.cpp:456-465`). Always enabled, not checkable.
  2. `"1919 조선지형도 1:5만"` (`actionMap1919`) — calls `MainWindow::addHistoryGisMap1919` (`MainWindowRibbon.cpp:498`, `MainWindowOffline.cpp:467-477`). Not checkable; **enabled** when `hasHistoryKey` is true (tooltip `"1919년 조선지형도 1:5만 (국사편찬위원회 WMTS, EPSG:5179)"`) and **disabled** when no key is stored (tooltip `"더보기 → API 키 입력에 역사지리정보DB 키를 저장하면 켤 수 있습니다"`, `MainWindow.cpp:1426-1435`).

### Part C — C++ from Download to Legend
1. **Area asked for**:
   - `대동여지도`: Live NGII WMS 1.1.1 over the **entire map canvas** (`EPSG:5179` / `EPSG:5186` / `EPSG:3857`, no `survey_area` required); `afterBasemapAdded` zooms canvas to **`1:25,000`** when scale is `> 1:80,000` or `< 1:100` (`MainWindowOffline.cpp:365-366, 464`).
   - `1919 조선지형도 1:5만`: Live NIKH WMTS (`EPSG:5179`) over the **entire map canvas**; `afterBasemapAdded` zooms canvas to **`1:25,000`** when scale is `> 1:80,000` or `< 1:100` (`MainWindowOffline.cpp:365-366, 475`).
2. **Post-processing**:
   - `대동여지도`: Tries WMS 1.1.1 layers `korea_oldmap_ddymap_kyu`, then `korea_oldmap_addAlphaChannel` across `EPSG:5179` / `EPSG:5186` / `EPSG:3857` on `https://map.ngii.go.kr/spcemapserver/korea_old_map/ows` (`image/png`, `BasemapOps.cpp:1535-1575`).
   - `1919 조선지형도 1:5만`: Checks `VworldSettings::loadHistoryGisApiKey()` (`BasemapOps.cpp:1530-1532`: if empty, returns error `"1919 조선지형도는 국사편찬위원회 역사지리정보DB API 키가 필요합니다. 더보기 → API 키 입력에서 키를 저장하세요. https://hgis.history.go.kr/api/intro.do"`) and opens NIKH `EPSG:5179` WMTS raster (`BasemapOps.cpp:1486-1534`, `MainWindowOffline.cpp:467-477`).
3. **Files and folders written**: None in survey folder or app folders (live WMS / WMTS streams).
4. **Layers added**:
   - `"대동여지도"` (`BasemapOps.cpp:1563`) — placed at **root layer tree bottom** below survey layers and below already-loaded basemap rasters (`placeBasemapBelowVectors`, `BasemapOps.cpp:945-967, 1567`), live WMS raster, **visible**, **read-only** (`ka_hgis/basemap = true`, `BasemapOps.cpp:1566`).
   - `"1919 조선지형도 1:5만"` (`BasemapOps.cpp:1505`, `MainWindowOffline.cpp:475`) — placed at **root layer tree bottom** below survey layers and below already-loaded basemap rasters, live WMTS raster, **visible**, **read-only**.
5. **Status texts** (`afterBasemapAdded`, `MainWindowOffline.cpp:369, 464, 475`):
   - `대동여지도`: `"대동여지도을 올렸습니다. 「그리기」로 구역을 그리세요."`
   - `1919 조선지형도 1:5만`: `"1919 조선지형도을 올렸습니다. 「그리기」로 구역을 그리세요."`
6. **Second click when layer is there**: Selecting `"대동여지도"` or `"1919 조선지형도 1:5만"` a second time calls `removeExistingBasemap(project, layerName)` (`BasemapOps.cpp:884-895, 1536`), removing and recreating that raster layer (does **not** toggle it off).

### Part D — Rust Today
| ID | Where | Item | Status | Rust (`<file name>:line`) & Comparison |
|---|---|---|---|---|
| `11-A1` | `click` | Ribbon group (`배경 지도`) & Map-tab switch | `SAME` | `gui_ribbon_consts.rs:28, 35`, `gui_ribbon_click_b.rs:42` — in `배경 지도`, switches to `MainTab::Map`. |
| `11-A2` | `click` | Left-click action & chip active highlight | `DIFFERENT` | `gui_ribbon_top.rs:155, 167-169`, `gui_ribbon_click_b.rs:37-43`, `ref_tile_history.rs:51-53` — instead of opening `oldMenu`, left-click directly calls `toggle_reference_tile_layer(&DAEDONGYEOJIDO)` (owner decision 2026-11-15: `「이것은 위의 ui버튼이 눌려야 나오게하라」`), and `dark_active` highlights `"옛 지도"` whenever `"대동여지도"` or `"1919 조선지형도 1:5만"` is shown. |
| `11-B0` | `menu row` | Menu trigger (left click vs right click; no arrow) | `DIFFERENT` | `gui_ribbon_top.rs:167-168, 174-175`, `gui_ribbon_draw.rs:64-218` — no arrow in either, but C++ opens `oldMenu` on **left click only**, whereas Rust opens `ka_old_map_menu` on **right click only**. |
| `11-B1` | `menu row` | Row 1 `"대동여지도"` label & dispatch | `DIFFERENT` | `gui_ribbon_menus_a.rs:227-228, 241-243` — always enabled and unchecked, calls `add_daedongyeojido()`, except Rust prefixes the label with `"▤  "` (`"▤  대동여지도"`). |
| `11-B2` | `menu row` | Row 2 `"1919 조선지형도 1:5만"` label, enabled state, & dispatch | `DIFFERENT` | `gui_ribbon_menus_a.rs:229-236, 241-244`, `ref_tile_history.rs:14-43` — prefixed `"▤  1919 조선지형도 1:5만"`, always enabled (with hover tooltip `"1919년 조선지형도 1:5만 (국사편찬위원회 WMTS, EPSG:5179)"`), and calls `add_history_map_1919()`, checking `load_history_gis_api_key()` on click and showing verbatim `NO_KEY` on the status bar when empty (`ref_tile_history.rs:15, 37-39`), whereas C++ disables the row when no key is stored (`MainWindow.cpp:1426-1435`). |
| `11-C1` | `post-processing` | `대동여지도` WMS tile requests, 100 km circle scope, & framing | `DIFFERENT` | `basemap_tiles.rs:50-67`, `ref_tile_layers.rs:74-83, 141-165`, `ref_tile_paint.rs:118-216` — requires drawn `survey_area`, requests 256×256 `EPSG:3857` WMS 1.1.1 tiles (`korea_oldmap_ddymap_kyu`, `z <= 12`, no local files) clipped/feathered to a **100 km circle** around `survey_area` centre, and frames `r * 1.1` (`ViewOnAdd::FrameCircle`, owner decision 2026-11-15). |
| `11-C2` | `layer` | `대동여지도` layer name (`"대동여지도"`), visible, read-only | `SAME` | `ref_tile_layers.rs:76, 148-149` — verbatim `"대동여지도"`, visible, read-only tile row. |
| `11-C3` | `layer` | `대동여지도` stacking order & status bar text | `DIFFERENT` | `ref_tile_layers.rs:80-81, 105-131` — inserts `"대동여지도"` **above** older tile rows and shows `"대동여지도를 올렸습니다. 조사구역에서 반경 100km 안에 보입니다."` (vs C++ below existing basemaps and `"대동여지도을 올렸습니다. 「그리기」로 구역을 그리세요."`). |
| `11-C4` | `click` | Second selection of `대동여지도` when already in legend | `DIFFERENT` | `gui_ribbon_click_b.rs:43`, `ref_tile_layers.rs:136-155, 178-189` — left-click toggles `"대동여지도"` visibility (`"대동여지도를 껐습니다."` / `"대동여지도를 다시 켰습니다."`), and right-click menu Row 1 keeps the existing row (`ensure_row`) and re-frames the 100 km circle, instead of removing and recreating the WMS layer. |
| `11-C5` | `layer` | `"1919 조선지형도 1:5만"` WMTS scope, stacking order, & status text | `DIFFERENT` | `ref_tile_history.rs:18-43`, `basemap_tiles.rs:70-137`, `ref_tile_layers.rs:105-165` — adds `"1919 조선지형도 1:5만"` (`TileKind::History1919`, `EPSG:5179` NIKH WMTS `max_zoom: 16`) clipped/feathered to a **10 km circle** around `survey_area` centre (`ViewOnAdd::FrameCircle`, owner decision 2026-11-15), stacked above older tile rows, with status `"1919 조선지형도를 올렸습니다. 조사구역에서 반경 10km 안에 보입니다."` (vs full-canvas WMTS below basemaps + `1:25,000` zoom + `"1919 조선지형도을 올렸습니다. 「그리기」로 구역을 그리세요."` in C++). |

---

## All Differences (C++ vs Rust Today)

Every non-`SAME` item from Parts D above (`45 DIFFERENT` + `21 MISSING` + `11 FAKE` = **77 rows**), ordered by button:

| button | where | C++ (file:line) | Rust (file:line or `none`) | kind |
|---|---|---|---|---|
| `지적` | `click` (`1-A2`: pre-checks vs WMS add + modal open) | `MainWindowCadastral.cpp:82-173` | `gui_ribbon_click_a.rs:158-162`, `left_layer_panel.rs:55-73`, `cadastral_download_ui_render.rs:115-130` | `DIFFERENT` |
| `지적` | `menu row` (`1-B1`: `"VWorld 계정 설정"` dedicated vs shared modal) | `MainWindowRibbon.cpp:477`, `MainWindowCadastral.cpp:34-54` | `gui_ribbon_menus_a.rs:163, 180-181` | `DIFFERENT` |
| `지적` | `menu row` (`1-B2`: `"선 색·지번 표시"` style dialog) | `MainWindowRibbon.cpp:478`, `MainWindowCadastral.cpp:56-80` | `gui_ribbon_menus_a.rs:164, 182-184` (`none` for style dialog) | `MISSING` |
| `지적` | `post-processing` (`1-C1`: 5 km polygon buffer + `LT_C_ADSIGG_INFO` vs bbox ± 5 km) | `CadastralPortal.cpp:222-262` | `cadastral_download_ui_worker.rs:161-187`, `cadastral_download_ui_paths.rs:48-84` | `DIFFERENT` |
| `지적` | `post-processing` (`1-C2`: VWorld login `/v4po_usrlogin_a004.do`) | `CadastralPortal.cpp:264-292` | `cadastral_download_ui_worker.rs:93-117` | `MISSING` |
| `지적` | `post-processing` (`1-C3`: raw ZIP naming & `"PK\x03\x04"` cache check) | `CadastralPortal.cpp:299-350` | `cadastral_download_ui_paths.rs:28-33`, `cadastral_download_ui_worker.rs:124-142` | `DIFFERENT` |
| `지적` | `post-processing` (`1-C4`: SHP extract, CP949 decode, 5 km parcel clip, `<sha256>.gpkg`) | `CadastralPortal.cpp:317-329`, `CadastralImport.cpp:116-241` | `none` (`cadastral_download_ui_worker.rs:144-158` stops after `.zip`) | `MISSING` |
| `지적` | `layer` (`1-C5`: `"지적도 · 조사 주변 5km"` vector layer vs status-only message) | `CadastralImport.cpp:244-299`, `MainWindowCadastral.cpp:168` | `cadastral_download_ui_state.rs:106-117`, `cadastral_download_ui_paths.rs:41-45` | `FAKE` |
| `지적` | `click` (`1-C6`: second click reuses `<sha256>.gpkg` vs re-opens modal) | `MainWindowCadastral.cpp:82-174`, `CadastralImport.cpp:134-141, 275-282` | `gui_ribbon_click_a.rs:158-162` | `DIFFERENT` |
| `수치지형` | `click` (`2-A2`: visible `KaTopographicBrowser` vs hidden WebView2 + `[1/6]`–`[6/6]` gauge) | `MainWindowTopographic.cpp:18-72` | `topo_download_run.rs:84-111`, `topo_download.rs:101-144, 163-195` | `DIFFERENT` |
| `수치지형` | `menu row` (`2-B2`: `"받아 둔 수치지형도 폴더…"` confirm adds no layer) | `MainWindowRibbon.cpp:392`, `MainWindowTopographic.cpp:102-120`, `KaTopographicImportDialog.cpp:892-995` | `gui_ribbon_menus_a.rs:196, 216-218`, `dlg_topo_import_actions.rs:64-97` | `FAKE` |
| `수치지형` | `post-processing` (`2-C1`: canvas-centre 5 km + line clip vs `survey_area`-centre 5 km + touch filter) | `KaTopographicScopePanel.cpp:38, 507-523`, `KaTopographicImportDialog.cpp:721-726, 814` | `topo_download.rs:17, 106-110`, `crates/strata-topo/src/merge.rs:62-105` | `DIFFERENT` |
| `수치지형` | `post-processing` (`2-C2`: `files/`+`catalog/`+`SHP/`+`index/`+`receipts/` vs `strata-sheets.json`) | `KaTopographicScopePanel.cpp:495-496`, `KaTopographicImportDialog.cpp:919-937` | `topo_download.rs:36`, `crates/strata-topo/src/library.rs:19`, `crates/strata-topo/src/post.rs:25-26, 109-110` | `DIFFERENT` |
| `수치지형` | `click` (`2-C5`: second click raises browser vs re-runs cached order + re-merges) | `MainWindowTopographic.cpp:62-70` | `topo_download_run.rs:84-111`, `topo_download.rs:133-134`, `crates/strata-topo/src/post.rs:81-115` | `DIFFERENT` |
| `DEM` | `click` (`3-A1`: ribbon group `배경 지도` vs `자료 받기`) | `MainWindowRibbon.cpp:427` | `gui_ribbon_consts.rs:22-24` | `DIFFERENT` |
| `DEM` | `click` (`3-A3`: Copernicus GLO-30 HTTPS vs NGII `.img` hidden WebView2 order) | `MainWindowOverlay.cpp:337`, `MainWindowDownloads.cpp:118-160` | `gui_ribbon_click_a.rs:155`, `topo_download_run.rs:90-111`, `topo_download.rs:101-161` | `DIFFERENT` |
| `DEM` | `menu row` (`3-BX`: extra row `"국토지리정보원 계정 설정·자동 다운로드…"`) | `none` | `gui_ribbon_menus_a.rs:101`, `gui_ribbon_menu_actions.rs:81-82` | `DIFFERENT` |
| `DEM` | `menu row` (`3-B1`: `"국토지리원 DEM 불러오기(.img)…"` status-bar open error vs `Notice::Warning`) | `MainWindowRibbon.cpp:412`, `MainWindow.cpp:1686-1705`, `BasemapOps.cpp:1996-2037` | `gui_ribbon_menus_a.rs:102`, `gui_ribbon_menu_actions.rs:83-85`, `dem_import.rs:15-56` | `DIFFERENT` |
| `DEM` | `menu row` (`3-B2`: `"DEM 파일로 음영 만들기…"` opens `"DEM 표현"` instead of GeoTIFF hillshade) | `MainWindowRibbon.cpp:414`, `MainWindow.cpp:1707-1774`, `DemHillshadeFile.cpp:18-138` | `gui_ribbon_menus_a.rs:103`, `gui_ribbon_menu_actions.rs:87-89` (`none` for file hillshade) | `MISSING` |
| `DEM` | `menu row` (`3-B3`: `"DEM 표현…"` status-bar missing-DEM text & live-apply vs modal box & `"적용"` button) | `MainWindowRibbon.cpp:416`, `MainWindowOverlay.cpp:269-297`, `KaDemClassDialog.cpp:19-132` | `gui_ribbon_menus_a.rs:104`, `dem_look.rs:117-170`, `modal_style_demclass.rs:28-95` | `DIFFERENT` |
| `DEM` | `menu row` (`3-B4`: `"측량점으로 등고선…"` adds empty `"단면선"` + fake message) | `MainWindowRibbon.cpp:419`, `MainWindowSurveyContour.cpp:104-197` | `gui_ribbon_menus_a.rs:105`, `gui_ribbon_menu_actions.rs:90-93`, `dlg_survey_contour.rs:184-198` | `FAKE` |
| `DEM` | `post-processing` (`3-C1`: Copernicus `DEM.tif` vs NGII `DEM/원본`, `DEM/풀림`, `strata-dem.json`) | `MainWindowDownloads.cpp:131`, `BasemapOps.cpp:1996-2125` | `dem_post.rs:31-45, 79-219` | `DIFFERENT` |
| `DEM` | `layer` (`3-C2`: `"지표모델(DSM) 30m · Copernicus"` + `"지형 음영"` vs `"DEM"` + `"지형 음영"` + sheet-count status) | `MainWindowDownloads.cpp:147, 157`, `BasemapOps.cpp:1996-2090` | `dem_post.rs:18, 20, 141-152, 217`, `left_layer_panel.rs:93-122` | `DIFFERENT` |
| `DEM` | `click` (`3-C4`: toggle omits Copernicus out-of-coverage re-download check) | `MainWindowOverlay.cpp:307-335` | `gui_ribbon_click_a.rs:108-151` | `DIFFERENT` |
| `유산` | `click` (`4-A2`: `survey_area`/`survey_path` pre-checks & VWorld 5 km municipality resolver) | `MainWindow.cpp:3071-3121`, `HeritageRegionResolver.cpp:221-352`, `HeritageNearbyRegions.cpp:49-63, 186-236` | `gui_ribbon_click_a.rs:165-168` (`none` for resolver) | `MISSING` |
| `유산` | `click` (`4-A3`: `"주변유적 받을 시·군 선택"` dummy backdrop buttons & unverified reuse/account logic) | `KaHeritageSetupDialogs.cpp:81-150`, `MainWindow.cpp:3153-3166` | `dlg_heritage_region_select.rs:24-75, 140-157` | `FAKE` |
| `유산` | `click` (`4-A4`: `"주변유적 받기"` 7 mismatched dataset checkboxes & disabled `"받기"` `지시 WB1`) | `KaHeritageBrowser.cpp:204-215`, `HeritageStyle.cpp:26-33` | `dlg_heritage_browser.rs:127-135`, `dlg_heritage_browser_footer.rs:13, 77-78` | `DIFFERENT` |
| `유산` | `post-processing` (`4-C1`: intranet WebEngine login, automatic pledge agreement, & `서약서-*.txt` receipt) | `MainWindow.cpp:3146-3149, 3228-3244`, `KaHeritageBrowser.cpp:420-443, 957-1000` | `none` | `MISSING` |
| `유산` | `post-processing` (`4-C2`: `<AppLocalDataLocation>/주변유적/...` ZIPs, UTF-8 GPKG conversion, & 5 km clip) | `MainWindow.cpp:3171-3186, 3201-3209`, `HeritageImport.cpp:46-125, 178-270`, `SurveyScopeClip.cpp:75-100` | `none` | `MISSING` |
| `유산` | `layer` (`4-C3`: `"참조 지도"` -> `"<datasetName>"` 6-dataset hollow layers & status texts) | `HeritageImport.cpp:280-335`, `HeritageStyle.cpp:26-74`, `MainWindow.cpp:3143, 3220-3223` | `none` | `MISSING` |
| `유산` | `click` (`4-C4`: second click re-resolves & offers recent-download reuse vs static modal) | `MainWindow.cpp:3071-3191`, `KaHeritageSetupDialogs.cpp:121-131`, `HeritageImport.cpp:285-296` | `gui_ribbon_click_a.rs:165-168` | `DIFFERENT` |
| `지형` | `click` (`5-A2`: requires drawn `survey_area` before adding `"지형맵"`) | `MainWindowOverlay.cpp:239-267` | `ref_tile_layers.rs:92, 141-147` | `DIFFERENT` |
| `지형` | `post-processing` (`5-C2`: 100 km circle clip/feather & `ViewOnAdd::Keep` vs full-world XYZ + `1:10,000` zoom) | `BasemapOps.cpp:969-970, 1577-1595` | `ref_tile_layers.rs:53-54, 91, 150-165`, `ref_tile_paint.rs:118-216` | `DIFFERENT` |
| `지형` | `layer` (`5-C4`: new tile row inserted above older tile rows vs below existing basemaps) | `BasemapOps.cpp:945-967` | `ref_tile_layers.rs:105-131` | `DIFFERENT` |
| `등고선` | `click` (`6-A2`: missing `survey_path` & running-job pre-checks) | `MainWindowSurveyContour.cpp:105-117` | `gui_ribbon_click_b.rs:22-25` | `MISSING` |
| `등고선` | `click` (`6-A3`: dead `"찾기…"` button & hardcoded `"점 248개, 표고 24.5–48.2 m... 의심점 3개 제외."` preview) | `MainWindowSurveyContour.cpp:118-120` | `dlg_survey_contour.rs:43, 153-156` | `FAKE` |
| `등고선` | `post-processing` (`6-C1`: point DXF/CSV/TXT/XLSX reader, outlier filter, TIN, & contour generation) | `MainWindowSurveyContour.cpp:68-90`, `SurveyContourBuilder.cpp`, `SurveyContourTin.cpp` | `none` | `MISSING` |
| `등고선` | `post-processing` (`6-C2`: `<surveyDir>/측량등고선/<stem>/surface.tif` & `contours.gpkg`) | `MainWindowSurveyContour.cpp:122-130, 164`, `SurveyContourWrite.cpp:52-57, 79-100` | `none` | `MISSING` |
| `등고선` | `layer` (`6-C3`: `"등고선 만들기"` adds empty `"단면선"` + fake `"표고점 248개를 생성했습니다."` message) | `SurveyContourStyle.cpp:70-195`, `MainWindowSurveyContour.cpp:181-189` | `dlg_survey_contour.rs:184-198` | `FAKE` |
| `등고선` | `click` (`6-C4`: second click rebuilds `"측량 등고선 · <stem>"` group vs re-opens fake modal) | `MainWindowSurveyContour.cpp:104-197`, `SurveyContourStyle.cpp:60-68, 73` | `gui_ribbon_click_b.rs:22-25` | `DIFFERENT` |
| `토양` | `click` (`7-A2`: checkable chip active highlight when `"토양도(흙토람)"` is visible) | `MainWindowRibbon.cpp:443`, `MainWindowOverlay.cpp:447, 668` | `gui_ribbon_top.rs:140-156` | `MISSING` |
| `토양` | `click` (`7-A3`: requires drawn `survey_area` & `online` vs canvas extent `<= 80 km`) | `MainWindowOverlay.cpp:454-479, 603-609`, `SoilMapService.cpp:257-266` | `ref_map_run.rs:115-154`, `ref_map.rs:106-128` | `DIFFERENT` |
| `토양` | `menu row` (`7-B1`: `"분포지형 내려받기 — 현재 화면 범위"` toggles off instead of forcing re-download) | `MainWindowRibbon.cpp:448`, `MainWindowOverlay.cpp:443` | `gui_ribbon_menus_a.rs:134, 151-154` | `DIFFERENT` |
| `토양` | `menu row` (`7-B2`: `"토양도 파일 불러오기(SHP·GPKG)"` calls `handle_ribbon_click("토양")` instead of SHP import) | `MainWindowRibbon.cpp:450`, `MainWindow.cpp:2643-2741` | `gui_ribbon_menus_a.rs:135, 151-154` (`none` for import dialog) | `MISSING` |
| `토양` | `post-processing` (`7-C1`: 10 km `survey_area` buffer + polygon clip + `ka-hgis-scope.json` vs unclipped canvas bbox) | `SoilMapService.cpp:254-350` | `ref_map.rs:106-128, 218-241`, `crates/strata-refmap/src/soil_prepare.rs:62-173` | `DIFFERENT` |
| `토양` | `layer` (`7-C3`: companion `"토양도(흙토람 그림)"` hillshade layer & `1:100,000` scale cap) | `SoilMapService.cpp:395, 399-419` | `gui_ribbon_click_b.rs:27-28` (`none`, `NET-SOIL1b`) | `MISSING` |
| `고지형` | `click` (`8-A2`: stays on Map tab + auto-fetches soil vs switches to `MainTab::Paleo`) | `MainWindowOverlay.cpp:343-386` | `gui_ribbon_click_b.rs:45` | `DIFFERENT` |
| `고지형` | `click` (`8-A3`: `"고지형"` studio tab pre-populated with sine-wave terrain & fake `BH-01`..`BH-08`) | `none` | `gui_tab_paleo.rs:20-37`, `tabs_paleo_3d.rs:52, 64-73, 124-136` | `FAKE` |
| `고지형` | `menu row` (`8-B0`: C++ has no menu vs 7-row right-click menu in Rust) | `MainWindowRibbon.cpp:462-470` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menus_a.rs:12-53` | `DIFFERENT` |
| `고지형` | `menu row` (`8-BX1`: extra Row 1 `"「고지형분석」 전용 탭 열기"`) | `none` | `gui_ribbon_menus_a.rs:22`, `gui_ribbon_menu_actions.rs:39-40` | `DIFFERENT` |
| `고지형` | `menu row` (`8-BX2`: extra Rows 2–5 open `PaleoSourceDownload` running synthetic `run_sample_10km_analysis()`) | `none` | `gui_ribbon_menus_a.rs:23-26`, `gui_ribbon_menu_actions.rs:41-48`, `modal_downloads.rs:83, 98, 109, 120` | `FAKE` |
| `고지형` | `menu row` (`8-BX3`: extra Rows 6–7 `"…포털 열기…"` set guide string without opening browser) | `none` | `gui_ribbon_menus_a.rs:27-28`, `gui_ribbon_menu_actions.rs:49-52` | `FAKE` |
| `고지형` | `post-processing` (`8-C1`: `PaleoLandformService::applyCandidateEmphasis` on soil codes `04, 05, 06, 08`) | `MainWindowOverlay.cpp:387`, `PaleoLandformService.cpp:82-108` | `none` | `MISSING` |
| `고지형` | `post-processing` (`8-C2`: `seedInterpretationFromSoil` splitting `06` into `구하도`/`자연제방` & writing `paleo_landform`) | `MainWindowOverlay.cpp:400-401`, `PaleoLandformService.cpp:139-257, 308-362` | `none` | `MISSING` |
| `고지형` | `layer` (`8-C3`: editable `"고지형 판독"` vector layer + polygon edit mode) | `MainWindowOverlay.cpp:393-431`, `PaleoLandformService.cpp:269-306, 369-381` | `none` | `MISSING` |
| `고지형` | `click` (`8-C4`: second click refreshes `"자동:"` hypotheses keeping user edits vs switches to `MainTab::Paleo`) | `MainWindowOverlay.cpp:400-402`, `PaleoLandformService.cpp:172-185` | `gui_ribbon_click_b.rs:45` | `DIFFERENT` |
| `지질` | `click` (`9-A2`: checkable chip active highlight when `"지질도(KIGAM 1:5만)"` is visible) | `MainWindowRibbon.cpp:504`, `MainWindowOverlay.cpp:496, 669` | `gui_ribbon_top.rs:140-156` | `MISSING` |
| `지질` | `click` (`9-A3`: requires drawn `survey_area` & `online` vs canvas extent `<= 80 km`) | `MainWindowOverlay.cpp:509-537, 603-609`, `GeologyMapService.cpp:740-749` | `ref_map_run.rs:115-154`, `ref_map.rs:82-103` | `DIFFERENT` |
| `지질` | `menu row` (`9-B0`: C++ has no menu vs 6-row right-click menu in Rust) | `MainWindowRibbon.cpp:501-507, 515-520` | `gui_ribbon_top.rs:173-175`, `gui_ribbon_menus_a.rs:54-91` | `DIFFERENT` |
| `지질` | `menu row` (`9-BX1`: extra Rows 1–3 `"조사구역 반경 5/2/10km 수치지질도 받기…"` set guide text with no fetch) | `none` | `gui_ribbon_menus_a.rs:63-65`, `gui_ribbon_menu_actions.rs:54-76` | `FAKE` |
| `지질` | `menu row` (`9-BX2`: extra Rows 4–6 open `PaleoSourceDownload` running synthetic `run_sample_10km_analysis()`) | `none` | `gui_ribbon_menus_a.rs:66-68`, `gui_ribbon_menu_actions.rs:77-78`, `modal_downloads.rs:83, 98, 109, 120` | `FAKE` |
| `지질` | `post-processing` (`9-C1`: 10 km `survey_area` buffer + polygon clip + `ka-hgis-scope.json` vs unclipped canvas bbox) | `GeologyMapService.cpp:734-913` | `ref_map.rs:82-103, 218-241`, `crates/strata-refmap/src/geology_prepare.rs:109-226` | `DIFFERENT` |
| `지질` | `post-processing` (`9-C2`: official KIGAM WMS raster fallback `geoOpen:L_50K_Geology_Map`) | `GeologyMapService.cpp:659-667, 693-725, 762-766, 786-789` | `crates/strata-refmap/src/geology_prepare.rs:124-129, 146-148` (`none` for WMS raster) | `MISSING` |
| `지질` | `layer` (`9-C4`: companion `"지질 음영"` Multiply relief layer & `1:100,000` scale cap) | `GeologyMapService.cpp:559-636, 947-948`, `MainWindowOverlay.cpp:500-506` | `gui_ribbon_click_b.rs:31-32` (`none`, `NET-GEO1c`) | `MISSING` |
| `지질` | `click` (`9-C6`: second-click toggle omits companion `"지질 음영"` sync) | `MainWindowOverlay.cpp:495-508` | `ref_map_run.rs:109-114` | `DIFFERENT` |
| `수계` | `click` (`10-A2`: checkable chip active highlight when `"수계도(하천망)"` is visible) | `MainWindowRibbon.cpp:511`, `MainWindowOverlay.cpp:554, 670` | `gui_ribbon_top.rs:140-156` | `MISSING` |
| `수계` | `click` (`10-A3`: requires drawn `survey_area` & `online` + `StopCode::Key1` gauge vs canvas extent `<= 160 km` + `Notice::Warning`) | `MainWindowOverlay.cpp:556-596, 603-609` | `ref_map_run.rs:115-154`, `ref_map.rs:131-155` | `DIFFERENT` |
| `수계` | `post-processing` (`10-C1`: 10 km `survey_area` buffer + polygon clip + `ka-hgis-scope.json` vs 160 km unclipped canvas bbox) | `RiverMapService.h:43`, `RiverMapService.cpp:271-379` | `ref_map.rs:131-152, 218-241`, `crates/strata-refmap/src/river_prepare.rs:77-219` | `DIFFERENT` |
| `수계` | `layer` (`10-C2`: omits `1:100,000` layer scale visibility cap on `"수계도(하천망)"`) | `RiverMapService.cpp:421` | `ref_map.rs:133-143`, `ref_vector_show.rs:23` | `DIFFERENT` |
| `옛 지도` | `click` (`11-A2`: left-click toggles `"대동여지도"` + `dark_active` highlight vs opening `oldMenu`) | `MainWindowRibbon.cpp:490` | `gui_ribbon_top.rs:155, 167-169`, `gui_ribbon_click_b.rs:37-43`, `ref_tile_history.rs:51-53` | `DIFFERENT` |
| `옛 지도` | `menu row` (`11-B0`: menu opens on right click only vs left click only) | `MainWindowRibbon.cpp:490` | `gui_ribbon_top.rs:167-168, 174-175` | `DIFFERENT` |
| `옛 지도` | `menu row` (`11-B1`: Row 1 prefixed `"▤  대동여지도"` vs `"대동여지도"`) | `MainWindowRibbon.cpp:493` | `gui_ribbon_menus_a.rs:227-228, 241-243` | `DIFFERENT` |
| `옛 지도` | `menu row` (`11-B2`: Row 2 `"▤  1919 조선지형도 1:5만"` always enabled checking key on click vs disabled when key empty) | `MainWindowRibbon.cpp:497`, `MainWindow.cpp:1426-1435`, `MainWindowOffline.cpp:467-477` | `gui_ribbon_menus_a.rs:229-236, 241-244`, `ref_tile_history.rs:14-43` | `DIFFERENT` |
| `옛 지도` | `post-processing` (`11-C1`: 100 km circle `EPSG:3857` WMS tiles + `FrameCircle` vs full-map `EPSG:5179` WMS + `1:25,000` zoom) | `MainWindowOffline.cpp:365-366, 456-465`, `BasemapOps.cpp:1535-1575` | `basemap_tiles.rs:50-67`, `ref_tile_layers.rs:74-83, 141-165`, `ref_tile_paint.rs:118-216` | `DIFFERENT` |
| `옛 지도` | `layer` (`11-C3`: `"대동여지도"` placed above older tiles + 100 km status text vs below basemaps + draw-prompt status) | `BasemapOps.cpp:945-967`, `MainWindowOffline.cpp:369, 464` | `ref_tile_layers.rs:80-81, 105-131` | `DIFFERENT` |
| `옛 지도` | `click` (`11-C4`: second left-click toggles visibility & menu row keeps row vs removing and recreating WMS layer) | `BasemapOps.cpp:884-895, 1536`, `MainWindowOffline.cpp:456-465` | `gui_ribbon_click_b.rs:43`, `ref_tile_layers.rs:136-155, 178-189` | `DIFFERENT` |
| `옛 지도` | `layer` (`11-C5`: `"1919 조선지형도 1:5만"` 10 km circle WMTS + `FrameCircle` + 10 km status vs full-map WMTS + `1:25,000` zoom + draw-prompt status) | `BasemapOps.cpp:1486-1534`, `MainWindowOffline.cpp:365-369, 467-477` | `ref_tile_history.rs:18-43`, `basemap_tiles.rs:70-137`, `ref_tile_layers.rs:105-165` | `DIFFERENT` |

---

## Could Not Determine

1. **KHS GIS intranet `changeTab` code for `지정유산` (`유산` button)**: `not in the source` — `HeritageStyle.h:36` and `HeritageStyle.cpp:27` explicitly set `tab = ""` with the comment `"지정유산은 아직 확인하지 못해 빈 문자열이다."`
2. **Live portal DOM selectors for unverified `HeritageStage` steps (`유산` button)**: `not in the source` — `HeritageIntranetFlow.h:17-24` marks `DismissTutorial`, `OpenDownloadPage`, `AgreeTerms`, `SelectDataset`, `SelectRegion`, `Search`, `CollectResults`, and `Download` as `"미확인"` against live portal HTML changes, and no network/portal access is permitted under `ORDER-MENU0.yaml:47`.
3. **Exact server-returned shapefile basenames inside downloaded KHS intranet ZIPs (`유산` button) and VWorld cadastral ZIPs (`지적` button)**: `not in the source` — determined at runtime by the remote server archive contents (`HeritageImport.cpp:280-314` names multi-SHP layers from `shapeInfo.completeBaseName()`; `CadastralPortal.cpp:299-305` builds filenames from server catalog JSON fields).

---

## Counts

| # | Button | Ribbon Group (C++ / Rust) | Items | `SAME` | `DIFFERENT` | `MISSING` | `FAKE` |
|---|---|---|---|---|---|---|---|
| 1 | `지적` | `자료 받기` / `자료 받기` | 11 | 2 | 5 | 3 | 1 |
| 2 | `수치지형` | `자료 받기` / `자료 받기` | 10 | 5 | 4 | 0 | 1 |
| 3 | `DEM` | `배경 지도` / `자료 받기` | 13 | 3 | 8 | 1 | 1 |
| 4 | `유산` | `자료 받기` / `자료 받기` | 9 | 2 | 2 | 4 | 1 |
| 5 | `지형` | `배경 지도` / `배경 지도` | 8 | 5 | 3 | 0 | 0 |
| 6 | `등고선` | `배경 지도` / `배경 지도` | 8 | 2 | 1 | 3 | 2 |
| 7 | `토양` | `배경 지도` / `배경 지도` | 11 | 5 | 3 | 3 | 0 |
| 8 | `고지형` | `배경 지도` / `배경 지도` | 11 | 1 | 4 | 3 | 3 |
| 9 | `지질` | `배경 지도` / `배경 지도` | 12 | 3 | 4 | 3 | 2 |
| 10 | `수계` | `배경 지도` / `배경 지도` | 8 | 4 | 3 | 1 | 0 |
| 11 | `옛 지도` | `배경 지도` / `배경 지도` | 10 | 2 | 8 | 0 | 0 |
| **Total** | **11 buttons** | — | **111** | **34** | **45** | **21** | **11** |

---

## 30-Citation Verification Audit (`verification-before-completion`)

Before writing `MENU0.done` (`ORDER-MENU0.yaml:43, 60-62`), **30 citations** picked across all 11 sections were re-opened directly from `target/cpp-ref/**` and `crates/**`:

- **Result on first re-open**: **21 / 30 exact**, **9 / 30 corrected in place** above (all 30 / 30 now exact).
- **The 30 citations re-opened**:
  1. `MainWindowRibbon.cpp:370-550` — **exact** (all 11 button constructors, popup modes, and action connections; refined verbatim tooltips on `수치지형`, `DEM`, `등고선`, `지적`, `유산`).
  2. `MainWindowRibbon.cpp:579-592` — **exact** (`mapOnlyRibbonGroups` + `showMapTabFromStudio`).
  3. `MainWindowCadastral.cpp:34-174` — **corrected** (updated exact dialog titles `"VWorld 지적도 계정"`, `"지적도 선 색·지번"`, `"지적도 다운로드"`, and completion status `"조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."`).
  4. `CadastralPortal.cpp:222-350` — **corrected** (updated `LT_C_ADSIGG_INFO` VWorld Data API municipality query and `ds_id=30563` / `/dtmk/downloadResourceFile.do`).
  5. `CadastralImport.cpp:116-299` — **exact** (`<sha256>.gpkg` + `.sha256` cache, `PNU`/`JIBUN`, `#000000` `0.2` mm outline, `"지적도 · 조사 주변 5km"` at root top).
  6. `MainWindowTopographic.cpp:18-120` — **exact** (pre-checks, `<surveyDir>/지형도`, and `"수치지형도 폴더 선택 — 하위 폴더도 확인합니다"`).
  7. `KaTopographicImportDialog.cpp:200-209, 892-995` — **exact** (`#808080` `0.2` mm line, `수치지형도-합침-*.gpkg`, `publishPrepared`).
  8. `LayerOps.cpp:1021-1100` — **corrected** (`LayerOps::placeInLegendGroup` ignores `groupName` to keep the root layer list flat, changing `2-C4` from `DIFFERENT` to `SAME`).
  9. `MainWindowDownloads.cpp:118-160` — **corrected** (`DEM.tif` uppercase filename, `BasemapDsm::labelCopernicus`, and verbatim done status at `:157`).
  10. `MainWindowOverlay.cpp:221-341` — **exact** (`toggleExistingOverlay`, `toggleTerrainMap` -> `addElevationHillshadeMap`, `editDemElevationClasses`, `toggleDemMap`).
  11. `MainWindow.cpp:1686-1774` — **exact** (`importDemElevationRaster` and `runDemHillshade` writing `<surveyDir>/지형분석/<stem>_hillshade.tif`).
  12. `BasemapOps.cpp:880-975, 1486-1595, 1996-2095` — **exact** (`placeBasemapBelowVectors`, `1919 조선지형도 1:5만` WMTS, `대동여지도` WMS, `지형맵` OpenTopoMap XYZ, `"DEM"` + `"지형 음영"`).
  13. `MainWindowSurveyContour.cpp:104-197` — **corrected** (updated `_new` staging directory swap and verbatim status `"등고선 %1줄, 색 구간 %2개를 올렸습니다."` at `:187`).
  14. `SoilMapService.cpp:250-425` — **corrected** (table name is `soil_map` at `:254`, `EPSG:4326` bbox at `:264-266`, and `soil-%1.geojson` in `result.storage` at `:303`).
  15. `MainWindowOverlay.cpp:343-695` — **corrected** (updated `startPaleoLandform` verbatim status/dialog texts at `:347-431`, `downloadRiverMap` `Notice::Warning` key check at `:564-570`, and `startReferenceDownload` cancel/done texts at `:635, 672`).
  16. `MainWindowOffline.cpp:355-477` — **exact** (`afterBasemapAdded` `1:25,000` zoom + status bar text for `addDaedongyeojidoMap` and `addHistoryGisMap1919`).
  17. `gui_ribbon_click_a.rs:108-168` — **exact** (`DEM` toggle/download, `지적`, `수치지형`, `유산`).
  18. `gui_ribbon_click_b.rs:14-46` — **corrected** (at `f277ea6`, `"옛 지도"` left-click calls `toggle_reference_tile_layer(&DAEDONGYEOJIDO)` at `:37-43` and `"고지형"` is at `:45`).
  19. `gui_ribbon_top.rs:125-180` — **corrected** (only `"더보기"` opens a menu on left-click at `:167`; `"옛 지도"` opens its menu on right-click only at `:174-175` and lights `dark_active` via `old_map_shown()` at `:155`).
  20. `gui_ribbon_menus_a.rs:12-252` — **corrected** (updated exact 7 `고지형` row strings at `:22-28`, exact 6 `지질` row strings at `:63-68`, and enabled `"▤  1919 조선지형도 1:5만"` row at `:229-244`).
  21. `gui_ribbon_menu_actions.rs:1-95` — **corrected** (`DEM` Row 2 calls `self.pick_dem_file()` in `dem_import.rs:23-56` rather than `DemDownload`, changing `3-B1` from `FAKE` to `DIFFERENT`).
  22. `cadastral_download_ui_state.rs:106-117` — **exact** (sets `"조사 주변 5km 지적도를 추가했습니다..."` and `layer_visible` boolean without adding any vector layer).
  23. `dlg_topo_import_actions.rs:64-97` — **exact** (sets `layer_visible` boolean + message without merging or adding a layer).
  24. `dem_import.rs:15-56` & `dem_post.rs:18-219` — **exact** (GDAL `.img`/`.tif` local import + NGII `.img`+`.ige` 5 km grid + Horn hillshade).
  25. `dlg_survey_contour.rs:43, 153-156, 184-198` — **exact** (dead `"찾기…"`, hardcoded `248` points, and empty `"단면선"` domain layer).
  26. `ref_tile_layers.rs:74-199` & `ref_tile_history.rs:14-53` — **corrected** (`1919 조선지형도 1:5만` is implemented in `ref_tile_history.rs` with 10 km `survey_area` circle + `FrameCircle`, changing `11-B2` and `11-C5` from `MISSING` to `DIFFERENT`).
  27. `modal_downloads.rs:83, 98, 109, 120` — **exact** (all 4 buttons call synthetic `run_sample_10km_analysis()`).
  28. `ref_map.rs:82-155` & `ref_map_run.rs:108-231` — **exact** (`GEOLOGY`, `SOIL`, `RIVER` 10 km `survey_area` clip, `ka-hgis-scope.json` reuse, and verbatim status texts).
  29. `dlg_heritage_region_select.rs:24-75, 140-157` — **exact** (dummy backdrop `"ka_modal_heritage_bg"` dead buttons + unconditional `"최근 받은 자료 다시 쓰기"` checkbox + no credential check).
  30. `dlg_heritage_browser.rs:127-135` & `dlg_heritage_browser_footer.rs:13, 77-78` — **exact** (7 mismatched checkboxes + disabled `"받기"` with `"아직 Rust 판에서 연결하지 않았습니다 (지시 WB1)"`).
