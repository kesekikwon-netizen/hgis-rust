# C++ post-processing rules: 고지형 and 1919 조선지형도

READ-ONLY source study. Root R = `%USERPROFILE%/Documents/hgis/target/cpp-ref`. Paths below are relative to R. No key or ini file was opened.

HEADLINE: the 고지형 button downloads NO paleo-landform dataset. It downloads the 흙토람 soil map (a different dataset), recolours it, and derives hypothesis polygons from soil terrain codes. 1919 is a live WMTS layer with no local post-processing.

## 1. 고지형 (ribbon btnPaleo, text "고지형")

Tooltip (src/app/MainWindowRibbon.cpp:467): "흙토람 입지 후보를 강조하고 구하도·자연제방 등 가설을 그립니다. 확정이 아닙니다". Button is NOT checkable; clicking again re-runs the same flow (no toggle). Handler MainWindow::startPaleoLandform, src/app/MainWindowOverlay.cpp:343-435.

1. Guards (Overlay:345-355). Ignored while opening/closing a survey. No survey file (m_surveyPath empty): box 「고지형」: "먼저 「새 조사」로 저장 위치를 만드세요.\n그다음 조사구역으로 확대한 뒤 다시 「고지형」을 누르면 흙토람 분포지형이 깔립니다." Then LayerOps::clampCanvasToThematicScale (LayerOps.cpp:1131): if canvas scale > 1:100000 (kThematicMinScaleDenom, LayerOps.h:41) zoomScale(100000, true).

2. Extent. No survey-area geometry and no radius is used anywhere (survey 100/5/10 km radii: not in the source). Extent = current canvas extent, transformBoundingBox to EPSG:5186 if the canvas CRS differs (Overlay:359-371; failure box: "화면 범위를 좌표 변환하지 못했습니다." / hint "작업 좌표계를 확인한 뒤 다시 누르세요."). Reject if width or height > SoilMapService::maxSpanMeters() = 80000 m (SoilMapService.h:62): "지금 화면이 너무 넓습니다. 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 「고지형」을 누르세요.\n전국·시도 화면에는 분포지형을 깔지 않습니다." prepare() repeats the check (SoilMapService.cpp:257): "토양도 범위가 너무 넓습니다. 한 변 80 km 이하로 확대하고 다시 내려받으세요." Extent is NOT grown to 80 km (test_workflow.cpp:1548-1571, 1573-1619 pin bbox = view).

3. Reuse. If ANY valid vector layer with field soil_type_geo exists (PaleoLandformService::findSoilTerrainLayer, .cpp:259; includes an imported soil SHP, first in project map order) nothing is downloaded, even if the view is outside its extent; go to step 8. Else download (step 4).

4. Source (SoilMapService.cpp:43-107). WFS 2.0.0 GET `https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_{1,2,3}&srsName=EPSG:5186&count=100000` + for data: `&outputFormat=application/json&propertyName=soil_type_geo,geom`; for count: `&resultType=hits`; + `&bbox=<latMin>,<lonMin>,<latMax>,<lonMax>,urn:ogc:def:crs:EPSG::4326` (8 decimals; lat first). No key, no account. Header UserAgent "ka-hgis/0.3" (soilWfsRequest); a global preprocessor (BasemapOps.cpp:113-124, KaApplication.cpp:831-842) also sets UA "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"; whether it overrides the first for blocking requests is not decidable from source. Referer: none (refererForUrl only returns one for VWorld, BasemapPolicy.h:129). Runs as a background QgsTask with a cancel-able progress dialog titled "토양도" (Overlay:603-612, KaReferenceDownloadJob.cpp).

