# Strata (ka-hgis) 100% 순수 Rust 전환 아키텍처 설계서

- **작성일**: 2026-11-07
- **기준 저장소**:
  - 최신 기능 기준 (`kyi/main`): `https://github.com/kyi0109657401-hub/hgis.git` (`95d15a9`, 고지형분석 4단계 엔진·국토지리정보원 DEM 합성·시추주상도·KIGAM 연동)
  - 통합 기능 기준 (`origin/main`): `https://github.com/kesekikwon-netizen/gogohak1.git` (`c04aa37`, `strata-3d` 연동·HWPX 보고서·시굴 전 종합 평가·RAG/DBpia 계약)
  - 자매 Rust 레퍼런스: `strata-arch3d` (Rust `1.91.1`, `eframe 0.31` / `egui 0.31` / `wgpu 24.0.5`, 멀티 크레이트 워크스페이스)

---

## 1. 전환 목표 및 핵심 원칙

1. **C++ / Qt 6 / OSGeo4W(QGIS·GDAL C++) 완전 퇴출 및 100% Rust 단일 실행 파일화**:
   - 1GB가 넘는 무거운 OSGeo4W 의존성과 `Qt6Xml.dll` 누락 문제, QGIS 멀티스레드 렌더링 충돌(`0xc0000005`), OGR GPKG 파일 잠금(`MoveFileEx` 실패), `QGraphicsScene` 스냅 크래시, `QgsLayout` 러버밴드 크래시를 근본적으로 제거한다.
   - 설치본·포터블 모두 외부 DLL 폴더 없이 단일 네이티브 실행 파일(`ka-hgis.exe`, 약 30~50MB)로 동작하게 한다.
2. **기존 구조·화면 디자인·도메인 규칙 100% 계승**:
   - **7대 조사 도메인 레이어** (`survey_area`, `feature_poly`, `feature_line`, `section_line`, `control_points`, `artifact_point`, `trial_trench`, 스키마 버전 3) 및 `ka_hgis/layer_key` 정체성 규칙 유지.
   - **4대 레이어 트리 구역** (`조사 데이터` / `지적도` / `주변·참조` / `배경 지도`) 및 읽기 전용 참조 레이어 보호, VWorld 지적 그림 스냅 제외 규칙 유지.
   - **작업 좌표계(`EPSG:5187` 기본 / `EPSG:5186`) 및 제출 꾸러미(`EPSG:5179` SHP + `조사도면.pdf` + `MANIFEST.sha256` + `README_submit.txt`)** 유지.
   - **UI 디자인(`KaTheme` 토큰 & 「조용한 도판」 `paperLook`)**: 1열 리본(`조사`·`기록`·`자료 받기`·`배경 지도`·`정합`·`내보내기`·`기타` + 우측 주소·지번 검색 `KaAppBar`), 보조 툴바(`subToolbar`), 좌측 레이어·파일함, 중앙 탭(`홈`·`지도`·`고지형분석`·`도면`·`입체지형`·`보고서`), 우측 인스펙터(`속성 조사카드`·`스타일`·`배경 지도 빠른 카드`) 구성을 그대로 계승한다.
