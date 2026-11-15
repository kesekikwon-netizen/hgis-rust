# Survey contour function of the C++ app: rules as found in the source

Source study of the C++ survey-contour function (read-only). The function does not read any layer of the project. The user picks a coordinate file (`*.dxf *.xlsx *.xls *.csv`) in the dialog, and nothing in the contour code touches the `control_points` layer. The `control_points` schema (`data/schemas/ka_hgis_layers.yaml:76`) has x/y/z fields, but no contour code refers to them.

Paths are relative to `%USERPROFILE%/Documents/hgis/target/cpp-ref/`. Abbreviations:
- `Reader` = `src/core/SurveyPointReader.cpp`
- `Arrange` = `src/core/SurveyPointArrange.cpp`
- `Schema` = `src/core/SurveyPointSchema.cpp`
- `Build` = `src/core/SurveyContourBuilder.cpp`
- `Write` = `src/core/SurveyContourWrite.cpp`
- `Style` = `src/core/SurveyContourStyle.cpp`
- `Win` = `src/app/MainWindowSurveyContour.cpp`
- `Dlg` = `src/app/KaSurveyContourDialog.cpp`

## Entries (`src/app/MainWindowRibbon.cpp`)
- DEM button menu row 「측량점으로 등고선…」 at :418.
- Own ribbon button 「등고선」 at :429-436, tooltip 「측량 DXF·엑셀 좌표로 등고선과 해발 색 구간을 만듭니다」.
- The `addWeb("contour", "국토정보맵 지형도")` at :701 is a different feature.

## 1. Input

1. Elevation comes from a DXF POINT's Z, or from the sheet's elevation column (Reader:197-225).
2. The sheet is the first layer with at least 3 fields (Reader:122-128). Columns are guessed from header names (Schema:isId/isNorth/isEast/isElev, case-insensitive, spaces and `_` removed):
   - id = 점번호, 측점, 점명, 포인트명, 번호, no, pt, id, name, point.
   - X = 북, 북쪽, x, x(n), n, north, northing.
   - Y = 동, 동쪽, y, y(e), e, east, easting.
   - Z = 표고, 높이, 해발, z, z(h), h, el, elev, elevation.
3. If a column is still missing and there are 3 or more fields, columns are taken by position: base = 1 if there are at least 4 fields and no id column, else 0; x = base, y = base+1, z = base+2 (Schema:guessSurveyColumns). Headerless sheets are handled (Schema:open, surveyNumericHeaders).
4. Numbers: commas are decimal or thousands separators (Schema:surveyParseNumber).
5. A skipped sheet row is counted and noted (at most 40 notes, `kMaxIssues`):
   - 「%1행: 빈 행」
   - 「%1행: X·Y가 숫자가 아닙니다.」
   - 「%1행: 표고가 숫자가 아닙니다.」
6. DXF handling:
   - POINT and MULTIPOINT count (the first point only). Other shapes are skipped with 「점이 아닌 도형 %1개는 건너뛰었습니다.」
   - If any CAD layer has a point with Z ≠ 0, layers with no such point are dropped: 「높이가 없는 점 레이어는 뺐습니다: …」 (Reader:161-174, 253-260).
   - Lines whose every vertex has a finite, non-zero Z (at least 2 vertices) are kept as possible breaklines; others are counted as `flatLineCount` (Reader:45-70).
   - The point name for DXF is the entity index as text.
7. Points are tagged "suspicious" if z == 0 or |z| ≥ 999 (Reader:234). They are used unless the 「의심점 빼기」 box is ticked (Win:85).
8. Duplicates: points are bucketed by x and y rounded to mm (Reader:244). The z values are averaged and the first point's name is kept. If the z values differ by more than 0.001, the note is 「같은 좌표(%1, %2)의 표고 %3개를 평균했습니다.」
9. Axes (Arrange:37-48):
   - The swap tick means "X is north".
   - It is pre-ticked when the headers say north/east (북/north/(n) with 동/east/(e)).
   - It is also pre-ticked when swapping the first row falls inside the Korea extent and |x−200000| > |y−200000| + 50000 (Reader:235-242).
