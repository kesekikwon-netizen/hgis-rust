# ALIGN0 - C++ georeferencing (정합 / 맞추기) survey

Read-only survey. C++ tree: `%USERPROFILE%\Documents\hgis\target\cpp-ref` (abbreviated `cpp:`). Every line number below was read in the file.
Abbreviations: `MWA` = `src/app/MainWindowAlign.cpp`, `KAT` = `src/app/KaAlignMapTool.cpp`, `KATE` = `KaAlignMapToolEdit.cpp`, `KATS` = `KaAlignMapToolSave.cpp`, `GS` = `src/core/GeorefService.cpp`, `GB` = `src/core/GeorefBackup.cpp`, `GQ` = `src/core/GeorefQuality.cpp`, `KIV` = `src/app/KaImageView.cpp`.
Rust tree: `%USERPROFILE%\Documents\hgis\.claude\worktrees\fin-wb\crates\...` (abbreviated `rs:`).

Coordinates used by the tool: "source" = left pane (picture pixels, or CAD map units), "map" = right pane (work CRS map units).

---

## 1. Entry: `georefAssistant`

### 1.1 Triggers
- Ribbon button id `align`, icon `georef`, text 「정합」, tooltip 「좌표없는 사진·CAD를 도면에 합치기」: `MainWindowRibbon.cpp:522-523`. Not checkable; it just calls `georefAssistant`.
- Menu 「더보기」 item 「맞추기」 (icon `georef`): `MainWindowRibbon.cpp:643-644`.
- CAD import with no CRS found calls `startAlignSession` directly (see 7.1): `KaCadImport.cpp:254-258`, hook wired at `MainWindow.cpp:2944`.

### 1.2 "Alignable" test (`GeorefService::isAlignableLayer`, `GS:502-512`)
A layer is alignable when ALL hold:
1. layer non-null and `isValid()` (`GS:503`);
2. not a domain survey layer: `ka_hgis/layer_key` is none of `survey_area, feature_poly, feature_line, section_line, control_points, artifact_point, trial_trench` (`GS:494-500`, `GS:504`);
3. raster layer: `providerType() == "gdal"` (`GS:505-506`); vector layer: provider `"ogr"` or `"memory"` (`GS:507-510`); anything else (WMS/XYZ tiles etc.) is not.

There is NO requirement for an "align pending" flag. That flag (`ka_hgis/align_pending`, `LayerOps.h:38`, `LayerOps.cpp:1154-1164`) is separate:
- set on a picture at load when `isImagePath(path) && looksUnreferencedRaster` (`MainWindow.cpp:2892-2895`), and on every raster session start (`KAT:126`);
- while set, `pushVisible` skips the layer when stacking the canvas (`LayerOps.cpp:2002`), so the picture is not drawn on the map until 이동/저장 clears it;
- also makes the legend context menu treat the layer as ExternalRaster/ExternalVector (`MainWindowContextMenus.cpp:56`).
Consequence: any current legend layer that passes 1-3 (a plain referenced GeoTIFF, a cadastral GPKG, a reference vector) goes straight into a session with no file dialog.

### 1.3 Flow (`MWA:844-880`)
1. `layer = m_layerTree->currentLayer()` (`:846`).
2. If `!isAlignableLayer(layer)` (`:847`):
   - `QFileDialog::getOpenFileName(this, "맞출 도면", QString(), "도면 (*.png *.jpg *.jpeg *.tif *.tiff *.dxf *.dwg)")` (`:848-850`). Title 「맞출 도면」; filter text `도면 (*.png *.jpg *.jpeg *.tif *.tiff *.dxf *.dwg)`. Start dir empty. Cancel returns silently (`:851`).
   - `GeorefService::isCadPath(path)` (`.dxf`/`.dwg`, case-insensitive, `GS:521-524`): `addVectorFromPath(path); return;` (`:852`). The comment says "no coordinates -> that flow starts the alignment". `addVectorFromPath` -> `KaCadImport::run` (`MainWindow.cpp:2942-2946`). If the CRS is found the drawing is simply loaded and a notice offers to change it; no session.
   - otherwise `addRasterFromPath(path)` (`:853`, body `MainWindow.cpp:2884-2937`, see section 8). On `false` (`:854-862`):
     `KaUserError::warn(this, {title "맞추기", what "맞출 도면 파일을 열지 못했습니다.", why "지원하지 않는 그림 형식이거나 파일이 손상되었습니다.", how "JPG·PNG·TIF 그림인지 확인한 뒤 다시 시도하세요."})` (Spec field order title/what/why/how: `KaUserError.h:14-21`); return.
   - on success: `layer = current layer`; if none, loop over `QgsProject::mapLayers()` and keep the LAST alignable one (`:863-869`).
3. Still not alignable: `QMessageBox::information(this, "맞추기", "JPG·PNG·DXF 도면을 고르거나, 목록에서 도면을 선택한 뒤 다시 누르세요.")` (`:871-875`).
4. `startAlignSession(layer)` (`:876`). Note a referenced picture (GeoTIFF / picture with a world file) chosen through the dialog also reaches here (addRasterFromPath returned true) and is aligned anyway.
5. Non-QGIS build: `QMessageBox::warning(this, "맞추기", "QGIS 빌드 필요")` (`:878`) - irrelevant for Rust.

---

## 2. The session

### 2.1 `startAlignSession(layer)` (`MWA:799-842`)
Order:
1. return if no canvas/layer (`:801`); `m_alignApplied=false`, `m_alignCursorValid=false` (`:802-803`).
2. create `KaAlignMapTool(m_canvas)` once, parent = window; connect `statusChanged` -> `statusBar()->showMessage(t, 8000)`, `pairsChanged` -> `refreshAlignUi`, `cursorMoved` -> store cursor + `updateAlignOverlay` (`:804-817`).
3. `stopCaptureTool()` (`:818`).
4. CRS = `QgsProject::crs()` if valid, else `QgsCoordinateReferenceSystem(m_workCrs)` (`:820-823`). So the CRS written to the picture is the PROJECT CRS (work CRS 5186/5187), never a fixed value.
5. `m_alignTool->setBackupRoot(GeorefBackup::backupRootFor(m_surveyPath))` (`:825`).
6. `beginLayer(layer, crs, &err)`; on failure `KaUserError::warn({"맞추기","맞추기를 시작하지 못했습니다", err, {}})` and return (`:826-829`).
7. select the layer in the legend (`:830`); `showAlignSplit()`, `showSubToolsAlign()`, `applySnapConfig()` (`:831-833`); `m_canvas->setMapTool(m_alignTool)` (`:834`); `ensureOtfEnabled`, `syncMapCanvas` (`:835-836`); `refreshAlignUi()` (`:837`); `statusBar()->showMessage(statusText(), 10000)` (`:838`).

### 2.2 `beginLayer` (`KAT:102-175`)
- Starting on the clone of a drawing (`layer == m_layer` with hidden source and a CAD id) switches back to the original (`:105`). `endSession()` first (`:107`).
- failure text `이 레이어는 맞출 수 없습니다` (`:110`).
- sets `m_pairs` empty, `m_affine` invalid, `m_haveFrom=false`, `m_phase=WaitFrom` (`:113-118`).
- Raster (`:120-131`): remembers path (`source()`), original CRS, pixel width/height, `setAlignPending(rl,true)`; size < 2 px -> `그림이 너무 작습니다`.
- Vector (`:132-171`): non-memory layer -> clone to a memory layer named `"<name> 맞춤"` (`cloneToMemory`, `GS:394-470`), marked reference layer, `imported_reference` copied if the source had it or came from CAD; style transparent fill / black line 0.2 / point size 3.5 (`:147-148`); the clone is added to the project, the original is unchecked/hidden (`:149-153`); companion layers of the same CAD drawing are hidden (`:155-156`). A memory layer is aligned in place and a copy `"<name> 원본"` is kept as `m_displayClone` for the left pane (`:159-163`). `captureOriginals` stores every feature geometry by id (`KATS:96-104`); no geometry -> `도면에 도형이 없습니다` (`:166-169`).
- layer opacity 0.72 (`:172`); `emit statusChanged(statusText())` (`:173`).

