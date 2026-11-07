# Strata (ka-hgis) 100% 순수 Rust 전환 구현 계획 (Master Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 기존 C++20/Qt6/QGIS(OSGeo4W) 기반 Strata(`ka-hgis`) 약 10.6만 줄을 외부 OSGeo4W 의존성이 전혀 없는 100% 순수 Rust 단일 데스크톱 애플리케이션(12개 크레이트 워크스페이스)으로 전면 전환하며, 기존 화면 디자인(`KaTheme`·조용한 도판·1열 리본·6대 탭), 7대 도메인 레이어 스키마, 사이트 로그인 및 API 키(`VWorld`, `국가유산청 인트라넷`, `국토지리정보원`, `KIGAM`, `국토지반정보`, `DBpia/RAG`)를 100% 그대로 계승한다.

**Architecture:** `eframe 0.31` / `egui 0.31` / `wgpu 24.0.5` 기반 네이티브 데스크톱 셸 위에 순수 Rust 공간 엔진(`proj4rs`, `geo`, `rstar`, `spade`, `rusqlite` GPKG, `shapefile`, `dxf`, `tiff`), 비동기 네트워크·웹뷰 로그인 엔진(`reqwest`, `tokio`, `wry` WebView2), 병렬 고지형·3D 입체지형 엔진(`rayon`, WGSL 셰이더), 벡터 PDF/HWPX 조판 엔진(`krilla`/`pdf-writer`, `zip`+`quick-xml`)을 12개 모듈형 크레이트로 결합한다.

**Tech Stack:** Rust `1.91.1` (Edition 2021), `eframe 0.31`, `egui 0.31`, `wgpu 24.0.5`, `lyon 1.0`, `cosmic-text 0.12`, `proj4rs 0.1`, `geo 0.29`, `rstar 0.12`, `spade 2.12`, `rusqlite 0.32` (`bundled`), `shapefile 0.6`, `dxf 0.6`, `tiff 0.9`, `reqwest 0.12` (`rustls-tls`, `cookies`), `tokio 1.41`, `wry 0.47`, `rayon 1.10`, `ndarray 0.16`, `pdf-writer 0.12`, `rust_xlsxwriter 0.79`, `windows-sys 0.59` (DPAPI 전용).

**Spec:** `docs/superpowers/specs/2026-11-07-strata-rust-전환-설계.md`

## Global Constraints

