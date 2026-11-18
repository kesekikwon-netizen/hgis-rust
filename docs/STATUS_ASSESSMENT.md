# Strata / ka-hgis 현황 평가

작성: 2026-11-18. 기준 커밋: `f1835fa82e7798ecb427e148db64081cc43263aa` (`origin/main`, MENU22).
이 문서는 조사만 한 결과다. 제품 코드는 바꾸지 않았다. 기능별 C++ 대조는 [`PORT_PARITY_CHECKLIST.md`](PORT_PARITY_CHECKLIST.md).

## 결론

Rust 판은 조사 파일을 만들고, 면을 그리고, 저장하고, 받은 지도를 다시 열고, 도면을 그림 PDF로 뽑는 입구가 있다. 제출 꾸러미는 꺼져 있고, 시굴 「구역에 깔기」와 등고선은 하지 않은 일을 했다고 말한다. C++ 앱과 단추·문구·가장자리 경우까지 같다고 볼 단계는 아니다. 2026-11-18 소유자 인계는 MENU22(지도 투명도 카드의 포커스와 드래그)에서 멈췄고, 다음으로 적힌 일은 지도 카드와 도면 카드의 위치를 디스크에 따로 기억하는 것이다. 그 일은 아직 `main`에 없다. 스크린샷의 브랜치 `fix/opacity-selected-layer`(커밋 `fea3e42`)는 GitHub에 없다.

Google Drive에는 이 환경에서 접속하지 못했다. 이 파일과 대조표는 저장소에만 있다. 드라이브 폴더로 올리는 일은 데스크톱에서 인증된 세션이 해야 한다.

## 1. 앱과 저장소

Strata(실행 파일 이름 `ka-hgis`)는 고고학 현장용 데스크톱 GIS다. 원본은 C++20 / Qt 6 / QGIS(`kesekikwon-netizen/gogohak1`)이고, 이 저장소(`kesekikwon-netizen/hgis-rust`)는 그 기능을 Rust + egui로 옮기는 중이다. QGIS를 링크하지 않는다. 작업 좌표계는 EPSG:5187(기본) 또는 5186, 제출은 EPSG:5179, 타일은 EPSG:3857. 좌표 변환은 PROJ(`strata_engine::Transformer`)만 쓴다.

워크스페이스 버전은 `2.0.0`, 에디션 2021, 툴체인은 `rust-toolchain.toml`의 Rust 1.99.1. `unsafe`는 워크스페이스에서 금지다. 라이선스 필드는 `GPL-2.0-or-later`인데, 계획 문서의 배포 목표는 복사 좌파 없이 배포하는 것이다. 이 긴장은 소유자 결정으로 남아 있고 이번 조사에서 고치지 않는다.

크레이트(루트 `Cargo.toml` 멤버):

| 크레이트 | 하는 일 | 성숙도 |
|---|---|---|
| `strata-app` | egui 화면, 리본, 지도, 도면, 받기. 진입 `src/main.rs` | 가장 큼. 화면은 많이 연결됐고 꺼 둔 단추와 가짜 성공 문구가 남아 있다 |
| `strata-engine` | PROJ, GEOS, 선택 기능 GDAL(SHP/GPKG/DXF/HFA/GTiff/JPEG/PNG) | 앱이 `gdal` 기능을 켠 채로 의존한다. 소스 빌드 |
| `strata-geo` | 버퍼, 분할, 시굴격자 계산, 정합 변환 | 계산 라이브러리는 시험이 있다. 화면 연결은 기능마다 다르다 |
| `strata-storage` | 조사 GPKG 열기·저장, 최근 조사, Qt ini | 도형 저장은 동작한다. C++ 작업공간(`.qgz`, 내장 프로젝트)은 보존만 한다 |
| `strata-refmap` | 받은 참조 지도 가공 | 지질·토양·수계 준비 경로가 있다 |
| `strata-layout` | 도면 배치, 그림 PDF | 벡터 PDF(R3)가 아니다 |
| `strata-check-export` | 검수·제출 | 검수 판정은 연결됐다. 꾸러미 단추는 꺼져 있다 |
| `strata-net`, `strata-webview`, `strata-ngii-run`, `strata-ngii-fixture` | 사이트 접속, 숨은 브라우저, 수치지형 고정 응답 | 유산은 고정 응답으로 멀리 갔다. 수치지형 「도엽 확인」 단추는 아직 꺼져 있다 |
| `strata-heritage-fixture`, `strata-heritage-run` | 유산 인트라넷 고정 사이트와 실행 | 실제 사이트 성공은 문서가 명시적으로 부정한다 |
| `strata-secrets` | 키·계정 저장. Windows에서는 DPAPI | 키 값은 코드에 없다 |
| `strata-types`, `strata-canvas2d`, `strata-mapengine`, `strata-topo`, `strata-paleo`, `strata-terrain3d` | 공용 타입, 지도 보기, 라벨, 수치지형 묶음, 고지형, 3D | 3D 탭은 안내 문구와 비활성 경로가 남아 있다 |