3. **사이트 로그인·API 키·암호화 저장소 무중단 호환**:
   - 기존 사용자가 저장해 둔 `%LOCALAPPDATA%\ka-hgis\` 아래의 모든 설정 파일(`ka-hgis-vworld.ini`, `heritage-account.ini`, `ngii-account.ini`, `secrets.ini`), Windows 레지스트리(`HKCU\Software\ka-hgis`), Windows DPAPI 암호화(`entropy = "ka-hgis-account-v1"`), 포터블 XOR-Base64(`password_portable`)를 **재입력 없이 그대로 읽고 쓴다**.
4. **C++의 한계를 뛰어넘는 최신 Rust 기술 도입**:
   - **2D/3D 통합 GPU 렌더링 (`wgpu 24` + `vello`/`lyon`)**: 2D 대용량 벡터·타일 지도와 3D 입체지형(`strata-3d`의 쿼드트리 LOD + Nishita 하늘 + GTAO + AgX 색보정)을 외부 프로세스 파이프 없이 하나의 앱 안에서 60fps로 즉시 렌더링한다.
   - **초고속 병렬 공간 연산 (`rayon` + `geo` + `rstar` + `spade`)**: 반경 10km 4단계 고지형 분석(IDW, 1D/2D EDT 하천거리, 적분영상 TPI, 다방향 음영기복도), 수치지형도 다중 도엽 등고선 TIN/DEM 합성, 36분할 정밀 원형 버퍼(500m/1000m)를 모두 순수 Rust 병렬 엔진으로 구동한다.

---

## 2. 최신 Rust 기술 스택 매핑 (C++ → Rust)

| 기존 C++20 / Qt 6 / QGIS 구성요소 | 전환할 최신 Rust 기술 스택 (2026 기준) | 전환 이점 |
|---|---|---|
| **UI 프레임워크** (`Qt6Widgets`, `QMainWindow`, `QSS`) | `eframe 0.31` + `egui 0.31` + `egui_tiles` + `egui_extras` + 커스텀 `KaTheme` 렌더러 | `strata-arch3d`와 동일한 검증된 즉시모드 GPU UI, QSS 파싱 오버헤드 및 위젯 포인터 댕글링 크래시 완전 소멸 |
| **2D 지도 캔버스** (`QgsMapCanvas`, `QgsVectorLayer`, `QgsRubberBand`) | `wgpu 24.0` + `lyon` (벡터 테셀레이션) + `cosmic-text` (한글 라벨 충돌 회피/배치) + `rstar` (R*-Tree 공간 인덱스) | 타일 로딩 중 `isDrawing()` 충돌(`0xc0000005`) 원천 제거, 수십만 정점 CAD 스냅 지연 0ms |
| **3D 입체지형** (외부 `strata-3d.exe` + 로컬 파이프) | 내장 `strata-terrain3d` 크레이트 (`wgpu` WGSL 셰이더: R32F 쿼드트리 LOD, Nishita Sky, GTAO, AgX) | 별도 프로세스 실행·파이프 대기 없이 탭 전환 즉시 3D 지형·시굴 트렌치 단면 확인 |
| **좌표계 변환** (`QgsCoordinateTransform`, C PROJ) | `proj4rs` (순수 Rust PROJ) + 한국 측지계 파라미터 내장 (`EPSG:5187`, `5186`, `5179`, `5174`, `3857`, `4326`) | 외부 `proj.db` 누락 오류 제거, 스레드 안전 병렬 좌표 변환 |
| **GeoPackage 입출력** (GDAL/OGR GPKG Provider) | `rusqlite` (`bundled` SQLite 3 + R*Tree) + `wkb` / `geozero` + 세대 교체 저장기 | 레이어 삭제 시 윈도우 파일 잠금(`MoveFileEx` 오류) 완전 해결, 멀티폴리곤 자동 분할(`PolygonPieces`) |
| **SHP / DXF / GeoTIFF** (GDAL/OGR 드라이버) | `shapefile` + `dbase` + `encoding_rs`(CP949) / `dxf` / `tiff` + `async-tiff`(COG Range 읽기) | 제출용 `EPSG:5179` SHP, 수치지형도 DXF, Copernicus GLO-30 COG 무설치 직접 처리 |
| **공간 연산·버퍼·TIN** (GEOS, QGIS Geometry) | `geo 0.29+` (`i_overlay` 불리언·`LineJoin::Round` 버퍼) + `spade` (제약 델로네 삼각망 TIN) + `ndarray` + `rayon` | 대각선 모서리 500.0m/1000.0m 정밀 일치, 폴리곤 자르기·합치기·등고선 추출 고속화 |
| **네트워크·타일·API** (`QNetworkAccessManager`) | `reqwest` (`rustls-tls`, HTTP/2, 쿠키 스토어) + `tokio` + `moka` (비동기 타일 캐시) + `scraper` | VWorld(Referer 자동 주입, 약관 준수 메모리 전용 위성 캐시), KIGAM WFS, 국토지반정보 시추공 비동기 고속 수집 |
| **사이트 로그인 웹뷰** (`QWebEngineView` — 문화재 인트라넷, 국토지리정보원) | `wry` (Windows Edge WebView2 경량 바인딩) + 기존 검증된 프레임 탐색 JS (`HeritageIntranetFlow`, `ngii-topographic.js`) | 200MB짜리 `Qt6WebEngineCore.dll` 제거(윈도우 내장 WebView2 활용), RSA `goLogin()` 및 프레임 내부 폼 자동화 100% 호환 |
| **암호화·설정 저장소** (`QSettings`, `CryptProtectData`) | `configparser` (Qt INI 호환 파서) + `winreg` + `windows-sys` (`CryptProtectData`/`CryptUnprotectData`) + `zeroize` | 기존 `.ini` 파일 및 레지스트리, DPAPI/포터블 비밀번호와 바이트 단위 100% 호환 + 메모리 자동 소거(`zeroize`) |
| **도면 조판·PDF·인쇄·엑셀** (`QgsLayout`, `Qt6Pdf`, `SimpleXlsx`) | `krilla` / `pdf-writer` (벡터 PDF) + `resvg`/`tiny-skia` (래스터화·분할 인쇄 `TilePrint`) + `rust_xlsxwriter` | `QgsLayout` 크래시 제거, A3/A4 칼선 분할 인쇄 및 `조사도면.pdf`·주변유적 `.xlsx` 고속 생성 |

> **범위 제외 명시**: 한글(`.hwpx`) 보고서 생성기, GIS 실측 원장(`GisFactLedger`), Antigravity SDK MCP 직결(`strata-report`)은 사용자 지시에 따라 전환 범위에서 제외한다.

---

## 3. Cargo 워크스페이스 12개 크레이트 모듈 구조

루트 워크스페이스(`Cargo.toml`) 아래에 역할별로 명확히 분리된 **12개 크레이트**를 둔다. 각 소스 파일은 유지보수성을 위해 **300줄 이하** 원칙을 지킨다.

```text
strata-rs/ (또는 저장소 루트 Cargo.toml)
├── Cargo.toml
├── data/                         # 기존 data/ (schemas, rules, theme, providers, fonts) 그대로 승계
└── crates/
    ├── strata-types/             # 1. 공통 도메인 타입, 7대 레이어 스키마(v3), CRS 코드, 에러(StrataError)
    ├── strata-secrets/           # 2. INI/레지스트리 설정, DPAPI/포터블 암호화, VWorld·인트라넷·NGII 계정 호환
    ├── strata-storage/           # 3. GPKG(rusqlite+R*Tree) 읽기/쓰기, 세대 백업 저장, SHP/CSV/DXF/GeoTIFF 입출력
    ├── strata-geo/               # 4. proj4rs 좌표변환, geo 불리언/원형버퍼(500/1000m), rstar 스냅, TIN 등고선, 시굴격자(20/15/10m)
    ├── strata-net/               # 5. VWorld WMTS/WMS/지적WFS, KIGAM 지질/단층, 국토지반정보 시추공, 흙토람, wry 웹뷰(문화재청·NGII)
    ├── strata-paleo/             # 6. 반경 10km 4단계 고지형 엔진(IDW, EDT, 적분TPI, 다방향 음영, 7분류, 가상 지층 단면도)
    ├── strata-canvas2d/          # 7. wgpu 2D 지도 렌더러(타일 피라미드, 벡터/버텍스/스냅 마커, 한글 라벨, 지오레퍼런싱 정합 도구)
    ├── strata-terrain3d/         # 8. wgpu 3D 입체지형 엔진(R32F 쿼드트리 LOD, Nishita 하늘, GTAO, AgX, 3D 트렌치 단면)
    ├── strata-layout/            # 9. 도면 조판 스튜디오(용지·축척자·방위표·범례·유적번호 배지) + PDF/GeoTIFF/분할 인쇄(TilePrint)
    ├── strata-check-export/      # 10. ChecklistEngine(drawing_checklist.v1.json) + 제출 꾸러미(EPSG:5179 SHP·PDF·MANIFEST·README·XLSX)
    └── strata-app/               # 11. eframe/egui 데스크톱 본체(KaTheme/조용한 도판, 1열 리본, 5대 탭, 인스펙터, --smoke-quit)
