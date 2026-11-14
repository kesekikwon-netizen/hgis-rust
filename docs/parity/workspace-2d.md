# Survey Workspace (2D) — C++ Persistence vs. Rust Port Audit (`WS0`)

## 1. Where the workspace lives

### Embedded project (`qgis_projects`)
- **Table & row**: Inside the survey `.gpkg`, QGIS stores an embedded workspace in SQLite table `qgis_projects` (`SurveyStorage.cpp:357-360`, `crates/strata-check-export/src/user_sheet_tests.rs:58`) under row `name = 'survey'` (`crates/strata-check-export/src/user_sheet.rs:102`, `test_save_open.cpp:4661`), using URI `geopackage:<gpkgPath>?projectName=survey` (`SurveyStorage.cpp:346-350`).
- **Payload format**: `qgis_projects.content` stores a `.qgz` ZIP archive (either hex-encoded `TEXT` or raw `BLOB` / hex-in-`BLOB` — handled by `crates/strata-check-export/src/user_sheet.rs:117-125`), containing the `.qgs` XML document and optional auxiliary `<6char>_styles.db` attachment.
- **When written**: Written inside the `.ka-survey-gen-*` generation copy during `SurveyStorage::persistWorkspace` (`SurveyStorage.cpp:983`), `SurveyStorage::writeEmbedded` (`SurveyStorage.cpp:1179-1246`), `MainWindow::saveProjectAs` (`MainWindowSession.cpp:1570`), recovery snapshots (`SurveyStorage.cpp:688`), and external vector extraction (`SurveyStorage.cpp:1141`).

### Companion `.qgz` / `.qgs` & `.bak.qgz` backup
- **Companion path**: `<dir>/<completeBaseName>.qgz` derived from the survey `.gpkg` path (`SurveySession.cpp:37-41`), with `.qgs` accepted as secondary companion fallback (`MainWindowSession.cpp:284-286`).
- **Backup path**: `<dir>/<stem>.bak.qgz` (`KaSafeQgis.cpp:70-73`), rotated/written atomically inside `kaWriteQgisProjectAtomic` (`KaSafeQgis.cpp:75-202`, `:176-182`).
- **When written**: Created on new survey by `SurveyProjectFactory::createNewSurvey` (`SurveyProjectFactory.cpp:134`), updated on save by `SurveySession::persistWork` (`SurveySession.cpp:91-92`), and written on `Save As` (`MainWindowSession.cpp:1590`). `kaWriteQgisProjectAtomic` writes a temporary `.ka-writing-XXXXXX.qgz`, runs a probe read (`DontResolveLayers | DontLoadLayouts`), backs up the existing `.qgz` to `.bak.qgz`, and atomically swaps + calls `FlushFileBuffers`.

### Open preference order & fallback recovery
`MainWindow::openSurveyGpkg` (`MainWindowSession.cpp:242-443`, `OpenSurveyMode::PreferWorkspace`) opens a survey in four steps (note: picking `.qgz`/`.qgs` in `openProject` `:1645-1652` or `openRecentSurvey` `:584-592` redirects to the sibling `.gpkg` if present):
1. **Embedded `qgis_projects` (`name='survey'`)** inside `.gpkg`, unless marked unsafe (`MainWindowSession.cpp:291-305`, `SurveyStorage.cpp:1248-1267`). An access-violation crash during read marks the path unsafe in `unsafe-projects.txt` and aborts (`MainWindowSession.cpp:311-320`); non-crash errors fall through.
2. **Companion `.qgz`**, or **`.qgs`** if `.qgz` does not exist (`MainWindowSession.cpp:322-329`), unless marked unsafe.
3. **Backup `.bak.qgz`** if companion read failed without an AV crash (`MainWindowSession.cpp:334-343`).
4. **`LayersOnly` fallback**: Clears the project (`kaSafeClearQgisProject`), loads non-empty GPKG feature tables via `loadSurveyLayers` / `addNonEmptySavedGpkgLayers`, restores default styles from GPKG `layer_styles` (`LayerOps.cpp:1630, 1907`), marks the session read-only against overwriting the same file, and warns the user (`MainWindowSession.cpp:388-440`).

### Stale marker & fallback notice verbatim
- **Stale companion marker**: `<dir>/복구사본/<baseName>.qgz-오래됨.txt` (`SurveySession.cpp:43-47`), written with ISO timestamp + error message when `persistWork` embeds the `.gpkg` project successfully but companion `.qgz` save fails (`SurveySession.cpp:15-28, 92-102`), and removed on subsequent companion save success.
- **Fallback notice (verbatim, `SurveySession.cpp:61-69`)**:
  - Base text: `"조사 파일 안의 작업공간을 읽지 못해 동반 작업공간(%1)으로 열었습니다. 원래 작업공간은 같은 이름으로 덮어쓰지 않도록 막아 두었습니다. 「저장」을 누르면 다른 이름의 조사 파일로 저장합니다."`
  - Appended when `companionMayBeStale(gpkgPath)` is true: `" 이 작업공간은 마지막 저장 때 갱신하지 못해 그보다 오래된 상태일 수 있습니다."`