Rust 소스 약 1,119개 파일, 158,125줄(`find crates -name '*.rs'`). 소스에서 센 `#[test]`는 약 1,908개, `#[ignore]` 2개다. 통과 여부의 기준은 Windows CI다(6절). C++ 기준본은 `src`+`tests` 헤더·cpp 107,245줄, 파일 792개.

### 빌드와 실행

검증 기계는 Windows다. GDAL·PROJ는 시스템에 설치하지 않고 크레이트가 소스에서 빌드한다. PATH에 `cmake`와 `sqlite3`가 있어야 한다. 실행 파일은 `target/release/ka-hgis.exe`다.

```bash
cargo build --workspace -j 3
cargo test --workspace -j 3
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p strata-app -j 3
./target/release/ka-hgis --smoke-quit
./target/release/ka-hgis --verify-all
```

이 명령은 Windows CI(`.github/workflows/ci.yml`)가 도는 검증이다. `--smoke-quit`과 `--verify-all`은 그 빌드가 만든 `ka-hgis.exe`로 확인한다. Linux 빌드·시험·CI는 범위 밖이다(소유자 결정 2026-11-18).

Windows CI(`.github/workflows/ci.yml`)는 같은 세 검사를 `windows-latest`에서 하고, 이어서 `ka-hgis --verify-all`, 그리고 `strata-engine`·`strata-check-export`의 `--features gdal`을 별도 잡으로 돌린다. 앱 의존성이 이미 `gdal`을 켜므로 워크스페이스 빌드에도 GDAL 소스 빌드가 들어간다. 첫 빌드는 수십 분이다. 증분 빌드가 꼬이면 `cargo clean -p strata-app` 뒤 다시 빌드한다(인수인계 문서의 LLVM `anon` / LNK1120 사례).

화면 확인은 `ka-hgis --snap <폴더> --snap-steps "..."` 이고, 단계는 `crates/strata-app/src/snap_shot_steps.rs`에 있다. 사용자 원본 조사 파일은 임시 사본만 쓴다.

### C++ 기준을 여는 법

깃 체크아웃에는 `target/cpp-ref`가 없다(`target/`는 gitignore). 문서가 가리키는 위치는 다음과 같다.

- 원격: `https://github.com/kesekikwon-netizen/gogohak1`
- 핀: `b036c0905f6ac1e97b6d142a5e101ac5d30ca7c7` (`b036c09`, 2026-10-31, 「조판 장식 용지 밖 이탈 및 접힌 레이어 묶음 체크 복구」)
- 2026-11-18에 `refs/heads/main`도 같은 커밋이다. C++ main은 그 뒤로 움직이지 않았다.
- 받는 법: `git clone https://github.com/kesekikwon-netizen/gogohak1 target/cpp-ref` 후 `git -C target/cpp-ref checkout b036c09`

이번 조사는 그 커밋을 `/tmp/cpp-ref`에만 받았다. 저장소에는 넣지 않았다. 투명도 레일은 `src/app/KaLayerOpacityRail.cpp`, 지도 연결은 `src/app/MainWindow.cpp:963`, 도면 연결은 `src/app/KaDrawingStudio.cpp:1196`.

### 다른 저장소

이 저장소 안에서는 `dotak`, `dotak-independent-rubbing`, `dotak-studio-hgis-rust-workspace`라는 이름이 나오지 않는다. 사이드바의 그 폴더들과의 코드 의존은 여기 없다.

스캔한 컬러/흑백 도면 TIFF에서 격자선을 지우는 기능(스크린샷 오른쪽 실험)도 C++ `b036c09`와 이 Rust 트리 양쪽에서 찾지 못했다. 「좌표 격자」는 도면 위에 격자를 그리는 다른 기능이다. 격자선 지우기는 이 이식의 남은 항목이 아니다.

## 2. 브랜치, PR, 최근 커밋

