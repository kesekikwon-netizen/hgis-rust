# Strata Rust 판 인수인계 (2026-11-17 유산 흐름 갱신)

이 문서 하나로, 다른 컴퓨터에서 어떤 AI(Claude, Codex, Gemini 등)가 와도 이어서 개발할 수 있게 정리했다.
처음 오는 에이전트는 이 문서 → `AGENTS.md` → `docs/RUST_PLAN.md` → `docs/RESULTS.md` 의 마지막 절 순서로 읽는다.
`CLAUDE.md` 는 C++ 저장소에서 복사해 온 것이라 제품 규칙(조사 레이어, 좌표계, 키, 정해 둔 동작)만 유효하고, 빌드·시험·git 부분은 이 저장소에 맞지 않는다.

## 1. 무엇을 만드는가

- **Strata**: 고고학 현장 조사용 데스크톱 GIS. 원래 C++20/Qt 6/QGIS 앱이고, 이 저장소는 그것을 **Rust + egui** 로 옮기는 중이다.
- 목표(사용자 결정 2026-11-14): C++ 앱의 **모든 기능을 C++ 과 같게**(단추 하나, 문구 하나까지) 옮기고, 그 위에서 다듬는다. 끝에는 GPL 같은 복사 좌파 부품 없이 배포한다(QGIS 는 쓰지 않는다).
- 사용자는 **개발자가 아닌 고고학 현장 전문가**다. 요청은 짧은 한국어와 스크린샷으로 온다. 대답·보고는 한국어로 짧게, 결과부터 쓴다. 코드 식별자·명령은 영어 그대로.
- 작업 좌표계 EPSG:5187(기본) 또는 5186, 제출 좌표계 EPSG:5179, 타일 EPSG:3857.

## 2. 저장소와 환경

| 항목 | 내용 |
|---|---|
| 이 저장소 | `https://github.com/kesekikwon-netizen/hgis-rust` (공개), 브랜치 `main` 하나로 개발 |
| C++ 원본(기준) | `https://github.com/kesekikwon-netizen/gogohak1`. 문서의 `target/cpp-ref/src/...` 인용은 이 저장소를 `target/cpp-ref` 에 clone 한 자리다(이 PC 의 참조 판: 커밋 `b036c09`). 다른 PC 에서는 `git clone https://github.com/kesekikwon-netizen/gogohak1 target/cpp-ref` 후 `git -C target/cpp-ref checkout b036c09` |
| OS·도구 | Windows 11, Visual Studio 2022 Build Tools(MSVC), CMake, Git. Rust 는 `rust-toolchain.toml` 이 `1.99.1` 로 고정 |
| GDAL·PROJ | 외부 설치 없음. `gdal-src 0.3.0`(GDAL 3.12) 과 `proj-sys`(PROJ 9.6.2)를 소스에서 함께 빌드해 실행 파일 안에 넣는다. 켠 GDAL 드라이버: shape, gpkg, dxf, hfa, gtiff, jpeg, png (`crates/strata-engine/Cargo.toml`). 첫 빌드는 오래 걸린다(수십 분) |
| C++ 앱 실행 | Rust 개발에는 필요 없다. C++ 동작을 화면으로 볼 때만 OSGeo4W `qgis-dev` 가 필요하다(`docs/other-pc-setup.md`, C++ 시절 문서) |

## 3. 빌드·시험·실행

```bash
cargo build -j 3 -p strata-app                    # 디버그: target/debug/ka-hgis.exe
cargo build --release -j 3 -p strata-app          # 배포용: target/release/ka-hgis.exe
cargo test -j 3 -p strata-app                     # 앱 시험 (2026-11-17: 837개)
cargo test -j 3 -p strata-engine --features gdal  # GDAL 이 필요한 엔진 시험
cargo test -j 3 -p strata-geo                     # 기하 계산 시험
cargo clippy -j 3 -p strata-app -p strata-engine -p strata-geo --all-targets -- -D warnings
target/release/ka-hgis.exe --smoke-quit           # 켜졌다 바로 꺼짐, 종료 코드 0 이면 정상
```