### GeoPackage `layer_styles` & metadata/bundle artifacts
- **`layer_styles` table**: Written inside `writeEmbedded` via `LayerOps::saveGpkgDefaultStyles` (`SurveyStorage.cpp:1240`, `LayerOps.cpp:1775-1793`), calling `vl->saveStyleToDatabaseV2(QStringLiteral("ka_hgis"), QString(), true, QString(), err)` for every OGR layer whose source points to the survey GPKG. Read via `LayerOps::loadGpkgDefaultStyle` (`LayerOps.cpp:1763-1773`) when opening GPKG layers without workspace XML (`LayerOps.cpp:1630, 1907`). Excluded from user table enumeration by `kaIsGpkgMetadataTable` (`LayerOps.cpp:1647`).
- **Schema version metadata**: `gpkg_metadata`, `gpkg_metadata_reference`, and `gpkg_extensions` carrying `<GDALMultiDomainMetadata><Metadata><MDI key="KA_HGIS_SCHEMA_VERSION">3</MDI></Metadata></GDALMultiDomainMetadata>` (`SurveySchema.h:22-23`, `SurveyProjectFactory.cpp:115`; Rust also writes schema v3 metadata in `crates/strata-storage/src/survey_gpkg_create.rs:80-89`).
- **Bundled external files**: Copied under `<survey_dir>/가져온자료/<safeLayerName>/` (`SurveyBundle.cpp:181, 221-225`).
- **Crash guard**: `%LOCALAPPDATA%/ka-hgis/unsafe-projects.txt` (or portable `userConfigDir`) (`KaSafeQgis.cpp:301-357`).
- **Rust status today**: `crates/strata-check-export/src/user_sheet.rs` & `user_sheet_xml.rs` read ONLY `qgis_projects` / `.qgz` / `.qgs` for `user_sheet` map-item submit verification. `crates/strata-app/src/gui_tab_home_open.rs:30-78` opens `.gpkg` by scanning `gpkg_contents` / `gpkg_geometry_columns` only, ignoring `qgis_projects`, companion `.qgz`, `.bak.qgz`, and `layer_styles`. `crates/strata-storage/src/survey_gpkg_save.rs:30-37` (`VACUUM INTO`) preserves `qgis_projects`, `layer_styles`, and unknown tables byte-for-byte on GPKG save (`crates/strata-storage/src/survey_gpkg_save_more_tests.rs:19-33`), though companion `.qgz` is not updated.

---

## 2. Things stored in the workspace