5. Processing (SoilMapService.cpp:250-349, PreparedReferenceMap.cpp). Per table 1..3: hits request (15 s idle/total); numberMatched parsed by regex `numberMatched\s*="(\d+)"`; 0 -> skip table; >100000 -> "토양도 데이터가 한 번에 받을 수 있는 양을 넘었습니다. 범위를 좁혀 다시 내려받으세요. 기존 지도는 유지됩니다." Data request: idle 15 s, total 180 s (kLargeTransfer). Body must be a GeoJSON FeatureCollection and numberMatched/totalFeatures must not exceed received count (else "지도 데이터 일부만 받았습니다. 기존 지도는 유지됩니다. 범위를 좁혀 다시 내려받으세요."). Written to soil-N.geojson, reopened by OGR; rejected if featureCount >= 100000 or < hits ("토양도 일부를 완전히 읽지 못했습니다. ..."). Features merged into one memory MultiPolygon EPSG:5186 layer (fields of the first non-empty table, attributes copied by name, geometry convertToMultiType; null/empty geometry aborts: "토양도에 읽을 수 없는 구역이 있습니다. ..."). No clipping, no reprojection (server already returns 5186), no dedupe, no filtering. Zero features total: "이 범위에는 토양도 데이터가 없습니다. 다른 위치를 확인하세요. 기존 지도는 유지됩니다." Network messages: "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요." / "지도 서버에 연결하지 못했습니다. ..." / not-JSON: "지도 서버가 올바른 지도 데이터를 보내지 않았습니다. 잠시 후 다시 내려받으세요."

6. Storage. QTemporaryDir "ka-hgis-reference-XXXXXX" inside the directory of the requested path = folder of the survey file (ReferenceStorage.cpp:16, PreparedReferenceMap.cpp:17-28); inside: soil-1..3.geojson and 토양도_흙토람.gpkg, table "soil_map", UTF-8, written then re-read and feature count must equal (else "저장한 지도를 다시 확인하지 못했습니다. ..."). Kept (retainFiles) only after the layer was added; otherwise the folder is auto-removed; original untouched on any failure (test_reference_download.cpp:108-147). Replaced folders are removed at the next save point (ledger, Overlay:662-665). Success text: "토양도를 추가했습니다. 조사 저장으로 지도 구성을 보관하세요." (10 s); warnings: notice "토양도 확인 사항".

7. Soil layer (SoilMapService::addPrepared, .cpp:351-431). Title "토양도(흙토람)", ogr, "…|layername=soil_map". Replaces earlier layers named "토양도(흙토람)", "토양도(흙토람) […", "토양도(흙토람 그림)" (and " [" variants) or a tagged soil layer whose source has layername=soil_map (imported soil SHP never replaced). Categorized on soil_type_geo, official palette (.cpp:54-66), fill alpha 175, outline (40,44,52,180) 0.28 mm:
   - 01 산악지 (1,178,0)
   - 02 구릉지 (177,210,93)
   - 03 산록경사지 (139,68,108)
   - 04 곡간지/선상지 (254,244,182)
   - 05 해성평탄지 (132,186,230)
   - 06 하성평탄지 (192,239,245)
   - 07 고원지 (78,78,78)
   - 08 홍적대지 (254,160,0)
   - 09 용암류대지 (192,151,155)
   - 10 용암류평탄 (194,179,182)
   - 99 기타 (255,255,255)
   - else "미분류" (200,200,200)

   Props: layer_role=reference, ka_hgis/reference_kind=soil, abstract "좌표계 EPSG:5186" + ka_hgis/crs_label, ka_hgis/thematic_overlay=true with scale-based visibility OFF (LayerOps.cpp:1103). Also adds a picture layer "토양도(흙토람 그림)": XYZ `https://gis.naas.go.kr/Geodata/SF/TOP_A_SOIL_T_GEO/Layers/_alllayers/L{z}/R{y}/C{x}.png` zmin 6 zmax 15, crs EPSG:3857; URLs rewritten to L%02d/R%08x(hex)/C%08x.png by the request preprocessor (SoilMapService.cpp:115-138); props reference, ka_hgis/omit_sheet_legend=true, legend ExcludeByDefault; inserted just under the vector, but a wms/xyz layer counts as basemap and ensureSatelliteAtBottom (LayerOps::placeInLegendGroup calls it, LayerOps.cpp:1036; BasemapOps.cpp:362-401) sinks such layers to the bottom (derived from code, not run). Then OTF on, syncMapCanvas, refreshCanvasIfIdle. Soil ribbon button 「토양」 is set checked; when the kind is PaleoSoil, startPaleoLandform() is re-called via QTimer::singleShot(0) (Overlay:675-680) -> step 8.

