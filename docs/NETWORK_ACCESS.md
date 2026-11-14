# Outbound Network Access Inventory (`NET1`)

This document records every outbound network request that C++ Strata (`target/cpp-ref/src/...`) and Rust Strata (`crates/...`) can build or send, the public automated-use rules published by each host's operator as read on `2026-11-14`, and every code path matching findings `a`–`f` from `ORDER.yaml`. All API keys, account identifiers, passwords, and per-run archive IDs are written as `<key>`, `<id>`, `<pw>`, and `<uuid>`.

---

## Summary Table

Counts in columns `a`–`f` give the number of Part 3 findings affecting each host (with finding IDs in parentheses for direct cross-reference).

| host | auth kind | a | b | c | d | e | f |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `api.vworld.kr` | `none` (`HEAD /`); `API key` (query `key=` / `KEY=`, WMTS path `/1.0.0/<key>/`) | 4 (`A1`–`A4`) | 6 (`B1`–`B5`, `B7`) | 4 (`C1`–`C3`, `C5`) | 5 (`D1`–`D5`) | 0 | 4 (`F3`, `F5`–`F7`) |
| `xdworld.vworld.kr` | `none` (2D tiles); `API key` (query `APIKey=`, uncalled Rust constant) | 2 (`A1`, `A2`) | 2 (`B1`, `B2`) | 2 (`C1`, `C2`) | 0 | 0 | 1 (`F7`) |
| `www.vworld.kr` | `ID login` (C++ session cookie jar); `none` (Rust / external browser link) | 1 (`A4`) | 1 (`B4`) | 0 | 3 (`D1`, `D4`, `D5`) | 0 | 1 (`F6`) |
| `nominatim.openstreetmap.org` | `none` | 0 | 1 (`B3`) | 0 | 0 | 0 | 1 (`F3`) |
| `tile.openstreetmap.org` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 0 |
| `basemaps.cartocdn.com` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 0 |
| `tile.opentopomap.org` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 0 |
| `copernicus-dem-30m.s3.amazonaws.com` | `none` | 0 | 1 (`B6`) | 0 | 0 | 0 | 1 (`F1`) |
| `gibs.earthdata.nasa.gov` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 1 (`F7`) |
| `mt1.google.com` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 0 |
| `server.arcgisonline.com` | `none` | 1 (`A1`) | 1 (`B2`) | 1 (`C2`) | 0 | 0 | 0 |
| `intranet.gis-heritage.go.kr` | `ID login` (C++ `QWebEngineView` session); `none` (Rust HTTP / browser link) | 1 (`A5`) | 1 (`B8`) | 1 (`C6`) | 3 (`D1`, `D4`, `D6`) | 1 (`E1`) | 1 (`F2`) |
| `map.ngii.go.kr` | `none` (WMS & 5 km sheet query); `ID login` (C++ `QWebEngineView` order flow) | 2 (`A1`, `A6`) | 2 (`B2`, `B9`) | 2 (`C2`, `C7`) | 2 (`D1`, `D4`) | 1 (`E2`) | 1 (`F4`) |
| `www.ngii.go.kr` | `ID login` (C++ `QWebEngineView` SSO form) | 0 | 0 | 0 | 3 (`D1`, `D4`, `D5`) | 1 (`E2`) | 0 |
| `nlippd.ngii.go.kr` | `ID login` (C++ `QWebEngineView` popup session) | 0 | 1 (`B9`) | 0 | 0 | 0 | 0 |
| `data.kigam.re.kr` | `none` (C++ public OWS); `API key` (query `KEY=`, uncalled Rust builder) | 1 (`A3`) | 1 (`B7`) | 1 (`C5`) | 2 (`D1`, `D4`) | 0 | 1 (`F5`) |
| `gis.naas.go.kr` | `none` | 1 (`A1`) | 2 (`B2`, `B7`) | 2 (`C2`, `C5`) | 0 | 0 | 1 (`F5`) |
| `soil.rda.go.kr` | `none` (external browser link only) | 0 | 0 | 0 | 0 | 0 | 0 |
| `hgis.history.go.kr` | `API key` (query `apiKey=`) | 1 (`A1`) | 1 (`B2`) | 2 (`C2`, `C4`) | 4 (`D1`, `D3`–`D5`) | 0 | 0 |
| `www.dbpia.co.kr` | `none` (scraped `X-JSP-KEY` + jar); `ID login` (C++ `QWebEngineView`) | 1 (`A7`) | 1 (`B10`) | 0 | 1 (`D6`) | 0 | 1 (`F7`) |
| `ai.dbpia.co.kr` | `ID login` (C++ shared `QWebEngineProfile`) | 0 | 0 | 0 | 1 (`D6`) | 0 | 0 |
| **Distinct findings (`A`–`F`)** | — | **7** | **10** | **7** | **6** | **2** | **7** |

---

## Part 1 — Outbound Requests by Host

### Transport Architecture Note

- **C++ (`target/cpp-ref/src/...`)**: Uses Qt `QNetworkAccessManager` (direct REST/WFS/portal calls), QGIS `QgsNetworkAccessManager` (`wms` provider for XYZ/WMS/WMTS layers, with a global preprocessor at `target/cpp-ref/src/app/KaApplication.cpp:831-842` and `qgis/defaultTileMaxRetry = 6` at `:873`), GDAL (`GDAL_WMS` XML/TMS and `/vsicurl/`), Qt `QWebEngineView` (`KaHeritageBrowser`, `KaTopographicBrowser`, `KaHwpReportStudio`), and `QDesktopServices::openUrl` (`MainWindowRibbon.cpp:695-702`).
- **Rust (`crates/...`)**: Uses zero Rust HTTP crates (`reqwest`, `ureq`, `hyper`, and `TcpStream` do not appear in `Cargo.toml` or `crates/`). Every network call shells out to Windows `curl.exe` via `Command::new("curl")` / `Command::new("curl.exe")` (`crates/strata-net/src/internet_check.rs:13`, `crates/strata-app/src/basemap_fetch.rs:120`, `crates/strata-app/src/basemap_cadastral.rs:191`) or `curl_stdin::output_with_url_on_stdin` (`url = "..."` piped via `-K -` on stdin at `crates/strata-net/src/curl_stdin.rs:9-31` so `<key>` does not appear on the Windows process command line). The only other `Command::new` in `crates/` is `Command::new("powershell")` for local `Expand-Archive` ZIP extraction (`crates/strata-app/src/modal_topo_rows.rs:121`).
- **Uncalled URL builders / dead clients audited**:
  - `crates/strata-net/src/vworld_wms.rs:10-27` (`http://xdworld.vworld.kr:8080/XDServer/requestLayerNode?...&APIKey=<key>` and `https://api.vworld.kr/req/wms` helper): no HTTP caller in `crates/`.
  - `crates/strata-net/src/vworld_tiles.rs:28-46` (`Hybrid` WMTS tile URL builder): `basemap_tiles.rs` builds `Satellite` and `Base` directly; `Hybrid` is uncalled.
  - `crates/strata-net/src/vworld_heritage.rs:19-44` (`api.vworld.kr/req/address` reverse-geocode URL builder): uncalled in `strata-app`.
  - `crates/strata-net/src/kigam_soil_river.rs:10-79` (`data.kigam.re.kr/openapi/wfs`, `gis.naas.go.kr`, `api.vworld.kr/req/wfs` URL builders): uncalled in `strata-app` (`modal_downloads.rs:83` loads local sample data instead).
  - `crates/strata-app/src/tile_layers.rs:42-92`: tile layer metadata catalogue only; no HTTP calls.
  - `target/cpp-ref/src/core/HeritageHttpClient.cpp:22-201`: instantiated at `KaHeritageBrowser.cpp:231`, but `fetchDownloadPage()` and `searchRegion()` have no callers in C++.

---