### 2.3 Layout (`ensureAlignSplit`, `MWA:219-328`; `showAlignSplit`, `:379-415`; `hideAlignSplit`, `:417-426`)
- Map card gets a horizontal `QSplitter` named `alignSplitter` (not collapsible) holding `[left pane | m_canvas]` (`:227-293`). Sizes `{1000,1000}` while shown (`:413`), `{0,1}` when hidden (`:420`).
- Left pane = caption label (object name `subToolbarCaption`) + `KaImageView` + a hidden-by-default second `QgsMapCanvas` + `QListWidget` (`alignPointList`, max height 130, tooltip 「찍은 점. Delete로 지웁니다」) (`:231-286`).
- Picture session (`isRasterSession`, `:384-394`): image view shown, marks cleared, `m_alignImage->loadPath(rasterSourcePath)`; if Qt cannot read it (> 256 MB limit), fall back to `loadAlignPreviewFromRaster` (below). Label text `왼쪽 · 그림 — 여기를 먼저 찍기` (`:394`). Left canvas hidden. The left pane is the PICTURE IN PIXEL COORDINATES (a `QGraphicsView`, `KaImageView`), not a map canvas; scene coordinate = image pixel (`KIV:109-111`; click -> `mapToScene` * `m_srcScale`, `KIV:234-235`).
- CAD/vector session (`:395-412`): image view hidden; the left `QgsMapCanvas` shows `sourceDisplayLayer()` (the hidden original / the `원본` clone) with its own CRS, full extent, own zoom/pan; label `왼쪽 · CAD — 여기를 먼저 찍기` (`:411`). Default caption before a session: `왼쪽 · 맞출 도면` (`:235`).
- Large picture preview (`loadAlignPreviewFromRaster`, `:335-377`): long side capped at `maxSide = 4000` px, preview via `QgsRasterLayer::previewAsImage`; status messages `큰 그림이라 축소본을 만드는 중입니다 (%1 x %2 화소)...` (`:353`) and `원본 %1 x %2 화소는 Qt 로 한 번에 못 엽니다. 축소본(%3 x %4)을 띄우고, 확대하면 원본에서 다시 읽습니다 — 좌표는 원본 기준으로 계산됩니다.` (12000 ms, `:369-372`). On failure `notify(Warning, "사진·CAD 정합", "왼쪽에 그림을 띄우지 못했습니다." / "왼쪽에 그림을 띄우지 못했습니다: %1")` with the Qt reader error as details (`:341-342`, `:359-360`). Zoomed-in detail re-reads the visible window from the original through `GeorefService::readPixelWindow` (GDAL, 8-bit only, `GeorefPixelWindow.cpp:12-67`; 200 ms delay `KaAlignImageDetail.cpp:14`, max 4096x4096 output `:16`). Marks always report ORIGINAL pixels (`KIV:139-148`, `:183`).
- Left view interactions (`KIV`): wheel zoom x1.15 / /1.15 anchored under mouse (`:209-215`); middle-button pan (`:218-223`, `:249-256`); image fitted on load (`:202-207`, refit on resize while `m_fitted`, `:286-291`); cross cursor (`:97`). Left-click = `pixelClicked(x,y)` in original pixels (`:225-237`).

### 2.4 Placing a pair
Order: LEFT first, then RIGHT. Always.
1. Left click on the picture (`pixelClicked`) or on the left CAD canvas (`KaAlignPickTool::picked`, press handler `KATE:43-52`, which snaps through `snappingUtils()->snapToMap`, `KAT:37-48`) -> `setSourcePoint(x,y)` (`KAT:212-219`): stores the pending source, `m_haveFrom=true`, `m_phase=WaitTo`, emits status + `pairsChanged`. The window then hands the map tool back to the right canvas (`m_canvas->setMapTool(m_alignTool)`, `MWA:241-245`, `:262-266`).
2. Left click on the right (map) canvas (`canvasPressEvent`, `KAT:500-528`) with phase `WaitTo`: point = `e->mapPoint()`, replaced by the snapped point when the canvas snapping utils hit (`mapPointFromEvent`, `KAT:482-498`; blue snap circle size 16, pen 2, colour (30,103,198), fill (255,255,255,220), `KAT:464-480`). A `Pair{srcX,srcY,mapX,mapY}` is appended, `m_haveFrom=false`, phase `WaitFrom`, marks rebuilt, status + `pairsChanged`.
3. Left click on the right canvas while no left point waits: `statusChanged("왼쪽 도면에서 먼저 모서리를 찍으세요")` (`KAT:511-514`) - unless the click is within 10 px of a finished pair's map mark, which starts a drag (`beginPairDrag`, `KATE:75-83`; only when `!m_haveFrom && phase != WaitTo`; status `%1번 점을 끌어 옮기세요`).
4. Right click on the right canvas: cancels the pending left point (`m_haveFrom=false`, phase `WaitFrom`, status) (`KAT:501-507`).

Dragging (fine tuning):
- right-pane mark drag: `KATE:85-102`; grab radius 10 px (`KATE:17`), a release moving < 3 px (Manhattan) counts as a click and does nothing (`KATE:18`, `:37-39`, `:89-93`); a real drag sets `m_pairs[i].mapX/mapY`, invalidates `m_affine`, status `%1번 오른쪽 점을 옮겼습니다. 「이동」으로 다시 맞추세요`.
- left-pane drag (picture): `KIV:227-232`, `:243-247`, `:262-275`; release < 3 px = new click, else `markDragged(i,x,y)` -> `movePairSource` (`KATE:104-112`), status `%1번 왼쪽 점을 옮겼습니다. 「이동」으로 다시 맞추세요`. Left CAD canvas drag: `KATE:54-66` + `KAT:267-269`. Only FINISHED pairs are draggable (`KIV:191`).

### 2.5 What is drawn
- Picture pane marks (`KaImageView::addMarkItem`, `KIV:159-187`, `setMarks` `:189-196`): item ignores view transforms; circle radius 9, pen width 2.4, white fill (255,255,255,240); cross arms length 13 (r+4), pen 2; number bold 9 pt in (15,23,42), placed above the circle. Finished pairs ring colour red (220,38,38); the pending left point yellow (234,179,8) numbered `pairs+1`.
- Left CAD canvas marks (`refreshAlignUi`, `MWA:443-461`): `QgsVertexMarker` circle size 14, pen 2, fill (255,255,255,230); red for pairs, yellow for pending.
- Right canvas marks (`rebuildPairMarks`, `KAT:235-253`): `QgsVertexMarker` ICON_CROSS size 12, pen 2, red (220,38,38), one per pair (map point).
- Link overlay `KaAlignLinkOverlay` (`KIV:29-86`; geometry = splitter rect, mouse-transparent, `MWA:550-609`): finished pairs = blue (30,103,198,240) line width 2.4 with an arrow head (length 12, half-width 5, skipped if the line < 4 px, `KIV:43-53`) from the source point to the map point, with a numbered circle (radius 8, pen 2, white fill (255,255,255,240), bold 8 pt number) at BOTH ends carrying the pair number (`KIV:55-66`, `:73-77`). Live line for the pending point = orange (234,88,12,240) width 2.6 dashed `{6,4}`, numbered `done+1` at the source end (`KIV:78-85`). The live end follows the mouse: a 16 ms `QTimer` while a point is pending (`MWA:490-505`), `trackAlignPointer` (`:509-548`) uses the right canvas cursor (snapped if a snap hits, offsetting the on-screen end by the snap delta, `:526-537`) and calls `setMapHint`; also an event filter on MouseMove while mode is `align` (`MainWindow.cpp:2268-2273`). Overlay is hidden after 이동 (`m_alignApplied`, `MWA:552-555`) and re-laid out on wheel/resize/extent/scale changes (`MainWindow.cpp:2258-2266`, `MWA:246-249`, `:271-278`). Overlay coordinates use `mapFromGlobal(mapToGlobal(...))` because the overlay is a sibling of the views (`MWA:561-575`) - a past bug (test `updateAlignOverlay_remapsDestFromMapOnEveryPaint`).

