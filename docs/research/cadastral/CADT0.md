# `CADT0` — C++ Digital Cadastral Map (`수치지적도`) Test Specification

Read-only line-by-line inventory of `target/cpp-ref/tests/test_cadastral.cpp` (`903` lines) and the production units it exercises (`src/core/CadastralImport.{h,cpp}`, `src/core/CadastralEncoding.{h,cpp}`, `src/core/CadastralPortal.{h,cpp}`, `src/app/MainWindowCadastral.cpp`, `src/core/LayerOps.{h,cpp}`, `src/core/BasemapOps.cpp`, `src/core/PreparedReferenceMap.h`, `src/core/TopographicArchive.{h,cpp}`), paired against the Rust workspace (`crates/`) at `HEAD` `845f290`.

- **Scope boundary**: Covers the digital vector cadastral flow (`수치지적도`) behind the `지적` ribbon button (`지적도 · 조사 주변 5km`), plus the six non-import slots that share `tests/test_cadastral.cpp`. The yellow VWorld WMS raster overlay (`연속지적도`) is out of scope.
- **CMake registration check (`ORDER-CADT0.yaml:24`)**:
  - `tests/CMakeLists.txt`: `not in the source` (does not exist; all CTest targets are registered in top-level `CMakeLists.txt`).
  - Primary suite: `add_executable(ka_cadastral_tests tests/test_cadastral.cpp)` registered via `ka_add_qtest(cadastral ka_cadastral_tests)` with `TIMEOUT 90` at `CMakeLists.txt:1926-1929` (Windows `ENVIRONMENT` at `CMakeLists.txt:2110`; aggregated at `CMakeLists.txt:2142`).
  - Other CTest targets in `CMakeLists.txt` that register cadastral-related test slots outside `tests/test_cadastral.cpp`:
    - `ka_download_ui_tests` (`CMakeLists.txt:1723`, `tests/test_download_ui.cpp:76-119`): `cadastralProgressKeepsCompletionVisibleAndCancelsOnce` (`_data` `:76-80`).
    - `ka_save_open_tests` (`CMakeLists.txt:1934`, `tests/test_save_open.cpp`): filter `save_open_edit_delete` (`CMakeLists.txt:1987-1994`) runs `layerDeleteKeyRemovesCadastralAndKeepsItGone` (`:3089`), `cadastralGroupDeleteKeyRemovesChildren` (`:3116`), `cadastralGroupContextMenuCanRemove` (`:3227`); filter `save_open_erase` (`CMakeLists.txt:2004-2012`) runs `polygonCommands_leaveReferenceAndCadastralShapesAlone` (`:3922`).
    - `ka_account_status_tests` (`CMakeLists.txt:1579`, `tests/test_account_status.cpp:15`): checks `cadastralReady` in `AccountStatus::snapshot()`.
    - `ka_alert_polish_tests` (`CMakeLists.txt:1524`, `tests/test_alert_polish.cpp:55`): `mapAlignAndCadastralAlertsUseTheOneFrame`.
    - `ka_editing_tools_tests` (`CMakeLists.txt:853`, `tests/test_editing_tools.cpp:269`): `attributeEditingSkipsReferenceAndCadastral`.
    - `ka_heritage_region_tests` (`CMakeLists.txt:1571`, `tests/test_heritage_region.cpp:77`): `rejectedKeyExplainsBothAutomaticRegionAndCadastral`.
    - `ka_layer_list_chrome_tests` (`CMakeLists.txt:2373`, `tests/test_layer_list_chrome.cpp:101, 116`): `sections_cadastralStaysAtRoot`, `sections_vworldCadastralPictureJoinsCadastral`.
    - `ka_layer_roles_tests` (`CMakeLists.txt:1595`, `tests/test_layer_roles.cpp:30, 48, 98`): `LayerRole::Kind::Cadastral` resolution.
    - `ka_layer_state_tests` (`CMakeLists.txt:1583`, `tests/test_layer_state.cpp:532, 550, 559, 617`): `officialCadastralCountsSoAutoVworldIsNotNeeded`, `userRemovedCadastralStopsAutoAdd`, `referenceGroupListsOnlyCadastralPictures`, `referenceRootDeleteKeepsCadastralOut`.
    - `ka_parcel_copy_tests` & `ka_parcel_copy_keys_tests` (`CMakeLists.txt:865, 871`, `tests/test_parcel_copy.cpp`, `tests/test_parcel_copy_keys.cpp:201`): parcel-copy and snap-trace over cadastral polygons.
    - `ka_polygon_erase_tests` (`CMakeLists.txt:1528`, `tests/test_polygon_erase.cpp:196`): `referenceAndCadastralShapesAreNeverCutOrUsed`.
    - `ka_workflow_tests` (`CMakeLists.txt:839`, `tests/test_workflow.cpp:2595, 3148, 3818, 3825, 5373, 5383, 6099`): VWorld WMS/startup cadastral picture and snap-exclusion checks.
- **Source corrections against `ORDER-CADT0.yaml` and `ag/cad0:docs/parity/cadastral-flow.md` (`ORDER-CADT0.yaml:25-26, 73`)**:
  1. `ORDER-CADT0.yaml:25` cites `src/app/MainWindowCadastral.cpp:134-175`; the file ends at line `174` (`src/app/MainWindowCadastral.cpp:134-174`).
  2. `ag/cad0:docs/parity/cadastral-flow.md` (Section 5, step 15) states `crates/strata-geo/src/cadastral_parcel.rs:62-97` *"only implements `focus_parcel` on in-memory `CadastralParcel` structs"*. Source wins: `crates/strata-geo/src/cadastral_parcel.rs:43-141` implements `analyze_cadastral_inclusion` (`KaCadastralOverlapAnalyzer::analyzeCadastralInclusion`); no `focus_parcel` function exists anywhere in `crates/`.
  3. `ag/cad0:docs/parity/cadastral-flow.md` (Section 5, step 10) marks `crates/strata-net/src/vworld_cadastral.rs` `select_cadastral_resources` as `REAL` for 1-to-1 county matching without flagging a bug exposed by `selectsExactProvinceAndNestedDistrict` (`tests/test_cadastral.cpp:624-644`): in `crates/strata-net/src/vworld_cadastral.rs:183-185`, `let mut sections = stem.split('_'); let file_province = sections.next().unwrap_or(""); let file_county = sections.next().unwrap_or("");` keeps only the **first** `_`-token after the province (`"수원시"`), whereas C++ `name.section('_', 1)` (`src/core/CadastralPortal.cpp:192`) keeps **the entire remainder** (`"수원시_영통구"`). On `"경기도 수원시 영통구"`, `county == "수원시_영통구"` vs. `file_county == "수원시"`, so Rust returns `0` matches and fails.
  4. In `crates/strata-net/src/vworld_cadastral.rs:90-95`, `parse_cadastral_resources` splits `listFnc.download(...)` by single quote `'` only (`args.split('\'')`), whereas C++ `src/core/CadastralPortal.cpp:161` matches `['\"]`. Also, as `cadastral-flow.md` noted, `VworldClient::NO_ACCOUNT_MESSAGE` (`crates/strata-net/src/vworld_cadastral.rs:254-255`) holds C++'s missing-**API-key** string (`src/core/CadastralPortal.cpp:214`) instead of C++'s missing-**account** string (`"지적도 메뉴에서 VWorld 계정을 설정하세요."`, `src/core/CadastralPortal.cpp:216`).

### Shared File-Scope Helpers in `tests/test_cadastral.cpp:39-88`

Referenced by name in Section 1 below:

- `sourceCrs()` (`tests/test_cadastral.cpp:40`): returns `QgsCoordinateReferenceSystem(QStringLiteral("EPSG:5186"))`.
- `box(double x, double y, double width, double height)` (`tests/test_cadastral.cpp:41-43`): returns `QgsGeometry::fromRect(QgsRectangle(x, y, x + width, y + height))`.
- `pnu(int parcel)` (`tests/test_cadastral.cpp:44-46`): returns `QStringLiteral("471901010010%1").arg(parcel, 7, 10, QLatin1Char('0'))` (`19`-char string, e.g., `pnu(1)` = `"4719010100100000001"`).
- `struct Parcel { int id; QString jibun; QgsGeometry geometry; };` (`tests/test_cadastral.cpp:47`).
- `writeParcels(path, parcels, withJibun = true, validCrs = true)` (`tests/test_cadastral.cpp:50-75`): creates memory `QgsVectorLayer` named `"synthetic parcels"` with URI `"Polygon?field=PNU:string(19)&field=OWNER:string(80)"` + optional `"&field=JIBUN:string(40)"` + optional `"&crs=EPSG:5186"` (resets to invalid `QgsCoordinateReferenceSystem()` when `!validCrs`), populates each `Parcel` with `PNU = pnu(parcel.id)`, `OWNER = "제외할 개인정보"`, and optional `JIBUN = parcel.jibun`, and writes it via `QgsVectorFileWriter::writeAsVectorFormatV3` with `driverName = "ESRI Shapefile"` (if `path.endsWith(".shp")`) or `"GPKG"`, `fileEncoding = "UTF-8"`, `layerName = "parcels"`.
- `fileHashes(directory)` (`tests/test_cadastral.cpp:76-84`): returns `QMap<QString, QByteArray>` of filename (`QDir::Files`) -> raw 32-byte `QCryptographicHash::Sha256` digest.
- `outputUri(result)` (`tests/test_cadastral.cpp:85-87`): returns `result.gpkgPath + "|layername=" + result.tableName`.

---

## 1. Every Test Function in `tests/test_cadastral.cpp`

`class CadastralTest` (`tests/test_cadastral.cpp:90-891`) defines **22 test functions** and **4 companion QtTest `_data()` slots** (`26` `private slots:` methods total). Each `_data()` slot is documented inline with its test function.