8. Emphasis (PaleoLandformService::applyCandidateEmphasis, .cpp:82-108). Candidate codes {04,05,06,08} (.cpp:33). Renderer on soil_type_geo for 01..10,99: candidates = official colour alpha 220, outline 0.55 mm, legend "입지후보 · <name>"; others = each channel (c+440)/3, alpha 95, outline 0.28 mm, legend = name; fallback "미분류" (200,200,200,90) 0.2 mm. Outline (40,44,52,min(220,a+80)). Labels: SoilMapService::applyTerrainLabels(layer, 50000, true): only 04 곡간·선상, 05 해성평탄, 06 하성평탄, 08 홍적대지, areas >= 50000 m2, Malgun Gothic 8 pt bold (28,32,38), white buffer 0.7 (alpha 235), scale limit minimumScale 40000, fit-in-polygon. Soil layer made visible, set current.

9. Interpretation layer (ensureInterpretationLayer, .cpp:308-382). Title "고지형 판독" (kLayerTitle), table/key "paleo_landform" INSIDE THE SURVEY GPKG (path = m_surveyPath), reused if a layer with that layer_key exists. If the table is missing a memory Polygon layer (project CRS, else EPSG:5186) with String fields kind, note, status is written to the survey GPKG (CreateOrOverwriteLayer if file exists). Errors: "프로젝트가 없습니다." / "먼저 「새 조사」로 저장 위치를 만드세요." / "고지형 판독 레이어를 만들지 못했습니다." / "고지형 판독 레이어를 열 수 없습니다." / "고지형 판독 레이어를 넣지 못했습니다." Props: ka_hgis/layer_key=paleo_landform AND layer_role=reference (markReferenceLayer) AND thematic_overlay; NOT a domain layer (test_workflow.cpp:2076, 2141), not in ExportService (test 2157-2161, so not in the 5179 submit). Because role=reference, isSnapSourceLayer is false (not snappable, LayerOps.cpp:1278) and applyFeatureFormValues refuses ("참조 지도는 고칠 수 없습니다.", LayerOps.cpp:342); but it is editable/drawable (beginEdit; new drawn features get kind "미분류", status "가설", MainWindowEditing.cpp:820). Legend: no group is created, the layer list is flat (placeInLegendGroup ignores the group name, LayerOps.cpp:1021-1038); role is the property. Sheet legend does list it (test_workflow.cpp:5696).

10. Seeding (seedInterpretationFromSoil, .cpp:150-257). Starts editing; deletes every existing feature whose note starts "자동:" (counted replaced); others counted keptUser (kept). For each soil feature with geometry: code trimmed, 1 digit padded with "0"; skip unless in {04,05,06,08}; transform soil CRS -> paleo CRS if different (skip feature on failure).
    - 04 -> kind 선상지, 05 -> 해성평탄, 08 -> 하안단구, note "자동: 흙토람 " + terrainName (곡간지/선상지, 해성평탄지, 홍적대지).
    - 06 (하성평탄): r = sqrt(area/pi); inset = clamp(0.22*r, 8, 80) CRS units; core = buffer(-inset, 8 segments); if min(bbox w,h) >= 3*inset AND core non-empty AND core.area > 0.12*area -> core = 구하도 note "자동: 하성평탄 안쪽" + rim = difference(g, core) = 자연제방 note "자동: 하성평탄 가장자리"; else whole = 미저지 note "자동: 하성평탄 (폭이 좁아 분할 안 함)".
    - Any added geometry must be non-empty with area >= 1.0. status always "가설". commitChanges(false); on failure rollback, errorOut = commit errors, added=0.
    - Kinds 미고지, 개변 and 미분류 are never auto-seeded (only listed in the style).
    - Observation (not verified): soil geometries are MultiPolygon (step 5) and are added whole to a Polygon table without splitting; CLAUDE.md warns multi->Polygon GPKG can become a zero-area ring.

