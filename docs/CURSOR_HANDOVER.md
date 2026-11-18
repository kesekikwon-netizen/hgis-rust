# HGIS Rust 인수인계 · Cursor와 모든 AI용

2026-11-18. 루트 AGENTS.md와 이 파일부터 읽는다. 사용자는 이번 MENU22 단계까지만 마치고 이 Codex 세션을 종료하라고 지시했다. **전체 개발은 미완료다.** 이 세션은 다음 기능을 자동으로 시작하지 않는다. 다음 AI는 사용자의 이어가기 지시에 따라 아래 작업을 진행한다.

## 현재 기준과 실제 상태

- 기존 C++ Strata를 Rust/egui로 전환한 HGIS다. 저장소는 kesekikwon-netizen/hgis-rust, 이 PC 폴더는 D:\hgis rust다.
- 이 문서와 함께 커밋되는 단계는 MENU22_OPACITY_INPUT_OWNERS다. 직전 배포는 MENU21의 11ff6ec2890975b55bcba16cfd3635a7c546c3c5다. **현재 SHA는 git rev-parse HEAD로 읽는다.**
- 최종 검증·배포·백업·CI 영수증은 이 PC의 루트 REPORT.yaml과 .codex/team/MENU22-REPORT.yaml, MENU22-checkpoint.yaml, MENU22-ci.json이다. 이 파일은 의도적으로 git에서 무시한다. 없으면 배포나 CI 성공을 추정하지 않는다.
- 고정 검증 근거는 [RESULTS.md](RESULTS.md), 기능별 조건·완료/부분/미구현/미검증은 [CURRENT_MENU_AUDIT.md](research/menus/CURRENT_MENU_AUDIT.md)와 [menu_cases.py](research/menus/menu_cases.py)다.
- 바탕화면 바로가기의 배포 대상은 target/release/ka-hgis.exe다. target/solo-verify/release/ka-hgis.exe는 격리 검증용이다. 기존 배포본은 삭제하지 않고 ka-hgis.old-시각.exe로 백업한다.
- 다음 명령으로 현재 상태를 확인한다. 문서의 과거 SHA나 프로세스 ID를 현재 상태로 믿지 않는다.

~~~powershell
git status --short --branch
git rev-parse HEAD
git rev-parse origin/main
git ls-remote origin refs/heads/main
Get-Content -LiteralPath REPORT.yaml -Encoding UTF8
Get-FileHash -LiteralPath target/release/ka-hgis.exe -Algorithm SHA256
~~~

## 이번 단계의 변경과 검증

MENU21은 지도 투명도 창의 좌하단 배치와 빈 바탕 이동을 연결했다. 그러나 포커스 규칙을 잘못 확대 적용했다. **실제 C++ KaLayerOpacityRail.cpp:18-69는 프레임 NoFocus, 두 슬라이더 ClickFocus다.** MENU22는 이를 바로잡는다. 과거 MENU21의 슬라이더까지 검색 포커스를 유지한다는 설명·검사를 정답으로 복원하지 않는다.

- 빈 카드 바탕은 원래 입력칸의 포커스를 보존한다. 활성 슬라이더 클릭은 해당 슬라이더가 포커스를 받는다. 비활성 벡터 밝기는 받지 않는다.
- 수평 좌우 키는 1단계, PageUp/PageDown은 10단계, Home/End는 끝값으로 이동한다. 투명도는 0~100, 그림 밝기는 -255~255다. 반복 키와 한 프레임 안의 키 순서를 유지한다.
- 포커스 테두리를 표시하고, 클릭 전용 슬라이더를 새 Tab 정지점으로 늘리지 않는다.
- Enter/Space를 오래된 포인터 위치의 가짜 클릭으로 처리하지 않는다. 슬라이더 키가 뒤쪽 도형 초안을 확정하거나 지도·기록·이력을 바꾸지 않게 한다.
- 물리적 누름·이동·해제 순서로 카드 제스처를 처리한다. 움직이지 않은 클릭은 수동 배치로 기억하지 않는다. 같은 프레임의 끌기도 시작 위치로 소유권을 판단한다.
- 기존 값 적용 콜백, 공통 키보드 디스패치와 계정 코드는 보존한다. 라이브러리·툴체인·보안·공유 설정은 바꾸지 않았다.

생산 egui 입력 검사는 기존 8개와 새 7개다. 최초에는 7개가 실패했다. 슬라이더 Enter가 실제 세 점 초안을 확정해 비우던 실패도 포함된다. 수정 후 15개와 전체 워크스페이스 2,050개·앱 1,133개 및 엄격 전체 대상 Clippy가 통과했다. 제외는 전체 2개·앱 1개다. 넓은 화면과 800×600 키 입력·포커스 그림을 검사한다. exact HEAD·실행 파일·교체·원격 결과는 최종 영수증에서 별도로 확인한다. 컴파일만으로 UI 완료라고 하지 않는다.