- 이 PC 는 빌드를 여러 개 동시에 돌리다 전원이 나간 적이 있다. `-j 3` 을 지킨다.
- GitHub Actions(`.github/workflows/ci.yml`)는 `cargo clippy --workspace`, `cargo test --workspace`, `ka-hgis --verify-all`, GDAL 기능 시험을 돈다.
- 증분 빌드가 꼬여 링크 오류(LLVM `anon.*` 기호 없음, LNK1120)가 나면 `cargo clean -p strata-app` 뒤 다시 빌드한다.
- **화면 확인(사진 실행)**: `ka-hgis.exe --snap <출력 폴더> --snap-steps "<단계들>"`. 단계는 쉼표로 잇고 각 단계는 `@동작:사진이름` 이다. 단계마다 그 이름의 PNG 를 찍는다(이름이 없으면 그 단계는 실행되지 않는다). 동작 목록은 `crates/strata-app/src/snap_shot_steps.rs` 에 있다. 자주 쓰는 것: `@open=<조사.gpkg>`, `@wait`, `@tab_drawing`, `@drop=<파일;파일>`, `@pdf=<경로>`, `@heritage=<보관 폴더;시도;시군>`, `@coord=dx;dy`, `@dem`. **반드시 사용자 조사 파일의 임시 사본으로** 한다. 마우스·키보드 흉내 입력은 쓰지 않는다.

## 4. 코드 지도

| 크레이트 | 하는 일 |
|---|---|
| `strata-app` | 앱 전체(egui 화면, 리본, 지도, 도면, 받기 작업). 진입은 `src/main.rs`, 상태는 `app_state.rs`(필드), 모듈은 파일 하나에 기능 하나 |
| `strata-engine` | GDAL·PROJ 를 쓰는 일: 좌표 변환 `Transformer`, SHP/GPKG 표 읽고 쓰기, DXF 선, DEM, 그림 파일 `picture.rs`, 제출 SHP |
| `strata-geo` | 순수 기하 계산(버퍼, 분할, 붙이기, 시굴격자, 정합 변환 `align_fit.rs`) |
| `strata-storage` | 조사 파일(GPKG) 열기·저장(C++ 과 같은 구조), 최근 조사 |
| `strata-refmap` | 받은 참조 지도(지적, 유산, 지질, 토양 등) 가공 |
| `strata-net`, `strata-webview`, `strata-ngii-run` | 사이트 접속, 숨긴 브라우저(WebView2)로 사이트 화면을 조작하는 주문(수치지형도·DEM) |
| `strata-layout` | 도면 PDF(지금은 그림 PDF) |
| `strata-secrets` | 키·계정 저장(Windows DPAPI) |
| `strata-types`, `strata-canvas2d`, `strata-mapengine`, `strata-topo`, `strata-paleo`, `strata-terrain3d`, `strata-check-export` | 공용 타입, 지도 보기 변환, 라벨, 수치지형도 묶음, 고지형, 3D, 검수·내보내기 |

앱 안의 주요 기능 파일: 받은 지도 `ref_vector*.rs`·`ref_map*.rs`·`cadastral_*.rs`·`topo_*.rs`·`dem_*.rs`·`heritage_*.rs`, 사용자 파일 `user_files.rs`, 그림 레이어 `picture_layers.rs`, 받은 지도 다시 열기 `survey_map_list.rs`, 도면 `gui_tab_drawing*.rs`·`sheet_*.rs`, 편집 `app_state_*.rs`, 정합(아직 껍데기) `gui_ribbon_align*.rs`.

## 5. 일하는 방식 (사용자가 정한 것)