11. Style (applyInterpretationStyle, .cpp:269-306). Empty layer: single symbol fill (255,255,255,0), outline 0.45 mm. Otherwise categorized on kind, fill RGBA, outline 0.35 mm:
    - 자연제방 (210,140,70,150)
    - 선상지 (230,200,110,150)
    - 하안단구 (200,110,40,150)
    - 구하도 (40,90,160,160)
    - 미고지 (190,70,70,150)
    - 미저지 (70,130,95,150)
    - 해성평탄 (80,155,200,150)
    - 개변 (120,120,120,110)
    - 미분류 (90,90,90,90)

    Legend label "<kind> · 가설", or "<kind> · 자동 가설" if any feature of that kind has a "자동:" note.

12. Messages after step 10 (Overlay:400-431), then beginEdit(layer) (drawing mode).
    - added>0: notice 「고지형」 "흙토람 분포지형에서 가설 %1개를 자동으로 깔았습니다. 확정이 아닙니다." + box "흙토람 분포지형에서 가설 면을 자동으로 깔았습니다.\n\n선상지 · 해성평탄 · 하안단구는 토양 구분을 옮긴 것입니다.\n하성평탄은 안쪽을 구하도, 가장자리를 자연제방 가설로 나눕니다.\n\n옛 지형이 자동으로 복원된 것은 아닙니다. 틀린 면은 지우고 고치세요. 확정이 아닙니다." + status (no timeout) "고지형 가설 %1개 — 확정 아님. 좌클릭으로 추가, 우클릭으로 완료".
    - added==0: "분포지형을 올렸습니다. 진한 색이 입지 후보(곡간·선상·해성·하성평탄·홍적대지)입니다." + box "지도에 색 면이 생겼으면 그게 고지형입니다.\n\n진한 색 = 유적 입지 후보 (곡간·선상·해성평탄·하성평탄·홍적대지)\n연한 색 = 산지·구릉 등\n\n옛 지형이 자동으로 복원된 것은 아닙니다. 이제 그 위에 구하도·자연제방 가설을 그리세요. 확정이 아닙니다." + status "고지형 가설 — 진한 색이 입지 후보. 좌클릭으로 그리고 우클릭으로 완료".
    - Observation: the seed error string is never shown, so a failed seed lands in the added==0 text. Layer create failure: notice "판독 레이어를 만들지 못했습니다." or the service error.

13. Failures of the download. Cancel "토양도 내려받기를 취소했습니다. 기존 지도는 유지됩니다."; fail notice title "토양도 내려받기 실패" with the message (fallback "지도 자료를 받지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요."); display fail "토양도 표시 실패" ("받은 지도를 열지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요."; "저장한 토양도를 여는 데 실패했습니다."; "받은 지도에 필요한 항목이 없습니다. ..."; "토양도 레이어를 프로젝트에 넣지 못했습니다."). A second download while one runs: status "지도 자료를 내려받고 있습니다. 완료를 기다리거나 취소하세요.". Key/account: none, so key-missing/rejected: not applicable.