10. CRS:
    - Points end up in the work CRS: `QgsProject` CRS, else EPSG:5186 (Win:118-119).
    - 「파일 좌표계」 offers 작업 좌표계와 같음, 5185 서부원점, 5186 중부원점, 5187 동부원점, 5179 UTM-K, 4326 경위도 WGS84 (Dlg:66-76).
    - The transform is `QgsCoordinateTransform` with an empty transform context. A point that fails is dropped: 「좌표계를 바꾸지 못한 점 %1개는 뺐습니다.」
    - A bad CRS gives 「좌표계 %1 을 읽지 못했습니다.」
11. Warnings only, nothing is dropped:
    - 「점 %1개가 %2 좌표계의 한국 범위 밖입니다. …」
    - 「측량점이 조사구역에서 약 %1 km 떨어져 있습니다. 파일 좌표계(원점)를 확인하세요.」 (gap ≥ 1000 m)
12. Minimum and straightness checks (Build:74-80, 16-35):
    - Fewer than 3 points: 「등고선을 만들려면 점이 3개 이상 필요합니다.」
    - Points within 0.05 m of a straight line (maximum perpendicular offset from the line through the first point and the farthest point): 「점이 일직선이라 삼각망을 만들 수 없습니다.」
13. Points outside the survey area are not removed before the surface is built. Only the finished lines and bands are clipped (item 4).

## 2. Surface (Write:79-127)

1. `GDALGridCreate(GGA_Linear, radius 0, NoData -9999, Float32)`. This is Delaunay linear interpolation, so the surface is the convex hull of the points, and cells outside it stay NoData.
2. Grid:
   - Extent is the point bounds (breakline nodes included) padded by half a cell (Build:201-202).
   - `nx = max(2, ceil(span/cell))`, same for `ny`.
   - Above `kMaxCells` = 4,000,000 cells: 「격자 칸이 너무 많습니다. 격자 크기를 키우세요.」
3. Cell size: the dialog value if above 0, else `autoCellSize` (Build:82-100). It samples at most 300 points, takes each one's nearest-neighbour distance (greater than 1e-6), and uses the median × 0.5 clamped to 0.05–1.0. If it cannot compute one, it uses 0.5. The dialog's 계산 격자 is 0 to 5 m, with 0 = 자동.
4. `fitCellSize` (Build:106-118) widens a cell that would exceed the cell limit, in whole centimetres. The note is 「점이 촘촘하고 범위가 넓어 계산 격자를 %1 m로 넓혔습니다.」
5. Breaklines are optional and come only from DXF lines (`SurveyContourTin.cpp:21-58`):
   - They are densified into extra TIN nodes spaced `max(cell, total/200000, 1e-3)` apart.
   - Nodes within 1 mm of an existing point are skipped.
   - They are not written to `survey_points` (row = −1).
6. Maximum edge (`SurveyContourTin.cpp:60-96`):
   - Only when `maxEdgeM` > 0 (default off).
   - It builds its own `GDALTriangulationCreateDelaunay`.
   - For each triangle whose longest edge is greater than the limit, and whose determinant is not below 1e-12 (flat triangles are skipped), it sets the cells whose centres fall inside to NoData.
   - Note: 「변이 %1 m보다 긴 삼각형 %2개는 비워 두었습니다.」 If GDAL has no triangulation: 「이 GDAL은 삼각망을 만들 수 없어 긴 삼각형을 비우지 못했습니다.」
7. The grid is flipped north-up and saved as `surface.tif`, with the options and NoData listed under Output.
8. Concave boundary rules other than the maximum edge: not in the source.

## 3. Contours