### 1. `api.vworld.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 홈 화면 「인터넷 확인」 | `HEAD /` | `none` | C++: none (`QNetworkRequest` defaults); Rust: none (`curl.exe` defaults) | 1 (`m_internetButton` disabled while in flight in C++; 1 spawned thread per click in Rust) | No delay; 1 request only when user clicks 「인터넷 확인」 (`1 × HEAD = 1`) | C++: `6000 ms`; Rust: `--max-time 6` (`6 s`) | 0 | C++: any HTTP status (`> 0`) or `NoError` shows 「인터넷 · 연결됨」, timeout/DNS failure shows 「인터넷 · 연결 안 됨」 (nothing logged); Rust: any `%{http_code}` `> 0` (including `404`/`500`) marks `Done(true)`, `000`/spawn error marks `Done(false)` | none | `target/cpp-ref/src/app/KaHomeConnectionCard.cpp:67-75,188-207` | `crates/strata-net/src/internet_check.rs:7-25`, `crates/strata-app/src/home_connection_model.rs:69-79`, `crates/strata-app/src/snap_shot_run.rs:127` | C++ uses `QNetworkAccessManager::head` on the Qt event loop; Rust spawns a background thread running `curl.exe -s -I` |
| 리본 「주소·지번·장소 검색」 / 「지번 검색」 | `GET /req/search?service=search&request=search&version=2.0&crs=EPSG:4326&size={12\|1000}&page=1&query=<q>&type={ADDRESS\|place}[&category={PARCEL\|ROAD}]&format=json&errorformat=json&key=<key>` | `API key` (query `key=<key>`) | C++: `"User-Agent": "ka-hgis/0.3"`; Rust: `-A "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `-H "Accept: application/json"` | 1 (`m_pending` guard in C++; blocking UI call in Rust) | 0 ms delay before fallback: sends `1` VWorld request (`PARCEL` with `size=1000`, `ROAD` with `size=12`, or `place` with `size=12`) + `1` Nominatim fallback when `place` fails or returns 0 hits (`1..2` total requests per search) | C++: `20000 ms` shared deadline; Rust: `--max-time 15` (`15 s`) per call | 0 (falls back immediately to Nominatim on `place` failure) | On `parcel`/`road` HTTP error or JSON `status != "OK"`, both apps abort and show Korean notice (e.g. 「지번 검색 서버가 정상 결과를 보내지 않았습니다. VWorld API 키와 서비스 상태를 확인하세요.」); on `place` error or empty hits, both immediately query Nominatim | none | `target/cpp-ref/src/core/LocationSearch.cpp:120,168-226,292-411` | `crates/strata-net/src/address_geocode.rs:13-41,145-199`, `crates/strata-net/src/address_vworld_search.rs:17-219`, `crates/strata-app/src/gui_ribbon_search.rs:61-107` | C++ runs asynchronously under one 20 s `QTimer` deadline; Rust blocks the UI thread via `curl -K -` with 15 s per request (and also falls back to Nominatim when VWorld `place` returns `Ok` with 0 hits) |
| 「조사구역 경계 받기」 읍·면·동 경계 조회 | `GET /req/data?service=data&version=2.0&request=GetFeature&data=LT_C_ADEMD_INFO&key=<key>&domain=https://localhost&format=json&geometry=true&attribute=true&crs=EPSG:4326&size={10\|20}&attrFilter=<filter>` | `API key` (query `key=<key>`, plus `domain=https://localhost`) | C++: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `"Referer": "https://localhost"` | 1 (`cancel()` aborts prior reply) | 0 ms delay between fallback filters: tries `emd_kor_nm:=:<dong>` → `full_nm:like:<sido sigungu dong>` → `emd_kor_nm:like:<dong>` (`1..3` requests per lookup) | `15000 ms` (`kTimeoutMs`) per request | 0 per filter (chains up to 3 filters) | HTTP error, empty body, or JSON `status != "OK"` advances to next filter (`NOT_FOUND`) or emits `failed(...)` with Korean message; nothing written to session log unless caller logs | none | `target/cpp-ref/src/core/AdminBoundaryService.cpp:25-177` | `crates/strata-net/src/vworld_admin.rs:97-138` (URL/fetch helper exists; `LT_C_ADEMD_INFO` not wired in UI) | C++ runs up to 3 fallback `req/data` queries with `Mozilla/5.0` + `Referer`; Rust has an uncalled `AdminLayer::Emd` helper using honest `ka-hgis/0.3` UA and no `Referer` |
| 「지적도 다운로드」 시·군·구 코드 조회 | `GET /req/data?service=data&version=2.0&request=GetFeature&format=json&data=LT_C_ADSIGG_INFO&size=100&crs=EPSG:4326&geometry=true&attribute=true&page=<1..10>&key=<key>&geomFilter=BOX(<minLon>,<minLat>,<maxLon>,<maxLat>)` | `API key` (query `key=<key>`) | C++: `"User-Agent": "Mozilla/5.0 ka-hgis"`, `"Referer": "https://localhost"`; Rust: `-A "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `-H "Accept: application/json"` | 1 worker thread | 0 ms delay between pages; `page = 1..=10` (`size=100`). Typical single-survey 5 km bbox intersects 1–3 시·군·구 (`1` page = `1` request; worst case `10` requests) | C++: `40000 ms` idle & deadline; Rust: `--max-time 15` (`15 s`) | 0 | C++: HTTP error, non-JSON, or JSON `status != "OK"` aborts job and shows 「VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요.」; Rust: curl/parse failure on page 1 aborts worker and shows Korean error in cadastral modal | none (re-queried on every run) | `target/cpp-ref/src/core/CadastralPortal.cpp:45-56,231-262`, `target/cpp-ref/src/app/MainWindowCadastral.cpp:95-155` | `crates/strata-net/src/vworld_admin.rs:97-159`, `crates/strata-app/src/cadastral_download_ui_worker.rs:52-90` | C++ sends `Mozilla/5.0 ka-hgis` UA and `Referer: https://localhost` with 40 s timeout; Rust sends honest `ka-hgis/0.3` UA via `curl -K -` with no `Referer` and 15 s timeout |
| 「주변유적 받기」 중심점 역지오코딩 · 5 km 인접 시·군 조회 | `GET /req/address?service=address&request=getAddress&version=2.0&crs=EPSG:4326&point=<lon>,<lat>&format=json&type=PARCEL&zipcode=false&simple=false&key=<key>` and `GET /req/data?service=data&version=2.0&request=GetFeature&data=LT_C_ADSIGG_INFO&key=<key>&domain=https://localhost&format=json&geometry=false&attribute=true&crs=EPSG:4326&size={10\|100}&page=<1..5>&geomFilter={POINT(...)\|BOX(...)}` | `API key` (query `key=<key>`, plus `domain=https://localhost` on `/req/data`) | C++: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `"Referer": "https://localhost"` | 1 resolver + 1 nearby-region job | 0 ms delay. `HeritageRegionResolver` runs `getAddress` then falls back to `LT_C_ADSIGG_INFO` `POINT` (`1..2` requests); `HeritageNearbyRegions` pages `BOX` up to `kMaxPages = 5` (`1` request typical, `2..7` combined) | Resolver: `8000 ms`; Nearby: `10000 ms` | 0 | HTTP/JSON error on `getAddress` triggers `POINT` fallback; failure of both leaves region picker manual and shows inline hint in `KaHeritageRegionDialog` | none | `target/cpp-ref/src/core/HeritageRegionResolver.cpp:69-262`, `target/cpp-ref/src/core/HeritageNearbyRegions.cpp:67-186` | `crates/strata-net/src/vworld_heritage.rs:19-44` (URL builder only; uncalled) | Active in C++ when opening `KaHeritageRegionDialog`; Rust builds URL in `vworld_heritage.rs` only and does not call it |
| 배경지도 「위성지도」 / 「일반지도」 / 「하이브리드」 WMTS 타일 (및 「오프라인 지도 팩」 / 인쇄) | `GET /req/wmts/1.0.0/<key>/{Satellite\|Base\|Hybrid}/{z}/{y}/{x}.{jpeg\|png}` | `API key` (URL path `<key>`) | C++ QGIS: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"`, `"Referer": "https://localhost"`; C++ `TilePackService` (GDAL TMS): `<Referer>https://localhost</Referer>` + GDAL default UA; Rust: `-H "Referer: https://localhost"`, `-H "User-Agent: Mozilla/5.0"` | C++ QGIS: ~6/host (`QNetworkAccessManager`); C++ `TilePackService`: `4` (`<MaxConnections>4</MaxConnections>`); Rust: `1` synchronous UI-thread curl at a time | 0 ms pause between tiles. Typical 1920×1080 viewport = `6 × 4 = 24` to `8 × 6 = 48` tiles (`48` requests, or `96` when `Hybrid` overlays `Satellite`). Rust skips painting only above `count > 256` tiles/frame. C++ offline pack downloads up to `20,000` tiles | C++ QGIS: `15000 ms`; C++ `TilePackService`: `30 s` (`<Timeout>30</Timeout>`); Rust: `--max-time 12` (`12 s`) | C++ QGIS: `defaultTileMaxRetry = 6`, 0 ms delay, including 4xx; C++ `TilePackService`: 0 retries (`<ZeroBlockHttpCodes>204,404</ZeroBlockHttpCodes>`); Rust: **unlimited** — failed/4xx/XML tiles are not cached and re-run on **every egui paint frame** with 0 ms delay | C++ QGIS: logs `[qgis/WMS] ...` (`SecretMask::mask` redacts `<key>` only if GUID-shaped), leaves blank canvas tiles; `TilePackService` aborts `GDALTranslate` on server error (`<ZeroBlockOnServerException>false</ZeroBlockOnServerException>`). Rust: deletes non-image files (including VWorld HTTP 200 XML `<ServiceExceptionReport>`), draws subtle grid fallback, logs nothing, and retries next frame | C++: `QNetworkDiskCache` `2 GB` (`%LOCALAPPDATA%` or `cache/tiles`, HTTP-header expiry) + `<survey>/배경지도_오프라인.mbtiles` (permanent); Rust: `%LOCALAPPDATA%\ka-hgis\tiles\{satellite\|base}\{z}\{x}\{y}.{jpeg\|png}` (never expires) + unbounded RAM `HashMap` | `target/cpp-ref/src/app/KaApplication.cpp:813-842,873`, `target/cpp-ref/src/core/BasemapOps.cpp:1048-1070,1141-1187,1457-1470`, `target/cpp-ref/src/core/TilePackService.cpp:54-258` | `crates/strata-app/src/basemap_tiles.rs:28-48`, `crates/strata-app/src/basemap_fetch.rs:14-129`, `crates/strata-app/src/basemap_paint.rs:86-161` | C++ uses async QGIS WMTS (`Satellite`, `Base`, `Hybrid`) with 6 retries + 2 GB HTTP cache; Rust fetches `Satellite`/`Base` synchronously on the UI thread (no `Hybrid`) with bare `Mozilla/5.0` UA, permanent file cache, and infinite per-frame retry on failure |
| 「연속지적도」 / 「지적도(VWorld)」 및 「VWorld 등고선」 WMS 격자 | `GET /req/wms?KEY=<key>&SERVICE=WMS&REQUEST=GetMap&VERSION=1.3.0&LAYERS={lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun\|lt_c_upisuq}&STYLES=...&CRS=EPSG:3857&BBOX=<minx>,<miny>,<maxx>,<maxy>&WIDTH=512&HEIGHT=512&FORMAT=image/png&TRANSPARENT=true` (C++ GDAL_WMS fallback uses `key=<key>`, `EPSG:3857`, `512×512`) | `API key` (query `KEY=<key>` or `key=<key>`) | C++ QGIS WMS: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"`, `"Referer": "https://localhost"`; C++ GDAL_WMS: `<UserAgent>Mozilla/5.0 ka-hgis/0.3</UserAgent>`, `<Referer>https://localhost</Referer>`; Rust: `-H "Referer: https://localhost"`, `-H "User-Agent: Mozilla/5.0"` | C++ QGIS: ~6; C++ GDAL_WMS: GDAL default (1 per render read); Rust: `MAX_IN_FLIGHT = 6` threads | 0 ms pause between tiles. Typical zoomed-in (`z >= 16`) 1920×1080 viewport = `4 × 3 = 12` to `7 × 5 = 35` 512 px tiles (`12..35` requests; Rust caps at `MAX_CADASTRAL_TILES = 96` per paint) | C++ QGIS: `15000 ms`; C++ GDAL_WMS: GDAL default (`0` retries); Rust: `--max-time 15` (`15 s`) | C++ QGIS: `6` retries, 0 ms delay; C++ GDAL_WMS: 0; Rust: **unlimited** — retries failed/4xx/XML tiles every `RETRY_AFTER = 30 s` forever while visible | C++: logs `[qgis/WMS]` warning (key masked) and renders empty overlay. Rust: checks PNG magic `\x89PNG` (catching HTTP 200 XML key/quota errors), records `st.failed.insert(key, Instant::now())` without user error dialog or log, and re-queues every 30 s | C++: `2 GB` `QNetworkDiskCache`; Rust: `%LOCALAPPDATA%\ka-hgis\tiles\cadastral\{z}\{x}\{y}.png` (forever) + `MAX_CADASTRAL_TEXTURES = 160` RAM cache | `target/cpp-ref/src/core/BasemapOps.cpp:1189-1285,1472-1480`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `crates/strata-app/src/basemap_cadastral.rs:128-200`, `crates/strata-app/src/basemap_cadastral_jobs.rs:18-129`, `crates/strata-app/src/gui_tab_map_basemap.rs:81-154` | C++ uses QGIS WMS (plus `lt_c_upisuq` contour WMS) or `vworld-cadastral.xml` GDAL_WMS (writing `<key>` to XML on disk); Rust spawns 6 background `curl` threads for 512×512 PNG tiles and retries failures every 30 s indefinitely |
| 「수계도」 하천망 WFS 내려받기 | `GET /req/wfs?SERVICE=WFS&REQUEST=GetFeature&VERSION=2.0.0&TYPENAME=lt_c_wkmstrm&OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES=1000&STARTINDEX=<idx>&BBOX=<minLat>,<minLon>,<maxLat>,<maxLon>,urn:ogc:def:crs:EPSG::4326&KEY=<key>` | `API key` (query `KEY=<key>`) | C++: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `"Referer": "https://localhost"` | 1 (`m_referenceDownload` single-job guard) | 0 ms pause between pages and quadtree pieces. 1 survey bbox = `1` piece × `1` page (`1` request) typical; on `TooLarge`/`Slow` splits 2×2 up to `maxDepth = 2` (`1 + 4 + 16 = 21` piece calls × up to `kMaxPages = 10` = up to `210` requests worst case) | `15000 ms` idle / `180000 ms` per request (`kLargeTransfer`), `300000 ms` (`5 min`) overall job deadline | `retriesPerPiece = 1` immediate retry (`0 ms` delay) on `Transient` or `Slow`, plus quadtree split on `TooLarge`/`Slow` | HTTP 401/403 or XML key rejection shows 「VWorld 인증키가 거부되었습니다. 배경지도 설정에서 키를 확인한 뒤 다시 내려받으세요.」; 429/5xx triggers 1 immediate retry then aborts; logged via `KaReferenceDownloadJob` | Saved to `<survey>/수계도_VWorld.gpkg` (permanent file, re-fetched if layer is not currently in `QgsProject`) | `target/cpp-ref/src/core/RiverMapService.cpp:103-190`, `target/cpp-ref/src/core/PreparedReferenceMap.h:47-55`, `target/cpp-ref/src/core/PreparedReferenceMap.cpp:62-154`, `target/cpp-ref/src/core/ReferenceTiledFetch.cpp:24-150`, `target/cpp-ref/src/app/MainWindowOverlay.cpp:550-601` | `crates/strata-net/src/kigam_soil_river.rs:60-79` (URL builder only; uncalled) | Implemented end-to-end in C++; Rust only has an unused URL builder and loads offline sample layers in `modal_downloads.rs:83` |

---

### 2. `xdworld.vworld.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| API 키 없을 때(또는 WMTS 폴백) 배경지도 「위성지도」 / 「일반지도」 / 「하이브리드」 2D 타일 (및 「오프라인 지도 팩」) | `GET /2d/{Satellite\|Base\|Hybrid}/service/{z}/{x}/{y}.{jpeg\|png}` | `none` | C++ QGIS: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"`, `"Referer": "https://localhost"`; C++ `TilePackService`: `<Referer>https://localhost</Referer>`; Rust: `-H "Referer: https://localhost"`, `-H "User-Agent: Mozilla/5.0"` | C++ QGIS: ~6; C++ `TilePackService`: `4`; Rust: `1` synchronous UI-thread curl | 0 ms pause between tiles; typical 1920×1080 viewport = `24..48` tiles (`24..48` requests); Rust skips above `256` tiles/paint; C++ offline pack cap `20,000` tiles | C++ QGIS: `15000 ms`; C++ `TilePackService`: `30 s`; Rust: `12 s` | C++ QGIS: `6` retries (`defaultTileMaxRetry`), 0 ms delay; C++ `TilePackService`: 0; Rust: **unlimited** per-frame retry (0 ms delay) | Same as `api.vworld.kr` WMTS tiles: C++ logs `[qgis/WMS]` warnings and leaves empty tiles; Rust deletes non-image bodies, renders fallback grid, and re-fetches on the next paint frame | C++: `2 GB` `QNetworkDiskCache` + `.mbtiles` pack; Rust: `%LOCALAPPDATA%\ka-hgis\tiles\{satellite\|base}\{z}\{x}\{y}.{jpeg\|png}` (forever) + RAM `HashMap` | `target/cpp-ref/src/core/BasemapOps.cpp:1056-1059,1170-1175,1465-1468`, `target/cpp-ref/src/core/BasemapPolicy.h:129-131`, `target/cpp-ref/src/app/KaApplication.cpp:831-842,873`, `target/cpp-ref/src/core/TilePackService.cpp:54-258` | `crates/strata-app/src/basemap_tiles.rs:33,41`, `crates/strata-app/src/basemap_fetch.rs:14-129`, `crates/strata-app/src/basemap_paint.rs:86-161` | Both apps fall back to keyless `xdworld.vworld.kr` when VWorld API key is absent and still attach `Referer: https://localhost`; Rust runs synchronously on UI thread and retries failed tiles every frame |
| 미연결 3D XDServer 레이어 URL 빌더 (호출 없음) | `GET http://xdworld.vworld.kr:8080/XDServer/requestLayerNode?APIKey=<key>&Layer=<layer>&Level=<z>&IDX=<x>&IDY=<y>` | `API key` (query `APIKey=<key>`) | none (URL builder only) | 0 (never called) | Never called (`0` requests) | n/a | n/a | n/a | none | `none` | `crates/strata-net/src/vworld_wms.rs:10-27` | Present only as an uncalled Rust constant/builder over plaintext `http://` port `8080` |

---

