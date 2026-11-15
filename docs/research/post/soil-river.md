# Post-processing rules of 토양도 (soil) and 수계도 (river) in the C++ app

Read-only source study. All paths are under `%USERPROFILE%/Documents/hgis/target/cpp-ref`. Nothing was edited, run, downloaded or contacted, and no ini or secret file was opened.

Correction to the premise: neither kind uses the survey area. Both use the map canvas view as the extent. "Without a survey area" only means "no saved survey file", and that case is blocked (soil rule 1.4).

## 토양도

### 1. Extent
1. The button is `downloadSoilTerrain` (MainWindowOverlay.cpp:439-488).
2. A click on the button body only toggles visibility of an existing layer. The menu row "분포지형 내려받기 — 현재 화면 범위" re-downloads, because the code checks `qobject_cast<QAction*>(sender())` at :443.
3. `toggleExistingOverlay` (:221-236) shows "토양도(흙토람)를 껐습니다." or "토양도(흙토람)를 다시 켰습니다." (4000 ms).
4. A saved survey file is required. `referenceDownloadFolder` (MainWindowDownloads.cpp:93-103) otherwise shows Notice::Info titled "토양도": "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다." The survey folder is the folder holding the survey file (ReferenceStorage.cpp:39-43).
5. The canvas is first clamped to 1:100000 when it is more zoomed out (`clampCanvasToThematicScale`, LayerOps.cpp:1131; `kThematicMinScaleDenom` = 100000.0, LayerOps.h:41). Status text: "축척을 1:100000으로 맞춘 뒤 토양도를 받습니다." (4000 ms).
6. The extent is `canvas->extent()`, transformed with `transformBoundingBox` to EPSG:5186 if the canvas CRS differs. On failure: Notice::Critical "토양도" "화면 범위를 좌표 변환하지 못했습니다."
7. The limit is `maxSpanMeters()` = 80000 m (SoilMapService.h:62), and it is a reject limit only. If width or height exceeds it, the UI shows Notice::Warning "토양도" "범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요."
8. `prepare` repeats the check (SoilMapService.cpp:257-261): "토양도 범위가 너무 넓습니다. 한 변 80 km 이하로 확대하고 다시 내려받으세요." It also rejects an empty or non-finite extent.
9. The extent is never grown. This is asserted by the tests `soilDownload_usesViewExtentAndTerrainFieldOnly` and `thematicDownloadCovers1in10000Drawing`.
10. The 5186 box becomes a 4326 box in `prepare` (SoilMapService.cpp:262-270). On failure: "지도의 위치를 변환하지 못했습니다. 조사 좌표계를 확인하고 다시 내려받으세요."

### 2. Source
1. The URL is `https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_<n>&srsName=EPSG:5186&count=100000` (SoilMapService.cpp:43, 88-107).
2. The hits request appends `&resultType=hits`. The data request appends `&outputFormat=application/json&propertyName=soil_type_geo,geom`.
3. Both append `&bbox=<latMin>,<lonMin>,<latMax>,<lonMax>,urn:ogc:def:crs:EPSG::4326`, each number with 8 decimals.
4. Tables n = 1, 2, 3 are fetched in order. There is no key, account or paging. The User-Agent is `ka-hgis/0.3` (SoilMapService.cpp:80-84).
5. Timeouts: the hits request uses 15 s idle and 15 s total. The data request uses `kLargeTransfer`, 15 s idle and 180 s total (PreparedReferenceMap.h:52).

