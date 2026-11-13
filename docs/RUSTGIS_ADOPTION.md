# RustGIS 도입 대기열 (R1)

지시 R1(hgis-2e, 2026-11-13). 사용자: 「rustgis 필요한 기능 다 사용해서 진행해」, 「우리는 문화재 gis의 독자적인걸 만든다」.
이 문서는 **아직 아무것도 연결하지 않는다**(Cargo 변경 없음, 앱 동작 변경 없음). 우리가 손으로 짠 엔진 기능 가운데
RustGIS 로 바꿀 수 있는 것과 그 차례, C++ 과 같은지 확인할 방법을 적는다.

- 대상: [opengeos/RustGIS](https://github.com/opengeos/RustGIS) v0.6.0 (커밋 `6db08fe`, 2026-11-08), Apache-2.0,
  edition 2024, rust-version 1.90(우리 1.91.1). crates.io 에 없어 git 의존을 **커밋 해시(rev)로** 고정한다.
- 붙이는 자리: `crates/strata-engine` 하나(연결부). 앱·저장 크레이트는 `rustgis-*` 를 직접 쓰지 않는다. `rustgis-ui-egui` 는 쓰지 않는다.
- 좌표계는 PROJ(E0)에 남는다. RustGIS 의 proj4rs 길은 쓰기에 쓰지 않는다(화면 표시 비교만 숫자로 남길 수 있다).

## 지금 막혀 있는 것 (2026-11-13 확인)

| 막힘 | 사실 | 풀리는 때 |
|---|---|---|
| SQLite 두 벌 | `rustgis-data` 는 rusqlite 0.41(→ libsqlite3-sys 0.39)을 늘 쓴다(`[features]` 없음, 모든 태그). 우리는 rusqlite 0.32(→ libsqlite3-sys 0.30.1)이고, E0 의 proj-sys 0.27.0 은 libsqlite3-sys `>=0.28,<0.36` 만 받는다. Cargo 는 `links = "sqlite3"` 크레이트를 하나만 허락한다 — 시험 크레이트에서 `cargo generate-lockfile` 이 「Only one package in the dependency graph may specify the same links value」로 멈춘다. | S2: rusqlite 0.41.x + proj 0.32 / proj-sys 0.28(libsqlite3-sys `>=0.28,<1.0`) |
| 덜어 낼 수 없음 | `rustgis-data` 는 데스크톱에서 postgres·mysql·tiberius·tokio·rustls/ring·parquet·ureq 를 늘 끌어온다. `rustgis-map`·`-render`·`-tiles` 는 모두 `rustgis-data` 에 기댄다. | 고칠 수 없음(포크하지 않는다) — 빌드 시간·exe 크기를 R1b 에서 잰다 |
| 의존 나이(rules_v1 2주) | v0.6.0 2026-11-08, v0.5.0 10-31, v0.4.0 10-24. proj 0.32·proj-sys 0.28 은 2026-11-12. rusqlite 0.41.1(10-28)·libsqlite3-sys 0.39.1(10-25)은 이미 통과. | v0.6.0 은 2026-11-22, proj 0.32 는 2026-11-26 |
| `rustgis-geom` 만은 됨 | `geographiclib-rs 0.2.7`(이미 우리 Cargo.lock 에 있음) + serde 뿐 — 새 레지스트리 크레이트도 충돌도 없다. | 쓰는 지시가 생길 때 그날 2주 지난 가장 새 태그로 |

결정(hgis-2e): R1 = 이 문서만. **S2**(2026-11-26 이후, 단독, 2A·2B·2C 파일 대조 + E0 변환 goldens + 내장 proj.db + ini 시험) → **R1b**(`rustgis-data`·`-map`·`-render`·`-tiles` 를 그날 나이를 넘긴 태그로). R2(제출 SHP)는 RustGIS 를 기다리지 않는다.

## 영역별 대기열

RustGIS 쪽은 v0.6.0 **코드**(README·ROADMAP 의 계획이 아니라)에서 확인한 것만 적었다. 경로는 RustGIS 저장소 기준.
「읽기/쓰기」는 RustGIS 가 그 일을 읽는지 쓰는지, 「필요」는 geom 만으로 되는지 `rustgis-data`(S2 뒤) 가 있어야 하는지.
우리 쪽 「가짜」= 화면은 「했습니다」인데 실제 파일·결과가 없다(대조표 docs/RUST_PARITY_CHECKLIST.md).

| 영역 | 우리가 손으로 짠 파일 (지금 상태) | RustGIS 가 가진 것 | 읽기/쓰기 · 필요 | C++ 과 같은지 보는 문(gate) | 판단 · 지시 |
|---|---|---|---|---|---|
| 제출 SHP(EPSG:5179) | `strata-storage/src/shp_writer.rs`(가짜: .shp 가 WKT 글, .shx·.dbf 4바이트, .prj 만 진짜), `strata-check-export/src/submit_export.rs`(가짜: 조사 자료 대신 100 m 네모), `submit_manifest.rs`(진짜, shp·pdf·README 만 셈) | `rustgis-data` `crates/data/src/io.rs` `write_path_in`·`shapefile_parts_in`(shapefile 0.9 / dbase 0.8). 필드 N(18,0)·N(24,d)·L·C(≤254바이트), 날짜형 없음, 이름 10바이트 자름, `.cpg` 는 늘 `UTF-8`(io.rs:2660), .prj 는 PROJ 문자열로 만든 WKT1. 도형은 WGS84 경위도로 들고 있다가 proj4rs 로 바꿔 쓴다(io.rs:394, 실패하면 말없이 NaN). | 쓰기 · data | C++ 실물 꾸러미 `docs/senior/ref-files/cpp-submit-package-demo/`(C++ ExportService 를 직접 불러 만듦, `.cpg`=UTF-8): `.cpg`·`.prj` 바이트 같음, `.dbf` 머리말·필드 서술자 바이트 같음, `.shp`·`.shx` 는 ogrinfo 로 도형·필드·인코딩 대조, MANIFEST.sha256 | **RustGIS 안 씀** — 쓰기 좌표 변환이 PROJ 가 아니고(규칙 위반) 경위도를 거쳐 정확도가 깎인다. R2 는 georust `gdal`(OSGeo4W gdal-dev 3.14) 과 순수 Rust `shapefile` 크레이트 가운데 바이트 문을 통과하는 쪽(hgis-2e). RustGIS 의 DBF 필드 매핑만 참고. → **R2** |
| 조사도면 PDF · 배치 | `strata-layout/src/sheet_pdf_export.rs`(가짜: 빈 네모 5개, xref 위치 없음), `sheet_layout.rs`·`sheet_scalebar.rs`·`sheet_decorations.rs`(계산은 진짜, 일부는 부르는 곳 없음), 앱 `gui_tab_drawing_sheet*.rs`(화면만, 범례는 이름으로 박아 둔 목록), 「인쇄」는 글만(`dlg_drawing_print_actions.rs`) | `rustgis-map` `crates/map/src/layout.rs` `Layout`·`ElementKind::{MapFrame, NorthArrow, ScaleBar, Legend, Text, Picture, Table, Chart}`·`PAGES`(A5–A0, mm), `rustgis-render` `crates/render/src/layout.rs` `render`·`render_drawing`, `crates/render/src/pdf.rs` `write`(손으로 짠 PDF, 글리프 윤곽으로 만든 Type 3 글꼴 + 유니코드 대응표). 한글은 `C:\Windows\Fonts\malgun.ttf` 먼저(`fonts.rs` `cjk_font`), 글자 모양 다듬기(shaping) 없음 | 쓰기 · data | C++ `LayoutService`·`QgsLayoutExporter`(PdfExportSettings.cpp)가 만든 같은 조사의 조사도면.pdf: 용지 크기·요소 자리(mm), 글자 뽑기(ToUnicode)로 같은 글, 150 dpi 래스터 비교 | 쓸 만함 — 요소 종류가 C++ 배치와 거의 겹친다. 맑은 고딕 Type 3 글꼴이 C++ PDF 와 같은 글자를 뽑아 주는지가 첫 문. → **R3**(R1b 뒤) |
| 지도 타일 XYZ/WMTS/WMS · 캐시 | `strata-app/src/basemap_tiles.rs`(VWorld WMTS 주소, 키 없으면 xdworld), `basemap_fetch.rs`(curl, 디스크 캐시 `%LOCALAPPDATA%\ka-hgis\tiles`, 만료·LRU 없음), `basemap_cadastral*.rs`(WMS 1.3.0 EPSG:3857), `basemap_paint.rs`·`basemap_mesh.rs`(손 TM 으로 다시 투영), `strata-net` 의 같은 일 하는 파일은 안 쓰임 | `rustgis-tiles` `crates/tiles/src/template.rs` `expand`({z}{x}{y}{-y}{s}{quadkey}{bbox-epsg-3857}), `source::Spec`; WMS GetMap·WMTS GetTile/REST 는 `rustgis-data` `crates/data/src/ogc.rs`; 캐시 `crates/tiles/src/cache.rs` 는 **메모리만**(데스크톱 384 MiB LRU), 디스크에 안 씀; ureq + rustls | 읽기 · data | C++ 과 같은 요청 주소(제품 규칙: 키 있는 api.vworld.kr WMTS 먼저, 지적 WMS 는 `crs=EPSG:3857` + KEY, DOMAIN 안 보냄), 시니어 C++ 캡처와 화면 비교, 디스크 캐시는 C++ `QNetworkDiskCache`(KaApplication.cpp:817-823)처럼 남긴다 | 받기·주소 틀은 쓸 만함, 디스크 캐시는 우리 것을 남긴다. 주의: `crates/tiles/src/source.rs:183` 이 다시 읽기 실패 때 키가 든 주소를 그대로 `log::warn!` 한다 — 쓰게 되면 키 주소가 로그로 가지 않게 막아야 한다. → **R4** |
| 라벨 놓기 | `strata-app/src/gui_tab_map_survey_labels.rs` `place_labels`(욕심쟁이 겹침 검사, 못 놓으면 버림), `survey_paint_layers.rs` `polygon_centroid`(오목한 면이면 밖에 놓일 수 있음), `strata-layout/src/badge_placer.rs`(도면), `sheet_labels.rs`(시험만) | `rustgis-render` `crates/render/src/lib.rs` `place_labels_with_previews`(**비공개**: 우선순위·충돌 상자·격자 색인·대안 자리), `line_labels.rs`·`label_places.rs`(선 따라, 꺾임 25° 넘으면 건너뜀), `rustgis-data` `Geometry::label_point`(중심이 안이면 중심, 아니면 가장 넓은 가로 구간) | 쓰기(화면) · data | C++ 은 QGIS PAL(`QgsPalLayerSettings`). 같은 축척 C++ 캡처와 라벨 자리·보인 라벨 수를 픽셀 도구로 대조 | 놓기 함수가 비공개라 그대로는 못 부른다 — `label_point` 와 선 라벨만 먼저, 충돌 놓기는 공개 API 가 생기거나 R1b 때 다시 본다. → **R5** |
| DXF 가져오기 | `strata-storage/src/cad_dxf_input.rs`(단순 code/value 훑기, 시험용), `strata-net/src/topo_scan.rs` `read_dxf`(레이어·범위만, 쓰임), `dlg_cad_import_notice.rs`(글만) | `rustgis-data` `crates/data/src/dxf.rs` `read`·`layers`·`Options`(ASCII 만; POINT·LINE·(LW)POLYLINE·CIRCLE·ARC·ELLIPSE·SPLINE·TEXT/MTEXT·HATCH·3DFACE·INSERT). UTF-8 아닌 글은 Latin-1 로 읽는다 — **CP949 한글이 깨진다**. 내보내기 없음, DWG 는 바깥 ogr2ogr | 읽기 · data | C++ `CadDrawingReader.cpp`·`CadDwgConverter.cpp`(GDAL/OGR). 국토지리정보원 DXF 도엽 같은 파일로 ogrinfo 와 레이어 이름·도형 수·글(한글) 대조 | 도형은 쓸 만함, 글은 CP949 를 우리가 다시 풀어야 한다(또는 업스트림 고침). → **R6** |
| GeoTIFF 내보내기 | `strata-paleo/src/borehole_geotiff.rs` `write_float32_geotiff`(진짜, tiff 크레이트), `strata-layout` `export_geotiff_with_cancel`(**가짜**: 픽셀 없는 머리말, 리본은 「GeoTIFF 저장 완료」), `strata-net/src/dem_pipeline.rs` `mosaic_tiles`(GDAL 명령), 픽셀 읽기 없음 | `rustgis-data` `crates/data/src/raster/geotiff.rs` `read`·`read_file`·`read_sparse`(Deflate·LZW·PackBits·JPEG·ZSTD, GeoKey EPSG, GDAL_NODATA), `write`(Deflate, 북쪽 위, f32 1밴드 또는 RGB(A)), `cogwrite.rs` `write` | 읽기·쓰기 · data | C++ GDAL GTiff(DemDownloadService.cpp:256 `TILED=YES`·`COMPRESS=DEFLATE`, DemHillshadeFile.cpp): `gdalinfo -checksum` 으로 크기·지오트랜스폼·좌표계·nodata·압축·픽셀 합 대조 | 쓸 만함(읽기가 특히 — 지금 우리는 받은 dem.tif 를 다시 못 읽는다). 타일 옵션이 C++ 과 다르면 문에서 걸린다. → **R7** |
| 측정(면적·길이) | 모두 평면: `strata-canvas2d/src/cad_session.rs`(신발끈 공식), `strata-app/src/gui_tab_map_measure.rs`, `survey_measure.rs`(엔진 GEOS `wkt_area`·`union_area`) | `rustgis-geom` `crates/geom/src/lib.rs` `wgs84_length_m`·`wgs84_distance_m`·`wgs84_ring_perimeter_area_m2`(geographiclib 타원체), `geodesic_*`(이름과 달리 구면). 투영 좌표계의 평면 측정 없음 | 읽기 · **geom 만** | C++ 은 평면이다: `MeasureOps::pinPlanarEllipsoid`(MeasureOps.cpp:62-70)가 타원체를 NONE 으로 고정 | **안 씀** — 타원체 값은 C++ 과 숫자가 달라진다. GEOS 평면을 남긴다. R8 은 C++ 이 타원체로 바뀔 때만. |
| 래스터 DEM(음영·경사·등고선·IDW) | `strata-paleo/src/terrain_hillshade.rs`·`terrain_interp.rs`(진짜, 앱에선 만든 가짜 DEM 에만), `borehole_idw.rs`·`borehole_isopach.rs`(시추 도면, 쓰임), `strata-geo/src/contour_tin.rs`·`strata-terrain3d/src/trench_hillshade.rs`·`strata-paleo/src/ngii_dem.rs`(부르는 곳 없음), `dlg_survey_contour.rs`(글만, 「점 248개」 박아 둠) | `rustgis-map` `crates/map/src/raster.rs` `hillshade`(Horn, 그릴 때), `rustgis-tiles` `dem.rs`; `rustgis-analysis` `rasterops.rs` `slope`·`aspect`·`hillshade`·`idw`(칸 크기 도 단위), `kriging.rs`. **등고선은 RustGIS 어디에도 없다** | 읽기·쓰기 · data(+analysis) | C++ `DemHillshadeFile.cpp` `hillshadeHorn`(자체 Horn → GTiff), `QgsHillshadeRenderer`(BasemapOps·DemAnalyzer): 같은 DEM 으로 음영 래스터 픽셀 차 ≤1, 등고선은 C++ 결과와 선 수·높이 대조 | 음영은 쓸 만함(R7 읽기와 함께), 등고선은 우리 것(또는 GDAL)을 남긴다. → **R9** |
| GeoPackage | `strata-storage/src/survey_gpkg_*.rs`(C++ 파일과 구조 같음, 세대 사본 저장 — 2A·2B·2C 검수 통과) | `rustgis-data` `crates/data/src/source/gpkg.rs` `GpkgSource`, `write`·`write_table_in`(`GpkgMode::{NewFile, AddTable, ReplaceTable}`), `gpkg/edit.rs` `EditTable` | 읽기·쓰기 · data | 2A·2C 의 파일 대조 | **조사 파일에는 안 씀**(사용자 자료 안전 — C++ 과 같은 저장 길이 이미 있다). 다른 GPKG 를 읽기만 하는 가져오기에서 다시 본다. |
| 좌표계 | E0: PROJ 9.6.2(`strata-engine`). 남은 손 TM: `strata-geo/src/crs_transform.rs` `lonlat_to_tm`·`tm_to_lonlat`(배경지도·검색·새 조사 등, 5174 를 GRS80 으로 계산) | `rustgis-data` `crates/data/src/raster/crs.rs` `epsg_proj4`·`Transformer`(proj4rs + crs-definitions; 5179·5186·5187 은 `epsg_axes.rs:138-139` 목록에 있음, 그리드 이동 없음) | — | E0 goldens(gdaltransform) | **쓰기에는 안 씀**(규칙). 남은 손 TM 은 E1 이 엔진(PROJ)으로 옮긴다. proj4rs 와 PROJ 의 화면 표시 비교 숫자는 R1b 에서 남긴다. |

## RustGIS 를 붙일 때(R1b) 지킬 것

- `crates/render/build.rs:38` 은 빌드할 때 `curl` 로 `https://assets.rustgis.app/data` 에서 GeoJSON 을 받는다(`RUSTGIS_DATA_DIR` 가 없으면). CI 와 오프라인 빌드는 이 폴더를 저장소 밖에서 받아 고정하거나 네트워크를 허락해야 한다 — 받은 파일의 해시를 적어 둔다.
- 키가 든 주소: RustGIS `source.rs:183` 의 경고 로그(위). 우리 쪽도 지금 `basemap_fetch.rs:119-133`·`basemap_cadastral.rs:190`·`strata-net/src/address_geocode.rs` 가 키 든 주소를 `curl` 명령줄로 넘겨 프로세스 목록에 보인다 — R4 에서 HTTP 크레이트(ureq)로 바꾸면 함께 없어진다.
- `rustgis-data` 는 늘 SQLite(C)·zstd(C)·parquet·postgres·mysql·tiberius(Windows 인증)·tokio·sysinfo 를 끌어온다 — R1b 보고에 빌드 시간·exe 크기 차이를 적는다.
- `unsafe` 없음(RustGIS 작업공간이 `unsafe_code = "forbid"`).
- 연결부 밖으로 `rustgis-*` 형식이 새지 않게 `strata-engine` 이 우리 형식(WKT·좌표 배열)으로 바꿔 내놓는다(E0 과 같은 방식).

## 라이선스

- RustGIS 는 Apache-2.0 이다. 고지 파일: 저장소의 `LICENSE`, `ATTRIBUTION.md`(자산 출처), `THIRD-PARTY-LICENSES.txt`
  (RustGIS 가 쓰는 크레이트 고지, `cargo xtask notices` 로 만든다). EPSG 표(crs-definitions)는 IOGP 소유 고지와 이용 조건 링크가 함께 있어야 한다.
- 우리 앱은 GPL-2.0-or-later 다. Apache-2.0 은 GPLv2 와 함께 배포할 수 없고 GPLv3 와는 된다 — RustGIS 를 링크해 배포하면
  **합친 프로그램은 GPL-3.0-or-later 로** 배포한다(우리 코드의 「or later」를 쓴다). QGIS(C++ 참조판)는 링크하지 않으므로 상관없다.
- `THIRD_PARTY_NOTICES.md` 에는 실제로 빌드에 들어갈 때(R1b 또는 geom 을 쓰는 지시) 그 크레이트와 위 파일들의 위치를 적는다. 지금은 아무것도 링크하지 않으므로 적지 않는다.