1. **C++ 과 정확히 같게.** 바꾸기 전에 C++ 소스의 해당 줄을 직접 연다. 문구는 C++ 문자열 그대로, 근거는 커밋·주석에 `파일:줄` 로 남긴다. C++ 에 없는 문구를 쓸 때는 「이 앱의 말」이라고 주석에 적는다.
2. **시험 먼저.** 실패하는 시험을 먼저 쓰고 실패를 본 뒤 고친다. C++ 시험(`target/cpp-ref/tests/*.cpp`)이 있으면 옮긴다. 다 된 뒤에는 핵심 줄을 일부러 틀리게 바꿔(변이) 시험이 잡는지 확인한다(스크립트: 원본을 보관했다가 시험 뒤 되돌리는 방식. 중간에 멈추면 원본이 남지 않게 주의).
3. **실제 화면으로 확인.** 화면이 바뀌는 기능은 위의 사진 실행으로 임시 사본에서 찍어 본다.
4. **다 되면 바로 올린다.** 커밋 → `git push origin main` → release 빌드 → 사용자 바탕화면 바로가기가 여는 `target/release/ka-hgis.exe`(원래 저장소 폴더 안)를 새 판으로 바꾼다. 기존 exe 는 지우지 않고 `ka-hgis.old-<시각>.exe` 로 이름만 바꾼다(앱이 열려 있어도 안전). 바꾼 뒤 `--smoke-quit` 확인.
5. **커밋 메시지**: 영어 제목 `type(scope): ...`, 본문에 무엇이 왜 바뀌었는지와 C++ 근거, 시험 결과 줄, 끝에 `Co-Authored-By:`. 기존 시험의 기대값을 바꾸거나 지웠으면 `시험 변경: <이유>` 줄을 반드시 쓴다.
6. **코드 규칙**: 파일당 300줄 이하(넘으면 새 파일), `unsafe` 금지, clippy 경고 0, 새 외부 크레이트는 라이선스(복사 좌파 금지)와 판을 확인하고 커밋에 적는다. 좌표 변환은 반드시 PROJ(`strata_engine::Transformer`)로.
7. **사용자에게는** 진행 보고를 길게 하지 않는다. 물으면 한국어로 짧게 답하고, 사용자만 정할 수 있는 것만 추천 답과 함께 묻는다. 기술 결정은 가장 안전한 쪽으로 하고 기록한다.

## 6. 안전 규칙 (어기면 안 되는 것)

