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

## 2. 300줄 초과 파일 — **닫힘 (2026-11-09)**

> 시작할 때 50개였고, 지금은 **1개**다. 남은 1개는 코드가 아니라 데이터다.

| 줄 | 파일 | 사유 |
|---|---|---|
| 316 | `strata-net/src/address_sido_data.rs` | `pub const BUILTIN_KOREA_SIDO` 단일 리터럴. 쪼개면 상수의 의미가 흐려지고 이득이 없다. 바이트 동일 확인(5850/5850). |

나눈 방식은 전부 같다: 원본 파일을 얇은 facade(`pub use`)로 남기고 내용을
`#[path]` 서브모듈로 옮겼다. 그래서 **기존 import 경로가 하나도 바뀌지 않았다.**

가장 컸던 것들:

| 원본 | 줄 | 결과 |
|---|---|---|
| `dialogs_modal.rs` | 3,431 | 100 facade + 7 `modal_*` + 시험 3 |
| `dialogs_download_print.rs` | 3,077 | 208 facade + 13 `dlg_*` + 시험 |
| `gui_ribbon_bar.rs` | 2,892 | 56 facade + 18 `gui_ribbon_*` |
| `gui_tab_map.rs` | 2,286 | 45 facade + 16 `gui_tab_map_*` |
| `gui_tab_drawing.rs` | 1,579 | 49 facade + 12 `gui_tab_drawing_*` |
| `gui_layer_panel.rs` | 1,187 | 21 facade + 7 `gui_layer_panel_*` |
| `gui_shell.rs` | 1,119 | 160 + `shell_workspace`·`shell_dialogs`·시험 2 |
| `right_inspector.rs` | 1,079 | 204 + `ri_types`·`ri_body*`·`ri_*_card`·시험 |

### 나누다가 실제로 잡은 버그 (전부 시험·게이트가 못 잡는 것들이었다)

1. **`render_cad_import_notice` 가 자기 자신을 부르는 껍데기**가 되어 있었다.
   진짜 96줄 본문이 분할 때 사라졌다. 컴파일은 되므로 실행해야만 무한 재귀인 줄 안다.
2. **`paint_map_overlays` 호출이 사라져** 지도 탭의 줌·축척자·격자 오버레이가
   렌더되지 않았다. `dead_code` 경고만이 단서였고, V10 은 탭이 rect 를 반환하는지만 본다.
3. **`sheet_p.rect_filled(map_rect, 0.0, map_bg)` 한 줄이 사라져** 도면 지도 배경
   채우기가 없어졌다. `unused variable: map_bg` 가 유일한 단서였다.
4. **리본 match 분할 의미 오류** — `handle_ribbon_click` 을 두 match 로 나누자
   뒤쪽의 `_ => active_tab = Map` catch-all 이 앞쪽 라벨에도 걸려, 저장·선택·그리기를
   누르면 조용히 지도 탭으로 튕겼다. clippy `unreachable pattern` 이 유일한 단서였다.
5. **속성이 이동을 따라가지 않음 — 5건.** `#[derive(Debug, Clone)]` 이 옮겨진
   구조체를 두고 `pub use` 에 붙어 컴파일 오류가 났고, `ShellGridConfig` 는
   `Clone` 을 잃어 `egui::IdTypeMap` 이 거부했고, `#[must_use]` 는 `topo_record_key`
   에서 잘려 나갔다. **속성은 문장이 아니라서 "줄 타일링 감사"가 구조적으로 못 본다.**

공통 교훈: 파일을 옮기는 것 자체는 안전하지만, **옮긴 뒤 그 코드가 "아직 불리는지"**
는 아무도 검사하지 않는다. 그래서 지금은 잘라낸 조각마다
`{`−`}` 합이 0 인지 쓰기 전에 단언하고, 옮긴 메서드마다 호출자가 있는지 스크립트로
확인한다. 그래도 잡히지 않는 마지막 그물은 사람 눈이다.

검증: `cargo clippy --workspace --all-targets -- -D warnings` = 0,
`cargo test --workspace` = 344 passed, `--verify-all` = V1~V17 PASS.

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