### 3. Processing (SoilMapService.cpp:272-348)
1. Per table, the hits body is parsed with the regex `numberMatched\s*="(\d+)"`.
2. 0 hits skips the table.
3. More than 100000 hits stops with: "토양도 데이터가 한 번에 받을 수 있는 양을 넘었습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다."
4. If the hits request fails or the regex does not match, the download goes on with no expected count. There is no retry.
5. The data body must be a FeatureCollection, and `numberMatched` or `totalFeatures` must not exceed the received count (`validateCompleteFeatureCollection`, PreparedReferenceMap.cpp:107-129).
6. The body is written unchanged to `soil-<n>.geojson` and opened with OGR. The part is rejected if it is invalid, has 100000 or more features, or has fewer features than the hits count. Message: "토양도 일부를 완전히 읽지 못했습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다."
7. The merge target is a memory layer `MultiPolygon?crs=EPSG:5186`. Its fields are copied from the first non-empty part. Later parts are matched by field name.
8. There is no reprojection, because the server answers in 5186. There is no clipping, repair, dissolve, dedupe across tables, simplification or renaming.
9. Each geometry goes through `convertToMultiType`. A null or empty geometry, or a failed conversion, fails the whole run: "토양도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요."
10. Zero merged features gives: "이 범위에는 토양도 데이터가 없습니다. 다른 위치를 확인하세요. 기존 지도는 유지됩니다."
11. Other messages: "토양도 항목을 준비하지 못했습니다. 다시 내려받으세요." and "토양도 일부를 합치지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요."
12. `saveVector` writes the GPKG with the GPKG driver, UTF-8, table `soil_map`, then reopens it and compares the feature count (PreparedReferenceMap.cpp:131-155).
13. Save-failure messages: "새 지도 파일을 저장하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요. 기존 지도는 유지됩니다." and "저장한 지도를 다시 확인하지 못했습니다. 기존 지도는 유지됩니다. 다른 저장 폴더에서 다시 내려받으세요."

### 4. Storage
1. `initializeStorage` creates `<survey folder>/ka-hgis-reference-XXXXXX/` as a QTemporaryDir (PreparedReferenceMap.cpp:17-28).
2. Inside it are `토양도_흙토람.gpkg` (requested name from MainWindowOverlay.cpp:481, table `soil_map`) and `soil-1..3.geojson`.
3. The requested path itself is never written. The test `preparedFilesAreOwnedUntilAccepted` asserts that the generated file is removed when the result is destroyed and that `base` does not exist.
4. `retainFiles()` is called only after `addMapLayer` succeeds. On any earlier failure or cancel, the folder is deleted when the result is destroyed. The test `partialSoilFailurePreservesOriginal` asserts that an existing file at `base` stays byte-identical.
5. An earlier download folder is deleted only at a save point, and only if this session created it, a newer download replaced it, and nothing references it (ReferenceStorage.cpp:77-112; MainWindowOverlay.cpp:662-665; MainWindowDownloads.cpp:105-116).
6. An existing file is never reused. The only reuse is the toggle of the layer already in the project.

### 5. Display (addPrepared, SoilMapService.cpp:351-431)
1. Layer title is "토양도(흙토람)". Source is `<gpkg>|layername=soil_map`.
2. Old layers are removed only after the new one is added. A layer is removed if its name equals "토양도(흙토람)" or "토양도(흙토람 그림)", or either name followed by " [", or if it is tagged kind soil with a source containing `layername=soil_map`. An imported soil SHP is not replaced.
3. The style requires the field `soil_type_geo`. Otherwise: "받은 지도에 필요한 항목이 없습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요."
4. The renderer is categorized on `soil_type_geo` (SoilMapService.cpp:54-66, 152-171). Colours as R,G,B:

| Code | Label | R,G,B |
|---|---|---|
| 01 | 산악지 | 1,178,0 |
| 02 | 구릉지 | 177,210,93 |
| 03 | 산록경사지 | 139,68,108 |
| 04 | 곡간지/선상지 | 254,244,182 |
| 05 | 해성평탄지 | 132,186,230 |
| 06 | 하성평탄지 | 192,239,245 |
| 07 | 고원지 | 78,78,78 |
| 08 | 홍적대지 | 254,160,0 |
| 09 | 용암류대지 | 192,151,155 |
| 10 | 용암류평탄 | 194,179,182 |
| 99 | 기타 | 255,255,255 |