| name | lines | what it builds first (fixture, one line) | the production call(s) it makes | the inputs that matter, verbatim (radii, CRS codes, file names, field names, encodings, sizes) | EVERY expected value, verbatim (counts, strings, numbers with their tolerance) | Rust equivalent test (file:line) or `none` |
| --- | --- | --- | --- | --- | --- | --- |
| `mapServerMessageIsBenign` (with `mapServerMessageIsBenign_data`) | `tests/test_cadastral.cpp:114-118` (`_data` `:94-112`) | Seven QtTest data rows pairing server/provider `message` strings with `benign` booleans. | `LayerOps::mapServerMessageIsBenign(message)` (`src/core/BasemapOps.cpp:1132-1139`) | 7 rows (`message`): `"범례 미지원"` -> `"Returned legend image is flawed [URL: https://api.vworld.kr/req/wms?SERVICE=WMS&REQUEST=GetLegendGraphic]"`; `"국토 밖 타일"` -> `"Tile request error (Status: 200; Content-Type: application/xml;charset=UTF-8; Length: 436)"`; `"렌더 취소"` -> `"Rendering was canceled"`; `"연결 거절"` -> `"Tile request error (Status: 403; Content-Type: text/html)"`; `"응답 없음"` -> `"Connection to server refused"`; `"시간 초과"` -> `"요청 시간 초과 · api.vworld.kr"`; `"빈 메시지"` -> `QString()` (`""`). | `benign` per row in order: `true`, `true`, `true`, `false`, `false`, `false`, `false`. | `none` |
| `trickleResponseHasAbsoluteDeadline` | `tests/test_cadastral.cpp:121-141` | Localhost `QTcpServer` sending `"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n"` then `"x"` every `20` ms via `QTimer`. | `CadastralPortal::downloadBytesForTest(url, 200, &error)` (`src/core/CadastralPortal.cpp:344-352`) | URL `"http://127.0.0.1:%1/trickle"` (`server.serverPort()`), `Content-Length: 100000`, trickle chunk `"x"` every `20` ms, `absoluteTimeoutMs = 200`. | `server.listen(QHostAddress::LocalHost) == true`; `body.isEmpty() == true`; `timer.elapsed() < 2500` (ms); `error.contains("응답 시간이 초과") == true` (full error in production: `"지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요."`, `src/core/CadastralPortal.cpp:67`). | `none` |
| `historyGis1919_emptyKeyDisablesAddAndLeavesNoLayer` | `tests/test_cadastral.cpp:143-153` | Empty `QgsProject proj` and empty `QString err`. | `LayerOps::historyGisApiKeyUsable`, `LayerOps::addHistoryGisMap1919(&proj, nullptr, QString(), &err)` (`src/core/BasemapOps.cpp:1482-1484, 1525-1545`) | `apiKey = QString()` (`""`), `apiKey = "   "`, `addHistoryGisMap1919(&proj, nullptr, QString(), &err)`. | `!historyGisApiKeyUsable("")`; `!historyGisApiKeyUsable("   ")`; `!addHistoryGisMap1919(...)`; `err.contains("역사지리정보DB") == true`; `err.contains("API 키") == true`; `proj.mapLayers().isEmpty() == true`; `!err.contains("840e2e2c", Qt::CaseInsensitive) == true`. | `crates/strata-app/src/ref_tile_history_tests.rs:60-81` (`the_1919_map_needs_its_key_and_a_survey_area`; blank-key URL check at `crates/strata-app/src/ref_tile_history_tests.rs:14-21`) |
| `historyGis1919_placeholderKeyBuildsOfficialUriWithoutSampleSecret` | `tests/test_cadastral.cpp:155-171` | Placeholder API key string `key = "TEST-KEY-1919"`. | `LayerOps::historyGisApiKeyUsable(key)`, `LayerOps::historyGisWmtsUri(key, "history:map1919")`, `LayerOps::historyGisWmtsUri(key, "map1919")` (`src/core/BasemapOps.cpp:1482-1504`) | `key = "TEST-KEY-1919"`, `layerId = "history:map1919"` and `"map1919"`. | `historyGisApiKeyUsable("TEST-KEY-1919") == true`; `uri` contains `"hgis.history.go.kr"`, `"GetCapabilities"`, `"TEST-KEY-1919"`, `"history:map1919"`, `"EPSG:5179"`, `"tileMatrixSet=EPSG:5179"`; `!uri.contains("840e2e2c", Qt::CaseInsensitive)`; `fallback` contains `"layers=map1919"` and `!fallback.contains("840e2e2c", Qt::CaseInsensitive)`. | `crates/strata-app/src/ref_tile_history_tests.rs:14-21` (`a_tile_of_the_1919_map_is_asked_for_in_the_servers_grid` — adapted to direct WMTS `GetTile` URL; no `map1919` override) |
| `historyGis1919_retriesTheRealLayerAndHidesTheKeyWhenItFails` | `tests/test_cadastral.cpp:175-184` | Placeholder `key = "TEST-KEY-1919"` and simulated WMS error string containing `apiKey%3D` + `key`. | `LayerOps::historyGisMap1919Uris(key)`, `LayerOps::historyGisWmtsUri(key, "history:map1919")`, `LayerOps::historyGisFailureText(raw, key)` (`src/core/BasemapOps.cpp:1486-1523`) | `key = "TEST-KEY-1919"`; `raw = "WMS provider: Tile layer or tile matrix set not found (URI: ...apiKey%3DTEST-KEY-1919)"`. | `uris.size() >= 4` (production returns `4`); every `uri` in `uris` `== LayerOps::historyGisWmtsUri("TEST-KEY-1919", "history:map1919")`; `shown.contains("잠시 뒤") == true`; `!shown.contains("TEST-KEY-1919") == true`. | `none` |
| `daedongyeojido_buildsOfficialNgiiWmsUriWithoutKey` | `tests/test_cadastral.cpp:186-202` | None (direct URI builder calls). | `LayerOps::daedongyeojidoWmsUri()`, `LayerOps::daedongyeojidoWmsUri("korea_oldmap_addAlphaChannel")` (`src/core/BasemapOps.cpp:1547-1563`) | Default `layerId = ""` (`"korea_oldmap_ddymap_kyu"`) and explicit `layerId = "korea_oldmap_addAlphaChannel"`. | `uri` contains `"map.ngii.go.kr"`, `"korea_old_map"`, `"korea_oldmap_ddymap_kyu"`, `"EPSG:5179"`, `"version=1.1.1"`; `uri` does NOT contain `"apiKey="` (case-insensitive), `"apikey="` (case-insensitive), `"840e2e2c"` (case-insensitive), `"mt1.google.com"`, `"kakao"`; `fallback` contains `"layers=korea_oldmap_addAlphaChannel"` and `!fallback.contains("apiKey=", Qt::CaseInsensitive)`. | `crates/strata-app/src/ref_tile_layers_tests_b.rs:58-72` (`test_f1_old_map_tile_address_is_the_wms_getmap_of_the_tile_square` — adapted to `SRS=EPSG:3857` tile `GetMap`; no `korea_oldmap_addAlphaChannel` fallback) |
| `focusesOnlyExactPnuWithoutEditing` (with `focusesOnlyExactPnuWithoutEditing_data`) | `tests/test_cadastral.cpp:210-255` (`_data` `:204-208`) | `lookup.gpkg` (`EPSG:5186`, 3 features: two `pnu(1)` parts and one `pnu(2)`) loaded as hidden OGR layer `"지적도"` in `QgsProject` + `800x600` `QgsMapCanvas` (`destination` CRS). | `CadastralImport::applyStyle(layer)`, `CadastralImport::focusParcel(&project, &canvas, pnu, &marker)` (`src/core/CadastralImport.cpp:244-267, 296-340`) | `_data` rows: `"5186"` -> `"EPSG:5186"`, `"5187"` -> `"EPSG:5187"`; `writeParcels("lookup.gpkg", {{1, "1", box(200000, 550000, 25, 25)}, {1, "1", box(200060, 550000, 20, 20)}, {2, "2", box(202000, 550000, 20, 20)}})` (`EPSG:5186`, `UTF-8`, fields `PNU`, `OWNER`, `JIBUN`); `canvas.resize(800, 600)`; layer `"지적도"` unchecked; `focusParcel` queries: `pnu(1)` (`"4719010100100000001"`), `pnu(999)` (`"4719010100100000999"`), `"1' OR 1=1"`. | `!path.isEmpty()`; `layer->isValid()`; `applyStyle(layer) == true`; `focusParcel(..., pnu(1), &marker) == true`; `layer->selectedFeatureCount() == 2`; `node->isVisible() == true`; `canvas.extent()` contains transformed `(200010, 550010)`, transformed `(200070, 550010)`, and `marker`, and does NOT contain transformed `(202010, 550010)`; `project.crs().authid() == destination`; `layer->crs().authid() == "EPSG:5186"`; `!layer->isEditable()`; `!focusParcel(..., pnu(999), &marker)`; `!focusParcel(..., "1' OR 1=1", &marker)`; `layer->selectedFeatureIds() == selection`; `canvas.extent() == extent`; `fileHashes(files.path()) == original`. | `none` |
| `importsWholeIntersectingParcels` (with `importsWholeIntersectingParcels_data`) | `tests/test_cadastral.cpp:263-324` (`_data` `:257-262`) | `source.<suffix>` (3 parcels in `EPSG:5186`) + `crossing.gpkg` (1 parcel in `EPSG:5186`) + diamond `scope` transformed to `workAuthId`. | `CadastralImport::prepare({source, extra}, scope, workCrs, context, output.path())` (`src/core/CadastralImport.cpp:66-242`) | `_data` rows: `"shp-same-crs"` -> `("shp", "EPSG:5186")`, `"gpkg-reproject"` -> `("gpkg", "EPSG:5187")`; `source.<suffix>` (`EPSG:5186`, `UTF-8`, fields `PNU`, `OWNER="제외할 개인정보"`, `JIBUN`): `{1, "산 12-3", box(200045., 550000., 40., 10.)}`, `{2, "15", box(199995., 549995., 10., 10.)}`, `{3, "999", box(200080., 550080., 10., 10.)}`; `crossing.gpkg`: `{4, "16-1", box(200075., 550000., 40., 10.)}`; `scope` WKT in `EPSG:5186` (then transformed to `workCrs`): `"POLYGON((200000 549900,200100 550000,200000 550100,199900 550000,200000 549900))"`. | `result.isReady() == true`; `result.tableName == "cadastral"`; `layer.isValid() == true`; `layer.crs() == workCrs`; `layer.featureCount() == 3LL` (parcels `1`, `2`, `4` kept; parcel `3` dropped); `layer.hasSpatialIndex() == Qgis::SpatialIndexPresence::Present`; upper-cased field set contains `"PNU"` and `"JIBUN"` and is empty after removing `"PNU"`, `"JIBUN"`, `"FID"`; `JIBUN` values: `pnu(1) -> "산 12-3"`, `pnu(2) -> "15"`, `pnu(4) -> "16-1"`; for each kept feature, `difference = feature.geometry().symDifference(fullParcel_in_workCrs)` has `!difference.isNull()` and `difference.area() < 0.01` (m²); `fileHashes(input.path()) == before`. | `none` (reusable building blocks in `crates/strata-refmap/src/heritage_tests.rs:144-167`, `crates/strata-engine/src/scope_tests.rs:166-173`, `crates/strata-engine/src/vector_table_tests.rs:59-77`) |
| `overlappingSourcesDeduplicatePnu` | `tests/test_cadastral.cpp:326-344` | `first.gpkg` (`{a}`) and `second.shp` (`{a, b}`) in `EPSG:5186` with identical parcel `a`. | `CadastralImport::prepare({first, second}, box(199900., 549900., 400., 400.), sourceCrs(), {}, output.path())` (`src/core/CadastralImport.cpp:66-242`) | `a = {1, "1", box(200000., 550000., 10., 10.)}`, `b = {2, "2", box(200020., 550000., 10., 10.)}`; `first.gpkg`, `second.shp` (`EPSG:5186`, `UTF-8`); `scope = box(199900., 549900., 400., 400.)`; `crs = "EPSG:5186"`. | `result.isReady() == true`; `layer.featureCount() == 2LL`; `unique` `PNU` set `== QSet<QString>({pnu(1), pnu(2)})` (`{"4719010100100000001", "4719010100100000002"}`). | `none` |
| `reusesCompletedCacheWithoutRewritingOriginalOrOutput` | `tests/test_cadastral.cpp:346-376` | `source.gpkg` (`{1, "산 12-3", box(200000., 550000., 10., 10.)}`) prepared once, with output `.gpkg` mtime backdated to `946684800` UTC before a second `prepare`. | `CadastralImport::prepare({source}, scope, sourceCrs(), {}, output.path())` (called twice) (`src/core/CadastralImport.cpp:66-242`) | `source.gpkg` (`EPSG:5186`, `UTF-8`); `scope = box(199900., 549900., 400., 400.)`; `crs = "EPSG:5186"`; mtime `QDateTime::fromSecsSinceEpoch(946684800, QTimeZone::UTC)`. | `first.isReady() == true`; `second.isReady() == true`; `second.gpkgPath == first.gpkgPath`; `QFileInfo(second.gpkgPath).lastModified() == modified` (`946684800` UTC); `fileHashes(output.path()) == preparedHashes`; `fileHashes(input.path()) == originals`; `cached.isValid() == true`; `cached.featureCount() == 1LL`; `feature.attribute("JIBUN").toString() == "산 12-3"`. | `none` (ZIP extraction cache reuse only in `crates/strata-topo/src/archive_tests.rs:22-53`) |
| `samePnuKeepsDistinctGeometryAndDeduplicatesEquivalentRings` | `tests/test_cadastral.cpp:378-405` | `first.gpkg` (`firstGeometry`) and `second.gpkg` (`reversedRing` + disjoint `secondGeometry`), all sharing `pnu(1)`. | `CadastralImport::prepare({first, second}, box(199900., 549900., 400., 400.), sourceCrs(), {}, output.path())` (`src/core/CadastralImport.cpp:66-242`) | `firstGeometry = box(200000., 550000., 10., 10.)`; `secondGeometry = box(200030., 550000., 10., 10.)`; `reversedRing = "POLYGON((200010 550010,200010 550000,200000 550000,200000 550010,200010 550010))"`; all with `id = 1` (`pnu(1)`), `jibun = "1"`, `EPSG:5186`; `scope = box(199900., 549900., 400., 400.)`. | `result.isReady() == true`; `layer.featureCount() == 2LL`; every output feature has `PNU == pnu(1)` (`"4719010100100000001"`); `difference = combined.symDifference(firstGeometry.combine(secondGeometry))` has `!difference.isNull()` and `difference.area() < 0.01` (m²). | `none` |
| `rejectsInvalidInputs` (with `rejectsInvalidInputs_data`) | `tests/test_cadastral.cpp:412-428` (`_data` `:407-411`) | `source.shp` (`{1, "1", box(200000., 550000., 10., 10.)}`) mutated across 5 invalid-input cases. | `CadastralImport::prepare({source}, scope, crs, {}, output.path())` (`src/core/CadastralImport.cpp:66-242`) | 5 `caseName` rows: `"missing-jibun"` (`withJibun = false`), `"missing-crs"` (`validCrs = false`), `"missing-file"` (`source = input.filePath("absent.shp")`), `"invalid-work-crs"` (`crs = QgsCoordinateReferenceSystem()`), `"empty-scope"` (`scope = QgsGeometry()`); otherwise `scope = box(199900., 549900., 400., 400.)`, `crs = "EPSG:5186"`. | For all 5 rows: `result.status == PreparedReferenceMap::Status::Failed`; `!result.error.isEmpty()`; `!result.isReady()`. | `none` (reusable building blocks in `crates/strata-engine/src/shapefile_tests.rs:66-74`, `crates/strata-refmap/src/heritage_tests.rs:166`, `crates/strata-engine/src/scope_tests.rs:21-24, 155-161`) |
| `cancellationDoesNotReturnReady` | `tests/test_cadastral.cpp:430-443` | `source.gpkg` (`{1, "1", box(200000., 550000., 10., 10.)}`) with pre-cancelled callback `[] { return true; }`. | `CadastralImport::prepare({source}, box(199900., 549900., 400., 400.), sourceCrs(), {}, output.path(), [] { return true; })` (`src/core/CadastralImport.cpp:66-242`) | `source.gpkg` (`EPSG:5186`), `scope = box(199900., 549900., 400., 400.)`, `crs = "EPSG:5186"`, `cancel = [] { return true; }`. | `result.status == PreparedReferenceMap::Status::Cancelled`; `!result.isReady()`; `!result.outputCommitted`; `fileHashes(input.path()) == original`. | `none` (reusable cancel patterns in `crates/strata-refmap/src/soil_prepare_tests_b.rs:95-103`, `crates/strata-topo/src/archive_tests.rs:94-105`) |
| `cancellationDuringPreparationLeavesNoCompletedMap` | `tests/test_cadastral.cpp:445-459` | `source.gpkg` (`{1, "1", box(200000., 550000., 10., 10.)}`) with `cancel` flipped to `true` inside `progress` when `progress < 100`. | `CadastralImport::prepare({source}, box(199900., 549900., 400., 400.), sourceCrs(), {}, output.path(), [&] { return cancel; }, [&](int progress, const QString&) { if (progress < 100) cancel = true; })` (`src/core/CadastralImport.cpp:66-242`) | `source.gpkg` (`EPSG:5186`), `scope = box(199900., 549900., 400., 400.)`, `crs = "EPSG:5186"`, trigger `if (progress < 100) cancel = true`. | `cancel == true`; `result.status == PreparedReferenceMap::Status::Cancelled`; `!result.outputCommitted`; `QDir(output.path()).entryList({"*.gpkg"}, QDir::Files).isEmpty() == true`. | `none` (reusable mid-run cancel pattern in `crates/strata-refmap/src/soil_prepare_tests_b.rs:105-127`) |
| `shapefileWithoutCpgKeepsKoreanJibun` | `tests/test_cadastral.cpp:463-522` | Three DBF byte samples plus `LSMD_CONT_LDREG_fixture.shp` (`EPSG:5186`) whose `.dbf` is binary-patched to CP949 `"\xBB\xEA 23-1"` and whose `.cpg` is removed then restored. | `CadastralEncoding::detectDbfEncoding`, `CadastralEncoding::fallbackFor`, `CadastralImport::prepare` (`src/core/CadastralEncoding.cpp:31-65`, `src/core/CadastralImport.cpp:66-242`) | Samples: `QByteArray("\xBB\xEA", 2) + QByteArray(" 23-1    ")`; `QStringLiteral("산 23-1   산 24   답 3   대 5").toUtf8()`; `QByteArray("1-2 3-4 5")`. Shapefile: `"LSMD_CONT_LDREG_fixture.shp"`, `"Polygon?field=PNU:string(19)&field=JIBUN:string(40)&crs=EPSG:5186"`, `PNU = pnu(7)`, `JIBUN = "XX 23-1"` patched at `"XX"` -> `"\xBB\xEA"`, `box(200000., 550000., 10., 10.)`, `.cpg` deleted, `scope = box(199900., 549900., 400., 400.)`, `crs = "EPSG:5186"`; then `.cpg` written with `"CP949"`; plus `"other.gpkg"`. | `detectDbfEncoding` -> `"CP949"`, `"UTF-8"`, `"CP949"`; without `.cpg`: `fallbackFor(shp) == "CP949"`, `prepared.isReady() == true`, `result` (`<gpkgPath>|layername=cadastral`) valid, `kept.attribute("JIBUN").toString() == "산 23-1"`; with `.cpg` (`"CP949"`): `fallbackFor(shp).isEmpty() == true`; for `"other.gpkg"`: `fallbackFor(...).isEmpty() == true`. | `none` (reusable `heritage`/`strata-engine` tests: `crates/strata-refmap/src/heritage_tests.rs:45-68`, `crates/strata-engine/src/shapefile_tests.rs:49-62`; note `heritage::shapefile_encoding` does not skip the dBASE binary header) |
| `referenceLayerHasOutlineAndOptionalJibunLabels` | `tests/test_cadastral.cpp:524-599` | Prepared `source.gpkg` (`{1, "산 23-1", box(200000., 550000., 10., 10.)}`) added to `EPSG:5187` `QgsProject` alongside `"기존 조사"`, restyled (`#d02030`, `labels = false`), written to `cadastral-style.qgz` and reopened. | `CadastralImport::prepare`, `CadastralImport::addPrepared`, `CadastralImport::applyStyle`, `LayerOps::isCadastralLayer`, `LayerOps::isReferenceLayer`, `QgsProject::write`, `QgsProject::read` (`src/core/CadastralImport.cpp:66-294`, `src/core/LayerOps.cpp:1183-1188`) | `source.gpkg` (`EPSG:5186`, `{1, "산 23-1", box(200000., 550000., 10., 10.)}`), `scope = box(199900., 549900., 400., 400.)`, `sourceCrs() = "EPSG:5186"`; project CRS `"EPSG:5187"`, memory layer `"Polygon?crs=EPSG:5187"` named `"기존 조사"`; `applyStyle(layer, QColor("#d02030"), false)`; file `"cadastral-style.qgz"`. | `prepared.isReady()`; `layer != nullptr`; `project.mapLayers().size() == 2`; `project.mapLayer(surveyId) == survey`; `project.crs().authid() == "EPSG:5187"`; `isCadastralLayer(layer) == true`; `!isReferenceLayer(layer)`; `cadastralNode->parent() == project.layerTreeRoot()`; not in `"참조 지도"`; no `"지적도"` (`kGroupCadastral`) group; initial fill: `Qt::NoBrush`, `strokeColor() == QColor(Qt::black)`, `strokeWidth() == 0.2`, `Qgis::RenderUnit::Millimeters`, `labelsEnabled() == true`, `fieldName == "JIBUN"`; after `applyStyle(..., "#d02030", false)` & `.qgz` reopen: `isCadastralLayer == true`, `!isReferenceLayer`, `!labelsEnabled()`, `fieldName == "JIBUN"`, root node, not in `"참조 지도"`, no `"지적도"` group, `strokeColor() == QColor("#d02030")`, `strokeWidth() == 0.2`, `Qt::NoBrush`, `surveyId` kept, `reopened.crs() == project.crs()`. | `none` |
| `parsesObservedPortalRowsWithoutCrossPairing` | `tests/test_cadastral.cpp:601-622` | UTF-8 HTML `<ul>` with 5 `<li>` items (3 complete cadastral rows, 1 title-only row, 1 button-only non-cadastral row). | `CadastralPortal::parseResources(html)` (`src/core/CadastralPortal.cpp:157-172`) | 5 `<li>` items: (1) `LSMD_CONT_LDREG_경북_구미시.zip` + `listFnc.download('30563', '64', '39394' );`; (2) `LSMD_CONT_LDREG_서울_강서구.zip` + `listFnc.download('30563', '215', '4649' );`; (3) `LSMD_CONT_LDREG_부산_강서구.zip` + `listFnc.download('30563', '216', '12982' );`; (4) `LSMD_CONT_LDREG_경북_없는군.zip` (no button); (5) `listFnc.download('30563', '999', '1');` (`다른 자료`, no `LSMD_CONT_LDREG_*.zip`). | `parsed.size() == 3`; `"LSMD_CONT_LDREG_경북_구미시.zip"` -> `fileNo == "64"`, `sizeKb == 39394LL`; `"LSMD_CONT_LDREG_서울_강서구.zip"` -> `fileNo == "215"`; `"LSMD_CONT_LDREG_부산_강서구.zip"` -> `fileNo == "216"`. | `crates/strata-net/src/vworld_client_tests.rs:133-150` (`test_vworld_parse_cadastral_resources_reads_zip_rows` — partial: 3 rows, omits non-cadastral `'999'` button row) |
| `selectsExactProvinceAndNestedDistrict` | `tests/test_cadastral.cpp:624-644` | 4 `CadastralPortal::Resource` entries (`서울_강서구`, `부산_강서구`, `경기_수원시_영통구`, `경기_수원시_팔달구`). | `CadastralPortal::selectResources(resources, districts, &error)` (called twice) (`src/core/CadastralPortal.cpp:174-202`) | `resources`: `{"215", "LSMD_CONT_LDREG_서울_강서구.zip", 4649}`, `{"216", "LSMD_CONT_LDREG_부산_강서구.zip", 12982}`, `{"300", "LSMD_CONT_LDREG_경기_수원시_영통구.zip", 100}`, `{"301", "LSMD_CONT_LDREG_경기_수원시_팔달구.zip", 100}`; Call 1: `{{"11500", "서울특별시 강서구"}, {"41117", "경기도 수원시 영통구"}}`; Call 2: `{{"26440", "부산광역시 강서구"}}`. | Call 1: `error.isEmpty() == true`, `selected.size() == 2`, `ids == QSet<QString>({"215", "300"})`; Call 2: `error.isEmpty() == true`, `selected.size() == 1`, `selected.first().fileNo == "216"`. | `crates/strata-net/src/vworld_client_tests_admin.rs:10-35` (`test_vworld_select_cadastral_resources_picks_one_zip_per_district` — partial: single district `"47110"` `"경상북도 경주시"` only; nested `"경기도 수원시 영통구"` currently fails in `vworld_cadastral.rs:184-185`) |
| `refusesMissingAndAmbiguousResources` | `tests/test_cadastral.cpp:646-662` | 1-item `resources` (`경북_구미시`), then 2-item `ambiguous` duplicating `경북_구미시` under `"64"` and `"65"`. | `CadastralPortal::selectResources` (called twice) (`src/core/CadastralPortal.cpp:174-202`) | Call 1: `resources = {{"64", "LSMD_CONT_LDREG_경북_구미시.zip", 39394}}`, `districts = {{"47190", "경상북도 구미시"}, {"11500", "서울특별시 강서구"}}`; Call 2: `ambiguous = {{"64", ..., 39394}, {"65", "LSMD_CONT_LDREG_경북_구미시.zip", 39394}}`, `districts = {{"47190", "경상북도 구미시"}}`. | Both calls: `selected.isEmpty() == true`; `!error.isEmpty() == true` (production formats `"%1 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요."`, `src/core/CadastralPortal.cpp:196`). | `crates/strata-net/src/vworld_client_tests_admin.rs:37-56` (`test_vworld_select_cadastral_resources_picks_one_zip_per_district` — partial: ambiguous case only; omits missing-district case) |
| `selectsSejongMunicipalityInsteadOfProvinceArchive` | `tests/test_cadastral.cpp:664-676` | 2 Sejong `CadastralPortal::Resource` entries (`세종시` `"306"` and `세종` `"312"`). | `CadastralPortal::selectResources(resources, {{"36110", name}}, &error)` (2 iterations) (`src/core/CadastralPortal.cpp:174-202`) | `resources = {{"306", "LSMD_CONT_LDREG_세종시.zip", 100}, {"312", "LSMD_CONT_LDREG_세종.zip", 200}}`; `district.code = "36110"`, `name` in `{"세종특별자치시", "세종특별자치시 세종시"}`. | Both iterations: `error.isEmpty() == true`; `selected.size() == 1`; `selected.first().fileNo == "306"`. | `none` |
| `livePortalDownloadsFiveKilometerSurveyScope` | `tests/test_cadastral.cpp:678-797` | Opt-in live QA harness gated on `KA_CADASTRAL_LIVE_QA` (`QSKIP` when unset) with `box(128.3499, 36.1199, 0.0002, 0.0002)` in `EPSG:4326`. | `CadastralPortal::credentials()`, `VworldSettings::loadApiKey()`, `CadastralPortal::prepare(request, {}, progress)`, `CadastralImport::applyStyle(&layer)` (`src/core/CadastralPortal.cpp:123-127, 204-342`, `src/core/CadastralImport.cpp:244-267`) | Env `"KA_CADASTRAL_LIVE_QA"`; org/app `"ka-hgis"`; `surveyCrs = "EPSG:4326"`, `workCrs = "EPSG:5187"`, `survey = box(128.3499, 36.1199, 0.0002, 0.0002)`, `bufferMeters = 5000.`, `scope.buffer(5000., 24)`; overview `1200x1200` @ `96.` dpi, `Qt::white`, `"cadastral-5km.png"`; detail `halfWidth = 300.` m (fallback parcel `area() < 10000.` m² @ `halfWidth = 150.` m), `"cadastral-jibun-detail.png"`; folder `"원본"`. | `QSKIP` if env empty; `QFileInfo(directory).isAbsolute()`; non-empty `username`, `password`, `apiKey`; `prepared.isReady()`; `layer.isValid()`; `layer.crs() == "EPSG:5187"`; `layer.featureCount() > 0`; `SpatialIndexPresence::Present`; `originalFeatures >= layer.featureCount()`; every feature `intersects(scope)` and `!attribute("JIBUN").toString().trimmed().isEmpty()`; `applyStyle(&layer)`; `cadastral-5km.png` saved & `darkPixels > 100` (`qGray < 200`); `cadastral-jibun-detail.png` saved & `placedLabels > 0`. | `none` |
| `prepareBreakdown_clipDedupFasterThanWkbHash` | `tests/test_cadastral.cpp:799-890` | `grid.gpkg` (`EPSG:5186`) holding a `120 x 80` (`9600`-parcel) `8 m x 8 m` grid spaced `10 m` apart from `(200000., 550000.)`, plus `scope = box(200000., 550000., 800., 400.)`. | `CadastralImport::prepare({source}, scope, sourceCrs(), {}, output.path(), {}, {}, &timing)`, `CadastralImport::applyStyle(&layer)` (`src/core/CadastralImport.cpp:52-63, 66-267`) | `cols = 120`, `rows = 80` (`9600` parcels), `id = r * cols + c + 1`, `jibun = QString::number(id)`, `box(200000. + c * 10., 550000. + r * 10., 8., 8.)` in `"grid.gpkg"` (`EPSG:5186`); `scope = box(200000., 550000., 800., 400.)`; cheap key `"%1:%2:%3:%4:%5:%6:%7"` (`* 1000.0`); render `800x600` @ `96.` dpi in `"EPSG:5186"`. | `newMs * 10 <= oldMs * 7` (`oldMs = qMax<qint64>(1, oldTimer.elapsed())`); `result.isReady() == true`; `timing.written >= 3200 && timing.written <= 3600` (`81 * 41 = 3321` grid squares touch `[200000..200800] x [550000..550400]`); `timing.candidates >= timing.written`; `timing.hashMs >= 0`; `timing.clipMs >= 0`; `timing.indexMs >= 0`; `layer.isValid()`; `layer.featureCount() == timing.written`; `SpatialIndexPresence::Present`; `!job.renderedImage().isNull()`; `font().family() == "Malgun Gothic"`. | `none` |

