# C++ 앱 · QGIS 학습 정리 (Rust 포팅 기준)

작성 2026-11-10. 지시: 「qgis 와 c++앱을 학습 먼저하라. 전부다.」
이 문서는 `D:\hgis`(C++20/Qt6/QGIS, HEAD `d7afcc8`)를 읽고 **Rust 로 옮길 때
반드시 알아야 하는 것**만 추린 것이다. 새 작업을 시작할 때 이 파일부터 읽는다.

---

## 1. 규모와 구조

| 항목 | C++ (`D:\hgis`) | Rust (`D:\hgis rust`) |
|---|---|---|
| 파일 | 587 (.cpp/.h) | 463 (.rs) |
| 줄 | 97,915 | 59,732 (61%) |
| crate | — | 11 |

C++ 디렉터리:
- `src/core` (287 파일) → 정적 라이브러리 `ka_core`. 창/UI 코드 없음.
- `src/app` (300 파일) → 실행 파일 `ka-hgis`. MainWindow + Ka* 위젯 + 지도도구.

`MainWindow` 는 **16개 파일로 쪼개져** 있다:
`MainWindow.cpp` + `MainWindow{Align,Cadastral,ContextMenus,Downloads,Editing,Erase,Export,Files,Offline,Overlay,Ribbon,Session,SurveyContour,Topographic,Undo}.cpp`

핵심 core 모듈: `LayerOps`, `BasemapOps`, `SurveyStorage`/`SurveySession`/
`SurveyProjectFactory`, `ExportService`, `LayoutService`, `ChecklistEngine`,
`ProjectStateBuilder`, `Heritage*`, `Topographic*`, `Cadastral*`, `PolygonErase`,
`FeaturePick`.

---

## 2. QGIS 의존 — Rust 가 직접 구현해야 하는 것

C++ 은 **렌더링·좌표변환·파일 I/O 를 전부 QGIS 에 맡긴다.** Rust 에는 QGIS 가 없으니
같은 일을 직접 해야 한다. 사용 빈도(실측):

| QGIS 클래스 | 호출 | Rust 대응 |
|---|---|---|
| `QgsVectorLayer` | 906 | `strata-storage`(GPKG) + `strata-canvas2d`(그리기) |
| `QgsGeometry` | 462 | `strata-geo` (polygon_ops, buffer_round, snap_index …) |
| `QgsCoordinateTransform` | 308 | `strata-geo/crs_transform` (TM 투영 직접 구현) |
| `QgsMapCanvas` | 235 | `strata-canvas2d/viewport` + `strata-app/gui_tab_map*` |
| `QgsRasterLayer` | 146 | `strata-app/basemap_tiles`(타일) + `strata-paleo`(DEM) |
| `QgsFeatureIterator` | 123 | `rusqlite` 직접 조회 |
| `QgsVectorFileWriter` | 116 | `strata-layout`/`strata-storage` (SHP 쓰기 직접) |
| `QgsSymbol` | 109 | `strata-canvas2d/painter_*` (egui 페인터) |
| `QgsPrintLayout` | 58 | `strata-layout` (자체 조판) |
| `QgsLayoutExporter` | 13 | `strata-layout` (PDF 직접 생성) |
| `QgsMapRenderer` | 12 | 없음(egui 가 그림) |

**결론: Rust 는 QGIS 를 대체하는 것이 아니라 QGIS 가 하던 일을 직접 한다.**
따라서 "C++ 코드를 그대로 옮긴다"가 아니라 **"C++ 이 QGIS 를 부려 얻던 결과를
Rust 로 재현한다"** 가 올바른 목표다.

---

## 3. 데이터 흐름 (docs/architecture/data-flow.md)

```
시작 → 홈(자동 복원 없음)
  ├ 새 조사 → SurveyProjectFactory → survey.gpkg (7 도메인 테이블 + embedded workspace)
  ├ 열기   → SurveyStorage::readEmbedded + LayerOps 복구/재장착
  ├ 그리기 → KaCaptureMapTool → LayerOps::ensureDomainLayer → gpkg
  └ Ctrl+S → SurveyStorage::persistWorkspace(세대 복사·검증·게시) → gpkg
gpkg → ProjectStateBuilder → ChecklistEngine ← drawing_checklist.v1.json
     → ExportService → EPSG:5179 SHP + 조사도면.pdf + MANIFEST.sha256
```

**도메인 테이블 7종**(정체성은 `ka_hgis/layer_key` 속성, 한국어 제목은 라벨):
`survey_area`, `feature_poly`, `feature_line`, `section_line`, `control_points`,
`artifact_point`, `trial_trench` — 스키마 `data/schemas/ka_hgis_layers.yaml`.

배경지도(VWorld·지형·토양·유산)는 **조사 데이터가 아니다.** 다운로드한 지적도와
VWorld 지적은 레이어 트리 **맨 아래(root)** 에 둔다.

---

## 4. 지적도(연속지적도) — 오늘 확인한 사실

Rust 에서 위성은 되는데 지적이 안 보여 원인을 추적한 결과.

### C++ 이 하는 일
1. `BasemapOps.cpp:1189-1212` `makeVworldWmsUri` 로 **WMS URI** 를 만든다.
   - `VERSION=1.3.0`, `CRS=EPSG:3857`, `FORMAT=image/png`, `TRANSPARENT=true`
   - `LAYERS=lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun`
   - `http-header:referer=https://localhost` **필수** (DOMAIN 은 절대 넣지 않는다)
2. 그 URI 를 `addXyzBasemap`(`:860`) → `tryCreateXyzLayer` 로 넣어
   **타일 레이어**로 만든다. 즉 C++ 도 WMS 를 타일처럼 쓴다.
