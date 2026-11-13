# 고고학 전용 HGIS (ka-hgis, 앱 이름 Strata) — Pure Rust Edition

100% 순수 Rust(`eframe`/`egui` + `rusqlite` + `geo` + `proj4rs` + `reqwest`) 독립 실행형 필드고고학 HGIS 데스크톱 애플리케이션.
기존 C++20/Qt6/QGIS 원본(`95d15a9`)을 파일·위젯 단위로 1:1 대조 전환하여 OSGeo4W·Qt6 외부 런타임 의존성 없이 단일 네이티브 실행 파일(`ka-hgis.exe`)로 구동됩니다.

- **작업 좌표계:** `EPSG:5186`(중부원점) / `EPSG:5187`(동부원점, 기본)
- **제출 좌표계:** `EPSG:5179`(국가유산청 제출 SHP 전용)
- **저장소:** https://github.com/kesekikwon-netizen/hgis-rust · 브랜치 `main`

## Cargo 워크스페이스 구조 (11 Crates)

| 크레이트 | 역할 | 대응 C++ 모듈 (`95d15a9`) |
|---|---|---|
| `strata-types` | 7대 도메인 레이어 스키마(v3), 좌표계(`EPSG:5186/5187/5179`), 조용한 도판(`KaThemePaper`) 토큰 | `LayerSchema`, `CrsPolicy`, `KaThemePaper` |
| `strata-secrets` | DPAPI / 포터블 암호화(`ka-hgis-account-v1`), VWorld 키 및 3대 외부 사이트 계정 관리 | `KaSecretStore`, `KaAccountStore`, `KaVworldConfig` |
| `strata-storage` | OGC GeoPackage(`.gpkg`) SQLite/WKB 코덱, 다중 폴리곤 피스 분해, 세대 저장·재열기, DXF/CSV 임포트 | `ProjectFile`, `KaWorkspaceSaver` |
| `strata-geo` | 36분할 500m/1000m 원형 버퍼, 20m 표준 시굴 트렌치 격자, 폴리곤 분할, Helmert 정합 | `KaEasyDrawEngine`, `KaTrenchGridDialog`, `KaGeorefDialog` |
| `strata-net` | VWorld WMTS/WMS(지적도 `DOMAIN=` 제외, HTTP 200 XML 오류 감지), 지오인포 시추공, 인트라넷/NGII 자동화 | `VworldProvider`, `KaHeritageWebDialog`, `KaNgiiLoginDialog` |
| `strata-paleo` | 반경 10km 4단계 고지형 분류(7대 미지형) 및 2D 지층 단면도(`A-A'`) RGBA 렌더러 | `KaPaleoEngine`, `KaPaleoCrossSectionWidget` |
| `strata-canvas2d` | 2D 지도 캔버스 투영, 마우스 CAD 드로잉·스냅·측거, 유산 번호표 배지 | `KaCanvasTheme`, `KaDrawingTools`, `KaMeasureTool` |
| `strata-terrain3d` | AgX 톤매핑 + Nishita 대기 산란 기반 DEM 3D 입체지형 렌더러 | `KaTerrain3dView`, `KaTerrain3dStudio` |
| `strata-layout` | A4/A3 종/횡 도면 스튜디오 조판(`ka_map`, 방위표, 축척자, 범례, 표제란), 벡터 PDF 및 GeoTIFF(`Tag 34735`) | `KaDrawingStudio`, `LayoutEngine`, `KaTitleBlock` |
| `strata-check-export` | 12항목 제출 전 자동 검수 및 국가유산청 제출 꾸러미(`EPSG:5179` SHP + `조사도면.pdf` + `MANIFEST.sha256` + `README.txt`) | `ChecklistEngine`, `ExportPackage` |
| `strata-app` (`ka-hgis`) | 1:1 C++ 대조 데스크톱 UI (`KaAppBar`, 17개 광역 지역선택 팝업, 7그룹 초보자 리본, 보조 툴바, 5대 탭, 하단 상태바) | `MainWindow`, `KaStartPage`, `KaBeginnerRibbon`, `KaInspectorPanel` |

## 빌드 · 테스트 · 검증 하네스 실행

```powershell
# 전체 단위 테스트 (32 tests)
cargo test --workspace

# 클리피 정적 분석 (경고 0건 강제)
cargo clippy --workspace --all-targets -- -D warnings

# V1 ~ V14 전수 스모크·통합 검증 게이트 실행
cargo run -p strata-app --bin ka-hgis -- --verify-all

# 데스크톱 GUI 실행
cargo run -p strata-app --bin ka-hgis
```

### 제출 SHP 엔진(GDAL) — `strata-engine --features gdal`

앱은 아직 이 기능을 켜지 않는다(지시 R2-3 에서 GDAL DLL 과 함께 켠다). 엔진만 빌드·시험할 때:

```bash
# 빌드: GDAL_HOME = OSGeo4W 의 apps/gdal-dev (lib/gdal_i.lib, include/). GDAL_VERSION 은 gdal-sys 의
# 미리 만든 3.12 바인딩을 고른다(bindgen 은 3.14 헤더에서 gdal 0.19 를 깨뜨린다).
export GDAL_HOME='<OSGeo4W>\apps\gdal-dev' GDAL_VERSION=3.12.0
# 실행: 시험 프로세스의 PATH 에 <OSGeo4W>/apps/gdal-dev/bin 과 <OSGeo4W>/bin (GDAL DLL)
cargo clippy -p strata-engine --features gdal --all-targets -- -D warnings
cargo test -p strata-engine --features gdal
```

PROJ 자료(`proj.db`)는 GDAL 이 쓰는 PROJ 가 제 옆 `share/proj` 에서 찾는다. 앱은 환경 변수를 바꾸지 않고 `strata_engine::set_gdal_proj_search_paths` 로 폴더를 넘긴다. 제출 SHP 의 `.prj` 글은 `proj.db` 판을 따른다: C++ 기준 꾸러미(`crates/strata-engine/tests/fixtures/cpp_submit_demo/`)는 PROJ 9.8.1 의 `proj.db` 로 만들었다. CI 는 GISInternals GDAL 3.12.1 과 OSGeo4W `proj-runtime-data-9.8.1-1` 을 sha256 으로 고정해 받는다(`.github/workflows/ci.yml`).

## C++ (`95d15a9`) ↔ Rust 1:1 대조 계획서
- [`docs/superpowers/plans/2026-11-07-cpp-to-rust-1to1-대조-전환-계획.md`](docs/superpowers/plans/2026-11-07-cpp-to-rust-1to1-대조-전환-계획.md)
- [`docs/superpowers/specs/2026-11-07-strata-pure-rust-conversion-design.md`](docs/superpowers/specs/2026-11-07-strata-pure-rust-conversion-design.md)