14. Tests (non-obvious).
    - test_workflow.cpp:2070-2257: candidate alpha >= 160; non-candidate alpha >= 90 and < candidate; labels survive emphasis; empty paleo layer renders SINGLE symbol; seeding twice keeps the user feature and gives exactly one 선상지/해성평탄/하안단구; 400x200 floodplain splits, 20x20 becomes 미저지.
    - startPaleoLandform source text must contain seedInterpretationFromSoil, "startReferenceDownload(ReferenceMapKind::PaleoSoil", clampCanvasToThematicScale and must not contain ensureDomainLayer.
    - ExportService.cpp must not contain "paleo_landform".
    - test_reference_status.cpp:258-282 (legend labels).
    - Soil: request count 6, thematic max spans soil 80000 / geology 80000 / river 160000 (test_workflow.cpp:1541).

Studio: no "paleo studio" file or tab exists in src/ (grep paleo|고지형 over src/ finds none); the only Studios are KaDrawingStudio, KaHwpReportStudio, KaTerrain3dStudio. Not-used pieces: ReferenceTiledFetch, SurveyScopeClip, DemAnalyzer, ReferenceInventory are not referenced by Soil or Paleo.

## 2. 1919 조선지형도

Entry: ribbon button 「옛 지도」 (btnOldMaps, InstantPopup) menu row "1919 조선지형도 1:5만" (object actionMap1919, MainWindowRibbon.cpp:485-500; tooltip of the button "대동여지도와 1919년 조선지형도를 참조 지도로 올립니다"; the row is a plain action, not checkable). Handler MainWindow::addHistoryGisMap1919, src/app/MainWindowOffline.cpp:467-477.

1. Extent. None requested or limited: a live tiled layer, the server chooses tiles by the view. No survey area needed. After adding: see step 7 for view changes. Zoom/scale limits of the layer: not in the source (they come from the server capabilities).

2. Enabled state (MainWindow.cpp:1425-1436, updateHistoricalMapButtons; also after saving keys, MainWindowSession.cpp:1774). Key present: row enabled, tooltip "1919년 조선지형도 1:5만 (국사편찬위원회 WMTS, EPSG:5179)". Absent: row DISABLED, tooltip "더보기 → API 키 입력에 역사지리정보DB 키를 저장하면 켤 수 있습니다". Usable = trimmed non-empty (LayerOps::historyGisApiKeyUsable, BasemapOps.cpp:1482); no format check.

