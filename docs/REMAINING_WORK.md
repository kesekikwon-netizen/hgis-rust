# 남은 작업 (Rust 전환)

기준: `D:\hgis` C++ 완성본 `d7afcc8` · 타겟: `D:\hgis rust` (11 crate).

이 파일은 **아직 끝나지 않은 것**만 적는다. 끝난 일은 커밋 메시지와
`Strata_Rust_전환_개발_마스터플랜.html` 에 있다.

## 1. 시추 도면 레이어 스타일 미적용 (열린 항목 — 반드시 남길 것)

`strata-paleo::borehole_maps` 는 다섯 컬러 램프를 **계산해서 돌려주지만**,
`crates/strata-app/src/borehole_bridge.rs` 는 그 램프를 지도 레이어에 **칠하지 않는다**.

C++ `GeoInfoBoreholeMaps.cpp:45-81`(`addIdwMap`)과 `:83-166`(`addIsopachContourLayer`)이
하는 일 중 아직 옮기지 않은 것:

- `QgsColorRampShader` 로 래스터에 의사색 부여 (`minVal`/`maxVal` + `ColorRampItem` 목록)
- 레이어 불투명도 `0.72` (`:71`)
- 등층후선 선 색 `#92400e`, 굵기 `0.42` (`:143-144`)
- 등층후선 라벨 7.8pt, 색 `rgb(120,53,15)`, 흰색 버퍼 0.8 (`:149-154`)
- `QgsBilinearRasterResampler` 확대 보간 (`:72`)
- 범례 그룹 「참조 지도」 배치, `markReferenceLayer` (`:73-79`, `:158-164`)
- 같은 제목 레이어가 이미 있으면 지우고 새로 넣기 (`:55-56`, `:88-89`)

이 항목이 위험한 이유: 테스트는 전부 통과한다. 램프 값이 맞는지 보고,
파일이 만들어졌는지만 보기 때문이다. **화면에 색이 안 칠해져도 초록불이 뜬다.**
사람이 앱을 열어 확인하기 전에는 닫지 말 것.

## 2. 300줄 초과 파일 50개 (Phase 5 잔여)

`strata-geo` 는 전부 300줄 이하로 끝냈다(커밋 `198fd20`, `408fec3`).
남은 것은 앱·네트워크·저장 계층이다.

가장 큰 것부터:

| 줄 | 파일 |
|---|---|
| 3431 | `crates/strata-app/src/dialogs_modal.rs` |
| 3060 | `crates/strata-app/src/dialogs_download_print.rs` |
| 2892 | `crates/strata-app/src/gui_ribbon_bar.rs` |
| 2286 | `crates/strata-app/src/gui_tab_map.rs` |
| 1647 | `crates/strata-layout/src/sheet_composer.rs` |
| 1579 | `crates/strata-app/src/gui_tab_drawing.rs` |
| 1548 | `crates/strata-net/src/heritage_intranet.rs` |
| 1372 | `crates/strata-net/src/address_search.rs` |
| 1362 | `crates/strata-net/src/vworld_client.rs` |
| 1290 | `crates/strata-check-export/src/checklist_engine.rs` |

마스터플랜 Chapter 07 이 제안한 분할 단위를 그대로 쓰면 된다:
`dialogs_modal` / `dialogs_download_print` → 대화상자별 파일,
`gui_ribbon_bar` → `ribbon/{mod,layout_metrics,group_*,subtoolbar}`,
`sheet_composer` → 조판/PDF 분리.

## 3. Phase 4 잔여 (정밀 DEM·고지형 동기화)

`strata-paleo::dem_synthesizer` 는 다중 도엽 병합·무잠금 저장·이음새 블렌딩이 있고
시험도 있다. 남은 것은 앱 쪽 동기화다.

- 5km 단독 지질/시추 UI 와 범례를 C++ `67a3f33` 과 맞추기
- Imhof 다방향 음영·경사도 시각화를 실제 화면에 연결

## 4. 이 세션에서 배운 것 (다음 사람에게)

- **`PRAGMA integrity_check` 는 지오메트리 blob 을 검사하지 않는다.** GPKG 의
  geometry flags 바이트가 틀려도 `ok` 가 나온다. GDAL `ogrinfo` 가 잡았다.
  GPKG 를 만들면 반드시 `ogrinfo` 로 한 번 열어 볼 것.
- **테스트가 통과한다고 규칙이 고정된 것은 아니다.** `bed_gl` 삼항식과 `max()` 식은
  입력이 갈리는 지점이 따로 있다. 경계값 하나만 보고 "덮었다" 고 말하지 말 것.
- **커밋 해시를 말할 때는 `git log` 출력을 그대로 옮길 것.** 이 세션에서 존재하지 않는
  해시(`1f632db`)를 보고한 적이 있다. 실제 커밋은 `408fec3` 이다.