### 2.6 Point list (`refreshAlignUi`, `MWA:469-487`)
- One row per pair: `GeorefQuality::rowText` (`GQ:58-65`): `"%1번  왼쪽 → 오른쪽"` (two spaces after 번), plus ` · 어긋남 %1 m` (2 decimals) only when 4+ pairs (`measurable`), plus ` (가장 큼)` on the max-residual row when it exceeds 1.0 m (`kResidualWarn`, `GeorefQuality.h:14`).
- Pending row: `"%1번  왼쪽만 — 오른쪽 모서리를 찍으세요"` (`MWA:478-479`).
- Notes (non-selectable, `Qt::ItemIsEnabled` only; text colour `danger` token when the text starts with 「주의」, else `inkMuted`, `MWA:481-486`) from `GeorefQuality::notes` (`GQ:67-85`):
  - exactly 3 pairs: `점 4개부터 점마다 어긋남을 잽니다`
  - 4+ pairs and max residual > 1.0 m: `주의: %1번 점이 %2 m 어긋납니다. 두 쪽 점을 다시 확인하세요.` (2 decimals)
  - 3+ pairs and anisotropy > 1.05: `주의: 가로·세로 배율이 %1% 다릅니다. 잘못 찍은 점이 있으면 그림이 늘어납니다.` (0 decimals, `(anisotropy-1)*100`)
  - 3+ pairs and shear > 3.0 deg: `주의: 그림 축이 %1° 비뚤어집니다. 잘못 찍은 점이 있는지 보세요.` (1 decimal)
  Shape math (`GQ:8-32`): singular values of the 2x2 linear part from `E=(a+e)/2, F=(a-e)/2, G=(d+b)/2, H=(d-b)/2, Q=hypot(E,H), R=hypot(F,G)`; major=Q+R, minor=|Q-R|, anisotropy=major/minor; shear = `asin(min(1,|a*b+d*e|/(hypot(a,d)*hypot(b,e))))` in degrees. Constants `kAnisotropyWarn=1.05`, `kShearWarnDeg=3.0` (`GeorefQuality.h:16-18`).

### 2.7 `statusText()` verbatim (`KAT:453-462`)
| state | text |
|---|---|
| no layer | `맞출 도면을 고르세요` |
| phase WaitTo | `오른쪽 지도에서 같은 모서리를 찍으세요` |
| no pairs | `왼쪽 도면 모서리 → 오른쪽 지적 모서리` |
| else | `왼쪽 → 오른쪽 · %1점` + `GeorefQuality::summary` |
`summary` (`GQ:87-94`): 2 pairs ` · 한 점 더 찍으면 기울기도 맞습니다`; 3 pairs ` · 한 점 더 찍으면 어긋남을 잽니다`; 4+ pairs ` · 평균 어긋남 %1 m · 최대 %2 m` (RMS over ALL pairs, `sqrt(sum d^2 / n)`, and max, both 2 decimals; computed from the points, so they show before 「이동」 too, `GQ:34-56`).
Status is shown via `statusChanged` -> `statusBar()->showMessage(t, 8000)` (`MWA:807-809`); 「점 지우기」 shows it for 4000 ms (`MWA:728`); session start for 10000 ms (`:838`).

### 2.8 Esc, delete, undo, close
- Esc on the tool (`keyPressEvent`, `KAT:538-555`): if a left point is pending, cancel it (and reset the rubber band), accept. Esc with nothing pending falls through to `QgsMapTool` (nothing else).
- Backspace / Delete on the tool: `removeLastPair()` (`KAT:549-553`).
- `removeLastPair` (`KAT:319-329`): pending left point first; else the last pair. `removePairAt(i)` (`KAT:331-341`): removes pair i, clears pending, phase `WaitFrom`, **invalidates `m_affine`** (`{}`), rebuilds marks.
- 「점 지우기」 button -> `deleteSelectedAlignPoint` (`MWA:611-625`): no row selected -> `removeLastPair`; selected row beyond pair count (the pending/notes rows) -> `removeLastPair`; else `removePairAt(row)`. Then status 4000 ms.
- Window `Delete` shortcut (`MainWindow.cpp:259-268`): when focus is in the point list calls `deleteSelectedAlignPoint`.
- Ctrl+Z: `undoLastAction` removes the last pair while the align tool is the active canvas tool (`MainWindowUndo.cpp:334-337`).

---

## 3. The transform (`GS:43-176`)

`Affine {a,b,c,d,e,f, valid, pairCount, rmsMeters}`; map = `a*sx + b*sy + c`, `d*sx + e*sy + f` (`GS:124-129`). `fromPairs(pairs, sourceYDown)` (`GS:160-176`): `sourceYDown = m_raster` (`KAT:350`, `:428`).
- `< 2` pairs: invalid.
- exactly 2 pairs: **Helmert / similarity** `helmertFromTwo` (`GS:53-77`). `ky = sourceYDown ? -1 : 1`; `dsx=p1.src.x-p0.src.x; dsy=ky*(p1.src.y-p0.src.y)`; `dm = p1.map-p0.map`; scale = |dm|/|ds|; angle = atan2(dm) - atan2(ds); `a=scale*cos`, `b=-scale*sin*ky`, `d=scale*sin`, `e=scale*cos*ky`; `c = p0.mapX - a*p0.srcX - b*p0.srcY`, `f = p0.mapY - d*p0.srcX - e*p0.srcY`. Zero-length (`< 1e-12`) in source or map -> invalid. For pictures the Y flip makes det < 0 (test `twoPointRaster_flipsYAndKeepsThirdPointOnTarget`, `test_georef.cpp:62`, comment explains the mirror-image field bug).
- 3 or more pairs: **general 6-parameter affine, least squares** `affineLeastSquares` (`GS:82-120`) on coordinates centred at their means (needed: 200000-class CAD coordinates lose all digits otherwise, `GS:79-81`). Normal-equation solution: with centred sums `sxx, sxy, syy, sxX, syX, sxY, syY`, `det = sxx*syy - sxy^2`; `a=(sxX*syy - syX*sxy)/det`, `b=(syX*sxx - sxX*sxy)/det`, `d=(sxY*syy - syY*sxy)/det`, `e=(syY*sxx - sxY*sxy)/det`, `c = mapMeanX - a*meanSrcX - b*meanSrcY`, `f = mapMeanY - d*meanSrcX - e*meanSrcY`. Degenerate guard (collinear points): `!(sxx>0 && syy>0) || |det| <= 1e-10 * sxx * syy` (`kRelDetEps`, `GS:45`, `:110`) -> not valid -> `fromPairs` falls back to `helmertFromTwo(pairs[0], pairs[1], sourceYDown)` (`GS:166-169`). Test `absoluteSource_collinearFallsBackToHelmert` (`test_georef_quality.cpp`). The flip is carried by the data itself for 3+ points, not by a flag.
- Afterwards `pairCount = pairs.size()` and `rmsMeters = sqrt(sum((T(src)-map)^2)/n)` (`GS:171-175`, `:146-158`). With 3 points an affine fits exactly (rms 0), hence 4+ for a meaningful figure.
- No projective model anywhere. No residual rejection: 이동 never refuses because of residuals; they only appear as notes/status (text in 2.6/2.7). The only "cannot" message is `점 배치로 변환을 만들 수 없습니다` (`KAT:352`) when `fromPairs` is invalid (e.g. both points identical on either side).
- `invert` `GS:131-140` (det < 1e-18 -> false), `toQTransform` `GS:142-144` (`QTransform(a,d,b,e,c,f)`).
- Other fitters (`fitRasterToExtent`, `fitSrcBoxToExtent`, `GS:178-206`) and `KaAlignMapTool::fitToDisplay` (`KAT:255-281`) exist but nothing in `src/app` calls `fitToDisplay` except tests (`test_align_tool.cpp`, `vectorSave_keepsAnOlderAlignedFile`); the UI never shows a "fit to screen" action. `fitRasterToExtent`: `a=W/pw`, `e=-H/ph`, `c=xmin`, `f=ymax`.

---

## 4. 이동 (`applyAlignMove`, `MWA:627-703`; `KaAlignMapTool::applyMove`, `KAT:345-416`)