### 3. `www.vworld.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「지적도 다운로드」 공간정보 오픈마켓 목록 조회 | `GET /dtmk/dtmk_ntads_s002.do?dsId=30563[&datPageIndex=<1..10>&datPageSize=100]` | C++: `ID login` (pre-login seed GET then authenticated `QNetworkCookieJar` for 1 job); Rust: `none` | C++: `"User-Agent": "Mozilla/5.0 ka-hgis"`, `"Referer": "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"`; Rust: `-A "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `-H "Accept: application/json"` | 1 worker thread | 0 ms pause between pages. C++: `1` pre-login seed GET + `1..10` paged GETs until a page has `< 100` rows (`229` nationwide 시·군·구 files = `1 + 3 = 4` requests typical, `11` max). Rust: `1..10` paged GETs (`3` typical, `10` max) | C++: `40000 ms` idle & deadline per page; Rust: `--max-time 15` (`15 s`) | 0 | C++: HTTP error or `>= 10` full pages aborts job and shows Korean portal failure message; Rust: curl error or 0 parsed resources shows `NO_ACCOUNT_MESSAGE` in cadastral modal | none (HTML catalog is scraped fresh on every run) | `target/cpp-ref/src/core/CadastralPortal.cpp:45-56,265-267,294-305` | `crates/strata-net/src/vworld_cadastral.rs:35-39,273-279`, `crates/strata-app/src/cadastral_download_ui_worker.rs:94-118` | C++ seeds cookies, logs in, spoofs `Mozilla/5.0 ka-hgis` + `Referer`; Rust skips login, sends `Accept: application/json` to an HTML `.do` endpoint with honest UA and no cookies |
| 「지적도 다운로드」 포털 로그인 | `POST /v4po_usrlogin_a004.do` (body: `usrIdeE=base64(<id>)&usrPwdE=base64(<pw>)&nextUrl=%2Fdtmk%2Fdtmk_ntads_s002.do%3FdsId%3D30563`) | `ID login` (obtains session cookies into stack-local `QNetworkCookieJar` inside `Session` for the duration of one download run) | `"User-Agent": "Mozilla/5.0 ka-hgis"`, `"Referer": "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"`, `"Content-Type": "application/x-www-form-urlencoded"`, `"X-Requested-With": "XMLHttpRequest"` | 1 | 1 POST per user-initiated download job (`1` request) | `40000 ms` idle & deadline | 0 automatic retries | HTTP error, non-JSON, or `resultMap.result != "success"` sets `result.accountRejected`, stops immediately with Korean guidance, and `MainWindowCadastral.cpp:142-154` opens the credential dialog for user input (0 auto-relogins) | In-memory `QNetworkCookieJar` for 1 job only; discarded when `prepare()` returns | `target/cpp-ref/src/core/CadastralPortal.cpp:45-56,268-292`, `target/cpp-ref/src/app/MainWindowCadastral.cpp:104,142-154` | `none` (`cadastral_creds` stored in `crates/strata-app/src/dialogs_account.rs:16`, never sent) | C++ logs in via `v4po_usrlogin_a004.do`; Rust has not ported login yet, so authenticated ZIP download cannot succeed |
| 「지적도 다운로드」 시·군·구 연속지적도 ZIP 내려받기 | `GET /dtmk/downloadResourceFile.do?ds_id=30563&fileNo=<fileNo>` | C++: `ID login` (`QNetworkCookieJar` from `v4po_usrlogin_a004.do`); Rust: `none` | C++: `"User-Agent": "Mozilla/5.0 ka-hgis"`, `"Referer": "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563"`; Rust: `-A "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `-H "Accept: application/json"` | 1 | 0 ms pause between files; 1 GET per matched 시·군·구 not already cached on disk (`1..3` ZIPs typical) | C++: `120000 ms` (`2 min`) idle + `30 min` absolute deadline (`512 MB` cap); Rust: `--max-time 15` (`15 s`) | 0 | C++: if response lacks ZIP magic `PK\x03\x04` (HTML login/error screen) or HTTP != 200, aborts with 「VWorld에서 ZIP 대신 로그인·오류 화면을 받았습니다. 계정을 확인하세요.」; Rust: `bytes.starts_with(b"PK\x03\x04")` rejects HTML login page with the same `CADASTRAL_NOT_ZIP_MESSAGE` | `<survey>/지적도/원본/<fileNo>-<sizeKb>-<revision>-<name>` (permanent; skipped if file already exists and unpacks) | `target/cpp-ref/src/core/CadastralPortal.cpp:45-104,310-337` | `crates/strata-net/src/vworld_cadastral.rs:9-10,22-32,264-269`, `crates/strata-app/src/cadastral_download_ui_worker.rs:134-154` | C++ downloads with authenticated session cookies and 120 s idle / 30 min cap; Rust calls `downloadResourceFile.do` unauthenticated with 15 s total timeout and receives an HTML login page |
| 리본 더보기 → 웹 자료 → 「VWorld 지적도 자료」 | Browser open: `https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563` | `none` (delegates to OS default browser) | OS browser defaults | 1 user click | Manual click (`1` navigation) | OS browser | 0 | Handled by user's web browser | OS browser | `target/cpp-ref/src/app/MainWindowRibbon.cpp:699-700` | `none` | Present in C++ ribbon menu only |

---

### 4. `nominatim.openstreetmap.org`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 리본 「주소·지번·장소 검색」 폴백 검색 | `GET /search?q=<q>&format=json&addressdetails=1&limit=12&countrycodes=kr&accept-language=ko` | `none` | C++: `"User-Agent": "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `"Accept": "application/json"`; Rust: `-A "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"`, `-H "Accept: application/json"` | 1 (`m_pending` guard in C++; synchronous UI call in Rust) | 0 ms application rate limiter: 1 request per search when VWorld key is empty or VWorld `place` search fails/empties, so rapid Enter clicks can exceed `1 req/s` (`1` request/search) | C++: remaining `20000 ms` deadline; Rust: `--max-time 15` (`15 s`) | 0 | HTTP 403/429/5xx emits 「위치 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인하고 잠시 후 다시 검색하세요.」; non-array body emits 「위치 검색 서버가 올바른 결과를 보내지 않았습니다. 잠시 후 다시 검색하세요.」 | none | `target/cpp-ref/src/core/LocationSearch.cpp:120,222-290,327,343` | `crates/strata-net/src/address_geocode.rs:13-41,64-142,198`, `crates/strata-app/src/gui_ribbon_search.rs:88-107` | Query parameters, headers, and Korean error messages match 1:1; C++ runs async with a 20 s deadline whereas Rust blocks the UI thread via `curl -K -` with a 15 s timeout |

---

### 5. `tile.openstreetmap.org`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 「오픈스트리트맵」 / 일반지도 최종 폴백 XYZ 타일 | `GET /{z}/{x}/{y}.png` (`zmin=0`, `zmax=19`) | `none` | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager` per-host cap) | 0 ms pause between tiles; typical 1920×1080 viewport = `24..48` tiles (`24..48` requests). Excluded from bulk offline pack by `BasemapPolicy::offlineCachingForUrl` (`BasemapPolicy.h:109`) | `15000 ms` (`QgsNetworkAccessManager::setTimeout`) | `6` (`qgis/defaultTileMaxRetry`), 0 ms delay, including 4xx | Logs `[qgis/WMS]` warning to session log via `KaCrashGuard::logLine`; falls back to `Carto Light` if layer creation fails (`BasemapOps.cpp:986-1005`) | `2 GB` `QNetworkDiskCache` (honors HTTP cache headers) | `target/cpp-ref/src/core/BasemapOps.cpp:979-1010`, `target/cpp-ref/src/core/BasemapPolicy.h:102-122`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` (`crates/strata-app/src/tile_layers.rs:58-65` defines metadata only, never fetched) | Active in C++ via QGIS XYZ provider; not wired to network in Rust |

---

### 6. `basemaps.cartocdn.com`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 OSM 폴백 (`Carto Light`) XYZ 타일 | `GET https://{basemaps\|a.basemaps}.cartocdn.com/light_all/{z}/{x}/{y}.png` (`zmin=0`, `zmax=20`) | `none` (no Carto API key sent) | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; typical viewport = `24..48` tiles (`24..48` requests). Blocked from bulk offline pack (`BasemapPolicy.h:111`) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Logs `[qgis/WMS]` warning to session log; failed tiles render blank | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/BasemapOps.cpp:986-1009`, `target/cpp-ref/src/core/BasemapPolicy.h:111`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` | C++ only |

---

### 7. `tile.opentopomap.org`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 「지형맵」 (등고선 음영 OpenTopoMap) XYZ 타일 | `GET https://{tile\|a.tile}.opentopomap.org/{z}/{x}/{y}.png` (`zmin=5`, `zmax=17`) | `none` | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; typical viewport = `24..48` tiles (`24..48` requests). Blocked from bulk offline pack (`BasemapPolicy.h:110`) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Logs `[qgis/WMS]` warning to session log; failed tiles render blank | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/BasemapOps.cpp:1577-1595`, `target/cpp-ref/src/core/BasemapPolicy.h:110`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` | C++ only |

---

### 8. `copernicus-dem-30m.s3.amazonaws.com`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「표고(DEM) 내려받기」 조사 범위 DEM 내려받기 | C++: GDAL `/vsicurl/` `HEAD` + `GET` (`Range: bytes=...`) on `/Copernicus_DSM_COG_10_{N\|S}{lat:02d}_00_{E\|W}{lon:03d}_00_DEM/..._DEM.tif`; Rust: full-file `GET` on the same path | `none` | C++: GDAL libcurl default UA; Rust: `-A "ka-hgis/0.8 (Korean archaeology HGIS; contact: local)"` | 1 | 0 ms pause between 1°×1° tiles; typical Korean survey bbox spans `1..2` tiles (`1..2` tiles opened/downloaded; hard cap `kMaximumTiles` / `DEM_MAXIMUM_TILES = 24`) | C++: `GDAL_HTTP_CONNECTTIMEOUT = "5"`, `GDAL_HTTP_TIMEOUT = "15"` (`15 s`); Rust: `--connect-timeout 5 --speed-time 15 --speed-limit 1` | 0 (`GDAL_HTTP_MAX_RETRY = "0"` in C++; 0 in Rust) | HTTP `404` is treated as ocean tile and skipped (`missingTiles++`); other HTTP/TIFF validation errors abort the job and show Korean error dialog/modal | Raw 1°×1° tiles in Rust deleted with temp dir; C++ saves clipped `<survey>/DEM.tif` (`MainWindowDownloads.cpp:131`); Rust writes `ka-hgis-reference-*/dem.tif` in `%TEMP%` for current session only | `target/cpp-ref/src/core/DemDownloadService.cpp:25,32-48,85-225`, `target/cpp-ref/src/app/MainWindowDownloads.cpp:112-160` | `crates/strata-net/src/dem_fetch.rs:17-95`, `crates/strata-net/src/dem_pipeline.rs:76-198`, `crates/strata-app/src/modal_downloads.rs:88-137` | C++ streams only the survey window via GDAL `/vsicurl/` HTTP Range reads and saves `<survey>/DEM.tif`; Rust downloads the entire 1°×1° GeoTIFF(s) via `curl.exe` and leaves `dem.tif` in `%TEMP%` |
| 참조 지도 원클릭 「DEM」 원격 `/vsicurl/` VRT 모자이크 | `HEAD` + `GET` (`Range: bytes=...`) `/Copernicus_DSM_COG_10_{N\|S}{lat:02d}_00_{E\|W}{lon:03d}_00_DEM/..._DEM.tif` via `/vsicurl/` | `none` | GDAL libcurl default UA | 1 sequential open loop (up to `kDemMosaicMaxTiles = 24` cells), then QGIS raster block reads | 0 ms pause between `GDALOpenEx` probes (`1..24` cells) and pan/zoom overview reads | Global GDAL default | Global GDAL default (`0`) | Missing (`404`) ocean cells skipped; if `0` cells open, `addDemColorReliefMap` falls back automatically to `gibs.earthdata.nasa.gov` | GDAL `/vsicurl/` 16 MB RAM chunk cache; VRT XML written to `%TEMP%/ka-hgis-dem-<n>.vrt` | `target/cpp-ref/src/core/BasemapOps.cpp:1628-1765,2125-2130` | `none` | C++ only |

---

### 9. `gibs.earthdata.nasa.gov`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 참조 지도 「DEM」 폴백 음영기복 (및 「오프라인 지도 팩」) | `GET /wmts/epsg3857/best/ASTER_GDEM_Color_Shaded_Relief/default/[2000-01-01/]GoogleMapsCompatible_Level12/{z}/{y}/{x}.jpeg` (`zmin=2`, `zmax=12`) | `none` | QGIS: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`); `TilePackService`: GDAL default UA (no `Referer`) | QGIS: ~6; `TilePackService`: `4` (`<MaxConnections>4</MaxConnections>`) | 0 ms pause; at `z <= 12`, 1 viewport = `12..36` tiles (`12..36` requests) | QGIS: `15000 ms`; `TilePackService`: `30 s` | QGIS: `6` (`defaultTileMaxRetry`), 0 ms delay; `TilePackService`: 0 | Logs `[qgis/WMS]` warning on canvas failure; `TilePackService` aborts `GDALTranslate` on server error | `2 GB` `QNetworkDiskCache` + `<survey>/배경지도_오프라인.mbtiles` | `target/cpp-ref/src/core/BasemapOps.cpp:2125-2149`, `target/cpp-ref/src/core/BasemapPolicy.h:106`, `target/cpp-ref/src/core/TilePackService.cpp:54-258`, `target/cpp-ref/src/app/KaApplication.cpp:831-842,873` | `none` | C++ only |

---

### 10. `mt1.google.com`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 「Google 도로」 / 「Google 위성」 및 「3D 지형」 위성 텍스처 폴백 | `GET /vt/lyrs={m\|s}&x={x}&y={y}&z={z}` (`zmin=0`, `zmax=20`) | `none` (direct tile URL without Google Maps Platform API key or session token) | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; typical 2D viewport or 3D drape render = `24..48` tiles (`24..48` requests). Blocked from bulk offline pack (`BasemapPolicy.h:108`) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Logs `[qgis/WMS]` warning; failed 2D/3D tiles render blank or fall back to flat terrain color | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/BasemapOps.cpp:2388-2399`, `target/cpp-ref/src/core/Terrain3dService.cpp:547-552`, `target/cpp-ref/src/app/KaTerrain3dStudio.cpp:190-207`, `target/cpp-ref/src/core/BasemapPolicy.h:108,135-139`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` | C++ only; Rust omits direct Google tile URLs |

---

### 11. `server.arcgisonline.com`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「지질도」 배경 음영기복 (World Hillshade) XYZ 타일 | `GET /ArcGIS/rest/services/Elevation/World_Hillshade/MapServer/tile/{z}/{y}/{x}` (`zmin=0`, `zmax=16`) | `none` (no ArcGIS token/API key) | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; typical viewport = `16..36` tiles (`16..36` requests) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Logs `[qgis/WMS]` warning; geology polygons still render without hillshade backdrop | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/GeologyMapService.cpp:584-605`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` | C++ only |

---

### 12. `intranet.gis-heritage.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「주변유적 받기」 국가유산청 GIS 인트라넷 로그인·서약·지역검색·전체다운로드 | C++ (`QWebEngineView`): `GET /ngis/common/login.do` → form/JS login (`<id>`, `<pw>`) → `GET /ngis/common/main.do` → `GET /user/data/heritageDownload.do?pageMenuId=DAT010010` → pledge confirm → per dataset (`S,P,B,U,R,E`): `POST /user/data/heritageDownloadList.do` + `GET /user/data/heritageDownload/downloadFilesAll.do?...`. Rust (`curl.exe`): `GET /user/data/heritageDownload.do?pageMenuId=DAT010010` → `POST /user/data/heritageDownloadList.do` → `POST <form.action>` → 6 × `GET /user/data/heritageDownload/downloadFilesAll.do?...` | C++: `ID login` (`QWebEngineProfile("ka-heritage")` with `NoPersistentCookies`, session cookies in RAM for dialog lifetime); Rust: `none` (`cookie_jar: None`, stored credentials never sent) | C++ `QWebEngineView`: default QtWebEngine Chromium UA + browser navigation headers (uncalled `HeritageHttpClient.cpp:17-19,85-95` sets `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `"X-Requested-With": "XMLHttpRequest"`, `"Referer": "https://intranet.gis-heritage.go.kr/ngis/common/main.do?httpYn=Y"`); Rust: `-A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `-H "Content-Type: application/x-www-form-urlencoded; charset=UTF-8"`, `-H "Accept: text/html,application/xhtml+xml,*/*"`, `-H "X-Requested-With: XMLHttpRequest"`, `-H "Referer: https://intranet.gis-heritage.go.kr/ngis/common/main.do?httpYn=Y"` | 1 | C++: `700 ms` poll timer (`kPollMs = 700`) with `1.4..2.8 s` settle (`m_settle = 2..4`) and `4.2 s` post-ZIP idle (`m_idleTicks = 6`); 1 city × 6 datasets = `1` login + `1` download page + `6 × (1 search + 1 ZIP) = 14` actions (`QTimer::singleShot(0, ...)` chains next city immediately). Rust: **0 ms pause** across `1 GET + 2 POST + 6 GET = 9` sequential curl calls | C++: stage wait `75 × 700 ms = 52.5 s` (ZIP generation wait uncapped while in flight); Rust: `--max-time 30` (`30 s`) | C++: initial login fails once (`0` retries); download stage retries **unlimited times** (`5 s, 10 s, 20 s, 30 s, 30 s...`) via `scheduleDownloadRetry` + `reopenDownloadSession` (which re-logs in automatically). Rust: 0 retries | C++: wrong password stops after ~5.6 s (`m_waitTicks > 8`); during `Download`, HTTP error, empty file, or HTML body instead of `PK\x03\x04` logs `재수신 예약 N회` and retries + re-logs in forever until user clicks 「취소」. Rust: non-2xx, login HTML (`NO_SESSION`), or non-ZIP body aborts immediately and displays Korean error in modal | C++: `%LOCALAPPDATA%\ka-hgis\주변유적\<시도> <시군>\원본\<uuid>\<name>.zip` + `recent.json` (reused only when user checks reuse in `KaHeritageRegionDialog`); Rust: overwrites `<survey>/주변유적/.../{label}.zip` unconditionally (`heritage_reuse_recent` ignored) | `target/cpp-ref/src/app/KaHeritageBrowser.cpp:297-317,504-629,832-1526`, `target/cpp-ref/src/app/KaHeritageBrowserPlan.cpp:13-96`, `target/cpp-ref/src/core/HeritageIntranetFlow.cpp:39-640`, `target/cpp-ref/src/core/HeritageHttpClient.cpp:15-201` | `crates/strata-net/src/heritage_http.rs:39-100`, `crates/strata-net/src/heritage_pipeline_http.rs:15-170`, `crates/strata-app/src/dialogs_download_print.rs:95-156` | C++ drives a real logged-in Chromium session with pledge confirmation, 1.4–4.2 s pacing, and unbounded auto-relogin retry on download failure; Rust fires 9 unauthenticated `curl` calls back-to-back with spoofed `Mozilla/5.0`/`Referer`/`X-Requested-With` headers |
| 리본 더보기 → 웹 자료 → 「문화재 GIS 인트라넷」 | Browser open: `https://intranet.gis-heritage.go.kr/` | `none` (OS browser) | OS browser defaults | 1 user click | Manual click (`1` navigation) | OS browser | 0 | Handled by user's web browser | OS browser | `target/cpp-ref/src/app/MainWindowRibbon.cpp:695-696` | `none` | Present in C++ ribbon menu only |