3. 레이어 이름 `VWorld 지적(본번·부번)`, 실패 시 본번/부번 따로 시도.
4. **축척 조정** (`addVworldCadastralMap:1382-1384`):
   ```cpp
   const double s = canvas->scale();
   if (s > 80000.0 || s < 200.0)
     canvas->zoomScale(25000.0, true);   // 1:25,000 으로 당긴다
   ```
   그리고 지적 레이어는 `setCrs(EPSG:3857)` 로 고정(`:1379`).
   다른 경로(`:712`)는 `if (scale > 80000.0) zoomScale(5000.0)`,
   `:969` 는 `zoomScale(10000.0)` 을 쓴다. → **80,000 이 상한**이라는 점이 일관된다.

### 실측으로 확인한 제약 (2026-11-10)
- `VERSION=1.1.1` + `SRS=` → **`INVALID_RANGE` XML** (HTTP 200). 1.3.0 + `CRS=` 만 통한다.
- 레이어 이름이 틀리면 `LayerNotDefined`. `lt_c_upisuq` 는 없는 이름이었다.
- 올바른 조합으로 256px PNG 가 실제로 온다(필지선·지번 확인).
- 512px 로 요청해도 정상 PNG 가 온다.

### Rust 구현 상태
- `basemap_cadastral.rs` — 위 규칙 그대로 URL 생성, curl 로 받아 PNG 검증.
- 화면 전체를 한 장으로 받으면 1px 흔들림에도 매번 새로 받아 화면이 빈다.
  → **위성과 같은 격자 방식**(`CADASTRAL_CELL_M = 1000m`, 512px)으로 바꿨다.
- `cadastral_visible_at_scale()` — C++ 과 같이 **1:80,000 보다 넓으면 그리지 않는다**
  (사용자 제안 2026-11-10 과 같은 방향).

---

## 5. 위성 타일 — C++ 규칙

`BasemapOps.cpp:1161-1175`:
- 키 있음: `https://api.vworld.kr/req/wmts/1.0.0/<키>/Satellite/{z}/{y}/{x}.jpeg`
  — **z/y/x 순서**(XYZ 표준과 다르다).
- 키 없음: `https://xdworld.vworld.kr/2d/Satellite/service/{z}/{x}/{y}.jpeg`
- `zmin=6, zmax=19, crs=EPSG:3857`
- VWorld 는 키 만료 시 **HTTP 200 + XML** 을 준다 → 앞 4바이트로 이미지 여부를
  반드시 검사해야 한다(안 하면 깨진 타일이 칠해진다).

---

## 6. 절대 깨면 안 되는 제품 규칙 (CLAUDE.md)

- C++20/Qt6 + OSGeo4W qgis-dev. **QGIS 나 PROJ/GDAL/렌더링/CRS 변환을 포크하거나
  재구현하지 않는다** — 이건 C++ 쪽 규칙. Rust 는 애초에 QGIS 가 없어 직접 한다.
- `qgis/parallel_rendering=false` 유지. WMS/XYZ 타일 로딩 중
  `stopRendering`/`clearCache`/`refresh` 금지(0xc0000005 크래시 원인).
- CRS: 작업 좌표계 **EPSG:5187(기본) 또는 5186**, 5179 로 가정하지 않는다.
  제출은 EPSG:5179 SHP + PDF + MANIFEST.sha256.
- 키는 절대 하드코딩 금지. `loadApiKey` 순서:
  `%LOCALAPPDATA%\ka-hgis\ka-hgis-vworld.ini` → `HKCU\Software\ka-hgis` →
  환경변수 `VWORLD_API_KEY` → gitignored `config/secrets.ini`.
- 지적 WMS: `crs=EPSG:3857` + KEY, **DOMAIN 전송 금지**, CRS 후보 4326→3857→900913.
- 새 조사/저장은 `preferredSurveyDir()` 에서 시작, 바탕화면 금지.
- 사용자 원본 조사 데이터는 절대 수정하지 않는다.
- 이름 Strata, 리본 한 줄, 맑은 고딕.

---

## 7. 이 문서를 어떻게 쓸 것인가

1. **기능을 만들기 전에** C++ 에서 그 기능이 어느 파일·함수에 있는지 먼저 찾는다
   (`Select-String -Path src\core\*.cpp,src\app\*.cpp -Pattern '<함수명>'`).
2. C++ 이 QGIS 로 "얻는 결과"를 적는다(무엇이 화면에 어떻게 보이는가).
3. Rust 에서 그 결과를 **직접 재현**한다.
4. **앱을 실행해 스크린샷으로 확인**한다(`--snap`). 코드만 보고 "됐다"고 하지 않는다.
5. C++ 앱과 나란히 비교한다. C++ 앱 실행 방법은 아래.

### C++ 앱 실행 (검증용)
```powershell
cd D:\hgis
. .\scripts\dev-env.ps1          # OSGEO4W_ROOT=D:\OSGeo4W 등 설정 (필수)
Start-Process D:\hgis\build\Release\ka-hgis.exe -WorkingDirectory D:\hgis\build\Release
```
- 그냥 실행하면 `0xC0000135`(DLL 없음)로 죽는다. **반드시 dev-env 를 먼저 점한다.**
- 이 PC 는 4K 모니터 + 200% 배율이라 창이 화면을 넘는다.
  `\.git\fit_capture.ps1` 로 작업영역에 맞춘 뒤 캡처한다.

### Rust 앱 실행 (검증용)
```powershell
cd 'D:\hgis rust'
cargo build --release -p strata-app --bin ka-hgis
.\target\release\ka-hgis.exe --snap .git\snaps --snap-steps "@new_survey_go:10_map"
```
`--snap` 은 앱이 스스로 화면을 PNG 로 저장하고 끝난다(DPI·포커스 문제 없음).