| 브랜치 | 팁 | 의미 |
|---|---|---|
| `main` = `origin/main` | `f1835fa` 2026-11-18 MENU22 | 개발 줄. 열린 PR 없음 |
| `origin/feat/real-survey-file` | `64e1408` | `main`보다 327커밋 뒤, 고유 35커밋. 현재 개발 줄이 아니다 |
| `origin/backup/local-2026-11-10-work` | `869d1e4` 2026-11-16 「토양도 경로 보존」 | 백업 팁. `main`에 이미 토양 경로가 있다 |
| `fix/opacity-selected-layer` / `fea3e42` | GitHub에 없음 (`gh api` 422) | 스크린샷 PC의 로컬 작업으로 보인다. 이 클론에서는 내용을 열 수 없다 |

최근 2주 주제:

- 2026-11-16: 편집(여러 개 선택, 묶기, 나누기, Delete), 받은 지도 다시 열기, SHP·그림 레이어, 정합 변환 계산, 유산 받기 시작, 인수인계 문서.
- 2026-11-17: 유산 파이프(HER-CODEX, 고정 응답·숨은 세션), UI1–UI6(옛 지도, 범례 크기, 인쇄 탭, 래스터 정합, 홈을 폴더 탐색으로, 작은 화면), MENU1–4, SNAP1.
- 2026-11-18: MENU5–22. 팝업·키보드가 그리던 도형을 지우지 않게 하고, CRS 실패를 거절하고, 버퍼 저장, 용지 카드, 둥근 범례, 지도 투명도 카드 이동과 슬라이더 포커스.