---

### 13. `map.ngii.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 「대동여지도」 WMS 1.1.1 | `GET /spcemapserver/korea_old_map/ows?SERVICE=WMS&VERSION=1.1.1&REQUEST=GetMap&LAYERS={korea_oldmap_ddymap_kyu\|korea_oldmap_addAlphaChannel}&SRS=EPSG:5179&FORMAT=image/png&TRANSPARENT=TRUE&...` | `none` | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; 1 canvas render = `1` full-viewport WMS `GetMap` (or tiled sub-requests on print) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Probe tries `korea_oldmap_ddymap_kyu` then falls back to `korea_oldmap_addAlphaChannel`; runtime failure logs `[qgis/WMS]` warning | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/BasemapOps.cpp:1547-1575`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` | C++ only |
| 「수치지형도 내려받기」 반경 5 km 도엽 목록 조회 | `POST /ms/map/combineBufferQuery.do` (`x=<x5179>&y=<y5179>&buffer=5000`) → `POST /ms/map/suchiQuery.do` (`index=&type=0&scale=25000&wkt=<wkt>&minx=...&miny=...&maxx=...&maxy=...`) | `none` | C++ (`QWebEngineView` `fetch()` on `NlipMap.do`): Chromium UA, `"Content-Type": "application/x-www-form-urlencoded; charset=UTF-8"`, same-origin `Origin`/`Referer`; Rust (`curl.exe`): `-A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`, `-H "Content-Type: application/x-www-form-urlencoded; charset=UTF-8"`, `-H "Referer: https://map.ngii.go.kr/ms/map/NlipMap.do"` | 1 | 0 ms pause between the 2 POSTs (`1 + 1 = 2` requests per scope lookup) | C++: `30000 ms` (`AbortSignal.timeout(30000)`) / `90 s` phase timer; Rust: `--max-time 30` (`30 s`) | 0 | C++: non-OK HTTP or unexpected JSON sets `state='blocked'` and shows Korean notice in `KaTopographicBrowser`; Rust: non-zero curl exit, WAF block, empty body, or invalid JSON sets modal error text | C++: skips duplicate lookup in RAM via `m_scopeKey` (`KaTopographicBrowser.cpp:685`); Rust: none (re-POSTs on every click) | `target/cpp-ref/data/providers/ngii-topographic.js:33-82`, `target/cpp-ref/src/app/KaTopographicBrowser.cpp:684-744` | `crates/strata-net/src/ngii_fetch.rs:14-64`, `crates/strata-app/src/ngii_topographic_state.rs:44-110` | C++ runs `fetch()` inside a real `NlipMap.do` browser page; Rust calls the two `.do` endpoints directly from `curl.exe` with spoofed `Mozilla/5.0` UA and `Referer` |
| 「수치지형도 내려받기」 로그인 상태 확인 및 사용 신청서 제출 | `GET /ms/map/NlipMap.do?tabGb=total`, `GET /mn/loginPopup.do`, `GET /anyid/common/anyidLogin.do`, `POST /mn/loginCheck.do`, `POST /pd/shbtManage/shbtInsertNRM.do` | `ID login` (`QWebEngineProfile("ka-ngii")` with `NoPersistentCookies`; session cookies kept in RAM while browser dialog lives) | QtWebEngine Chromium UA + same-origin `fetch` / form headers | 1 (`state.loginInFlight` guard) | `loginCheck.do` polled every `1500 ms` up to `120` polls (`300 s` max; typically `1..3` polls after SSO redirect); `shbtInsertNRM.do` submitted `1` time per order | `loginCheck.do`: `30000 ms` per poll (`300 s` login deadline); `shbtInsertNRM.do`: `90000 ms` (`never` retried on timeout) | `loginCheck.do`: `2` retries (`++state.loginFailures < 3` = `3` total attempts, backoff `1500 ms, 3000 ms`, including on HTTP 4xx); `shbtInsertNRM.do`: `0` retries | `loginCheck.do` failure after 3 errors or 120 polls sets `blocked` with Korean message; `shbtInsertNRM.do` error/timeout aborts without re-POSTing so duplicate orders are not created | Downloaded sheets indexed permanently under `<survey>/지형도/{원본,files,SHP,index,receipts}` | `target/cpp-ref/src/app/KaTopographicBrowser.cpp:228,315-340,705-883`, `target/cpp-ref/src/app/KaTopographicScopePanel.cpp:290-340`, `target/cpp-ref/data/providers/ngii-topographic.js:159-177,204-335` | `none` (`crates/strata-app/src/ngii_topographic_state.rs:28-33` stops with blocked notice) | C++ automates login verification and order insertion inside `QWebEngineView`; Rust stops after sheet lookup |
| 리본 더보기 → 웹 자료 → 「국토정보맵 지형도」 | Browser open: `https://map.ngii.go.kr/ms/map/NlipMap.do` | `none` (OS browser) | OS browser defaults | 1 user click | Manual click (`1` navigation) | OS browser | 0 | Handled by user's web browser | OS browser | `target/cpp-ref/src/app/MainWindowRibbon.cpp:701-702` | `none` | Present in C++ ribbon menu only |

---

### 14. `www.ngii.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「수치지형도 내려받기」 국토지리정보원 통합회원 로그인 | `GET /member/login.do` → `POST /member/process.do` (`mem_id=<id>&mem_pw=<pw>` via `#formLogin` submit to `hidden_iframe` inside `KaTopographicBrowser`) | `ID login` (sets SSO session cookies in `QWebEngineProfile("ka-ngii")` RAM cookie store) | QtWebEngine Chromium navigation headers (`Origin: https://www.ngii.go.kr`, `Referer: https://www.ngii.go.kr/member/login.do`) | 1 | `1` GET + `1` POST per `(generation, loginAttempt)` (`2` requests); resets if `m_automationGeneration` increments on new 5 km scope or `NlipMap.do` reload | `300000 ms` (`5 min`) login phase timeout (`ngii-topographic.js:155`) | `0` within one generation; **unbounded** across scope changes / `NlipMap.do` reloads (`E2`) | If CAPTCHA input/iframe/`.g-recaptcha` is visible on `/member/login.do`, stops auto-submit immediately and asks user to complete it manually (`ngii-topographic.js:194-195`); wrong-password alert stops current generation | RAM cookies only (`NoPersistentCookies`) | `target/cpp-ref/src/app/KaTopographicBrowser.cpp:315-330,586-590,745-784`, `target/cpp-ref/src/app/KaTopographicScopePanel.cpp:397,425,518-532`, `target/cpp-ref/data/providers/ngii-topographic.js:183-203` | `none` (`ngii_creds` stored in `crates/strata-app/src/dialogs_account.rs:15`, never sent) | C++ submits SSO form in `QWebEngineView`; Rust stores credentials only and does not log in |

---

### 15. `nlippd.ngii.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「수치지형도 내려받기」 신청 성과물 목록 팝업 및 파일 내려받기 | `POST /nlippd/pd/innorix/innorixdownLoad.do` (`downForm` submit) → `GET /nlippd/pd/innorix/innorixDownLoad2.do?...` (1 GET per file in order) | `ID login` (authenticated `QWebEngineProfile("ka-ngii")` popup session) | QtWebEngine Chromium navigation/download headers | 1 (`m_activeDownload` serializes 1 file at a time) | 0 ms pause between consecutive files (`startNextQueuedDownload` fires immediately on `QWebEngineDownloadRequest::DownloadCompleted`); 1 order = `1` popup POST + `N` file GETs (`2..6` requests typical) | `90000 ms` popup phase + `60000 ms` byte-progress stall timer per file (`1 GB` size cap) | 0 | Stall timeout, HTTP failure, or non-archive/HTML payload cancels download, deletes `.part` file, and shows Korean failure message in `KaTopographicBrowser` | `<survey>/지형도/원본/<uuid>/` + `<survey>/지형도/index/*.json` (permanent; already-indexed sheets are skipped before ordering) | `target/cpp-ref/src/app/KaTopographicBrowser.cpp:634-640,884-1105`, `target/cpp-ref/data/providers/ngii-topographic.js:16-32,336-399` | `none` | C++ only |

---

### 16. `data.kigam.re.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「지질도」 1:5만 지질도 WFS 도형 내려받기 | `GET /geoserver/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames={geoOpen:l_50k_geology_litho_latest\|geoOpen:l_jeju_50k_geology_litho_view}&outputFormat=application/json&srsName=EPSG:5186&count=100000&bbox=<minLat>,<minLon>,<maxLat>,<maxLon>,urn:ogc:def:crs:EPSG::4326` | C++: `none` (public GeoServer OWS); Rust uncalled builder: `API key` (`KEY=<key>` on `/openapi/wfs`) | C++: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"` | 1 (`m_referenceDownload` guard) | 0 ms pause between quadtree pieces and mainland/Jeju layers. Mainland survey = `1` request typical (`2` in Jeju), splitting up to `21` quadtree calls on `TooLarge`/`Slow` | `15000 ms` idle / `180000 ms` total per piece (`kLargeTransfer`), `300000 ms` (`5 min`) wall budget | `1` immediate retry (`0 ms` delay) per piece on `Transient`/`Slow`, plus 2×2/4×4 bbox split | Non-GeoJSON/OGC XML marks `Fatal` (no split); 429/5xx retries once immediately then aborts; shows Korean error dialog + session log | `<survey>/지질도_KIGAM.gpkg` (permanent file, overwritten if layer is not currently in `QgsProject`) | `target/cpp-ref/src/core/GeologyMapService.cpp:227-290`, `target/cpp-ref/src/core/PreparedReferenceMap.cpp:62-154`, `target/cpp-ref/src/core/ReferenceTiledFetch.cpp:24-150`, `target/cpp-ref/src/app/MainWindowOverlay.cpp:492-542` | `crates/strata-net/src/kigam_soil_river.rs:10-33` (uncalled `/openapi/wfs?...&KEY=<key>` URL builder) | C++ queries keyless `/geoserver/ows` WFS 2.0; Rust stores an unused `kigam_key` and uncalled `/openapi/wfs` URL builder |
| 「지질도」 공식 지질 색상 WMS `GetMap` 및 `GetCapabilities` | `GET /geoserver/ows?service=WMS&version=1.1.1&request=GetMap&layers=geoOpen:L_50K_Geology_Map&styles=&srs=EPSG:4326&bbox=<minLon>,<minLat>,<maxLon>,<maxLat>&width=<w>&height=<h>&format=image/png&transparent=true` (up to 1400 px) and `GET /geoserver/ows?SERVICE=WMS&REQUEST=GetCapabilities` | `none` | `GetMap`: `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3"`; `GetCapabilities`: `QNetworkRequest` default | 1 | Immediately follows WFS download with `0 ms` pause (`1` `GetMap` + `0..1` `GetCapabilities` = `1..2` requests) | `GetMap`: `180000 ms` (`kLargeTransfer`); `GetCapabilities`: `15000 ms` (`kSmallRequest`) | 0 | Non-image `GetMap` falls back to deterministic ICS lithology palette without failing the vector geology layer | Saved inside `<survey>/지질도_KIGAM.gpkg` layer style | `target/cpp-ref/src/core/GeologyMapService.cpp:169-223,292-320` | `none` | C++ only |

---

### 17. `gis.naas.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 「토양도」 / 「고지형」 정밀토양도 WFS 내려받기 | `GET /geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames={soilmap:SOIL_1\|soilmap:SOIL_2\|soilmap:SOIL_3}&srsName=EPSG:5186&count=100000[&resultType=hits\|&outputFormat=application/json&propertyName=...,geom]&bbox=<minLat>,<minLon>,<maxLat>,<maxLon>,urn:ogc:def:crs:EPSG::4326` | `none` | `"User-Agent": "ka-hgis/0.3"` (no `Mozilla/5.0`, no `Referer`) | 1 (`m_referenceDownload` guard) | 0 ms pause between layers and pieces. Per bbox piece, queries 3 layers × (`1` `hits` + `0..1` `GetFeature`) = `3..6` requests typical; up to `21` quadtree calls × `6` = `126` requests worst case | `hits`: `15000 ms` (`kSmallRequest`); `GetFeature`: `15000 ms` idle / `180000 ms` total (`kLargeTransfer`); `5 min` wall cap | `1` immediate retry (`0 ms` delay) per piece on `Transient`/`Slow`, plus quadtree split when `hits` exceeds feature cap | HTTP error or OGC `<ExceptionReport>` triggers 1 immediate retry (if transient) or aborts with Korean error dialog + session log | `<survey>/토양도_흙토람.gpkg` (permanent file, overwritten if not loaded in `QgsProject` or triggered from dropdown `QAction`) | `target/cpp-ref/src/core/SoilMapService.cpp:80-107,210-355,392-396`, `target/cpp-ref/src/core/ReferenceTiledFetch.cpp:24-150`, `target/cpp-ref/src/app/MainWindowOverlay.cpp:440-484` | `crates/strata-net/src/kigam_soil_river.rs:39-55` (uncalled URL builder) | C++ fetches WFS 2.0 with honest `ka-hgis/0.3` UA; Rust has an uncalled URL builder and uses local sample data |
| 「토양도」 배경 래스터 타일 | `GET /Geodata/SF/TOP_A_SOIL_T_GEO/Layers/_alllayers/L{z:02d}/R{y:08x}/C{x:08x}.png` (`zmin=6`, `zmax=15`, rewritten from `L{z}/R{y}/C{x}.png` by `SoilMapService::rewriteArcGisCacheUrl`) | `none` | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause; typical viewport = `24..48` tiles (`24..48` requests) | `15000 ms` | `6` (`defaultTileMaxRetry`), 0 ms delay | Logs `[qgis/WMS]` warning on missing/404 ArcGIS cache tiles; vector soil layer remains visible | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/SoilMapService.cpp:109-138,398-410`, `target/cpp-ref/src/app/KaApplication.cpp:839-842,873` | `none` | C++ only |

---

### 18. `soil.rda.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 리본 더보기 → 웹 자료 → 「농진청 토양도」 | Browser open: `http://soil.rda.go.kr/geoweb/soilmain.do` | `none` (OS browser) | OS browser defaults | 1 user click | Manual click (`1` navigation) | OS browser | 0 | Handled by user's web browser | OS browser | `target/cpp-ref/src/app/MainWindowRibbon.cpp:697-698` | `none` | Present in C++ ribbon menu only |