| # | Thing | Where it is stored (table / XML element & attribute / custom property key, verbatim) | Written by (C++ file:line) | Read by (C++ file:line) | What the user sees if it is lost | Rust today | Rust file:line | C++ test that proves it (file:line, name) |
|---|---|---|---|---|---|---|---|---|
| 1 | Project title / name | `qgis_projects.content` / `.qgz`: `<qgis projectname="...">`, `<title>` | `SurveyProjectFactory.cpp:131` | `MainWindowSession.cpp:149-236` (`QgsProject::read`) | Survey title reverts to raw filename stem instead of saved project name | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:55`, `crates/strata-storage/src/survey_gpkg_save.rs:30-37` | none |
| 2 | Work CRS (EPSG:5187 / 5186) | `<qgis><projectCrs><spatialrefsys><authid>` | `SurveyProjectFactory.cpp:130`, `SurveyStorage.cpp:1203` | `MainWindowSession.cpp:160-162` | Survey opens in wrong work CRS (default 5187 instead of 5186, shifting grids/coordinates) | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:54`, `crates/strata-storage/src/survey_gpkg_read.rs:118` | `test_workflow.cpp:7503` (`gpkgEmbeddedProject_writesAndReadsBack`), `test_workflow.cpp:7475` (`atomicProjectWrite_writtenFileReadsBackAsQgz`) |
| 3 | Layer Z-order / rendering stack | `<layerorder><layer id="..."/>` + `<custom-order enabled="...">` + `<layer-tree-group>` order | `SurveyStorage.cpp:1235` (`QgsProject::write`) | `LayerOps.cpp:2275` (`applyLayerOrderToLabels`), `QgsProject::read` | Polygons obscure points/lines; saved top-to-bottom stack is inverted or scrambled | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:97-120` | none |
| 4 | Legend groups & hierarchy | `<layer-tree-group name="조사 데이터"\|"참조 지도"\|"지정유산"\|...>` | `LayerOps.h:29-31`, `LayerOps.cpp:1910` | `LayerOps.cpp:1816-1829`, `QgsProject::read` | Custom legend folders vanish; layers fall into flat default sections | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/left_layer_panel.rs:47` | none |
| 5 | Layer visibility (checked state) | `<layer-tree-layer checked="Qt::Checked\|Qt::Unchecked">` | `SurveyStorage.cpp:1235` (`QgsProject::write`) | `QgsProject::read` | Hidden layers pop back visible on open, cluttering map and sheets | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/left_layer_panel.rs:47` | none |
| 6 | Shown layer display name | `<layer-tree-layer name="...">` and `<maplayer><layername>` | `SurveyStorage.cpp:1235`, `LayerOps.cpp:1900` | `QgsProject::read` | User-renamed layers revert to raw table names (`saved_gpkg_layer_title`) | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:82-95` | none |
| 7 | Survey domain identity & role | `<maplayer><customproperties><Option type="Map">`: `ka_hgis/layer_key`, `ka_hgis/layer_role`, `ka_hgis/is_survey_area`, `ka_hgis/survey_table`, `ka_hgis/admin_emd` | `LayerOps.cpp:1042-1055`, `LayerOps.cpp:1536-1537`, `BasemapOps.cpp:769` | `LayerOps.cpp:1040-1065`, `LayerOps.cpp:1849-1895` | Non-standard tables or cloned layers lose survey role (edit targets, submit domain mapping) | KEPT ON SAVE BUT NOT READ | `crates/strata-storage/src/survey_gpkg_read.rs:153` | `test_workflow.cpp:7531` |
| 8 | Simple vector styles (fill/stroke/width/marker/dash) | `layer_styles` (`styleQML`, `styleSLD`, `useAsDefault`) + `<maplayer><renderer-v2>` + custom props `ka_hgis/style_fill`, `ka_hgis/style_stroke`, `ka_hgis/style_width_mm`, `ka_hgis/style_marker_mm`, `ka_hgis/style_no_fill`, `ka_hgis/style_no_stroke`, `ka_hgis/style_dashed` | `LayerOps.cpp:510-516`, `LayerOps.cpp:1775-1793` (`saveStyleToDatabaseV2`) | `LayerOps.cpp:495-575`, `LayerOps.cpp:1763-1773` (`loadGpkgDefaultStyle`) | User layer colors, line widths, dashed outlines, and marker sizes reset to defaults | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/real_survey_types.rs:49`, `crates/strata-app/src/real_survey_load.rs:22` | `test_workflow.cpp:7363` (`saveGpkgDefaultStyles`) |
| 9 | Categorical / preset vector styles | `<maplayer><renderer-v2 type="categorizedSymbol\|RuleRenderer">` + custom props `ka_hgis/style_mode` (`by_kind` \| `preset`), `ka_hgis/original_style` | `LayerStyleKinds.cpp:106`, `FeatureRecordStyle.cpp:25, 222`, `LineMapStyle.cpp:46` | `FeatureRecordStyle.cpp:15-40`, `QgsProject::read` | Feature-type / status color coding disappears into uniform single color | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/survey_style.rs` | none |
| 10 | Per-layer opacity | `<maplayer><layerOpacity>` | `SurveyStorage.cpp:1235` (`QgsProject::write`) | `QgsProject::read` | Translucent polygons/overlays render fully opaque and hide imagery | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/app_state.rs` (global opacity only) | none |
| 11 | Vector labels & halo/font styling | `<maplayer><labeling>` + custom props `ka_hgis/label_field`, `ka_hgis/label_font_size`, `ka_hgis/label_show_area`, `ka_hgis/label_color`, `ka_hgis/label_halo`, `ka_hgis/layout_numbers_visible`, `<customproperties>rendering/renderAboveLabels` | `LabelOps.cpp:80-82, 182, 197`, `HeritageLayoutNumbers.cpp:1185`, `LayerTreeLabelStack.cpp:272` | `LabelOps.cpp:80-200`, `LayerOps.cpp:2275` | Site numbers, parcel labels, and area text disappear until manually reconfigured | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/real_survey_load.rs:23` (`label_views.clear()`) | none |
| 12 | Basemaps & reference map layers (XYZ/WMS/WMTS/raster/external OGR) | `<maplayer><datasource>`, `<provider>`, `<customproperties>`: `ka_hgis/reference_kind`, `ka_hgis/imported_reference`, `ka_hgis/thematic_overlay`, `ka_hgis/omit_sheet_legend`, `ka_hgis/crs_label` | `BasemapOps.cpp:1158-1210`, `MainWindow.cpp:2905`, `LayerOps.cpp:1105, 1179`, `BasemapOps.cpp:2087` | `MainWindowSession.cpp:164-230`, `MainWindow.cpp:1319` | Imported satellite/ortho/thematic GeoTIFFs, SHPs, and WMS/WMTS choices are missing; Rust resets to VWorld Satellite+Cadastral | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:66` | `test_storage_safety.cpp:1036` (`persistWorkspace_keepsExternalHeritagePathOnReopen`), `test_save_open.cpp:548-551` |
| 13 | Downloaded / imported cadastral layers & skip flag | `<maplayer><customproperties>`: `ka_hgis/cadastral`, `ka_hgis/cadastral_color`, `ka_hgis/cadastral_copy`; project `<properties>`: `ka_hgis/skip_auto_cadastral` | `LayerOps.cpp:1049`, `CadastralImport.cpp:264-265`, `ParcelClipboard.cpp:55`, `LayerOps.cpp:1212-1218` | `LayerOps.cpp:1049, 1212-1218` | Cadastral layers lose distinct color/role or auto-fetch suppression resets | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:60-64` | none |
| 14 | Heritage overlay metadata | `<maplayer><customproperties>`: `ka_hgis/heritage_region`, `ka_hgis/heritage_dataset` | `HeritagePledge.cpp:82`, `HeritageLayoutNumbers.cpp:371` | `HeritageLayoutNumbers.cpp:365-388` | Heritage protection zones lose pledge/numbering linkage | KEPT ON SAVE BUT NOT READ | none | none |
| 15 | Imported topographic & contour group metadata | `<maplayer><customproperties>`: `ka_hgis/topographic_group`, `ka_hgis/topographic_source`, `ka_hgis/topographic_source_keys`, `ka_hgis/topographic_sources`, `ka_hgis/topographic_sheet`, `ka_hgis/topographic_cad_layer`, `ka_hgis/contour_group` | `KaTopographicImportDialog.cpp:702, 830-832, 943-945, 969-972`, `SurveyContourStyle.cpp:55` | `KaTopographicImportDialog.cpp:702-975` | Imported NGII topo sheets / contour groups lose provenance & CAD pairing | KEPT ON SAVE BUT NOT READ | none | none |
| 16 | CAD import & alignment pending flags | `<maplayer><customproperties>`: `ka_hgis/cad_drawing`, `ka_hgis/cad_unsure`, `ka_hgis/align_pending` | `CadDrawingLayers.h:20-21`, `KaCadImport.cpp:210`, `LayerOps.cpp:1157` | `LayerOps.cpp:1157`, `KaCadImport.cpp:210` | Unaligned CAD layers lose alignment warning / CAD grouping | KEPT ON SAVE BUT NOT READ | none | `test_cad_layers.cpp:103` (`addToProject_buildsTheGroupUnderReference`) |
| 17 | DEM presentation, hillshade relief & section raster | `<maplayer><customproperties>`: `ka_hgis/dem_display_version`, `ka_hgis/dem_preset`, `ka_hgis/dem_relief_enabled`, `ka_hgis/dem_z_factor`, `ka_hgis/dem_relief_strength`, `ka_hgis/dem_cover_wgs84`, `ka_hgis/dem_relief_error`, `ka_hgis/section_raster` | `DemPresentation.cpp:40-41`, `KaDemClassDialog.cpp:122-124`, `BasemapOps.cpp:1750, 1951-1952`, `KaTerrain3dStudio.cpp:355` | `DemPresentation.cpp:40-41`, `BasemapOps.cpp:1750, 1951-1952` | DEM elevation bands, hillshade relief settings, and section rasters lose state | KEPT ON SAVE BUT NOT READ | none | none |
| 18 | Absorbed non-schema GPKG vector tables | SQLite tables in `.gpkg` (`user_poly_*`, `주변_500m`, absorbed OGR/memory layers) + `gpkg_contents` | `SurveyStorage.cpp:703-786` (`absorbExternalVectors`) | `LayerOps.cpp:1831-1915` (`addNonEmptySavedGpkgLayers`) | Geometry & attributes load in Rust as generic SurveyData layers, but custom properties & layer styles are missing | READ (features only; styling/props KEPT ON SAVE BUT NOT READ) | `crates/strata-storage/src/survey_gpkg_read.rs:82`, `crates/strata-app/src/gui_tab_home_open.rs:60-64` | `test_workflow.cpp:7531` (`surveyFileTravelsAloneWithAbsorbedShp`) |
| 19 | Snapping settings | `<snapping-settings enabled mode tolerance type unit intersection-snapping self-snapping>` + `<individual-layer-settings>`, plus `<properties><ka_hgis>`: `snap_target`, `snap_chosen`, `topological` | `LayerOps.cpp:2847-2897` (`applySnapSettings`) | `LayerOps.cpp:2875-2897` (`readSnapSettings`) | Digitizing snap mode, tolerance, chosen layers, and topological edit flag reset | KEPT ON SAVE BUT NOT READ | none | `test_workflow.cpp:3080` (`snapSettings_surviveProjectWriteAndReopen`) |
| 20 | Measure ellipsoid setting | `<properties><Measure><Ellipsoid>` (`GeoNone`) | `MeasureOps.cpp:62-71` (`pinPlanarEllipsoid`) | `MeasureOps.cpp:62-71` | Ellipsoidal vs. planar distance measurement preference is ignored (Rust hardcodes planar) | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/survey_measure.rs` | none |
| 21 | Project full/default extent & initial viewport | `<ProjectViewSettings><PresetFullExtent>` / `<DefaultViewExtent>` (plus `MainWindowSession.cpp:187-188` data-extent zoom on open) | `BasemapOps.cpp:2464-2480` | `MainWindowSession.cpp:187-188` | Map opens zoomed out to all of Korea instead of fitting the saved survey data | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_home_open.rs:70-71`, `crates/strata-app/src/extent_history.rs:5-8` | none |
| 22 | Active drawing sheet (`user_sheet`) & `section_sheet` | `<Layouts><Layout name="user_sheet"\|"section_sheet">`, `<LayoutItem type="65639">`, custom props: `ka_hgis/auto_template`, `ka_hgis/user_composed`, `ka_hgis/grid_enabled`, `ka_hgis/grid_interval_m`, `ka_hgis/grid_numbers`, `ka_hgis/grid_follow_map`, `ka_hgis/heritage_names_large_scale`, `ka_hgis/chrome_placed`, `ka_hgis/chrome_size_percent`, `ka_hgis/legend_kept_on_page`, `ka_hgis/legend_flow_width`, `ka_hgis/title_block_name`, `ka_hgis/trench_placed_layout`, `ka_hgis/released_*`, callout IDs `ka_coord_box_*` / `ka_coord_let_*` | `LayoutService.cpp:381-382, 1062-1092, 1455-1505`, `KaDrawingStudio.cpp:984, 1521-1539, 2460-2461, 2694`, `KaTitleBlock.cpp:21`, `KaDrawingCoordCallout.cpp:75, 89`, `TrenchLayerEdit.h:21`, `SurveyHandles.cpp:12-14` | `LayoutService.cpp:381-382, 1455-1505`, `KaDrawingStudio.cpp:1521-1539`, `KaDrawingCoordCallout.cpp:75, 89` | Drawing sheet layout, map frame extent, grid intervals, legend box, title block, and coord callouts are invisible in 2D/Drawing tab (only boolean submit-checklist check works) | READ (submit `SheetCheck` presence only) / KEPT ON SAVE BUT NOT READ (rest) | `crates/strata-check-export/src/user_sheet.rs:63`, `crates/strata-check-export/src/user_sheet_xml.rs` | `test_save_open.cpp:293, 4801, 4850` (`oldVersionSurveysStillOpen`) |
| 23 | Saved drawing sheet set | `<Layouts><Layout name="도면 보관: %1">` with custom props `ka_hgis/saved_sheet=true`, `ka_hgis/saved_sheet_title`, `ka_hgis/auto_template=false`, `ka_hgis/user_composed=false` | `KaDrawingSheetSet.cpp:14-15, 29-31, 77-81` | `KaDrawingSheetSet.cpp:1-164` | Archived map sheets cannot be viewed, switched, or exported | KEPT ON SAVE BUT NOT READ | `crates/strata-app/src/gui_tab_drawing_sheet.rs` | `test_drawing_studio.cpp:164` (`sheetSet_storeAndReopenKeepsWorkingSheet`) |
| 24 | Path base & saved survey directory | `<properties><Paths><Absolute>` + `<properties><ka_hgis><survey_dir>` + `<qgis>` `presetHomePath` | `SurveyStorage.cpp:1204-1211, 1233`, `SurveyBundle.cpp:253-256` | `SurveyBundle.cpp:248-251, 258-267`, `LayerOps.cpp:2127-2202` | External reference maps fail path rebase when survey folder moves to another PC | KEPT ON SAVE BUT NOT READ | none | `test_storage_safety.cpp:995` (`repairPersisted_restoresDroppedWindowsHome`), `test_save_open.cpp:4850` (`oldVersionSurveysStillOpen`) |
| 25 | Recovery snapshot external ref note | Recovery `.qgz` `<properties><ka_hgis><recovery_external_references>` | `SurveyStorage.cpp:685-686` | `SurveyStorage.cpp:584-701` | Crash recovery snapshot loses list of external sources omitted from recovery copy | KEPT ON SAVE BUT NOT READ | none | none |
| 26 | QGIS auxiliary styles DB & project metadata | Embedded `<6char>_styles.db` SQLite entry in `.qgz` (`<ProjectStyleSettings projectStyleId="attachment:///..._styles.db">`), `<qgis version saveDateTime saveUser>` | `SurveyStorage.cpp:1235` (`QgsProject::write`) | `QgsProject::read` | Named symbol libraries embedded in `.qgz` are unread | KEPT ON SAVE BUT NOT READ | none | none |

