# Geology (지질) post-processing rules in the C++ app

Read-only source study of `target/cpp-ref/`. `G` = `src/core/GeologyMapService.cpp`; `O` = `src/app/MainWindowOverlay.cpp`; `P` = `src/core/PreparedReferenceMap.cpp`; `T` = `src/core/ReferenceTiledFetch.cpp`.

## 1. Extent

1. The button "지질" has the tooltip "KIGAM 1:5만 지질 색 위에 지형 음영을 겹칩니다. 다시 누르면 숨깁니다" (`MainWindowRibbon.cpp:501-507`). It is a checkable action with no menu. If a geology layer already exists, the click only toggles visibility and never fetches again (O:495-508).
2. A saved survey file is required. Otherwise a notice with title "지질도" says "먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다." (`MainWindowDownloads.cpp:93-103`, O:509).
3. The survey-area geometry is NOT used. `SurveyScopeClip` is not referenced by the geology code. The extent is the current map-canvas view (O:517).
4. If the canvas scale is above 1:100000, the canvas is zoomed to 1:100000 first (`LayerOps.cpp:1131`, `kThematicMinScaleDenom` = 100000, `LayerOps.h:41`). The status bar shows "축척을 1:100000으로 맞춘 뒤 지질도를 받습니다." for 4000 ms (O:514).
5. The canvas extent is converted to EPSG:5186 if the canvas CRS differs (O:520). On failure the notice (title "지질도") says "화면 범위를 좌표 변환하지 못했습니다."
6. If width or height exceeds `maxSpanMeters()` = 80000 m (`GeologyMapService.h:74`), the notice says "범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요." (O:530-537). `prepare` repeats this check (G:742-749).
7. A smaller view is grown: `LayerOps::expandExtentToMaxSpan` scales it about its centre so the longer side is 80 km (`LayerOps.cpp:1140`, G:740). Aspect ratio and centre are kept. A 100 m view therefore requests an ~80 km box. Test `thematicDownloadCovers1in10000Drawing` confirms this (test_workflow.cpp:1548).
8. The 5186 box is converted to EPSG:4326 with `transformBoundingBox` and the project's transform context (G:751-760). There is no minimum size and no radius.

## 2. Source

9. Service address: `https://data.kigam.re.kr/geoserver/ows` (G:55). No key and no account (header comment, `GeologyMapService.h:23`).
10. Vector request, WFS 2.0.0 GetFeature (G:231-241):
    - `typeNames=<layer>`, `outputFormat=application/json`, `srsName=EPSG:5186`, `count=100000`.
    - `bbox=<latMin>,<lonMin>,<latMax>,<lonMax>,urn:ogc:def:crs:EPSG::4326`, with 8 decimals. Axis order is lat,lon.
    - User-Agent: `Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3`.
11. Layer choice (G:638-657), tested against the 4326 box with `intersects`:
    - Mainland `geoOpen:l_50k_geology_litho_latest` if the box overlaps lon 124.60999365859936–129.58472763044148, lat 33.96870721929159–38.60507408091925.
    - Else Jeju `geoOpen:l_jeju_50k_geology_litho_view` if it overlaps lon 126.10–127.00, lat 33.10–33.62.
    - Else the raster fallback in rule 21.
    - Only one scale (1:50,000) exists. There is no scale choice.
12. Splitting (`T`, `ReferenceTiledFetch.h:32-36`):
    - The whole box is tried first.
    - A TooLarge or Slow piece is split into 4 quarters (NW, NE, SW, SE), at most 2 levels deep (1, 4, 16 pieces).
    - A Transient or Slow failure is retried once per piece.
    - Total budget is 300000 ms. After that no further split or retry happens.
13. A piece is TooLarge if its feature count is ≥ 100000, or if `numberMatched` or `totalFeatures` is larger than the number of features received (P:115-129, G:259-267).
14. Timeouts (`PreparedReferenceMap.h:52`): `kLargeTransfer` is 15000 ms idle and 180000 ms total. The GetCapabilities request uses 15000 ms. The download timeout is mapped to Slow; other failures are mapped to Transient.

## 3. Processing before storage (all in `prepare`, G:734-913)

15. Each piece must start with `{` and be a FeatureCollection. Otherwise it is Fatal with "서버가 GeoJSON 대신 다른 응답을 보냈습니다." (G:253-258). XML error bodies are NOT turned into a raster fallback (test `geologyServerErrorIsNotNoDataFallback`).
16. Merging (`T:102-132`):
    - Features are de-duplicated by `id`, or by the SHA1 of the compact JSON when there is no `id`.
    - The first body's other members, such as `crs`, are kept.
    - `numberMatched`, `numberReturned` and `totalFeatures` are rewritten to the merged count.