5. A null-value catch-all labelled "미분류" is added with colour 200,200,200. That makes 12 categories.
6. Each symbol has fill alpha 175 and outline 40,44,52 at alpha 180. Outline width is 0.28 MM.
7. Labels use the expression `CASE WHEN area($geometry) < 40000.0 THEN '' WHEN "soil_type_geo"='01' THEN '산악지' …`. Label text per code:
   - 01 산악지, 02 구릉지, 03 산록경사
   - 04 곡간·선상, 05 해성평탄, 06 하성평탄
   - 07 고원지, 08 홍적대지, 09 용암대지, 10 용암평탄, 99 기타
   - `ELSE ''`
8. `terrainLabelExpression(minArea, candidatesOnly)` is at SoilMapService.cpp:173-198. With `candidatesOnly`, only codes 04/05/06/08 are labelled.
9. Label format: Malgun Gothic 8 pt bold, colour 28,32,38, white buffer 0.7 at alpha 235, OverPoint placement, `fitInPolygonOnly`, obstacle on. Labels show only at scale 1:40000 or closer (`scaleVisibility`, min 40000, max 0). The layer display expression is the same expression.
10. Custom properties on the vector layer:
    - `ka_hgis/layer_role=reference` (LayerOps.cpp:1052)
    - `ka_hgis/reference_kind=soil` (ReferenceKind.h:25, 39)
    - `ka_hgis/crs_label=EPSG:5186`, with the abstract "좌표계 EPSG:5186" (LayerOps.cpp:1171)
    - `ka_hgis/thematic_overlay=true`, with scale-based visibility off and min/max scale 0 (LayerOps.cpp:1103)
11. `placeInLegendGroup(…, "참조 지도")` does NOT create or move into a group (LayerOps.cpp:1021-1038). The comment there says the list is flat. The call only checks the row visible, runs `ensureSatelliteAtBottom` and prunes empty groups. The reference/survey split is the `layer_role` property.
12. A second layer, the picture, is then added (SoilMapService.cpp:398-419):
    - Title is "토양도(흙토람 그림)".
    - Provider is wms with the URI from `terrainPictureUri()` (:109-113): `type=xyz&url=https://gis.naas.go.kr/Geodata/SF/TOP_A_SOIL_T_GEO/Layers/_alllayers/L{z}/R{y}/C{x}.png&zmax=15&zmin=6&crs=EPSG:3857&tilePixelRatio=1`. The braces are %7B/%7D encoded.
    - It is inserted in the root directly below the vector layer (index + 1).
    - It carries `layer_role=reference`, `ka_hgis/omit_sheet_legend=true`, the legend flag ExcludeByDefault, the CRS label EPSG:3857, and the thematic scale properties.
    - It gets no reference_kind tag. If it is invalid it is silently dropped, with no message.
13. `rewriteArcGisCacheUrl` (SoilMapService.cpp:115-138) is applied to every request by the network preprocessor (BasemapOps.cpp:113-124; also KaApplication.cpp:839, not read). It rewrites `/L<z>/R<y>/C<x>.png` on host gis.naas.go.kr, path containing `/TOP_A_SOIL_T_GEO/`, to `/L%02d/R%08x/C%08x.png` (z decimal, y and x lowercase hex).
14. Then `ensureOtfEnabled` (project CRS, or EPSG:5186 if invalid), `syncMapCanvas(…, false)`, and `refreshCanvasIfIdle`.

### 6. Stops and messages
- A download already running: "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요." (5000 ms).
- Progress dialog (MainWindowDownloads.cpp:22-33):
  - label "토양도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다."
  - button "취소"
  - window title "토양도 내려받기"
  - non-modal