*(Note on `save_survey`: in Rust, `crates/strata-storage/src/survey_gpkg_save.rs:30-37` copies `qgis_projects` and `layer_styles` untouched via `VACUUM INTO`, so existing embedded data is KEPT ON SAVE rather than dropped; however, Rust never writes updates to `qgis_projects`, `layer_styles`, or companion `.qgz`, so new Rust edits are absent from QGIS workspace tables, and legacy `crates/strata-storage/src/gpkg_repository.rs:120-176` `ka_hgis_workspace_state` applies only to `.crash_recovery.gpkg` `crates/strata-app/src/app_state_projects.rs:263-282`.)*

---

## 3. Paths and safety

### How the embedded project names the survey GPKG and reference files
- Inside `SurveyStorage::writeEmbedded` (`SurveyStorage.cpp:1203-1233`):
  1. `project.setFileName(publishedGpkg)` (`:1203`) and `project.setPresetHomePath(surveyDir)` with any `.ka-survey-gen-*` temp segment stripped (`:1204-1208`).
  2. `QgsPathResolver::setPathWriter` rewrites references pointing at the temporary generation folder (`genHomeDir` / `workingGpkg`) to the final `publishedGpkg` (`:1220-1226`).
  3. `project.setFilePathStorage(Qgis::FilePathType::Absolute)` (`:1233`) so embedded GPKG layer URIs are written consistently, allowing `SurveyStorage::remountCopiedSurveyLayers` (`:193-217`) on open to compare table names / filename matches against the opened `gpkgPath` and re-point them even if the `.gpkg` file was copied or renamed.
