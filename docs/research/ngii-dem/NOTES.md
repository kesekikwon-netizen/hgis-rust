# NGII public DEM — what the public pages of map.ngii.go.kr say (read 2026-11-15, no login)

Fetched once each with a browser User-Agent: NlipMap.do?tabGb=total (HTML) and four static scripts
(integSearch.js, download.js, measure.js, csQuery.js). Files are beside this note. Nothing was ordered and no
account was used. This is for order F3 (DEM button). The C++ app has no code for this: its DEM download is
Copernicus GLO-30 and its NGII path is a manual file import (MainWindowRibbon.cpp:405-413).

## Facts

- The integrated search has a product 「공개DEM」 next to 수치지도. Product numbers of `downloadMapData(mapType, …)`
  (download.js:140-160): 1 수치지도, 2 항공사진, 3 정사영상, **4 공개DEM**, 5 측량기준점, 6 온맵.
- Search: `POST /ms/map/demQuery.do` with the same search parameters as the other products
  (`mapQuery.getParams()`: the area of `combineBufferQuery.do`) plus `toYear`, `fromYear` (csQuery.js:990-1000).
  Years offered: `demYearsQuery.do` (csQuery.js:2448). Count: `demCountQuery.do`.
- A result row (integSearch.js:4077-4090) is a checkbox `input[name="cb_dem"]` with attributes
  `year, mapName, minx, miny, maxx, maxy, sigu, num`; items of the answer: `{year, nam, minx, miny, maxx, maxy, sigu, num}`.
- Order (download.js:1463-1552 `fnDownloadTritVidoDem`): after the same login check (`/mn/loginCheck.do`) and the same
  application popup, `POST /pd/shbtManage/shbtInsertTritVidoDem.do` with
  `downloadType` (`index` = by sheet, `bound` = by district), `mapNum`, `mapName`, `zoneYy` (year), `brthdy`,
  `purchsPurps`, `detailCn`, **`basis`** (상세용도: required for products 2, 3 and 4, download.js:457-466).
  Success: `resultCode === "1"` and `allDataMap.orderDownList` → `fnFileDownload(orderDownList, unityOrderNo)`,
  the same transfer popup as the topographic sheets.
- The example in the site's own comment (csQuery.js:2044): `types=index, years=2014, nums=37713, names=용인` —
  a five-digit sheet number, i.e. the DEM is offered per 1:50,000 sheet (the topographic download uses the
  six-digit 1:25,000 sheets, ngii-topographic.js).

## Consequence for the plan

The DEM order is the topographic order with product 4 and one more field. It needs the same hidden browser
session (login, application popup, transfer popup). So F4 (topographic sheets, complete C++ reference) is built
first and F3 (DEM) on top of it — the owner's wording fits: 「수치지형도와 같은 구역의 dem을 별도로」.

## Not known yet (needs a logged-in session, the owner's account)

- Grid size and file format of the delivered DEM (the C++ menu row names `.img`), CRS, whether one file per sheet.
- Whether a DEM order is delivered at once like the sheets or waits for approval.
- The exact rows `demQuery.do` returns for a 5 km circle (how many sheets and years).