---

## 2. The Import as the Tests Pin It Down (`CadastralImport` & `MainWindowCadastral`)

### 2.1 Kept Shapefile Fields (Verbatim)

- **Output GeoPackage table name**: `"cadastral"` (`src/core/CadastralImport.cpp:75, 139`; checked at `tests/test_cadastral.cpp:293, 509`).
- **Output attribute columns**:
  1. `"PNU"` (`QMetaType::Type::QString`, column `0`, `src/core/CadastralImport.cpp:136, 191`) — trimmed via `.toString().trimmed()` (`src/core/CadastralImport.cpp:184`). **Optional in source**: looked up via `input.fields().lookupField(QStringLiteral("PNU"))` (`src/core/CadastralImport.cpp:164`); when `< 0`, `id` is `QString()` (`""`).
  2. `"JIBUN"` (`QMetaType::Type::QString`, column `1`, `src/core/CadastralImport.cpp:137, 191`) — copied via `feature.attribute(jibun).toString()` without trimming (`src/core/CadastralImport.cpp:191`). **Required in source**: `input.fields().lookupField(QStringLiteral("JIBUN")) < 0` fails `prepare()` with `Status::Failed` (`src/core/CadastralImport.cpp:163, 165`; tested by `"missing-jibun"` at `tests/test_cadastral.cpp:409, 418, 425-427`).