GitHub Actions run [44252014705](https://github.com/kesekikwon-netizen/hgis-rust/actions/runs/44252014705)는 이 HEAD에서 `check-and-test`, `gdal-feature`, `newest-stable`이 모두 success다(Windows, 45분 44초). 그 앞 7개 `main` 푸시도 success다. 문서 PR [44297770561](https://github.com/kesekikwon-netizen/hgis-rust/actions/runs/44297770561)도 같은 세 잡이 success다.

## 3. 기능이 어디까지 왔는가

판정 단어는 대조표와 같다. `done`은 그 조각이 C++와 같고 입구가 살아 있다. `partial`은 일부만. `missing`은 없거나 항상 꺼져 있다. `divergent`는 동작이 다르거나, 하지 않은 일을 했다고 말한다.

| 영역 | 지금 | 근거 |
|---|---|---|
| 조사 파일 만들기·열기·저장 | partial | 7개 도메인 레이어 GPKG, 원자적 세대 저장, 미저장 질문(MENU9·10). 내장 QGIS 작업공간·복구 사본·다른 PC에서 바뀐 파일 경고는 없다 |
| 그리기·선택·되돌리기 | partial | 면 그리기, 선·점 캡처 경로, Shift·상자 선택, 묶기·나누기, 레이어별 되돌리기가 커밋과 시험 파일로 이어져 있다. 쉽게그리기·구간 분리는 비활성. 꼭짓점 메뉴는 꺼져 있다 |
| 투명도 카드 | partial, 그리고 divergent | 지도 카드만 있다. 세션 안에서만 이동한다. 슬라이더는 고른 레이어만이 아니라 전역 값과 배경을 같이 바꾼다. 도면 카드와 디스크 복원은 없다 |
| 도면·인쇄 | partial | 용지 카드, 둥근 범례, 그림 PDF 내보내기는 단추에 연결되어 있다. 나눈 장 PDF는 꺼져 있다. 제출용 벡터 PDF는 아니다 |
| 검수·제출 | partial / missing | 27규칙 판정과 「위치 보기」는 있다. 「제출 꾸러미 만들기…」와 「레이어 5179 변환…」는 `지시 R2`로 꺼져 있다 |
| 배경·받은 지도 | partial | VWorld 타일, 지적 WMS, 지질·토양·수계 준비, 그림·SHP 적재, `<조사>.strata-maps.json` 재오픈. OSM/Google은 VWorld로 바뀐다 |
| 유산 인트라넷 | partial | 「받기」는 계정·조사가 있으면 `start_heritage_fetch`를 호출한다. 실제 사이트 로그인은 문서가 미실행으로 적는다 |
| 수치지형·DEM 자동 받기 | partial | 리본은 받아 둔 파일을 올리거나 숨은 주문을 시작한다 (`topo_download_run.rs:86`). 그 코드는 Windows 전용 크레이트를 조건 없이 가져온다. 옛 창의 「도엽 확인」만 `WB2`로 꺼져 있다 |
| 정합 | partial | 래스터는 점 짝으로 월드 파일·`.prj`·백업을 쓴다 (`gui_ribbon_align_actions.rs:37`, `raster_alignment.rs:88`). 벡터·CAD 세션과 먹선 스타일은 없다 |
| 시굴격자·등고선 | divergent | 「구역에 깔기」는 격자를 만들지 않고 배치했다고 말한다. 등고선은 파일을 읽지 않고 「점 248개」를 보여 준다 |
| 홈 | divergent | C++ 시작 페이지 대신 폴더 탐색(UI5, `ab8c8f1`). 소유자 결정으로 문서에 있다 |
| 더보기 남은 항목 | divergent | 맞지 않는 항목은 계정 창을 연다 (`gui_ribbon_menus_b.rs:113`) |

현장 결과를 막는 구멍은 제출 꾸러미, 작업공간 왕복, 정합의 나머지, 그리고 하지 않은 일을 했다고 말하는 등고선·시굴 깔기다.

## 4. 문서, TODO

처음 읽는 순서: `docs/CURSOR_HANDOVER.md` → `docs/HANDOVER.md` → `AGENTS.md` → `docs/RUST_PLAN.md` → `docs/RESULTS.md` 마지막. `CLAUDE.md`는 C++ 저장소에서 온 것이라 제품 규칙만 유효하다.

서로 날짜가 다르다. 다음 개발이 옛 표를 현재 상태로 읽으면 이미 고친 일을 다시 한다.

| 문서 | 날짜·기준 | 지금 쓰면 안 되는 이유 |
|---|---|---|
| `docs/RUST_PARITY_CHECKLIST.md` | 2026-11-12, Rust `fb26f67` | 저장이 도형을 안 쓰고 제출 SHP가 가짜라고 적는다. 그 뒤 저장·검수·파일 적재가 들어왔다 |
| `docs/PARITY_STATUS.md` | 2026-11-14, `8c2913bf`, 89기능 | 기능 목록으로는 아직 쓸 수 있다. 상태 열은 홈·PDF 단추·유산 받기·SHP 적재·폴리곤 묶기 이후다 |
| `docs/RUST_PLAN.md` §5 | 2026-11-13 | 「선·점 그리기, 제출, 도면 PDF, 유산 받기가 안 됐다」는 그 날짜의 문장이다 |
| `docs/research/more/MORE0.md` | 메뉴 감사 | 「To build」 43항은 받은 지도 재오픈·저장 질문·화살표 키처럼 그 뒤 커밋이 닫은 항목을 아직 연다 |
| `docs/HANDOVER.md` §8 | 2026-11-16 우선순위 + 뒤에 MENU 일지 | 앞의 8절과 뒤의 MENU 일지가 한 문서에 섞여 있다. 끝은 MENU22와 `CURSOR_HANDOVER.md`다 |
| `docs/CURSOR_HANDOVER.md` | 2026-11-18 MENU22 | 지금 손댈 다음 일의 기준. 투명도 값의 적용 대상과 도면 카드·디스크 위치를 다음으로 적는다 |
| `docs/research/menus/CURRENT_MENU_AUDIT.md` | 메뉴 113행 | 전체 기능 대조가 아니다. 완료 20, 부분 31, 미구현 8, 검증 안 됨 54(MENU22 시점) |

`crates/` 안의 `TODO` / `FIXME` / `todo!` / `unimplemented!`는 없다. 남은 일은 그 대신 툴팁 「아직 Rust 판에서 연결하지 않았습니다 (지시 …)」로 꺼 두었다. 이번 세션에 확인한 지시 번호: `R2`(제출), `R7`(GeoTIFF), `W-50`(나눈 장 PDF), `W-H1`(버퍼 엑셀), `WB2`(수치지형 도엽 확인), `W-56`(입체지형), `W-A12`(레이어 불러오기), `W-T1`(시굴 메뉴), `R4b`(오프라인 타일), `CM2a-2`, `CM2a-3`, `MENU17`(쉽게그리기·구간 분리). `third-party/rusqlite`의 TODO는 벤더 코드라 제품 핫스팟이 아니다.

## 5. 기술 부채와 위험

1. 대조 문서가 네 시점으로 갈라져 있다. 새 담당자가 `RUST_PARITY_CHECKLIST.md`부터 고치면 이미 닫힌 저장·적재 버그를 다시 연다.
2. `apply_rail_opacity`는 고른 레이어, 전역 `layer_opacity`, 배경 투명도를 한 번에 쓴다(`gui_tab_map_opacity.rs:44-49`). C++는 현재 레이어만 바꾼다(`MainWindow.cpp:989-1000`). 슬라이더 한 번이 위성까지 흐리게 할 수 있다.
3. 등고선과 시굴 「구역에 깔기」는 성공 문장을 내면서 도형을 만들지 않는다. 현장 사용자가 결과를 믿게 되는 종류의 차이다.
4. 제출 꾸러미가 꺼져 있어 이 앱만으로는 5179 SHP + PDF + MANIFEST를 만들지 못한다. 검수 창은 열리므로 끝난 것처럼 보인다.
5. 받은 지도 목록은 조사 파일 옆 `.strata-maps.json`이다. C++는 조사 파일 안 작업공간이다. 두 앱이 같은 조사를 열면 참조 지도·색·도면 장이 같이 오지 않는다.
6. 복구 사본(C++ 2분)은 없다. 제품 결정은 자동 저장 없음이다. 두 문장을 한 구현으로 합치지 말고, 복구 사본을 넣을지 소유자가 정해야 한다.
7. 검증은 Windows CI뿐이다. Linux 빌드·시험·CI는 범위 밖이다(소유자 결정 2026-11-18). DPAPI, WebView2, 인쇄 보조 프로세스는 Windows 기능이다.
8. `AGENTS.md`는 구현자가 `main`에 푸시하지 않고 사용자에게 보고하지 않는다고 적는다. `CURSOR_HANDOVER.md`와 `RUST_PLAN.md` §6은 끝나면 `main`에 올리고 바탕화면 실행 파일을 바꾼다고 적는다. 누가 합치는지가 문서마다 다르다.
9. 공개 저장소다. 키·계정·조사 좌표를 문서와 로그에 넣지 않는다. Drive 본문 URL도 인수인계가 공개 트리에 복사하지 말라고 한다.
10. 파일당 300줄 분할이 기능 하나를 여러 파일로 찢어, 입구를 찾을 때 파일 이름 규칙(`*_a`, `*_b`)을 알아야 한다.

## 6. 검증

기준은 Windows CI다. Linux 빌드, Linux 시험, Linux CI는 하지 않는다(소유자 결정 2026-11-18). WU-11은 그 결정으로 떨어졌다.

| 기준 | 실행 | 결과 |
|---|---|---|
| 제품 `main` `f1835fa` | [44252014705](https://github.com/kesekikwon-netizen/hgis-rust/actions/runs/44252014705) | `check-and-test`, `gdal-feature`, `newest-stable` success |
| 이 문서 PR `fa7c62a` | [44297770561](https://github.com/kesekikwon-netizen/hgis-rust/actions/runs/44297770561) | 같은 세 잡 success |

도구 체인은 `rust-toolchain.toml`의 Rust 1.99.1. 잡은 `.github/workflows/ci.yml`의 `windows-latest`다. 인쇄 보조 프로세스는 `powershell.exe`와 `scripts/print-device.ps1`이다.

## 7. 다음 순서

위가 먼저다. 각 항목의 검증은 Windows CI다. 단위는 대조표의 WU 목록이다. WU-11은 하지 않는다.

1. **투명도 세 조각.** (a) 슬라이더는 고른 레이어만 바꾼다. (b) 도면에도 C++와 같은 카드를 붙인다. (c) 지도 `ui/opacityRail/map`과 도면 `ui/opacityRail/drawing`을 사용자 ini에 따로 저장하고, 연 뒤 늦은 읽기가 사용자가 이미 옮긴 자리를 덮지 않게 한다. (c)의 도면 키는 (b)가 있어야 의미가 있다. 지도 키만 먼저 넣어도 된다. 기본 위치는 C++의 왼쪽 위 `(10, 10+inset)`이 아니라, 주석이 적은 소유자 UI11의 왼쪽 아래다. 그 차이를 되돌리지 않는다.
2. **거짓 성공 두 곳.** 시굴 「구역에 깔기」와 등고선 「점 248개」. 계산을 연결하거나, 연결 전까지 단추를 끄고 만든 것처럼 말하지 않는다.
3. **제출 꾸러미(R2).** 검수 오류가 있어도 한 번 묻고 README에 남긴 뒤 5179 SHP와 MANIFEST를 만든다. 도면 용지가 없으면 PDF를 넣지 않고 이유를 알린다(계획 문서의 R2 통과 문).
4. **더보기 sink.** CSV 기준점, 정보, 임시 자료 정리, 메뉴의 지도 넓게/패널 접기가 계정 창으로 떨어지지 않게 한다. 계정 창이 맞을 때만 연다.
5. **정합의 나머지.** 래스터의 점 짝·월드 파일·백업은 연결되어 있다. 다시 만들지 않는다. 남은 것은 벡터·CAD 세션과 맞춘 스캔의 먹선 스타일이다.
6. **작업공간 2D.** C++가 조사 파일 안에 넣는 레이어 순서·색·참조 지도·도면 장. 옆의 `.strata-maps.json`은 그 전까지의 임시 목록이다.
7. **수치지형 옛 창.** 리본은 이미 숨은 주문으로 간다. 꺼져 있는 「도엽 확인」을 두 번째 구현으로 살리지 않는다. 실제 사이트 로그인은 사용자 허락 뒤에만 한다.
8. **인쇄·조판 (WU-12).** 용지 210×297과 현장 317×220, 미리보기, 300/600 DPI 선택은 있다. 「PDF 내보내기」는 300 DPI 그림이고, 나눈 장 PDF는 꺼져 있다. 표제·범례·방위·글꼴을 C++ 조판과 맞추는 일은 대조표 5절이다. 확인은 Windows다.

`fea3e42`를 이 순서에 넣지 않는다. 그 커밋이 푸시되기 전에는 로컬 PC의 작업 복사본으로만 다룬다. 내용이 필요하면 그 브랜치를 원격에 올린 뒤 다시 읽는다.

## 8. Google Drive

시도: Google Drive MCP. 상태 `needsAuth`. `mcp_auth` 응답은 「Interactive MCP authentication is only available in the Cursor desktop IDE」였다. 이 클라우드 에이전트에서는 인증 카드를 마칠 수 없다. 기존 `hgis-rust` / `ka-hgis` / `Strata` 폴더를 검색하지 못했고, 파일을 올리지 못했고, Drive에 있는 인수인계나 C++ 자료를 읽지 못했다.

올릴 파일은 이 저장소의 `docs/STATUS_ASSESSMENT.md`와 `docs/PORT_PARITY_CHECKLIST.md`다. 데스크톱에서 Drive가 인증된 뒤, 같은 이름의 폴더가 없으면 `hgis-rust`를 만들어 그 두 파일을 넣으면 된다. 공개 저장소 커밋에는 Drive URL이나 본문을 복사하지 않는다(`CURSOR_HANDOVER.md`의 자료 절).

## 9. 이번 조사에서 정한 것

| 판단 | 이유 | 근거 |
|---|---|---|
| 제품 코드는 커밋하지 않음 | 요청이 조사와 문서뿐임 | 이 브랜치의 변경 파일은 문서 둘 |
| C++는 `/tmp/cpp-ref`의 `b036c09`로 읽음 | 체크아웃에 `target/cpp-ref`가 없음. 원격 main도 그 커밋 | `git ls-remote` |
| 11월 12·14일 표의 상태 열을 현재로 복사하지 않음 | 그 사이 저장·적재·유산·PDF·홈이 바뀜 | `docs/PARITY_STATUS.md` 머리, 이번 세션에 연 파일 |
| 조사구역 기본 투명도 0.25를 main의 사실로 적지 않음 | 새 조사는 `layer_opacity: 1.0`. 0.25는 슬라이더를 25% 위치에 둔 시험 | `real_survey_load.rs:28`, `opacity_placement_input_tests_a.rs:65-67` |
| 투명도 기본 모서리를 C++ 왼쪽 위로 되돌리는 일을 추천하지 않음 | 코드가 소유자 UI11로 왼쪽 아래를 명시 | `gui_tab_map_opacity_placement.rs:69-71` |
| 래스터 정합의 월드 파일은 이미 쓴다 | 후속 대조가 `create_aligned_raster`를 가리켰고, 그 함수가 월드 파일과 `.prj`를 쓴다 | `raster_alignment.rs:88-89`, `gui_ribbon_align_actions.rs:37` |
| 제출 실물은 `submit_builder`이고 `submit_export.rs`가 아니다 | 빌더는 `gdal` 기능 뒤에 있고 앱이 그 기능을 켜지 않는다. 다른 파일은 고정 네모를 쓴다 | `strata-check-export/src/lib.rs:21-22`, `Cargo.toml:17` |
| Linux 작업은 하지 않음 | 소유자 결정 2026-11-18. 검증은 Windows CI | WU-11 dropped |
| 인쇄·조판은 WU-12로 나눔 | 용지 크기는 이미 같고 PDF는 그림이다 | 대조표 5절 |