- Cancel: "토양도 내려받기를 취소했습니다. 기존 지도는 유지됩니다." (6000 ms). The inner text is "지도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."
- Failure: Warning titled "토양도 내려받기 실패" with the error, or the default "지도 자료를 받지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요."
- Network errors from `download`:
  - timeout: "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요."
  - other: "지도 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요."
  - bad body: "지도 서버가 올바른 지도 데이터를 보내지 않았습니다. 잠시 후 다시 내려받으세요."
  - partial body: "지도 데이터 일부만 받았습니다. 기존 지도는 유지됩니다. 범위를 좁혀 다시 내려받으세요."
- Layer-registration failures:
  - "저장한 토양도를 여는 데 실패했습니다."
  - "토양도 레이어를 프로젝트에 넣지 못했습니다."
  - UI title "토양도 표시 실패" with the default "받은 지도를 열지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요."
- Success: "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요." (10000 ms). The project is set dirty and the button is checked.
- Soil has no retry and no splitting. A result from a different survey generation is dropped (MainWindowOverlay.cpp:633).

### 7. Tests
- `soilDownload_usesViewExtentAndTerrainFieldOnly` (test_workflow.cpp:1573): 6 requests (3 hits and 3 data), bbox equals the transformed view, `propertyName=soil_type_geo,geom`.
- `soilTerrainLegend_officialCodesAndStyle` (:1014): unknown code "77" gives "미분류" and colour 200,200,200 per `terrainName`/`terrainColor`. 12 categories. A layer without the field fails.
- `soilTerrainPicture_is3857ArcGisCacheNot5186Wms` (:1640): `/L13/R3167/C7003.png` becomes `/L13/R00000c5f/C00001b5b.png`, and `/L8/R98/C218.png` becomes `/L08/R00000062/C000000da.png`. A non-naas host is left unchanged.
- `thematicDownloadMaxSpanFourTimesPrior` (:1541): the span limits 80000 / 80000 / 160000.
- Sheet legend (:5688, :5820): the picture is omitted from the legend, the class names (산악지 …) are shown, and an unchecked soil layer is absent from the legend.

### Related paths not studied
- The menu row "토양도 파일 불러오기(SHP·GPKG)" calls `importSoilShapefile` (MainWindow.cpp:2643); not read.
- 고지형 (`startPaleoLandform`, MainWindowOverlay.cpp:343) reuses the soil download as `ReferenceMapKind::PaleoSoil` with its own wording; I read only the start of it.

## 수계도

### 1. Extent
1. `downloadRiverMap` (MainWindowOverlay.cpp:550-601): toggle if the layer exists, otherwise the folder check. The messages are the same as soil with title "수계도". The unsaved-survey case also unchecks the action.
2. Clamp to 1:100000: "축척을 1:100000으로 맞춘 뒤 수계도를 받습니다."
3. A key is required via `VworldSettings::loadApiKey()`. If empty: Notice::Warning "수계도" "VWorld 인증키가 없습니다. 배경지도 설정에서 키를 먼저 등록하세요." `prepare` has its own wording (RiverMapService.cpp:278-282): "VWorld 인증키가 없습니다. 지도 탭의 배경지도 설정에서 키를 먼저 등록하세요."
4. The lookup order is quoted from the repo's CLAUDE.md, not verified in VworldSettings.cpp (not read): `%LOCALAPPDATA%\ka-hgis\ka-hgis-vworld.ini`, then `HKCU\Software\ka-hgis`, then env `VWORLD_API_KEY`, then `config/secrets.ini`.
5. The extent is the canvas extent in 5186. The limit is 160000 m (RiverMapService.h:43). The UI message is "범위가 너무 넓습니다. 지도를 조사구역(한 변 160km 이하)으로 확대한 뒤 다시 내려받으세요."
6. `prepare` then grows the extent with `expandExtentToMaxSpan` (LayerOps.cpp:1140-1152). The box is scaled about its centre so the longer side equals 160 km, keeping the aspect ratio. This is asserted by `thematicDownloadCovers1in10000Drawing`. `prepare` re-checks and rejects with "범위가 너무 넓습니다. 지도를 조사구역(한 변 160km 이하)으로 확대한 뒤 다시 내려받으세요."
7. The box is converted to 4326. Failure: "좌표 변환에 실패했습니다."