17. The merged GeoJSON is written to `geology.geojson` in the temp folder. It is read with OGR. If 0 features come back, the raster fallback is used (G:786).
18. The CRS check (G:795-820):
    - A `crs` member with an unknown name is rejected with "받은 지질도의 좌표계를 확인할 수 없습니다. 기존 지도는 유지됩니다."
    - A file with no `crs` member is read as OGR's WGS84 default.
    - Every geometry is transformed to EPSG:5186 if the source CRS differs. A failed transform or non-finite bbox gives "지질도 좌표를 변환하지 못했습니다. 기존 지도는 유지됩니다."
19. Geometry and attributes:
    - There is NO clipping to the survey or the box and NO validity repair. Whole polygons are kept.
    - `convertToMultiType` is applied. One null or empty geometry aborts everything with "지질도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다."
    - All source fields are copied. String values containing `<` have the regex `<[^>]*>` removed (G:853-857).
    - Field `era_class` (string) is added, computed from field `시대` (G:860-862).
    - Era classification: `eraClass` takes the first match in the `kEras` array order (G:73-96). Matching is by substring, also with spaces removed (Jeju writes "제 4기"). No match gives "시대미상".
    - Keyword to class and RGB:

| Keyword | Class | RGB |
|---|---|---|
| 제4기 | 제4기 | 249,249,127 |
| 네오기, 신제3기 | 네오기(신제3기) | 255,230,25 |
| 팔레오기, 고제3기 | 팔레오기(고제3기) | 253,154,82 |
| 백악기 | 백악기 | 127,198,78 |
| 쥐라기, 쥬라기 | 쥐라기 | 52,178,201 |
| 트라이아스기 | 트라이아스기 | 129,43,146 |
| 페름기 | 페름기 | 240,64,40 |
| 석탄기 | 석탄기 | 103,165,153 |
| 데본기 | 데본기 | 203,140,55 |
| 실루리아기 | 실루리아기 | 179,225,182 |
| 오르도비스기 | 오르도비스기 | 0,146,112 |
| 캄브리아기 | 캄브리아기 | 127,160,86 |
| 신생대 | 신생대 | 242,249,29 |
| 중생대 | 중생대 | 103,197,202 |
| 고생대 | 고생대 | 153,192,141 |
| 원생누대 | 원생누대 | 247,53,99 |
| 시생누대 | 시생누대 | 240,4,127 |
| 선캄브리아 | 선캄브리아시대 | 247,67,112 |

    - Array order (index 0..20) is: 제4기, 네오기, 신제3기, 팔레오기, 고제3기, 백악기, 쥐라기, 쥬라기, 트라이아스기, 페름기, 석탄기, 데본기, 실루리아기, 오르도비스기, 캄브리아기, 신생대, 중생대, 고생대, 원생누대, 시생누대, 선캄브리아. Unmatched text ranks 21.
    - An unknown class color is (200,200,200).
20. Official colors (G:166-223, 839-907):
    - The best point per symbol is the `pointOnSurface` of the symbol's largest polygon (by area, in 5186).
    - Colors are sampled from ONE WMS 1.1.1 GetMap of `geoOpen:L_50K_Geology_Map` over the whole 4326 box: `srs=EPSG:4326`, bbox in lon,lat order, `format=image/png`, `transparent=true`, `styles=` empty.
    - The long image side is 1400 px; the other side is scaled to match, minimum 64.
    - Point to pixel: px = round((x−xmin)/w·(imgW−1)), py = round((ymax−y)/h·(imgH−1)).
    - `pickPixelColor` looks at 13 offsets: (0,0), (±3,0), (0,±3), (±6,0), (0,±6), (−3,−3), (3,3), (−3,3), (3,−3).
    - Pixels with alpha < 200 or lightness < 80 are dropped. The median by lightness is taken, with alpha set to 255.
    - If no color is obtained, the download still succeeds with the warning "공식 지질도 색을 받지 못해 지질시대 기준색으로 표시합니다. 인터넷 연결 후 다시 내려받으면 공식 색을 적용합니다."