- By contrast, companion `.qgz` files saved via `kaWriteQgisProjectAtomic` retain relative paths (`<Paths><Absolute>false</Absolute>`).
- Bundled external files are stored relative to `<survey_dir>` or under `<survey_dir>/가져온자료/<safeLayerName>/` (`SurveyBundle.cpp:181, 221-225`), excluding temporary `dem-relief/` caches (`SurveyBundle.cpp:193-196`). Sidecars (`.shx`, `.dbf`, `.prj`, `.cpg`, `.aux.xml`) are copied alongside vector files.

### Repair logic when a survey moves to another folder or another PC
1. **Before/on save**:
   - `SurveyStorage::absorbExternalVectors` (`SurveyStorage.cpp:703-786`) absorbs non-reference, non-cadastral `ogr`/memory vector layers directly into tables inside the survey `.gpkg` (`test_workflow.cpp:7531`).
   - `SurveyBundle::collectIntoSurvey` (`SurveyBundle.cpp:198-246`) copies local reference files from old survey folders, `AppLocalData`, `Temp`, exe dirs, or `.ka-survey-gen-*` into `<surveyDir>/가져온자료/<safeLayerName>/`.
   - `SurveyBundle::rememberSurveyDir` (`SurveyBundle.cpp:253-256`) writes `<surveyDir>` into project property `ka_hgis/survey_dir` (`SurveyStorage.cpp:1211`).