### 2. Source
1. URL: `https://api.vworld.kr/req/wfs?SERVICE=WFS&REQUEST=GetFeature&VERSION=2.0.0&TYPENAME=lt_c_wkmstrm&OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES=1000&STARTINDEX=<page*1000>&BBOX=<latMin>,<lonMin>,<latMax>,<lonMax>,urn:ogc:def:crs:EPSG::4326&KEY=<KEY>` (RiverMapService.cpp:39-40, 104-119).
2. DOMAIN is deliberately not sent. The comment at :108-110 says a DOMAIN=localhost makes VWorld reject the key.
3. Headers: User-Agent `Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3`, and `Referer: https://localhost`.
4. The global preprocessor (BasemapOps.cpp:113-124) may overwrite User-Agent and Referer. Whether it applies to these blocking requests, and what `BasemapPolicy::refererForUrl` returns (file not read), is not verified.
5. Paging: up to 10 pages of 1000 per piece. A page with fewer than 1000 features ends the piece (:155).
6. Each page request uses `kLargeTransfer` (15 s idle, 180 s total).

### 3. Processing
1. `ReferenceTiledFetch::fetch` (ReferenceTiledFetch.cpp:134-158) runs over the 4326 box. Options (ReferenceTiledFetch.h:32-36): maxDepth 2 (whole, then 2x2, then 4x4), retriesPerPiece 1, budget 300000 ms.
2. Outcome per piece:
   - Page-limit overflow, or an incomplete count (`numberMatched`/`totalFeatures` greater than received), is `TooLarge`. The piece is split into quarters (NW, NE, SW, SE in 4326) and never repeated.
   - A total-timeout while data was arriving is `Slow`. It is split, or retried at the finest level.
   - A dropped or silent connection is `Transient`. It is retried once.
   - Cancel stops the fetch.
   - Anything else is `Fatal` (a non-JSON body, a ServiceException, a body that is not a FeatureCollection).
3. After the budget of 300000 ms is spent (checked once at least one request was made) there is no more splitting or retry. Message: "지도 자료를 받는 시간이 5분을 넘어 멈췄습니다. 기존 지도는 유지됩니다."
4. If the fetch needed more than one request and still failed, the message becomes "지도 자료를 %1번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n%2".
5. `prepare` wraps a non-empty error as "VWorld 서버 연결 실패: %1". An empty error becomes "수계도를 내려받지 못했습니다."
6. Pieces are merged by `mergeFeatureCollections` (ReferenceTiledFetch.cpp:102-132):
   - A feature is skipped if its key was already seen. The key is the string or number `id` (`id:<id>`), or `json:<SHA1 of the compact JSON>` when there is no id.
   - The first body's other members (crs etc.) are kept. `numberMatched`, `numberReturned` and `totalFeatures` are rewritten to the merged count.
   - A single piece is passed through unchanged.
7. A non-JSON page gives (RiverMapService.cpp:134-144):
   - with no ServiceException text: "서버가 GeoJSON 대신 다른 응답을 보냈습니다. VWorld 인증키를 확인하세요."
   - with ServiceException text containing "KEY" (case-insensitive): "VWorld 인증키가 거부되었습니다. 배경지도 설정에서 키를 확인한 뒤 다시 내려받으세요."
   - with any other ServiceException text: "VWorld가 하천 지도를 보내지 못했습니다. 인터넷 연결을 확인한 뒤 잠시 후 다시 내려받으세요."
