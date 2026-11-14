# DEM Parity Trace — C++ Strata vs. Rust Port (`DEM0`)

- **C++ reference**: `../../../target/cpp-ref` (read-only; cited as `<file name>:line`, unique under `target/cpp-ref/src`).
- **Rust columns**:
  1. `main` (`49384eaaee8c18ff083ced59061deb995db56a47`, checked out at `ag/dem0`).
  2. `branch ag/dl2b` (`18840b2fca793cba5ad5771128bc65982cd78416`, unmerged; inspected via `git show ag/dl2b:<path>` and `git diff 49384ea...ag/dl2b --stat`).
- **Prior download window inventory**: `docs/DOWNLOAD_FLOWS.md:13` and `Flow 1` (`docs/DOWNLOAD_FLOWS.md:165-217`) already list the windows and progress stages of the one-click Copernicus download. Their window table is not repeated below; five places where the source differs from `docs/DOWNLOAD_FLOWS.md` are corrected in [Corrections to `docs/DOWNLOAD_FLOWS.md` Flow 1](#corrections-to-docsdownload_flowsmd-flow-1).
- **Explicit exclusion (`지형` / `지형맵` is NOT the DEM)**: `LayerOps::addElevationHillshadeMap` (`BasemapOps.cpp:1577-1595`) does **not** load or shade a DEM file. It adds an online OpenTopoMap XYZ raster layer (`type=xyz&url=https://tile.opentopomap.org/%7Bz%7D/%7Bx%7D/%7By%7D.png&zmax=17&zmin=5&crs=EPSG:3857&tilePixelRatio=1`, fallback `a.tile.opentopomap.org`) titled `"지형맵"` (`ReferenceKind::kTerrain = "terrain"`, `ReferenceKind.h:35`, `ReferenceKind.h:66`) for the separate ribbon chip `"지형"` (`MainWindowRibbon.cpp:370-381`, `MainWindowOverlay.cpp:239-267`). Per `ORDER.yaml:32`, it is left out of the DEM trace below (except in Section 5 to note two Rust comments that mis-cite `addElevationHillshadeMap` for DEM rendering).

---

## Corrections to `docs/DOWNLOAD_FLOWS.md` `Flow 1`

1. **Flat legend root, not a `"참조 지도"` group (`docs/DOWNLOAD_FLOWS.md:215`)**: `LayerOps::addDemElevationRaster` calls `LayerOps::placeInLegendGroup(project, rl, QStringLiteral("참조 지도"))` (`BasemapOps.cpp:2025`), and `MainWindow::runDemHillshade` does the same (`MainWindow.cpp:1769`), but `LayerOps::placeInLegendGroup` (`LayerOps.cpp:1021-1038`) has `Q_UNUSED(groupName); Q_UNUSED(insertAtBottom);` and leaves the node at the flat `project->layerTreeRoot()` above basemap/satellite layers (`BasemapOps.cpp:362-401`), separated from survey layers by `ka_hgis/layer_role = "reference"` (`LayerOps.cpp:1052-1056`). `LayerOps::ensureDemRelief` inserts `"지형 음영"` into the same parent (`root`) at `parent->children().indexOf(demNode)` (`BasemapOps.cpp:2091-2103`), immediately **above** the DEM layer. Only `"측량점으로 등고선…"` (`SurveyContourStyle.cpp:79-86`) creates an actual `"참조 지도"` group node.
2. **Staging-folder failure wording (`docs/DOWNLOAD_FLOWS.md:199`)**: `ReferenceMapPreparation::initializeStorage` (`PreparedReferenceMap.cpp:23`) sets `"지도를 저장할 임시 폴더를 만들지 못했습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인하세요."`, not `"참조 자료를 준비할 임시 폴더를..."`.
3. **GDAL cancel callback line range (`docs/DOWNLOAD_FLOWS.md:186`)**: `Progress::update` is at `DemDownloadService.cpp:68-82`; `DemDownloadService.cpp:96-105` holds `isMissingTile` and `gridAligned`.
4. **Omitted stop conditions (`docs/DOWNLOAD_FLOWS.md:192-210`)**: `Flow 1` omits four C++ stop paths:
   - Single-slot busy guard in `MainWindow::startFileDownload` (`MainWindowDownloads.cpp:39-41`): `"지도 자료를 받고 있습니다. 완료를 기다리거나 진행 창에서 취소하세요."` (5 s status bar).
   - `GDALTranslateOptionsNew` allocation failure (`DemDownloadService.cpp:263-265`): `"DEM 저장 설정을 만들지 못했습니다. 다시 내려받으세요."`.
   - Catch-all exception handler in `DemDownloadService::prepare` (`DemDownloadService.cpp:300-303`): `"DEM 내려받기를 완료하지 못했습니다. 인터넷 연결과 저장 공간을 확인한 뒤 다시 시도하세요. 기존 지도는 유지됩니다."`.
   - Post-download `apply` exception catch in `MainWindow::startFileDownload` (`MainWindowDownloads.cpp:76-80`): warning `"DEM 표시 실패"` with `"자료를 받았지만 지도에 표시하지 못했습니다. 현재 작업을 저장한 뒤 파일함에서 다시 열어 주세요.\n%1"`.
5. **Rust file path & unclipped extent (`docs/DOWNLOAD_FLOWS.md:13`, `docs/DOWNLOAD_FLOWS.md:172`, `docs/DOWNLOAD_FLOWS.md:214`)**: `dem_target_base_path` returns `<surveyDir>/DEM/원본/dem.tif` (`crates/strata-app/src/dem_download_ui_paths.rs:25-35`), which `create_temp_storage` (`crates/strata-net/src/dem_fetch.rs:130-139`) turns into `<surveyDir>/DEM/원본/ka-hgis-reference-<6hex>/dem.tif` (`crates/strata-net/src/dem_pipeline.rs:160, 190-192`), leaving `<surveyDir>/DEM/원본/dem.tif` itself unwritten; `mosaic_tiles` (`crates/strata-net/src/dem_pipeline.rs:66-89`) also omits `-srcwin`, saving full 1°×1° tiles unclipped.

---

## Section 1 — The Button `「DEM」` and Its Four Menu Rows

### 1.0 Ribbon Button Setup (`S1`)

- **Step `S1`** (`MainWindowRibbon.cpp:399-428`, `MainWindowRibbon.cpp:251`, `MainWindow.cpp:1398-1423`):
  - Widget: `QToolButton* m_btnDem` (`objectName = "btnDem"`, icon `KaIcons::icon("dem")`, style `Qt::ToolButtonTextUnderIcon`, `setCheckable(true)`), added to ribbon group `"basemap"` (`"배경 지도"`, `MainWindowRibbon.cpp:251`, `MainWindowRibbon.cpp:428`).
  - Label verbatim: `"DEM"` (`MainWindowRibbon.cpp:402`).
  - Tooltip verbatim (`MainWindowRibbon.cpp:407-409`):
    > `고도를 색으로 보여 줍니다. 받는 자료는 지표모델(DSM) 30m · Copernicus라 나무·건물 높이가 섞여 있습니다.\n미세지형 판독에는 우클릭 → 「국토지리원 DEM 불러오기」를 쓰세요. 다시 누르면 숨깁니다. 우클릭: 상세 메뉴`
  - Context menu (`QMenu* demMenu`, opened on right-click via `Qt::CustomContextMenu` at `MainWindowRibbon.cpp:410-423`), four actions in order:
    1. `"국토지리원 DEM 불러오기(.img)…"` → `MainWindow::importDemElevationRaster` (`MainWindowRibbon.cpp:411-413`)
    2. `"DEM 파일로 음영 만들기…"` → `MainWindow::runDemHillshade` (`MainWindowRibbon.cpp:414-415`)
    3. `"DEM 표현…"` → `MainWindow::editDemElevationClasses` (`MainWindowRibbon.cpp:416-417`)
    4. `"측량점으로 등고선…"` → `MainWindow::createSurveyContours` (`MainWindowRibbon.cpp:418-419`)
  - Left-click signal: `MainWindow::toggleDemMap` (`MainWindowRibbon.cpp:424`), followed by `QTimer::singleShot(0, ...)` calling `MainWindow::syncThematicButtons()` (`MainWindowRibbon.cpp:425-427`), which syncs `m_btnDem->setChecked(...)` from `LayerOps::isLayerVisible(proj, "DEM")` (`MainWindow.cpp:1404-1416`, resolved via `ReferenceKind::kDem` in `BasemapOps.cpp:2352-2367` and `ReferenceKind.h:80-92`).

### 1.1 Left-Clicking `「DEM」` (`MainWindow::toggleDemMap`, `S2`–`S5`)

1. **Step `S2` — First click when no DEM layer is in the project** (`MainWindowOverlay.cpp:299-306`, `MainWindowOverlay.cpp:337`, `BasemapDsm.h:48-57`):
   - If `!m_canvas`, returns silently (`MainWindowOverlay.cpp:301-302`).
   - Looks up an existing DEM by `BasemapDsm::findDem(proj)` (`MainWindowOverlay.cpp:305`), which searches `ReferenceKind::find(proj, "dem")` — any `QgsRasterLayer` carrying `ka_hgis/reference_kind = "dem"`, or an untagged legacy non-survey layer named `"DEM"` (`BasemapDsm.h:48-57`, `ReferenceKind.h:98-135`).
   - When `dem == nullptr`, calls `MainWindow::startDemDownload()` (`MainWindowOverlay.cpp:337`) with no file dialog (full download pipeline in Section 2).
2. **Step `S3` — Post-download layer registration & disk retention** (`MainWindowDownloads.cpp:64-75`, `MainWindowDownloads.cpp:105-116`, `MainWindowDownloads.cpp:135-159`, `BasemapOps.cpp:1996-2037`, `BasemapDsm.h:21-40`, `ReferenceStorage.cpp:45-117`, `MainWindowSession.cpp:1238`):
   - `startFileDownload` calls `result.retainFiles()` (`PreparedReferenceMap.h:35-37`, disabling `QTemporaryDir` auto-remove on `<surveyDir>/ka-hgis-reference-XXXXXX`) and invokes the `apply` lambda (`MainWindowDownloads.cpp:64-67`).
   - `LayerOps::addDemElevationRaster(QgsProject::instance(), window->m_canvas, result.rasterUri, &error)` (`MainWindowDownloads.cpp:138`, `BasemapOps.cpp:1996-2037`):
     - Opens `<surveyDir>/ka-hgis-reference-XXXXXX/dem.tif` as `new QgsRasterLayer(path, "DEM", "gdal")`, checks `rl->isValid() && rl->bandCount() >= 1` (`BasemapOps.cpp:2007-2014`).
     - Styles with `LayerOps::applyDemElevationStyle(rl)` (`"national"` 0–2000 m 16-stop ramp, bilinear resampling, opacity `1.0`, `DemColorRampLegend`, `BasemapOps.cpp:2015`, `DemPresentation.cpp:44-108`).
     - Marks `ka_hgis/layer_role = "reference"`, sets `ka_hgis/crs_label = "EPSG:4326"` and abstract `"좌표계 EPSG:4326"`, tags `ka_hgis/reference_kind = "dem"`, removes any previous `ReferenceKind::kDem` layer (`removeReferenceKind(project, ReferenceKind::kDem)`), and adds `rl` to `QgsProject` (`BasemapOps.cpp:2016-2024`, `LayerOps.cpp:1052-1056`, `LayerOps.cpp:1171-1181`).
     - Calls `LayerOps::placeInLegendGroup(project, rl, "참조 지도")` (flat root placement above basemaps/satellites, `BasemapOps.cpp:2025`, `LayerOps.cpp:1021-1038`), `LayerOps::ensureDemRelief(project, rl)` (adds/updates `"지형 음영"` directly above DEM in `root`, `BasemapOps.cpp:2026`, `BasemapOps.cpp:2091-2103`), and `DemPresentation::followCanvas(rl, canvas)` (`BasemapOps.cpp:2027`).
     - Because the downloaded raster CRS is geographic (`EPSG:4326`), `!rl->crs().isGeographic()` is `false`, so the canvas is **not** zoomed; `LayerOps::syncMapCanvas(project, canvas, false)` refreshes the current view (`BasemapOps.cpp:2028-2035`).
   - `startDemDownload` then renames the layer via `BasemapDsm::labelCopernicus(layer)` (`MainWindowDownloads.cpp:147`, `BasemapDsm.h:21-40`):
     - Layer title verbatim: `"지표모델(DSM) 30m · Copernicus"` (`BasemapDsm.h:21`, `BasemapDsm.h:39`).
     - Layer abstract / tooltip verbatim (`BasemapDsm.h:23-28`, `BasemapDsm.h:35-38`):
       > `Copernicus GLO-30 지표모델(DSM), 격자 약 30 m. 나무·건물 높이가 섞여 있어 능선·단구·옛 물길 같은 미세지형 판독에는 맞지 않습니다. 정밀 판독에는 국토지리정보원 DEM을 불러오세요.\n좌표계 EPSG:4326`
   - Stores `ka_hgis/dem_cover_wgs84 = "<xmin>,<ymin>,<xmax>,<ymax>"` (9 decimal places, `EPSG:4326`, `MainWindowDownloads.cpp:148-151`), selects the layer in `m_layerTree` (`MainWindowDownloads.cpp:152`), warns `"DEM 음영 표시 실패"` if `ka_hgis/dem_relief_error` is non-empty (`MainWindowDownloads.cpp:153-154`), marks the project dirty (`MainWindowDownloads.cpp:156`), and shows status bar (8 s, `MainWindowDownloads.cpp:157`):
     > `DEM을 추가했습니다. DEM 버튼의 「DEM 표현」에서 색과 음영을 조절하세요.`
   - `ReferenceStorage::sessionLedger()` adopts the new `ka-hgis-reference-XXXXXX` folder and marks any superseded `ka-hgis-reference-XXXXXX` folder retired (`MainWindowDownloads.cpp:70-74`, `ReferenceStorage.cpp:77-88`); retired folders are deleted only when the survey is saved (`MainWindow::pruneRetiredReferenceFolders`, `MainWindowDownloads.cpp:105-116`, called from `MainWindow::markSurveySaved` at `MainWindowSession.cpp:1238`).
3. **Step `S4` — Second click on `「DEM」` while DEM is visible** (`MainWindowOverlay.cpp:307-334`):
   - `wasVisible = LayerOps::isLayerVisible(proj, "DEM")` is `true` → `makeVisible = false` (`MainWindowOverlay.cpp:308-309`).
   - Hides the `ReferenceKind::kDem` layer via `LayerOps::toggleLayerVisibility(proj, m_canvas, "DEM", false)` (`MainWindowOverlay.cpp:315`, `BasemapOps.cpp:2331-2350`) and unchecks `m_btnDem` (`MainWindowOverlay.cpp:316-317`).
   - If `"지질도"` (`ReferenceKind::kGeology`) is not visible, also hides `"지형 음영"` (`ReferenceKind::kDemRelief`) via `LayerOps::toggleLayerVisibility(proj, m_canvas, "지형 음영", false)` (`MainWindowOverlay.cpp:323-325`).
   - Syncs and refreshes `m_canvas` (`MainWindowOverlay.cpp:326-329`) and shows status bar (4 s, `MainWindowOverlay.cpp:330-333`):
     > `DEM 지형을 숨겼습니다.`
   - No files are deleted or written.
4. **Step `S5` — Clicking `「DEM」` again while DEM exists but is hidden** (`MainWindowOverlay.cpp:308-334`, `BasemapOps.cpp:1793-1829`):
   - `wasVisible` is `false` → `makeVisible = true` (`MainWindowOverlay.cpp:308-309`).
   - Calls `LayerOps::demCoversCanvas(proj, m_canvas)` (`MainWindowOverlay.cpp:311`, `BasemapOps.cpp:1793-1829`):
     - Reads `ka_hgis/dem_cover_wgs84` (`4` comma-separated `f64` values in `EPSG:4326`). If missing or malformed — including an imported NGII `.img` DEM, which never sets `ka_hgis/dem_cover_wgs84` (`BasemapOps.cpp:1996-2037`) — returns `false` (`BasemapOps.cpp:1802-1812`).
     - Projects `canvas->extent()` from `canvas->mapSettings().destinationCrs()` to `EPSG:4326` and checks `covered.contains(ext)` (`BasemapOps.cpp:1815-1828`).
   - If `demCoversCanvas` returns `false`, calls `startDemDownload()` to download a new Copernicus DEM covering the current view and replace the previous `kDem` layer (`MainWindowOverlay.cpp:311-314`).
   - Otherwise turns `kDem` visible, checks `m_btnDem`, calls `LayerOps::ensureDemRelief(proj, dem)` and `DemPresentation::followCanvas(dem, m_canvas)`, refreshes `m_canvas`, and shows status bar (4 s, `MainWindowOverlay.cpp:315-334`):
     > `DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요.`

### 1.2 Menu Row 1 — `「국토지리원 DEM 불러오기(.img)…」` (`MainWindow::importDemElevationRaster`, `S6`)

- **Step `S6`** (`MainWindowRibbon.cpp:411-413`, `MainWindow.cpp:1686-1705`, `BasemapOps.cpp:1996-2037`):
  1. If `!m_canvas`, returns silently (`MainWindow.cpp:1688`).
  2. Opens `QFileDialog::getOpenFileName` (`MainWindow.cpp:1689-1691`):
     - Window title verbatim: `"국토지리원 DEM"`
     - Start folder verbatim: `QString()` (empty string; Qt default working/last directory)
     - Filter verbatim: `"국토지리원 DEM (*.img *.IMG *.tif *.tiff *.TIF *.TIFF)"`
     - Cancel (`path.isEmpty()`) returns silently (`MainWindow.cpp:1692`).
  3. Calls `LayerOps::addDemElevationRaster(QgsProject::instance(), m_canvas, path, &err)` (`MainWindow.cpp:1694`, `BasemapOps.cpp:1996-2037`):
     - Checks `!project` (`"프로젝트가 없습니다."`), `!fi.exists()` (`"DEM 파일이 없습니다."`), and `new QgsRasterLayer(path, "DEM", "gdal")` validity + `bandCount() >= 1` (`"국토지리원 DEM(.img)을 열 수 없습니다: %1"`, `BasemapOps.cpp:1998-2014`); on failure `importDemElevationRaster` shows `Notice::Warning` titled `"DEM"` (fallback `"DEM 파일을 열지 못했습니다."`) and unchecks `m_btnDem` (`MainWindow.cpp:1695-1698`).
     - Note: if `rl->bandCount() > 1` (e.g. a 3-band RGB GeoTIFF), `addDemElevationRaster` accepts `bandCount() >= 1` (`BasemapOps.cpp:2008`), `applyDemElevationStyle` -> `DemPresentation::apply` returns `false` because `bandCount() != 1` (`DemPresentation.cpp:47-49`), QGIS keeps its default multi-band renderer, and `ensureDemRelief` records `ka_hgis/dem_relief_error = "이 지도는 표고값 DEM이 아닙니다. 단일 밴드 표고 자료를 불러오세요."` without creating `"지형 음영"` (`BasemapOps.cpp:2041-2045`).
     - Files written: **none** inside the survey folder (references `path` in place; if the file has a projected CRS, no `.vrt` is written; if geographic, `<CacheLocation>/dem-relief/<sha256>.vrt` is written by `DemPresentation::reliefSource`, `DemPresentation.cpp:169-223` — see Section 4).
     - Layers added: replaces any existing `ReferenceKind::kDem` layer (`BasemapOps.cpp:2019`), adds `"DEM"` (`ka_hgis/reference_kind = "dem"`, `ka_hgis/layer_role = "reference"`) at flat `layerTreeRoot()` above basemaps/satellites, and adds/updates `"지형 음영"` (`ka_hgis/reference_kind = "dem_relief"`) immediately above `"DEM"` (`BasemapOps.cpp:2015-2027`, `BasemapOps.cpp:2091-2103`).
     - Canvas zoom: if `!rl->crs().isGeographic()` and `0 < width < 20000.0` and `height < 20000.0` (metres), zooms canvas to the layer via `LayerOps::zoomToLayerMax(canvas, rl)`; otherwise calls `LayerOps::syncMapCanvas(project, canvas, false)` (`BasemapOps.cpp:2028-2035`).
  4. Checks `m_btnDem` (`MainWindow.cpp:1700`) and shows status bar (6 s, `MainWindow.cpp:1701`):
     > `국토지리원 DEM을 올렸습니다. 범례에 높이(m)가 표시됩니다.`
  5. **Second invocation**: Picking another file calls `removeReferenceKind(project, ReferenceKind::kDem)` (`BasemapOps.cpp:2019`), replacing the previous DEM layer (`"DEM"` or `"지표모델(DSM) 30m · Copernicus"`) and updating the single `"지형 음영"` layer in place (`BasemapOps.cpp:2047-2066`). Note that `importDemElevationRaster` does **not** set `ka_hgis/dem_cover_wgs84`, so hiding `"DEM"` with the ribbon chip and clicking `「DEM」` again triggers `demCoversCanvas == false` → `startDemDownload()` (`MainWindowOverlay.cpp:311-314`, `BasemapOps.cpp:1802-1804`).

### 1.3 Menu Row 2 — `「DEM 파일로 음영 만들기…」` (`MainWindow::runDemHillshade`, `S7`–`S9`)

1. **Step `S7` — Source GeoTIFF picker and modal parameter dialog** (`MainWindowRibbon.cpp:414-415`, `MainWindow.cpp:1707-1745`):
   - Opens `QFileDialog::getOpenFileName` (`MainWindow.cpp:1709-1711`):
     - Window title verbatim: `"DEM GeoTIFF"`
     - Start folder verbatim: `QString()`
     - Filter verbatim: `"GeoTIFF (*.tif *.tiff *.TIF *.TIFF)"` (`.img` is not in this filter).
     - Cancel (`dem.isEmpty()`) returns silently (`MainWindow.cpp:1712-1713`).
   - Opens modal `QDialog` titled `"지형분석"` (`MainWindow.cpp:1714-1740`) with four `QFormLayout` rows and `Ok | Cancel`:
     - `"방식"`: `QComboBox` with `"다중광원 음영 (유구·분묘)"` (`DemAnalyzer::HillshadeMode::Multi`, default) and `"단방향 음영"` (`DemAnalyzer::HillshadeMode::Single`) (`MainWindow.cpp:1717-1719`, `MainWindow.cpp:1731`).
     - `"방위각"`: `QDoubleSpinBox`, range `0..360`, default `315`, suffix `" °"` (`MainWindow.cpp:1720-1723`, `MainWindow.cpp:1732`).
     - `"고도각"`: `QDoubleSpinBox`, range `1..90`, default `45`, suffix `" °"` (`MainWindow.cpp:1724-1727`, `MainWindow.cpp:1733`).
     - `"Z계수"`: `QDoubleSpinBox`, range `0.01..50`, default `1.0`, no suffix (`MainWindow.cpp:1728-1730`, `MainWindow.cpp:1734`).
     - Cancel (`dlg.exec() != QDialog::Accepted`) returns silently (`MainWindow.cpp:1739-1740`).
2. **Step `S8` — Output path resolution and Horn strip GeoTIFF generation** (`MainWindow.cpp:1747-1759`, `DemHillshadeFile.cpp:14-23`, `DemHillshadeFile.cpp:51-138`, `DemAnalyzer.cpp:18-53`, `DemAnalyzer.cpp:205-241`):
   - `DemAnalyzer::hillshadeOutputPath(dem, m_surveyPath)` (`DemHillshadeFile.cpp:128-138`):
     - `stem` = `completeBaseName()` of `dem` (fallback `"DEM"` if empty).
     - When a survey is open (`!m_surveyPath.isEmpty()`): `<surveyDir>/지형분석/<stem>_hillshade.tif`; if that file already exists, increments `<surveyDir>/지형분석/<stem>_hillshade_2.tif` … `_9999.tif` (`DemHillshadeFile.cpp:131-137`).
     - When no survey is open (`m_surveyPath.isEmpty()`): defaults to `<demDir>/<stem>_hillshade.tif` (`_2`… on collision) and prompts `QFileDialog::getSaveFileName(this, "음영기복 저장", out, "GeoTIFF (*.tif)")` (`MainWindow.cpp:1748-1753`).
   - `DemAnalyzer::runHillshadeFile(dem, out, opt, &err)` (`DemHillshadeFile.cpp:51-126`) runs **synchronously on the UI thread**:
     - Opens `dem` via `GDALOpen(..., GA_ReadOnly)`, checks `band != nullptr && width >= 3 && height >= 3` (`DemHillshadeFile.cpp:55-64`; failures `"DEM을 열 수 없습니다."`, `"DEM에 음영을 계산할 칸이 부족합니다."`). Does **not** reproject or check CRS units (`xres = geotransform[1]`, `yres = geotransform[5]` used directly in `DemAnalyzer.cpp:39-40`, `DemAnalyzer.cpp:210-211`).
     - Creates 1-band `GDT_Byte` GeoTIFF at `out` with `COMPRESS=DEFLATE`, `TILED=YES`, `BIGTIFF=IF_SAFER`, copies geotransform and projection WKT, and sets band `NoData = 0` (`DemHillshadeFile.cpp:71-85`; failure `"음영 GeoTIFF를 만들 수 없습니다. 저장 폴더의 쓰기 권한을 확인하세요."`).
     - Processes strips of `kStripCells / width` rows (`kStripCells = 4 * 1024 * 1024`, 1-row top/bottom halo, `DemHillshadeFile.cpp:14`, `DemHillshadeFile.cpp:88-118`) using own C++ 3×3 Horn (1981) kernel (`hornDeriv` + `shadeOne` in `DemAnalyzer.cpp:18-53`, `DemAnalyzer.cpp:205-241`): `Single` shades at `(azimuthDeg, altitudeDeg)`; `Multi` averages `shadeOne` across four azimuths `{225°, 270°, 315°, 360°}` (`sum / 4`, integer division, `DemAnalyzer.cpp:216`, `DemAnalyzer.cpp:230-235`).
     - Outer 1-px raster border and any pixel whose 3×3 neighbourhood contains source `NoData` stay `0` (`NoData`); valid shaded pixels are clamped to `std::max<uint8_t>(value, 1)` so dark slopes never collide with `NoData = 0` (`DemHillshadeFile.cpp:17-23`, `DemHillshadeFile.cpp:106-115`). On I/O failure deletes `out` and returns `"DEM을 읽거나 음영을 쓰다가 멈췄습니다. 파일과 저장 공간을 확인하세요."` (`DemHillshadeFile.cpp:121-124`), surfaced as `Notice::Warning` `"지형분석"` / `"음영기복을 만들지 못했습니다."` (`MainWindow.cpp:1755-1758`).
3. **Step `S9` — Adding `"음영기복"` to the map** (`MainWindow.cpp:1760-1773`, `MainWindow.cpp:2884-2937`):
   - Calls `MainWindow::addRasterFromPath(out)` (`MainWindow.cpp:1760`, `MainWindow.cpp:2884-2937`):
     - Opens `QgsRasterLayer(out, completeBaseName, "gdal")`; if `!rl->crs().isValid()` and project CRS is valid, assigns project CRS and shows status `"GeoTIFF에 좌표계가 없어 작업 좌표계(%1)로 올렸습니다."` (`MainWindow.cpp:2896-2900`); marks `ka_hgis/layer_role = "reference"` and `ka_hgis/imported_reference = true`, sets CRS label, adds to `QgsProject`, runs `LayerOps::knockOutRasterPaper(rl)` (near-white paper knockout on `.tif`), zooms if `layerSitsOnWorkMap`, and shows `"그림 추가: %1 · %2 · 화면 %3"` (`MainWindow.cpp:2901-2932`). If `addRasterFromPath` fails, warns `"지형분석"` / `"음영 지도를 지도 화면에 올리지 못했습니다."` (`MainWindow.cpp:1760-1764`).
   - `runDemHillshade` then finds the added layer by `QFileInfo(out).completeBaseName()`, renames it to `"음영기복"` (note: if `applyLegendCrsLabel` inside `addRasterFromPath` appended `" [EPSG:…]"` to the layer name at `LayerOps.cpp:1173-1175` and `mapLayersByName(completeBaseName)` misses, the layer keeps `<stem>_hillshade [EPSG:…]`; when matched, its title becomes `"음영기복"` without `ReferenceKind::kDemRelief`), calls `placeInLegendGroup(..., "참조 지도")` (flat root), syncs canvas, and overwrites the status bar (6 s, `MainWindow.cpp:1765-1772`):
     > `음영기복을 참조 지도에 올렸습니다.`
   - **Second invocation**: Writes `<stem>_hillshade_2.tif` (`DemHillshadeFile.cpp:135-136`) and adds **another** `"음영기복"` layer without removing the previous one (because `"음영기복"` is untagged by `ReferenceKind`, `MainWindow.cpp:1765-1770`).

### 1.4 Menu Row 3 — `「DEM 표현…」` (`MainWindow::editDemElevationClasses` & `KaDemClassDialog`, `S10`–`S11`)

1. **Step `S10` — Guard check and non-modal dialog open** (`MainWindowRibbon.cpp:416-417`, `MainWindowOverlay.cpp:269-297`):
   - Searches `ReferenceKind::find(proj, "dem")` for a valid `QgsRasterLayer` whose renderer is a `QgsSingleBandPseudoColorRenderer` (`MainWindowOverlay.cpp:274-283`).
   - If none exists, shows modal `QMessageBox::information` (`MainWindowOverlay.cpp:285-289`):
     - Title verbatim: `"DEM 표현"`
     - Body verbatim: `"먼저 DEM을 켜 주세요. 표고 자료를 불러오면 색 표현과 음영을 조절할 수 있습니다."`
   - Otherwise opens a **non-modal** `KaDemClassDialog(dem, this, m_canvas)` with `Qt::WA_DeleteOnClose` (`MainWindowOverlay.cpp:291-293`, `KaDemClassDialog.cpp:22`). A second click while open creates another non-modal `KaDemClassDialog` instance (`MainWindowOverlay.cpp:291-293`).
2. **Step `S11` — Controls, flat-site hint, and `"적용"`** (`KaDemClassDialog.cpp:19-132`):
   - Window title verbatim: `"DEM 표현"` (`KaDemClassDialog.cpp:21`).
   - Top hint verbatim (`KaDemClassDialog.cpp:25-26`):
     > `표고에 따라 색이 부드럽게 이어집니다. 유적처럼 높이차가 작은 곳은 「현재 화면에 색 맞추기」로 화면 안의 높이만으로 색을 나누세요.`
   - Row `"색 표현"` (`KaDemClassDialog.cpp:29-44`):
     - `QComboBox#demPreset`: `"전국 표준 (0–2000m)"` (`"national"`), `"저지대 강조"` (`"lowland"`), `"현재 화면 맞춤"` (`"viewport"`), initialized from `ka_hgis/dem_preset` (default `"national"`).
     - `QPushButton#demFitView`: `"현재 화면에 색 맞추기"`, tooltip verbatim `"지금 지도 화면에 보이는 가장 낮은 곳과 높은 곳으로 색띠를 다시 나눕니다. 지도를 옮기면 따라 바뀝니다."` (`KaDemClassDialog.cpp:36-39`); clicking selects `"viewport"` and immediately runs `applyStyle()` (`KaDemClassDialog.cpp:89-92`).
   - Checkbox `QCheckBox#demReliefEnabled`: `"지형 음영 합성"`, initialized from `ka_hgis/dem_relief_enabled` (default `true`, `KaDemClassDialog.cpp:45-48`).
   - Row `"수직과장"`: `QDoubleSpinBox#demExaggeration`, range `0.1..5.0`, step `0.1`, suffix `" 배"`, initialized from `ka_hgis/dem_z_factor` (default `1.0`, `KaDemClassDialog.cpp:49-55`).
   - Row `"음영 세기"`: `QSpinBox#demReliefStrength`, range `0..80`, suffix `" %"`, initialized from `qRound(ka_hgis/dem_relief_strength * 100)` (default `30`, `KaDemClassDialog.cpp:56-61`).
   - Buttons: `"적용"` and `"닫기"` (`KaDemClassDialog.cpp:66-72`).
   - On open, installs `DemPresentation::followCanvas(layer, canvas)` (`KaDemClassDialog.cpp:74`) and calls `suggestViewport()` (`KaDemClassDialog.cpp:75`, `KaDemClassDialog.cpp:95-107`): if preset != `"viewport"` and source is local (not `vsicurl`/`http`) and `DemPresentation::sampleRange` finds `high - low < 100.0` m in the current view, sets `m_status` to:
     > `지금 화면의 높이차는 약 %1 m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다.`
   - Clicking `"적용"` (`KaDemClassDialog::applyStyle`, `KaDemClassDialog.cpp:109-132`):
     - Validation messages in `m_status`: `"지도 화면에서 DEM 표현을 다시 열어 주세요."` (`:114`), `"화면 좌표를 변환하지 못했습니다. 작업 좌표계를 확인하세요."` (`:116`), `"이 화면에서 표고값을 읽지 못했습니다. DEM이 있는 곳으로 이동한 뒤 적용하세요."` (`:120`).
     - Calls `DemPresentation::apply(m_layer, preset, extent)`, writes `ka_hgis/dem_relief_enabled`, `ka_hgis/dem_z_factor`, and `ka_hgis/dem_relief_strength` onto `m_layer`, calls `LayerOps::ensureDemRelief(project, m_layer)` (which may create `<CacheLocation>/dem-relief/<sha256>.vrt` if `dem_layer` is geographic), syncs canvas, marks `project->setDirty(true)`, and sets `m_status` (`KaDemClassDialog.cpp:119-131`) either to `ka_hgis/dem_relief_error` or to:
       > `적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.`

### 1.5 Menu Row 4 — `「측량점으로 등고선…」` (`MainWindow::createSurveyContours`, `S12`–`S13`)

*(Also reachable from the dedicated ribbon chip `"등고선"` at `MainWindowRibbon.cpp:429-435`.)*

1. **Step `S12` — Survey guard, dialog setup, and live point-file inspection** (`MainWindowRibbon.cpp:418-419`, `MainWindowSurveyContour.cpp:104-125`, `KaSurveyContourDialog.cpp:23-255`, `SurveyContourBuilder.cpp:74-80`):
   - If `m_surveyPath.isEmpty()`, stops with `KaUserError::warn` (`MainWindowSurveyContour.cpp:105-112`):
     - Title: `"측량 등고선"`
     - Lines: `"조사를 연 뒤에 등고선을 만들 수 있습니다."` / `"결과 파일은 조사 폴더 안에만 둡니다."` / `"조사 열기 또는 새 조사로 시작한 뒤 다시 누르세요."`
   - If `findChild<QProgressDialog*>("contourProgress")` is active, shows status `"등고선을 만들고 있습니다. 끝나거나 취소를 누르세요."` (5 s) and returns (`MainWindowSurveyContour.cpp:114-116`).
   - Opens modal `KaSurveyContourDialog(crs, this)` titled `"측량 등고선"` (`MainWindowSurveyContour.cpp:122-124`, `KaSurveyContourDialog.cpp:25`):
     - `"좌표 파일"`: label `"파일을 고르세요"` + button `"찾기…"`, which opens `QFileDialog::getOpenFileName(this, "측량 좌표", QString(), "측량 좌표 (*.dxf *.xlsx *.xls *.csv)")` (`KaSurveyContourDialog.cpp:29-34`, `KaSurveyContourDialog.cpp:142-147`).
     - `"등고선 간격"`: `0.1..100` m, default `0.5` m (`0.1` m auto-selected when elevation span `< 2.0` m and untouched, `KaSurveyContourDialog.cpp:35-42`, `KaSurveyContourDialog.cpp:166-169`).
     - `"축 순서"`: checkbox `"X를 북쪽, Y를 동쪽으로 읽기"`; `"이상한 점"`: checkbox `"의심점 빼기"` (enabled as `"의심점 %1개 빼기"` when points have `z == 0` or `z >= 999` m, `KaSurveyContourDialog.cpp:43-53`, `KaSurveyContourDialog.cpp:188-194`).
     - Collapsible `"고급 설정"`: `"파일 좌표계"` (`"작업 좌표계와 같음 (%1)"`, `EPSG:5185 서부원점`, `EPSG:5186 중부원점`, `EPSG:5187 동부원점`, `EPSG:5179 UTM-K`, `EPSG:4326 경위도 WGS84`), `"최대 삼각형 변"` (`0..1000` m, `0` = `"쓰지 않음"`), `"단절선"` (`"높이가 있는 DXF 선을 단절선으로 쓰기"`), `"계산 격자"` (`0..5` m, `0` = `"자동"`, `KaSurveyContourDialog.cpp:55-103`).
     - Initial preview label verbatim: `"DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다."` (`KaSurveyContourDialog.cpp:105`). After reading a file via `SurveyPointReader::read` + `SurveyPointArrange::arrange` (`KaSurveyContourDialog.cpp:161-186`), shows `"점 %1개, 표고 %2–%3 m. 등고선 약 %4줄."` (+ `"줄이 많습니다. 간격을 넓히세요."` if `lines > 500`, `KaSurveyContourDialog.cpp:227-233`).
     - Buttons: `"등고선 만들기"` and `"취소"` (`KaSurveyContourDialog.cpp:110-111`). On `"등고선 만들기"`, validates non-empty path (`"좌표 파일을 고르세요."`), `m_report.fatal`, and `SurveyContourBuilder::rejection(usedPoints())` (`< 3` points: `"등고선을 만들려면 점이 3개 이상 필요합니다."`; collinear: `"점이 일직선이라 삼각망을 만들 수 없습니다."`, `KaSurveyContourDialog.cpp:126-134`, `SurveyContourBuilder.cpp:74-80`).
2. **Step `S13` — Background TIN gridding, contour generation, atomic swap, and `"참조 지도"` group styling** (`MainWindowSurveyContour.cpp:126-196`, `SurveyContourBuilder.h:48`, `SurveyContourBuilder.cpp:37`, `SurveyContourBuilder.cpp:120-207`, `SurveyContourWrite.cpp:23`, `SurveyContourWrite.cpp:79-290`, `SurveyContourStyle.cpp:70-166`):
   - Shows non-modal `QProgressDialog#contourProgress` (`"측량점으로 등고선을 계산하고 있습니다."`, button `"취소"`, `0..100`, `MainWindowSurveyContour.cpp:132-137`, `MainWindowSurveyContour.cpp:195-196`) and runs `SurveyContourTask` on `QgsApplication::taskManager()`.
   - `SurveyContourBuilder::build` + `writeSurveyContourFiles` (`SurveyContourBuilder.cpp:165-207`, `SurveyContourWrite.cpp:79-290`):
     - Densifies optional 3D breaklines, computes cell size (`autoCellSize` = clamped half median nearest-neighbour distance in `0.05..1.0` m, widened if needed to fit `kMaxCells = 4,000,000`, `SurveyContourBuilder.h:48`, `SurveyContourBuilder.cpp:82-118`, `SurveyContourBuilder.cpp:181-200`).
     - Interpolates a linear TIN (`GDALGridCreate(GGA_Linear, ...)` with `dfRadius = 0`, `dfNoDataValue = -9999.0`, `GDT_Float32`), optionally masks triangles with edge `> maxEdgeM`, flips north-up, and writes staging files under `<surveyDir>/측량등고선/<safeStem>/_new/`:
       1. `surface.tif` (`GTiff`, `GDT_Float32`, `COMPRESS=DEFLATE`, `TILED=YES`, `PREDICTOR=3`, `NoData = -9999.0`, work CRS, `SurveyContourWrite.cpp:129-153`).
       2. `contours.gpkg` (`GPKG`, work CRS) with layers `survey_points` (`wkbPoint25D`), `contour_lines` (`wkbLineString`, via `GDALContourGenerateEx` with `LEVEL_INTERVAL`, `LEVEL_BASE=0`, `NODATA=-9999`, `index_line = 1` every 5th line), and `contour_bands` (`wkbMultiPolygon`, via `GDALContourGenerateEx` with `POLYGONIZE=YES`, `FIXED_LEVELS`), clipped to the `survey_area` polygon WKT when non-empty (`MainWindowSurveyContour.cpp:121, 131`, `SurveyContourWrite.cpp:154-286`).
   - On completion on the UI thread (`MainWindowSurveyContour.cpp:158-190`):
     - Removes any existing layers tagged `ka_hgis/contour_group = "측량 등고선 · <stem>"` (`SurveyContourStyle::removeGroup`, `MainWindowSurveyContour.cpp:161`, `SurveyContourStyle.cpp:60-68`), atomically swaps `_new/{contours.gpkg, surface.tif}` into `<surveyDir>/측량등고선/<safeStem>/{contours.gpkg, surface.tif}` with `.old` rollback on lock failure (`SurveyContourBuilder::installFiles`, `SurveyContourBuilder.cpp:120-163`), and styles via `SurveyContourStyle::apply` (`SurveyContourStyle.cpp:70-166`).
     - **Layers & group added**: Creates or reuses group `"참조 지도"` at `layerTreeRoot()`, creates subgroup `"측량 등고선 · <stem>"`, and inserts three `ka_hgis/layer_role = "reference"` OGR vector layers (top to bottom inside subgroup: `"측량점"` [unchecked/hidden by default, `#5c6b73` 1.6 mm circle], `"등고선"` [rule-based `"계곡선"` `#3a2f24` 0.7 mm + `"주곡선"` `#8a7a66` 0.25 mm, curved `Malgun Gothic` 8 pt labels on index lines], `"색 구간"` [graduated 16-stop elevation fill, alpha `170/255`], `SurveyContourStyle.cpp:79-165`). Note: `surface.tif` stays on disk in `<surveyDir>/측량등고선/<safeStem>/surface.tif` and is **not** added as a map layer (`SurveyContourStyle.cpp:70-166`).
     - Zooms canvas to `"등고선"` (`LayerOps::zoomToLayerMax`), marks project dirty, and shows status bar (10 s, `MainWindowSurveyContour.cpp:184-189`):
       > `등고선 %1줄, 색 구간 %2개를 올렸습니다.` (+ trailing space and `result.warning` note from `SurveyContourBuilder.cpp:46-70`).
   - **Second invocation**: With the same file stem, replaces subgroup `"측량 등고선 · <stem>"` and atomically swaps `contours.gpkg` and `surface.tif` (rolling back and restoring the previous group if locked, `MainWindowSurveyContour.cpp:161-176`); with a different file stem, adds a sibling `"측량 등고선 · <otherStem>"` subgroup under `"참조 지도"`.

---

## Section 2 — The One-Click Download as It Is Today

### 2.1 Extent Choice, Source Bucket, and Tile Selection (`S14`–`S16`)

- **Step `S14` — Survey folder preflight & canvas view extent (NOT survey-area polygon)** (`MainWindowDownloads.cpp:93-103`, `MainWindowDownloads.cpp:118-131`, `ReferenceStorage.cpp:39-43`):
  - `MainWindow::startDemDownload()` requires an open survey file (`ReferenceStorage::surveyFolder(m_surveyPath)` returns `<surveyDir>` only if `m_surveyPath` is an existing file, `ReferenceStorage.cpp:39-43`, `MainWindowDownloads.cpp:94-101`).
  - **Which extent is downloaded**: **the current 2D map canvas view extent**, `QgsRectangle extent = m_canvas->extent()`, **not** the `survey_area` polygon (`MainWindowDownloads.cpp:122`). `startDemDownload` transforms that bounding box from `m_canvas->mapSettings().destinationCrs()` (work CRS `EPSG:5187` or `EPSG:5186`) to `EPSG:4326` (`MainWindowDownloads.cpp:123-126`) and passes `target = QDir(dir).filePath("DEM.tif")` (`MainWindowDownloads.cpp:131`).
- **Step `S15` — Copernicus GLO-30 host, tile URI pattern, and 1°×1° grid limits** (`DemDownloadService.cpp:25`, `DemDownloadService.cpp:85-94`, `DemDownloadService.cpp:127-147`):
  - **Host & tile path pattern** (`tileUri`, `DemDownloadService.cpp:85-94`):
    - Host: `https://copernicus-dem-30m.s3.amazonaws.com` (public AWS S3 bucket for Copernicus GLO-30 10-arcsecond / ~30 m Cloud-Optimized GeoTIFFs, accessed via GDAL `/vsicurl/`).
    - Tile name for integer southwest corner `(latitude, longitude)` in degrees (`EPSG:4326`):
      `Copernicus_DSM_COG_10_<N|S><|lat|:02d>_00_<E|W><|lon|:03d>_00_DEM`
    - Full GDAL URI:
      `/vsicurl/https://copernicus-dem-30m.s3.amazonaws.com/<name>/<name>.tif`
  - **How tiles for `wgs84Extent` are chosen** (`DemDownloadService.cpp:138-142`):
    - `lonFrom = floor(xMinimum)`, `lonTo = ceil(xMaximum) - 1`, `latFrom = floor(yMinimum)`, `latTo = ceil(yMaximum) - 1`.
    - Iterates outer loop `latitude = latFrom..latTo`, inner loop `longitude = lonFrom..lonTo` (`DemDownloadService.cpp:160-161`).
  - **Limits**:
    - World bounds check: `!isEmpty() && isFinite() && xMinimum >= -180.0 && xMaximum <= 180.0 && yMinimum >= -90.0 && yMaximum <= 90.0` (`DemDownloadService.cpp:127-129`).
    - Tile-count cap: `kMaximumTiles = 24` (`DemDownloadService.cpp:25`); `tileCount = (lonTo - lonFrom + 1) * (latTo - latFrom + 1)` must satisfy `1 <= tileCount <= 24` (`DemDownloadService.cpp:142-147`, at most 24 `1°×1°` tiles, e.g. up to `6°×4°` or `4°×6°`).
- **Step `S16` — Per-tile `/vsicurl/` open, HTTP timeouts, ocean 404 handling, and raster grid validation** (`DemDownloadService.cpp:32-48`, `DemDownloadService.cpp:96-105`, `DemDownloadService.cpp:152-211`):
  - Sets thread-local GDAL options (`ThreadConfiguration`, `DemDownloadService.cpp:34-39`): `GDAL_HTTP_CONNECTTIMEOUT=5` s, `GDAL_HTTP_TIMEOUT=15` s, `GDAL_HTTP_MAX_RETRY=0`, `GDAL_DISABLE_READDIR_ON_OPEN=EMPTY_DIR`.
  - Opens each tile read-only via `GDALOpenEx(tileUri(...), GDAL_OF_RASTER | GDAL_OF_READONLY, ...)` (`DemDownloadService.cpp:164-165`):
    - If `!source` and `CPLGetLastErrorMsg()` matches `HTTP response code: 404`, `HTTP error code : 404`, or `404 Not Found` (`isMissingTile`, `DemDownloadService.cpp:96-100`), increments `missingTiles` and continues (`DemDownloadService.cpp:169-170`); any other open error aborts immediately (`DemDownloadService.cpp:171-174`).
    - For every opened tile, verifies: `GDALGetRasterCount == 1`, `XSize > 0`, `YSize > 0`, finite 6-element `GeoTransform`, band 1 type `GDT_Float32`, north-up unrotated pixels (`transform[1] > 0`, `transform[5] < 0`, `transform[2] == 0`, `transform[4] == 0`), and `OSRGetAuthorityCode == "4326"` (`DemDownloadService.cpp:176-189`).
    - Across multiple tiles, verifies identical pixel sizes (`|dx - dx0| <= 1e-12`, `|dy - dy0| <= 1e-12`) and integer-cell origin alignment (`gridAligned`: `|delta/spacing - round(delta/spacing)| < 1e-5`, `DemDownloadService.cpp:102-105`, `DemDownloadService.cpp:190-198`).
    - If **all** tiles were 404 (`sources.empty()`), aborts (`DemDownloadService.cpp:206-209`); if some were 404 (`missingTiles > 0`), records non-fatal warning `W01` (`DemDownloadService.cpp:210-211`).

### 2.2 Mosaic, Reprojection (None), Clip, Output File, and Verification (`S17`)

- **Step `S17`** (`PreparedReferenceMap.cpp:17-28`, `DemDownloadService.cpp:148-151`, `DemDownloadService.cpp:213-299`):
  - **Staging & final output file**: `ReferenceMapPreparation::initializeStorage(result, "<surveyDir>/DEM.tif")` creates `QTemporaryDir("<surveyDir>/ka-hgis-reference-XXXXXX")` (`PreparedReferenceMap.cpp:20-21`); `DemDownloadService::prepare` clears `result.gpkgPath` and writes **one** file:
    `<surveyDir>/ka-hgis-reference-XXXXXX/dem.tif` (`DemDownloadService.cpp:148-151`, `DemDownloadService.cpp:298`). When `retainFiles()` is called (`MainWindowDownloads.cpp:64, 142`), that directory and `dem.tif` remain permanently in `<surveyDir>` as the layer's data source (until superseded by a later download in the same session and pruned on save by `pruneRetiredReferenceFolders`, `MainWindowDownloads.cpp:105-116`).
  - **Mosaic call**: In-memory `GDALBuildVRT("", sourceHandles.size(), sourceHandles.data(), nullptr, vrtOptions, &usageError)` with options `-resolution highest -vrtnodata nan -strict` (`DemDownloadService.cpp:213-236`).
  - **Reprojection / target CRS / resolution / resampling / no-data**:
    - **No reprojection occurs during download**: `dem.tif` stays in **`EPSG:4326`** (`MainWindowDownloads.cpp:148` comment: `// downloaded raster stays in WGS84`).
    - **Resolution & resampling**: Native Copernicus GLO-30 cell spacing (`10"` latitude = `1/360° ≈ 0.0002777778°`, longitude `10"` on mainland Korea latitudes; `-resolution highest` + integer pixel window `-srcwin` performs **zero resampling**, copying exact source `Float32` elevations, `DemDownloadService.cpp:214-215`, `DemDownloadService.cpp:243-250`).
    - **No-data**: VRT mosaic nodata is `NaN` (`-vrtnodata nan`, `DemDownloadService.cpp:216-217`), propagated into `dem.tif` by `GDALTranslate` (missing ocean tiles inside the mosaic become `NaN`).
  - **Window clip & GeoTIFF write**: Computes integer pixel window `x0 = clamp(floor((xmin - gt[0]) / gt[1]), 0, width)`, `x1 = clamp(ceil((xmax - gt[0]) / gt[1]), 0, width)`, `y0 = clamp(floor((ymax - gt[3]) / gt[5]), 0, height)`, `y1 = clamp(ceil((ymin - gt[3]) / gt[5]), 0, height)` (`DemDownloadService.cpp:245-250`), then calls `GDALTranslate(rasterPath, mosaic, translateOptions, &usageError)` with `-of GTiff -co TILED=YES -co COMPRESS=DEFLATE -co PREDICTOR=3 -co BIGTIFF=IF_SAFER -srcwin x0 y0 (x1-x0) (y1-y0)` (`DemDownloadService.cpp:255-277`) and `GDALClose` (`DemDownloadService.cpp:278-281`).
  - **Post-write verification**: Reopens `dem.tif` via `GDALOpenEx`, verifying `RasterCount == 1`, `XSize == x1 - x0`, `YSize == y1 - y0`, `GDALGetGeoTransform == CE_None`, `|savedTransform[1] - transform[1]| <= 1e-12`, `|savedTransform[5] - transform[5]| <= 1e-12`, and `QFileInfo(rasterPath).size() > 0` (`DemDownloadService.cpp:285-295`).

### 2.3 Every Stop Condition (and Partial-Ocean Warning) of the One-Click Download

*(20 stop conditions `SC01`–`SC20`, plus 1 non-fatal partial-ocean warning `W01`.)*

| ID | Trigger condition | C++ `<file name>:line` | Verbatim user-visible text (or silent guard) |
| --- | --- | --- | --- |
| `SC01` | `!m_canvas` on button click / `startDemDownload`, or `m_isOpeningSurvey \|\| m_closingWindow` in `startFileDownload`, or window closed / survey switched before job finishes | `MainWindowOverlay.cpp:301-302`, `MainWindowDownloads.cpp:38, 49-51, 119` | *(Silent early return; no dialog or status message.)* |
| `SC02` | Another reference download (`m_referenceDownload`) is already running | `MainWindowDownloads.cpp:39-41` | Status bar (5 s): `지도 자료를 받고 있습니다. 완료를 기다리거나 진행 창에서 취소하세요.` |
| `SC03` | No survey is open (`ReferenceStorage::surveyFolder(m_surveyPath)` is empty) | `MainWindowDownloads.cpp:94-101`, `MainWindowDownloads.cpp:120-121` | `Notice::Info` titled `DEM`: `먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.` |
| `SC04` | Canvas extent → `EPSG:4326` throws `QgsException` | `MainWindowDownloads.cpp:124-130` | `Notice::Warning` titled `DEM`: `화면의 좌표 범위를 계산하지 못했습니다. 조사 좌표계를 확인하고 다시 실행하세요.` |
| `SC05` | User cancels (`QProgressDialog::canceled` / `feedback->isCanceled()` / `cancellationRequested()`) at any checkpoint | `MainWindowDownloads.cpp:53-55`, `DemDownloadService.cpp:114-119, 126, 162, 167, 232, 273, 284, 296` | Status bar (6 s): `DEM 내려받기를 취소했습니다. 기존 자료는 그대로입니다.` *(worker sets `result.error = "DEM 내려받기를 취소했습니다. 기존 지도는 유지됩니다."`)* |
| `SC06` | `wgs84Extent` is empty, non-finite, or outside `[-180..180, -90..90]` | `DemDownloadService.cpp:127-132`, `MainWindowDownloads.cpp:57-60` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM을 받을 지도 범위가 올바르지 않습니다. 조사구역으로 이동한 뒤 다시 내려받으세요.` |
| `SC07` | `targetBasePath` is empty or has an empty file name | `DemDownloadService.cpp:133-136` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 저장 위치가 없습니다. 저장 폴더를 선택하고 다시 내려받으세요.` |
| `SC08` | `tileCount <= 0` or `tileCount > 24` (`kMaximumTiles`) | `DemDownloadService.cpp:25`, `DemDownloadService.cpp:142-147` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM을 받을 범위가 너무 넓습니다. 한 번에 최대 24칸까지 받을 수 있습니다. 조사구역으로 확대하고 다시 내려받으세요.` |
| `SC09` | `QTemporaryDir("<surveyDir>/ka-hgis-reference-XXXXXX")` is invalid | `PreparedReferenceMap.cpp:20-25`, `DemDownloadService.cpp:148` | `Notice::Warning` titled `DEM 내려받기 실패`: `지도를 저장할 임시 폴더를 만들지 못했습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인하세요.` |
| `SC10` | `GDALOpenEx` on a tile fails and error is not HTTP 404 | `DemDownloadService.cpp:96-100`, `DemDownloadService.cpp:164-174` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 서버에 연결하지 못했거나 응답이 늦어 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요. 기존 지도는 유지됩니다.` |
| `SC11` | Opened tile is not 1-band `GDT_Float32`, north-up unrotated finite geotransform, or authority code `"4326"` | `DemDownloadService.cpp:176-189` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 자료의 좌표나 격자 형식이 예상과 다릅니다. 기존 지도는 유지됩니다. 다른 표고 자료를 불러오세요.` |
| `SC12` | Multi-tile pixel spacing differs (`> 1e-12`) or tile origins fail `gridAligned` (`>= 1e-5` cell) | `DemDownloadService.cpp:102-105`, `DemDownloadService.cpp:192-198` | `Notice::Warning` titled `DEM 내려받기 실패`: `이 범위의 DEM 격자 간격이 서로 다릅니다. 원본 정밀도를 유지하려면 범위를 좁혀 각각 내려받으세요.` |
| `SC13` | Every tile in extent returned HTTP 404 (`sources.empty()`, open sea) | `DemDownloadService.cpp:206-209` | `Notice::Warning` titled `DEM 내려받기 실패`: `이 범위에는 제공되는 DEM 자료가 없습니다. 바다를 제외한 조사구역으로 이동하고 다시 내려받으세요.` |
| `SC14` | `GDALBuildVRTOptionsNew` returns `nullptr` | `DemDownloadService.cpp:219-224` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 연결 설정을 만들지 못했습니다. 앱을 다시 실행하고 내려받으세요.` |
| `SC15` | `GDALBuildVRT` fails (`!mosaic`, `usageError`, `callbackFailed`, or `CE_Failure`) | `DemDownloadService.cpp:230-236` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 자료를 하나로 연결하지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.` |
| `SC16` | `GDALGetGeoTransform(mosaic)` fails, or clipped window `x1 <= x0 \|\| y1 <= y0` | `DemDownloadService.cpp:239-242`, `DemDownloadService.cpp:251-254` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM의 지도 범위를 확인하지 못했습니다. 다시 내려받으세요.` / `선택 범위에 표고 자료가 없습니다. 조사구역을 확인한 뒤 다시 내려받으세요.` |
| `SC17` | `GDALTranslateOptionsNew` returns `nullptr` | `DemDownloadService.cpp:261-266` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM 저장 설정을 만들지 못했습니다. 다시 내려받으세요.` |
| `SC18` | `GDALTranslate` fails (`!output`, `usageError`, `callbackFailed`, `CE_Failure`) or `GDALClose != CE_None` | `DemDownloadService.cpp:272-281` | `Notice::Warning` titled `DEM 내려받기 실패`: `DEM을 끝까지 내려받거나 저장하지 못했습니다. 인터넷 연결과 저장 공간을 확인하세요. 기존 지도는 유지됩니다.` / `DEM 파일 저장을 마무리하지 못했습니다. 저장 공간을 확인한 뒤 다시 내려받으세요.` |
| `SC19` | Reopen `GDALOpenEx` verification fails (band count, dimensions, geotransform, spacing, or `size <= 0`) or uncaught C++ exception in `prepare` | `DemDownloadService.cpp:285-295`, `DemDownloadService.cpp:300-303` | `Notice::Warning` titled `DEM 내려받기 실패`: `저장한 DEM을 다시 확인하지 못했습니다. 기존 지도는 유지됩니다. 다른 저장 폴더에서 다시 내려받으세요.` *(exception fallback: `DEM 내려받기를 완료하지 못했습니다. 인터넷 연결과 저장 공간을 확인한 뒤 다시 시도하세요. 기존 지도는 유지됩니다.`; empty-error fallback in `startFileDownload` `:58-59`: `자료를 받지 못했습니다. 인터넷 연결과 저장 공간을 확인하고 다시 시도하세요.`)* |
| `SC20` | `LayerOps::addDemElevationRaster` fails on UI thread, or `apply` throws an exception | `MainWindowDownloads.cpp:76-80`, `MainWindowDownloads.cpp:138-141`, `BasemapOps.cpp:1998-2023` | `Notice::Warning` titled `DEM 표시 실패`: `프로젝트가 없습니다.` / `DEM 파일이 없습니다.` / `국토지리원 DEM(.img)을 열 수 없습니다: %1` / `DEM 레이어를 넣지 못했습니다.` *(or on thrown exception: `자료를 받았지만 지도에 표시하지 못했습니다. 현재 작업을 저장한 뒤 파일함에서 다시 열어 주세요.\n%1`)* |
| `W01` *(non-fatal)* | Some tiles returned 404 while `>= 1` tile succeeded (`missingTiles > 0`), or `ensureDemRelief` recorded `ka_hgis/dem_relief_error` | `DemDownloadService.cpp:210-211`, `MainWindowDownloads.cpp:153-155, 158` | `Notice::Warning` titled `DEM 자료 확인`: `요청 범위 중 %1칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다.` *(relief error if any: `Notice::Warning` titled `DEM 음영 표시 실패` with `reliefSource` message from `DemPresentation.cpp:166, 181, 193, 209, 220`)* |

---

## Section 3 — How a DEM Is Shown

### 3.1 Colour Classes, Presets, Resampling, and Opacity (`S18`)

- **Step `S18`** (`DemPresentation.cpp:34-108`, `BasemapOps.cpp:1920-1956`):
  - `DemPresentation::apply` requires a valid 1-band `"gdal"` `QgsRasterLayer` (`DemPresentation.cpp:47-49`) and builds a **continuous linear** `QgsSingleBandPseudoColorRenderer` on band `1` using `QgsColorRampShader` (`ShaderInterpolationMethod::Linear`, `setClip(false)` so finite elevations outside `[minimum, maximum]` clamp to the end colours while GDAL band NoData stays transparent, `DemPresentation.cpp:80-96`).
  - Both zoomed-in and zoomed-out raster resamplers are set to `QgsBilinearRasterResampler` (`DemPresentation.cpp:97-100`).
  - **Default opacity** of the DEM colour layer: `1.0` (`100%`), set once when `ka_hgis/dem_display_version != 1` so later user opacity edits are preserved (`DemPresentation.cpp:101-104`).
  - **16 stops (`kStops`) and their exact break values (m) and hex colours verbatim** (`DemPresentation.cpp:35-39`, `DemPresentation.cpp:67-79`):

| Stop `#` | `"national"` (`전국 표준 (0–2000m)`) break `z` (m) | `"lowland"` (`저지대 강조`) formula & exact `z` (m) (`z <= 200 ? 0.5*z : 100 + (z-200)*(1900/1800)`) | `"viewport"` (`현재 화면 맞춤`) break `z` (m) | Hex colour (`kStops[i].color`) | Item label (`"%1 m"`, `0` decimals) |
| --- | --- | --- | --- | --- | --- |
| 0 | `0` | `0` | `min + (max - min) * 0 / 15` | `#72c6b5` | `0 m` / `0 m` / `<round> m` |
| 1 | `10` | `5` | `min + (max - min) * 1 / 15` | `#81ceaa` | `10 m` / `5 m` / `<round> m` |
| 2 | `25` | `12.5` | `min + (max - min) * 2 / 15` | `#91d396` | `25 m` / `12 m` (`12.5` rounded) / `<round> m` |
| 3 | `50` | `25` | `min + (max - min) * 3 / 15` | `#a5d680` | `50 m` / `25 m` / `<round> m` |
| 4 | `75` | `37.5` | `min + (max - min) * 4 / 15` | `#bad975` | `75 m` / `38 m` / `<round> m` |
| 5 | `100` | `50` | `min + (max - min) * 5 / 15` | `#d0dc78` | `100 m` / `50 m` / `<round> m` |
| 6 | `150` | `75` | `min + (max - min) * 6 / 15` | `#e0da7b` | `150 m` / `75 m` / `<round> m` |
| 7 | `200` | `100` | `min + (max - min) * 7 / 15` | `#ebd27a` | `200 m` / `100 m` / `<round> m` |
| 8 | `300` | `205.5555556` (`100 + 100*19/18`) | `min + (max - min) * 8 / 15` | `#edbe70` | `300 m` / `206 m` / `<round> m` |
| 9 | `400` | `311.1111111` (`100 + 200*19/18`) | `min + (max - min) * 9 / 15` | `#e8a666` | `400 m` / `311 m` / `<round> m` |
| 10 | `600` | `522.2222222` (`100 + 400*19/18`) | `min + (max - min) * 10 / 15` | `#d98b5a` | `600 m` / `522 m` / `<round> m` |
| 11 | `800` | `733.3333333` (`100 + 600*19/18`) | `min + (max - min) * 11 / 15` | `#c47452` | `800 m` / `733 m` / `<round> m` |
| 12 | `1000` | `944.4444444` (`100 + 800*19/18`) | `min + (max - min) * 12 / 15` | `#ad6650` | `1000 m` / `944 m` / `<round> m` |
| 13 | `1300` | `1261.1111111` (`100 + 1100*19/18`) | `min + (max - min) * 13 / 15` | `#b69886` | `1300 m` / `1261 m` / `<round> m` |
| 14 | `1600` | `1577.7777778` (`100 + 1400*19/18`) | `min + (max - min) * 14 / 15` | `#d1c5bb` | `1600 m` / `1578 m` / `<round> m` |
| 15 | `2000` | `2000` | `min + (max - min) * 15 / 15` (`= max`) | `#f4f3ef` | `2000 m` / `2000 m` / `<round> m` |

  - *(Legacy helper note: `BasemapOps.cpp:1831-1896` also contains an older 6-anchor purple-to-red discrete class builder `buildDemElevationClasses`, and `LayerOps::applyDemElevationStyle` at `BasemapOps.cpp:1928-1956` accepts non-empty legacy `style.classes` as `"custom"`, but every live UI caller passes an empty `DemElevationStyle`, delegating directly to `DemPresentation::apply` at `BasemapOps.cpp:1931-1932`.)*

### 3.2 How Elevation Range Is Sampled and What `followCanvas` Changes (`S19`–`S20`)

- **Step `S19` — Two range-sampling paths** (`DemPresentation.cpp:53-64`, `DemPresentation.cpp:141-159`, `DemAnalyzer.cpp:104-203`, `KaDemClassDialog.cpp:95-107`):
  1. **`"viewport"` preset classification (`DemPresentation::apply`, `DemPresentation.cpp:53-64`)**: Intersects `extent` (already transformed into `layer->crs()`) with `layer->extent()`. If non-empty, calls `layer->dataProvider()->bandStatistics(1, Min | Max, clipped, 25000)` (up to `25,000` provider sample pixels). Sets `minimum = floor(stats.minimumValue)` and `maximum = ceil(stats.maximumValue)`; if `maximum <= minimum`, sets `maximum = minimum + 1.0`.
  2. **Flat-site hint on dialog open (`DemPresentation::sampleRange`, `DemPresentation.cpp:141-159`, called by `KaDemClassDialog::suggestViewport` at `KaDemClassDialog.cpp:102`)**: Calls `DemAnalyzer::readFloatWindow(layer->source(), xmin, ymin, xmax, ymax, /*maxEdge=*/128, &z, &info, nullptr)` (`DemAnalyzer.cpp:104-203`), which inverts the GDAL geotransform, clamps pixel bounds `[x0..x1, y0..y1]` (requiring `winW >= 2 && winH >= 2`), downsamples via `GDALRasterBand::RasterIO(GF_Read, ..., outW, outH, GDT_Float32)` so `max(outW, outH) <= 128`, and scans finite non-NoData pixels for exact `min`/`max`.
- **Step `S20` — `DemPresentation::followCanvas` on pan and zoom** (`DemPresentation.cpp:117-139`):
  - Attaches a single-shot child `QTimer` named `"demViewportTimer"` (`350` ms interval) to `layer` once (`DemPresentation.cpp:118-122`).
  - On every `QgsMapCanvas::extentsChanged` (pan or zoom), if `layer->customProperty("ka_hgis/dem_preset") == "viewport"`, restarts the 350 ms debounce timer (`DemPresentation.cpp:124-126`); for `"national"`, `"lowland"`, or `"custom"`, pan and zoom change **nothing**.
  - When the 350 ms timer fires and preset is still `"viewport"`, transforms `canvas->extent()` from `canvas->mapSettings().destinationCrs()` to `layer->crs()` and calls `DemPresentation::apply(layer, "viewport", extent)`, rebuilding the 16-stop `QgsSingleBandPseudoColorRenderer`, reinstalling `DemColorRampLegend`, and triggering repaint (`DemPresentation.cpp:127-133`, `DemPresentation.cpp:105-106`).
  - If `apply` returns `false` (panned outside DEM or only NoData in view), keeps the previous colour ramp and emits canvas warning `"현재 화면에 표고 자료가 없어 이전 색띠를 유지합니다."` (`DemPresentation.cpp:133-134`); on `QgsCsException`, emits `"화면 좌표를 변환하지 못해 이전 색띠를 유지합니다."` (`DemPresentation.cpp:135-137`).

### 3.3 Dynamic Hillshade Layer (`reliefSource` + `ensureDemRelief`, `S21`–`S22`)

- **Step `S21` — Metre-CRS source preparation (`DemPresentation::reliefSource`)** (`DemPresentation.cpp:161-224`):
  - Requires a valid `"gdal"` `QgsRasterLayer` with `layer->crs().isValid()` (`DemPresentation.cpp:164-168`; error `"표고 자료의 좌표계를 확인할 수 없어 음영을 만들지 못했습니다. 좌표계가 있는 DEM을 불러오세요."`).
  - **Projected DEMs (e.g. NGII `.img` in `EPSG:5186` / `5187` / `5179`)**: `layer->crs().mapUnits() == Qgis::DistanceUnit::Meters` is `true` → returns `layer->source()` directly with **no file written** (`DemPresentation.cpp:169`).
  - **Geographic DEMs (e.g. Copernicus `dem.tif` in `EPSG:4326`)**: Because slope requires horizontal units in metres matching vertical metres, builds a cached **GDAL warped VRT** (no heavy GeoTIFF copy) (`DemPresentation.cpp:170-223`):
    - Target CRS: `workCrs` if `workCrs.authid()` is `"EPSG:5186"` or `"EPSG:5187"`, otherwise fallback `"EPSG:5186"` (`DemPresentation.cpp:170-172`).
    - Cache path: `<QStandardPaths::CacheLocation>/dem-relief/<sha256>.vrt`, where `<sha256>` hashes `source | size | mtime_ms | srcWkt | dstWkt` (`DemPresentation.cpp:173-187`).
    - Calls `GDALWarp` via `GDALWarpAppOptionsNew({"-of", "VRT", "-s_srs", srcWkt, "-t_srs", dstWkt, "-r", "bilinear"})` into `warp-XXXXXX.vrt` and renames to `<sha256>.vrt` (`DemPresentation.cpp:188-223`).
- **Step `S22` — Live QGIS hillshade layer `"지형 음영"` (`LayerOps::ensureDemRelief`)** (`BasemapOps.cpp:2039-2123`):
  - **How it is rendered**: **Not** own Horn file-writing code (`DemHillshadeFile.cpp` is only for Menu Row 2 `"DEM 파일로 음영 만들기…"`). Instead, creates/updates a live `QgsRasterLayer(source, "지형 음영", "gdal")` rendered on the fly by QGIS's native **`QgsHillshadeRenderer`** on band `1` (`BasemapOps.cpp:2064-2077`):
    - Azimuth: `315.0°` (`BasemapOps.cpp:2072`)
    - Altitude: `45.0°` (`BasemapOps.cpp:2072`)
    - Multi-directional: `true` (`renderer->setMultiDirectional(true)`, `BasemapOps.cpp:2076`)
    - `zFactor`: `std::clamp(demLayer->customProperty("ka_hgis/dem_z_factor", 1.0), 0.1, 5.0)` (`BasemapOps.cpp:2073-2075`, default `1.0`)
    - Resampling: `Bilinear` for both zoomed-in and zoomed-out at `Qgis::RasterResamplingStage::Provider` (`BasemapOps.cpp:2080-2082`)
    - Blend mode: **`QPainter::CompositionMode_Multiply`** (`BasemapOps.cpp:2083`)
    - **Default opacity**: `0.30` (`30%`), clamped to `[0.0, 0.80]` from `ka_hgis/dem_relief_strength` (`BasemapOps.cpp:2084-2086`)
    - Sheet legend flag: `ka_hgis/omit_sheet_legend = true` (omits `"지형 음영"` from print-layout legends, `BasemapOps.cpp:2087`), `ka_hgis/layer_role = "reference"`, `ka_hgis/reference_kind = "dem_relief"` (`BasemapOps.cpp:2088-2089`).
  - **Legend placement & observer**: Placed in the same parent group as `demNode` (`root`) at `parent->children().indexOf(demNode)` — **immediately above** the DEM layer so Multiply blending darkens the DEM colours underneath (`BasemapOps.cpp:2091-2103`). Visibility is `enabled && (!demNode || demNode->isVisible())` (`BasemapOps.cpp:2106`), and `"demVisibilityObserver"` syncs manual legend checkbox toggles on `"지형 음영"` back into `demLayer`'s `ka_hgis/dem_relief_enabled` (`BasemapOps.cpp:2108-2120`).

### 3.4 Custom Legend Node (`DemColorRampLegend`, `S23`)

- **Step `S23`** (`DemColorRampLegend.cpp:21-226`, installed at `DemPresentation.cpp:105, 114` and `BasemapOps.cpp:1953`):
  - Replaces QGIS's long discrete class list with a single custom `QgsColorRampLegendNode` (`DemLegendNode`, key `"ka_dem_elevation"`, `DemColorRampLegend.cpp:162-226`).
  - **Dimensions**: `104 × 156` logical px in the layer tree (`SizeHintRole`, colour bar `18 × 108` px at `(5, 24)` with `0.6` px `#5a5f64` border), and `27 × 40.5` mm in print layouts (`kLayoutSize`, `DemColorRampLegend.cpp:21-27`, `DemColorRampLegend.cpp:140-142`, `DemColorRampLegend.cpp:181`, `DemColorRampLegend.cpp:190-202`).
  - **Tick selection (`snapshot`, `DemColorRampLegend.cpp:51-90`)**: Starts with `minimum`, appends standard stops `{0, 100, 200, 500, 1000, 1500, 2000}` strictly inside `(minimum, maximum)`, appends `maximum`. If `< 5` ticks, replaces with `5` evenly spaced ticks `lerp(minimum, maximum, i / 4.0)`; if `> 7` ticks, subsamples to `7` ticks (`qRound(i * (all.size() - 1.0) / 6.0)`). Ticks are spaced **equally along the vertical bar** (`DemColorRampLegend.cpp:37-46`, `DemColorRampLegend.cpp:145-153`), while bar colours are sampled from the actual shader at the piecewise-linear elevation of each vertical position.
  - **Verbatim legend texts** (`DemColorRampLegend.cpp:119`, `DemColorRampLegend.cpp:158`, `DemColorRampLegend.cpp:185`):
    - Header (`11` px bold `#1f2328`): `"표고 (m)"`
    - Footer (`9` px `#555a5f`): `"비선형 눈금"`
    - Tooltip / accessible text: `"표고 (m) · 비선형 눈금\n<comma-separated ticks>"`

### 3.5 Project Persistence and Restore on Open (`S24`)

- **Step `S24`** (`SurveyStorage.cpp:1179-1235`, `SurveyBundle.cpp:193-196`, `MainWindowSession.cpp:172-180`, `MainWindowSession.cpp:284-305`, `MainWindowSession.cpp:403-410`, `MainWindowSession.cpp:1702-1708`, `DemPresentation.cpp:110-115`):
  - **Where project state is stored**: `SurveyStorage::writeEmbedded` writes the full `QgsProject` XML into the survey GeoPackage's `qgis_projects` table (`SurveyStorage.cpp:1179-1235`, with absolute paths rewritten relative to the survey directory via `SurveyBundle::rememberSurveyDir`), alongside any companion `<survey>.qgz` (`MainWindowSession.cpp:284-305`).
  - **What is stored in the `QgsProject` XML for the DEM & hillshade**:
    - On the DEM `QgsRasterLayer`: data source path (`<surveyDir>/ka-hgis-reference-XXXXXX/dem.tif` or external `.img`/`.tif`), layer title (`"지표모델(DSM) 30m · Copernicus"` or `"DEM"`), layer opacity, `QgsSingleBandPseudoColorRenderer` XML (16 colour-ramp items + classification min/max + bilinear resampler), and custom properties:
      - `ka_hgis/layer_role = "reference"` (`LayerOps.cpp:1055`)
      - `ka_hgis/reference_kind = "dem"` (`BasemapOps.cpp:2018`, `BasemapDsm.h:33`)
      - `ka_hgis/crs_label = "<EPSG:...>"` (`LayerOps.cpp:1179`)
      - `ka_hgis/dem_preset = "national" | "lowland" | "viewport" | "custom"` (`DemPresentation.cpp:40, 103`, `BasemapOps.cpp:1952`)
      - `ka_hgis/dem_display_version = 1` (`DemPresentation.cpp:41, 104`)
      - `ka_hgis/dem_cover_wgs84 = "<xmin>,<ymin>,<xmax>,<ymax>"` (downloaded Copernicus DSM only, `MainWindowDownloads.cpp:149-151`)
      - `ka_hgis/dem_relief_enabled` (`bool`, default `true`), `ka_hgis/dem_z_factor` (`double`, default `1.0`), `ka_hgis/dem_relief_strength` (`double`, default `0.30`) (`KaDemClassDialog.cpp:122-124`, `BasemapOps.cpp:2118`).
    - On `"지형 음영"`: `ka_hgis/layer_role = "reference"`, `ka_hgis/reference_kind = "dem_relief"`, `ka_hgis/omit_sheet_legend = true`, `QgsHillshadeRenderer` XML, Multiply blend mode, opacity, and `<CacheLocation>/dem-relief/<sha256>.vrt` (or direct metre DEM path). `SurveyBundle::isRegeneratedCache` (`SurveyBundle.cpp:193-196`) deliberately **skips** copying `/dem-relief/` `.vrt` files into `<surveyDir>/가져온자료` on save/Save As because `ensureDemRelief` regenerates them on open.
  - **What runs on open**: All three session-open paths (`finishOpenedProject` at `MainWindowSession.cpp:172-180`, `openSurveyGpkg` layers-only fallback at `MainWindowSession.cpp:403-410`, and `.qgz` open at `MainWindowSession.cpp:1702-1708`) find every valid `BasemapDsm::isDemLayer` raster and call:
    1. `DemPresentation::restore(dem)` (`DemPresentation.cpp:110-115`): if `ka_hgis/dem_display_version != 1` (legacy dark discrete project), migrates in memory via `apply(dem)` (`"national"`); otherwise reinstalls the transient C++ `DemColorRampLegend` (`QgsProject` XML does not serialize custom `QgsMapLayerLegend` subclasses).
    2. `DemPresentation::followCanvas(dem, m_canvas)`: reconnects the `"demViewportTimer"` pan/zoom listener.
    3. If the DEM tree node is visible, `LayerOps::ensureDemRelief(project, dem)`: recreates `<CacheLocation>/dem-relief/<sha256>.vrt` if the cache was cleared and re-establishes `"지형 음영"` above the DEM.

---

## Section 4 — The Import of an NGII DEM File (`.img`)

1. **Step `S25` — Accepted extensions and GDAL open** (`MainWindow.cpp:1689-1692`, `BasemapOps.cpp:2002-2014`):
   - `MainWindow::importDemElevationRaster` file dialog filter verbatim: `"국토지리원 DEM (*.img *.IMG *.tif *.tiff *.TIF *.TIFF)"` (`MainWindow.cpp:1691`) — accepts ERDAS Imagine `.img` / `.IMG` and GeoTIFF `.tif` / `.tiff` / `.TIF` / `.TIFF`.
   - `LayerOps::addDemElevationRaster` opens the chosen path directly via `new QgsRasterLayer(path, "DEM", "gdal")` (GDAL `HFA` driver for `.img`, `GTiff` for `.tif`/`.tiff`) and checks `rl->isValid() && rl->bandCount() >= 1` (`BasemapOps.cpp:2007-2014`).
2. **Step `S26` — How CRS is found when the `.img` file has none (`TopographicSourceCrs` is NOT used)** (`BasemapOps.cpp:2007, 2017, 2026`, `LayerOps.cpp:1171-1181`, `DemPresentation.cpp:164-168`, `TopographicSourceCrs.cpp:175, 359`, `KaTopographicScopePanel.cpp:7, 247`):
   - **No fallback CRS inference or assignment is performed for `.img` DEMs**:
     - `TopographicSourceCrs` (`TopographicSourceCrs.cpp:175, 359`) is included and called **only** in `KaTopographicScopePanel.cpp:7, 247` for vector NGII DXF/SHP topographic sheets; it is **never** called from `MainWindow::importDemElevationRaster`, `LayerOps::addDemElevationRaster`, `DemPresentation`, or `DemAnalyzer`.
     - Unlike `MainWindow::addRasterFromPath` (`MainWindow.cpp:2896-2900`, which assigns `QgsProject::instance()->crs()` to CRS-less GeoTIFFs imported via Menu Row 2 or general raster import), `LayerOps::addDemElevationRaster` (`BasemapOps.cpp:1996-2037`) relies solely on what GDAL/QGIS reads from the `.img` header or companion `.aux.xml` / `.prj`.
     - If `rl->crs().isValid()` is `false`:
       1. `LayerOps::applyLegendCrsLabel(rl)` (`LayerOps.cpp:1176-1180`) sees empty `authid()` and leaves `ka_hgis/crs_label` unset.
       2. `DemPresentation::apply(rl, "national")` still colours band 1 (`0–2000 m`), because `"national"` does not transform coordinates (`DemPresentation.cpp:44-108`).
       3. `LayerOps::ensureDemRelief` → `DemPresentation::reliefSource` fails `!layer->crs().isValid()` (`DemPresentation.cpp:164-168`), sets `ka_hgis/dem_relief_error = "표고 자료의 좌표계를 확인할 수 없어 음영을 만들지 못했습니다. 좌표계가 있는 DEM을 불러오세요."` (`BasemapOps.cpp:2055-2060`), and creates **no** `"지형 음영"` layer (`importDemElevationRaster` at `MainWindow.cpp:1694-1702` does not inspect `ka_hgis/dem_relief_error`, so no warning popup is shown until the user clicks `"적용"` in `"DEM 표현…"` at `KaDemClassDialog.cpp:129-130`).
       4. `BasemapOps.cpp:2030` (`!rl->crs().isGeographic()`) is `true` on an invalid `QgsCoordinateReferenceSystem`, so if `0 < width < 20000 && height < 20000`, `zoomToLayerMax` zooms to raw pixel/source coordinates; otherwise `syncMapCanvas` runs (`BasemapOps.cpp:2029-2035`).
3. **Step `S27` — Elevation unit and NoData handling** (`DemPresentation.cpp:47-49, 80-96, 152-157, 169`, `BasemapOps.cpp:2041-2044, 2075`):
   - **Vertical unit**: Assumed to be **metres** without unit conversion (`0–2000 m` stops and `" m"` suffix in `DemPresentation.cpp:35-39, 76, 89`; `BasemapOps.cpp:2075` comment: `// Horizontal and vertical units are both metres.`). Vertical unit metadata (`GDALRasterBand::GetUnitType`) is `not in the source`.
   - **NoData**: Read automatically by QGIS's `"gdal"` `QgsRasterDataProvider` from the `.img`/`.tif` band NoData metadata (e.g. `-9999`, `-32768`, `NaN`). `setClip(false)` clamps finite values below `minimum` or above `maximum` to the end colours while provider NoData pixels render transparent (`DemPresentation.cpp:82`); `DemPresentation::sampleRange` explicitly skips `!std::isfinite(value)` and `info.hasNoData && value == info.noData` (`DemPresentation.cpp:152-154`). If the `.img` encodes missing cells as sentinel numbers without setting GDAL band NoData metadata, no custom sentinel filter is applied (`not in the source`).
4. **Step `S28` — Single-file replacement (no mosaicking) and where the file is kept** (`MainWindow.cpp:1689`, `BasemapOps.cpp:2007, 2019`, `SurveyBundle.cpp:183-245`):
   - **Mosaicking**: **None**. `QFileDialog::getOpenFileName` (singular, `MainWindow.cpp:1689`) picks a **single** file, and `removeReferenceKind(project, ReferenceKind::kDem)` (`BasemapOps.cpp:2019`) removes any previously loaded DEM before adding the new one. Multi-sheet NGII `.img` mosaicking is `not in the source`.
   - **Where the copy is kept**: **No copy is made inside the survey folder** when importing an external `.img`/`.tif` from a user directory, nor on normal `"저장"` (`Ctrl+S`). `LayerOps::addDemElevationRaster` points `QgsRasterLayer` directly at the user's `path` (`BasemapOps.cpp:2007`), and `SurveyBundle::collectIntoSurvey` (`SurveyBundle.cpp:213-215`) only copies external layer files into `<surveyDir>/가져온자료/<safeLayerName>/` when `fromPrevious` (moving between survey folders) or `SurveyBundle::isAppManagedPath(file)` (`QStandardPaths` app/temp roots or `.ka-survey-gen-*`, `SurveyBundle.cpp:183-191`) is true — leaving a user-folder `.img` referenced at its original external path.

---

## Section 5 — The Rust App Today (`main 49384ea` vs. `branch ag/dl2b 18840b2`)

### 5.1 Plain Statement on 2D Map DEM Pixels

**Neither `main` (`49384ea`) nor `branch ag/dl2b` (`18840b2`) ever draws a single DEM pixel or hillshade pixel on the 2D map canvas.**

- On both branches, the 2D map renderer (`crates/strata-app/src/gui_tab_map_heritage.rs:36-45`) contains only a comment explaining that an earlier fake green 2100×1440 m rectangle was deleted and that drawing the downloaded DEM raster in place is not implemented yet (*`받은 DEM 을 제자리에 그리는 일은 따로`*; note that `:40` and `:44`, along with `crates/strata-app/src/tabs_home_map_state.rs:84`, mistakenly cite `LayerOps::addElevationHillshadeMap` instead of `addDemElevationRaster` / `ensureDemRelief`).
- `HomeMapTabState::dem_grid` (`crates/strata-app/src/tabs_home_map_state.rs:85-88`) is initialized to `None` (`crates/strata-app/src/tabs_home_map_state.rs:141`) and is **never assigned or read anywhere in the workspace**.
- `current_dem_raster_path()` (`crates/strata-app/src/dem_download_ui_state.rs:159-164` on `main`; removed on `ag/dl2b`) has **zero callers**.
- `strata-engine`'s optional `gdal` feature (`crates/strata-engine/Cargo.toml:22-27`) compiles only OGR vector `driver_shape` and `driver_gpkg` — no GDAL raster reader is linked into `strata-app`.
- The only visual change anywhere in the app after `"DEM"` is marked visible is in the **print drawing sheet** (`crates/strata-app/src/gui_tab_drawing_sheet.rs:105-114`), which tints the flat background rectangle from `rgb(252, 251, 248)` to solid `rgb(241, 244, 235)` — still zero raster pixels.

### 5.2 Step-by-Step Comparison (`S1`–`S28`)

| Step | C++ step summary (Sections 1–4) | `main` (`49384ea`) | `branch ag/dl2b` (`18840b2`) |
| --- | --- | --- | --- |
| `S1` | Ribbon `"DEM"` chip in `"배경 지도"`, verbatim tooltip, right-click menu | **`REAL`** — `crates/strata-app/src/gui_ribbon_consts.rs:24-26`, `crates/strata-app/src/ribbon_icons_map.rs:58`, `crates/strata-app/src/gui_ribbon_top.rs:152, 162-172`, `crates/strata-app/src/gui_ribbon_menus_a.rs:93-125` *(divergences: `warm_active` is hardcoded `label == "DEM" && !is_home` at `gui_ribbon_top.rs:152`, and `gui_ribbon_menus_a.rs:100-106` prepends an extra 5th row `"국토지리정보원 계정 설정·자동 다운로드…"`)* | **`REAL`** — unchanged from `main` (`ag/dl2b` does not touch `gui_ribbon_*.rs`) |
| `S2` | First left-click on `"DEM"` when no DEM is loaded triggers download flow | **`REAL`** — `crates/strata-app/src/gui_ribbon_click_a.rs:108-120, 154-156` opens `ActiveModalDialog::DemDownload`; clicking `"DEM 받기"` (`crates/strata-app/src/dem_download_ui_render.rs:87-106, 134-136`) spawns worker thread (`crates/strata-app/src/dem_download_ui_state.rs:52-106`) *(extra modal step vs. C++)* | **`REAL`** — `crates/strata-app/src/gui_ribbon_click_a.rs:108-120, 154-156` opens chooser modal (`ag/dl2b:crates/strata-app/src/dem_download_ui_render.rs:16-88`); clicking `"DEM 받기"` queues job `"DEM"` on bottom gauge `app.download_jobs`, closes modal immediately (`:81-85`), and polls every frame via `poll_background_downloads` (`ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:118-198, 285-289`, `ag/dl2b:crates/strata-app/src/shell_workspace.rs:35`) |
| `S3` | Post-download registration: retain `dem.tif`, add `"지표모델(DSM) 30m · Copernicus"` + `"지형 음영"` to layer tree & 2D map, store `dem_cover_wgs84`, ledger cleanup on save | **`FAKE`** — Proof: `crates/strata-app/src/dem_download_ui_state.rs:108-125` (`apply_result`) only inserts `"DEM"` and `"지표모델(DSM) 30m · Coperni..."` into `app.home_map.real_survey.layer_visible` and sets `status_bar_hint` / `draw_guide_message = "DEM {downloaded}칸을 받아 한 장으로 붙였습니다."` (`crates/strata-app/src/dem_download_ui_paths.rs:83-90`). It adds **no** `LayerTreeEntry` to `app.left_layers.entries`, never opens `dem.tif`, leaves `dem_grid = None` (`crates/strata-app/src/tabs_home_map_state.rs:141`), and draws nothing (`crates/strata-app/src/gui_tab_map_heritage.rs:36-45`) | **`FAKE`** — Proof: `ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:269-277` (`Poll::Ready(DemVerdict::Done)`) still only inserts `DEM_LEGEND_KEYS` into `real_survey.layer_visible`, sets `status_bar_hint` / `draw_guide_message`, and stores `state.result = Some(result)` — no layer-tree entry, no raster load, `dem_grid` still `None`, zero 2D pixels |
| `S4` | Second click on `"DEM"` while visible hides DEM & `"지형 음영"` (unless `"지질도"` on) and shows `"DEM 지형을 숨겼습니다."` | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_click_a.rs:120-152` flips `"DEM"`, `"지표모델(DSM) 30m · Coperni..."`, and `"지형 음영"` booleans in `real_survey.layer_visible` and sets `status_bar_hint = "DEM 지형을 숨겼습니다."`, while no DEM or hillshade layer exists in `left_layers` or on the 2D canvas | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_click_a.rs:120-152` (unchanged on `ag/dl2b`) |
| `S5` | Re-click on `"DEM"` while hidden checks `demCoversCanvas`; re-downloads if panned outside coverage, else restores DEM + relief + `followCanvas` | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_click_a.rs:120-150` performs **no** `demCoversCanvas` extent check, creates no relief layer, attaches no viewport listener, and unconditionally sets `layer_visible` to `true` with `status_bar_hint = "DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."` | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_click_a.rs:120-150` (unchanged on `ag/dl2b`) |
| `S6` | Menu Row 1 `"국토지리원 DEM 불러오기(.img)…"` opens `.img`/`.tif` file dialog and loads `"DEM"` + `"지형 음영"` | **`MISSING`** — `crates/strata-app/src/gui_ribbon_menu_actions.rs:83-84` misroutes `"DEM 불러오기"` to `ActiveModalDialog::DemDownload` (the Copernicus online downloader); no `.img` file dialog or loader exists | **`MISSING`** — `crates/strata-app/src/gui_ribbon_menu_actions.rs:83-84` (unchanged on `ag/dl2b`) |
| `S7` | Menu Row 2 `"DEM 파일로 음영 만들기…"` opens `"DEM GeoTIFF"` picker + modal `"지형분석"` (`Multi`/`Single`, `315°`, `45°`, `Z=1.0`) | **`MISSING`** — `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87` misroutes `"음영 만들기"` to `ActiveModalDialog::DemClass` (`"DEM 표현"`); neither the GeoTIFF file picker nor the `"지형분석"` dialog exists | **`MISSING`** — `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87` (unchanged on `ag/dl2b`) |
| `S8` | Menu Row 2 computes `<surveyDir>/지형분석/<stem>_hillshade.tif` (`_2`… on collision) via strip-based 3×3 Horn multi/single writer (`NoData=0`, valid `>= 1`) | **`MISSING`** — Not wired in `strata-app`. (`crates/strata-terrain3d/src/trench_hillshade.rs:16-24, 30-92` has uncalled pure functions `hillshade_output_path` [no `지형분석/` dir, no `_2` collision suffix] and `compute_horn_hillshade_byte_grid` [single-azimuth only, no `zFactor`, no `NoData=0` clamp, no GeoTIFF writer]) | **`MISSING`** — unchanged on `ag/dl2b` |
| `S9` | Menu Row 2 loads output GeoTIFF via `addRasterFromPath`, renames to `"음영기복"`, and adds a new layer on repeat runs | **`MISSING`** — No GeoTIFF layer loader or `"음영기복"` layer exists in `crates/strata-app` | **`MISSING`** — unchanged on `ag/dl2b` |
| `S10` | Menu Row 3 `"DEM 표현…"` checks for active `kDem` pseudocolor raster (else `"먼저 DEM을 켜 주세요..."` info box) and opens non-modal `KaDemClassDialog` | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87` opens `ActiveModalDialog::DemClass` unconditionally even when no DEM exists, and `crates/strata-app/src/modal_style_demclass.rs:44-55` renders an invented `"DEM 모드"` (`"등고"` / `"음영"` / `"고도"`) radio row absent from C++ | **`FAKE`** — Proof: `crates/strata-app/src/gui_ribbon_menu_actions.rs:85-87`, `crates/strata-app/src/modal_style_demclass.rs:44-55` (unchanged on `ag/dl2b`) |
| `S11` | Menu Row 3 `KaDemClassDialog` presets, `"현재 화면에 색 맞추기"`, flat-site hint, and `"적용"` updating DEM shader + `"지형 음영"` | **`FAKE`** — Proof: `crates/strata-app/src/modal_style_demclass.rs:78-84, 144-148` hardcodes `"현재 화면 범위(32.4 m ~ 74.8 m)에 맞춰 고도 색 띠를 재조정했습니다."` and `"0 m".."300 m"` / `"32.4 m".."74.8 m"`, paints synthetic RGB math (`:104-131`), and on `"적용"`/`"확인"` (`:202-220, 232-253`) only sets `layer_visible.insert("지형 음영", true)` + status text — which in turn makes `"자료 준비 상태"` (`crates/strata-app/src/modal_reference.rs:35-39`, `crates/strata-app/src/modal_reference_data.rs:53-66`) display hardcoded `"2026-11-08 02:20"` and `"6.2 MB"` with zero DEM file on disk | **`FAKE`** — Proof: `crates/strata-app/src/modal_style_demclass.rs:78-84, 104-148, 202-253`, `crates/strata-app/src/modal_reference_data.rs:53-66` (unchanged on `ag/dl2b`) |
| `S12` | Menu Row 4 `"측량점으로 등고선…"` checks open survey, opens `"측량 좌표"` file dialog, and reads DXF/XLSX/XLS/CSV for live point/elevation preview | **`FAKE`** — Proof: `crates/strata-app/src/dlg_survey_contour.rs:10-202` has no open-survey guard, discards `"찾기…"` button clicks via `let _ = ui.add_sized(..., egui::Button::new("찾기…"));` (`:43`), and for any non-empty text in `contour_file_path` displays hardcoded `"점 248개, 표고 24.5–48.2 m. 등고선 약 {line_est}줄 (간격 {:.1} m). 의심점 3개 제외."` (`:146-157`) without reading any file (`crates/strata-terrain3d/src/trench_survey_points.rs:30-215` is never called) | **`FAKE`** — Proof: `crates/strata-app/src/dlg_survey_contour.rs:43, 146-157` (unchanged on `ag/dl2b`) |
| `S13` | Menu Row 4 runs `GDALGridCreate` + `GDALContourGenerateEx`, writes `측량등고선/<stem>/{contours.gpkg, surface.tif}`, and adds `"참조 지도"` → `"측량 등고선 · <stem>"` (`"측량점"`, `"등고선"`, `"색 구간"`) | **`FAKE`** — Proof: `crates/strata-app/src/dlg_survey_contour.rs:184-199` reads/writes zero files, adds the empty survey domain layer `DomainLayerKey::SectionLine` (`"단면선"`) via `ensure_domain_layer` (`:189-191`), and sets `draw_guide_message = Some("측량점 등고선 약 {}줄(간격 {:.1} m)과 표고점 248개를 생성했습니다.")` (`:192-195`) | **`FAKE`** — Proof: `crates/strata-app/src/dlg_survey_contour.rs:184-199` (unchanged on `ag/dl2b`) |
| `S14` | One-click download preflight: open-survey check + project **canvas view extent** (`viewport.world_bbox()`) to `EPSG:4326` | **`REAL`** — `crates/strata-app/src/dem_download_ui_paths.rs:25-79`, `crates/strata-app/src/dem_download_ui_state.rs:57-72` *(note: `wgs84_extent_from_work` at `dem_download_ui_paths.rs:51-66` uses `strata_geo::crs_transform::tm_to_lonlat` instead of `strata_engine::Transformer`)* | **`REAL`** — `crates/strata-app/src/dem_download_ui_paths.rs:25-79`, `ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:120-149`, `ag/dl2b:crates/strata-app/src/dem_download_ui_failures.rs:10-29` *(adds single-slot busy guard at `:120-126`, `DemFolderGuard` empty-folder rollback at `:35-59`, and `StopCode::{Dir1, Dir2, Area1, Area2}` mapping)* |
| `S15` | Copernicus GLO-30 S3 URL pattern, `floor(min)`..`ceil(max)-1` 1° tile selection, and `DEM_MAXIMUM_TILES = 24` cap | **`REAL`** — `crates/strata-net/src/dem_consts.rs:7-10, 49-53`, `crates/strata-net/src/dem_tiles.rs:44-51, 89-104, 143-189` | **`REAL`** — unchanged (`crates/strata-net/src/dem_*.rs` is byte-for-byte identical between `49384ea` and `18840b2`) |
| `S16` | Per-tile HTTP fetch (`5` s connect, `15` s stall, `0` retries), HTTP 404 ocean skip + warning, and tile header checks | **`REAL`** — `crates/strata-net/src/dem_fetch.rs:34-99` (`curl.exe` `--connect-timeout 5 --speed-time 15 --speed-limit 1 -f -w %{http_code}` + 100 ms cancel poll), `crates/strata-net/src/dem_pipeline.rs:141-159, 186-188` *(omits C++ per-tile GDAL CRS/Float32/geotransform checks `SC11`–`SC12` before `gdalbuildvrt -strict`; `grid_aligned` in `dem_tiles.rs:117-120` is test-only)* | **`REAL`** — `crates/strata-net/src/dem_fetch.rs:34-99`, `crates/strata-net/src/dem_pipeline.rs:141-159`, `ag/dl2b:crates/strata-app/src/dem_download_ui_failures.rs:30-38` *(maps `DEM_NO_TILE_KO` → `StopCode::None1`, `DEM_CONNECTION_FAILED_KO` → `StopCode::Net1`; note `dem_fetch.rs` was not switched to `ag/dl2b`'s new `curl_cancel.rs`)* |
| `S17` | Staging dir, `gdalbuildvrt` (`-resolution highest -vrtnodata nan -strict`), `gdal_translate` (`TILED=YES`, `COMPRESS=DEFLATE`, `PREDICTOR=3`, `BIGTIFF=IF_SAFER`), keep `EPSG:4326`, verify output | **`REAL`** — `crates/strata-net/src/dem_fetch.rs:130-139`, `crates/strata-net/src/dem_pipeline.rs:44-89, 114-125, 160-197` *(divergences: single-tile shortcut copies the raw COG without `gdal_translate` at `:165-167`; `-srcwin` view clipping is omitted at `:66-85`; output lands at `<surveyDir>/DEM/원본/ka-hgis-reference-<6hex>/dem.tif`; verification at `:179-182` checks only `metadata.len() > 0`, not GDAL band/dimensions)* | **`REAL`** — `crates/strata-net/src/dem_pipeline.rs:44-89, 160-197`, `ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:56-59, 180-194` *(adds `DemFolderGuard` cleanup on cancel/error/disconnect and maps mosaic/save/verify failures to `StopCode::Data1` at `ag/dl2b:crates/strata-app/src/dem_download_ui_failures.rs:39-41`)* |
| `S18` | 16-stop `kStops` linear pseudocolor shader (`national` / `lowland` / `viewport`), bilinear resampling, default opacity `1.0` on 2D map | **`MISSING`** — No 2D raster colour-ramp shader or texture painter exists (`crates/strata-app/src/gui_tab_map_heritage.rs:36-45`, `crates/strata-app/src/gui_tab_drawing_sheet.rs:105-114`) | **`MISSING`** — unchanged on `ag/dl2b` |
| `S19` | Elevation range sampling (`bandStatistics` 25,000 samples for `"viewport"`, 128-px `sampleRange` window for `suggestViewport`) | **`FAKE`** — Proof: `crates/strata-app/src/modal_style_demclass.rs:78-84, 147` pretends to fit the current view with hardcoded `"32.4 m ~ 74.8 m"` without reading any raster band | **`FAKE`** — Proof: `crates/strata-app/src/modal_style_demclass.rs:78-84, 147` (unchanged on `ag/dl2b`) |
| `S20` | `DemPresentation::followCanvas` (`350` ms `demViewportTimer` re-classifying `"viewport"` ramp on pan/zoom) | **`MISSING`** — No viewport debounce timer or pan/zoom DEM re-classification hook exists in `crates/strata-app` | **`MISSING`** — unchanged on `ag/dl2b` |
| `S21` | `DemPresentation::reliefSource` (`GDALWarp` bilinear VRT to `EPSG:5186`/`5187` in `<CacheLocation>/dem-relief/<sha256>.vrt`) | **`MISSING`** — No `gdalwarp` / VRT cache or geographic-to-work-CRS raster reprojection exists in `crates/` | **`MISSING`** — unchanged on `ag/dl2b` |
| `S22` | Live multi-directional hillshade layer `"지형 음영"` (`315°`/`45°`, `zFactor` `0.1..5.0`, Multiply blend, default opacity `0.30`) above DEM | **`MISSING`** — No `"지형 음영"` raster layer or Multiply hillshade compositor exists (`crates/strata-app/src/gui_tab_map_heritage.rs:36-45`) | **`MISSING`** — unchanged on `ag/dl2b` |
| `S23` | Custom `DemColorRampLegend` (`104×156` px / `27×40.5` mm, `"표고 (m)"`, `"비선형 눈금"`, 5–7 ticks) | **`MISSING`** — `crates/strata-app/src/gui_layer_panel_row_spec.rs:44-46, 65` only draws `SwatchKind::RasterChecker`; `build_dem_legend_ticks` in `crates/strata-terrain3d/src/trench_elevation_profile.rs:39-78` is never called outside unit tests | **`MISSING`** — unchanged on `ag/dl2b` |
| `S24` | Persist DEM & hillshade layers + `ka_hgis/dem_*` custom properties in survey `.gpkg` and restore on open | **`MISSING`** — `crates/strata-storage/src/survey_gpkg_save.rs:7` saves only survey vector tables, and `crates/strata-app/src/gui_tab_home_open.rs:16-78` restores no DEM or hillshade layer on survey open | **`MISSING`** — unchanged on `ag/dl2b` |
| `S25` | NGII `.img` / `.tif` open via GDAL (`HFA` / `GTiff`) & band validation | **`MISSING`** — No `.img` or GeoTIFF raster open path exists (`crates/strata-engine/Cargo.toml:22-27` enables only vector `driver_shape`, `driver_gpkg`) | **`MISSING`** — unchanged on `ag/dl2b` |
| `S26` | Embedded-only CRS check on `.img` (`TopographicSourceCrs` not used; `dem_relief_error` when CRS missing) | **`MISSING`** — No `.img` CRS inspection or `dem_relief_error` handling exists in `crates/` | **`MISSING`** — unchanged on `ag/dl2b` |
| `S27` | Single-band metre elevations & GDAL band NoData transparency on `.img` | **`MISSING`** — No `.img` band reader or NoData mask exists in 2D map rendering (`crates/strata-terrain3d/src/terrain3d_dem.rs:7-60` is an uncalled in-memory slice helper) | **`MISSING`** — unchanged on `ag/dl2b` |
| `S28` | Single-file `kDem` replacement (no `.img` mosaicking) & external-path retention | **`MISSING`** — No `.img` import or `ReferenceKind::kDem` layer replacement exists in `crates/strata-app` *(note: `crates/strata-paleo/src/ngii_dem.rs:1-60` and `crates/strata-paleo/src/dem_synthesis_io.rs:111-145` are for paleo contour synthesis, not `.img` import)* | **`MISSING`** — unchanged on `ag/dl2b` |

### 5.3 What `branch ag/dl2b` (`18840b2`) Changes Relative to `main` (`49384ea`)

`git diff 49384ea...ag/dl2b --stat` touches **5 non-test DEM files** (`dem_download_ui.rs`, `dem_download_ui_failures.rs`, `dem_download_ui_render.rs`, `dem_download_ui_state.rs`, `shell_workspace.rs`) plus the shared bottom download gauge (`download_jobs.rs`, `download_jobs_list.rs`):

1. Replaces the global `OnceLock<Mutex<DemDownloadState>>` (`crates/strata-app/src/dem_download_ui_state.rs:48-51`) with per-app `DemWorkerSlot` on `StrataAppState` (`ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:77-83`).
2. Converts `ka_modal_dem_download` into a chooser-only window that closes as soon as `"DEM 받기"` is clicked (`ag/dl2b:crates/strata-app/src/dem_download_ui_render.rs:81-85`), reporting progress, cancel, completion, and structured `StopCode` (`Dir1`, `Dir2`, `Area1`, `Area2`, `None1`, `Net1`, `Data1`, `App1`) through the non-blocking bottom download gauge (`ag/dl2b:crates/strata-app/src/dem_download_ui_failures.rs:10-48`, `ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:118-289`, `ag/dl2b:crates/strata-app/src/shell_workspace.rs:35`).
3. Adds single-slot `"DEM"` busy guard (`ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:120-126`) and `DemFolderGuard` cleanup of newly created empty `DEM/원본` directories on preflight/worker failure or cancel (`ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:35-59`).
4. Does **not** change any `REAL` / `FAKE` / `MISSING` step classification (`S1`–`S28`): `crates/strata-net/src/dem_*.rs` is untouched, `Poll::Ready(DemVerdict::Done)` (`ag/dl2b:crates/strata-app/src/dem_download_ui_state.rs:269-277`) still only flips `DEM_LEGEND_KEYS` in `real_survey.layer_visible`, and zero DEM pixels are drawn on the 2D map.

---

## End — `Could not determine`, Counts, and 30-Citation Verification

### `Could not determine` (`not in the source`)

1. **NGII `.img` vertical-unit metadata & unflagged sentinel values**: `not in the source` — `LayerOps::addDemElevationRaster` (`BasemapOps.cpp:1996-2037`) and `DemPresentation::apply` (`DemPresentation.cpp:44-108`) never read `GDALRasterBand::GetUnitType()` and apply no custom filter for sentinel values (such as `-9999` or `0`) unless the `.img` header itself declares them as GDAL band NoData.
2. **Fallback CRS inference when an NGII `.img` lacks embedded CRS / `.prj` / `.aux.xml`**: `not in the source` — `TopographicSourceCrs` is never called for `.img` rasters (`BasemapOps.cpp:1996-2037`), and no user CRS prompt or default EPSG assignment exists on that path.
3. **Multi-file NGII `.img` mosaicking & survey-area (`5 km` / sheet-matched) NGII DEM download**: `not in the source` — the C++ app imports one `.img` at a time (`MainWindow.cpp:1689`) and its one-click download fetches Copernicus GLO-30 over `m_canvas->extent()` (`MainWindowDownloads.cpp:122`, `DemDownloadService.cpp:85-94`), never NGII DEM sheets.
4. **Largest allowed metre span for `startDemDownload` beyond `24` `1°×1°` tiles**: `not in the source` — C++ enforces only `tileCount <= 24` in `EPSG:4326` (`DemDownloadService.cpp:25, 142-147`) and imposes no separate metre or pixel-count cap before `GDALTranslate`.

### Counts

- **Traced steps (`S1`–`S28`)**: **`28`**
  - Section 1 (Button & 4 menu rows): `13` (`S1`–`S13`)
  - Section 2 (One-click download today): `4` (`S14`–`S17`)
  - Section 3 (How a DEM is shown & stored): `7` (`S18`–`S24`)
  - Section 4 (NGII `.img` import): `4` (`S25`–`S28`)
- **Stop conditions**:
  - Section 2 one-click download stop conditions: **`20`** (`SC01`–`SC20`, plus `1` non-fatal partial-ocean/relief warning row `W01`)
  - Additional stop/error conditions across Menu Rows 1–4 in Section 1: **`17`** (Row 1: `!m_canvas`/cancel + 4 `addDemElevationRaster` errors = `5`; Row 2: cancel + 4 `runHillshadeFile` errors + `addRasterFromPath` error = `6`; Row 3: missing-DEM `QMessageBox` + 3 `applyStyle` errors = `4`; Row 4: no-survey `KaUserError` + `contourProgress` busy + `<3`/collinear/task failure = `2` pre-task guards + dialog/task rejections) — **`37`** stop conditions across all DEM entry points combined.
- **Per-column Rust status counts across `S1`–`S28`**:

| Column | `REAL` | `FAKE` | `MISSING` | Total | Any DEM pixel ever drawn on 2D map? |
| --- | --- | --- | --- | --- | --- |
| `main` (`49384ea`) | **`6`** (`S1`, `S2`, `S14`, `S15`, `S16`, `S17`) | **`8`** (`S3`, `S4`, `S5`, `S10`, `S11`, `S12`, `S13`, `S19`) | **`14`** (`S6`–`S9`, `S18`, `S20`–`S28`) | **`28`** | **No (`0` pixels)** |
| `branch ag/dl2b` (`18840b2`) | **`6`** (`S1`, `S2`, `S14`, `S15`, `S16`, `S17`) | **`8`** (`S3`, `S4`, `S5`, `S10`, `S11`, `S12`, `S13`, `S19`) | **`14`** (`S6`–`S9`, `S18`, `S20`–`S28`) | **`28`** | **No (`0` pixels)** |

### 30 Randomly Sampled Citations Re-Opened (`verification-before-completion`)

30 citations from this document were selected with a fixed-seed random sample and re-opened directly against `../../../target/cpp-ref`, `HEAD` (`49384ea`), and `git show ag/dl2b:<path>` (`18840b2`): **`30 / 30` exact** (`100%` match on file path, line numbers, function/symbol, and verbatim string).

1. `MainWindowDownloads.cpp:38, 49-51, 119` — silent early-return guards (`m_isOpeningSurvey || m_closingWindow`, `!m_canvas`) (`exact`).
2. `DemPresentation.cpp:118-122` — `"demViewportTimer"` single-shot 350 ms setup in `followCanvas` (`exact`).
3. `MainWindowDownloads.cpp:123-126` — `QgsCoordinateTransform` from canvas CRS to `EPSG:4326` (`exact`).
4. `ReferenceStorage.cpp:39-43` — `surveyFolder(const QString& surveyPath)` (`exact`).
5. `BasemapOps.cpp:2118` — `ka_hgis/dem_relief_enabled` synced from `shadeNode->itemVisibilityChecked()` (`exact`).
6. `crates/strata-app/src/ribbon_icons_map.rs:58` — `"DEM"` verbatim two-line tooltip in Rust (`exact`).
7. `DemPresentation.cpp:166, 181, 193, 209, 220` — all five Korean error strings in `DemPresentation::reliefSource` (`exact`).
8. `MainWindowRibbon.cpp:399-428` — `m_btnDem` button, tooltip, 4-row `demMenu`, and `"basemap"` registration (`exact`).
9. `SurveyStorage.cpp:1179-1235` — `SurveyStorage::writeEmbedded` saving `QgsProject` into `.gpkg` (`exact`).
10. `DemColorRampLegend.cpp:51-90` — `snapshot(QgsRasterLayer*)` 5–7 tick calculation (`exact`).
11. `MainWindowOverlay.cpp:307-334` — `toggleDemMap` existing-DEM branch (`demCoversCanvas`, visibility toggle, status bar) (`exact`).
12. `MainWindow.cpp:1769` — `LayerOps::placeInLegendGroup(QgsProject::instance(), rl, QStringLiteral("참조 지도"))` in `runDemHillshade` (`exact`).
13. `DemDownloadService.cpp:190-198` — multi-tile spacing & `gridAligned` check (`exact`).
14. `DemDownloadService.cpp:102-105` — `gridAligned(double delta, double spacing)` (`1e-5` cell tolerance) (`exact`).
15. `BasemapOps.cpp:2027` — `DemPresentation::followCanvas(rl, canvas);` inside `addDemElevationRaster` (`exact`).
16. `MainWindowOverlay.cpp:337` — `startDemDownload();` when no DEM layer exists (`exact`).
17. `DemPresentation.cpp:169` — metre-CRS pass-through `if (layer->crs().mapUnits() == Qgis::DistanceUnit::Meters) return layer->source();` (`exact`).
18. `DemPresentation.cpp:173-187` — `<CacheLocation>/dem-relief/<sha256>.vrt` cache path and signature (`exact`).
19. `crates/strata-net/src/dem_tiles.rs:44-51, 89-104, 143-189` — `is_valid_wgs84`, `tile_name`/`tile_url`, `tile_range`, `tiles_for_extent` (`exact`).
20. `MainWindow.cpp:1731` — `form->addRow(QStringLiteral("방식"), mode);` in `"지형분석"` dialog (`exact`).
21. `MainWindowRibbon.cpp:411-413` — Menu Row 1 `"국토지리원 DEM 불러오기(.img)…"` → `importDemElevationRaster` (`exact`).
22. `MainWindowDownloads.cpp:120-121` — `referenceDownloadFolder(QStringLiteral("DEM"))` guard (`exact`).
23. `DemPresentation.cpp:135-137` — `QgsCsException` canvas warning `"화면 좌표를 변환하지 못해 이전 색띠를 유지합니다."` (`exact`).
24. `MainWindow.cpp:1717-1719` — `"다중광원 음영 (유구·분묘)"` (`Multi`) / `"단방향 음영"` (`Single`) combo items (`exact`).
25. `SurveyContourBuilder.cpp:74-80` — `SurveyContourBuilder::rejection` `< 3` points and collinear checks (`exact`).
26. `BasemapOps.cpp:1793-1829` — `LayerOps::demCoversCanvas` (`exact`).
27. `BasemapDsm.h:39` — `if (layer->name() != copernicusTitle()) layer->setName(copernicusTitle());` (`exact`).
28. `crates/strata-app/src/gui_ribbon_click_a.rs:120-150` — Rust `"DEM"` visibility flip without `demCoversCanvas` (`exact`).
29. `SurveyContourBuilder.cpp:181-200` — breakline densification and `fitCellSize` in `SurveyContourBuilder::build` (`exact`).
30. `crates/strata-storage/src/survey_gpkg_save.rs:7` — comment confirming `.qgz` / `qgis_projects` persistence is absent (`exact`).