- **언어 및 린트 규칙**: 워크스페이스 전체 `edition = "2021"`, `clippy::all = "warn"`, `clippy::unwrap_used = "warn"`, 프로덕션 코드 내 `unwrap()`/`expect()` 금지 (`Result<T, StrataError>` 및 `?` 사용). Windows DPAPI FFI를 격리한 `strata-secrets::dpapi` 모듈 외 모든 11개 크레이트에 `#![forbid(unsafe_code)]` 적용.
- **파일 크기 제한**: 모든 신규 `.rs` 소스 파일은 **300줄 이하**로 모듈을 분할한다.
- **도메인 레이어 불변 규칙**: 7대 도메인 레이어(`survey_area`, `feature_poly`, `feature_line`, `section_line`, `control_points`, `artifact_point`, `trial_trench`, 스키마 v3)는 오직 `ensure_domain_layer`로만 추가하며, 빈 도메인 레이어를 자동 추가하지 않는다. 정체성은 `ka_hgis/layer_key`로만 식별한다.
- **레이어 구역 및 좌표계 규칙**: 레이어 트리는 `조사 데이터` / `지적도` / `주변·참조` / `배경 지도` 4구역을 유지한다. 작업 좌표계는 `EPSG:5187`(기본) 또는 `EPSG:5186`이며, 제출 꾸러미(`ExportService`)만 `EPSG:5179` SHP로 변환해 내보낸다.
- **버퍼 및 폴리곤 무결성**: 주변 유적 반경(`500m`/`1000m`) 버퍼는 반드시 `LineJoin::Round`(36분할)와 유클리드 최단거리(`distance <= radius`)를 사용한다(`Miter` 금지). 단일 `POLYGON` GPKG 테이블 저장 시 멀티폴리곤은 조각별 개별 피처로 분리(`split_polygon_pieces`)한다.
- **기존 계정·API 키 100% 호환**: `%LOCALAPPDATA%\ka-hgis\` 아래 `ka-hgis-vworld.ini`, `heritage-account.ini`, `ngii-account.ini`, `secrets.ini`, 레지스트리 `HKCU\Software\ka-hgis`, DPAPI 엔트로피 `"ka-hgis-account-v1"`, 포터블 XOR-Base64(`password_portable`)를 수정 없이 그대로 읽고 쓴다.
- **원본 보호**: 사용자의 조사 원본(`.gpkg`)은 분석·3D·평가 시 읽기 전용으로만 열며 SHA-256 지문(`SurveyFileFingerprint`)이 절대 변하지 않아야 한다.

## Review Focus

1. **기존 DPAPI/포터블 암호 복호화 호환 실패**: C++ `KaSecretStore`가 저장한 `heritage/password_dpapi`, `ngii/password_dpapi`, `password_portable` 값을 Rust `strata-secrets`가 읽을 때 엔트로피(`"ka-hgis-account-v1"`, NULL 종료 제외한 18바이트)가 1바이트라도 다르면 복호화가 실패한다 → **Task 2**에서 바이트 단위 교차 검증 테스트로 고정.
2. **단일 POLYGON 테이블에 멀티폴리곤 저장 시 0면적 링 손상**: 고지형 분석 결과나 잘라낸 폴리곤이 `MultiPolygon`일 때 단일 `POLYGON` 테이블에 그대로 넣으면 깨진다 → **Task 3**에서 `split_polygon_pieces` 자동 분할 테스트로 고정.
3. **500m/1000m 버퍼 대각선 모서리 돌출(`Miter` 오차)**: 사각형 조사구역 버퍼가 대각선에서 $\sqrt{2}$배(707m)로 튀어나가면 반경 밖 유적이 잘못 포함된다 → **Task 4**에서 36분할 `Round` 버퍼의 모든 방향 거리 오차 `< 0.5%` 테스트로 고정.
4. **만료된 VWorld API 키가 HTTP 200 + XML 오류를 반환할 때 크래시 또는 깨진 이미지 표시**: VWorld WMS/WMTS는 키가 만료되어도 HTTP 200에 XML `ServiceExceptionReport`를 내려보낸다 → **Task 5**에서 XML 본문 감지 시 빈 투명 타일 반환 및 로그 비노출(`SecretMask`) 테스트로 고정.
5. **시굴격자 트렌치 길이 평균 축소 버그**: `20m`·`15m`·`10m` 체크란에서 고른 단일 표준 길이 대신 잔여 면적에 맞춰 임의로 짧은 트렌치를 생성하면 현장 규격에 어긋난다 → **Task 4**에서 선택된 표준 길이(`20.0m`, `15.0m`, `10.0m`)만 정확히 배치하는 테스트로 고정.

---

### Task 1: 최신 원격 브랜치(`kyi/main`) 동기화 및 Cargo 12-크레이트 워크스페이스·`strata-types` 구축

**Files:**
- Create: `Cargo.toml`
- Create: `crates/strata-types/Cargo.toml`
- Create: `crates/strata-types/src/lib.rs`
- Create: `crates/strata-types/src/schema.rs`
- Create: `crates/strata-types/src/crs.rs`
- Create: `crates/strata-types/src/theme_tokens.rs`
- Create: `crates/strata-types/src/error.rs`

**Interfaces:**
- Consumes: `data/schemas/ka_hgis_layers.yaml`, `src/app/KaTheme.h`, `src/app/KaThemePaper.cpp`
- Produces:
  - `strata_types::schema::{DomainLayerKey, LayerSchema, FieldSpec, FieldType, SurveySchemaV3}`
  - `strata_types::crs::{WorkCrs, SubmitCrs, CrsCode}` (`Epsg5187`, `Epsg5186`, `Epsg5179`, `Epsg3857`, `Epsg4326`)
  - `strata_types::theme_tokens::{ColorTheme, DisplayOptions, ThemeTokens, IconPalette, ButtonMetrics}`
  - `strata_types::error::{StrataError, StrataResult<T>}`

- [ ] **Step 1: Write the failing test in `crates/strata-types/src/schema.rs` and `crates/strata-types/src/theme_tokens.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_v3_matches_seven_domain_layers_and_yaml() {
        let schema = SurveySchemaV3::default();
        assert_eq!(schema.version, 3);
        assert_eq!(schema.default_work_crs, WorkCrs::Epsg5187);
        assert_eq!(schema.submit_crs, SubmitCrs::Epsg5179);
        assert_eq!(schema.layers.len(), 7);
        assert_eq!(schema.layer(DomainLayerKey::SurveyArea).name_ko, "조사구역");
        assert_eq!(schema.layer(DomainLayerKey::TrialTrench).name_ko, "시굴격자");
    }

    #[test]
    fn paper_look_tokens_satisfy_wcag_contrast_and_clay_accent() {
        let opts = DisplayOptions { paper_look: true, ..Default::default() };
        let t = ThemeTokens::for_options(&opts);
        assert_eq!(t.accent, [0xB5, 0x57, 0x3A, 0xFF]);
        assert_eq!(t.dialog_ground, [0xFA, 0xF9, 0xF5, 0xFF]);
        assert!(t.contrast_ratio(t.ink_muted, t.surface) >= 4.5);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-types`
Expected: FAIL (crate and types not yet implemented)

- [ ] **Step 3: Implement `Cargo.toml` workspace and `strata-types` modules (`schema.rs`, `crs.rs`, `theme_tokens.rs`, `error.rs`)**

`data/schemas/ka_hgis_layers.yaml`의 7대 도메인 레이어 필드 정의와 `KaTheme.h`/`KaThemePaper.cpp`의 색상 토큰(`Default`, `SkySnow`, `BoltYellow`, `BlossomPastel`, `paper_look`, `high_contrast`, `large_text`) 및 WCAG 명도비 함수를 순수 Rust로 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-types && cargo clippy -p strata-types -- -D warnings`
Expected: PASS (0 warnings)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-types
git commit -m "feat(rust-core): Cargo 워크스페이스 및 strata-types 도메인 스키마·테마 토큰 크레이트를 구축한다"
```

---

### Task 2: 보안 설정·API 키·사이트 계정 호환 저장소 (`strata-secrets`) 구현

**Files:**
- Create: `crates/strata-secrets/Cargo.toml`
- Create: `crates/strata-secrets/src/lib.rs`
- Create: `crates/strata-secrets/src/dpapi.rs`
- Create: `crates/strata-secrets/src/secret_store.rs`
- Create: `crates/strata-secrets/src/vworld.rs`
- Create: `crates/strata-secrets/src/accounts.rs`
- Create: `crates/strata-secrets/src/mask.rs`

**Interfaces:**
- Consumes: `strata_types::error::StrataResult`
- Produces:
  - `strata_secrets::secret_store::SecretStore::{read_password, write_password, unwrap_portable, wrap_portable}` (entropy `b"ka-hgis-account-v1"`)
  - `strata_secrets::vworld::VworldSettings::{load_api_key, save_api_key, load_history_gis_api_key, save_history_gis_api_key}`
  - `strata_secrets::accounts::{AccountKind, Credentials, AccountSettings}` (`HeritageIntranet` -> `heritage-account.ini`, `NgiiTopographic` -> `ngii-account.ini`, `VworldCadastral`)
  - `strata_secrets::mask::SecretMask::redact(&str) -> String`

- [ ] **Step 1: Write the failing test in `crates/strata-secrets/src/secret_store.rs` and `crates/strata-secrets/src/vworld.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn portable_xor_entropy_matches_cpp_ka_secret_store_byte_for_byte() {
        // C++ xorEntropy("ka-hgis-account-v1") 호환 검증
        let wrapped = SecretStore::wrap_portable("MyFieldPass!2026");
        let recovered = SecretStore::unwrap_portable(&wrapped).unwrap();
        assert_eq!(recovered, "MyFieldPass!2026");
    }

    #[test]
    fn vworld_key_load_priority_and_secret_masking() {
        let tmp = TempDir::new().unwrap();
        let ini_path = tmp.path().join("ka-hgis-vworld.ini");
        std::fs::write(&ini_path, "[VWorld]\nApiKey=TEST-VWORLD-KEY-999\n").unwrap();
        let key = VworldSettings::load_from_ini(&ini_path).unwrap();
        assert_eq!(key, "TEST-VWORLD-KEY-999");
        let masked = SecretMask::redact("GET https://api.vworld.kr/req/wmts?key=TEST-VWORLD-KEY-999", &[&key]);
        assert!(!masked.contains("TEST-VWORLD-KEY-999"));
    }

    #[test]
    fn empty_personal_account_file_disables_default_login_without_falling_back_to_bundled() {
        let tmp = TempDir::new().unwrap();
        let personal = tmp.path().join("heritage-account.ini");
        let bundled = tmp.path().join("heritage-local.ini");
        AccountSettings::save_to_file(&personal, AccountKind::HeritageIntranet, &Credentials::empty(), true).unwrap();
        AccountSettings::save_to_file(&bundled, AccountKind::HeritageIntranet, &Credentials::new("bundled_id", "bundled_pw"), true).unwrap();
        let loaded = AccountSettings::read_from_files(&personal, &[bundled], AccountKind::HeritageIntranet).unwrap();
        assert!(loaded.username.is_empty());
        assert!(loaded.password.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-secrets`
Expected: FAIL

- [ ] **Step 3: Implement `strata-secrets` (`dpapi.rs`, `secret_store.rs`, `vworld.rs`, `accounts.rs`, `mask.rs`)**

`VworldSettings.cpp`, `KaSecretStore.cpp`, `HeritageIntranetSettings.cpp`, `TopographicSettings.cpp`의 파일 경로·우선순위·원자적 임시 파일 교체(`staging` → `rename`)·DPAPI(`CryptProtectData`/`CryptUnprotectData` with `b"ka-hgis-account-v1"`)·포터블 XOR을 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-secrets && cargo clippy -p strata-secrets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-secrets
git commit -m "feat(secrets): 기존 INI·레지스트리·DPAPI·포터블 계정과 100% 호환되는 strata-secrets를 구현한다"
```

---

### Task 3: GeoPackage(`.gpkg`)·SHP·DXF·CSV 스토리지 및 세대 교체 저장 엔진 (`strata-storage`) 구현

**Files:**
- Create: `crates/strata-storage/Cargo.toml`
- Create: `crates/strata-storage/src/lib.rs`
- Create: `crates/strata-storage/src/gpkg.rs`
- Create: `crates/strata-storage/src/polygon_pieces.rs`
- Create: `crates/strata-storage/src/edit_buffer.rs`
- Create: `crates/strata-storage/src/session_save.rs`
- Create: `crates/strata-storage/src/shp_io.rs`
- Create: `crates/strata-storage/src/cad_csv_io.rs`

**Interfaces:**
- Consumes: `strata_types::schema::*`, `strata_types::crs::*`
- Produces:
  - `strata_storage::gpkg::GpkgRepository::{create_new_survey, open_read_only, read_domain_layer, write_generation_copy}`
  - `strata_storage::polygon_pieces::split_polygon_pieces(geom: &geo::Geometry<f64>) -> Vec<geo::Polygon<f64>>`
  - `strata_storage::edit_buffer::{LayerEditBuffer, UndoStack, FeatureRecord}` (미저장 피처 음수 ID 할당, 레이어별 독립 Undo/Redo)
  - `strata_storage::session_save::{persist_workspace, save_as_copy, sha256_fingerprint}`
  - `strata_storage::shp_io::{read_shapefile_cp949_or_utf8, write_shapefile_utf8}`

- [ ] **Step 1: Write the failing test in `crates/strata-storage/src/session_save.rs` and `crates/strata-storage/src/polygon_pieces.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;
    use tempfile::TempDir;

    #[test]
    fn multipolygon_splits_into_non_zero_single_polygons_on_gpkg_write() {
        let mp = wkt!(MULTIPOLYGON(
            ((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)),
            ((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0))
        ));
        let pieces = split_polygon_pieces(&geo::Geometry::MultiPolygon(mp));
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn persist_workspace_after_layer_delete_never_locks_file_or_falls_back_to_dash_save() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("경주시_성동동.gpkg");
        let mut session = GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).unwrap();
        session.ensure_domain_layer(DomainLayerKey::SurveyArea);
        session.remove_domain_layer(DomainLayerKey::SurveyArea);
        persist_workspace(&mut session, &path).unwrap();
        assert!(path.exists());
        assert!(!tmp.path().join("경주시_성동동-저장.gpkg").exists());
    }

    #[test]
    fn save_as_copy_leaves_original_survey_fingerprint_untouched() {
        let tmp = TempDir::new().unwrap();
        let orig = tmp.path().join("orig.gpkg");
        let copy = tmp.path().join("copy.gpkg");
        let mut session = GpkgRepository::create_new_survey(&orig, WorkCrs::Epsg5187).unwrap();
        let hash_before = sha256_fingerprint(&orig).unwrap();
        session.ensure_domain_layer(DomainLayerKey::FeaturePoly);
        save_as_copy(&mut session, &orig, &copy).unwrap();
        let hash_after = sha256_fingerprint(&orig).unwrap();
        assert_eq!(hash_before, hash_after);
        assert!(copy.exists());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-storage`
Expected: FAIL

- [ ] **Step 3: Implement `strata-storage` (`gpkg.rs`, `polygon_pieces.rs`, `edit_buffer.rs`, `session_save.rs`, `shp_io.rs`, `cad_csv_io.rs`)**

`rusqlite` (`bundled`) 기반으로 GeoPackage 표준 메타데이터 테이블(`gpkg_spatial_ref_sys`, `gpkg_contents`, `gpkg_geometry_columns`, WKB 인코딩)과 내장 프로젝트 상태(`ka_hgis_project` 테이블)를 읽고 쓰며, 세대 교체 저장(`persist_workspace`) 및 CP949/UTF-8 자동 감지 Shapefile/CSV/DXF 리더를 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-storage && cargo clippy -p strata-storage -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-storage
git commit -m "feat(storage): 순수 Rust GPKG·SHP·DXF 입출력 및 무잠금 세대 교체 저장소를 구현한다"
```

---

### Task 4: 순수 Rust 공간 연산·좌표변환·36분할 원형 버퍼·시굴격자·TIN 등고선 (`strata-geo`) 구현

**Files:**
- Create: `crates/strata-geo/Cargo.toml`
- Create: `crates/strata-geo/src/lib.rs`
- Create: `crates/strata-geo/src/crs_transform.rs`
- Create: `crates/strata-geo/src/buffer_round.rs`
- Create: `crates/strata-geo/src/snap_index.rs`
- Create: `crates/strata-geo/src/polygon_ops.rs`
- Create: `crates/strata-geo/src/trench_grid.rs`
- Create: `crates/strata-geo/src/contour_tin.rs`

**Interfaces:**
- Consumes: `strata_types::crs::*`, `geo`, `proj4rs`, `rstar`, `spade`
- Produces:
  - `strata_geo::crs_transform::CrsTransformer::{new, transform_point, transform_geometry}` (`EPSG:5187` ↔ `5186` ↔ `5179` ↔ `3857` ↔ `4326`)
  - `strata_geo::buffer_round::{round_buffer_polygon, filter_within_euclidean_radius}` (36분할 `LineJoin::Round`, `500.0m` / `1000.0m`)
  - `strata_geo::snap_index::{SnapIndex, SnapHit, SnapMode}` (`rstar::RTree` 정점·선분·교차점 스냅, WMS 지적 그림 제외)
  - `strata_geo::trench_grid::{TrenchStandardLength, TrenchConfig, generate_standard_trenches}` (`20m`·`15m`·`10m` 단일 표준 길이 배치, 평균 길이 축소 금지)
  - `strata_geo::contour_tin::build_contours_from_points` (`spade` TIN + 등고선 추출)

- [ ] **Step 1: Write the failing test in `crates/strata-geo/src/buffer_round.rs` and `crates/strata-geo/src/trench_grid.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use geo::{wkt, Euclidean, Distance};

    #[test]
    fn round_buffer_500m_has_no_miter_corner_spikes() {
        let site = wkt!(POLYGON((0.0 0.0, 100.0 0.0, 100.0 100.0, 0.0 100.0, 0.0 0.0)));
        let ring = round_buffer_polygon(&site, 500.0, 36);
        for coord in ring.exterior().coords() {
            let pt = geo::Point::new(coord.x, coord.y);
            let dist = Euclidean.distance(&site, &pt);
            assert!((dist - 500.0).abs() < 2.5, "corner distance {} deviated from 500m", dist);
        }
    }

    #[test]
    fn trench_grid_uses_only_selected_standard_length_without_shrinking() {
        let area = wkt!(POLYGON((0.0 0.0, 100.0 0.0, 100.0 80.0, 0.0 80.0, 0.0 0.0)));
        let cfg = TrenchConfig {
            standard_length: TrenchStandardLength::M20,
            width_m: 2.0,
            coverage_ratio: 0.10,
            angle_deg: 0.0,
        };
        let trenches = generate_standard_trenches(&area, &cfg);
        assert!(!trenches.is_empty());
        for t in &trenches {
            assert!((t.length_m - 20.0).abs() < 1e-6);
            assert!((t.width_m - 2.0).abs() < 1e-6);
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-geo`
Expected: FAIL

- [ ] **Step 3: Implement `strata-geo` (`crs_transform.rs`, `buffer_round.rs`, `snap_index.rs`, `polygon_ops.rs`, `trench_grid.rs`, `contour_tin.rs`)**

`proj4rs` 한국 좌표계 정의, `geo::Buffer` (`LineJoin::Round`), `geo::BooleanOps` (`PolygonErase`), `rstar` R*-Tree 스냅 인덱스, 표준 길이(`20m`·`15m`·`10m`) 시굴격자 생성기(`8cefb5a`, `1d0cb41` 커밋 규칙 반영), `spade` 델로네 삼각망 등고선 추출기를 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-geo && cargo clippy -p strata-geo -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-geo
git commit -m "feat(geo): 한국 좌표변환·36분할 원형 버퍼·R*Tree 스냅·표준 시굴격자·TIN 등고선 엔진을 구현한다"
```

---

### Task 5: 외부 API·배경 타일·사이트 로그인 자동화 엔진 (`strata-net`) 구현

**Files:**
- Create: `crates/strata-net/Cargo.toml`
- Create: `crates/strata-net/src/lib.rs`
- Create: `crates/strata-net/src/vworld_client.rs`
- Create: `crates/strata-net/src/heritage_intranet.rs`
- Create: `crates/strata-net/src/ngii_topographic.rs`
- Create: `crates/strata-net/src/geoinfo_borehole.rs`
- Create: `crates/strata-net/src/kigam_soil_river.rs`

**Interfaces:**
- Consumes: `strata_secrets::*`, `strata_types::*`, `strata_geo::*`
- Produces:
  - `strata_net::vworld_client::VworldClient` (위성 WMTS 메모리 전용 타일 캐시 + `Referer: https://localhost`, 지적 WMS `EPSG:3857`+`KEY` 및 XML 200 오류 감지, 지적 5km WFS, 주소·지번 검색 `LocationSearch`)
  - `strata_net::heritage_intranet::HeritageIntranetFlow` (`https://intranet.gis-heritage.go.kr` 프레임 탐색 JS, `goLogin()` RSA 로그인, 튜토리얼 닫기, 서약서 동의 영수증 기록, `codedetaCd`/`codeCdSg` 지역 선택, 전체 다운로드)
  - `strata_net::ngii_topographic::NgiiTopographicClient` (`https://map.ngii.go.kr` 로그인, 1:5,000 수치지형도 및 DEM 자동 다운로드)
  - `strata_net::geoinfo_borehole::GeoInfoBoreholeService` (`searchHoleByExtent.do`, `getSichudanData.do` `jsonView` 파서, 150m 격자 균형 추출)
  - `strata_net::kigam_soil_river::{KigamPaleoService, SoilMapService, GeologyMapService, RiverMapService}`

- [ ] **Step 1: Write the failing test in `crates/strata-net/src/vworld_client.rs`, `crates/strata-net/src/heritage_intranet.rs`, and `crates/strata-net/src/geoinfo_borehole.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_cadastral_wms_url_never_sends_domain_and_detects_xml_error_on_http_200() {
        let url = VworldClient::cadastral_wms_url("TEST_KEY", [14135000.0, 4518000.0, 14136000.0, 4519000.0], "EPSG:3857");
        assert!(url.contains("crs=EPSG:3857"));
        assert!(url.contains("KEY=TEST_KEY"));
        assert!(!url.to_lowercase().contains("domain="));
        let xml_body = b"<?xml version=\"1.0\"?><ServiceExceptionReport><ServiceException code=\"INCORRECT_KEY\">Invalid Key</ServiceException></ServiceExceptionReport>";
        assert!(VworldClient::is_xml_error_tile(xml_body));
    }

    #[test]
    fn heritage_intranet_scripts_never_navigate_page_and_redact_login_urls() {
        let search_js = HeritageIntranetFlow::search_script();
        assert!(search_js.contains("searchForm"));
        assert!(!search_js.contains("location.href="));
        let redacted = HeritageIntranetFlow::redact_request_line("POST", "https://intranet.gis-heritage.go.kr/j_spring_security_check?pwd=secret");
        assert_eq!(redacted, "POST <로그인 요청 — 내용을 남기지 않음>");
    }

    #[test]
    fn geoinfo_sichudan_json_parser_extracts_fill_alluvial_and_weathered_depths() {
        let sample = br#"{"jsonView":"[{\"FROM_DEPTH\":0.0,\"TO_DEPTH\":1.5,\"LAYER_NAME\":\"매립층\",\"USCS\":\"SM\"},{\"FROM_DEPTH\":1.5,\"TO_DEPTH\":6.0,\"LAYER_NAME\":\"충적층\",\"USCS\":\"CL\"},{\"FROM_DEPTH\":6.0,\"TO_DEPTH\":10.0,\"LAYER_NAME\":\"풍화암\",\"USCS\":\"WR\"}]"}"#;
        let rec = GeoInfoBoreholeService::parse_sichudan_json(sample).unwrap();
        assert!((rec.fill_depth_m - 1.5).abs() < 1e-6);
        assert!((rec.alluvial_thickness_m - 4.5).abs() < 1e-6);
        assert!((rec.weathered_depth_m - 6.0).abs() < 1e-6);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-net`
Expected: FAIL

- [ ] **Step 3: Implement `strata-net` (`vworld_client.rs`, `heritage_intranet.rs`, `ngii_topographic.rs`, `geoinfo_borehole.rs`, `kigam_soil_river.rs`)**

`VworldClient`, `HeritageIntranetFlow.cpp`의 검증된 JS 스크립트 빌더 + `wry` WebView2 컨트롤러, `ngii-topographic.js` 연동, `GeoInfoBoreholeParser.cpp`의 시추주상도 파서 및 150m 공간 클러스터링(`selectSpatiallyBalancedCandidates`), `KigamPaleoService.cpp` WFS 클라이언트를 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-net && cargo clippy -p strata-net -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-net
git commit -m "feat(net): VWorld·국가유산청 인트라넷·국토지리정보원·국토지반정보·KIGAM 연동 엔진을 구현한다"
```

---

### Task 6: 반경 10km 4단계 고지형 분석 및 다중 도엽 정밀 DEM 합성 엔진 (`strata-paleo`) 구현

**Files:**
- Create: `crates/strata-paleo/Cargo.toml`
- Create: `crates/strata-paleo/src/lib.rs`
- Create: `crates/strata-paleo/src/borehole_strata.rs`
- Create: `crates/strata-paleo/src/terrain_edt_tpi.rs`
- Create: `crates/strata-paleo/src/pipeline.rs`
- Create: `crates/strata-paleo/src/dem_synthesizer.rs`

**Interfaces:**
- Consumes: `strata_net::geoinfo_borehole::*`, `strata_geo::contour_tin::*`, `rayon`, `ndarray`
- Produces:
  - `strata_paleo::pipeline::{PaleoClass, PaleoAnalysisInput, PaleoAnalysisResult, CrossSectionProfile, run_paleo_pipeline}`
  - `strata_paleo::dem_synthesizer::synthesize_precision_dem_from_topographic_sheets` (다중 도엽 등고선 병합 및 무잠금 정밀 DEM 합성)

- [ ] **Step 1: Write the failing test in `crates/strata-paleo/src/pipeline.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paleo_pipeline_classifies_natural_levee_and_generates_cross_section() {
        let tf = GridTransform { origin_x: 200_000.0, origin_y: 500_000.0, cell_size: 10.0, width: 32, height: 32 };
        let z_curr = vec![15.0; 32 * 32];
        let mut river_mask = vec![0u8; 32 * 32];
        river_mask[16 * 32 + 16] = 1;
        let aoi_mask = vec![1u8; 32 * 32];
        let bh = vec![BoreholeRawInput {
            hole_id: "BH-1".into(),
            x: 200_160.0,
            y: 499_840.0,
            ground_elev_m: 15.0,
            water_depth_m: 2.5,
            strata: vec![
                StratumLayer { depth_from_m: 0.0, depth_to_m: 1.0, layer_name: "매립토".into(), uscs_code: "SM".into(), description: "".into(), n_value: 5.0 },
                StratumLayer { depth_from_m: 1.0, depth_to_m: 4.0, layer_name: "충적사질토".into(), uscs_code: "SM".into(), description: "".into(), n_value: 12.0 },
                StratumLayer { depth_from_m: 4.0, depth_to_m: 8.0, layer_name: "풍화암".into(), uscs_code: "WR".into(), description: "".into(), n_value: 50.0 },
            ],
        }];
        let res = run_paleo_pipeline(&z_curr, &tf, &bh, &[], &[], &river_mask, &aoi_mask, (200_160.0, 499_840.0));
        assert_eq!(res.z_paleo.len(), 32 * 32);
        assert_eq!(res.paleo_rgba_buffer.len(), 32 * 32 * 4);
        assert!(!res.center_cross_section.dist_m.is_empty());
        assert!(res.aoi_summary_text.contains("고지형"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-paleo`
Expected: FAIL

- [ ] **Step 3: Implement `strata-paleo` (`borehole_strata.rs`, `terrain_edt_tpi.rs`, `pipeline.rs`, `dem_synthesizer.rs`)**

`kyi/main`의 `HgisPaleoBorehole.cpp`, `HgisPaleoTerrain.cpp`, `HgisPaleoPipeline.cpp`, `PaleoDemClassifier.cpp`를 `rayon` 병렬 루프와 순수 Rust 슬라이스 연산으로 이식하고, Imhof 다방향 음영기복도·경사도 컬러맵 및 2D 가상 지층 단면도 데이터를 생성한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-paleo && cargo clippy -p strata-paleo -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-paleo
git commit -m "feat(paleo): 반경 10km 4단계 고지형 분석 및 다중 도엽 정밀 DEM 합성 엔진을 구현한다"
```

---

### Task 7: `wgpu` 2D 지도 캔버스 (`strata-canvas2d`) 및 3D 입체지형 엔진 (`strata-terrain3d`) 구현

**Files:**
- Create: `crates/strata-canvas2d/Cargo.toml`
- Create: `crates/strata-canvas2d/src/lib.rs`
- Create: `crates/strata-canvas2d/src/viewport.rs`
- Create: `crates/strata-canvas2d/src/vector_tessellator.rs`
- Create: `crates/strata-canvas2d/src/tile_layer.rs`
- Create: `crates/strata-canvas2d/src/map_tools.rs`
- Create: `crates/strata-terrain3d/Cargo.toml`
- Create: `crates/strata-terrain3d/src/lib.rs`
- Create: `crates/strata-terrain3d/src/quadtree_lod.rs`
- Create: `crates/strata-terrain3d/src/atmosphere_agx.rs`
- Create: `crates/strata-terrain3d/src/trench_profile3d.rs`

**Interfaces:**
- Consumes: `wgpu`, `lyon`, `strata_geo::*`, `strata_storage::*`
- Produces:
  - `strata_canvas2d::viewport::MapCanvas2d` (좌표↔픽셀 변환, 줌/패닝, 비동기 타일 렌더링, `lyon` 폴리곤/선분/정점/스냅 마커 렌더링, 레이어 순서 기반 한글 라벨 배치)
  - `strata_canvas2d::map_tools::{ActiveMapTool, CaptureToolState, SelectToolState, MeasureToolState, GeorefToolState}`
  - `strata_terrain3d::Terrain3dEngine` (R32F 높이맵 쿼드트리 LOD, Nishita 하늘 + GTAO + AgX 색보정(`OCIO` 기준표 오차 `< 2e-5`), 14-DOP BVH 피킹, 3D 시굴 트렌치 단면도 및 고고학 입지 평가)

- [ ] **Step 1: Write the failing test in `crates/strata-canvas2d/src/viewport.rs` and `crates/strata-terrain3d/src/atmosphere_agx.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_canvas_world_to_screen_roundtrip_preserves_submillimeter_precision() {
        let vp = ViewportTransform::new([200_000.0, 500_000.0], 1.0, [1920.0, 1080.0]);
        let screen = vp.world_to_screen([200_123.456, 500_654.321]);
        let world = vp.screen_to_world(screen);
        assert!((world[0] - 200_123.456).abs() < 1e-6);
        assert!((world[1] - 500_654.321).abs() < 1e-6);
    }

    #[test]
    fn agx_tonemapping_and_quadtree_lod_produce_seamless_terrain_mesh() {
        let rgb = agx_tonemap_srgb([1.2, 0.8, 0.4]);
        assert!(rgb.iter().all(|&c| (0.0..=1.0).contains(&c)));
        let patches = build_quadtree_patches([0.0, 0.0], 4096.0, 4);
        assert!(!patches.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-canvas2d -p strata-terrain3d`
Expected: FAIL

- [ ] **Step 3: Implement `strata-canvas2d` and `strata-terrain3d`**

2D 지도 뷰포트(`lyon` 벡터 테셀레이션, 스냅 마커 재사용, 지오레퍼런싱 GCP 정합 도구, 반투명 선택 색상 `rgba(255, 235, 59, 45)`)와 `hgis-3d-engine`의 3D 입체지형 파이프라인(쿼드트리 LOD, Nishita Sky, GTAO, AgX, 14-DOP BVH, 3D 시굴 트렌치 단면)을 순수 Rust + `wgpu`로 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-canvas2d -p strata-terrain3d && cargo clippy -p strata-canvas2d -p strata-terrain3d -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-canvas2d crates/strata-terrain3d
git commit -m "feat(render): wgpu 기반 2D 지도 캔버스(strata-canvas2d)와 3D 입체지형 엔진(strata-terrain3d)을 구현한다"
```

---

### Task 8: 도면 조판 스튜디오·PDF/GeoTIFF 출력·칼선 분할 인쇄 (`strata-layout`) 및 검수·제출 꾸러미 (`strata-check-export`) 구현

**Files:**
- Create: `crates/strata-layout/Cargo.toml`
- Create: `crates/strata-layout/src/lib.rs`
- Create: `crates/strata-layout/src/sheet_composer.rs`
- Create: `crates/strata-layout/src/badge_placer.rs`
- Create: `crates/strata-layout/src/pdf_geotiff.rs`
- Create: `crates/strata-layout/src/tile_print.rs`
- Create: `crates/strata-check-export/Cargo.toml`
- Create: `crates/strata-check-export/src/lib.rs`
- Create: `crates/strata-check-export/src/checklist_engine.rs`
- Create: `crates/strata-check-export/src/submit_package.rs`
- Create: `crates/strata-check-export/src/nearby_xlsx.rs`

**Interfaces:**
- Consumes: `strata_storage::*`, `strata_geo::*`, `data/rules/drawing_checklist.v1.json`
- Produces:
  - `strata_layout::sheet_composer::{SheetPaperSize, SheetOrientation, DrawingSheet, apply_paper_change_keeping_footer_band}` (용지 변경 시 축척자·축척 글·좌표계·방위표는 하단 띠 고정, 범례는 용지 밖 배치 허용 `legend_kept_on_page`)
  - `strata_layout::tile_print::{TilePrintPlan, verify_printer_paper_match}` (요청 용지/방향 불일치 시 인쇄 차단 및 안내)
  - `strata_check_export::checklist_engine::ChecklistEngine::evaluate`
  - `strata_check_export::submit_package::export_submission_package` (`EPSG:5179` SHP + `조사도면.pdf` + `MANIFEST.sha256` + `README_submit.txt`, 체크리스트 미통과 항목은 차단하지 않고 `README_submit.txt`에 기록)
  - `strata_check_export::nearby_xlsx::export_nearby_heritage_xlsx`

- [ ] **Step 1: Write the failing test in `crates/strata-layout/src/sheet_composer.rs` and `crates/strata-check-export/src/submit_package.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn changing_paper_size_keeps_scale_bar_crs_and_north_arrow_in_bottom_band_and_preserves_offpage_legend() {
        let mut sheet = DrawingSheet::new_a4_landscape();
        sheet.set_legend_position([350.0, 20.0]); // 용지(297mm) 바깥 배치
        sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Landscape);
        assert!(sheet.legend_position()[0] > 297.0);
        assert!(sheet.decorations_within_paper_bounds());
        assert!(sheet.footer_items_in_bottom_band());
    }

    #[test]
    fn submission_package_writes_epsg5179_shp_manifest_and_records_checklist_findings_in_readme() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("submit");
        let res = export_submission_package_for_test(&pkg_dir, true).unwrap();
        assert!(pkg_dir.join("MANIFEST.sha256").exists());
        assert!(pkg_dir.join("README_submit.txt").exists());
        assert!(res.epsg_code == 5179);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-layout -p strata-check-export`
Expected: FAIL

- [ ] **Step 3: Implement `strata-layout` and `strata-check-export`**

`KaDrawingStudio.cpp`, `LayoutService.cpp`, `LayoutBadgePlacer.cpp`, `TilePrint.cpp`(`0001a93`, `e85de5a`, `c9e58e3` 커밋 규칙 포함), `ChecklistEngine.cpp`, `ExportService.cpp`, `NearbyHeritageExport.cpp`를 순수 Rust로 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-layout -p strata-check-export && cargo clippy -p strata-layout -p strata-check-export -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-layout crates/strata-check-export
git commit -m "feat(layout,export): 도면 조판·칼선 분할 인쇄 및 EPSG:5179 제출 꾸러미·엑셀 내보내기를 구현한다"
```

---

### Task 9: 한글(`.hwpx`) 보고서·GIS 실측 원장(`GisFactLedger`)·시굴 전 평가·MCP 브리지 (`strata-report`) 구현

**Files:**
- Create: `crates/strata-report/Cargo.toml`
- Create: `crates/strata-report/src/lib.rs`
- Create: `crates/strata-report/src/gis_fact_ledger.rs`
- Create: `crates/strata-report/src/assessment_contract.rs`
- Create: `crates/strata-report/src/hwpx_writer.rs`
- Create: `crates/strata-report/src/rag_sqlite.rs`
- Create: `crates/strata-report/src/mcp_server.rs`

**Interfaces:**
- Consumes: `strata_storage::*`, `strata_geo::*`, `zip`, `quick-xml`, `rusqlite`
- Produces:
  - `strata_report::gis_fact_ledger::{GisFactLedger, SiteDistanceFact, UncollectedDomain}` (읽기 전용 GIS 실측 원장)
  - `strata_report::assessment_contract::{FactsBundle, EvidenceItem, EvidenceTier, AssessmentReport, validate_assessment_contract}` (4단계 근거 라벨 및 AI 없이도 생성 가능한 시굴 전 평가 뼈대 + GIS 번호 인용)
  - `strata_report::hwpx_writer::write_hwpx_report` (순수 Rust OWPML `.hwpx` 생성기 — `OpenHwpSDK` C++ 정적 라이브러리 의존성 완전 제거)
  - `strata_report::mcp_server::StrataGisMcpServer` (Antigravity SDK 연동용 읽기 전용 GIS MCP 핸들러)

- [ ] **Step 1: Write the failing test in `crates/strata-report/src/assessment_contract.rs` and `crates/strata-report/src/hwpx_writer.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn assessment_skeleton_renders_field_korean_terms_and_gis_number_citations_without_ai() {
        let ledger = GisFactLedger::sample_gyeongju();
        let report = AssessmentReport::build_deterministic_skeleton(&ledger);
        assert!(validate_assessment_contract(&ledger, &report).is_ok());
        assert!(!report.summary_ko.contains("uncollectedDomains")); // 내부 개발 용어 노출 금지
        assert!(report.summary_ko.contains("[GIS-1]"));
    }

    #[test]
    fn hwpx_writer_creates_valid_owpml_zip_package_with_sections_and_tables() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("발굴조사보고서.hwpx");
        let ledger = GisFactLedger::sample_gyeongju();
        let report = AssessmentReport::build_deterministic_skeleton(&ledger);
        write_hwpx_report(&out, &ledger, &report).unwrap();
        assert!(out.exists());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-report`
Expected: FAIL

- [ ] **Step 3: Implement `strata-report` (`gis_fact_ledger.rs`, `assessment_contract.rs`, `hwpx_writer.rs`, `rag_sqlite.rs`, `mcp_server.rs`)**

`origin/main`의 시굴 전 종합 평가 계약(`eb78ae9`, `5d15c26`, `d2b2495`), 6대 권역·161개 시군구 SQLite RAG 리더, 순수 Rust `.hwpx` 조립기, Antigravity SDK 연동용 `strata-gis` MCP 인터페이스를 구현한다.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p strata-report && cargo clippy -p strata-report -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-report
git commit -m "feat(report): 순수 Rust HWPX 보고서 조판·GIS 실측 원장·시굴 전 종합 평가·MCP 브리지를 구현한다"
```

---

### Task 10: `eframe`/`egui` 데스크톱 본체 (`strata-app`) — `KaTheme`, 1열 리본, 6대 탭, 인스펙터, `--smoke-quit` 완성

**Files:**
- Create: `crates/strata-app/Cargo.toml`
- Create: `crates/strata-app/src/main.rs`
- Create: `crates/strata-app/src/app_state.rs`
- Create: `crates/strata-app/src/ribbon.rs`
- Create: `crates/strata-app/src/left_layer_panel.rs`
- Create: `crates/strata-app/src/right_inspector.rs`
- Create: `crates/strata-app/src/tabs_home_map.rs`
- Create: `crates/strata-app/src/tabs_paleo_3d.rs`
- Create: `crates/strata-app/src/tabs_drawing_report.rs`
- Create: `crates/strata-app/src/dialogs_account.rs`

**Interfaces:**
- Consumes: 앞서 구현한 11개 크레이트 전체 (`strata-types` ~ `strata-report`)
- Produces:
  - 단일 실행 파일 `ka-hgis` (`target/release/ka-hgis.exe`)
  - CLI 인자: `--smoke-quit` (초기화·테마·스키마·렌더러 점검 후 종료 코드 `0`), `--verify-all` (헤드리스 통합 자가검증), `--capture-window <path.png>`

- [ ] **Step 1: Write the failing test in `crates/strata-app/src/app_state.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_shows_home_tab_only_with_no_autosave_and_marks_unsaved_star_in_title() {
        let mut app = StrataAppState::new_for_test();
        assert_eq!(app.active_tab(), MainTab::Home);
        assert_eq!(app.window_title(), "Strata");
        app.create_new_survey_in_memory("경주 성동동 시굴조사", WorkCrs::Epsg5187);
        app.mark_dirty();
        assert_eq!(app.window_title(), "Strata — 경주 성동동 시굴조사 *");
    }

    #[test]
    fn clicking_map_only_ribbon_tool_from_drawing_tab_switches_to_map_tab_first() {
        let mut app = StrataAppState::new_for_test();
        app.set_active_tab(MainTab::Drawing);
        app.trigger_ribbon_action(RibbonAction::StartDrawPolygon);
        assert_eq!(app.active_tab(), MainTab::Map);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p strata-app`
Expected: FAIL

- [ ] **Step 3: Implement `strata-app` (`main.rs`, `app_state.rs`, `ribbon.rs`, `left_layer_panel.rs`, `right_inspector.rs`, `tabs_*.rs`, `dialogs_account.rs`)**

기존 Strata의 디자인과 조작감을 그대로 구현한다:
- 첫 화면은 `홈` 탭만 표시(자동 복원·자동 저장 없음, 미저장 시 제목에 `" *"` 표시).
- 상단 1열 리본(`조사`·`기록`·`자료 받기`·`배경 지도`·`정합`·`내보내기`·`기타` + 우측 주소·지번 검색 `KaAppBar` 및 시도 선택기)과 2열 `subToolbar`.
- 좌측 4구역 레이어 트리(`조사 데이터` / `지적도` / `주변·참조` / `배경 지도`) 및 우측 인스펙터(`속성 조사카드` / `스타일` / `배경 지도 빠른 카드`).
- 중앙 6대 탭(`홈` / `지도` / `고지형분석` / `도면` / `입체지형` / `보고서`) 및 계정·API 키 입력 대화상자.

- [ ] **Step 4: Run test and smoke verification**

Run:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p strata-app -- --smoke-quit
```
Expected: All unit/integration tests PASS, 0 clippy warnings, `--smoke-quit` exits with code `0`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/strata-app
git commit -m "feat(app): eframe/egui 기반 순수 Rust Strata 데스크톱 본체(1열 리본·6대 탭·인스펙터)를 완성한다"
```