- **Dropped fields**: `request.setSubsetOfAttributes({jibun, pnu})` (`src/core/CadastralImport.cpp:171-172`) reads no other columns; `importsWholeIntersectingParcels` (`tests/test_cadastral.cpp:52, 61, 299-306`) writes `OWNER = "제외할 개인정보"` into the source and asserts that only `"PNU"`, `"JIBUN"`, and OGR's `"FID"` exist in the output.

### 2.2 Geometry Type Written & Spatial Index

- **Writer options** (`src/core/CadastralImport.cpp:138-141`): `driverName = "GPKG"`, `layerName = "cadastral"`, `fileEncoding = "UTF-8"`, geometry type `Qgis::WkbType::MultiPolygon`, `layerOptions = {"SPATIAL_INDEX=YES"}` (checked via `Qgis::SpatialIndexPresence::Present` at `tests/test_cadastral.cpp:298, 712, 870`).
- **Whole-parcel preservation (no boundary clipping)**: Each intersecting feature is converted via `geometry.convertToMultiType()` (`src/core/CadastralImport.cpp:189`) in `workCrs` and written whole without cutting rings along `scope` (`tests/test_cadastral.cpp:281-284, 316-320`, `difference.area() < 0.01`).

### 2.3 Duplicate Key (`cheapParcelKey`)