- 키·계정 값(VWorld 키, 국토정보플랫폼 계정, 국가유산 인트라넷 계정, 역사GIS 키)을 출력·파일·커밋·메시지에 넣지 않는다. 저장 위치는 이름만 안다: VWorld 키 `%LOCALAPPDATA%\ka-hgis\ka-hgis-vworld.ini` → `HKCU\Software\ka-hgis` → 환경변수 `VWORLD_API_KEY` → `config/secrets.ini`(git 무시). 계정은 `%LOCALAPPDATA%\ka-hgis\ka-hgis\` 아래(예: `heritage-account.ini`)와 Windows 비밀 저장소. 새 PC 에서는 사용자가 앱의 「더보기」 계정 창에서 직접 넣는다.
- 저장소는 **공개**다. 올리기 전에 커밋 전체에서 키·계정·개인정보·조사 좌표를 훑는다.
- 사용자 원본 조사 자료(조사 GPKG, 받은 자료)를 고치지 않는다. 시험·사진은 임시 사본으로만 한다.
- 실제 사이트에 주문·로그인하는 실행은 사용자 허락 뒤에만 한다. 거절된 로그인은 다시 시도하지 않는다. 「수치지형」 주문 단추를 사진 실행 단계에 넣지 않는다(실제 신청이 들어간다).
- **국가유산 인트라넷 실제 접속·로그인은 아직 실행하지 않았다.** 2026-11-17 사용자는 바탕화면 앱 교체, Git 커밋·푸시와 GitHub Actions 확인을 명시적으로 지시했다. 검증한 빌드를 백업 후 교체한다. 이 배포 지시는 실제 사이트 시험 로그인 허락과 구분한다.

## 7. 지금까지 된 것

판별 결과와 확인 방법은 `docs/RESULTS.md` 에 날짜별로 있다. 2026-11-16 하루 동안 된 것(커밋 순):
좌표점이 땅을 기억함, 도면 목록, 새 용지, Shift 여러 개 고르기·폴리곤 묶기·나누기, Delete 여러 개, 고르기 상자, 화살표 키 오동작 제거, 새 조사·열기 전 저장 질문, 받아 둔 수치지형·DEM 먼저 쓰기, 받은 지도 다시 열면 돌아옴(`<조사 이름>.strata-maps.json`), PDF 진행 창 모달, SHP 끌어 놓기·파일함·「벡터·도면 불러오기」(33b52fb), 그림 레이어(JPG·PNG·TIFF, 월드 파일, 6361e82), 정합 변환 계산(5c04460, 아직 화면에 연결 안 됨).

## 8. 다음 할 일 (우선순위 순)

1. **정합(맞추기) 나머지** — 사용자가 직접 「안 된다」고 한 기능. 설계와 C++ 근거는 `docs/research/align/ALIGN0.md`(끝의 「Port order」 18단계). 된 것: 그림 레이어(Port order 4·8), 변환 계산·품질 문구(1·2, `strata-geo/src/align_fit.rs`). 남은 순서: 월드 파일·`.prj` 쓰기(3) → 정합백업·되돌리기(5) → 그림에 좌표 붙이기(6) → 붙인 그림 표시 방식(7) → 진입과 「맞출 도면」 파일 창(9) → 왼쪽 그림 창과 점 짝 찍기(10) → 표시(번호·화살표·목록, 11) → 끌어 고치기(12) → 이동(13) → 되돌리기·맞추기 저장(14) → 벡터·CAD 정합(15·16, DXF 불러오기 뒤) → 큰 그림 미리보기(17) → 가짜 코드 정리(18: `strata-geo` 의 `helmert_solve.rs`·`georef_quality.rs`·`gcp_solve.rs` 는 C++ 에 없는 이름을 인용한 가짜 이식이고, `gui_ribbon_align*.rs` 의 상태 문구 일부도 가짜다).
2. **유산 인트라넷 받기 후속** — 숨긴 WebView에서 사이트 자신의 `goLogin()`과 다운로드 동작을 호출하는 흐름을 연결했다. 설정 창은 받기 시작 때 닫고, 수치지형·지적도와 같은 아래 게이지와 상태줄에 진행·오류를 표시한다. 실제 사이트 시험은 남았다. C++ 근거는 `docs/research/heritage/HER-NET0.md`와 `docs/research/post/heritage-plate.md`다.
   - 종류별 모든 파일의 완료와 700 ms 간격 여섯 번의 안정 확인 후, 한 `DatasetReady` 묶음을 보낸다. 실제 GIS 적재가 그 정확한 묶음을 승인해야 다음 종류로 간다. `recent.json`과 지도 줄은 승인된 현재 작업에만 반영한다.
   - 망가진 전송·압축 파일은 현재 종류를 유지하고, 숨김 세션을 닫고 프로필을 정리한 뒤 새 세션으로 재로그인한다. 대기는 5·10·20·30초로 늘고 횟수 제한은 없다. 거절된 로그인은 반복하지 않는다. 취소·조사 변경 뒤의 결과는 버린다.
   - 파일은 앱 보관 폴더의 `주변유적/<시도> <시군>/원본/offer-...`에 각각 배정한다. 서약서 원문과 동의 시각은 조사 폴더의 영수증에 기록한다. 유산 세션만 사이트 `confirm`에 동의하며, 다른 브라우저의 기본 정책은 유지한다.
   - 로컬 확인: `cargo test --workspace --quiet` 통과, 앱 837개·net 241개·독립 계약 22개 포함. `cargo run --offline -p strata-heritage-run --example heritage_run`의 실제 WebView 시나리오 11개가 통과했고, 부모 창 표시 상태 97,841회가 모두 숨김이었다. 묶음 GIS 적재는 임시 SHP 시험으로 별도 확인했다. 네이티브 시나리오의 ZIP은 `a.txt` 전송 고정물이므로 GIS 적재 통합 확인으로 세지 않는다.
   - 2026-11-17 REGION2: 실제 사용에서 「시·군·구 칸을 찾지 못했습니다」가 보고됐다. C++ `HeritageIntranetFlow.cpp:272-328`의 `시/군/구선택`, `시·도선택`, `시/도선택` 조건이 Rust 이식에서 빠져 있었다. C++의 두 필드 계열과 원래 표기로 숨김 WebView에서 같은 오류를 재현한 뒤 조건을 복구했다. 시험 고정물도 B/U/R/E의 `bjdCd/bjdCd1`과 세 표기 변형을 사용한다. 이전 11개 통과는 이 표기 누락을 검사하지 못한 좁은 고정물의 결과였고, 실제 사이트 성공의 증거가 아니다.
   - 추가 창 금지 지시 때문에 앱 `--snap`은 실행하지 않았다. 네이티브 시험은 렌더링하지 않는 숨김 winit 부모만 사용한다. 실패했던 팝업 소유 문서와 중첩 로그인도 위 11개에 포함한다.
3. **받은 지도 목록 후속**(검수 지적): 범례 체크 끄기와 투명도 막대가 「저장 안 됨」을 켜지 않음, 이름 바꾸기도 그렇다, 1919 키가 없는 PC 에서 열고 저장하면 그 줄이 목록에서 사라짐, 유산 줄은 아직 목록에 안 남음.
4. **메뉴 감사표의 남은 것** — `docs/research/more/MORE0.md` 끝의 「To build」 목록: 제출 꾸러미·5179 변환(#5), 시굴격자 배치(#6), 등고선(#7), 인쇄 창(#9), CSV 기준점(#10), OSM·Google 배경(#14), 주소 검색이 화면을 멈춤(#18), F9/F11(#19), 표시 설정(#21), 스타일 색 저장(#22).
5. **파일 불러오기 나머지**: 점 SHP(엔진 `ShapeKind` 와 그리기에 점이 없다), GPKG 표, GeoJSON(GDAL `driver_geojson` 을 켜야 함), DXF(CAD 공지 창 → 좌표계 확인; GDAL DXF 드라이버는 이미 있음), DWG(C++ 은 GPL 인 LibreDWG 를 따로 부른다: 쓰지 않는다. GDAL CAD 드라이버는 DWG R2000 만), 이름 글자, 범례의 「레이어 불러오기」 줄(C++ 은 새 빈 레이어 만들기 창).
6. **도면·편집**: 구간 분리, 트렌치 이동 도구, 도면 오른쪽 패널 디자인, 도면 항목 이동의 되돌리기, 표제란 이름 질문, 도면 목록을 조사 파일에 저장(작업공간 2D, 계획 표의 2D 행), 인쇄. 도면 조사는 `docs/research/sheet/SHEET0.md`, 편집은 `docs/research/edit/EDIT0.md`.

## 9. 정해 둔 것 (사용자가 물으면 알릴 것)

- 조사구역 도형이 여러 개면 가장 큰 도형을 지도·도면의 가운데로 쓴다. 대동여지도·지형은 반경 100 km, 나머지 참조 지도는 10 km.
- 지도에서 Delete 는 조사 레이어 자체를 지우지 않는다(범례에서만). 「도형선택」에서 끌면 고르기 상자(가운데·오른쪽 단추로 끌면 지도 이동).
- 시작 화면의 「rust」 는 사용자가 쓴 대로 소문자. 도면 PDF 는 마지막 화면 배율로 그린 300 DPI 그림 PDF(임시, 계획 표 「R3-임시」).
- 받은 지도·타일 지도·사용자 파일을 올리면 「저장 안 됨」이 된다(끌 때 묻는다). 받은 지도 목록은 조사 파일 옆 파일(`<이름>.strata-maps.json`)이다(작업공간 2D 가 생기면 그리로 옮긴다).
- 같은 이름의 파일 두 개는 「이름 (2)」 로 구분한다. 사용자 SHP 는 「조사 데이터」 줄, 그림은 「주변·참조」 줄. 그림은 배경 위·지적선 아래에 그린다.

## 10. 문서 지도

| 문서 | 내용 |
|---|---|
| `docs/RUST_PLAN.md` | 계획 표. 행마다 C++ 근거·통과 조건·라이선스·상태. 새 일은 행을 먼저 쓴다 |
| `docs/RESULTS.md` | 무엇이 달라졌고 어떻게 확인했는지(날짜별), 운영에서 배운 것 |
| `docs/research/` | C++ 기능 조사 보고서(이 PC 의 지시자 폴더에서 옮김): 메뉴 감사 `more/MORE0.md`·`menus/MENU0.md`, 정합 `align/ALIGN0.md`, 유산 `heritage/HER-NET0.md`·`post/heritage-plate.md`, 도면 `sheet/SHEET0.md`, 편집 `edit/EDIT0.md`, 지적 `cadastral/*`, 참조 지도 `post/*`, RustGIS 조사 `rustgis/RG-SURVEY1.md` |
| `AGENTS.md` | 에이전트 공통 규칙(역할, 좌표, 시험, 커밋) |
| `docs/RUST_PARITY_CHECKLIST.md` | C++ 기능 대조표(오래됨: MORE0 이 더 새롭다) |
| `CLAUDE.md`, `docs/HANDOFF.md`, `docs/other-pc-setup.md` | C++ 시절 문서. 제품 규칙만 참고 |

## 11. 다른 PC 에서 처음 할 일

1. Git, VS 2022 Build Tools(C++ 데스크톱), CMake, rustup 설치. `git clone https://github.com/kesekikwon-netizen/hgis-rust`.
2. `cargo build -j 3 -p strata-app`(첫 빌드에 GDAL·PROJ 소스 빌드가 들어 있어 오래 걸린다), `cargo test -j 3 -p strata-app` 이 모두 통과하는지 본다.
3. C++ 근거를 열 수 있게 `target/cpp-ref` 에 gogohak1 을 clone 하고 `b036c09` 로 맞춘다.
4. 사용자에게 키·계정은 앱 화면에서 직접 넣어 달라고 한다(값을 받아 파일에 쓰지 않는다).
5. 이 문서의 8절 1번(정합)부터 이어서 한다. 한 기능이 끝날 때마다 `docs/RESULTS.md` 에 행을 쓰고, 이 문서의 7·8절을 고친다.

### 2026-11-17 작업 지시 변경

- 사용자가 모든 보조 에이전트를 종료하고 한 세션에서 개발하도록 지시했다. 기존 작업은 보존하고 추가 에이전트는 만들지 않는다.
- 개발·검증·문서와 커밋에 관련 pstack 플러그인 스킬을 적용한다. 에이전트 간 기록이 필요하면 영어 YAML을 쓴다.

### 2026-11-17 작은 메뉴 감사 후속

위의 8절은 초기 이식 때의 목록이다. 이후 UI 작업과 이번 메뉴 수정이 반영된 항목은 다음과 같다. 전체 메뉴가 C++와 같다고 판정한 것은 아니다.

- `MENU1`, `1fde558`: 레이어 세트의 이름 지정·덮어쓰기·적용·삭제를 연결했다. 실제 지도 켜짐 상태를 저장하며, 명시적 조사 저장 때 `ka_hgis_layer_sets`에 기록한다. 보관한 도면 전체의 디스크 저장은 아직 없다.
- `MENU2`, `ec8b9ef`: 레이어 우클릭 메뉴의 화면 경계·스크롤·긴 이름과 하위 메뉴 이동을 수정했다. 지도 우클릭 메뉴는 별도 구현이므로 이 결과에 포함하지 않는다.
- `MENU3`, `b0289e4`: DEM·레이어 우클릭 메뉴와 지역 주소창의 Esc가 그리던 점을 지우지 않게 했다. 앱이 직접 소유한 팝업 상태를 휠과 Esc에서 함께 사용한다.
- `MENU4_NATIVE_MENU_ESCAPE`: egui 기본 메뉴 단추에도 Esc 소유권을 연결했다. 캔버스 Esc만 메뉴 렌더 뒤에 처리한다. 대기 동작은 한 렌더 호출 안에서만 살며, 원래 도구와 탭이 바뀌면 적용하지 않는다. 다른 단축키는 기존 시점에 처리하되, Esc와 함께 들어온 Enter는 도형을 확정하지 않는다.
- 기본 메뉴의 열기·닫기·다시 열기, 작은 화면의 레이어 세트 하위 메뉴, 클릭과 Esc의 같은 입력 묶음, 남은 팝업 ID와 정상 캔버스 취소를 실제 egui 입력으로 검사한다. 원본은 쓰지 않고 추적된 데모의 임시 사본을 사용한다. 추가 창 금지 때문에 GPU 창과 `--snap` 대신 창 없는 실제 egui PNG를 확인한다.
- `MENU5_SNAP_TARGET_POPUP_ESCAPE`, 2026-11-18: 그리기 줄의 「모든 조사 레이어」·「현재 레이어」 팝업도 첫 Esc가 그리던 점을 지우지 않게 했다. 실제 팝업 본문이 그려진 경우에만 네이티브 닫기 뒤 Esc를 소비한다. 두 선택값의 유지와 반복 열기를 검사하며, 다른 직접 그린 팝업에는 일괄 적용하지 않는다.
- 다음 감사에는 `ComboBox`, 색 선택기, 속성창의 `popup_below_widget`, 홈 탐색기 우클릭, 다른 키와 메뉴의 상호작용을 포함한다. `menu_button` 검색만으로 전체 팝업을 셀 수 없다. 그리기 도구 줄의 좁은 화면 넘침도 남았다.
- 실제 도형 그리기의 스냅 연결을 별도로 확인해야 한다. `gui_tab_map_capture.rs:40`은 화면 위치를 변환해 `sketch_press`로 넘기고, `app_state_draw_commit.rs:80-94`는 그 값을 저장한다. 대상 선택 필터가 있는 `cad_session.rs` 경로는 이 캡처 경로와 다르다. 팝업 선택값 검사만으로 실제 도형 스냅을 검증했다고 해서는 안 된다.
- 정합의 스캔·사진 표시 규칙, 벡터·CAD 정합, 도면 보관 전체 저장과 C++ QGZ 세트 가져오기는 계속 남은 일이다. 새 GIS 파일 규격이나 코덱은 아직 도입하지 않았다.