### 4.1 Tool part
1. `< 2` pairs: error text `점을 2개 이상 찍은 뒤 「이동」을 누르세요` (`KAT:346-349`); window shows `KaUserError::warn({"이동","도면을 옮기지 못했습니다", err, {}})` (`MWA:632`).
2. `m_affine = fromPairs(m_pairs, m_raster)`; invalid -> `점 배치로 변환을 만들 수 없습니다`.
3. **Picture (raster)** (`KAT:355-397`):
   - `ensureRasterBackup` (section 5) - failure text `원래 좌표를 백업하지 못해 그림 파일을 바꾸지 않았습니다. %1` (`KATE:114-122`) and nothing is written.
   - `GeorefService::persistAlignedRaster(layer, affine, workCrs)` (`GS:332-341`) = `applyWorldFileToRaster` + check `looksUnreferencedRaster` (fail text `그림을 지도 좌표로 붙이지 못했습니다. 점을 다시 찍고 이동하세요.`).
   - **`applyWorldFileToRaster`** (`GS:290-330`), in this exact order:
     1. `releaseRasterHandle` = `dataProvider()->reloadData()` (`GB:167-170`) so the open GDAL handle does not re-flush PAM data over the new files (`GS:297-302`).
     2. `writeWorldFile` (`GS:224-250`).
     3. `.prj` via `writeSidecarPrj(src, crs)` if the CRS is valid (errors ignored) (`GS:304`, `:252-265`).
     4. `dropGdalPamSidecar`: for both `<image>.<ext>.aux.xml` and `<basename>.aux.xml` (`GB:151-158`), delete it only if it contains `<GeoTransform` or `<GCPList` (`GB:178-183`); statistics-only PAM files are kept (`GS:284-288`).
     5. If the file is a GeoTIFF with its own geotransform/GCPs (`hasInternalGeoref`, `GB:185-188`, uses `GEOREF_SOURCES=INTERNAL`, `GB:40`), the file's own geotransform is overwritten with `GDALSetGeoTransform(ds, {c,a,b,f,d,e})` opened `GA_Update` (falls back to read-only open = PAM sidecar) (`GS:269-280`, `:309`). Plain JPG/PNG/untagged TIFF bytes are never modified.
     6. `reopenRasterLayer(layer, crs)` = `setDataSource(source, name, "gdal", false)` + `setCrs(crs)` (`GB:172-176`) - SAME `QgsRasterLayer` object and id; never remove+recreate (`GeorefService.h:51`). If a georeferencing PAM reappeared, drop it and reopen once more (`GS:313-316`).
     7. invalid layer after reopen -> `맞춰진 그림을 다시 열지 못했습니다`.
     8. `setAlignPending(layer,false)`, tick the layer in the legend, `triggerRepaint` (`GS:321-328`).
   - If `persistAlignedRaster` fails but a world file exists, a fallback rebuilds the layer: remove it from the project, create a new `QgsRasterLayer(src, name, "gdal")`; if still invalid/unreferenced -> `그림을 지도 좌표로 붙이지 못했습니다. 점을 다시 찍고 이동하세요.`; otherwise `setCrs(workCrs)`, reference-layer mark, copy `imported_reference`, align-pending off, legend CRS label, overlay style, add to project (`KAT:363-389`). On the happy path: `markReferenceLayer`, `applyLegendCrsLabel`, `styleAlignedRasterOverlay` (`KAT:390-394`).
4. **Vector** (`KAT:395-397` -> `applyPreview`, `KAT:283-317`): `canvas->freeze(true)`; `applyAffineToVector(layer, affine, originals, workCrs)` (`GS:353-392`): `startEditing`, every feature's ORIGINAL geometry (from the session snapshot, so repeated 이동 is not cumulative) is transformed with `QgsGeometry::transform(QTransform(a,d,b,e,c,f))` and written with `changeGeometry`; any failure -> `rollBack` + `도형 변환 실패`; `commitChanges` failure -> `도면 저장 실패`; `startEditing` failure -> `도면을 고칠 수 없습니다`; then `layer->setCrs(workCrs)`, `reloadData`, `updateExtents`, `triggerRepaint`. Then restyle: transparent fill, stroke (30,103,198), width 0.9, point size 4.0, opacity 1.0 (`KAT:302-304`); `freeze(false)` + `refreshCanvasIfIdle`.
5. Tail: `setAlignPending(false)`, tick layer visible, `refreshCanvasIfIdle`, `rebuildPairMarks`, status `이동 완료 · %1점` + (4+ pairs) ` · 평균 어긋남 %1 m` (2 decimals, `rmsMeters`), `pairsChanged` (`KAT:398-415`).