Computed on `id` (`PNU` trimmed, or `""`) and `geometry` **already transformed to `workCrs`** (`src/core/CadastralImport.cpp:52-63, 180-188`; mirrored verbatim in `tests/test_cadastral.cpp:833-843`):

```text
"%1:%2:%3:%4:%5:%6:%7"
  %1 = id                                                       (trimmed PNU, or "")
  %2 = qRound64(geometry.boundingBox().xMinimum() * 1000.0)     (xmin in integer mm)
  %3 = qRound64(geometry.boundingBox().yMinimum() * 1000.0)     (ymin in integer mm)
  %4 = qRound64(geometry.boundingBox().xMaximum() * 1000.0)     (xmax in integer mm)
  %5 = qRound64(geometry.boundingBox().yMaximum() * 1000.0)     (ymax in integer mm)
  %6 = qRound64(geometry.area() * 1000.0)                       (area in integer 0.001 m² units)
  %7 = geometry.constGet() ? int(geometry.constGet()->nCoordinates()) : 0
```

- First occurrence is inserted into `QSet<QString> seen` across `ordered` (`sources` deduplicated and lexicographically sorted at `src/core/CadastralImport.cpp:97`); later matches are skipped (`:187-188`).
- Pinned behaviour: deduplicates identical `PNU`+geometry across county files (`overlappingSourcesDeduplicatePnu`, `tests/test_cadastral.cpp:326-344`), deduplicates reversed-ring duplicates while keeping disjoint parts of the same `PNU` (`samePnuKeepsDistinctGeometryAndDeduplicatesEquivalentRings`, `tests/test_cadastral.cpp:378-405`), and runs at least `30%` faster (`newMs * 10 <= oldMs * 7`) than canonical WKB SHA-256 (`prepareBreakdown_clipDedupFasterThanWkbHash`, `tests/test_cadastral.cpp:817-851`).

### 2.4 What Is Skipped and Why

1. **Outside bbox filter** (`src/core/CadastralImport.cpp:170`): `request.setFilterRect(sameCrs ? scope.boundingBox() : toSource.transformBoundingBox(scope.boundingBox()))` — OGR skips features outside the `scope` envelope before `nextFeature`.
2. **Empty geometry** (`src/core/CadastralImport.cpp:178`): `if (geometry.isEmpty()) continue;` — ignored before incrementing `candidates`.
3. **Inside bbox, outside `scope` polygon** (`src/core/CadastralImport.cpp:146-147, 183`): `if (!inside->intersects(geometry.constGet())) continue;` (prepared GEOS engine on `scope`) — drops bbox-corner parcels that do not touch/intersect `scope` (pinned by parcel `3` `"999"` at `box(200080., 550080., 10., 10.)` against the diamond `scope` in `tests/test_cadastral.cpp:271-274, 297, 307`).
4. **Duplicate `cheapParcelKey`** (`src/core/CadastralImport.cpp:187`): `if (seen.contains(key)) continue;` — drops duplicate parcels from adjacent county archives or equivalent rings.
5. **Hard errors (not skipped)**: Invalid/non-polygon layer or invalid source CRS (`:157-159`), missing `"JIBUN"` (`:165`), failed `geometry.transform(toWork)` (`:180-182`), `QgsCsException` (`:197-199`), `writer->addFeature` / `flushBuffer` failure (`:192, 202-204`), and `written == 0` (`:209`) abort with `Status::Failed`.

### 2.5 Source CRS Discovery & Output CRS

- **Source CRS**: Discovered by OGR (`QgsVectorLayer input(source, QString(), QStringLiteral("ogr"))`, `src/core/CadastralImport.cpp:156-157`) from `.prj` / GPKG metadata; exact portal shapefile EPSG code is `not in the source` (test fixtures invent `"EPSG:5186"` at `tests/test_cadastral.cpp:40, 54`). Must satisfy `input.crs().isValid()` and `input.geometryType() == Qgis::GeometryType::Polygon` (`src/core/CadastralImport.cpp:157`; tested by `"missing-crs"` at `tests/test_cadastral.cpp:409, 418`).
- **Output CRS**: `crs` (`workCrs` — project work CRS `"EPSG:5187"` or `"EPSG:5186"`, `src/app/MainWindowCadastral.cpp:90`, `src/core/CadastralPortal.cpp:210-212`, `src/core/CadastralImport.cpp:81, 141, 166-168, 212`; tested for both `"EPSG:5186"` and `"EPSG:5187"` in `tests/test_cadastral.cpp:260-261, 287-296` and rejected when invalid by `"invalid-work-crs"` at `:409, 423`).

### 2.6 Cache Identity, File Names, and Folder Names (Verbatim)

- **Folder hierarchy** (`src/app/MainWindowCadastral.cpp:121`, `src/core/CadastralPortal.cpp:306, 315, 317, 328, 340`, `src/core/TopographicArchive.cpp:170, 179, 238-247`):
  - Cadastral root: `<surveyDir>/지적도`
  - Raw county archives: `<surveyDir>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<name>`
  - Extracted archives (`TopographicArchive::prepare`): `<surveyDir>/지적도/원본/추출/<zipSha256Hex>/payload` (with `<zipSha256Hex>/manifest.json`, lock `<zipSha256Hex>.lock`, staging `.preparing-XXXXXX` holding `payload` and `archives`)
  - Display GeoPackages: `<surveyDir>/지적도/표시`
- **Display GeoPackage `identity` SHA-256 (`src/core/CadastralImport.cpp:87-116`)** — exact byte sequence fed to `QCryptographicHash(QCryptographicHash::Sha256)` (`identityHex = QString::fromLatin1(identity.result().toHex())`):
  1. `"cadastral-scope-v2"` (`18` ASCII bytes, `:90`)
  2. `scope.asWkb()` (`:91`)
  3. `crs.toWkt().toUtf8()` (`:91`)
  4. `transformDocument.toByteArray()` (`QDomDocument` with `<transforms>` written by `context.writeXml(transformElement, QgsReadWriteContext())`, `:92-96`)
  5. For each `source` in `ordered` (`sources` with `removeDuplicates()` then `sort()`, `:97`):
     - Build `files = {source}`; if `QFileInfo(source).suffix()` equals `"shp"` (case-insensitive), check `"dbf"`, `"shx"`, `"prj"`, `"cpg"` in that exact order (`<base>.<ext>`, falling back to `<base>.<EXT>` if lowercase does not exist) and append each existing sibling (`:99-107`).
     - For each `file` in `files`: `identity.addData(file.toUtf8())` then `identity.addData(hash)` where `hash` is the 64-byte lowercase hex SHA-256 of `file`'s bytes (`hashFile`, `:36-47, 108-113`).
- **Display filenames in `directory` (`src/core/CadastralImport.cpp:116-135, 216-232`)**:
  - Output GeoPackage: `<identityHex>.gpkg`
  - Lock file: `<identityHex>.gpkg.lock` (`QLockFile::tryLock(0)`)
  - Digest sidecar: `<identityHex>.gpkg.sha256` (64-byte lowercase hex SHA-256 of `<identityHex>.gpkg`)
  - Temporary staging dir & draft: `.cadastral-XXXXXX/cadastral.gpkg`
- **Cache reuse check (`src/core/CadastralImport.cpp:120-130`, tested at `tests/test_cadastral.cpp:346-376`)**: Reuses `<identityHex>.gpkg` without rewriting when `<identityHex>.gpkg` exists, `<identityHex>.gpkg.sha256` trimmed matches `hashFile(path)`, and `<path>|layername=cadastral` opens valid with `cached.crs() == crs`, `cached.fields().indexOf("JIBUN") >= 0`, and `cached.featureCount() > 0`.

### 2.7 Layer Placement, Styling, and Exact-PNU Focus (`addPrepared`, `applyStyle`, `focusParcel`)

- `addPrepared` (`src/core/CadastralImport.cpp:269-294`, `src/core/LayerOps.cpp:1046-1101`, tested at `tests/test_cadastral.cpp:540-599`): opens `<gpkgPath>|layername=cadastral` as OGR `QgsVectorLayer` named `"지적도 · 조사 주변 5km"`, carries over `ka_hgis/cadastral_color` (default `"#000000"`) and `labelsEnabled()` (default `true`) from any existing layer named `"지적도 · 조사 주변 5km"`, sets `ka_hgis/layer_role = "cadastral"` and `ka_hgis/cadastral = true` (`isCadastralLayer == true`, `isReferenceLayer == false`), clears `ka_hgis/skip_auto_cadastral`, inserts at root index `0` of `layerTreeRoot()` outside `"참조 지도"` (unbundling any legacy `"지적도"` group), and removes superseded `"지적도 · 조사 주변 5km"` layers.
- `applyStyle` (`src/core/CadastralImport.cpp:244-267`, tested at `tests/test_cadastral.cpp:230, 555-596, 735, 874-886`): requires `"JIBUN"` field; sets `QgsFillSymbol` with `style = "no"` (`Qt::NoBrush`), `outline_color = color.name(QColor::HexArgb)` (default `Qt::black`), `outline_width = "0.2"`, `outline_width_unit = "MM"`; sets `QgsVectorLayerSimpleLabeling` on `"JIBUN"` (`isExpression = false`), font `"Malgun Gothic"` `8` pt (`Points`) in `color`, `OverPoint`, `scaleVisibility = true`, `minimumScale = 10000.`, `maximumScale = 0.`, `labelsEnabled = labels`; writes `"ka_hgis/cadastral" = true` and `"ka_hgis/cadastral_color" = color.name(QColor::HexArgb)`.
- `focusParcel` (`src/core/CadastralImport.cpp:296-340`, tested at `tests/test_cadastral.cpp:210-255`): requires regex `"^[0-9]{19}$"`, scans valid vector layers with `"ka_hgis/cadastral" == true` and field `"PNU"`, queries quoted `"PNU" = '<pnu>'`, transforms geometries to canvas destination CRS, sets `marker = geometry.pointOnSurface().asPoint()` from the first matching part, combines all matching parts' `boundingBox()`, enables layer tree visibility recursively, calls `selectByIds(ids)`, scales `bounds.scale(1.4)`, and sets canvas extent without editing the layer.