21. Raster fallback (`prepareOfficialRaster`, G:292-342):
    - It runs when no layer covers the box or the WFS returned 0 features.
    - It fetches `…/ows?SERVICE=WMS&REQUEST=GetCapabilities`.
    - It rewrites every OnlineResource under GetMap or GetFeatureInfo to `https://data.kigam.re.kr/geoserver/ows?`.
    - It saves the result as `capabilities.xml`.
    - It builds a local WMS URI (G:659-667): `…&crs=EPSG:4326&dpiMode=7&format=image/png&transparent=true&layers=geoOpen:L_50K_Geology_Map&styles&url=<file URL>`.
    - It checks the layer is valid.
    - Warning: "이 범위는 암상 벡터가 없어 공식 지질도 그림을 사용합니다. 지도 표시에는 인터넷 연결이 필요합니다."

## 4. Storage

22. The survey folder is the folder of the survey file (`ReferenceStorage::surveyFolder`). `outGpkg` = `<survey folder>/지질도_KIGAM.gpkg` (O:539).
23. The real file is written inside a new temp folder `<survey folder>/ka-hgis-reference-XXXXXX/` (6 letters or digits) as `지질도_KIGAM.gpkg`. The requested base path itself is never touched (P:17-28).
24. GPKG details: table `geology_map`, UTF-8, written by `writeAsVectorFormatV3` as EPSG:5186 MultiPolygon. It is reopened and the feature count must match, else "저장한 지도를 다시 확인하지 못했습니다. 기존 지도는 유지됩니다. 다른 저장 폴더에서 다시 내려받으세요." (P:131-155).
25. Sidecar files:
    - `geology.geojson` is removed after reading (G:895). In the 0-feature fallback path it is not removed. The return value of the removal is not checked.
    - `capabilities.xml` exists only for the raster fallback.
26. Folder lifetime:
    - The folder is kept only after the layer is added (`retainFiles`). Otherwise it is auto-removed.
    - A replaced folder is deleted at the next survey save (`pruneRetiredReferenceFolders`; `ReferenceStorage.cpp`).
    - Nothing is reused instead of fetching, apart from the toggle in rule 1. A new download is possible only after the old layer is removed.
27. There is no geology data file under `data/`. The only match is `data/icons/icons-mockup.qrc`. All colors, codes and keywords are in the C++ source.

## 5. Display

28. Layer title is "지질도(KIGAM 1:5만)". The source is `<gpkg>|layername=geology_map` via OGR (G:924).
29. Properties and flags:
    - `ka_hgis/layer_role`=`reference`.
    - `ka_hgis/reference_kind`=`geology`.
    - `ka_hgis/crs_label`="EPSG:5186", and abstract "좌표계 EPSG:5186".
    - `ka_hgis/thematic_overlay`=true, with scale-based visibility off and min/max scale 0 (`LayerOps.cpp:1103`).
    - There is no read-only flag and no opacity setting. Not in the source.
30. Legend placement:
    - `placeInLegendGroup(…, "참조 지도")` creates NO group. The name is ignored (`LayerOps.cpp:1021-1038`).
    - It only checks the layer visible, calls `ensureSatelliteAtBottom` and prunes empty groups.
    - Old layers are removed: same title, title + " [", or a tagged geology layer whose source contains `layername=geology_map` or `L_50K_Geology_Map` (G:678-691).
    - Afterwards OTF is enabled, the canvas is synced and `refreshCanvasIfIdle` is called. The project CRS does not change.
31. Style (`applyGeologyStyle`, G:382-458):
    - The style is a categorized renderer on field `기호`.
    - One category per distinct non-empty symbol present in the data. The label is `지층`, or the symbol if `지층` is empty.
    - Sort order is `kEras` array index (younger to older, unmatched last), then symbol text. Note that 네오기 and 신제3기 have different indices (1 and 2).
    - Color is the official sampled color. Otherwise it is the era color made lighter by `lighter(100 + step·12)`, where step = (sum of the UTF-8 bytes of the symbol % 5) − 2.
    - The catch-all category has a null value, color (200,200,200) and label "기타".
    - Fill alpha is 220. Outline is (80,80,80,130) with width 0.12 mm.
    - If the `기호` field is missing the style returns false. The error text is "받은 지도에 필요한 항목이 없습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요."