---

### 19. `hgis.history.go.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 배경지도 「1919 조선지형도 1:5만」 WMTS (`history:map1919`) | `GET /openapi/get.do?Service=WMTS&Request=GetCapabilities&apiKey=<key>` followed by WMTS `GetTile` requests on `/openapi/get.do?...&apiKey=<key>` (`EPSG:5179`, `image/png`) | `API key` (query `apiKey=<key>`) | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` (no `Referer`) | ~6 (`QNetworkAccessManager`) | 0 ms pause: `1..4` `GetCapabilities` probes + `20..40` `GetTile` calls for viewport (`21..44` requests) | `15000 ms` | `GetCapabilities`: **4 immediate attempts (`0 ms` delay)** via `QStringList(4, historyGisWmtsUri(...))` even on HTTP `400`; `GetTile`: `6` (`defaultTileMaxRetry`), 0 ms delay | On 401/400/5xx/invalid capabilities after 4 tries, masks `<key>` as `[가림]` in error text (`BasemapOps.cpp:1517-1518`), logs `[qgis/WMS]` warning (`SecretMask::mask` masks `apiKey=`), and shows Korean notice explaining HistoryGIS key / `GWC configuration out of synch` server status | `2 GB` `QNetworkDiskCache` | `target/cpp-ref/src/core/BasemapOps.cpp:1034-1046,1486-1545`, `target/cpp-ref/src/app/KaApplication.cpp:813-842,873` | `none` (`history_key` stored in `crates/strata-app/src/dialogs_account.rs:13`, never sent; `tile_layers.rs:84-91` metadata only) | C++ loads WMTS via QGIS with 4× immediate retry on HTTP 400; Rust stores the key only |

---

### 20. `www.dbpia.co.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 보고서 스튜디오 학술문헌 자동 수집 초기 세션·`searchResultKey` 추출 (`btnHarvest` 현재 숨김) | `GET /search/topSearch?searchOption=all&query=<seedQuery>` | `none` (populates in-memory `QNetworkCookieJar` and scrapes `searchResultKey` token from HTML) | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:135.0) Gecko/20100101 Firefox/135.0"` | 1 | `1` GET at start of `startResearchAndEvaluate`, then immediately calls `runNextDbpiaQuery()` (`0 ms` pause) | `7000 ms` | 0 | Even on HTTP error or empty `m_dbpiaSearchKey`, proceeds immediately into `runNextDbpiaQuery()` | none | `target/cpp-ref/src/app/KaGeminiBridgeClient.cpp:13,114-132`, `target/cpp-ref/src/app/KaHwpReportStudio.cpp:153-154` | `none` | C++ code present (`btnHarvest` hidden in UI via `setVisible(false)` at `KaHwpReportStudio.cpp:154`); absent in Rust |
| 보고서 스튜디오 학술문헌 다회차 검색 (`/goal` 루프, `btnHarvest` 현재 숨김) | `POST /api/search/list` (JSON body: `{"query": "<q>", "page": 1, "pageSize": 20, "sort": "RANK", "collection": "ALL", ...}`) | Scraped `X-JSP-KEY` header + `QNetworkCookieJar` from `/search/topSearch` | `"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:135.0) Gecko/20100101 Firefox/135.0"`, `"Content-Type": "application/json"`, `"Referer": "https://www.dbpia.co.kr/search/topSearch"`, `"X-JSP-KEY": "<scraped>"` | 1 | **0 ms pause** between sequential queries across up to 5 rounds: Round 1 = `5` queries + Rounds 2–5 = up to `4 × 8 = 32` expanded queries (`1 + 5 + 32 = 38` total requests per run) | `8000 ms` per POST | 0 (failed individual query parses as empty JSON and immediately fires next query) | Non-2xx or invalid JSON yields 0 hits for that query and advances to `++m_queryIndex` without error banner | none (`m_seenNodeIds` deduplicates in RAM for 1 run only) | `target/cpp-ref/src/app/KaGeminiBridgeClient.cpp:134-250` | `none` | C++ only |
| 보고서 스튜디오 DBpia 탭 「개인 로그인」 / 「기관 로그인」 | `GET /member/login` or `GET /member/b2bLogin` inside `QWebEngineView` (`QWebEngineProfile("ka-hgis-dbpia")`) | `ID login` (manual user login in embedded `QWebEngineView`; `NoPersistentCookies` by default, `ForcePersistentCookies` on disk when 「로그인 기억」 checked) | QtWebEngine Chromium headers | 1 | Manual user navigation (`1` page load) | QtWebEngine default | 0 | Displayed directly in embedded browser view | Persistent Chromium cookie store on disk if 「로그인 기억」 is checked; RAM otherwise | `target/cpp-ref/src/app/KaHwpReportStudio.cpp:23-30,150-174` | `none` | C++ only |

---

### 21. `ai.dbpia.co.kr`

| purpose (Korean UI name) | method + path pattern | auth | headers set (literal) | max parallel | pacing & typical job arithmetic | timeout | retry | on 401 / 403 / 429 / 5xx / empty or XML-error body | cache | C++ `file:line` | Rust `file:line` | difference between the two apps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 보고서 스튜디오 DBpia 탭 「DBpia AI」 | `GET /ko/search` inside `QWebEngineView` (`QWebEngineProfile("ka-hgis-dbpia")`) | `ID login` (shares `ka-hgis-dbpia` cookie store with `www.dbpia.co.kr`) | QtWebEngine Chromium headers | 1 | Manual user navigation (`1` page load) | QtWebEngine default | 0 | Displayed directly in embedded browser view | Persistent Chromium cookie store on disk when 「로그인 기억」 is checked; RAM otherwise | `target/cpp-ref/src/app/KaHwpReportStudio.cpp:152,175` | `none` | C++ only |

---

## Part 2 — Operator Rules on Automated Use (Read `2026-11-14`)

Each public page below was requested at most once on `2026-11-14`. Where an endpoint returned HTTP 404, a WAF block page, a binary block image, or required interactive portal login/JavaScript rendering, it is recorded as `not read` without guessing any quota or rate number.

### 1. `api.vworld.kr`
- **`robots.txt`**: `https://api.vworld.kr/robots.txt` (read `2026-11-14`, HTTP 200) — allows `/req/*` for general user-agents and sets a 30-second crawl delay only for `bingbot`:
  > `"User-agent: bingbot"` / `"Crawl-delay: 30"` / `"User-agent: *"` / `"Allow: /"`
- **Open API usage terms & per-key quotas / per-second limits**: `https://www.vworld.kr/dev/v4dv_openapiguide2_s001.do` and `https://www.vworld.kr/v4po_svcstpl_s001.do` (attempted `2026-11-14`, both returned HTTP 404; per-key quota management screens require portal login) — `not read`.

### 2. `xdworld.vworld.kr`
- **`robots.txt`**: `https://xdworld.vworld.kr/robots.txt` (read `2026-11-14`, HTTP 200) — allows `/2d/*` for general user-agents:
  > `"User-agent: bingbot"` / `"Crawl-delay: 30"` / `"User-agent: *"` / `"Allow: /"`
- **Open API / keyless 2D tile usage terms & rate limits**: `not read` (no standalone public terms page on `xdworld.vworld.kr`; `www.vworld.kr` guide/terms URLs returned HTTP 404 on `2026-11-14`).

### 3. `www.vworld.kr`
- **`robots.txt`**: `https://www.vworld.kr/robots.txt` (read `2026-11-14`, HTTP 200) — allows `/dtmk/` and `/v4po_usrlogin_a004.do` under `Allow: /`, **but** specifies a **30-second delay between requests** for all user-agents (`User-agent: *`):
  > `"User-agent: *"` / `"Crawl-delay: 30"` / `"User-agent: *"` / `"Allow: /"` / `"User-agent: Googlebot"` / `"Disallow: /v4po_search.do"` / `"Disallow: /v4po_brd*"`
- **Portal login & dataset download terms of use**: `https://www.vworld.kr/v4po_svcstpl_s001.do` (attempted `2026-11-14`, returned HTTP 404) — `not read`.

### 4. `nominatim.openstreetmap.org`
- **`robots.txt`**: `https://nominatim.openstreetmap.org/robots.txt` (read `2026-11-14`, HTTP 200) — disallows `/search` to web crawlers:
  > `"User-agent: *"` / `"Disallow: /search.php"` / `"Disallow: /search"`
- **Nominatim Usage Policy**: `https://operations.osmfoundation.org/policies/nominatim/` (read `2026-11-14`, HTTP 200) — caps traffic at **1 request per second across the entire application**, requires an identifying `User-Agent` or `Referer`, and asks clients to cache results:
  > `"No heavy uses (an absolute maximum of 1 request per second )."`
  > `"Provide a valid HTTP Referer or User-Agent identifying the application (stock User-Agents as set by http libraries will not do)."`
  > `"Note that the usage limits above apply per website/application : the sum of traffic by all your users should not exceed the limits."`
  > `"If at all possible, set up a proxy and also enable caching of requests."`

### 5. `tile.openstreetmap.org`
- **`robots.txt`**: `https://tile.openstreetmap.org/robots.txt` (attempted `2026-11-14`) — `not read` as `robots.txt` text; server returned a PNG block image with HTTP header `x-blocked: Access denied. See https://operations.osmfoundation.org/policies/tiles/`.
- **Tile Usage Policy**: `https://operations.osmfoundation.org/policies/tiles/` (read `2026-11-14`, HTTP 200) — forbids masquerading as a web browser `User-Agent`, forbids bulk scraping, and requires >= 7-day or HTTP-header local caching:
  > `"Send a valid HTTP User-Agent that clearly identifies your application (or a platform X-Requested-With app ID where set automatically)."`
  > `"Cache tiles locally according to HTTP caching headers (or at least 7 days if your cache cannot read them)."`
  > `"You must not: Bulk download (“scrape”) tiles or offer prefetch features."`
  > `"Masquerade as another app’s User-Agent, or rely on a library’s default User-Agent."`

### 6. `basemaps.cartocdn.com`
- **`robots.txt`**: `https://basemaps.cartocdn.com/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `Not Found`) — `not read`.
- **Basemap Terms**: `https://carto.com/legal/basemap-terms/` (read `2026-11-14`, HTTP 200) — page shell states binding service terms, while numeric Fair Use / non-commercial grant clauses were not present in the static HTML shell (`not read` for exact numeric limit):
  > `"If Customer does not agree with any of the terms in this Agreement, Customer must not use the Services."`

### 7. `tile.opentopomap.org`
- **`robots.txt`**: `https://tile.opentopomap.org/robots.txt` (read `2026-11-14`, HTTP 200) — disallows all automated crawling of `/`:
  > `"User-agent: *"` / `"Disallow: /"`
- **Operator About / Usage Page**: `https://opentopomap.org/about` (read `2026-11-14`, HTTP 200) — volunteer service with no SLA or numeric per-second quota published on `/about`:
  > `"It was never my goal to make any profit, at the cost that I never gave any service or support guarantee."`

### 8. `copernicus-dem-30m.s3.amazonaws.com`
- **`robots.txt`**: `https://copernicus-dem-30m.s3.amazonaws.com/robots.txt` (attempted `2026-11-14`, returned HTTP 404 S3 `NoSuchKey`) — `not read`.
- **Public Dataset Page & Bucket Readme**: `https://registry.opendata.aws/copernicus-dem/` and `https://copernicus-dem-30m.s3.amazonaws.com/readme.html` (both read `2026-11-14`, HTTP 200) — published free of charge on AWS Open Data with no per-second request limit stated on either page (detailed licence PDF linked externally was `not read`):
  > `"GLO-30 Public and GLO-90 are available on a free basis for the general public under the terms and conditions of the Licence found here ."`

### 9. `gibs.earthdata.nasa.gov`
- **`robots.txt`**: `https://gibs.earthdata.nasa.gov/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `Not Found. The requested URL was not found on this server.`) — `not read`.
- **GIBS Access Documentation**: `https://nasa-gibs.github.io/gibs-api-docs/access-basics/` (read `2026-11-14`, HTTP 200) — open standards-compliant access without API key; no numeric per-second cap stated on `/access-basics/`:
  > `"NASA's Global Imagery Browse Services (GIBS) visualizations are accessible through public standards-compliant web services, hosted on the https://gibs.earthdata.nasa.gov domain."`

### 10. `mt1.google.com`
- **`robots.txt`**: `https://mt1.google.com/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `Error 404 (Not Found)!!1`) — `not read`.
- **Google Maps Platform Terms of Service**: `https://cloud.google.com/maps-platform/terms` (read `2026-11-14`, HTTP 200) — requires compliance with Google Maps Platform Service terms and licensing restrictions (expanded sub-clauses in dynamic page sections `not read` from static response):
  > `"1. Accessing the Services This section outlines the requirements to use the Services, in compliance with the terms of the Agreement."`
  > `"3. License This section outlines the licensing terms for Google Maps Platform Services, focusing on the restrictions and requirements on how to use the Services."`

### 11. `server.arcgisonline.com`
- **`robots.txt`**: `https://server.arcgisonline.com/robots.txt` (read `2026-11-14`, HTTP 200, response header `X-Robots-Tag: noindex`) — empty `Disallow:` (permits paths while prohibiting search indexing):
  > `"User-agent: *"` / `"Disallow:"`
- **ArcGIS Basemap Usage / Attribution Terms**: `https://developers.arcgis.com/documentation/mapping-and-location-services/deployment/basemap-attribution/` (attempted `2026-11-14`, returned HTTP 404) — `not read`.

### 12. `intranet.gis-heritage.go.kr`
- **`robots.txt`**: `https://intranet.gis-heritage.go.kr/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `<title>error page</title>`) — `not read`.
- **Login & Data Pledge Terms (`원본자료 사용 서약서`)**: Gated behind authenticated intranet login (`/user/data/heritageDownload.do?pageMenuId=DAT010010`) — `not read` from public web (captured dynamically at runtime in C++ `target/cpp-ref/src/core/HeritagePledge.h:10-16`).

### 13. `map.ngii.go.kr`
- **`robots.txt`**: `https://map.ngii.go.kr/robots.txt` (attempted `2026-11-14`, WAF returned HTTP 400 `<H1>Request Blocked</H1>`) — `not read`.
- **Terms of Use / Open API Rules**: Behind the same WAF / authenticated portal session — `not read`.