### 2.8 Every Import & UI Message Verbatim (`CadastralImport.cpp` & `MainWindowCadastral.cpp`)

- **`src/core/CadastralImport.cpp`**:
  - `:82`: `"지적도 원본과 유효한 조사 범위·좌표계가 필요합니다."`
  - `:85`: `"지적도 표시 파일을 저장할 폴더를 만들지 못했습니다."`
  - `:111`: `"지적도 원본 파일을 읽지 못했습니다."`
  - `:118`: `"같은 범위의 지적도를 준비 중입니다."`
  - `:127` (progress `100`): `"저장된 조사 주변 지적도를 사용합니다."`
  - `:133`: `"지적도 임시 파일을 준비하지 못했습니다."`
  - `:143`: `"지적도 표시 파일을 만들지 못했습니다."`
  - `:155` (progress `sourceIndex * 90 / ordered.size()`): `"조사 주변 필지와 지번을 정리하고 있습니다…"`
  - `:158`: `"지적도 도형 또는 원본 좌표계를 확인하지 못했습니다."`
  - `:165`: `"지적도 원본에 지번(JIBUN) 항목이 없습니다."`
  - `:181`: `"필지 좌표를 조사 좌표계로 변환하지 못했습니다."`
  - `:192`: `"지적도 필지를 저장하지 못했습니다."`
  - `:198`: `"지적도와 조사 좌표계 사이의 변환에 실패했습니다."`
  - `:203`: `"지적도 저장을 완료하지 못했습니다."`
  - `:209`: `"조사 주변 범위와 겹치는 필지가 없습니다."`
  - `:213`: `"저장한 지적도 도형 수와 좌표계를 확인하지 못했습니다."`
  - `:219`: `"완료한 지적도를 저장 위치로 옮기지 못했습니다."`
  - `:225`: `"지적도 파일 저장 공간을 확인하세요."`
  - `:228`: `"지적도 파일을 확정하지 못했습니다."`
  - `:234` (session log): `"[cadastral] hash=%1ms clip=%2ms index=%3ms n=%4/%5"`
  - `:240` (progress `100`): `"주변 필지 %1개를 준비했습니다."`
  - `:273, 279` (layer name): `"지적도 · 조사 주변 5km"`
  - `:275`: `"준비한 지적도를 표시하지 못했습니다."`
- **`src/app/MainWindowCadastral.cpp`**:
  - `:36, 40, 41, 50`: `"VWorld 지적도 계정"` / `"아이디"` / `"비밀번호"` / `"이 PC의 개인 설정에 저장하여 다음 다운로드에 사용합니다."` / `"계정을 저장하지 못했습니다"`
  - `:60`: title `"지적도"`, body `"지적도를 먼저 받아 주세요."`
  - `:61, 66, 69, 70, 72`: `"지적도 선 색·지번"` / `"경계선 색"` / `"경계선 0.2mm"` / `"지번 표시"` / `"지번은 1:10,000보다 확대했을 때 표시됩니다."`
  - `:84` (status bar `5000` ms): `"지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요."`
  - `:86`: title `"지적도"`, body `"먼저 조사를 열거나 저장하고 조사구역을 그려 주세요."`
  - `:107-110`: title `"지적도"`, `"조사구역 좌표계를 확인하지 못했습니다."` / `"조사구역 레이어의 좌표계와 작업 좌표계를 맞추지 못했습니다."` / `"조사구역을 다시 그리거나 작업 좌표계를 확인한 뒤 다시 받아 주세요."`
  - `:114`: title `"지적도"`, body `"조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다."`
  - `:122-123`: `"지적도 다운로드"` / `"VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."`
  - `:139` (status bar `5000` ms): `"지적도 받기를 취소했습니다."`
  - `:145-149`: title `"지적도 받기"`, `"지적도를 받지 못했습니다."`, fallback `"아이디나 비밀번호가 맞지 않습니다."`, `"아이디·비밀번호를 다시 입력한 뒤 받기를 누르세요."`, action `"아이디·비밀번호 다시 입력"`
  - `:158-162`: title `"지적도 받기"`, `"지적도를 준비하지 못했습니다."`, fallback `"서버 응답이 없거나 받은 자료를 지도에 올리지 못했습니다."`, `"인터넷 연결과 계정을 확인한 뒤 다시 받아 주세요."`
  - `:168` (status bar `10000` ms): `"조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."`
  - `:170` (job name): `"조사 주변 지적도"`

---

## 3. Encoding Detection (`CadastralEncoding`)

Production implementation: `src/core/CadastralEncoding.h:12-21`, `src/core/CadastralEncoding.cpp:8-65`. Test coverage: `shapefileWithoutCpgKeepsKoreanJibun` (`tests/test_cadastral.cpp:463-522`).

### 3.1 Every Test Case in `shapefileWithoutCpgKeepsKoreanJibun`

| # | Call & `.cpg` state | `.dbf` / sample bytes (verbatim) | Expected encoding / result (verbatim) | Source lines |
| --- | --- | --- | --- | --- |
| 1 | `CadastralEncoding::detectDbfEncoding(sample)` (n/a) | `QByteArray("\xBB\xEA", 2) + QByteArray(" 23-1    ")` (`0xBB 0xEA` = CP949 `"산"`, invalid UTF-8 lead/continuation pair, followed by `" 23-1    "`, `11` bytes total) | `"CP949"` | `tests/test_cadastral.cpp:464-466` |
| 2 | `CadastralEncoding::detectDbfEncoding(sample)` (n/a) | `QStringLiteral("산 23-1   산 24   답 3   대 5").toUtf8()` (`4` valid 3-byte UTF-8 Hangul syllables `"산"`, `"산"`, `"답"`, `"대"` plus ASCII digits/spaces) | `"UTF-8"` | `tests/test_cadastral.cpp:467-469` |
| 3 | `CadastralEncoding::detectDbfEncoding(sample)` (n/a) | `QByteArray("1-2 3-4 5")` (pure ASCII `< 0x80`, `0` multi-byte sequences) | `"CP949"` | `tests/test_cadastral.cpp:470` |
| 4 | `CadastralEncoding::fallbackFor(shp)` + `CadastralImport::prepare({shp}, ...)` with `LSMD_CONT_LDREG_fixture.cpg` **deleted** | `LSMD_CONT_LDREG_fixture.dbf` written by OGR (`PNU = pnu(7)`, `JIBUN = "XX 23-1"`) with `"XX"` replaced in-place by `"\xBB\xEA"` (`CP949` `"산"`) after the dBASE header | `fallbackFor(shp) == "CP949"`; `prepared.isReady() == true`; output `JIBUN == "산 23-1"` | `tests/test_cadastral.cpp:487-515` |
| 5 | `CadastralEncoding::fallbackFor(shp)` with `LSMD_CONT_LDREG_fixture.cpg` holding `"CP949"` | Same `LSMD_CONT_LDREG_fixture.dbf` (`"\xBB\xEA 23-1"`) | `""` (`isEmpty() == true` — `.cpg` honoured by OGR/GDAL without override) | `tests/test_cadastral.cpp:516-520` |
| 6 | `CadastralEncoding::fallbackFor(input.filePath("other.gpkg"))` (non-`.shp` suffix) | None | `""` (`isEmpty() == true`) | `tests/test_cadastral.cpp:521` |

### 3.2 Production Algorithm Rules Underlying the Cases (`src/core/CadastralEncoding.cpp:8-65`)

- `fallbackFor(sourcePath)` (`:50-65`): returns `{}` if suffix is not `"shp"` (case-insensitive), if sibling `.cpg`/`.CPG` exists, if sibling `.dbf`/`.DBF` is missing, or if `.dbf` cannot be opened. Reads the `32`-byte dBASE header (`:59`); returns `"CP949"` if `< 12` bytes or if seeking to `headerLength = h[8] | (h[9] << 8)` fails (`:60-63`); otherwise passes `file.read(32768)` (record bytes only, after the binary header) to `detectDbfEncoding` (`:64`).
- `detectDbfEncoding(sample)` (`:17-48`): scans bytes `>= 0x80` for valid UTF-8 sequences (`2`-byte `0xC2..0xDF`, `3`-byte `(c & 0xF0) == 0xE0`, `4`-byte `0xF0..0xF4`, with continuation bytes `(b & 0xC0) == 0x80`). Any invalid high byte returns `"CP949"` immediately unless `size - i < 4` (truncated sequence at end of the `32768`-byte buffer, `:39-41`). Returns `"UTF-8"` iff `multiByte >= 4`, else `"CP949"` (`:47`).

---

## 4. The Portal (`CadastralPortal`)

Production implementation: `src/core/CadastralPortal.h:5-30`, `src/core/CadastralPortal.cpp:26-352`.

### 4.1 Every Portal Case Covered by `tests/test_cadastral.cpp`