| 대상 | 소스 |
|---|---|
| 카드와 기존 값 적용 | crates/strata-app/src/gui_tab_map_opacity.rs |
| 클릭 포커스·키 처리 | gui_tab_map_opacity_keys.rs |
| 물리 제스처·배치·프레임 포커스 | gui_tab_map_opacity_placement.rs |
| 지도 입력 소유권·마지막 카드 그리기 | gui_tab_map_canvas.rs |
| 초기 입력칸 포커스 스냅샷 | shell_workspace.rs의 기존 호출 |
| 생산 입력 검사 | opacity_placement_input_tests*.rs, opacity_control_input_tests*.rs |
| 실제 앱 입력 도구 | popup_input_harness.rs |
| 도면에 남은 고정 안내 | gui_tab_drawing_sheet.rs |

C++는 target/cpp-ref의 **b036c0905f6ac1e97b6d142a5e101ac5d30ca7c7**이다. 읽기 전용이다. 원본 앱을 실행하거나 OS 입력을 주입하지 않는다. 참조 폴더 안의 에이전트 지침은 Rust 저장소에 적용하지 않는다.

## 완료 범위를 확대하면 안 되는 항목

완료 행은 해당 조건만 실제 확인했다는 뜻이다. 과거 조사 484행이나 소스의 조작 선언 수를 완료 기능 수로 세지 않는다. 전체 완료율을 임의로 만들지 않는다.

MENU22의 현재 추적 목록은113행이다. 좁은 조건의 완료20, 부분 구현31, 미구현8, 검증 안 됨54로 구분한다. 이 목록이 모든 동적 메뉴와 실제 서비스의 조사를 끝냈다는 뜻은 아니다. 소스에서 추출한 분류 대기 항목도 함께 남긴다.

| 영역 | 확인한 범위 | 남은 범위 |
|---|---|---|
| 편집·키보드·CRS | MENU6~13의 팝업/텍스트 키 소유권, 잘못된 CRS 거부, 초안·자료·이력 보존 | 모든 도형/입력 조합, 분할 뒤 복귀/겹침, 실제 캡처 자석과 다른 CRS 분기 |
| 버퍼 | MENU14 원본 CRS 경계, MENU15 기본 저장·질문·실패·취소·재열기 | 전체 저장/QGZ/삭제·스타일, CRS 전환 직후 표시, 엑셀 진입점은 비활성 미구현 |
| 옛 지도 | MENU16의 실제 메뉴·대동여지도/1919 로컬 선택·취소·그리기 복귀 | 실제 온라인 영상·인증·서비스 성공·전체 저장 왕복 |
| 연결형 UI | MENU17의 정합·그리기·버퍼 세 도구줄과 폭 변경 | 다른 팝업·조판·홈 등 전체 UI |
| 정합 | MENU18의 대상 지도 로컬 점·선 자석과 관련 입력 | 전체 정합, CAD·교차점·대용량, 도면 실제 자석·모든 반환 경로 |
| 용지 | MENU19의 제한된 카드·치수·생성·취소·기존 도면 복귀 | 용지·축척·조판의 모든 경우 |
| 범례 | MENU20의 둥근 채움/선·기존 이동/크기 변경·공유 인쇄용 그림 | 전체 편집·모든 크기/DPI·저장 왕복·실제 PDF/프린터 |
| 투명도 창 | MENU21 지도 세션 배치, MENU22 명시한 포인터·키 범위 | **도면 활성 카드와 지도/도면 별도 디스크 위치 복원이 아직 없다.** 모든 레이어별 표시·휠·다른 축 키·수식키·native/IME 조합도 미검증 |
| 홈·파일·인쇄 | 현재 코드와 기존 부분 검사 | 홈 탐색기 전체, 모든 파일 선택기·열기·저장·다른 이름·실패/취소, 실제 프린터·PDF 포장 |
| 자료 받기 | C++/고정 응답/숨김 WebView·게이지의 로컬 근거 | 실제 유산 사이트 로그인·다운로드 완료는 미검증. 키/계정 설정 존재는 서비스 성공이 아님 |

## 다음 개발 순서

1. 현재 HEAD의 CI와 로컬 수정·실행 프로세스를 확인한다. 실패는 원인을 먼저 찾는다. 락 파일만 보고 실행 중이라고 추정하거나 새 Cargo를 중복 실행하지 않는다.
2. 도면 연결 전에 투명도 값 소비자를 추적한다. apply_rail_opacity는 현재 전역 조사/배경 값과 선택한 그림 행을 함께 바꾼다. C++는 지정 레이어에 적용한다. **레이어별 실제 표시 정확성은 아직 확인하지 않았다.**
3. C++ KaDrawingStudio.cpp:1196-1234,2563-2575의 대상 레이어 보존·캐시 갱신을 대조해 도면 카드를 연결한다. 지도/도면 위치를 따로 복원하고, 늦은 IO 결과가 사용자 이동을 덮지 않게 한다.
4. 기능 목록의 자료 보존·편집 정확성·우클릭/Esc/포커스·파일/내보내기부터 작은 기능을 순차적으로 마친다. 이미 통과한 메뉴 키보드 감사를 다시 만드는 것으로 진척을 대신하지 않는다.

