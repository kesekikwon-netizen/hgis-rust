# 남은 작업 (Rust 전환)

기준: `D:\hgis` C++ 완성본 `d7afcc8` · 타겟: `D:\hgis rust` (11 crate).

이 파일은 **아직 끝나지 않은 것**만 적는다. 끝난 일은 커밋 메시지와
`Strata_Rust_전환_개발_마스터플랜.html` 에 있다.

## 1. 시추 도면 레이어 스타일 — **닫힘 (2026-11-09)**

> 이 항목은 **해결됐다.** 아래 기록은 "테스트는 통과하는데 화면은 비어 있었다"는
> 사실을 지우지 않으려고 남긴다. 같은 실수가 다른 곳에서 반복될 수 있기 때문이다.

닫은 방법:
- `strata-paleo::borehole_color_ramp` 에 `rasterize_ramp_rgba` / `rasterize_ramp_rgba8` 추가 —
  Float32 격자를 램프로 칠해 RGBA 로 바꾼다. 알파는 C++ `:71` `setOpacity(0.72)` = 184.
- `borehole_maps::BoreholeMapBundle` 에 `painted_grids` 추가 — C++ 은 QGIS 레이어가
  격자를 들고 있지만(`:75` `addMapLayer`) Rust 는 그 계층이 없어 번들로 넘긴다.
- `strata-app::borehole_map_painter` 신설 — 격자를 egui 텍스처로 지도 위에 실제로 그린다.
  `gui_tab_paleo` 가 이 함수를 매 프레임 부른다.
- **V17 게이트** 추가 — 격자가 남았는지·불투명 픽셀이 나오는지·알파가 184 인지 확인한다.
  현재 출력: `grids=5 ramps=5 alpha=184 visible_px=162000`.

V16 만 있던 동안 왜 초록불이었는가: V16 은 파일이 만들어졌는지만 봤다.
**파일 생성과 화면 표시는 다른 일이다.** 게이트를 만들 때 "무엇이 만들어졌나"와
"사용자가 무엇을 보나"를 함께 물어야 한다.

아직 남은 스타일 세부(사소하지만 C++ 과 다름):
- `QgsBilinearRasterResampler` (`:72`) — Rust 는 `TextureOptions::LINEAR` 로 근사한다.
- 등층후선 **벡터** 레이어를 지도에 칠하는 것(`:143-164` 선 색·라벨) — GPKG 파일은
  만들지만 화면에 선을 그리는 경로는 아직 없다(래스터 5종만 칠한다).
- 같은 제목 레이어 교체(`:55-56`, `:88-89`) — Rust 는 `painted_grids` 를 새로
  덮어쓰므로 결과는 같다(옛 격자가 남지 않는다).

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