1. Tool: `GDALContourGenerateEx` with LEVEL_INTERVAL = intervalCm/100 (`.2f`), LEVEL_BASE = baseCm/100 (`.2f`, the dialog fixes baseCm = 0), NODATA -9999 (Write:188-198). Closed and open lines are whatever GDAL emits. Smoothing or simplification: not in the source. Minimum line length: not in the source.
2. Interval:
   - Default 0.5 m, range 0.1–100, step 0.1, one decimal (Dlg:35-41).
   - Interval below 10 cm: 「등고선 간격과 결과 폴더가 필요합니다.」
   - It is set to 0.1 automatically when the file's height range is under 2 m (200 cm) and the user has not touched it (Dlg:166).
3. Post-process each line (Write:236-252):
   - `cm = snap(elev, baseCm, intervalCm)` (rounded to the nearest step).
   - `mod = ((cm−base)/interval) % indexEvery`, negative corrected.
   - `index_line` = 1 when `mod` = 0. `indexEvery` is 5 (Dlg:247), so every 5th level is 계곡선 and the rest are 주곡선.
4. Label: `elev_label` = metres with 1 decimal if the interval is under 100 cm, else 0 decimals. Heights are stored in whole centimetres (`qRound(m*100)`, `SurveyContourMath.h`) so labels show no binary residue.
5. Colour bands:
   - Built if `colorBands` and max > min (Write:211).
   - `bandIntervalCm` = 0 means auto: `max(10, qRound(max(10, range_cm)/100.0)*10)` (Build:102-104).
   - `FIXED_LEVELS` runs from minCm in band steps for `ceil(range/band)` classes, with `POLYGONIZE=YES`.
   - A band failure is only a warning: 「색 구간 면을 만들지 못했습니다.」

## 4. Clip

- The clip area is the union (`combine`) of all `survey_area` layers' features, transformed to the work CRS (Win:36-58, 121, 131). It is not a buffer and not the hull.
- `clipSurveyLayerToWkt` (`SurveyContourClip.cpp:43-87`) is applied to `contour_lines` and `contour_bands` (Write:276-281).
- An invalid mask is repaired with `MakeValid`. Each feature gets `Intersection`; the result is split into line or polygon parts.
- The first part replaces the feature and extra parts become new features. Features with no part left are deleted.
- If GEOS fails on a feature, that feature is kept whole, and `clipped` becomes false.
- The `survey_points` layer and `surface.tif` are not clipped.
- With no survey area the whole result is kept, with the note 「조사구역이 없어 측량점 범위까지 그렸습니다.」
- Other notes (Build:58-68):
  - 「조사구역과 측량점이 겹치지 않아 남은 선이 없습니다. 파일 좌표계를 확인하세요.」 (all lines removed by the clip)
  - 「높이차가 등고선 간격보다 작아 선이 없습니다.」
  - 「조사구역 안에만 표시합니다.」
  - 「조사구역 모양으로 다 자르지 못해 일부는 측량점 범위까지 그렸습니다.」

## 5. Output

- Folder: `<folder of the .qgz>/측량등고선/<stem>/` (Win:128). `stem` is the file's base name with `\ / : * ? " < > |` replaced by `_`; an empty name becomes 「측량」.
- Writing: new files go to `_new` first. `installFiles` (Build:120-163) then moves them in. Old files are kept as `.old` and put back if any move fails, so an earlier result is replaced only on success.
- `contours.gpkg`, in this order, all in the work CRS (`OAMS_TRADITIONAL_GIS_ORDER`):
  - `survey_points`: Point25D; `name` string, `elev` real (m), `elev_cm` int. Only real points (row ≥ 0).
  - `contour_lines`: LineString; `id` int, `elev` real (snapped metres), `elev_cm` int, `elev_label` string, `index_line` int.
  - `contour_bands`: MultiPolygon; `elev_min`, `elev_max` real, `elev_min_cm`, `elev_max_cm` int, `elev_mid` real, `legend` string `"%1 – %2 m"`. Values are snapped to the band step with base 0.
- GPKG metadata items: `KA_HGIS_MIN_CM`, `KA_HGIS_MAX_CM`, `KA_HGIS_BAND_CM` (Write:283-285).
- `surface.tif`: Float32 GeoTIFF, `COMPRESS=DEFLATE TILED=YES PREDICTOR=3`, NoData -9999, north-up (Write:133-152).