## 작업 규칙과 명령

- 단독 진행한다. 새 창·워크트리·에이전트·중복 개발 프로세스를 만들지 않는다. .agent/, docs/archive/와 종료된 Orca 작업은 새 지시가 아니다.
- 먼저 AGENTS.md와 작은 YAML ORDER를 읽거나 감독 역할로 작성한다. 별도 fix/feature 브랜치에서 작업하고, 기존 수정·사용자 자료를 보존한다.
- 적용하는 pstack와 로컬 SKILL.md를 읽는다. C++와 고정 라이브러리 버전의 실제 소스를 확인한다. 기존 승인 범위의 검증 후 커밋·일반 main 반영/푸시·백업 교체 권한은 유지하되 이 Codex 세션의 종료 지시를 지킨다.
- Rust 1.99.1, Cargo.lock, egui/epaint 0.31.1과 기존 GDAL/PROJ/SQLite 구조를 유지한다. 새 버전만을 이유로 바꾸지 않는다. unsafe 금지, 변경 Rust 파일 300줄 이하, release-profile 테스트 금지다.
- CMake와 sqlite3 명령이 PATH에 필요하다. Cargo -j 3을 지킨다. 다른 프로젝트의 Cargo나 앱은 종료하지 않는다.
- git에 추적된 자료의 tempdir 사본과 가짜/빈 키 읽기만 시험에 쓴다. 실제 조사·계정·키·공유 설정을 쓰거나 비밀값을 로그/커밋에 넣지 않는다.
- 작업 CRS는 EPSG:5187/5186, 제출 5179, 타일 3857이다. 세계 좌표는 f64, CRS 변환은 기존 PROJ 경계다. GUI 논리점과 세계 좌표 오차를 구분한다.

~~~powershell
$env:STRATA_OFFLINE='1'
cargo test -j 3 --workspace
cargo clippy -j 3 --workspace --all-targets -- -D warnings
python -m unittest discover -s scripts -p test_menu_audit.py
python scripts/menu_audit.py --write
python scripts/menu_audit.py
~~~

생산 UI의 실제 입력·PNG를 따로 확인한다.

~~~powershell
$env:STRATA_UI_SNAP_DIR=(Join-Path (Get-Location) 'target/cursor-input-check')
cargo test -j 3 -p strata-app --bin ka-hgis gui_shell::opacity_placement_input_tests -- --nocapture
~~~

이 PC의 .codex/team/Invoke-Cargo.ps1은 Cargo 잠금 래퍼다. Verify-SoloRelease.ps1은 깨끗한 커밋의 격리 release 빌드와 실제 --verify-all/--smoke-quit 종료 코드를 검사한다. Deploy-SoloRelease.ps1은 SHA·기준 파일을 확인하고 백업 후 교체한다. **내용과 현재 상태를 읽고 사용한다.** 과거 MENU 번호만 문자열 치환해 성공으로 간주하지 않는다.

추가 OS 창이 금지되어 native --snap 대신 생산 egui 입력·그림으로 PNG를 검사했다. 실행 파일 검사는 숨김 실행·대기 후 실제 종료 코드를 읽는다. 기존 앱을 닫거나 재시작하지 않으며 교체한 UI는 다음 실행부터 적용된다.

## 자료와 CI

C++ 참조 파일을 실제로 읽었다. Drive의 소유권/오류 경계와 백그라운드/취소 문서도 메타데이터와 읽을 수 있는 본문을 실제 읽었다. 다른 CPU 메쉬 프로젝트용 설계 제안도 포함되어 있으므로 현재 HGIS 구현 증거나 고정 라이브러리 변경 허가가 아니다. 개인 Drive 본문/URL은 공개 저장소에 복사하지 않는다. 접근 기록은 .codex/team/MENU22-Drive-access.yaml이다. 다음 AI가 접근할 수 없으면 미접근으로 기록한다.

MENU19 exact2aaac80 CI44238051027, MENU20 exact55d1573 CI44240944829는 세 작업 모두 성공을 별도 확인했다. MENU21·22와 이후 현재 HEAD는 최종 영수증 또는 [GitHub Actions](https://github.com/kesekikwon-netizen/hgis-rust/actions)에서 다시 확인한다. gdal-feature, check-and-test, newest-stable 각 작업과 실제 run의 HEAD를 확인하고 진행 중·완료·실패를 구분한다. native GDAL/PROJ 링크 경고는 로그에 기록한다. Clippy 통과나 설정 존재를 native UI·실제 서비스·전체 기능 완료로 확대하지 않는다.