8. The page-limit message at the last page is: "하천 데이터가 한 번에 받을 수 있는 양을 넘었습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다."
9. The merged body is written as `river.geojson` and opened with OGR.
10. An invalid layer or 0 features removes the file and gives: "이 범위에는 하천망 데이터가 없습니다. (국가·지방하천이 없는 지역입니다)"
11. The merge target is a memory layer `MultiPolygon?crs=EPSG:5186` with all source fields copied (RiverMapService.cpp:324-377). Each geometry is transformed 4326 to 5186 with `g.transform`. Failure: "하천 좌표를 변환하지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요."
12. Then `convertToMultiType`. A null or empty geometry, or a failed conversion, fails the whole run: "하천 지도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다."
13. There is no clipping, repair, dissolve, attribute filtering, renaming or simplification.
14. Other messages: "하천 지도 항목을 준비하지 못했습니다. 다시 내려받으세요.", "이 범위에는 하천망 데이터가 없습니다.", "하천 지도 일부를 준비하지 못했습니다. 기존 지도는 유지됩니다."
15. `saveVector` as for soil, table `river_map`. `river.geojson` is removed after the copy.

### 4. Storage
- The same temporary-folder, retain and ledger rules as soil apply. The file is `<survey folder>/ka-hgis-reference-XXXXXX/수계도_VWorld.gpkg` (requested name at MainWindowOverlay.cpp:594, table `river_map`).
- A successful download leaves only the GPKG, because the geojson is deleted.
- There is no file reuse. The test `cancelledPreparationCanRetry` asserts that after a cancel the retry succeeds, the retained file exists, and `base` was never created.

### 5. Display (applyRiverStyle RiverMapService.cpp:192-261; addPrepared :381-431)
1. Title is "수계도(하천망)". Source is `<gpkg>|layername=river_map`.
2. Replacement is as soil (title, title followed by " [", or tag river plus `layername=river_map`), except that no picture layer exists. Old layers are removed only after the new one is added. The test `failedRegistrationPreservesOldLayer` asserts the old layer stays when registration fails.
3. The style requires at least one of `riv_level` or `riv_nm`. Otherwise: "받은 지도에 필요한 항목이 없습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요."
4. The renderer is categorized on `riv_level`. Only levels present in the data (non-empty) get a row. Sort is by rank, then alphabetical. The category value and label are the raw `riv_level` string.
5. Rank by substring (RiverMapService.cpp:51-64):

| Substring in `riv_level` | Rank | R,G,B |
|---|---|---|
| 국가 | 0 | 0,126,212 |
| 지방1급 | 1 | 16,146,222 |
| 지방2급 | 2 | 40,166,232 |
| anything else | 3 | 120,196,236 |

6. A null-value catch-all "기타 수계" is appended with colour 120,196,236.
7. The `label` strings in the table ("국가하천" etc.) are never used by the code.
8. Each symbol has fill alpha 230 and outline `base.darker(150)` at alpha 255. Outline width is 0.32 MM.
9. Labels (only if `riv_nm` exists) use the field `riv_nm` directly, not an expression:
   - Malgun Gothic 8 pt bold, colour 0,72,186
   - white buffer 0.7 at alpha 225
   - OverPoint placement, polygon placement inside allowed
   - obstacle off
   - no scale rule and no area rule
10. Custom properties are the same as the soil vector layer, with `reference_kind=river`.
11. There is no picture layer.

### 6. Stops and messages
- Progress, cancel, failure and success texts are as soil with title "수계도". The success text is "수계도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요."
- Retry rules are in processing rules 1-3. A manual retry always starts from scratch.
- Registration failures: "저장한 수계도를 여는 데 실패했습니다." and "수계도 레이어를 프로젝트에 넣지 못했습니다."

### 7. Tests
- `riverPageLimitIsFailure` (test_reference_download.cpp:149): every page full gives exactly 30 requests (whole, quarter, leaf, 10 pages each), status Failed, error contains "양을 넘었습니다".
- `riverRetriesDroppedPieceOnce` (:173): the first call fails, the second succeeds, 2 calls in total.
- `cancelledPreparationCanRetry` (:357): after cancel, status is Cancelled and 1 call was made.
- `riverLevelLegend_waterStyleAndNameLabels` (test_workflow.cpp:1501):
  - 4 categories, ordered 국가하천, 지방1급하천, 지방2급하천 (then 기타 수계)
  - the first has RGB 0,126,212 (ignoring alpha)
  - blue exceeds red by more than 150
  - labels are enabled
  - a layer without the fields fails