```

---

## 4. 핵심 서브시스템 상세 설계

### 4.1 계정·API 키·보안 저장소 (`strata-secrets`) — 100% 하위 호환
- **VWorld / HistoryGIS 키 (`VworldSettings` 호환)**:
  - 탐색 경로 순서:
    1. 포터블 번들일 때: `<portable_config>/ka-hgis-vworld.ini` → `<portable_config>/secrets.ini` → 환경변수 `VWORLD_API_KEY`
    2. 일반 설치본일 때: `%LOCALAPPDATA%\ka-hgis\ka-hgis-vworld.ini` (`VWorld/ApiKey`, `vworld/apiKey`, `HistoryGis/ApiKey`) → Windows 레지스트리 `HKCU\Software\ka-hgis\ka-hgis` → 환경변수 `VWORLD_API_KEY` / `HISTORY_GIS_API_KEY` → 저장소 `config/secrets.ini`
- **국가유산청 인트라넷 & 국토지리정보원 계정 (`KaSecretStore` 호환)**:
  - 파일 경로: `%LOCALAPPDATA%\ka-hgis\heritage-account.ini` (`heritage/username`, `heritage/password_dpapi`, `heritage/password_portable`), `%LOCALAPPDATA%\ka-hgis\ngii-account.ini` (`ngii/username`, `ngii/password_dpapi`, `ngii/password_portable`) 및 번들 폴백 `config/heritage-local.ini`, `config/ngii-local.ini`.
  - **DPAPI 복호화/암호화**: Windows `CryptUnprotectData` / `CryptProtectData` 호출 시 기존 C++과 동일한 엔트로피 바이트열(`b"ka-hgis-account-v1"`, 18바이트)과 `CRYPTPROTECT_UI_FORBIDDEN` 플래그를 사용하여 기존에 저장된 암호를 그대로 복호화한다.
  - **포터블 암복호화**: `b"ka-hgis-account-v1"` 순환 XOR 후 Base64 인코딩/디코딩.
  - **로그 마스킹 (`SecretMask`)**: 모든 로그·오류 문자열에서 API 키(`key=...`), 계정 ID, 비밀번호를 정규식으로 즉시 마스킹한다.

### 4.2 공간 데이터베이스 및 조사 세션 (`strata-storage`)
- **7대 도메인 레이어 (`ka_hgis_layers.yaml` v3)**:
  - `survey_area`(Polygon), `feature_poly`(Polygon), `feature_line`(LineString), `section_line`(LineString), `control_points`(Point), `artifact_point`(Point), `trial_trench`(Polygon).
  - 빈 레이어 자동 추가 금지: `ensure_domain_layer(layer_key)`를 통해서만 생성하며, 레이어 식별은 한국어 표시명이 아닌 `ka_hgis/layer_key` 메타데이터로만 수행한다.
- **안전한 저장 및 파일 잠금 원천 방지 (`SurveyStorage`)**:
  - 조사 파일(`.gpkg`)을 열 때 읽기/편집 버퍼를 메모리(`EditBuffer` — 미저장 피처는 음수 ID 부여, 레이어별 독립 Undo/Redo 스택 보유)로 분리하여, SQLite 파일 핸들을 상시 점유하지 않는다.
  - `Ctrl+S` 저장 시: 임시 세대 사본(`.gpkg.gen-tmp`) 생성 → 스키마 v3 마이그레이션(`migrate_generation_copy`) → 단일 `POLYGON` 테이블의 멀티폴리곤 조각 자동 분리(`split_polygon_pieces`) → 무결성 검증(`PRAGMA integrity_check` 및 피처 수 대조) → 원자적 파일 교체(`std::fs::rename` / `ReplaceFileW`).
  - **원본 보호 불변 규칙**: 분석·3D·평가·다른 이름 저장 시 사용자의 원본 `.gpkg`는 읽기 전용으로만 열며, 해시(`SurveyFileFingerprint` SHA-256)가 변하지 않음을 보장한다.

### 4.3 2D 지도 캔버스 및 정밀 CAD 편집기 (`strata-canvas2d` & `strata-geo`)
- **렌더링 파이프라인 (`wgpu`)**:
  - 배경 타일 레이어(VWorld 위성 WMTS, 기본도, 대동여지도, 1919 조선지형도, OSM)는 백그라운드 `tokio` 워커가 비동기로 받아 GPU 텍스처 아틀라스에 올리고, 준비된 타일만 즉시 그리므로 `isDrawing()` 경합 크래시가 구조적으로 불가능하다.
  - 지적 WMS(`crs=EPSG:3857` + `KEY`, `DOMAIN` 미전송, 후보 `4326→3857→900913`)는 만료 키의 `HTTP 200 + XML serviceException` 응답을 자동 감지하여 빈 타일로 조용히 처리한다.
- **스냅 및 도형 편집 (`rstar` + `geo`)**:
  - `rstar::RTree`에 편집 가능/참조 벡터(VWorld 지적 그림 WMS 제외)의 정점·선분을 인덱싱하여 마우스 이동 시 0.1ms 이내에 정점·선분·교차점 자석(스냅) 마커를 재사용 렌더링한다(`QGraphicsScene` BspTree 크래시 완전 제거).
  - 폴리곤 나누기·합치기·구멍 뚫기(`PolygonErase`), 시굴격자(`20m`·`15m`·`10m` 단일 선택 길이 표준 트렌치 10% 자동 배치 및 L자/회전/끌기 편집)를 순수 Rust `geo` 알고리즘으로 수행한다.

### 4.4 고지형 분석 4단계 엔진 및 DEM 합성 (`strata-paleo`)
- **국토지리정보원 수치지형도 등고선 기반 정밀 DEM 자동 합성**:
  - 다중 도엽 DXF/SHP/NGI에서 등고선(`1000`~`1019`)과 표고점을 추출하고, 파일 잠금 없이 메모리에서 병합한 뒤 `spade` 제약 델로네 삼각망(TIN) 보간으로 정밀 DEM 격자를 합성한다.
- **반경 10km 4단계 고지형 파이프라인 (`HgisPaleoEngine` Rust 이식)**:
  1. **시추주상도 층서 파싱 (`parse_single_borehole`)**: 매립층(`fill_thickness_m`), 충적층(`alluvial_thickness_m`), 풍화대/기반암 출현 심도(`weathered_top_depth_m`), USCS 토질 분류, 유기물 포함 여부 추출 및 150m 격자 공간 균형 샘플링(`select_spatially_balanced_candidates`).
  2. **과거 지형면(`z_paleo`) 복원 및 하천 상대고도(`rem`)**: `rayon` 병렬 IDW 보간 + Felzenszwalb-Huttenlocher 1D/2D 선형 시간 유클리드 거리변환(`compute_river_distance_edt`).
  3. **적분영상(Integral Image) 기반 다중 스케일 TPI 및 다방향 음영(Imhof 색채 + 경사도)**: $O(1)$ 윈도우 평균 고도 계산(`compute_tpi_integral_image`) 및 Horn 3×3 법선 기반 4방향 가중 음영기복도.
  4. **7대 고지형 자동 분류(`PaleoClass`) 및 2D 가상 지층 단면도(`CrossSectionProfile`)**: 자연제방/미고지, 하안단구/홍적대지, 구릉 완사면, 구하도/배후습지, 현하도/범람원, 매립/교란지, 산지 급경사지 판별 및 조사구역 중심 단면 프로파일 생성.

### 4.5 6대 탭 UI 및 도면·제출·보고서 (`strata-app`, `strata-layout`, `strata-check-export`, `strata-report`)
- **화면 구성 (`strata-app`)**:
  - 상단 1열 리본(`KaBeginnerRibbon`) + 우측 `KaAppBar`(주소·지번 검색 및 시도 빠른 이동) + 조건부 2열 세부도구(`subToolbar`).
  - 좌측 패널: 4구역 레이어 트리(`조사 데이터` / `지적도` / `주변·참조` / `배경 지도`, 불투명도 슬라이더, `Delete` 키 삭제) + 파일함.
  - 중앙 6대 탭:
    1. `홈`: 새 조사 / 열기 / 최근 조사 목록 / 작업 순서 안내 / 연결 상태 배지.
    2. `지도`: 2D GPU 지도 캔버스 + 상단 그리기 안내 띠(`KaDrawGuideBand`).
    3. `고지형분석`: 좌측 레이어 트리 + 중앙 상단 고지형 컬러맵·시추공 마커 뷰 + 중앙 하단 2D 가상 지층 단면도 + 우측 반경 10km 원클릭 분석·7대 범례·판독 요약·「도면에 반영」.
    4. `도면`: A3/A4 용지 조판(제목·축척자·축척 글·좌표계·방위표 하단 띠 고정, 용지 밖 배치 허용 범례, 유적 번호 원형 배지), PDF/GeoTIFF 저장, 프린터 용지/방향 사전 검증 및 분할 인쇄(`TilePrint`).
    5. `입체지형`: 내장 `strata-terrain3d` 뷰포트 + 시굴 트렌치 3D 그리기 + 하단 거리-표고 단면도 + 500m/1000m 고고학 입지 평가 요약 + 입체 도면(PNG/PDF) 출력.

---

## 5. 단계별 전환 로드맵 (8단계 마일스톤)

기존 C++ 코드를 참고하되 C++/QGIS 시절의 임시 우회 코드(해킹성 타이머, 러버밴드 우회, 더미 레이어 등)는 모두 걷어내고 순수 Rust 정석 아키텍처로 구현한다.

- **Phase 1**: 최신 Git(`kyi/main`) 동기화 및 Cargo 12-크레이트 워크스페이스·공통 타입(`strata-types`)·보안 설정(`strata-secrets`) 구축
- **Phase 2**: 순수 Rust 공간 엔진(`strata-geo`: `proj4rs` 좌표변환, `geo` 원형 버퍼·폴리곤 편집, `rstar` 스냅, `spade` TIN 등고선, 시굴격자) 및 GPKG/SHP 스토리지(`strata-storage`) 구축
- **Phase 3**: 비동기 외부 자료 수집·로그인 엔진(`strata-net`: VWorld WMTS/WMS/지적, KIGAM, 국토지반정보 시추공, 흙토람, `wry` 기반 문화재 인트라넷·국토지리정보원 자동화) 구축
- **Phase 4**: 반경 10km 4단계 고지형 분석·수치지형도 정밀 DEM 합성 엔진(`strata-paleo`) 구축
- **Phase 5**: `wgpu` 기반 2D 지도 캔버스(`strata-canvas2d`) 및 3D 입체지형 엔진(`strata-terrain3d`) 통합 구현
- **Phase 6**: 도면 조판·분할 인쇄(`strata-layout`) 및 체크리스트 검수·제출 꾸러미·엑셀 내보내기(`strata-check-export`) 구현
- **Phase 7**: 한글(HWPX) 보고서·GIS 실측 원장·시굴 전 평가·MCP 브리지(`strata-report`) 구현
- **Phase 8**: `eframe`/`egui` 데스크톱 본체(`strata-app`: `KaTheme`·조용한 도판, 1열 리본, 6대 탭, 인스펙터, `--smoke-quit` 및 바탕화면 바로가기 자동 연결) 완성 및 전수 검증