### 14. `www.ngii.go.kr`
- **`robots.txt`**: `https://www.ngii.go.kr/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `<title>국토지리정보원 서비스 이용안내</title>`) — `not read`.
- **Integrated Member Login Terms**: `not read` (portal terms page not publicly readable via plain HTTP check).

### 15. `nlippd.ngii.go.kr`
- **`robots.txt`**: `https://nlippd.ngii.go.kr/robots.txt` (attempted `2026-11-14`, WAF returned HTTP 400 `<H1>Request Blocked</H1>`) — `not read`.
- **Download Service Terms**: Gated behind authenticated `map.ngii.go.kr` order popup — `not read`.

### 16. `data.kigam.re.kr`
- **`robots.txt`**: `https://data.kigam.re.kr/robots.txt` (attempted `2026-11-14`, WAF returned HTTP 400 `<H1>Request Blocked</H1>`) — `not read`.
- **GeoBigData Open Platform Terms / OpenAPI Quotas**: Blocked to non-browser HTTP reader by the same WAF — `not read`.

### 17. `gis.naas.go.kr`
- **`robots.txt`**: `https://gis.naas.go.kr/robots.txt` (attempted `2026-11-14`, returned HTTP 404 `404 - Not Found`) — `not read`.
- **GeoServer WFS / Tile Terms & Rate Limits**: No public terms document published on `gis.naas.go.kr` (`not read`; parent portal `soil.rda.go.kr` `robots.txt` below).

### 18. `soil.rda.go.kr`
- **`robots.txt`**: `https://soil.rda.go.kr/robots.txt` (read `2026-11-14`, HTTP 200 after `http://` → `https://` redirect) — allows `/geoweb/soilmain.do` under `Allow: /` and disallows internal/AJAX paths:
  > `"User-agent: *"` / `"Allow: /"` / `"Disallow: /common/"` / `"Disallow: /gpkisecureweb/"` / `"Disallow: /RDServer/"` / `"Disallow: /search/"` / `"Disallow: /soiladm/"` / `"Disallow: /xml/"` / `"Disallow: /exam/"` / `"Disallow: /aerims/"` / `"Disallow: /mntrng/"` / `"Disallow: /board/boardList.do?bid=1018"` / `"Disallow: /board/boardDetail.do?bid=1018"` / `"Disallow: /cmm/common/ajaxCall.do"`