2. **On open** (`MainWindowSession.cpp:164, 425` -> `LayerOps::repairPersistedFileSources`, `LayerOps.cpp:2127-2202`):
   - Inspects invalid `ogr` and `gdal` layers (`LayerOps.cpp:2131-2136`).
   - `SurveyBundle::rebaseIntoSurvey` (`SurveyBundle.cpp:258-267`) substitutes `savedSurveyDir` (`ka_hgis/survey_dir`) with the current `surveyDir`.
   - `SurveyBundle::onSurveyDrive` (`SurveyBundle.cpp:269-278`) swaps a stale Windows drive letter (`X:\...`) to the drive hosting `surveyDir`.
   - `kaRestoreDroppedHomeComponent` (`LayerOps.cpp:2070-2077`) repairs stripped home folder prefixes.
   - `kaResolvePersistedSource` (`LayerOps.cpp:2079-2125`) tries candidate base paths, strips `.ka-survey-gen-home` and leading `../`, probes trailing path segments inside `surveyDir`, and falls back to `surveyGpkg|layername=...` for `.gpkg` sources. Unresolvable missing files trigger a non-fatal notification (`MainWindowSession.cpp:1119-1132`).

### What C++ never writes vs. what it actually writes into layer URIs
- **Never written to project `<properties>`**: `KaSecretStore` credentials (`ngii` password in `TopographicSettings.cpp:25, 79`) and raw `QSettings` INI keys (`VWorld/ApiKey`, `HistoryGis/ApiKey` in `VworldSettings.cpp:40-42`) — **no rule found** writing standalone key entries into `.qgs` `<properties>`.
- **Actually written into `<datasource>` layer URIs**:
  - VWorld WMTS/XYZ layers (`BasemapOps.cpp:1162` calling `withVworldApiKey` `:403-421`), VWorld WMS URIs (`makeVworldWmsUri` `:1189-1210` with `?KEY=...`), GDAL_WMS XML strings (`refreshedVworldGdalSource` `:425-493`), and HistoryGIS WMTS URLs embed the active API key directly in the `<datasource>` string persisted inside `.qgz` and `qgis_projects`.
  - On open, `MainWindow.cpp:1319` calls `LayerOps::refreshVworldApiKeyInLayers` (`BasemapOps.cpp:495-512`) to replace any persisted key with the local user's configured `VWorld/ApiKey`.

### What C++ refuses vs. what Rust must enforce
- **C++ refuses**:
  - Any project path recorded in `%LOCALAPPDATA%/ka-hgis/unsafe-projects.txt` (`KaSafeQgis.cpp:301-357`, `MainWindowSession.cpp:292-298, 326-329, 1669`).
  - Corrupt GPKGs failing header validation, missing `gpkg_contents`, or failing `PRAGMA quick_check` (`SurveyStorage.cpp:371-389`).
- **Data source outside the survey folder**: `no rule found` refusing external files in C++ (C++ deliberately permits and repairs external reference rasters/vectors). **Rust requirement**: validate normalized canonical paths, reject unresolved `..` traversal escaping intended root prefixes when auto-resolving companion paths, and treat missing outside-folder sources cleanly without crashing.
- **URL with an API key in it**: `no rule found` rejecting keyed URLs in C++ (C++ embeds and later refreshes keys). **Rust requirement** (`AGENTS.md` secret rule): NEVER write API keys (`key=`, `/req/wmts/1.0.0/<key>/`, `apiKey=`) into saved `.qgz`/`qgis_projects` datasources, logs, or tests; when reading a C++-created workspace containing an embedded key in `<datasource>`, strip the embedded key before retaining/logging the URL and inject only the local user's key from `strata-secrets` at tile-fetch time in memory.

---

## 4. What Rust must read first (ordered by visual impact)

When opening a C++-saved `.gpkg` / `.qgz`, read in this priority order so the survey renders cleanly and accurately:
1. **Work CRS & initial map extent** (rows `2`, `21`) — prevents coordinate shifts (`5186` vs `5187`) and fits the map on the actual survey site immediately.
2. **Layer visibility, Z-order, legend groups, & shown layer names** (rows `3`, `4`, `5`, `6`, `7`) — makes the left layer tree and polygon-over-point draw stack match C++.
3. **Simple & categorical vector styles + opacity + layer labels** (rows `8`, `9`, `10`, `11`, plus GPKG `layer_styles` fallback) — restores user colors, dashed boundaries, fill suppression, translucency, and site number labels.
4. **Basemaps, imported reference rasters/vectors, cadastral & DEM settings + moved-PC path repair & API key scrubbing** (rows `12`, `13`, `15`, `16`, `17`, `24`, `14`) — brings back ortho/topo/CAD overlays even when moved to another PC, without leaking or reusing stale embedded keys.
5. **Drawing sheets (`user_sheet`, `section_sheet`, `도면 보관: %1`)** (rows `22`, `23`) — restores print layout framing, grid settings, legend placement, and archived sheets in the Drawing tab.
6. **Snapping & measurement ellipsoid settings** (rows `19`, `20`) — matches editing snap tolerance/targets and planar measure rule.
7. **Workspace metadata, recovery & auxiliary DB items** (rows `1`, `25`, `26`, `18` properties) — title, auxiliary style catalog, and non-visual properties.

---

## 5. Proposed orders

> **Crate check (`Cargo.lock` & `Cargo.toml`)**: Zero new crates needed. Reuse existing workspace dependencies already pinned in `Cargo.toml`:
> - `zip = { version = "2.2", default-features = false, features = ["deflate"] }` (`Cargo.toml:45`; `Cargo.lock` resolves `zip 2.6.1` [`MIT`], `flate2 1.1.11` [`MIT OR Apache-2.0`], `miniz_oxide 0.8.9` [`MIT OR Zlib OR Apache-2.0`], `zlib-rs 0.6.8` [`Zlib`])
> - `quick-xml = "0.37"` (`Cargo.toml:46`; `Cargo.lock` resolves `quick-xml 0.37.5` [`MIT`])
> Already used in `crates/strata-check-export/src/user_sheet_xml.rs`. (For exact workspace pin rules, verify `strata-storage/Cargo.toml` on implementation.)