32. Labels: field `기호`, OverPoint placement with AllowPlacementInsideOfPolygon, not an obstacle. Font Malgun Gothic 8 pt, color (40,40,40). Buffer 0.6 in white (255,255,255,220). Labels are on.
33. Relief layer "지질 음영" (`ensureReliefUnderlay`, G:609-636). Calls are made after every successful add and after every toggle-on. The geology layer itself is set to SourceOver (blend and feature blend).
    - An existing valid "지질 음영" layer is reused. An invalid one is removed and rebuilt.
    - The source is a single-band, non-wms/xyz raster in the project that overlaps the geology extent. A DEM-tagged raster is preferred, otherwise the first one.
    - The renderer is hillshade with band 1, azimuth 315, altitude 45, zFactor 111120 if the CRS is geographic else 3.5, not multidirectional.
    - If there is no DEM, an XYZ layer from `server.arcgisonline.com/ArcGIS/rest/services/Elevation/World_Hillshade/MapServer/tile/{z}/{y}/{x}` is used (`zmin=1`, `zmax=16`, `crs=EPSG:3857`, `tilePixelRatio=1`).
    - The shade is Multiply at opacity 0.48.
    - The shade is placed directly above the geology layer in the layer tree (`stackOver`).
    - It carries `ka_hgis/omit_sheet_legend`=true and the thematic scale range.
    - If it fails, the error is ignored and the geology layer stays.
    - Toggle: turning geology off also hides "지질 음영" (O:505).
34. Raster fallback layer: WMS layer with the same title, CRS 4326, same properties (G:693-725). Tagged `reference_kind` geology. There is no style and no labels.
35. Printed sheet legend (`LayoutService.cpp:1385-1396, 1566-1581`):
    - Categories whose label or value contains 부정합, 주향경사, 지질경계 or 단층추정 are dropped.
    - "지질 음영" and every wms/xyz layer are left out of the sheet legend.
36. Old projects: `restoreThematicOverlayVisibility` upgrades the old scale limit (min = 100001) on tagged layers. It also covers layers whose source matches `geology_map` or contains `data.kigam.re.kr/geoserver` (`LayerOps.cpp:1111-1129`).

## 6. Stops and messages (a failure leaves the old map untouched)

37. Progress window:
    - Title "지질도 내려받기", text "지질도 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.", button "취소", non-modal (`MainWindowDownloads.cpp:22`).
    - A second download gives "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요." for 5000 ms (O:606).
38. Cancel: "지도 내려받기를 취소했습니다. 기존 지도는 유지됩니다." (P:33). The status bar shows "지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다." for 6000 ms.
39. Failure notice: title "지질도 내려받기 실패". Default text: "지도 자료를 받지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요."
40. Messages from `prepare`:
    - Server unreachable or HTTP error: "지도 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요."
    - Timeout: "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요."
    - Whole WFS failure: "KIGAM 서버 연결 실패: %1". If %1 is empty: "지질도를 내려받지 못했습니다."
    - After more than one request: "지도 자료를 %1번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n%2"
    - Over budget: "지도 자료를 받는 시간이 5분을 넘어 멈췄습니다. 기존 지도는 유지됩니다."
    - Incomplete data: "지도 데이터 일부만 받았습니다. 기존 지도는 유지됩니다. 범위를 좁혀 다시 내려받으세요."
    - Bad JSON: "지도 서버가 올바른 지도 데이터를 보내지 않았습니다. 잠시 후 다시 내려받으세요."
    - OGR cannot read the file: "지질도 전체를 읽지 못했습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다."
    - All features dropped: "이 범위에는 지질도 데이터가 없습니다."
    - Other short messages: "지질도 항목을 준비하지 못했습니다. 다시 내려받으세요.", "지질도 일부를 준비하지 못했습니다. 기존 지도는 유지됩니다.", "새 지도 파일을 저장하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요. 기존 지도는 유지됩니다.", "지도를 저장할 임시 폴더를 만들지 못했습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인하세요."
    - Raster fallback: "지질도 서버의 지도 안내를 읽지 못했습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요.", "공식 지질도 연결을 준비하지 못했습니다. 기존 지도는 유지됩니다. 잠시 후 다시 내려받으세요."
41. Display failure: title "지질도 표시 실패".
    - Default text: "받은 지도를 열지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요."
    - Also "저장한 지질도를 여는 데 실패했습니다.", "지질도 레이어를 프로젝트에 넣지 못했습니다.", "공식 5만 지질도 래스터를 열지 못했습니다."
    - Exception during display: "받은 지도를 표시하는 중 오류가 발생했습니다. 현재 작업을 저장하고 레이어 목록을 확인한 뒤 다시 내려받으세요."
    - Worker exception: "지도 자료를 준비하다 오류가 발생했습니다. 기존 지도는 유지됩니다. 저장 공간을 확인한 뒤 다시 시도하세요."
