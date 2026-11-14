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

`.cargo/config.toml` 이 `STRATA_OFFLINE=1` 을 걸어 두므로 `cargo test`·`cargo run` 과 `--snap`·`--verify-all`·`--smoke-quit`·`--smoke` 실행은 외부 사이트에 요청을 보내지 않는다(`target/release/ka-hgis.exe` 를 직접 켤 때만 온라인).
화면 캡처 한 번만 실제 타일이 필요할 때는 `--snap <폴더> --snap-online` 을 함께 주며, 다른 실행이나 `STRATA_OFFLINE=1` 아래에서는 열리지 않는다.

### 제출 SHP 엔진(GDAL) — `strata-engine --features gdal`

앱(`strata-app`)은 아직 이 기능을 켜지 않는다(지시 R2-3c 에서 켠다). `gdal-src` `0.3.0+3.12.1`(`gdal-sys` `bundled`)이 `ESRI Shapefile` 과 `GPKG`/`SQLite` 드라이버만 넣어 GDAL 3.12.1 을 소스에서 정적으로 빌드하므로 OSGeo4W 나 외부 GDAL DLL, `GDAL_HOME`·`GDAL_VERSION` 환경 변수가 필요 없다. 빌드에는 `cmake` 와 `sqlite3` 명령줄 도구만 `PATH` 에 있으면 된다(PROJ 소스 빌드가 `proj.db` 를 만들 때 쓴다):

```powershell
cargo clippy -p strata-engine --features gdal --all-targets -- -D warnings
cargo test -p strata-engine --features gdal
cargo clippy -p strata-check-export --features gdal --all-targets -- -D warnings
cargo test -p strata-check-export --features gdal
```

제출 SHP 의 `.prj` 글은 `proj.db` 판을 따른다. C++ 기준 꾸러미(`crates/strata-engine/tests/fixtures/cpp_submit_demo/` 등)가 만들어진 PROJ 9.8.1 의 `proj.db` 가 `crates/strata-engine/data/proj-9.8.1/proj.db` 로 프로그램 안에 들어 있으며, 첫 사용 때 폴더에 풀고 환경 변수를 바꾸지 않은 채 GDAL `PROJ_DATA` 설정(`strata_engine::install_submit_proj_db` / `set_gdal_proj_search_paths`)으로 넘긴다. CI 도 `sqlite3` 명령줄 도구만 받아 같은 정적 빌드로 검증한다(`.github/workflows/ci.yml`).

## C++ (`95d15a9`) ↔ Rust 1:1 대조 계획서
- [`docs/superpowers/plans/2026-11-07-cpp-to-rust-1to1-대조-전환-계획.md`](docs/superpowers/plans/2026-11-07-cpp-to-rust-1to1-대조-전환-계획.md)
- [`docs/superpowers/specs/2026-11-07-strata-pure-rust-conversion-design.md`](docs/superpowers/specs/2026-11-07-strata-pure-rust-conversion-design.md)