3. Key = setting "역사지리정보DB API 키" (dialog 「API 키 입력」, KaApiKeyDialog.cpp:50, placeholder "hgis.history.go.kr 인증키", hint "1919 조선지형도 전용. VWorld 키로는 인증되지 않습니다." plus link https://hgis.history.go.kr/api/intro.do). Stored under ini key HistoryGis/ApiKey. Load order (VworldSettings.cpp:196-220): ka-hgis-vworld.ini in the user config folder; portable bundle: portable secrets.ini; else native store (QSettings org/app, copied back into the ini); then env HISTORY_GIS_API_KEY. Save writes ini + native (or portable secrets). Status line after save: "%1 키 저장됨" with "역사지리정보DB" (or "API 키 삭제됨" when both empty). The 더보기 status table lists "역사지리정보DB 키", used for "1919 조선지형도", "설정됨"/"미설정" (AccountStatus.cpp:23).

4. Source (BasemapOps.cpp:1486-1511). WMTS KVP via QGIS wms provider: capabilities URL `https://hgis.history.go.kr/openapi/get.do?Service=WMTS&Request=GetCapabilities&apiKey=<KEY>` (percent-encoded into the layer source), layer id history:map1919 (the only layer the server has; bare map1919 is not in its capabilities), URI: `contextualWMSLegend=0&crs=EPSG:5179&dpiMode=7&format=image/png&layers=history:map1919&styles&tileMatrixSet=EPSG:5179&url=<enc>`. So CRS EPSG:5179, tile matrix set EPSG:5179 (tile matrix ids/zoom range come from the capabilities, not in the source). No Referer sent (only VWorld gets one); UA as in the 고지형 step 4 above. Official sample key is deliberately absent (test_cadastral.cpp:166).

5. Retry: the same URI is tried 4 times in a row (historyGisMap1919Uris), because the server answers GetCapabilities with HTTP 400 "GWC configuration out of synch" about half of the time (comment, checked 2026-10-04, 3 of 6). addBasemapWithFallbacks keeps the last error.

6. Post-processing: none on the data (no download, no cache file, no clipping, no reprojection code, no file under the survey folder). Layer handling (BasemapOps.cpp:860-977, addXyzBasemap):
   - (a) removes an earlier copy of ours (same reference_kind history_1919, or untagged same-title tile layer) first; so a second click REPLACES, it does not toggle.
   - (b) QgsRasterLayer(uri, "1919 조선지형도 1:5만", "wms"); invalid -> "Basemap 실패 (1919 조선지형도 1:5만): <detail>" (detail = provider message; if it contains INVALID_KEY, 등록되지 않은, 인증키, 인증URL 불일치 or InvalidParameterValue it becomes "등록되지 않은 VWorld 키입니다. 리본의 더보기 → API 키 입력에서 키를 확인하세요." (wording says VWorld; derived from code, whether the history server returns those tokens is not in the source)).
   - (c) tuneBasemapLayer (BasemapOps.cpp:147-177): blend SourceOver; the URI has no type=xyz/crs=4326/3857 so CRS stays the provider's (5179; 3857 only if invalid); bilinear resampling zoomed in and out; limitTileOversampling sets maxOversampling 1.0 because the source contains tileMatrixSet= (reason: a 300 DPI sheet PDF piece needed >256 tiles and came out blank, 2026-10-19; test_basemap_reference.cpp:216-238).
   - (d) layer_role=reference, ka_hgis/reference_kind=history_1919.
   - (e) applyLegendCrsLabel: name kept, abstract "좌표계 EPSG:5179", ka_hgis/crs_label.
   - (f) added to the layer tree root, visible. No opacity value or blend other than SourceOver is set (default opacity 1.0, not set in source). Not snappable (reference role; raster). Not in the submit package (reference). Sheet legend: omitted because provider is wms (LayoutService.cpp:1385-1394).

7. Position: placeKindInReferenceGroup -> placeInLegendGroup, which creates NO group (flat list, LayerOps.cpp:1021-1038) and calls ensureSatelliteAtBottom; every wms/xyz layer is treated as basemap and sunk to the bottom of the list, satellite lowest (BasemapOps.cpp:362-401). So 1919 sits at the bottom, just above satellite (derived, not run). View: ensureOtfEnabled; if the view scale is 50..500000 centre kept and, if scale > 80000, zoom to 1:10000; if extent empty or scale > 400000 or < 100, zoom to Korea at 1:50000; then afterBasemapAdded (MainWindowOffline.cpp:360-373): if scale > 80000 or < 100 zoom to 1:25000; clampCanvasToKorea; refreshXyzBasemapTiles. Status line (8 s): "1919 조선지형도을 올렸습니다. 「그리기」로 구역을 그리세요." (the 을 is verbatim in the source).

8. Failures.
   - No key: LayerOps returns "1919 조선지형도는 국사편찬위원회 역사지리정보DB API 키가 필요합니다. 더보기 → API 키 입력에서 키를 저장하세요. https://hgis.history.go.kr/api/intro.do" and adds no layer (test_cadastral.cpp:143-153).
   - Server failure after 4 tries (key replaced by "[가림]", raw and percent-encoded): "국사편찬위원회 역사지리정보DB 서버가 지도 목록을 보내 주지 않았습니다(4번 시도). 서버 쪽이 잠시 불안정할 때 생기는 일이니 잠시 뒤 다시 눌러 보세요.\n\n" + detail. Shown as warning notice titled "1919 조선지형도": "1919 조선지형도를 올리지 못했습니다." + that text.
   - Tile-time key rejection (HTTP 200 + error image/XML): not in the source for this server.

9. Key in the project file: the layer source contains the key (inside the url=). I found no code that strips or refreshes the history key on save or open (withVworldApiKey only handles vworld.kr, BasemapOps.cpp:403). Log masking: test_log_session.cpp:126-141 expects "apiKey=****" in logs.

10. Other UI: the 「배경 지도」 card has an 옛 지도 entry that only opens the 옛 지도 menu (MainWindowChrome.cpp:133) and shows checked if 대동여지도 or "1919 조선지형도 1:5만" is visible (Chrome:245). test_theme.cpp:496 pins twoLine("1919 조선지형도\n1:5만") == "1919 조선지형도 1:5만"; test_window_plate.cpp:124-140 pins the key dialog hint text. Same menu's 대동여지도 (not asked): NGII WMS 1.1.1 `https://map.ngii.go.kr/spcemapserver/korea_old_map/ows`, layers korea_oldmap_ddymap_kyu then korea_oldmap_addAlphaChannel, transparent=true, EPSG:5179, no key (BasemapOps.cpp:1547-1575).

## Files

Read in full: src/core/PaleoLandformService.cpp, .h; src/core/SoilMapService.cpp, .h; src/core/PreparedReferenceMap.cpp, .h; src/core/ReferenceStorage.cpp; src/core/ReferenceKind.h; src/core/LayerRole.cpp; src/core/LayerOpsInternal.h; src/core/AccountStatus.cpp; src/app/KaReferenceDownloadJob.cpp.

Read in part:
- src/app/MainWindowOverlay.cpp 300-450, 590-695
- src/app/MainWindowRibbon.cpp 425-524
- src/core/BasemapOps.cpp 60-190, 190-320, 340-415, 820-1060, 1440-1580, 2505-2535
- src/core/LayerOps.cpp 325-355, 1015-1215, 1265-1300
- src/core/LayerOps.h 25-50, 150-210
- src/core/VworldSettings.cpp 30-234
- src/core/BasemapPolicy.h 90-160
- src/core/LayoutService.cpp 1365-1395
- src/app/MainWindowOffline.cpp 300-490
- src/app/MainWindow.cpp 1405-1445
- src/app/MainWindowChrome.cpp 205-250
- src/app/MainWindowSession.cpp 1760-1800
- src/app/MainWindowEditing.cpp 800-850, 932-1000
- src/app/KaApiKeyDialog.cpp 18-76
- src/app/KaApplication.cpp 538-552, 820-850
- src/app/KaFeatureSelectTool.cpp 286-296
- headers only (first 20 lines): ReferenceTiledFetch.h, SurveyScopeClip.h, ReferenceInventory.h
- Tests: tests/test_workflow.cpp 1015-1035, 1535-1675, 2066-2260, 5686-5702; test_reference_status.cpp 250-296; test_reference_download.cpp 90-170; test_cadastral.cpp 140-200; test_basemap_reference.cpp 200-260; grep hits only in test_theme.cpp, test_window_plate.cpp, test_log_session.cpp, test_vworld_settings.cpp, test_account_status.cpp, test_save_open.cpp, test_ribbon_pixels.cpp, test_hwp_survey_report.cpp.

Not read: ReferenceTiledFetch.cpp, ReferenceInventory.cpp, SurveyScopeClip.cpp, DemAnalyzer.cpp bodies (grep shows no Soil/Paleo/1919 use); RiverMapService.cpp, GeologyMapService.cpp; MainWindowOverlay.cpp 1-300 and 450-590 (soil toggle downloadSoilTerrain, geology/river); LayerOps::ensureOtfEnabled/syncMapCanvas/clampCanvasToKorea bodies beyond line 2535; FeatureRecord::stampNew; the survey save/open code that restores the paleo_landform layer (not found by grep beyond LayerOps.cpp:1122); docs/ and data/ (only listed by grep).