42. Success: status bar "지질도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요." for 10000 ms. Warnings go in a notice titled "지질도 확인 사항". The project is marked dirty (no autosave).
43. On any failure before registration, the temp folder is removed automatically.

## 7. Test assertions (`tests/test_reference_download.cpp`, `test_workflow.cpp`, `test_layer_state.cpp`, `test_reference_status.cpp`)

44. `geologySplitsPartialResponseIntoQuarters`:
    - A whole request returning 1 feature against `numberMatched`=5 triggers 4 quarter requests (5 WFS requests in total).
    - The shared boundary feature `litho.1` is stored once.
    - The result is 5 features in table `geology_map`.
45. `geologyResponseCrsPreservesSurveyLocation`:
    - Exactly 1 GetFeature with `srsName=EPSG:5186` and 1 GetMap.
    - Declared CRS 5186, 5187, `urn:ogc:def:crs:EPSG::4326`, or no `crs` member all give a ready result. The layer CRS is 5186, the project and canvas CRS are unchanged, and the centroid is within 0.01 m.
    - Declared `EPSG:999999`, or a declared 4326 with metric coordinates, fails with no GetMap and an empty `rasterUri`.
46. Raster fallback test: an empty FeatureCollection gives a local `file` URI, `capabilities.xml`, and the htmlMetadata still contains `https://data.kigam.re.kr/geoserver/ows`.
47. Era and legend tests:
    - Specific values: "신생대 제 4기" gives 제4기; "선캄브리아시대" gives 선캄브리아시대; an empty string gives 시대미상; the color of 제4기 is (249,249,127).
    - Legend: 2 units plus "기타" gives 3 categories. Qa comes first (label 충적층, official color (250,244,180) applied). The classification attribute is `기호`.
    - Jeju: `Qbmtb` has label 부면동조면현무암, `Qdtbs` has label 대포동조면현무암 분석구.
    - Region and URI: Jeju box (126.40,33.40,126.60,33.55) uses the Jeju layer; Gangneung (128.80,37.70,128.95,37.80) uses the mainland layer.
    - Relief: Multiply, opacity between 0.34 and 0.71, placed above the geology layer. The geology layer has no "지형맵" fallback and the source has no `refreshAllLayers`.
    - `maxSpanMeters` = 80000 (river 160000, soil 80000).
    - Sheet legend: `omitFromSheetLegend` is true for 부정합경계, 주향경사점, 주향경사, 지질경계 and 단층추정. It is false for 충적층, 백악기퇴적암 and 단층.
48. `test_reference_status.cpp`:
    - A geology GPKG layer in the inventory has kind "참조 지도", is "현장 사용 가능", and has extentText "약 3.0 × 2.0 km".

## Files read in full

`src/core/GeologyMapService.h` and `.cpp`, `PreparedReferenceMap.h` and `.cpp`, `ReferenceTiledFetch.h` and `.cpp`, `ReferenceStorage.cpp`, `ReferenceInventory.cpp`, `ReferenceKind.h`, `SurveyScopeClip.cpp`, `src/app/KaReferenceDownloadJob.cpp`, `src/core/LayerOpsInternal.h` (lines 1-45).

## Read in part

- `src/app/MainWindowOverlay.cpp`: 200-250, 290-695.
- `src/app/MainWindowRibbon.cpp`: 360-540.
- `src/app/MainWindowDownloads.cpp`: 15-120.
- `src/app/MainWindow.cpp`: 1395-1440.
- `src/core/LayerOps.cpp`: 1015-1224.
- `src/core/BasemapOps.cpp`: 2255-2390.
- `src/core/LayoutService.cpp`: 1362-1400, 1565-1584.
- `src/core/LayerRole.cpp`: 55-77.
- `tests/test_reference_download.cpp`: 60-260, 478-640.
- `tests/test_workflow.cpp`: 1045-1365, 1538-1570, 5745-5820.
- `tests/test_layer_state.cpp`: 395-440.
- `tests/test_reference_status.cpp`: 55-105.
- Grep hits only: `test_basemap_reference.cpp`, `test_layer_labels.cpp`, `test_heritage_style.cpp`, `HeritageLayoutNumbers.cpp`.

## Not read

- `BasemapDsm::isDemLayer`.
- `LayerOps::ensureSatelliteAtBottom`, `ensureOtfEnabled`, `syncMapCanvas`, `refreshCanvasNowOrLater`.
- `KaIcons` geology icon.
- `docs/`.