- Test data in `riverData()`: `riv_nm` "시험하천", `riv_level` "국가하천", Polygon at lon 127 / lat 37.

### Ribbon
- Soil button (MainWindowRibbon.cpp:437-461):
  - Text "토양", object name `btnSoil`, icon "soil", checkable, menu-button popup.
  - Tooltip: "흙토람 토양도를 겹칩니다. 다시 누르면 숨깁니다. 우클릭: 상세 메뉴".
  - Menu rows: "분포지형 내려받기 — 현재 화면 범위" and "토양도 파일 불러오기(SHP·GPKG)". Right-click opens the same menu.
- River action (:508-520):
  - Text "수계", icon "river", tooltip "하천망을 겹칩니다. 다시 누르면 숨깁니다", checkable, no menu.
- `syncThematicButtons` (MainWindow.cpp:1398-1423) follows legend visibility, looking layers up by the titles "토양도(흙토람)" and "수계도(하천망)".
- Other soil menu rows I saw but did not read (MainWindowRibbon.cpp:639, 697, 705): "토양도 SHP 불러오기", a web link "농진청 토양도" to `http://soil.rda.go.kr/geoweb/soilmain.do`, and "토양도 SHP 불러오기(흙토람 다운로드)…".

## Files

**Read in full:**
- src/core/SoilMapService.cpp/.h
- src/core/RiverMapService.cpp/.h
- src/core/PreparedReferenceMap.cpp/.h
- src/core/ReferenceTiledFetch.cpp/.h
- src/core/ReferenceStorage.cpp/.h
- src/core/ReferenceKind.h
- src/core/ReferenceInventory.cpp
- src/core/SurveyScopeClip.cpp
- src/app/KaReferenceDownloadJob.cpp

**Read in part:**
- src/app/MainWindowOverlay.cpp: 215-245, 340-695
- src/app/MainWindowRibbon.cpp: 430-530
- src/app/MainWindowDownloads.cpp: 15-120
- src/app/MainWindow.cpp: 1395-1435
- src/core/LayerOps.cpp: 1015-1215
- src/core/BasemapOps.cpp: 105-145, 340-400
- src/core/LayerOps.h and LayerOpsInternal.h: grep context only
- tests/test_reference_download.cpp: 60-210, 325-395
- tests/test_workflow.cpp: 1014-1045, 1501-1667, 5688-5745, 5820-5860

**Not relevant:** SurveyScopeClip.cpp is not used by soil or river. It is a 5 km "조사 주변" clip used elsewhere.

**Not read, but relevant:**
- `VworldSettings::loadApiKey` (src/core/VworldSettings.cpp:111)
- `BasemapPolicy::refererForUrl` and `offlineCaching`
- `KaApplication.cpp:839` (second URL-rewrite preprocessor)
- `LayerOps::isLayerVisible` and `toggleLayerVisibility` (BasemapOps.cpp:2331-2356; title matching)
- `ensureOtfEnabled`, `syncMapCanvas`, `refreshCanvasIfIdle`
- `pruneEmptyLegendGroups`
- `importSoilShapefile` (MainWindow.cpp:2643) and test `soilShpImport_crsOverrideAndCategorizedStyle` (test_workflow.cpp:896)
- `PaleoLandformService`
- The sheet-legend code for the river layer (LayoutService.cpp ~1389, 1579)
- tests/test_layer_list_chrome.cpp (mentions a river node), test_layer_roles.cpp, test_layer_state.cpp, test_save_open.cpp

I ran no tests; the test values above come from reading the test source.