| Case | Test function (`lines`) | Fake server answer (one line) | Expected results & messages, verbatim |
| --- | --- | --- | --- |
| Absolute HTTP deadline on trickling body | `trickleResponseHasAbsoluteDeadline` (`tests/test_cadastral.cpp:121-141`) | In-process `QTcpServer` on `127.0.0.1:<port>/trickle` writes `"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\n"` then `"x"` every `20` ms. | `downloadBytesForTest(url, 200, &error)` returns `""` (`body.isEmpty()`), `timer.elapsed() < 2500` ms, and `error.contains("응답 시간이 초과")` (production sets `"지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요."`, `src/core/CadastralPortal.cpp:67`). |
| Catalogue HTML scraping without cross-pairing `<li>` rows | `parsesObservedPortalRowsWithoutCrossPairing` (`tests/test_cadastral.cpp:601-622`) | No server (in-memory UTF-8 HTML string of 5 `<li>` rows passed directly to `parseResources`). | `parsed.size() == 3`: `"LSMD_CONT_LDREG_경북_구미시.zip"` -> `fileNo = "64"`, `sizeKb = 39394LL`; `"LSMD_CONT_LDREG_서울_강서구.zip"` -> `fileNo = "215"`; `"LSMD_CONT_LDREG_부산_강서구.zip"` -> `fileNo = "216"`. Unlinked `LSMD_CONT_LDREG_경북_없는군.zip` and non-cadastral `'999'` button (`다른 자료`) are both ignored without pairing across `<li>` boundaries. |
| Selecting 1 archive per county (same-name `강서구` + nested `수원시 영통구`) | `selectsExactProvinceAndNestedDistrict` (`tests/test_cadastral.cpp:624-644`) | No server (in-memory `QList<Resource>` passed directly to `selectResources`). | `{{"11500", "서울특별시 강서구"}, {"41117", "경기도 수원시 영통구"}}` -> `error == ""`, `selected.size() == 2`, `fileNo` set `{"215", "300"}`; `{{"26440", "부산광역시 강서구"}}` -> `error == ""`, `selected.size() == 1`, `fileNo == "216"`. |
| Refusing missing (`0` matches) and ambiguous (`2` matches) county archives | `refusesMissingAndAmbiguousResources` (`tests/test_cadastral.cpp:646-662`) | No server (in-memory `QList<Resource>` passed directly to `selectResources`). | Missing `"11500"` `"서울특별시 강서구"` -> `selected.isEmpty()`, `!error.isEmpty()` (`"서울특별시 강서구 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요."`); duplicate `"64"`/`"65"` on `"47190"` `"경상북도 구미시"` -> `selected.isEmpty()`, `!error.isEmpty()` (`"경상북도 구미시 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요."`). |
| Sejong `"36110"` municipality vs. province archive | `selectsSejongMunicipalityInsteadOfProvinceArchive` (`tests/test_cadastral.cpp:664-676`) | No server (in-memory `QList<Resource>` `{{"306", "LSMD_CONT_LDREG_세종시.zip", 100}, {"312", "LSMD_CONT_LDREG_세종.zip", 200}}`). | Both `"세종특별자치시"` and `"세종특별자치시 세종시"` (`code = "36110"`) -> `error == ""`, `selected.size() == 1`, `selected.first().fileNo == "306"`. |
| End-to-end 5 km portal login, WFS, catalogue, ZIP check, unpack, import & render (opt-in live QA) | `livePortalDownloadsFiveKilometerSurveyScope` (`tests/test_cadastral.cpp:678-797`) | None (`QSKIP` when `KA_CADASTRAL_LIVE_QA` is unset; hits live `api.vworld.kr` and `www.vworld.kr` only when explicitly invoked). | `prepared.isReady()`, `layer.crs() == "EPSG:5187"`, `featureCount() > 0`, `SpatialIndexPresence::Present`, `originalFeatures >= layer.featureCount()`, every feature intersects `scope.buffer(5000., 24)` and has non-empty trimmed `"JIBUN"`, `cadastral-5km.png` `darkPixels > 100`, `cadastral-jibun-detail.png` `placedLabels > 0`. |

### 4.2 Portal Paths Without a Fake-Server Unit Test (`not in tests/test_cadastral.cpp`) & All Verbatim Portal Messages

`POST /v4po_usrlogin_a004.do` login, `LT_C_ADSIGG_INFO` WFS paging, and the 4-byte `"PK\003\004"` ZIP header check have **no fake-server unit test** in `tests/test_cadastral.cpp` (exercised in production `src/core/CadastralPortal.cpp` and opt-in `livePortalDownloadsFiveKilometerSurveyScope` only). Every message in `src/core/CadastralPortal.cpp` verbatim:

- **HTTP `Session::request` (`src/core/CadastralPortal.cpp:67-101`)**:
  - `:67`: `"지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요."` (`idleMs = 40000` / `deadlineMs = 40000` for API/HTML; `idleMs = 120000` / `deadlineMs = 1800000` for ZIP file)
  - `:76`: `"지적도 서버 응답이 허용 크기를 넘었습니다."` (`> 32 MiB` API/HTML; `> 512 MiB` ZIP file)
  - `:80`: `"지적도 저장 공간을 확인하세요."`
  - `:87` (progress `15..65`): `"지적도 원본을 받고 있습니다…"`
  - `:97`: `"VWorld 통신에 실패했습니다. 인터넷 연결과 계정 설정을 확인하세요."` (non-`200` HTTP status or `QNetworkReply` error)
  - `:100` (`!prefix.startsWith("PK\003\004")` on file download): `"VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요."`
- **Account INI persistence (`src/core/CadastralPortal.cpp:145, 153`)**:
  - `:145`: `"계정 설정을 기록하지 못했습니다."`
  - `:153`: `"VWorld 계정을 저장하지 못했습니다. 설정 폴더 권한을 확인하세요."`
- **`selectResources` (`src/core/CadastralPortal.cpp:196`)**:
  - `:196`: `"%1 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요."`
- **`prepare` preflight, WFS, login, catalogue, ZIP & archive (`src/core/CadastralPortal.cpp:212-338`)**:
  - `:212`: `"조사구역과 미터 단위 작업 좌표계를 먼저 확인하세요."`
  - `:214`: `"5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요."`
  - `:216`: `"지적도 메뉴에서 VWorld 계정을 설정하세요."`
  - `:224`: `"조사 주변 5km 범위를 만들지 못했습니다."`
  - `:229` (progress `2`): `"조사 주변 5km의 시·군·구를 확인하고 있습니다…"`
  - `:242`: `"VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요."`
  - `:247`: `"시·군·구 응답 좌표계가 요청과 다릅니다."`
  - `:255`: `"시·군·구 경계 응답에 필요한 정보가 없습니다."`
  - `:262`: `"조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다."`
  - `:263`: `"조사 범위를 행정구역 좌표계로 변환하지 못했습니다."`
  - `:264` (progress `8`): `"VWorld 계정을 확인하고 있습니다…"`
  - `:285` (empty `resultMap.result`, `accountRejected = false`): `"VWorld 로그인 응답을 읽지 못했습니다. 누리집의 로그인 방식이 바뀌었을 수 있습니다. 잠시 뒤 다시 시도하고, 계속되면 개발자에게 알려 주세요."`
  - `:287` (`resultMap.result == "error"`, `accountRejected = true`): `"VWorld 로그인에 실패했습니다. 아이디·비밀번호는 VWorld 누리집(vworld.kr) 로그인 계정입니다(API 키가 아님)."`
  - `:289` (other non-`"success"` `resultMap.result`, `accountRejected = true`): `"VWorld 누리집이 로그인 전에 추가 절차를 요구합니다. 인터넷 브라우저로 vworld.kr 에 로그인해 안내를 마친 뒤 다시 시도하세요."`
  - `:290` (appended when stripped `resultMap.msg` is non-empty): `"\n\nVWorld 안내: "` + `reason`
  - `:293` (progress `12`): `"해당 시·군·구의 지적도 파일을 찾고 있습니다…"`
  - `:303`: `"VWorld 자료 목록이 끝나지 않아 내려받기를 중단했습니다."`
  - `:307`: `"조사 폴더에 지적도 저장 위치를 만들지 못했습니다."`
  - `:312` (`sizeKb <= 0 || sizeKb > 512000`): `"해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다."`
  - `:321`: `"지적도 원본 저장 위치를 열지 못했습니다."`
  - `:327`: `"지적도 원본 저장을 완료하지 못했습니다."`
  - `:335`: `"받은 시·군·구 ZIP에 SHP 도형이 없습니다."`
  - `:338` (progress `65`): `"5km 범위의 경계선과 지번을 정리하고 있습니다…"`

---

## 5. Production Constants & Literals Checked by `tests/test_cadastral.cpp`