- **흙토람 OpenAPI / Service Terms**: `not read` (app only opens `soilmain.do` in the user's external browser).

### 19. `hgis.history.go.kr`
- **`robots.txt`**: `https://hgis.history.go.kr/robots.txt` (attempted `2026-11-14`, WAF returned HTTP 400 `<H1>Request Blocked</H1>`) — `not read`.
- **Open API Guide**: `https://hgis.history.go.kr/api/intro.do` (read `2026-11-14`, HTTP 200) — explicitly authorizes adding the WMTS URL with an issued `apiKey` in QGIS, documents `401` on invalid/unauthorized key, and publishes no numeric daily or per-second limit on `/api/intro.do`:
  > `"요청 URL을 인증받은 키 값과 함께 QGIS 프로그램 WMTS 레이어로 추가하거나 사용자 클라이언트에 Javascript 소스로 추가하시면 사용하실 수 있습니다."`
  > `"401(인증 실패) API 권한이 설정되지 않은 경우 or API키가 잘못된 경우 API 권한과 API키를 확인합니다."`

### 20. `www.dbpia.co.kr`
- **`robots.txt`**: `https://www.dbpia.co.kr/robots.txt` (read `2026-11-14`, HTTP 200) — **forbids all automated access (`Disallow: /`)** to every user-agent except `Googlebot`, `Yeti`, and `Bingbot` (each given `Crawl-delay: 5`):
  > `"#──[기본 정책]"` / `"User-agent: *"` / `"Disallow: /"`
- **Terms of Use**: `not read`.

### 21. `ai.dbpia.co.kr`
- **`robots.txt`**: `https://ai.dbpia.co.kr/robots.txt` (read `2026-11-14`, HTTP 200) — allows `/`:
  > `"User-Agent: *"` / `"Allow: /"`
- **Terms of Use**: `not read`.

---

## Part 3 — Findings (`a`–`f`)

### Category `a` — Browser `User-Agent`, Fabricated `Referer`, or `X-Requested-With`

- **`A1` — Global QGIS request preprocessor spoofs `Mozilla/5.0` on all 11 map hosts and sends `Referer: https://localhost` to keyless `xdworld.vworld.kr`**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`, `tile.openstreetmap.org`, `basemaps.cartocdn.com`, `tile.opentopomap.org`, `gibs.earthdata.nasa.gov`, `mt1.google.com`, `server.arcgisonline.com`, `map.ngii.go.kr`, `gis.naas.go.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaApplication.cpp:831-842`, `target/cpp-ref/src/core/BasemapPolicy.h:129-131`
  - **Rust**: `none`
  - **Operator / quota risk**: Sending `"Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3 QGIS"` to `tile.openstreetmap.org` directly violates OSMF's Tile Usage Policy rule against masquerading as a browser `User-Agent`, and attaching `Referer: https://localhost` to keyless `xdworld.vworld.kr` claims a browser origin the desktop app did not navigate from.
  - **Smallest change**: Replace the `Mozilla/5.0 (...)` prefix in `KaApplication.cpp:834` with `"ka-hgis/0.3 QGIS"` and restrict `BasemapPolicy::refererForUrl` to authenticated `api.vworld.kr` requests whose registered VWorld API key domain requires it.

- **`A2` — Direct WMTS/XYZ/WMS fetchers and offline tile pack send bare `Mozilla/5.0` (Rust) or `Mozilla/5.0 ka-hgis/0.3` (C++) plus `Referer: https://localhost`**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/BasemapOps.cpp:1266-1267`, `target/cpp-ref/src/core/TilePackService.cpp:84-85`
  - **Rust**: `crates/strata-app/src/basemap_fetch.rs:119-123`, `crates/strata-app/src/basemap_cadastral.rs:190-194`
  - **Operator / quota risk**: Rust's literal `User-Agent: Mozilla/5.0` with no product token looks like a generic scraper signature to CDN/WAF filters, and both apps send `Referer: https://localhost` even on unauthenticated `xdworld.vworld.kr` tile fetches.
  - **Smallest change**: Set an honest application `User-Agent` (`ka-hgis/<ver>`) in `basemap_fetch.rs`, `basemap_cadastral.rs`, and `BasemapOps.cpp`, and omit `Referer` when fetching keyless `xdworld.vworld.kr` tiles.

- **`A3` — C++ direct REST, WFS, and WMS downloaders prepend `Mozilla/5.0 (Windows NT 10.0; Win64; x64)` and `Referer: https://localhost`**
  - **Hosts**: `api.vworld.kr`, `data.kigam.re.kr`
  - **C++**: `target/cpp-ref/src/core/AdminBoundaryService.cpp:108-110`, `target/cpp-ref/src/core/HeritageRegionResolver.cpp:159-161,218-220`, `target/cpp-ref/src/core/HeritageNearbyRegions.cpp:103-105`, `target/cpp-ref/src/core/RiverMapService.cpp:122-124`, `target/cpp-ref/src/core/GeologyMapService.cpp:207-208,243-244`
  - **Rust**: `none` (Rust uses `"ka-hgis/0.3 (Korean archaeology HGIS; contact: local)"` without `Mozilla/5.0` or `Referer` in `vworld_admin.rs:124` and `address_geocode.rs:25`)
  - **Operator / quota risk**: Public GeoServer (`data.kigam.re.kr`) and VWorld operators inspecting logs see desktop batch WFS/WMS downloads pretending to be a Windows web browser.
  - **Smallest change**: Use `"ka-hgis/0.3"` (matching `LocationSearch.cpp:313` and `SoilMapService.cpp:82`) in those five C++ files and send `Referer` only where VWorld key validation requires it.

- **`A4` — C++ `CadastralPortal` spoofs `Mozilla/5.0 ka-hgis`, portal `Referer`, and `X-Requested-With: XMLHttpRequest`**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/CadastralPortal.cpp:48-55`
  - **Rust**: `none`
  - **Operator / quota risk**: Presenting automated catalog scraping and binary ZIP downloads on `www.vworld.kr` with `Mozilla/5.0`, a fabricated `/dtmk/dtmk_ntads_s002.do?dsId=30563` `Referer`, and `X-Requested-With: XMLHttpRequest` mimics in-page browser AJAX traffic on a portal whose `robots.txt` specifies `Crawl-delay: 30`.
  - **Smallest change**: Identify the client as `ka-hgis/0.3` and drop fabricated `Referer` and `X-Requested-With` headers from non-browser requests (or route portal login/downloads only through an explicit user-driven browser session).

- **`A5` — Heritage intranet HTTP clients spoof `Mozilla/5.0`, `Referer: .../ngis/common/main.do?httpYn=Y`, and `X-Requested-With: XMLHttpRequest`**
  - **Hosts**: `intranet.gis-heritage.go.kr`
  - **C++**: `target/cpp-ref/src/core/HeritageHttpClient.cpp:17-19,85-95` (uncalled helper)
  - **Rust**: `crates/strata-net/src/heritage_http.rs:48-55`, `crates/strata-net/src/heritage_pipeline_http.rs:25-145`
  - **Operator / quota risk**: Rust sends `Mozilla/5.0`, a forged post-login `main.do?httpYn=Y` `Referer`, and `X-Requested-With: XMLHttpRequest` across 9 unauthenticated `curl` calls (including `GET` ZIP URLs), which intranet security logs can flag as scripted session-bypass probing against a government intranet.
  - **Smallest change**: Remove direct `curl` scraping in `heritage_http.rs` / `heritage_pipeline_http.rs` (and delete uncalled `HeritageHttpClient.cpp`) so intranet access occurs only after an authenticated user session with honest headers.

- **`A6` — Rust NGII 5 km sheet lookup spoofs `Mozilla/5.0` and `Referer: https://map.ngii.go.kr/ms/map/NlipMap.do` from `curl.exe`**
  - **Hosts**: `map.ngii.go.kr`
  - **C++**: `none` (runs inside `QWebEngineView` genuinely loaded on `NlipMap.do`)
  - **Rust**: `crates/strata-net/src/ngii_fetch.rs:40-46`
  - **Operator / quota risk**: Calling internal `.do` endpoints from `curl.exe` while claiming `Referer: .../NlipMap.do` and `Mozilla/5.0` looks like cross-tool scraping to NGII's WAF (which already blocks non-browser requests on `map.ngii.go.kr`).
  - **Smallest change**: Compute 1:25,000 / 1:5,000 Korean standard map sheet numbers locally from EPSG:5179 coordinates instead of POSTing to `combineBufferQuery.do` and `suchiQuery.do` from `curl.exe`.

- **`A7` — C++ `KaGeminiBridgeClient` impersonates desktop Firefox 135 (`rv:135.0`) and `Referer: https://www.dbpia.co.kr/search/topSearch`**
  - **Hosts**: `www.dbpia.co.kr`
  - **C++**: `target/cpp-ref/src/app/KaGeminiBridgeClient.cpp:13,121,147-149`
  - **Rust**: `none`
  - **Operator / quota risk**: Stripping the app name entirely to send a stock Firefox 135 `User-Agent`, a forged `topSearch` `Referer`, and a scraped `X-JSP-KEY` header across up to 38 rapid requests against a site whose `robots.txt` declares `Disallow: /` looks indistinguishable from a credentialless scraping bot.
  - **Smallest change**: Remove automated `/api/search/list` scraping in `KaGeminiBridgeClient.cpp` or switch to an officially licensed bibliographic API with an honest `ka-hgis` `User-Agent`.

---

### Category `b` — Parallel Requests Without a Cap, or Back-to-Back Without a Pause

- **`B1` — Rust basemap painter fires up to 256 `curl.exe` tile requests back-to-back with 0 ms pause on the UI thread**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`
  - **C++**: `none`
  - **Rust**: `crates/strata-app/src/basemap_paint.rs:110-137`, `crates/strata-app/src/basemap_fetch.rs:104-129`
  - **Operator / quota risk**: Rapid panning or zooming on a high-DPI monitor launches dozens to `256` back-to-back WMTS/2D tile requests with zero inter-request spacing, burning per-second and daily VWorld API key quotas while blocking the UI thread.
  - **Smallest change**: Move tile fetching onto a bounded async queue (`<= 2..4` workers) with viewport debounce (`~150–250 ms` after pan/zoom settles) and a token-bucket rate limit per host.

- **`B2` — C++ QGIS tile providers, `TilePackService`, `TilePrint`, and Rust `basemap_cadastral_jobs` fire unthrottled parallel bursts with 0 ms pause**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`, `tile.openstreetmap.org`, `basemaps.cartocdn.com`, `tile.opentopomap.org`, `gibs.earthdata.nasa.gov`, `mt1.google.com`, `server.arcgisonline.com`, `map.ngii.go.kr`, `gis.naas.go.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaApplication.cpp:811,883`, `target/cpp-ref/src/core/TilePackService.cpp:83,168-205`, `target/cpp-ref/src/core/TilePrint.cpp:270-355`
  - **Rust**: `crates/strata-app/src/basemap_cadastral_jobs.rs:18,45-90`
  - **Operator / quota risk**: Rust spawns `MAX_IN_FLIGHT = 6` simultaneous `curl.exe` threads for up to 96 cadastral WMS tiles per frame with 0 ms pause, and C++ `TilePackService` streams up to `20,000` tiles across `<MaxConnections>4</MaxConnections>` with 0 ms delay, which can exhaust VWorld key quotas or trip CDN rate limits in seconds.
  - **Smallest change**: Cap per-host concurrency to `2` for WMS/tile packs and insert a minimum per-host request interval (`50–100 ms` on interactive tiles, `>= 200 ms` on bulk offline packs) with pan/zoom debounce.

- **`B3` — Address/place search chains VWorld `place` → Nominatim fallback with 0 ms pause and enforces no `1 req/s` rate limit on `nominatim.openstreetmap.org`**
  - **Hosts**: `api.vworld.kr`, `nominatim.openstreetmap.org`
  - **C++**: `target/cpp-ref/src/core/LocationSearch.cpp:222-226,327,343`
  - **Rust**: `crates/strata-net/src/address_geocode.rs:191-198`, `crates/strata-net/src/address_vworld_search.rs:49,73`, `crates/strata-app/src/gui_ribbon_search.rs:88-107`
  - **Operator / quota risk**: Neither app enforces a minimum interval between consecutive searches, so rapid Enter presses on unkeyed or unmatched queries violate OSMF's hard `1 request per second` Nominatim cap, risking IP or `User-Agent` blocking.
  - **Smallest change**: Enforce a global `>= 1100 ms` minimum spacing gate before any `nominatim.openstreetmap.org` request and `>= 200 ms` spacing on `api.vworld.kr/req/search`.

- **`B4` — Cadastral download loops WFS pages, `dtmk` HTML catalog pages, and ZIP downloads back-to-back with 0 ms pause (`www.vworld.kr` `Crawl-delay: 30`)**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/CadastralPortal.cpp:231-261,295-303,310-337`
  - **Rust**: `crates/strata-app/src/cadastral_download_ui_worker.rs:53-85,95-113,135-153`
  - **Operator / quota risk**: Scraping multiple `dtmk_ntads_s002.do` pages and downloading multiple `downloadResourceFile.do` ZIPs with 0 ms pause directly breaches `www.vworld.kr/robots.txt` (`Crawl-delay: 30`) and can trigger portal account or IP throttling.
  - **Smallest change**: Add an explicit inter-request delay between `www.vworld.kr` catalog/download requests and stop paging as soon as all target 시·군·구 codes are matched.

- **`B5` — C++ administrative boundary and heritage region resolvers fire 2–5 sequential `api.vworld.kr` calls with 0 ms pause**
  - **Hosts**: `api.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/AdminBoundaryService.cpp:102-177`, `target/cpp-ref/src/core/HeritageRegionResolver.cpp:150-262`, `target/cpp-ref/src/core/HeritageNearbyRegions.cpp:95-186`
  - **Rust**: `none`
  - **Operator / quota risk**: Opening the heritage download dialog fires `HeritageRegionResolver` (up to 2 calls) and `HeritageNearbyRegions` (up to 5 paged calls) concurrently and back-to-back with 0 ms pause, multiplying per-second `api.vworld.kr` key hits.
  - **Smallest change**: Share one per-host rate limiter (`>= 100–200 ms` spacing) across all `api.vworld.kr` REST/WFS helpers and serialize resolver + nearby-region queries.

- **`B6` — Copernicus DEM downloaders and `/vsicurl/` VRT probe up to 24 GeoTIFF tiles back-to-back with 0 ms pause**
  - **Hosts**: `copernicus-dem-30m.s3.amazonaws.com`
  - **C++**: `target/cpp-ref/src/core/DemDownloadService.cpp:160-205`, `target/cpp-ref/src/core/BasemapOps.cpp:1707-1719`
  - **Rust**: `crates/strata-net/src/dem_pipeline.rs:96-119`
  - **Operator / quota risk**: Rust downloads up to 24 full 1°×1° COG GeoTIFFs (~600 MB–1 GB) back-to-back with zero pause (and C++ probes up to 24 `/vsicurl/` tiles sequentially with 0 ms pause), creating sustained S3 egress bursts.
  - **Smallest change**: Fetch only the survey bbox sub-window via HTTP `Range` requests in Rust (matching C++ `/vsicurl/`), cap single-survey DEM downloads to `<= 4` tiles, and add a `200 ms` pause between tiles.

- **`B7` — C++ thematic reference map downloaders chain up to 21 quadtree bboxes × multi-page/multi-layer WFS/WMS calls with 0 ms pause**
  - **Hosts**: `api.vworld.kr`, `data.kigam.re.kr`, `gis.naas.go.kr`
  - **C++**: `target/cpp-ref/src/core/ReferenceTiledFetch.cpp:34-94`, `target/cpp-ref/src/core/RiverMapService.cpp:103-161`, `target/cpp-ref/src/core/GeologyMapService.cpp:194-303`, `target/cpp-ref/src/core/SoilMapService.cpp:230-349`
  - **Rust**: `none`
  - **Operator / quota risk**: When a WFS bbox is slow or dense, `ReferenceTiledFetch` recursively splits into 4 and 16 sub-boxes and fires up to 126–210 GeoServer/VWorld WFS calls back-to-back with 0 ms pause, compounding load on an already slow server.
  - **Smallest change**: Insert a `250–500 ms` pacing delay between piece/page requests in `ReferenceTiledFetch.cpp` and `RiverMapService.cpp`, increasing backoff when `Outcome::Slow` occurs.

- **`B8` — Heritage intranet batch downloads run 9 requests with 0 ms pause in Rust and chain multi-city jobs with `singleShot(0)` in C++**
  - **Hosts**: `intranet.gis-heritage.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaHeritageBrowserPlan.cpp:51-59`
  - **Rust**: `crates/strata-net/src/heritage_pipeline_http.rs:25-145`
  - **Operator / quota risk**: Rust blasts 9 back-to-back intranet requests (`GET` + 2 `POST` + 6 `downloadFilesAll.do` ZIPs) with 0 ms pause, and C++ launches the next city's full 6-dataset browser run via `QTimer::singleShot(0, ...)`, which can overload heavy server-side shapefile-ZIP generation and flag the user's intranet account.
  - **Smallest change**: Enforce a multi-second pause (`>= 2–5 s`) between dataset ZIP requests and between consecutive city runs.

- **`B9` — NGII 5 km sheet query chains two `.do` POSTs with 0 ms pause, and C++ chains sequential `innorixDownLoad2.do` file GETs with 0 ms pause**
  - **Hosts**: `map.ngii.go.kr`, `nlippd.ngii.go.kr`
  - **C++**: `target/cpp-ref/data/providers/ngii-topographic.js:42-53`, `target/cpp-ref/src/app/KaTopographicBrowser.cpp:1051-1083`
  - **Rust**: `crates/strata-net/src/ngii_fetch.rs:20-25`, `crates/strata-app/src/ngii_topographic_state.rs:67-76`
  - **Operator / quota risk**: Back-to-back POSTs to `combineBufferQuery.do` and `suchiQuery.do` on every map scope change and zero-delay chaining of large `innorixDownLoad2.do` archives create burst patterns against WAF-protected NGII servers.
  - **Smallest change**: Debounce scope changes and insert a `>= 500–1000 ms` delay before `suchiQuery.do` and between queued `innorixDownLoad2.do` downloads.

- **`B10` — C++ `KaGeminiBridgeClient` fires `GET /search/topSearch` + up to 37 `POST /api/search/list` requests back-to-back with 0 ms pause**
  - **Hosts**: `www.dbpia.co.kr`
  - **C++**: `target/cpp-ref/src/app/KaGeminiBridgeClient.cpp:114-250`
  - **Rust**: `none`
  - **Operator / quota risk**: Sending up to 38 sequential search requests with zero delay against a commercial academic database whose `robots.txt` sets `Crawl-delay: 5` even for whitelisted search engines is a classic automated-scraping burst that triggers IP/session blocks.
  - **Smallest change**: Cap search queries per user action and enforce `>= 5 s` spacing (or delete unused automated DBpia scraping code).

---

### Category `c` — Retry Without Delay, Without Limit, or After 4xx

- **`C1` — Rust WMTS/XYZ basemap painter retries failed, 4xx, and HTTP-200-XML-error tiles on every egui paint frame with 0 ms delay and no cap**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`
  - **C++**: `none`
  - **Rust**: `crates/strata-app/src/basemap_paint.rs:124-132`, `crates/strata-app/src/basemap_fetch.rs:104-115`
  - **Operator / quota risk**: Because `basemap_paint.rs` inserts into `TileTextureCache` only when `load_tile_texture` succeeds, an invalid VWorld API key (401/403 or HTTP 200 XML `<ServiceExceptionReport>`) or 404/429 causes every repaint frame to re-run `curl` for the entire viewport without limit or delay, rapidly exhausting quotas or hammering `api.vworld.kr`.
  - **Smallest change**: Record negative cache entries per tile (with bounded exponential backoff on 5xx/network errors, zero retries on 400/401/403/404, and circuit-breaker pause on XML key/quota errors).

- **`C2` — C++ sets global `qgis/defaultTileMaxRetry = 6`, retrying failed tiles 6 times with 0 ms backoff including on 4xx**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`, `tile.openstreetmap.org`, `basemaps.cartocdn.com`, `tile.opentopomap.org`, `gibs.earthdata.nasa.gov`, `mt1.google.com`, `server.arcgisonline.com`, `map.ngii.go.kr`, `gis.naas.go.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaApplication.cpp:873`
  - **Rust**: `none`
  - **Operator / quota risk**: Raising QGIS's tile retry count from 3 to 6 multiplies every 401/403/404/429 viewport failure by `7×` (`48 tiles × 7 = 336` immediate requests for one screen), turning a bad API key or rate-limit response into an immediate request storm.
  - **Smallest change**: Remove `tileSettings.setValue("qgis/defaultTileMaxRetry", 6)` (or lower to `<= 1..2`) and validate API keys once before mounting authenticated tile layers.

- **`C3` — Rust cadastral WMS worker retries failed, 4xx, and XML-error tiles every 30 s forever without a retry limit**
  - **Hosts**: `api.vworld.kr`
  - **C++**: `none`
  - **Rust**: `crates/strata-app/src/basemap_cadastral_jobs.rs:21,55-60,74-76`
  - **Operator / quota risk**: Leaving the map open with 연속지적도 enabled and an expired/unauthorized VWorld key re-fetches up to 96 failing WMS tiles every `RETRY_AFTER = 30 s` indefinitely (`~11,520` wasted API calls per hour), guaranteeing daily quota exhaustion.
  - **Smallest change**: Cap retries per tile (e.g. max 2 retries on transient 5xx/timeout only) and disable the layer with a single user-facing key warning upon 401/403 or XML `ServiceException`.

- **`C4` — C++ HistoryGIS 1919 WMTS probe retries `GetCapabilities` 4 times with 0 ms delay after HTTP 400**
  - **Hosts**: `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/core/BasemapOps.cpp:1034-1046,1509-1511,1539`
  - **Rust**: `none`
  - **Operator / quota risk**: `QStringList(4, historyGisWmtsUri(...))` immediately fires 4 identical `GetCapabilities?apiKey=<key>` calls with 0 ms pause even when the server returns HTTP 400 (`GWC configuration out of synch`) or HTTP 401/403.
  - **Smallest change**: Do not retry on HTTP 401/403; if retrying transient `GWC configuration out of synch`, wait `>= 1–2 s` between at most 2 attempts.

- **`C5` — C++ `ReferenceTiledFetch` retries `Transient` and `Slow` WFS piece downloads with 0 ms delay**
  - **Hosts**: `api.vworld.kr`, `data.kigam.re.kr`, `gis.naas.go.kr`
  - **C++**: `target/cpp-ref/src/core/ReferenceTiledFetch.h:47-51`, `target/cpp-ref/src/core/ReferenceTiledFetch.cpp:34-94`
  - **Rust**: `none`
  - **Operator / quota risk**: Immediately re-issuing a heavy WFS `GetFeature` request (`0 ms` delay) right after a server timeout or HTTP 5xx/429 adds load to a struggling GeoServer/VWorld backend before splitting into 4 more immediate requests.
  - **Smallest change**: Add exponential backoff (`1–2 s`, honoring `Retry-After` on 429) before the per-piece retry in `ReferenceTiledFetch.cpp`.

- **`C6` — C++ `KaHeritageBrowser::scheduleDownloadRetry` retries failed or HTML-returning intranet downloads without any retry limit**
  - **Hosts**: `intranet.gis-heritage.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaHeritageBrowser.cpp:140-141,504-521,834-847,1356-1373,1478-1495`
  - **Rust**: `none`
  - **Operator / quota risk**: If `intranet.gis-heritage.go.kr` returns an HTML error/maintenance page or times out on a dataset, `m_downloadRetryCount` increments forever (`5 s, 10 s, 20 s, 30 s, 30 s...` with UI text 「정상 파일을 받을 때까지 계속 재시도합니다」), repeatedly hitting the intranet under the user's logged-in ID until manually cancelled.
  - **Smallest change**: Enforce a hard cap (e.g. `kMaxDownloadRetries = 2` or `3`) in `scheduleDownloadRetry`, then stop and show `fail(...)` so the user's intranet account never loops unattended.

- **`C7` — C++ `ngii-topographic.js` retries `POST /mn/loginCheck.do` up to 2 times (3 total attempts) even after HTTP 4xx**
  - **Hosts**: `map.ngii.go.kr`
  - **C++**: `target/cpp-ref/data/providers/ngii-topographic.js:221-222,261-268`
  - **Rust**: `none`
  - **Operator / quota risk**: Because `!r.ok` throws `'http'` and the `.catch()` retries while `++state.loginFailures < 3` (`1500 ms, 3000 ms`), HTTP 400/403/404 responses (such as an NGII WAF block) are retried twice instead of stopping immediately.
  - **Smallest change**: Distinguish `r.status >= 400 && r.status < 500` in `ngii-topographic.js:222` and transition straight to `state = 'blocked'` without retrying on 4xx.

---

### Category `d` — Key, Password, Session Cookie, or Account ID Exposed to Log, File Name, Error Text, Screen, or Saved File

- **`D1` — Plaintext API keys, account IDs, and toggle-unmasked passwords rendered on screen**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`, `intranet.gis-heritage.go.kr`, `map.ngii.go.kr`, `www.ngii.go.kr`, `data.kigam.re.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaApiKeyDialog.cpp:20-27`, `target/cpp-ref/src/app/KaTopographicBrowser.cpp:155-156,266`
  - **Rust**: `crates/strata-app/src/modal_paper_account.rs:136-215,233`, `crates/strata-app/src/debug_control.rs:98-100`
  - **Operator / quota risk**: Displaying full `VWorld`, `HistoryGIS`, and `KIGAM` API keys in plain text inputs (and exposing all three portal passwords when `modal:account_show` is toggled or `--snap` captures the modal) leaks keys and credentials into screenshots, screen shares, or shoulder-surfing.
  - **Smallest change**: Use masked echo mode (`PasswordEchoOnEdit` / `.password(true)`) by default for API keys as well as passwords, and remove the debug `modal:account_show` trigger from `debug_control.rs`.

- **`D2` — C++ writes plaintext VWorld `<key>` into `vworld-cadastral.xml` and `SurveyBundle::collectIntoSurvey` copies it into the saved survey folder**
  - **Hosts**: `api.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/BasemapOps.cpp:1215-1285`, `target/cpp-ref/src/core/SurveyBundle.cpp:172-175,183-245`
  - **Rust**: `none`
  - **Operator / quota risk**: `BasemapOps` writes `<ServerUrl>https://api.vworld.kr/req/wms?key=<key>&amp;</ServerUrl>` to `%LOCALAPPDATA%/ka-hgis/vworld-cadastral.xml`, and `SurveyBundle::collectIntoSurvey` copies app-managed `gdal` layer files into `<surveyDir>/가져온자료/VWorld 지적(본번·부번)/vworld-cadastral.xml`, shipping the user's personal VWorld API key inside shared survey folders so recipients burn the owner's quota.
  - **Smallest change**: Exclude `vworld-cadastral.xml` in `SurveyBundle::isRegeneratedCache` (`SurveyBundle.cpp:193-196`) and pass `<GDAL_WMS>...` XML in memory instead of writing `<key>` to disk.

- **`D3` — C++ `SurveyStorage` saves active `wms` layer URIs containing plaintext `VWorld` and `HistoryGIS` API keys into `.gpkg` files**
  - **Hosts**: `api.vworld.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/core/SurveyStorage.cpp:656-689,1234-1239`, `target/cpp-ref/src/core/BasemapOps.cpp:1053,1162,1201,1462,1475,1497`
  - **Rust**: `none`
  - **Operator / quota risk**: Saving a survey clones non-vector layers (`copied.reset(source->clone())`) and embeds their `<datasource>` strings (including `/req/wmts/1.0.0/<key>/...`, `/req/wms?KEY=<key>`, and `hgis.history.go.kr/openapi/get.do?...&apiKey=<key>`) into `<survey>.gpkg` (`qgis_projects` table) and `복구조사.gpkg`, leaking personal API keys whenever `.gpkg` deliverables are shared.
  - **Smallest change**: Exclude basemap/cadastral/HistoryGIS `wms` reference layers (or scrub `<key>`/`apiKey=` from their `<datasource>` URIs and re-inject from local secret storage on load) before `writeEmbedded` in `SurveyStorage.cpp`.

- **`D4` — Plaintext API keys and account IDs in INI/Registry files, plus static-key XOR obfuscation for portable passwords**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`, `intranet.gis-heritage.go.kr`, `map.ngii.go.kr`, `www.ngii.go.kr`, `data.kigam.re.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/core/VworldSettings.cpp:98-109,168-180,222-233`, `target/cpp-ref/src/core/HeritageIntranetSettings.cpp:95-99`, `target/cpp-ref/src/core/KaSecretStore.cpp:36-45,102-111`
  - **Rust**: `crates/strata-secrets/src/key_stores.rs:106-140`, `crates/strata-secrets/src/vworld.rs:42-77`, `crates/strata-secrets/src/accounts.rs:94-120`, `crates/strata-secrets/src/secret_store.rs:21-60,66-94`
  - **Operator / quota risk**: Copying a portable USB folder or backing up `%APPDATA%/ka-hgis/secrets.ini` / `ka-hgis.ini` exposes plaintext API keys, plaintext portal account IDs, and `password_portable` values reversibly XORed with the hardcoded public constant `"ka-hgis-account-v1"`, allowing compromised keys/accounts to be abused elsewhere.
  - **Smallest change**: Store API keys and passwords exclusively in Windows Credential Manager (`CredWriteW`) or OS DPAPI (`CryptProtectData`), and remove the reversible XOR `password_portable` fallback.

- **`D5` — Secret-redaction gaps in C++ `SecretMask::mask` and Rust `SecretMask::redact`**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`, `www.ngii.go.kr`, `hgis.history.go.kr`
  - **C++**: `target/cpp-ref/src/core/SecretMask.h:17-55`, `target/cpp-ref/src/app/KaApplication.cpp:860-866`, `target/cpp-ref/src/core/KaSessionLog.cpp:58`
  - **Rust**: `crates/strata-secrets/src/mask.rs:5-29`
  - **Operator / quota risk**: C++ `SecretMask::mask` only matches 36-char UUIDs in `/req/wmts/1.0.0/<key>/` (leaking non-UUID keys into `[qgis/WMS]` session logs), while Rust `SecretMask::redact` only checks exact `["key=", "KEY=", "pwd=", "password=", "USER_PW="]` and misses `apiKey=` (`hgis.history.go.kr`), percent-encoded `%3D`, `/req/wmts/1.0.0/<key>/` path segments, `usrPwdE=` (`www.vworld.kr`), and `mem_pw=` (`www.ngii.go.kr`).
  - **Smallest change**: Extend `mask.rs` and `SecretMask.h` to redact any `/req/wmts/1.0.0/[^/]+/` path segment, case-insensitive `apiKey=`, percent-encoded `%3D`/`%253D` parameters, `usrIdeE=`/`usrPwdE=`, and `mem_id=`/`mem_pw=`.

- **`D6` — Authenticated intranet HTML dumps and persistent DBpia session cookies written to disk**
  - **Hosts**: `intranet.gis-heritage.go.kr`, `www.dbpia.co.kr`, `ai.dbpia.co.kr`
  - **C++**: `target/cpp-ref/src/core/HeritageHttpClient.cpp:37-51,131,143`, `target/cpp-ref/src/app/KaHwpReportStudio.cpp:23-30,155-159`
  - **Rust**: `none`
  - **Operator / quota risk**: `HeritageHttpClient::dump` writes raw authenticated intranet HTML bodies (which can embed user/org names or CSRF/session tokens) to `<m_dumpDir>/download-list-*.html` and echoes the file path in error text, while `KaHwpReportStudio` writes persistent Chromium session cookies to disk when 「로그인 기억」 is checked.
  - **Smallest change**: Remove `HeritageHttpClient::dump` (or dead `HeritageHttpClient` entirely) and keep `QWebEngineProfile("ka-hgis-dbpia")` on `NoPersistentCookies` unless encrypted by OS DPAPI.

---

### Category `e` — Automatic Re-Login After Failed Login or Expired Session

- **`E1` — C++ `KaHeritageBrowser` re-opens `login.do` and auto-submits intranet credentials unlimited times during download retries**
  - **Hosts**: `intranet.gis-heritage.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaHeritageBrowser.cpp:504-521,533-564,896-924,1284-1286,1478-1495`, `target/cpp-ref/src/app/KaHeritageBrowserPlan.cpp:51-59`
  - **Rust**: `none`
  - **Operator / quota risk**: Although an initial wrong password on `login.do` stops after **1** attempt (`m_waitTicks > 8`), any session expiry, redirect, or HTML error body during `HeritageStage::Download` calls `scheduleDownloadRetry` → `reopenDownloadSession()`, which loads `HeritageIntranetFlow::loginUrl()` and auto-submits `<id>`/`<pw>` **unlimited times (`1, 2, 3, ...` every 5/10/20/30 s until user cancellation)**, risking automatic account lockout on `intranet.gis-heritage.go.kr` (and `KaHeritageBrowserPlan` also performs `1` fresh auto-login per city instead of reusing one session).
  - **Smallest change**: Cap automatic `reopenDownloadSession()` re-logins to **at most 1** per user-initiated job (requiring manual user confirmation thereafter) and reuse a single `KaHeritageBrowser` session across multiple cities in `KaHeritageBrowserPlan`.

- **`E2` — C++ `KaTopographicBrowser` resets `m_automationGeneration` on 5 km scope changes and `NlipMap.do` reloads, allowing repeated automatic SSO logins**
  - **Hosts**: `map.ngii.go.kr`, `www.ngii.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaTopographicBrowser.cpp:586-590,685-703,780-784`, `target/cpp-ref/src/app/KaTopographicScopePanel.cpp:397,425,518-532,627-631`, `target/cpp-ref/data/providers/ngii-topographic.js:183-203`
  - **Rust**: `none`
  - **Operator / quota risk**: While a single `(generation, loginAttempt)` submits `POST /member/process.do` only **once**, leaving 「수치지형도 자동 받기」 checked while panning across 5 km regions — or hitting an `NlipMap.do` reload during `openForm`/`form`/`submit` — increments `m_automationGeneration` and clears `m_loginAttempt`, automatically submitting stored NGII credentials again (**unbounded times across scope changes/reloads**), which can lock the user's NGII account if the password changed.
  - **Smallest change**: Track a session-wide `m_autoLoginFailed` / `m_autoLoginCount` counter on `KaTopographicBrowser` (max **1** failed auto-login until the user edits credentials or clicks retry explicitly).

---

### Category `f` — Re-Fetching Data When a Copy Already Exists on Disk

- **`F1` — Copernicus 30 m DEM pipelines discard raw 1°×1° tiles and re-download overlapping GeoTIFFs (and Rust never saves `dem.tif` to the survey)**
  - **Hosts**: `copernicus-dem-30m.s3.amazonaws.com`
  - **C++**: `target/cpp-ref/src/core/DemDownloadService.cpp:148-205`, `target/cpp-ref/src/app/MainWindowOverlay.cpp:311-314`
  - **Rust**: `crates/strata-net/src/dem_pipeline.rs:76-198`
  - **Operator / quota risk**: Nearby surveys inside the same 1°×1° (~90 km × 111 km) cell repeatedly re-fetch the same 25–45 MB GeoTIFFs because Rust deletes raw tiles from `%TEMP%` and writes `dem.tif` only to `%TEMP%/ka-hgis-reference-*` instead of the survey directory, while C++ re-runs `DemDownloadService` if `<survey>/DEM.tif` is not currently mounted in `QgsProject`.
  - **Smallest change**: Cache verified 1°×1° Copernicus tiles under `%LOCALAPPDATA%/ka-hgis/cache/dem/` and mount an existing `<survey>/DEM.tif` on disk before starting any DEM download in both apps.

- **`F2` — Heritage intranet download ignores `heritage_reuse_recent` in Rust and re-downloads into new `<uuid>` folders when unchecked in C++**
  - **Hosts**: `intranet.gis-heritage.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaHeritageBrowser.cpp:595,1308-1325`, `target/cpp-ref/src/app/KaHeritageBrowserPlan.cpp:17,66-96`
  - **Rust**: `crates/strata-app/src/dialogs_download_print.rs:95-156`, `crates/strata-net/src/heritage_pipeline_http.rs:130-152`
  - **Operator / quota risk**: Rust ignores the UI's `heritage_reuse_recent` toggle completely and overwrites all 6 `{label}.zip` files on every run, forcing expensive server-side shapefile ZIP generation on `intranet.gis-heritage.go.kr` for data already sitting in `<survey>/주변유적/`.
  - **Smallest change**: Honor `heritage_reuse_recent` (defaulting to reuse when a recent non-empty archive exists on disk) before issuing any intranet request.

- **`F3` — Address and place geocoding has no disk or in-memory cache on either side**
  - **Hosts**: `api.vworld.kr`, `nominatim.openstreetmap.org`
  - **C++**: `target/cpp-ref/src/core/LocationSearch.cpp:168-349`
  - **Rust**: `crates/strata-net/src/address_geocode.rs:13-41,64-199`, `crates/strata-net/src/address_vworld_search.rs:17-75`
  - **Operator / quota risk**: Re-searching the same parcel, site address, or town name sends identical queries to `api.vworld.kr` and `nominatim.openstreetmap.org`, wasting VWorld key quota and ignoring Nominatim's caching requirement.
  - **Smallest change**: Add a bounded LRU + SQLite/JSON cache keyed by `(provider, normalized_query)` with a 7–30 day TTL.

- **`F4` — NGII 5 km sheet lookup has no cache in Rust and only an in-RAM `m_scopeKey` in C++**
  - **Hosts**: `map.ngii.go.kr`
  - **C++**: `target/cpp-ref/src/app/KaTopographicBrowser.cpp:684-689,849-857`
  - **Rust**: `crates/strata-app/src/ngii_topographic_state.rs:44-110`, `crates/strata-net/src/ngii_fetch.rs:14-64`
  - **Operator / quota risk**: Rust re-POSTs `combineBufferQuery.do` + `suchiQuery.do` on every click for the same coordinates, and C++ re-POSTs after every app restart even when `<survey>/지형도/index/*.json` already records all sheets for that 5 km buffer.
  - **Smallest change**: Persist the `(x5179_rounded, y5179_rounded, scale)` sheet list in `<survey>/지형도/index/` (or compute standard Korean map sheet codes mathematically offline).

- **`F5` — C++ thematic reference overlays re-download `.gpkg` files when not mounted in `QgsProject` or when clicked from dropdown `QAction`s**
  - **Hosts**: `api.vworld.kr`, `data.kigam.re.kr`, `gis.naas.go.kr`
  - **C++**: `target/cpp-ref/src/app/MainWindowOverlay.cpp:443-483,495-541,553-596`
  - **Rust**: `none`
  - **Operator / quota risk**: If `<survey>/수계도_VWorld.gpkg`, `<survey>/지질도_KIGAM.gpkg`, or `<survey>/토양도_흙토람.gpkg` already exists on disk from an earlier session (or the user picks the dropdown `QAction` on 토양도), `MainWindowOverlay` runs the full WFS/WMS download again and overwrites the existing GeoPackage.
  - **Smallest change**: Check `QFileInfo::exists(<survey>/<name>.gpkg)` before starting `startReferenceDownload`, mounting the existing GeoPackage unless the user explicitly asks to refresh it.

- **`F6` — Cadastral portal download re-fetches all `api.vworld.kr` WFS pages and `www.vworld.kr` HTML catalog pages even when ZIPs already exist on disk**
  - **Hosts**: `api.vworld.kr`, `www.vworld.kr`
  - **C++**: `target/cpp-ref/src/core/CadastralPortal.cpp:231-319`
  - **Rust**: `crates/strata-app/src/cadastral_download_ui_worker.rs:53-146`
  - **Operator / quota risk**: Every click on 「지적도 다운로드」 queries `LT_C_ADSIGG_INFO`, logs into `www.vworld.kr` (in C++), and scrapes `dtmk_ntads_s002.do` pages before discovering at `CadastralPortal.cpp:315-319` / `cadastral_download_ui_worker.rs:144-146` that `<survey>/지적도/원본/<fileNo>-...` is already on disk.
  - **Smallest change**: Save a small manifest (`sig_cd` → cached ZIP path & revision) in `<survey>/지적도/원본/` and skip both WFS and `www.vworld.kr` requests when all intersecting 시·군·구 archives are already present.

- **`F7` — C++ `TilePackService` bypasses `QNetworkDiskCache` and replaces existing `.mbtiles` from scratch, and `KaGeminiBridgeClient` has no persistent search cache**
  - **Hosts**: `api.vworld.kr`, `xdworld.vworld.kr`, `gibs.earthdata.nasa.gov`, `www.dbpia.co.kr`
  - **C++**: `target/cpp-ref/src/core/TilePackService.cpp:54-89,168-256`, `target/cpp-ref/src/app/MainWindowOffline.cpp:259`, `target/cpp-ref/src/app/KaGeminiBridgeClient.cpp:94-95,114-250`
  - **Rust**: `none`
  - **Operator / quota risk**: `TilePackService::build` opens a standalone GDAL `<GDAL_WMS>` dataset into a fresh `map.mbtiles` temp file rather than reading QGIS's 2 GB `QNetworkDiskCache` or existing tiles in `<survey>/배경지도_오프라인.mbtiles`, re-downloading up to 20,000 tiles; similarly, `KaGeminiBridgeClient` clears `m_seenNodeIds` on each run and repeats identical 38-request DBpia harvests.
  - **Smallest change**: Seed `map.mbtiles` from existing local tiles / `QNetworkDiskCache` before fetching missing coordinates, and cache DBpia search responses on disk by query string.