### 4.2 World file (`GS:208-250`) - the answer to "what is written"
- Name (`worldFilePathFor`, `GS:208-217`): `.jpg`/`.jpeg` -> `.jgw` (the extension is cut off by its length and `jgw` appended, so `scan.JPG` becomes `scan.jgw`); `.png` -> `.pgw`; `.tif`/`.tiff` -> `.tfw`; anything else -> `<full image path>.wld`. Same folder as the image (it sits next to the original, not in a new folder).
- Content: six lines, each `QString::number(v, 'g', 16)` + `"\n"`, UTF-8, file opened with `QIODevice::Text` (so line ends become CRLF on Windows). Order: **A, D, B, E, C', F'**, where A=a, D=d, B=b, E=e and the origin is moved to the pixel CENTRE: `C' = c + 0.5*(a+b)`, `F' = f + 0.5*(d+e)` (`GS:235-248`). (Our `Affine` is corner-based; world-file convention is centre-of-first-pixel.) The in-file GeoTIFF geotransform uses the corner-based `{c, a, b, f, d, e}` (`GS:276`).
- `.prj`: `<folder>/<completeBaseName>.prj` (`prjPathFor`, `GS:219-222`), body = `QgsCoordinateReferenceSystem::toWkt()` with default arguments (so the WKT flavour is QGIS's default; the exact string is not in the source), UTF-8 (`GS:263`). CRS = project CRS (section 2.1 step 4), normally EPSG:5186/5187.
- No `.aux.xml` is written (existing georef-carrying ones are deleted). No VRT, no new image.
- The layer CRS is set on the layer object with `setCrs(crs)` (`GB:175`).

### 4.3 Window part after the tool succeeds (`MWA:635-702`)
`m_alignApplied=true`; hide overlay (`:636`); `hideAlignSplit()` (left pane hidden, splitter `{0,1}`; session object stays) (`:637`); `applySnapConfig`, `ensureDefaultBasemaps` (`:638-639`); clear pending + tick the layer (`:640-647`); `syncMapCanvas` (`:648`); **stack position** (`:649-665`): take the canvas layer list (or visible paint order), remove the aligned layer, insert it at the index of the first layer that is a basemap (`isBasemapLayer`) or whose name contains `위성` or `지적`, else append at the end, then `m_canvas->setLayers(stacked)`. `QgsMapCanvas::layers()` is ordered top-first, so the picture lands immediately above that first basemap/위성/지적 layer and below every layer listed before it (the C++ source comment says it goes on the cadastral; Port step 11 asks to confirm visually). Zoom (`:666-678`): only when the canvas CRS has known map units AND the aligned raster is no longer `looksUnreferencedRaster`: `setExtent(ext)`, `zoomToFeatureExtent(ext)`, then `zoomScale(scale()*1.25, true)` (25 % margin), where `ext = mapSettings().layerExtentToOutputExtent(layer, layer->extent())`. Then `refreshAlignUi`, three staggered repaint kicks: now, `singleShot(0)`, `singleShot(350)` (`:680-691`).
Status 10000 ms (`:695-701`): `맞춘 도면을 지금 보는 지적 위에 올렸습니다. 흰 종이만 빼고 먹선은 진하게 보이게 했습니다.` + (if a backup exists) ` 그림 옆 좌표 파일을 새로 썼고, 원래 좌표는 %1 에 백업했습니다.` (path with native separators).

### 4.4 Picture drawing style after alignment (`styleAlignedRasterOverlay`, `GS:603-610`)
Paper scan (`looksLikePaperScan`, `GS:526-561`: sample a 32x32 (at most) block; if >= 42 % of valid pixels have R,G,B all >= 236 it is a scan; if the block cannot be read, a source name containing `.tif` is NOT a scan, `.jpg/.jpeg/.png/.bmp/.gif` is) -> `styleAlignedInkScan` (`GS:563-583`): opacity 1.0, blend mode Multiply, resampling at provider stage with Nearest zoom in/out, brightness filter contrast +55, brightness -28, gamma 0.75, and a transparent-colour entry for white: `TransparentThreeValuePixel(255,255,255, opacity 0.0, tolerance 8,8,8)` (`:580-582`). Anything else (aerial GeoTIFF etc.) -> `styleAlignedColorRaster` (`GS:585-601`): opacity 1.0, SourceOver, Bilinear resampling, brightness/contrast 0, gamma 1.0, transparency cleared.

---

## 5. Backup and 되돌리기

### 5.1 Backup (`GeorefBackup::backupRaster`, `GB:190-230`)
- When: lazily, at the first write: `ensureRasterBackup` (`KATE:114-122`) is called from `applyPreview` (raster), `applyMove` and `saveAligned`; if `m_rasterBackup.valid` already, nothing more. A session that never writes never creates a folder (test `rasterRestore_beforeAnyWrite_saysNothingChanged`, `test_align_tool.cpp:80-100`).
- Where: `<survey folder>/정합백업/` (`backupRootFor`, `GB:145-149`: `QFileInfo(surveyPath).absoluteDir().filePath("정합백업")`; with no survey open: `<AppLocalDataLocation>/정합백업`). Per backup a sub-folder `<imageCompleteBaseName>_<yyyyMMdd-HHmmss>`, with `_2`, `_3`, ... appended when it exists (`GB:198-203`).
- What: a copy of each of these files that exists next to the image: world file (`worldFilePathFor`), `<base>.prj`, `<image>.<ext>.aux.xml`, and `<base>.aux.xml` (`GB:210-223`), plus `backup.json` (`GB:120-141`) = `{image, created (ISO), sidecars:[{file, existed}], internalGeoTransform?[6], internalGcpCount}`. For a GeoTIFF the internal geotransform and GCPs (+ GCP WKT) are held in memory in `RasterBackup` (`GB:35-65`, struct `GeorefBackup.h:28-37`) - NOT as a file copy.
- Errors: `그림 파일을 찾지 못했습니다`, `백업 폴더가 없습니다`, `백업 폴더를 만들지 못했습니다: %1`, `%1 을(를) 백업하지 못했습니다`, `그림 파일의 좌표를 읽지 못했습니다` (`GB:196-225`).
- The backup folder stays on disk after the session; the session just forgets the handle (`KAT:193-194`). Test `plainScan_moveThenRestore_leavesNoTrace`, `geotiff_restoreBringsBackEmbeddedCoordinates`, `pamWithGeoref_isBackedUpAndRestored` (`test_georef_backup.cpp:27-29`).

### 5.2 Restore (`restoreOriginals`, `KATE:124-168`; `GeorefBackup::restoreRasterLayer`, `GB:260-277`)
Common first (`KATE:125-129`): clears pairs, pending, phase `WaitFrom`, affine, drag index. Then by case; the returned text becomes `message`:
| case | text | returns |
|---|---|---|
| no layer | `되돌릴 맞추기가 없습니다` | true |
| vector, edit ok | `도면을 맞추기 전 자리로 되돌렸습니다` (writes every stored original geometry back with `changeGeometry` + `commitChanges`, repaint) | true |
| vector, edit failed | `찍은 점은 지웠지만 도면을 되돌리지 못했습니다` | false |
| picture, nothing backed up yet | `찍은 점을 지웠습니다. 그림 파일은 아직 바뀌지 않았습니다` | true |
| picture, restored | `그림 좌표를 맞추기 전으로 되돌렸습니다 (백업: %1)` (native path) ; also `setAlignPending(rl,true)` and `setOpacity(0.72)` | true |
| picture, failed | `찍은 점은 지웠지만 그림 좌표를 되돌리지 못했습니다. %1` | false |
Then marks cleared, `refreshCanvasIfIdle`, status + `pairsChanged`.
Picture restore mechanics (`GB:232-277`): release GDAL handle; first the internal GeoTIFF geotransform/GCPs (only if different: `writeInternal`, GCPs first because setting one clears the other, `GB:76-100`; failure text `그림 파일 안의 좌표를 되돌리지 못했습니다. 백업: %1`); then each sidecar: backed-up copy -> remove current + copy back, or "did not exist" -> delete it (`GB:245-253`; failure `%1 을(를) 되돌리지 못했습니다. 백업: %2`); reopen the same layer; if sidecars differ again (a flush of PAM by the closing handle) restore a second time (`GB:268-272`); `되돌린 그림을 다시 읽지 못했습니다` if invalid. Missing backup: `되돌릴 백업이 없습니다`.

Window side (`MWA:730-744`): `restored = restoreOriginals(&message)`, `m_alignApplied=false`; if there is a session and the left pane is hidden, `showAlignSplit()`; `refreshAlignUi()`; if restored -> `statusBar()->showMessage(message, 8000)` else `notify(Warning, "맞추기", message)`.

---

## 6. 「맞추기 저장」 (`MWA:745-792`, `saveAligned` `KAT:418-451`)
Button icon `save`, text 「맞추기 저장」.
1. `pairCount < 2 && !m_alignApplied` -> `notify(Warning, "맞추기", "점을 2곳 이상 찍은 뒤 저장하세요.")`; stop (`MWA:748-752`).
2. If not yet applied: run `applyAlignMove()`; if still not applied, stop (`:753-756`).
3. `m_alignTool->saveAligned(&path, &err)`; failure -> `notify(Warning, "맞추기", "맞춘 결과를 저장하지 못했습니다.", err)` (`:757-762`).
   `saveAligned` (`KAT:418-451`): no layer -> `맞출 도면이 없습니다`; `<2` pairs and no affine -> `점을 두 곳 이상 찍은 뒤 저장하세요`; with >= 2 pairs it recomputes the affine and `applyPreview()` again.
   - **Picture**: backup if needed, `persistAlignedRaster` again (writes the world file + prj + PAM cleanup again, i.e. identical to 이동), marks reference layer, legend CRS label, overlay style, pending off, ticked; `*savedPath = worldFilePathFor(rl->source())` (`KAT:431-448`). So "saving" a picture = the world file path; there is NO new GeoTIFF, no copy.
   - **Vector from a CAD drawing** (`m_cadDrawing` set): see 7.2; no extra file.
   - **Other vector** (`saveVectorAligned`, `KATS:64-83`): writes a COPY GPKG `<dir>/<name without " 맞춤">_aligned.gpkg` where dir = folder of the hidden source file (else of the layer source, else the temp dir). The path is made unique with `GeorefBackup::uniqueOutputPath` (`_2`, `_3`, ... up to 9999, then ms timestamp, `GB:279-289`): an older `_aligned.gpkg` is never deleted; saving again in the same session rewrites its own copy (test `vectorSave_keepsAnOlderAlignedFile`). Written by `QgsVectorFileWriter::writeAsVectorFormatV3` (driver GPKG, UTF-8), with a CRS transform to the work CRS if the layer CRS differs (`GS:472-492`; failure `GPKG 저장 실패`). Then marks reference layer + legend label.
4. Window: `hasSession() ? hideAlignSplit() : hideSubTools()` (CAD drawings end the session on save, so `hideSubTools` closes the toolbar) (`:763`); layer: pending off, ticked, re-inserted in the canvas layer list with the same "before first basemap / 위성 / 지적" rule as 이동 (`:764-787`), `triggerRepaint`, `refreshCanvasIfIdle`.
5. Messages: `notify(Success, "맞추기", "맞춰 두었습니다. 이 도면은 참고용이니 제출할 구역은 그리기로 직접 그리세요.", <native path>)` (`:788-790`) and status 8000 ms `맞춤 저장: %1` (path with the tool's separators, `:791`).
The layer is NOT moved to another legend group or file here; reference-layer marking (`markReferenceLayer`) puts it in the reference role (project rule: aligned rasters are reference maps).

---

## 7. CAD drawings (DXF/DWG) and vectors

### 7.1 Getting into alignment
`addVectorFromPath` -> `KaCadImport::run` (`MainWindow.cpp:2942-2946`) reads the drawing into converted tables (lines, fills, points, texts) grouped under 참조 지도. If no CRS could be guessed (`authId.isEmpty()`), it calls `hooks.startAlign(CadDrawingLayers::alignLayerOf(project, id))` (`KaCadImport.cpp:254-258`); `alignLayerOf` picks the first existing of tables `lines, fills, points, texts` in that order (`CadDrawingLayers.cpp:211-218`). With a CRS found, only a notice is shown and no session starts (`:259-260`). The view is not moved for coordinate-less drawings (`KaCadImport.cpp:221`). Status line `도면 「%1」 올림 · 도형 %2개 · %3` with `좌표 없음` as the CRS text (`KaCadImport.cpp:241-247`).

### 7.2 What alignment does to a CAD drawing
- Session works on a memory clone (see 2.2) named `"<layer> 맞춤"`; the other tables of the drawing are hidden meanwhile (`CadDrawingLayers::hideCompanions`). The left canvas shows the hidden original with its own CRS.
- Source coordinates are MAP UNITS (CAD coordinates, y up), so no Y flip: `fromPairs(..., false)`. Fit: 2 pairs Helmert, 3+ affine LSQ (centred; the CAD coordinates are 200000-class).
- 이동 transforms the clone's geometries (`applyAffineToVector`) in memory only; the file is untouched until save.
- Save for a drawing (`saveVectorAligned`, `KATS:54-63`): `toDrawingCrs` (`KATS:21-45`) re-expresses the affine in the drawing's own CRS when the work CRS changed since import (three points around the drawing extent half-span `max(w,h,100)/2` mapped through `QgsCoordinateTransform(work -> drawing CRS)` and refitted by `fromPairs(...,false)`; error `맞춘 자리를 도면 좌표계로 바꾸지 못했습니다.`); then `CadDrawingLayers::saveAlignment` (`CadDrawingLayers.cpp:286-295`) -> `CadDrawingStore::applyAffine` (`CadDrawingStore.cpp:194-244`) applies the SAME affine to EVERY table of the drawing directly on the data provider (`changeGeometryValues`); for text features it also changes attributes: `text_angle += atan2(d,a)` in degrees and `text_height *= hypot(a,d)`; on any failure it restores all touched layers and returns `맞춘 결과를 도면 전체에 적용하지 못했습니다.`; no layers -> `맞출 도면 레이어가 없습니다.`. Saved path returned = the first table layer's source file. NO `_aligned.gpkg` is written. The session then ends (`endSession`, which also removes the clone and shows all the drawing's layers again via `finishDrawingSession`, `KATS:86-94`).
- Closing without saving (`닫기`, `hideSubTools` -> `stopAlignSession` -> `endSession`) removes the clone and shows the drawing as it was (test `cancel_removesTheCloneAndShowsEverything`, `test_cad_align.cpp:139`). Restarting on the clone starts again from the original drawing (`KAT:105`, test `restartOnTheClone_beginsAgainOnTheDrawing`). If the drawing was re-imported meanwhile saving is refused (`drawingReplacedDuringAlign_refusesToSaveAndCleansUp`, `:176`). Work CRS changed after import: file is written in the drawing's CRS (`save_afterTheWorkCrsChanged_writesTheDrawingCrs`, `:198`). All tables moved: `save_movesEveryTableAndWritesNoAlignedCopy`, `:111`.
- Other vectors (not CAD): clone `... 맞춤` + `_aligned.gpkg` copy (6 above); a memory layer is edited in place.

---

## 8. The picture layer when loaded without coordinates

### 8.1 `addRasterFromPath` (`MainWindow.cpp:2884-2937`)
1. `QgsRasterLayer(path, completeBaseName, "gdal")`; invalid -> `false` (`:2886-2891`).
2. `unrefImage = isImagePath(path) && looksUnreferencedRaster(rl)` (`:2892-2893`). `isImagePath` = ends with `.jpg .jpeg .png .tif .tiff .gtiff` (case-insensitive, `GS:514-519`). If true: `setAlignPending(rl,true)` (`:2894-2895`). Else if the CRS is invalid and the project CRS is valid: `rl->setCrs(project CRS)` and status `GeoTIFF에 좌표계가 없어 작업 좌표계(%1)로 올렸습니다.` (8000 ms) (`:2896-2900`).
3. `markReferenceLayer`, property `ka_hgis/imported_reference = true`, `applyLegendCrsLabel`, add to the project (`:2904-2907`); if the file is `.tif/.tiff/.gtiff` -> `LayerOps::knockOutRasterPaper(rl)` (`:2908-2909`).
4. Select it in the legend; if `unrefImage`, uncheck its visibility (`:2910-2914`); `ensureOtfEnabled`, `pruneEmptyLegendGroups`, `refreshLayerEmptyState`, `syncMapCanvas` (`:2915-2919`).
5. `unrefImage`: status `좌표 없는 그림입니다. 더보기 → 맞추기로 지적 위에 올리세요.` (10000 ms) and return true (`:2920-2924`).
6. Referenced GeoTIFF that sits on the work map: `zoomToLayerMax`, else `그림은 올렸지만 범위를 잡지 못했습니다.` (`:2925-2928`); else status `그림 추가: %1 · %2 · 화면 %3` (title, CRS, work CRS) (`:2929-2931`).

### 8.2 `looksUnreferencedRaster` (`GS:612-631`) - what "unreferenced" means
True when ANY:
- layer null/invalid; width < 2 or height < 2 (`:613-616`);
- extent empty or non-finite (`:617-618`);
- extent lies inside the pixel plane: `xmin > -2 && ymin > -2 && xmax < pw+2 && ymax < ph+2` (`:622-624`) - i.e. GDAL gave the default 0..w,0..h geotransform;
- extent size equals the pixel size (`|W-pw| < 2 && |H-ph| < 2`) AND the origin is still near the pixel plane (`xmin < pw+10 && ymin < ph+10`) (`:627-629`) - a real 1 m/pixel image far from the origin is NOT unreferenced (`:625-626`).
An embedded GeoTIFF geotransform in map coordinates counts as referenced even with no `.tfw` (`:619-621`; test `geotiffEmbeddedGtIsReferenced`). Pictures with a world file next to them read as referenced through GDAL, so they are not flagged.

### 8.3 `knockOutRasterPaper` (`BasemapOps.cpp:193-215`)
- skip null/invalid layer and basemaps (`:194-195`);
- if the layer is align-pending but NOT unreferenced, clear the flag (an embedded-GT GeoTIFF must not stay hidden, `:196-198`);
- needs >= 3 bands and a renderer (`:199-201`);
- if the renderer already has a transparent three-value entry with red == 255 and opacity < 0.01, do nothing (`:202-208`);
- otherwise install a new `QgsRasterTransparency` with ONE entry: `TransparentThreeValuePixel(255,255,255, opacity 0.0, tolerance 16,16,16)` - white (each channel within 16 of 255, i.e. >= 239) becomes fully transparent - then `triggerRepaint` (`:209-214`).
It is also applied project-wide by `knockOutProjectRasterPaper` (`BasemapOps.cpp:223-227`) and only for TIF in `addRasterFromPath`. JPG/PNG get the white knock-out only AFTER alignment, through `styleAlignedInkScan` (tolerance 8, section 4.4). Test `knockOutRasterPaperMakesWhiteTransparent` (`test_georef.cpp`, function `knockOutRasterPaperMakesWhiteTransparent`).

---

## 9. Other things a faithful port must not miss
- Layer opacity 0.72 during a session for both pictures and clones (`KAT:172`), 1.0 after 이동 (raster style / `KAT:304`); restore sets 0.72 again (`KATE:155`).
- Auto-end of the session: `stopAlignSession` (`MWA:705-714`) = `hideAlignSplit`, unset the tool from the canvas, `endSession`, set the pan tool. Called from: sub-toolbar 닫기 -> `hideSubTools` (`MainWindowRibbon.cpp:769-772`), switching to buffer/draw sub toolbars (`:788`, `:844`), starting to draw (`MainWindowEditing.cpp:945`), and before project read / new survey / open / load survey layers (`MainWindowSession.cpp:263, 601, 707, 766, 1660`).
- `m_alignApplied` (`MainWindow.h:480`): after 이동 the overlay and left pane are hidden but the session stays, so 되돌리기 can bring the left pane back (`MWA:737-738`) and 저장 reuses the applied result (`:753`).
- Survey saving: nothing in the align code marks the survey dirty or saves it; alignment writes sidecar files and edits layers/properties only. The `ka_hgis/align_pending` layer property persists with the project (custom property). The Rust `is_dirty = true` on 이동 has no C++ counterpart (not in the source).
- Sounds: none (grep for sound/chip/beep in `MWA`, `KAT*`, `KIV` found nothing). No "chip" state either. Only status messages and `notify` banners.
- No automatic residual-based refusal; no sub-pixel snap in C++ (the Rust `find_subpixel_crosshair_center_15x15` has no counterpart: see 10).
- Snap: right canvas and left CAD canvas snap to features through the canvas snapping config (`applySnapConfig()` is called at start and after 이동); the picture pane never snaps. The VWorld cadastral picture cannot be snapped to (project rule).
- Thread/UI safety rules in the code: use `LayerOps::refreshCanvasIfIdle`, `canvas->freeze(true/false)` around geometry edits, and never `removeMapLayer`+recreate for a normal picture (project crash rules in `CLAUDE.md`).
- Layer-order rule on 이동/저장: before the first basemap / name contains 위성 / 지적 (`MWA:654-663`, `:774-782`).
- Test files that define the contract: `test_georef.cpp` (affine/Helmert, world file, unreferenced test, knock-out, style), `test_georef_quality.cpp` (fit exactness with 200000-class coordinates, collinear fallback, notes/summary texts), `test_georef_backup.cpp` (backup/restore, `정합백업` root, `uniqueOutputPath`, `readPixelWindow`), `test_align_tool.cpp` (restore messages, picture restore, vector save naming), `test_align_image_view.cpp` (drag reports original pixels, click on mark still picks, zoomed preview re-reads original window), `test_cad_align.cpp` (CAD drawing flow).

---

## 10. Rust today

(`rs:` = `...\fin-wb\crates\`.)

- **Entry (`georefAssistant`)**: PARTLY FAKE. `rs:strata-app/src/gui_ribbon_click_b.rs:49-62` handles the 「정합」 button, picking `alignable_layer_name` (`rs:strata-app/src/gui_ribbon_align_actions.rs:98-126`). That function decides alignability from legend keys and a `vworld_` prefix, not from "gdal raster / ogr / memory vector" and it picks the first list layer when the current one does not qualify. MISSING: the 「맞출 도면」 file dialog (title, filter), DXF/DWG branch, `addRasterFromPath` branch, the failure warning, the later "last alignable layer" search. REAL: the final message text for no layer (`gui_ribbon_click_b.rs:57-60`), reproduced as a guide message with a prefix. `rs:strata-app/src/user_files.rs:23-37` classifies extensions similarly to `isImagePath/isCadPath` (no `.gtiff` difference, `.json` extra) but is not wired to the align entry.
- **Session**: MISSING almost entirely. `start_align_session` (`gui_ribbon_align_actions.rs:129-139`) only stores a layer name, toggles the sub-toolbar and sets `MapToolKind::GeorefAlign`. There is no left pane, no picture view, no second canvas, no splitter, no overlay arrows, no marks, no point list, no click handling at all: `pairs.push` appears only in tests (`rs:strata-app/src/gui_ribbon_tests_align.rs:41-109`). The tool hint for `GeorefAlign` is FAKE: `rs:strata-app/src/ribbon_state.rs:128-130` says `정점을 클릭하여 드래그 이동하세요. [Delete]: 선택 정점 삭제` (not a C++ text; C++ texts are in 2.7), and `rs:strata-canvas2d/src/map_tools.rs:24,39` names it 「도면정합」 / `사진과 지도에서 같은 자리를 짝지어 클릭합니다` (not in C++). Pair/pending state (`rs:strata-app/src/gui_ribbon_align.rs:11-22`) mirrors `m_pairs/m_haveFrom/m_alignApplied`; no phase, no drag index, no originals, no backup handle.
- **statusText and quality summary**: REAL for the first three states and 2/3-pair suffixes (`gui_ribbon_align.rs:27-42`, `:47-73`, tests `gui_ribbon_tests_align.rs:23-65`). FAKE for 4+ pairs: it computes a similarity (Helmert) residual (`gui_ribbon_align.rs:55-71`), whereas C++ computes affine residuals; max residual is computed against the wrong model. MISSING: `rowText`, `notes` (3-pair note, 주의 warnings), `shape` (anisotropy/shear), per-point residuals.
- **Transform**: FAKE/WRONG. `rs:strata-geo/src/helmert_solve.rs:47-103` (`solve_helmert_georef`) is a centred similarity least squares for any n >= 2 with NO Y flip and no 3+-point affine; `apply_align_move` uses it for all counts (`gui_ribbon_align_actions.rs:18-22`). C++ uses: 2 points = Helmert with Y flip for pictures; 3+ = affine LSQ with collinear fallback (section 3). `rs:strata-geo/src/gcp_solve.rs:87` (`solve_gcp_transform`, affine/Helmert) exists but is not used by the align state (not checked whether it matches C++; it documents itself as `KaGcpTransformSolver`, a name that is NOT in the C++ tree searched). `rs:strata-geo/src/georef_quality.rs` (`evaluate_rmse_grade` A-D grades, `georef_quality_summary_ko`, `find_subpixel_crosshair_center_15x15`) cites C++ `GeorefQuality::evaluateRmseGrade`, `georefQualitySummary`, `GeorefPixelWindow::findSubPixelCrosshairCenter`, `GeorefHelmert`: none of these names exist in the C++ tree (grep of `src` and `tests` found no match), so that Rust file is NOT a port of the C++ behaviour. `unique_georef_output_path` (`georef_quality.rs:137-170`) does correspond to `GeorefBackup::uniqueOutputPath`.
- **이동**: FAKE. `apply_align_move` (`gui_ribbon_align_actions.rs:14-29`) only stores the transform and sets `applied`/`is_dirty`; it writes no world file, no `.prj`, no backup, no PAM cleanup, no CRS, does not restyle (ink style / white knock-out), does not clear align-pending, does not reorder or zoom the canvas, shows no completion status. Error text `점을 2개 이상 찍은 뒤 「이동」을 누르세요` is REAL (`:16`), but the UI wraps it as `도면을 옮기지 못했습니다 — …` (`gui_ribbon_subtoolbar.rs:~60-63`) instead of the `KaUserError` card (title 「이동」).
- **Backup / 되돌리기**: FAKE. `restore_align_originals` (`gui_ribbon_align_actions.rs:33-52`) clears points and returns two C++ texts (`되돌릴 맞추기가 없습니다`, `찍은 점을 지웠습니다. 그림 파일은 아직 바뀌지 않았습니다`, `도면을 맞추기 전 자리로 되돌렸습니다`) but nothing is backed up or restored on disk; the `applied` flag stands in for `m_rasterBackup.valid`. The "restored" text is shown even though no file changed. MISSING: `정합백업` folder, `backup.json`, sidecar copy/restore, GeoTIFF internal-GT restore, failure texts, picture re-flagged as align-pending with opacity 0.72.
- **맞추기 저장**: FAKE. `save_align_result` (`gui_ribbon_align_actions.rs:56-82`): the < 2 points path and both success texts are verbatim (`:60`, `:75`), but nothing is written, `saved_path` is never set, so the status falls back to `status_text()`; no reorder; it ends the session unconditionally, while C++ ends it only for CAD drawings (`MWA:763`). The < 2-points refusal is shown as a plain guide message `맞추기 — 점을 2곳 이상 찍은 뒤 저장하세요.` (C++: `notify(Warning, "맞추기", ...)` without the trailing dash form).
- **CAD (DXF/DWG)**: MISSING. No clone layer, no `applyAffine` on drawing tables, no text rotation/scale, no `_aligned.gpkg`, no `toDrawingCrs`, no import-driven `startAlign`.
- **Unreferenced picture detection / align-pending / knock-out**: MISSING. No `looksUnreferencedRaster`, no `ka_hgis/align_pending` handling, no `knockOutRasterPaper`, no `looksLikePaperScan` / ink style; `rs:strata-layout/src/raster_stretch.rs` and other raster code were not checked against `GS:526-601`.
- **Close / end session**: REAL as far as the app state goes: `hide_sub_tools` (`gui_ribbon_align_actions.rs:143-153`) -> `stop_align_session` (`:86-93`) ends the session and returns to Pan; `gui_tab_home_survey.rs:124` resets align state on a new survey (C++ `MainWindowSession.cpp:263/601/707/766/1660`; the other call sites - starting to draw, switching sub-toolbars - not verified in Rust).

---

## Port order (small, individually testable steps)

Each step names the C++ test to mirror or a Rust unit test to write; later steps need earlier ones. Core crate for math/IO (`strata-geo`), UI in `strata-app`.

1. **Affine type + `from_pairs(pairs, source_y_down)`** in `strata-geo`: 2-pair Helmert with Y flip (`GS:53-77`), 3+ centred affine LSQ with `1e-10` relative singular guard and Helmert fallback (`GS:82-120`, `:160-176`), `rms_meters`, `transform`, `invert`. Tests: port `test_georef.cpp` cases `twoPointRaster_flipsY...`, `helmertTranslateScale`, `helmertRotate90`, `affineThreePoints`, `invertRoundTrip` and all of `test_georef_quality.cpp` fit cases (200000-class coordinates, collinear fallback).
2. **Quality report**: `assess`, `shape_of`, `row_text`, `notes`, `summary` with the exact texts and thresholds 1.0 m / 1.05 / 3.0 deg (`GQ:8-94`). Tests: `residuals_flagTheWrongPoint`, `threePoints_say...`, `anisotropyAndShear_areWarned`, `similarity_hasNoWarnings`. Then switch `status_text()` to it and delete the Helmert-based summary.
3. **World file + prj writer**: `world_file_path_for` (jgw/pgw/tfw/wld), `write_world_file` (A D B E C' F' with `{:.16g}`-equivalent formatting, centre-pixel shift, CRLF decision), `prj_path_for`, `write_sidecar_prj` (WKT of the project CRS from proj/georust). Test: `fitRasterToExtentWorldFile`, and read the file back with GDAL (`gdal` crate) to compare extent.
4. **`looks_unreferenced_raster`** (extent rules `GS:612-631`) over a `gdal` dataset: tests `pngWithoutWorldFileLooksUnreferenced`, `geotiffEmbeddedGtIsReferenced`, and the 1 m/pixel-far-from-origin case.
5. **Backup / restore** (`GB:145-277`): `정합백업/<base>_<yyyyMMdd-HHmmss>[_N]`, sidecar copies, `backup.json`, internal GeoTIFF GT/GCP save + `GDALSetGeoTransform`/`SetGCPs` restore, PAM drop rule, `unique_output_path`. Tests: `test_georef_backup.cpp` all six (plain scan leaves no trace, GeoTIFF restore, PAM with georef, unique path, root inside survey folder, `readPixelWindow`).
6. **`persist_aligned_raster`**: backup -> world file -> prj -> PAM drop -> GeoTIFF GT stamp -> reopen -> verify not unreferenced (`GS:290-341`). Tests: `persistAlignedRaster_keepsSameLayer...`, `persistAlignedRaster_jpegGetsMapExtentOn5187`, `rasterRestore_afterMove_putsTheImageBack`, `rasterRestore_beforeAnyWrite_saysNothingChanged`.
7. **Picture style**: `knock_out_raster_paper` (white, tolerance 16, `BasemapOps.cpp:193-215`), `looks_like_paper_scan` (32x32 block, >= 42 % >= 236), ink scan style and colour style (`GS:563-610`) as renderer settings of the Rust map. Tests: `knockOutRasterPaperMakesWhiteTransparent`, `styleAlignedRasterOverlay_*` (adapt to Rust renderer parameters).
8. **Load a picture without coordinates** (`addRasterFromPath`, section 8.1): align-pending flag, hidden in the legend, excluded from paint order, the two status texts, the TIF knock-out. Test: load a PNG, check flag + hidden + status; load an embedded-GT GeoTIFF, check flag cleared.
9. **Entry**: replace `alignable_layer_name` with the real alignable test (raster gdal-like, vector file/memory, not domain layer), add the 「맞출 도면」 file dialog with the exact title/filter, the CAD vs picture branch, the failure warning card and the final information box (`MWA:844-880`). Test: with no alignable layer the dialog is requested; with one, a session starts directly.
10. **Session view, picture only**: split view with the picture in pixel coordinates (zoom 1.15 per wheel step, middle-button pan, fit on load), left-then-right click order, `set_source_point`, pair append, Esc/right-click cancel, snap on the map side, `statusText` driven live. Tests: pair-placing state machine (`WaitFrom/WaitTo`, `removeLastPair`, `removePairAt` invalidating the affine, Esc), click on a mark with no movement still picks (`test_align_image_view.cpp`).
11. **Drawing of the session**: pending/finished marks (colours, sizes, numbers), right-canvas cross marks, link overlay arrows (blue done, orange dashed live, numbered circles at both ends), point list rows + notes + `왼쪽만 —` row. Visual check against the C++ app; unit-test the data feeding them (rows/notes). In this step also confirm by running the C++ app the exact stacking effect of `insertAt` (`MWA:654-663`) - the report could not settle top/bottom by reading alone.
12. **Drag fine-tuning**: right-mark drag (10 px grab, 3 px slop) and left-mark drag, with the two status texts (`KATE:43-112`).
13. **이동 for pictures**: wire `apply_align_move` -> `persist_aligned_raster`, clear pending, overlay style, reorder before basemap/위성/지적, zoom with the 1.25 margin only when on-map, completion status with backup path, the `KaUserError`-style card for errors, hide the left pane but keep the session (`MWA:627-703`, `KAT:345-416`).
14. **되돌리기 and 맞추기 저장 for pictures**: real restore with the exact message table (5.2), re-show the left pane, flags/opacity; save = persist again + `맞춰 두었습니다…` + `맞춤 저장: <world file path>`; keep the session open for pictures (only end it for CAD).
15. **Vector clone alignment**: `clone_to_memory`, originals snapshot, `apply_affine_to_vector` from originals (non-cumulative), restyle (blue 0.9 / 4.0), restore from originals, `_aligned.gpkg` copy with unique naming and CRS transform. Tests: `vectorAffineDoesNotNeedControlPoints`, `vectorSave_keepsAnOlderAlignedFile`, `vectorRestartOnTheClone_keepsOneClone`.
16. **CAD drawing alignment**: import hook starting the session when no CRS, `align_layer_of` order lines/fills/points/texts, hide companions, `to_drawing_crs`, `apply_affine` over all tables incl. text angle/height, all-or-nothing rollback, no aligned copy, session end and restore of visibility on cancel. Tests: all of `test_cad_align.cpp`.
17. **Large-picture preview** (4000 px cap, detail re-read of the visible window from the original with 200 ms debounce, 4096x4096 cap, original-pixel marks). Tests: `test_align_image_view.cpp` (`zoomedPreview_readsOriginalWindow`, `zoomedOutPreview_readsNothing`, `dragMark_reportsOriginalPixels`).
18. **Cleanup of fakes**: remove/replace the wrong tool hint (`ribbon_state.rs:128-130`, `map_tools.rs:39`), drop `find_subpixel_crosshair_center_15x15`/grade code from the align path (no C++ counterpart), stop marking the survey dirty on 이동 unless the owner decides otherwise; add the Ctrl+Z, Delete-in-list and session-end hooks (`MainWindowUndo.cpp:334`, `MainWindow.cpp:259-268`, the `stopAlignSession` call sites in 9).