1. **`2D-1` — Shared `.qgz` / `qgis_projects` / `.qgs` reader & key sanitizer (read-only core)**
   - **Covers Section 2 rows**: `1, 2, 3, 4, 5, 6, 7, 24` (structural parsing of `<qgis>`, `<projectCrs>`, `<layerorder>`, `<layer-tree-group>`, `<layer-tree-layer>`, `<maplayer>`, `<customproperties>`, `<properties><Paths>`, `<properties><ka_hgis><survey_dir>`, and preference order `qgis_projects` -> `.qgz` -> `.qgs` -> `.bak.qgz` + stale marker detection).
   - **Rust files to change**: new module under `crates/strata-storage/src/` (e.g. `workspace_qgz_read.rs`), `crates/strata-storage/Cargo.toml`, refactor `crates/strata-check-export/src/user_sheet.rs:42-135` & `user_sheet_xml.rs` to share ZIP/hex extraction.
   - **C++ tests to port**: `test_save_open.cpp:4668-4850` (`oldVersionSurveysStillOpen`), `test_storage_generation.cpp:240` (`persistWork_companionFailureMarksStaleWorkspace`), `test_workflow.cpp:7400` (`restoreLastSurvey_prefersEmbeddedWorkspaceWhenSafe`).
   - **1-line pass test**: Opening a tempdir copy of `cpp-new-survey-5186.gpkg`/`.qgz` parses `EPSG:5186`, layer tree, Z-order, custom properties, and strips any `key=` query/path token from returned URIs.
   - **Crates**: `zip`, `quick-xml`, `rusqlite` (all in `Cargo.lock`).
   - **Size**: M

2. **`2D-2` — Apply open order: Work CRS, Z-order, visibility, names, groups, & initial extent (read-only UI/session)**
   - **Covers Section 2 rows**: `1, 2, 3, 4, 5, 6, 7, 21`.
   - **Rust files to change**: `crates/strata-app/src/gui_tab_home_open.rs:30-120`, `crates/strata-app/src/left_layer_panel.rs:35-80`, `crates/strata-app/src/real_survey_load.rs:10-60`, `crates/strata-app/src/extent_history.rs:1-40`.
   - **C++ tests to port**: `test_workflow.cpp:7503` (`gpkgEmbeddedProject_writesAndReadsBack`), `test_workflow.cpp:7475` (`atomicProjectWrite_writtenFileReadsBackAsQgz`).
   - **1-line pass test**: Opening a C++ workspace sets `work_crs` to `EPSG:5186` when saved as 5186, preserves unchecked layers as hidden, uses saved `<layer-tree-layer name>` and Z-order, and zooms to project data extent.
   - **Crates**: none beyond `2D-1`.
   - **Size**: S

3. **`2D-3` — Read vector styles (`ka_hgis/style_*`, `<renderer-v2>`, `layer_styles`), opacity & labels (read-only render)**
   - **Covers Section 2 rows**: `8, 9, 10, 11, 18` (styles/labels on absorbed tables).
   - **Rust files to change**: `crates/strata-storage/src/` (QML `layer_styles` + `<renderer-v2>` + `ka_hgis/style_*` / `ka_hgis/label_*` parser), `crates/strata-app/src/survey_style.rs`, `crates/strata-app/src/real_survey_types.rs:45-65`, `crates/strata-app/src/real_survey_load.rs:20-50`.
   - **C++ tests to port**: `test_workflow.cpp:7363` (`leftoverRestore_keepsUserFillColorNotFactoryDomainStyle`).
   - **1-line pass test**: Layer colors, dashed/no-fill flags, opacity, and label field/color survive opening a C++-saved `.gpkg` both via `qgis_projects` and `LayersOnly` `layer_styles` fallback.
   - **Crates**: `quick-xml` (already in `Cargo.lock`).
   - **Size**: M

4. **`2D-4` — Reference maps, basemap selection, moved-PC path repair & safe URL re-keying (read-only sources)**
   - **Covers Section 2 rows**: `12, 13, 14, 15, 16, 17, 24` (+ `13` `skip_auto_cadastral`).
   - **Rust files to change**: `crates/strata-storage/src/` (`repair_persisted_file_sources` helper), `crates/strata-app/src/gui_tab_home_open.rs:58-75`, `crates/strata-app/src/tabs_home_map_state.rs`.
   - **C++ tests to port**: `test_storage_safety.cpp:995` (`repairPersisted_restoresDroppedWindowsHome`), `test_storage_safety.cpp:1036` (`persistWorkspace_keepsExternalHeritagePathOnReopen`), `test_cad_layers.cpp:103` (`addToProject_buildsTheGroupUnderReference`).
   - **1-line pass test**: Copying a survey + `가져온자료` folder to a new tempdir path resolves moved GeoTIFF/SHP reference sources cleanly, ignores traversal escapes, and scrubs persisted VWorld keys while rendering with the local runtime key.
   - **Crates**: none beyond `2D-1`.
   - **Size**: M