| Name / role | Value (verbatim) | Production `file:line` | Checked in `tests/test_cadastral.cpp:line` |
| --- | --- | --- | --- |
| `kBenign` substrings (`mapServerMessageIsBenign`) | `{"legend", "Status: 200", "canceled", "cancelled", "aborted"}`, `"범례"`, `"제공영역"` | `src/core/BasemapOps.cpp:1134-1138` | `tests/test_cadastral.cpp:97-118` |
| History-GIS 1919 empty-key error text | `"1919 조선지형도는 국사편찬위원회 역사지리정보DB API 키가 필요합니다. 더보기 → API 키 입력에서 키를 저장하세요. https://hgis.history.go.kr/api/intro.do"` | `src/core/BasemapOps.cpp:1529-1532` | `tests/test_cadastral.cpp:148-152` |
| History-GIS WMTS capabilities endpoint, default layer & CRS | `"https://hgis.history.go.kr/openapi/get.do"`, `"history:map1919"`, `"EPSG:5179"`, `"tileMatrixSet=EPSG:5179"` | `src/core/BasemapOps.cpp:1488-1503` | `tests/test_cadastral.cpp:158-170, 179` |
| History-GIS retry URI count (`historyGisMap1919Uris`) | `4` | `src/core/BasemapOps.cpp:1510` | `tests/test_cadastral.cpp:177-178` |
| History-GIS key redaction & failure text (`historyGisFailureText`) | `"[가림]"`, `"국사편찬위원회 역사지리정보DB 서버가 지도 목록을 보내 주지 않았습니다(4번 시도). 서버 쪽이 잠시 불안정할 때 생기는 일이니 잠시 뒤 다시 눌러 보세요.\n\n"` | `src/core/BasemapOps.cpp:1517-1522` | `tests/test_cadastral.cpp:181-184` |
| Daedongyeojido NGII WMS endpoint, version, CRS & layers | `"https://map.ngii.go.kr/spcemapserver/korea_old_map/ows"`, `"version=1.1.1"`, `"crs=EPSG:5179"`, `"korea_oldmap_ddymap_kyu"`, `"korea_oldmap_addAlphaChannel"` | `src/core/BasemapOps.cpp:1548-1570` | `tests/test_cadastral.cpp:187-201` |
| `cheapParcelKey` format & millimetre/milli-m² multiplier | `"%1:%2:%3:%4:%5:%6:%7"`, `* 1000.0` (`qRound64`) | `src/core/CadastralImport.cpp:55-62` | `tests/test_cadastral.cpp:338, 394, 836-843` |
| Prepared GeoPackage table name (`result.tableName`) | `"cadastral"` | `src/core/CadastralImport.cpp:75, 124, 139, 211, 272` | `tests/test_cadastral.cpp:293, 509` |
| Cache identity version prefix | `"cadastral-scope-v2"` | `src/core/CadastralImport.cpp:90` | `tests/test_cadastral.cpp:354-370` |
| Cache output, lock & digest suffixes | `".gpkg"`, `".lock"`, `".sha256"` | `src/core/CadastralImport.cpp:116-121, 230` | `tests/test_cadastral.cpp:363-368, 458` |
| Staging folder pattern & draft filename | `".cadastral-XXXXXX"`, `"cadastral.gpkg"` | `src/core/CadastralImport.cpp:132, 134` | `tests/test_cadastral.cpp:458` |
| Kept GeoPackage column names, geometry type & spatial index option | `"PNU"`, `"JIBUN"`, `Qgis::WkbType::MultiPolygon`, `"SPATIAL_INDEX=YES"` | `src/core/CadastralImport.cpp:136-141` | `tests/test_cadastral.cpp:298-306, 514, 712, 870` |
| Default style (`applyStyle` defaults & properties) | `color = Qt::black`, `labels = true`, `style = "no"` (`Qt::NoBrush`), `outline_width = "0.2"`, `outline_width_unit = "MM"`, `fieldName = "JIBUN"`, font `"Malgun Gothic"`, size `8` (`Points`), `minimumScale = 10000.`, `maximumScale = 0.` | `src/core/CadastralImport.h:25`, `src/core/CadastralImport.cpp:246-263` | `tests/test_cadastral.cpp:559-565, 594-596, 775-794, 886` |
| Custom layer properties & role (`markCadastralLayer`) | `"ka_hgis/cadastral"` (`true`), `"ka_hgis/cadastral_color"` (default `"#000000"`), `kPropLayerRole = "ka_hgis/layer_role"`, `kRoleCadastral = "cadastral"` | `src/core/CadastralImport.cpp:264-265, 283`, `src/core/LayerOps.h:33, 36`, `src/core/LayerOps.cpp:1048-1049` | `tests/test_cadastral.cpp:230-235, 548-549, 565-572, 580-581, 594` |
| Downloaded layer display title (`downloadedName`) | `"지적도 · 조사 주변 5km"` | `src/core/CadastralImport.cpp:273, 279` | `tests/test_cadastral.cpp:540-542` |
| Legend group names (`kGroupCadastral`, `kGroupReference`) | `"지적도"`, `"참조 지도"` | `src/core/LayerOps.h:30-31` | `tests/test_cadastral.cpp:552-554, 587-589` |
| PNU validation regex & focus extent pad (`focusParcel`) | `"^[0-9]{19}$"`, `bounds.scale(1.4)` | `src/core/CadastralImport.cpp:298, 333` | `tests/test_cadastral.cpp:235-251` |
| DBF encoding labels, header offset, sample size & UTF-8 threshold | `"CP949"`, `"UTF-8"`, `header = 32` bytes (`h[8] \| (h[9] << 8)`), `sample = 32768` bytes, `multiByte >= 4`, trailing cut-off `< 4` bytes | `src/core/CadastralEncoding.cpp:40-47, 59-64` | `tests/test_cadastral.cpp:465-470, 504, 520-521` |
| HTTP timeout message (`failTimeout`) | `"지적도 서버 응답 시간이 초과되었습니다. 다시 시도하세요."` | `src/core/CadastralPortal.cpp:67` | `tests/test_cadastral.cpp:140` |
| Portal dataset ID, archive prefix & regexes (`parseResources`) | `"30563"`, `"LSMD_CONT_LDREG_[^<>\\\"']+\\.zip"`, `"listFnc\\.download\\(\\s*['\"]30563['\"]\\s*,\\s*['\"]([0-9]+)['\"]\\s*,\\s*['\"]([0-9]+)['\"]"`, `"^LSMD_CONT_LDREG_[\\p{L}0-9_ -]+\\.zip$"` | `src/core/CadastralPortal.cpp:28, 159-162` | `tests/test_cadastral.cpp:603-622` |
| Province code prefixes, Sejong `"36110"` & selection error (`selectResources`) | `11`->`"서울"`, `26`->`"부산"`, `36`->`"세종"`, `41`->`"경기"`, `47`->`"경북"`, `code == "36110"` -> `"세종시"`, `"%1 지적도 파일을 정확히 하나로 확인하지 못했습니다. 행정구역 개편 또는 VWorld 자료 목록을 확인하세요."` | `src/core/CadastralPortal.cpp:108-112, 186-196` | `tests/test_cadastral.cpp:624-676` |
| Survey buffer radius, quarter-circle segments & sub-folders (`CadastralPortal::Request` / `prepare`) | `bufferMeters = 5000.`, `24` segments, `"원본"`, `"추출"`, `"표시"` | `src/core/CadastralPortal.h:14`, `src/core/CadastralPortal.cpp:223, 306, 317, 328, 340` | `tests/test_cadastral.cpp:694, 714, 728` |

---

## Could Not Determine

1. **Portal shapefile CRS and extra DBF schema on live VWorld**: `CadastralImport.cpp:156-168` accepts any valid OGR polygon CRS from `.prj` and looks up only `"JIBUN"` and `"PNU"`; the exact EPSG code(s) and extra column names inside live `LSMD_CONT_LDREG_*.zip` archives are `not in the source` (tests synthesize `"EPSG:5186"` and `"OWNER"`).
2. **Live `www.vworld.kr` cookie names and non-`"success"` / non-`"error"` `resultMap.result` codes**: Handled generically via `QNetworkAccessManager`'s cookie jar and the `else` branch at `src/core/CadastralPortal.cpp:265-290`; exact cookie names and site-action codes are `not in the source`.
3. **Live QA parcel counts**: `livePortalDownloadsFiveKilometerSurveyScope` (`tests/test_cadastral.cpp:678-797`) runs only when `KA_CADASTRAL_LIVE_QA` is set and asserts invariants (`featureCount() > 0`, `originalFeatures >= layer.featureCount()`), so exact live feature counts are `not in the source`.

## Counts & Technical Decision

- **Technical decision on Rust equivalence**: Counted a C++ test function as **with a Rust equivalent** only when a Rust test exercises the **same target feature/helper** (even if partial or adapted to Rust's tile/HTML representation); reusable tests on **other** ribbon buttons (`heritage`, `soil`, `strata-topo`, `strata-engine`) are cited inline in Section 1 and tallied separately below so neither strict nor cross-button reuse is obscured.
- **`tests/test_cadastral.cpp` totals**:
  - **Test functions**: `22` (accompanied by `4` QtTest `_data()` slots = `26` `private slots:` methods total; `6` basemap/HTTP helper tests + `16` digital-cadastral tests).
  - **With a Rust equivalent**: `6` (`3` historical/old-map URI & key tests: `historyGis1919_emptyKeyDisablesAddAndLeavesNoLayer`, `historyGis1919_placeholderKeyBuildsOfficialUriWithoutSampleSecret`, `daedongyeojido_buildsOfficialNgiiWmsUriWithoutKey`; `3` partial `vworld_cadastral` portal tests: `parsesObservedPortalRowsWithoutCrossPairing`, `selectsExactProvinceAndNestedDistrict`, `refusesMissingAndAmbiguousResources`; `0` exact 1:1 fixture ports of the `16` digital-cadastral tests).
  - **Without a Rust equivalent**: `16` (`3` helper/HTTP tests + `13` digital-cadastral tests; among those `16`, `6` have reusable building-block tests already ported for other buttons in `strata-refmap`, `strata-engine`, and `strata-topo`).

---

## Citation Verification (`verification-before-completion`)

Re-opened 20 citations picked across the header, Section 1, Section 2, Section 3, Section 4, and Section 5 before writing `CADT0.done`: **20 / 20 were exact on first re-open** (`0` line/value corrections needed):

1. `CMakeLists.txt:1926-1929` — `ka_cadastral_tests` executable, `ka_add_qtest(cadastral ka_cadastral_tests)`, `TIMEOUT 90` (`exact`).
2. `CMakeLists.txt:1987-1994` & `tests/test_save_open.cpp:3089, 3116, 3227` — `save_open_edit_delete` filter and its 3 cadastral slots (`exact`).
3. `crates/strata-geo/src/cadastral_parcel.rs:43-141` — `pub fn analyze_cadastral_inclusion` through closing brace (`exact`).
4. `crates/strata-net/src/vworld_cadastral.rs:183-185` — `sections = stem.split('_')` taking only first two `_`-tokens (`exact`).
5. `tests/test_cadastral.cpp:50-75` — `writeParcels(path, parcels, withJibun = true, validCrs = true)` (`exact`).
6. `tests/test_cadastral.cpp:155-171` — `historyGis1919_placeholderKeyBuildsOfficialUriWithoutSampleSecret` (`exact`).
7. `crates/strata-app/src/ref_tile_history_tests.rs:60-81` — `fn the_1919_map_needs_its_key_and_a_survey_area()` (`exact`).
8. `tests/test_cadastral.cpp:257-262, 263-324` — `importsWholeIntersectingParcels_data` & `importsWholeIntersectingParcels` (`exact`).
9. `crates/strata-refmap/src/heritage_tests.rs:144-167` — `fn the_sites_near_the_survey_area_are_kept_whole()` (`exact`).
10. `tests/test_cadastral.cpp:463-522` — `shapefileWithoutCpgKeepsKoreanJibun` and its 6 encoding assertions at `:464-466`, `:467-469`, `:470`, `:487-515`, `:516-520`, `:521` (`exact`).
11. `crates/strata-net/src/vworld_client_tests_admin.rs:37-56` — ambiguous county archive assertion in `test_vworld_select_cadastral_resources_picks_one_zip_per_district` (`exact`).
12. `tests/test_cadastral.cpp:799-890` — `prepareBreakdown_clipDedupFasterThanWkbHash` (`exact`).
13. `src/core/CadastralImport.cpp:52-63` — `cheapParcelKey` (`"%1:%2:%3:%4:%5:%6:%7"`, `* 1000.0`, `:55-62`) (`exact`).
14. `src/core/CadastralImport.cpp:87-116` — cache `identity` SHA-256 (`"cadastral-scope-v2"`, WKB, WKT, XML, sorted sources & sibling `.dbf/.shx/.prj/.cpg` hex digests) (`exact`).
15. `src/core/CadastralImport.cpp:244-267` — `CadastralImport::applyStyle` (`style="no"`, `"0.2"`, `"MM"`, `"JIBUN"`, `"Malgun Gothic"` `8` pt, `10000.`/`0.`, custom properties) (`exact`).
16. `src/app/MainWindowCadastral.cpp:139, 145-149, 158-162, 168, 170` — completion messages and job title (`exact`).
17. `src/core/CadastralEncoding.cpp:17-48, 50-65` — `utf8Sequence`, `detectDbfEncoding`, and `fallbackFor` (`exact`).
18. `src/core/CadastralPortal.cpp:67-101` — `Session::request` timeout/size/disk/progress/HTTP/ZIP-magic messages (`exact`).
19. `src/core/CadastralPortal.cpp:285-293, 303-338` — login `resultMap`, catalogue, ZIP size, save, unpack, `.shp`, and stage `65` messages (`exact`).
20. `src/core/BasemapOps.cpp:1488-1532, 1548-1570` — History-GIS 1919 and Daedongyeojido URI/retry/redaction/error literals (`exact`).