## 6. Display (`Style:70-166`)

- The previous layers with the same `ka_hgis/contour_group` are removed first.
- Placement: group 「참조 지도」, sub-group 「측량 등고선 · <stem>」. Layers are inserted at index 0 one by one, so the tree order is 측량점, 등고선, 색 구간 (confirmed by the test).
- Every layer gets `LayerOps::markReferenceLayer` and the custom property `ka_hgis/contour_group` = the group title. Draw order beyond tree order: not in the source.
- 「색 구간」:
  - Graduated renderer on `elev_mid`.
  - One class per band, labelled `"%1 – %2 m"`.
  - No outline.
  - Colours come from a 16-stop ramp (#72c6b5, #81ceaa, #91d396, #a5d680, #bad975, #d0dc78, #e0da7b, #ebd27a, #edbe70, #e8a666, #d98b5a, #c47452, #ad6650, #b69886, #d1c5bb, #f4f3ef), interpolated at (i+0.5)/classes, alpha 170.
- 「등고선」: rule-based renderer.
  - 계곡선, `"index_line" = 1`: #3a2f24, 0.7 mm.
  - 주곡선, `"index_line" = 0`: #8a7a66, 0.25 mm.
- Labels:
  - Only the 계곡선 rule, named 「계곡선 표고」.
  - Expression `"elev_label"`, curved placement, repeat distance 80.
  - Malgun Gothic bold 8, colour (31,35,40), white halo 0.8 with alpha 230.
- 「측량점」: marker #5c6b73, size 1.6 mm, unchecked (hidden).
- After success: zoom to 「등고선」, project marked dirty, status message for 10 s: 「등고선 %1줄, 색 구간 %2개를 올렸습니다.」 plus any warning (Win:178-189).

## 7. Window (`Dlg`; title 「측량 등고선」, minimum width 460)

- 「좌표 파일」: label 「파일을 고르세요」 and button 「찾기…」. File dialog title 「측량 좌표」, filter 「측량 좌표 (*.dxf *.xlsx *.xls *.csv)」.
- 「등고선 간격」: spin box 0.1–100 m, default 0.5.
- 「축 순서」: tick 「X를 북쪽, Y를 동쪽으로 읽기」. Tooltip: 「측량 성과표는 보통 X가 북쪽입니다. 파일을 고르면 자동으로 정하고, 등고선이 대각선으로 뒤집혀 놓이면 이 칸을 바꾸세요.」
- 「이상한 점」: tick 「의심점 빼기」. It is disabled until suspicious points exist, then reads 「의심점 %1개 빼기」. Tooltip: 「표고가 0이거나 999 m 이상인 점은 오측일 때가 많습니다. 빼면 삼각망에 넣지 않습니다.」
- Foldable 「고급 설정」:
  - 「파일 좌표계」 (see item 10 of section 1). Tooltip: 「측량 파일이 작업 좌표계와 다른 원점으로 되어 있을 때만 고르세요. 점을 작업 좌표계로 바꿔 놓습니다.」
  - 「최대 삼각형 변」: 0–1000 m, 0 shows 「쓰지 않음」. Tooltip: 「변이 이보다 긴 삼각형은 비워 둡니다. 트렌치·구덩이 사이 빈 곳을 가로지르는 가짜 사면을 막습니다.」
  - 「단절선」: tick 「높이가 있는 DXF 선을 단절선으로 쓰기」, with count text 「높이가 있는 DXF 선 %1개를 단절선으로 쓰기」.
  - 「계산 격자」: 0–5 m, step 0.05, 0 shows 「자동」. Tooltip: 「등고선을 뽑는 표고 격자 크기입니다. 자동은 점 간격의 절반입니다.」
- Preview label starts as 「DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다.」 Afterwards it shows 「점 %1개, 표고 %2–%3 m. 등고선 약 %4줄.」 (lines ≈ (max−min)/interval), then the notes from `Arrange.cpp:94-132`. Above 500 lines it adds 「줄이 많습니다. 간격을 넓히세요.」
- Notes from `Arrange.cpp:94-132`:
  - 「X를 북쪽, Y를 동쪽으로 읽었습니다(%1).」
  - 「자동 판정은 X=북쪽(%1)이지만 지금은 X를 동쪽으로 읽습니다.」
  - 「X·Y 순서를 바꾸면 모두 범위 안에 듭니다.」 / 「X·Y 순서나 파일 좌표계를 확인하세요.」
  - 「표고가 0이거나 999 m 이상인 의심점 %1개. 오측이면 「의심점 빼기」를 켜세요.」
  - 「같은 좌표 %1곳은 표고를 평균했습니다.」
  - 「읽지 못한 행 %1개.」
  - 「높이가 있는 선 %1개는 단절선으로 쓸 수 있습니다.」
  - 「높이가 없는 선 %1개는 쓰지 않습니다.」
  - 「그 밖의 알림 %1개는 줄였습니다.」
- Buttons: 「등고선 만들기」 and 「취소」.
- Fixed values in `job()` (Dlg:237-255): `indexEvery` 5, `baseCm` 0, `colorBands` true, `bandIntervalCm` 0 (auto).
- Progress window (Win:132): 「측량점으로 등고선을 계산하고 있습니다.」 with 「취소」. QgsTask name 「측량 등고선」.

## 8. Stop messages (the warn box has four lines: 「측량 등고선」 / the message / the detail / the advice)

- No survey open (Win:107-110):
  - 「조사를 연 뒤에 등고선을 만들 수 있습니다.」
  - 「결과 파일은 조사 폴더 안에만 둡니다.」
  - 「조사 열기 또는 새 조사로 시작한 뒤 다시 누르세요.」
- A run is already going: 「등고선을 만들고 있습니다. 끝나거나 취소를 누르세요.」
- Dialog OK with no file: 「좌표 파일을 고르세요.」
- File read errors:
  - 「파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요.」
  - 「시트나 CAD 레이어가 없습니다.」
  - 「X·Y·표고 칸을 찾지 못했습니다. 칸을 직접 지정하세요.」 (the dialog offers no way to pick columns)
  - Row-level errors: see the sheet-row messages in section 1.
- Build failures:
  - Fewer than 3 points, collinear, and the interval error: see sections 1 and 3.
  - 「결과 폴더를 만들지 못했습니다.」
  - 「표고 격자를 만들지 못했습니다.」
  - 「표고 격자 파일을 쓰지 못했습니다.」
  - 「등고선 파일을 만들지 못했습니다.」
  - 「등고선을 만들지 못했습니다.」
  - Cancel: 「등고선 만들기를 취소했습니다.」
  - Failure box: 「등고선을 만들지 못했습니다.」 / the error / 「점 배치와 등고선 간격을 확인한 뒤 다시 시도하세요.」
- Install or show failure:
  - 「계산은 끝났지만 지도에 올리지 못했습니다.」
  - 「이전 결과 파일이 다른 곳에서 열려 있어 바꾸지 못했습니다. 이전 결과는 그대로 두었습니다.」
  - 「결과 파일을 조사 폴더에 두지 못했습니다. 이전 결과는 그대로 두었습니다.」
  - 「레이어 목록이 없습니다.」
  - 「참조 지도 그룹을 만들지 못했습니다.」
  - 「색 구간 레이어를 열지 못했습니다.」
  - Advice: 「이전 등고선을 다시 올려 두었습니다. 측량등고선 파일을 연 프로그램을 닫고 다시 만드세요.」 or 「조사 폴더의 측량등고선 파일을 다시 열어 보세요.」

## 9. Tests

- `tests/test_survey_contour.cpp`:
  - xlsx with a 북/동-style table suggests a swap, and 4 points are read.
  - DXF layers A/B give 10 points each, with elevations 1000 and 1090 cm present.
  - A plane `z = 10 + 0.01·x`, interval 10 cm, cell 1: the 1050 cm line has every x within 1.01 of 50. The 1020 line sits at lower x than the 1080 line.
  - A north slope puts the 1080 line north of the 1020 line.
  - Tenth-metre labels have `elev_cm % 10 == 0` and no "9999" or "0001".
  - Contours and bands stay within 31.5 m of the centre for a 30 m radius point ring (the hull).
  - Band labels equal `surveyBandLabel` and contain " – " and end with " m".
  - Reference group has exactly 측량점, 등고선, 색 구간, the points layer is unchecked, and none of the layers has a domain `layer_key`.
  - Source files' SHA-256 are unchanged after a run.
  - Bad rows: 「빈 행」 and 「표고가 숫자가 아닙니다」 notes, 1 duplicate group, 2 points kept. 2 points are refused (the error contains 「3개」). 3 collinear points are refused (error contains 「일직선」).
  - 20000 points must finish in under 100 s (field PC measured 76.5 s).
- `tests/test_survey_contour_clip.cpp`:
  - A clip to 0–40 square leaves max x between 0 and 40.05.
  - Columns `x(N)`, `y(E)`, `z(h)` suggest a swap; the points then have x > 148000 and y between 98000 and 99000.
  - DXF layers GPS_Code and GPS_PointName at z = 0 are dropped (3 points kept, the issues mention GPS_Code).
- `tests/test_contour_safety.cpp`:
  - `fitCellSize` is 0.5 for a 10×10 area, and 0.05 over 250×250 gets enlarged to a whole-centimetre value within the cell limit.
  - A wide dense input sets `cellEnlarged`, and the warning contains 「계산 격자」.
  - With `maxEdgeM` = 5, no lines between 10.55 and 11.95 m survive in the gap, lines below 10.50 m do, and the warning contains 「긴 삼각형」.
  - Breakline nodes: without the ridge `lineCount` = 0; with it, `lineCount` > 0, `pointCount` = 4, 4 survey-points rows.
  - Clip applied: `clipped` is true and the warning contains 「조사구역 안에만」. A broken WKT keeps everything (`clipped` false). A far-away polygon leaves 0 lines and the warning contains 「겹치지 않아」.
  - `installFiles` keeps the old files when a swap fails (Windows lock test) and cleans up `.old` after success.
  - `readResult` and `reapply` round-trip: min, max, band interval and counts match, and the group has 3 layers.

## Files read in full
- `src/core/`: `SurveyContourBuilder.cpp/.h`, `SurveyContourTin.cpp/.h`, `SurveyContourClip.cpp/.h`, `SurveyContourMath.h`, `SurveyContourStyle.cpp/.h`, `SurveyContourWrite.cpp/.h`, `SurveyPointReader.cpp/.h`, `SurveyPointArrange.cpp`, `SurveyPointSchema.cpp`
- `src/app/`: `MainWindowSurveyContour.cpp`, `KaSurveyContourDialog.cpp/.h`
- `tests/`: `test_survey_contour.cpp`, `test_survey_contour_clip.cpp`, `test_contour_safety.cpp`

## Files read in part
- `src/app/MainWindowRibbon.cpp`: lines 405-440 only.
- `tests/survey_contour_fixture.h`: roughly lines 1-178 (the `surveyGrid`, `buildSurvey`, `writeSurveyTable`, `writeSurveyDxf` helpers).
- `data/schemas/ka_hgis_layers.yaml`: lines 70-95.
- `src/core/BasemapOps.cpp`: only the `koreaExtentForCrs` wrapper (:2421).

## Not read
- `computeKoreaExtentForCrs` (the actual Korea bounds).
- `HeritageImport::referenceGroupName` (the test confirms the value 「참조 지도」).
- `LayerOps::markReferenceLayer`.
- `tests/test_survey_point_arrange.cpp` and `tests/test_survey_point_dialogs.cpp`, whose names do not contain "contour" but which probably pin the reader/arrange/dialog rules above.
- `SurveyPointSchema.h` and `SurveyPointArrange.h`.
- `CMakeLists.txt`, apart from grepping for the contour targets: `ka_survey_contour_tests` is defined at line 1185, and `test_contour_safety` is built elsewhere.