5. **`2D-5` — Read snapping, measure ellipsoid & drawing sheets (`user_sheet`, `section_sheet`, `도면 보관: %1`)**
   - **Covers Section 2 rows**: `19, 20, 22, 23` (plus `26` auxiliary metadata tolerance).
   - **Rust files to change**: `crates/strata-storage/src/` (layout + snapping/measure parser), `crates/strata-check-export/src/user_sheet_xml.rs`, `crates/strata-app/src/tab_drawing.rs`, `crates/strata-app/src/gui_tab_drawing_sheet.rs`, `crates/strata-app/src/survey_measure.rs`.
   - **C++ tests to port**: `test_drawing_studio.cpp:164` (`sheetSet_storeAndReopenKeepsWorkingSheet`), `test_workflow.cpp:3080` (`snapSettings_surviveProjectWriteAndReopen`), `test_save_open.cpp:293, 4801, 4850` (`oldVersionSurveysStillOpen`).
   - **1-line pass test**: Opening a C++-saved workspace restores active `user_sheet` grid/title/callout properties, lists archived `"도면 보관: %1"` sheets, and applies saved snap settings.
   - **Crates**: `quick-xml` (already in `Cargo.lock`).
   - **Size**: M

6. **`2D-6` — Write workspace (`qgis_projects`, `layer_styles`, atomic `.qgz` + `.bak.qgz` + `오래됨.txt`, bundling, zero persisted keys)**
   - **Covers Section 2 rows**: `1-25` (write path; note `26` stays preserve-or-omit unless auxiliary style DB editing is added).
   - **Rust files to change**: `crates/strata-storage/src/survey_gpkg_save.rs:1-80`, `crates/strata-storage/src/survey_gpkg_create.rs:1-120`, new workspace writer module in `crates/strata-storage/src/`, `crates/strata-app/src/app_state_projects.rs:1-290`.
   - **C++ tests to port**: `test_workflow.cpp:7438` (`atomicProjectWrite_neverLeavesPartialFile`), `test_workflow.cpp:7454` (`atomicProjectWrite_keepsOneBackupGeneration`), `test_workflow.cpp:7475` (`atomicProjectWrite_writtenFileReadsBackAsQgz`), `test_workflow.cpp:7503` (`gpkgEmbeddedProject_writesAndReadsBack`), `test_workflow.cpp:7531` (`surveyFileTravelsAloneWithAbsorbedShp`), `test_storage_generation.cpp:240` (`persistWork_companionFailureMarksStaleWorkspace`).
   - **1-line pass test**: Saving a Rust-edited survey writes valid `.qgz` into `qgis_projects`, updates `layer_styles` and sibling `.qgz` atomically with `.bak.qgz` rotation, bundles local references into `가져온자료`, and contains zero API key substrings in the output XML.
   - **Crates**: `zip`, `quick-xml` (both in `Cargo.lock`).
   - **Size**: L

---

## Could not determine
- **Exact `custom_properties` written unconditionally vs. conditionally for third-party QGIS plugins/auxiliary `.db`**: Beyond `ProjectStyleSettings projectStyleId="attachment:///..._styles.db"` observed in the reference `.qgz` schema, no full inventory of runtime-generated QGIS internal `<mapcanvas>` elements appears in `cpp-new-survey-5186.qgz` because a headless/factory project has no interactive `QgsMapCanvas` XML block until saved interactively from the GUI.
- **Outside-folder refusal & keyed-URL refusal policy in C++**: `no rule found` in `target/cpp-ref/src` — C++ intentionally repairs external paths (`LayerOps.cpp:2127-2202`) and writes then refreshes VWorld API keys in `<datasource>` (`BasemapOps.cpp:403-512`), whereas Rust safety requirements in Section 3 are derived from repository policy (`AGENTS.md`) rather than existing C++ refusal code.
- **Row `26` interactive `_styles.db` usage**: Whether any Strata feature besides QGIS core symbol presets writes entries into `<6char>_styles.db` inside `.qgz` (`no rule found` in `target/cpp-ref/src`).

---

## Counts
- **things stored**: `26`
- **by Rust state**:
  - `READ`: `0` (strictly read in full; note rows `18` and `22` are partial — row `18` reads GPKG table geometries only while losing custom properties/styles; row `22` reads only `user_sheet` map-item existence for submit check while ignoring layout settings)
  - `KEPT ON SAVE BUT NOT READ`: `26` (24 purely `KEPT ON SAVE BUT NOT READ` + 2 partial `READ` / `KEPT ON SAVE BUT NOT READ` on `VACUUM INTO` GPKG save; note companion `.qgz` is left stale by Rust GPKG save)
  - `LOST ON SAVE`: `0` (verified: `VACUUM INTO` in `crates/strata-storage/src/survey_gpkg_save.rs:30-37` preserves `qgis_projects` and `layer_styles`, though note Rust does not update companion `.qgz` or write new edits into workspace tables)
  - `WRITTEN`: `0` (only `KA_HGIS_SCHEMA_VERSION` GPKG metadata in `crates/strata-storage/src/survey_gpkg_create.rs:80-89` and emergency `.crash_recovery.gpkg` state in `crates/strata-storage/src/gpkg_repository.rs:120-176`)
- **C++ tests found**: `14` (`test_workflow.cpp:3080, 7363, 7400, 7438, 7454, 7475, 7503, 7531`; `test_storage_generation.cpp:240`; `test_storage_safety.cpp:995, 1036`; `test_save_open.cpp:4850`; `test_drawing_studio.cpp:164`; `test_cad_layers.cpp:103`)
- **proposed orders**: `6` (`2D-1` through `2D-6`)
