# DeepSeek Harness 단독 완주용 C++ → Rust 1:1 이식 마스터 명세서 (`Task D19` ~ `Task D25`)

이 문서는 최신 GitHub C++ GIS 정본 브랜치(`kyi/main`, 로컬 체크아웃: `target/cpp-ref/src/app/` 및 `target/cpp-ref/src/core/`)와 현재 Rust 워크스페이스(`crates/strata-app/src/`, `crates/strata-*/src/`)를 전수 대조하여, **남아 있는 모든 미완성·차이점 과제를 DeepSeek Harness가 혼자서 끝까지 순서대로 완주할 수 있도록 정리한 마스터 작업 지시서**다.

---

## 0. 전 과제 공통 절대 원칙 (절대 위반 금지)

1. **최신 GitHub `kyi/main` (`target/cpp-ref/src/`) C++ GIS 앱 정본 기준**:
   - 예전 작업 흔적이나 임시 모사(껍데기 버튼, 고정 텍스트 상자, 가짜 순환 버튼)를 모두 제거하고, `target/cpp-ref/src/app/` 및 `target/cpp-ref/src/core/`의 최신 코드와 **같은 위치, 같은 디자인, 같은 창 크기, 같은 글자 크기, 같은 툴팁/설명, 같은 사용 방법, 같은 실제 기능**으로 1:1 구현한다.
2. **모든 모니터(FHD / 21:9·32:9 울트라와이드 / 4K HiDPI) 및 모든 그래픽카드(내장·외장 GPU) 호환**:
   - 고정 픽셀(`exact_width`, 화면 밖으로 나가는 고정 `fixed_size`)로 인해 와이드 모니터나 4K 배율(125%·150%·200%)에서 잘리거나 겹치는 현상이 없도록 한다.
   - 모든 대화상자 크기는 `ctx.screen_rect()` 범위 내로 클램프(`min(vis_w - 32.0)`, `min(vis_h - 32.0)`)하고, 좌·우 패널은 C++ `QSplitter` / `minimumWidth`·`maximumWidth` 규약을 따른다.
   - `wgpu` / `eframe` 렌더러는 특정 외장 그래픽카드에 종속되지 않도록 기본/폴백 어댑터와 소프트웨어 래스터 폴백이 항상 동작해야 한다.
3. **Rust 안전성 및 크로스 OS 규칙**:
   - `#![forbid(unsafe_code)]` 유지, 프로덕션 코드 내 `unwrap()` / `expect()` 금지.
   - 파일 줄바꿈은 반드시 `LF` (`\n`)만 사용하고, 경로 표기는 상대 슬래시 경로(`crates/strata-app/src/...`)만 사용한다.
4. **매 Task 완료 시 필수 검증 및 비교 절차**:
   - C++ 실물 창(`build\Release\ka-hgis.exe`)과 Rust 실물 창(`target\debug\ka-hgis.exe`)을 나란히 띄워 눈으로 비교:
     ```powershell
     powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:home"
     powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:map"
     powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:drawing"
     powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:paleo"
     powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:terrain3d"
     ```
   - 필수 검증 게이트 3종 통과 후 커밋 및 푸시:
     ```powershell
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace --no-fail-fast
     cargo run -p strata-app --bin ka-hgis -- --verify-all
     git add -A
     git commit -m "feat(...): ... (Task DXX)"
     git push origin main
     ```

---

## 현재까지 완료된 구간 요약 (`U1` ~ `D18`, 커밋 `37e2132`까지 완료)
- **완료됨**:
  - 상단 리본 바 24개 버튼 배치·크기·2줄 줄바꿈·우클릭 컨텍스트 메뉴 3종(`고지형` 7항목, `지질` 6항목, `DEM` 5항목) (`gui_ribbon_bar.rs`)
  - 홈 화면(`KaStartPage`), 하단 상태줄(`KaStatusBar` 13단계 축척·직접 입력·지도갱신 토글·좌표계·격자 2줄 컨트롤) (`gui_tab_home.rs`, `gui_shell.rs`)
  - 지도 탭 상단 그리기 안내 띠(`KaDrawGuideBand` 실시간 길이·면적), 자석(`KaSnapSettingsWidget`), 거리·면적 측정 HUD, 도면 정합(`KaAlignMapTool`), 좌상단 투명도·밝기 레일(`KaLayerOpacityRail`), 우하단 축척자(`KaMapScaleBar`), 우상단 빠른 배경지도 카드(`KaBasemapQuickCard`) (`gui_tab_map.rs`)
  - 모달 대화상자: 선 색·굵기(`KaLineColorDialog`), 조사구역 속성(`KaSurveyAreaDialog` WCAG 대비율), 검수·제출(`KaSubmitDialog` 8종 바로 고치기 버튼), 참조 지도 현황(`KaReferenceStatusDialog`), 고지형 자료 받기(`MainWindowPaleo::openPaleoSourceDialog` 3반경 콤보·버튼 순서), 고지형 6대 입력 자료 초록/빨강 배지 (`dialogs_modal.rs`, `gui_tab_paleo.rs`)

---

## 앞으로 개발해야 할 남은 과제 상세 목록 (`Task D19` ~ `Task D25`)

---

### [Task D19] `KaTrenchDialog`(시굴격자 속성) 하단 버튼·개별편집·사면안내 + `KaDrawingStudio`(도면 스튜디오) 투명도 레일·SpinBox·7버튼 툴팁 1:1 완성
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/KaTrenchDialog.cpp` (Lines 27–265)
  - `target/cpp-ref/src/app/KaTrenchDialogPlan.cpp` (Lines 27–235)
  - `target/cpp-ref/src/app/KaDrawingStudio.cpp` (Lines 888–1336)
- **Rust 대상 파일**:
  - `crates/strata-app/src/dialogs_download_print.rs` (`ActiveModalDialog::TrenchGenerator`, Lines 217–553)
  - `crates/strata-app/src/gui_tab_drawing.rs` (`render_drawing_tab_ui`, Lines 13–1219)
- **상세 구현 내용**:
  1. **`ActiveModalDialog::TrenchGenerator` (`dialogs_download_print.rs`)**:
     - `"지금 그린(또는 선택한) 조사구역에 자동 배치"` 체크박스 툴팁:
       `"조사구역이 여러 개면 선택한 곳만, 선택이 없으면 마지막에 그린 곳만 깝니다."` (`KaTrenchDialog.cpp:198-199`)
     - `"조사 종류"` 콤보/버튼 툴팁:
       `"시굴은 조사구역의 10%, 표본은 2%로 배치합니다. 폭 2 m·길이 20 m 이내에서 길이와 둑 간격을 맞추며, 모든 격자가 구역 안에 들어갑니다."` (`KaTrenchDialog.cpp:44-46`)
     - `"지형 사면 방향으로 넣기(등고선 직교)"` 체크박스 툴팁:
       `"트렌치 장축을 오르막 쪽으로 돌립니다. 등고선과 나란히 넣으면 같은 층만 따라가 층서를 못 읽습니다. DEM을 올려야 씁니다."` (`KaTrenchDialog.cpp:50-52`)
     - 사면 안내 라벨 (`m_terrainInfo`, `KaTrenchDialog.cpp:54-57, 227-237`):
       - `trench_slope_align == true`일 때: `"지형: 오르막 135° · 경사 4.2% — 트렌치 장축을 이 방향으로 넣습니다."`
       - `trench_slope_align == false`일 때: `"지형 자료가 없어 남북으로 깝니다(DEM·수치지형도 등고선 없음). 방위 칸에 다른 값을 넣으면 그 방향으로 깝니다."`
     - `"모양"` 5종(`곧은 트렌치`, `ㄴ자 트렌치 (15 m + 15 m)`, `ㄱ자 트렌치 (15 m + 15 m)`, `ㄴ자 좌우 반대 트렌치 (15 m + 15 m)`, `ㄱ자 좌우 반대 트렌치 (15 m + 15 m)`) 툴팁:
       `"ㄱ·ㄴ자는 팔 15 m 그대로 구역 안에 온전히 들어가는 것만 놓아, 비율이 목표를 조금 넘습니다."` (`KaTrenchDialogPlan.cpp:35`)
     - `"트렌치 길이"` 라디오(`20 m`, `15 m`, `10 m`) 툴팁:
       `"폭 2 m · 길이 {metres} m 트렌치로만 조사구역 안에 깝니다."` (`KaTrenchDialogPlan.cpp:47`)
     - `"좌표격자에 맞추기"` 체크박스 툴팁:
       `"ㄱ·ㄴ자 시굴·표본에서 꺾인 바깥 모서리를 좌표격자 20 m 교점에, 방향을 좌표격자 방향에 맞춥니다. 트렌치끼리는 1 m 이상 띄웁니다."` (`KaTrenchDialogPlan.cpp:96-97`)
     - `"둑(간격)"` 툴팁: `"트렌치 사이 간격입니다. 간격을 키우면 시굴 비율이 내려갑니다."` (`KaTrenchDialog.cpp:66`)
     - `"이름 prefix"` 라벨을 C++ 원문 **`"이름 접두"`**(`KaTrenchDialog.cpp:101`)로 수정.
     - `"회전(방위)"` 툴팁: `"격자 회전(북 기준 시계 방향). 「구역에 깔기」를 누르면 회전된 격자로 다시 배치합니다."` (`KaTrenchDialog.cpp:84`)
     - 격자 배치 후 편집 줄 (`m_afterPlace`, `KaTrenchDialog.cpp:114-135`):
       - 안내문: `"깐 격자는 맵에서 끌어 옮깁니다. 개별 편집은 트렌치 하나 이동·삭제입니다."`
       - 버튼 2개: `"개별 편집"` (툴팁 `"그래픽처럼 트렌치를 하나씩 선택·이동·삭제합니다."`), `"전체 이동"` (툴팁 `"격자를 끌어다 놓아 전체를 옮깁니다. Ctrl+Z로 되돌립니다."`).
     - 하단 실행 버튼 줄 (`btnRow`, `KaTrenchDialog.cpp:137-154`):
       - 기본 강조 버튼 `"구역에 깔기"` (툴팁 `"조사구역 위에 격자를 다시 깝니다. 깐 뒤 맵에서 끌어 옮기세요."`)
       - 보조 버튼 `"맵에 찍기"` (툴팁 `"맵을 한 번 찍어 그 점을 원점으로 놓습니다."`)
       - 우측 정렬 버튼 `"닫기"`.
  2. **`KaDrawingStudio` (`gui_tab_drawing.rs`)**:
     - 도면 캔버스 좌상단의 고정 사각형(`"투명도·밝기: 조절할 레이어를 고르세요"`, Lines 331–349)을 제거하고, 실제 **`self.render_layer_opacity_rail(ui, rect);`** 호출로 교체하여 도면 탭에서도 레이어 선택 시 투명도(`0~100%`)·밝기(`-100~+100`) 슬라이더가 실제 동작하게 한다 (`KaDrawingStudio.cpp:1190-1225`).
     - 우측 패널(`ka_drawing_right_panel`)을 `.resizable(true).default_width(280.0).width_range(260.0..=360.0)`로 변경 (`KaDrawingStudio.cpp:881, 1247-1248`).
     - 우측 패널의 가짜 순환 버튼 3개를 C++ `QSpinBox` / `QDoubleSpinBox` (`egui::DragValue`)로 교체:
       - `"글자"`: `range(5..=24)`, `suffix(" pt")` (`KaDrawingStudio.cpp:953-956`)
       - `"크기"` (방위): `range(3.0..=80.0)`, `fixed_decimals(0)`, `suffix(" mm")` (`KaDrawingStudio.cpp:1018-1022`)
       - `"1 :"`: `range(10..=5_000_000)` (`KaDrawingStudio.cpp:1055-1060`)
       - `"선택 항목 크기"`: `range(25..=200)`, `suffix(" %")`, 툴팁 `"용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다."` (`KaDrawingStudio.cpp:1137-1146`)
     - `"≡  범례"`, `"⊞  표제란"`, `"1:5,000 이하에서 유적 이름으로 표시"`, `"PDF 내보내기"`, `"인쇄"` 및 하단 7버튼(`좌표점`, `레이어를 가운데`, `지도 조정`, `화면 맞춤`, `항목 이동`, `용지/방향`, `도면 목록`)에 `KaDrawingStudio.cpp:906-1323`의 정확한 한국어 툴팁을 부여한다.

---

### [Task D20] 도면 스튜디오(`KaDrawingStudio`) 심화 기능 4종 (`지도 조정` 상단 바 · `좌표점` 콜아웃 · `도면 목록` 보관/불러오기 메뉴 · `세트 ▾` 레이어 테마 메뉴) 1:1 구현
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/KaDrawingStudio.cpp` (Lines 690–703, 822–838, 1292–1335, 1399–1438)
  - `target/cpp-ref/src/app/KaDrawingCoordCallout.cpp` (Lines 1–180)
  - `target/cpp-ref/src/app/KaDrawingSheetSet.cpp` (Lines 1–160)
  - `target/cpp-ref/src/app/KaDrawingLayerSets.cpp` (Lines 1–160)
- **Rust 대상 파일**:
  - `crates/strata-app/src/gui_tab_drawing.rs`
  - `crates/strata-app/src/tab_drawing.rs`
  - `crates/strata-app/src/gui_layer_panel.rs`
- **상세 구현 내용**:
  1. **지도 조정 상단 안내 바 (`m_adjustBar`, `KaDrawingStudio.cpp:822-838`)**:
     - 하단 7버튼 중 `"지도 조정"` 클릭 시 토글 상태(`self.drawing.map_adjust_active: bool`)를 켠다.
     - 켜져 있을 때 도면 캔버스 상단에 C++ `mapAdjustBar` 프레임을 표시한다:
       - 문구: `"지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」."`
       - 우측 버튼 2개: `"레이어를 가운데"` (클릭 시 조사구역 중심으로 도면 이동), `"조정끝"` (클릭 시 `map_adjust_active = false`).
     - `map_adjust_active == true`일 때 용지 위에서 마우스 드래그 시 도면 중심 이동, 마우스 휠 시 도면 축척(`scale_denominator`) 확대·축소가 실제로 동작해야 한다!
  2. **용지 위 좌표점 콜아웃 (`KaDrawingCoordCallout.cpp`, `KaDrawingStudio.cpp:1292-1294, 1399-1438`)**:
     - 하단 7버튼 중 `"좌표점"` 클릭 시 `self.drawing.coord_callout_mode: bool` 토글을 켜고, 용지 위를 클릭하면 해당 지점의 작업 좌표(`X(N), Y(E)`)를 계산하여 용지 위에 `A`, `B`, `C`... 태그와 지시선 화살표 + 좌표 상자(`N 359,xxx.xx / E 219,xxx.xx`)를 실제로 그려 넣는다 (`self.drawing.coord_callouts: Vec<CoordCallout>`). 우클릭 또는 `Ctrl+Z`로 마지막 좌표점을 되돌린다.
  3. **하단 `"도면 목록"` 팝업 메뉴 (`KaDrawingSheetSet.cpp`, `KaDrawingStudio.cpp:1315-1333`)**:
     - `"도면 목록"` 버튼 클릭 시 단순 상태 메시지가 아니라 C++ `KaDrawingSheetSet::fillMenu`와 동일한 팝업 메뉴를 띄운다:
       - `"지금 도면을 목록에 보관…"` (현재 축척·용지·범례·표제란 상태를 이름 붙여 `self.drawing.stored_sheets`에 저장)
       - 보관된 도면 목록 (클릭 시 해당 도면 설정 복원, 우측 `[삭제]` 버튼으로 보관본 삭제).
  4. **좌측 레이어 카드 `"세트 ▾"` 팝업 메뉴 (`KaDrawingLayerSets.cpp`, `KaDrawingStudio.cpp:690-703`, `gui_layer_panel.rs:250-276`)**:
     - 도면 탭 좌측 레이어 헤더의 `"세트 ▼"` 클릭 시 팝업 메뉴를 띄워 `"지금 레이어 구성을 세트로 저장…"` 및 프리셋/저장된 세트(`시굴격자 배치도`, `유구 실측도`, `지형·등고 분석도`, `지적·주변유산 대조도`)를 선택·적용·삭제할 수 있게 한다.

---

### [Task D21] 지도 캔버스 우클릭 메뉴(`populateMapContextMenu`) 및 왼쪽 레이어 트리 우클릭 메뉴(`showLayerTreeContextMenu`) C++ 1:1 구현
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/MainWindowContextMenus.cpp` (Lines 112–481)
- **Rust 대상 파일**:
  - `crates/strata-app/src/gui_tab_map.rs`
  - `crates/strata-app/src/gui_layer_panel.rs`
- **상세 구현 내용**:
  1. **지도 캔버스 우클릭 컨텍스트 메뉴 (`MainWindowContextMenus.cpp:112-161`)**:
     - 지도 탭 캔버스에서 우클릭(그리기·측거 도구가 아닐 때) 시 C++ `mapContextMenu` 팝업을 띄운다:
       - 도형 위에서 우클릭 시: `"이 도형 기록 입력"` (클릭 시 우측 속성 카드 포커스 또는 속성 입력 창 열기)
       - 서브메뉴 또는 항목 `"조사 도형 그리기"` ▸ `"조사구역"`, `"유구 면"`, `"유구 선"`, `"단면선 그리기"`
       - 구분선
       - `"이전 지도 범위"` / `"다음 지도 범위"` (뷰포트 히스토리 스택 연동)
       - `"선택한 레이어로 이동"` (`fit_to_survey` 또는 선택 레이어 범위로 이동)
       - `"전체 지도 보기"` (`zoom_to_korea_or_survey`)
       - `"지도 새로고침"`
       - 구분선
       - `"이 위치 좌표 복사"` (툴팁 및 클립보드 복사 텍스트: `"{N:.3}, {E:.3} ({EPSG})"`).
  2. **왼쪽 레이어 트리 행 우클릭 컨텍스트 메뉴 (`MainWindowContextMenus.cpp:176-448`)**:
     - `gui_layer_panel.rs`의 각 레이어 행을 우클릭하면 C++ `layerContextMenu` 팝업을 띄운다:
       - `"레이어 불러오기 (SHP·DXF·GPKG)"`
       - 구분선
       - `"이름 바꾸기 (두 번 클릭)"`
       - `"면·외곽선 색"` → 클릭 시 `ActiveModalDialog::LineColorAndWidth` (조사구역이면 `SurveyAreaProps`) 오픈
       - `"도형 속성"`
       - `"라벨·명칭 속성"` ▸ `"보이기/숨기기"`, `"면적(㎡) 함께 표시"`, `"글자 크기"` (`3, 4, 5, 6, 7, 8, 9, 10, 12, 14 pt`)
       - `"깨진 한글 바로잡기"` (`CP949`, `EUC-KR`, `UTF-8`, `이 컴퓨터 설정`)
       - `"투명도"` (`0% 투명하게`, `20% 투명하게`, `40% 투명하게`, `50% 투명하게`, `60% 투명하게`, `80% 투명하게`)
       - 구분선
       - `"이 레이어로 이동"`, `"전체 보기"`, `"같은 묶음의 맨 아래로"`
       - 구분선 (편집 가능 벡터/면 레이어인 경우):
         - `"그리기·편집 시작"`, `"꼭짓점 수정"`, `"폴리곤 묶기"`, `"면적 확인"` (시굴격자는 `"면적·조사 비율 확인"` — 클릭 시 면적·비율 요약 알림 표시)
         - 조사구역(`조사구역`)인 경우 `"시굴격자"` 서브메뉴: `"시굴 · 조사구역의 10%"`, `"표본 · 조사구역의 2%"`, `"규격 지정·격자 만들기…"` (`ActiveModalDialog::TrenchGenerator`)
       - 배경 타일 레이어인 경우: `"오프라인 저장 — 지금 화면 범위"` (`ActiveModalDialog::OfflineTileSave`)
       - 조사 레이어인 경우: `"제출 꾸러미 만들기…"` (`ActiveModalDialog::ExportSubmitPackage`)
       - 구분선
       - `"그린 도형 모두 지우기…"`, `"레이어 삭제"`.
     - 레이어 행을 **더블클릭**하면 C++ `KaDrawingStudio.cpp:738-744` / `MainWindow`와 동일하게 `"면·외곽선 색"` (`LineColorAndWidth` 또는 `SurveyAreaProps`) 대화상자를 즉시 연다.

---

### [Task D22] 우측 도형 속성 카드(`KaFeatureCard` / `KaInspectorPanel`) 및 `KaFeatureFormDialog` · `KaControlPointDialog` · `KaControlCsvBox` C++ 1:1 구현
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/KaFeatureCard.cpp`, `KaFeatureCardEdit.cpp`, `KaFeatureCardEditors.cpp`, `KaFeatureCardPlate.cpp`
  - `target/cpp-ref/src/app/KaInspectorPanel.cpp`, `KaInspectorStyle.cpp`
  - `target/cpp-ref/src/app/KaFeatureFormDialog.cpp`
  - `target/cpp-ref/src/app/KaControlPointDialog.cpp`, `KaControlCsvBox.cpp`
- **Rust 대상 파일**:
  - `crates/strata-app/src/right_inspector.rs`
  - `crates/strata-app/src/gui_tab_map.rs`
  - `crates/strata-app/src/dialogs_modal.rs`
- **상세 구현 내용**:
  1. **`KaFeatureCard` (`KaFeatureCard.cpp`, `KaFeatureCardEditors.cpp` → `gui_tab_map.rs` / `right_inspector.rs`)**:
     - 지도에서 유구·조사구역·시굴격자·기준점·단면선을 선택했을 때 우측 속성 패널에 표시되는 편집 필드를 C++ `KaFeatureCard`와 1:1로 맞춘다:
       - 유구(`feature_poly` / `feature_line` / `artifact_point`): `유구 번호`(`feature_no`, 자동 채번 버튼 포함), `종류`(`kind` 프리셋 칩: 주거지·수혈·구상유구·주공·석곽묘·토광묘·가마·건물지·기타), `시대`(`period` 프리셋 칩: 구석기·신석기·청동기·초기철기·원삼국·삼국·통일신라·고려·조선·시대미상), `이름`(`name`), `비고`(`note`), 읽기 전용 `면적(㎡)` 또는 `길이(m)`.
       - 조사구역(`survey_area`): `조사명`(`survey_name`), `유적명`(`site_name`), `비고`(`note`), `면적(㎡)` 및 `"시굴격자 채우기…"` 버튼.
       - 기준점(`control_points`): `기준점 번호`(`point_id`), `X(북)` / `Y(동)` / `Z(표고)` 수치 편집 및 `"CSV 불러오기/내보내기…"` (`KaControlCsvBox.cpp`).
       - 단면선(`section_line`): `단면 번호`(`section_id`, 예: `A-A'`), `비고`(`note`), 단면 지형 프로파일 미니 그림(`KaElevationFigure.cpp`).
     - 하단 액션 버튼: `"저장 (Enter)"`, `"되돌리기"`, `"도형 삭제 (Del)"`, `"면·선 색상…"`.

---

### [Task D23] `KaTerrain3dStudio`(입체지형 탭) + `KaTerrain3dLayoutStudio`(입체지형 도면출력 뷰) C++ 1:1 정밀 일치
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/KaTerrain3dStudio.cpp` (Lines 78–548)
  - `target/cpp-ref/src/app/KaTerrain3dView.cpp` (Lines 1–260)
  - `target/cpp-ref/src/app/KaTerrain3dLayoutStudio.cpp` (Lines 72–636)
  - `target/cpp-ref/src/core/Terrain3dService.cpp`, `Terrain3dLayoutService.cpp`
- **Rust 대상 파일**:
  - `crates/strata-app/src/gui_tab_terrain3d.rs`
- **상세 구현 내용**:
  1. **`KaTerrain3dStudio` 좌측 패널 (`KaTerrain3dStudio.cpp:85-145`)**:
     - 패널 폭: 반응형 `minimumWidth(200)` ~ `maximumWidth(260)` (`.resizable(true).default_width(230.0).width_range(200.0..=260.0)`).
     - `"높이 과장"` 컨트롤: C++는 `QDoubleSpinBox` (`range 0.2..=12.0`, `step 0.1`, `value 2.0`, `decimals 1`, `suffix " 배"`, `KaTerrain3dStudio.cpp:113-118`)다. 현재 `gui_tab_terrain3d.rs:77`의 슬라이더 옆에 `egui::DragValue` (`0.2..=12.0`, `speed(0.1)`, `fixed_decimals(1)`, `suffix(" 배")`)를 함께 배치하여 직접 수치 입력과 드래그가 모두 되게 한다.
     - 텍스처 드레이핑 모드 선택: 위성영상(`입체지형 위성`), 음영기복(`Hillshade`), 고지형 7대 분류도 중 선택하여 3D 표면에 실시간 반영되도록 한다 (`KaTerrain3dStudio.cpp:190-248`).
  2. **`"입체지형 도면출력"` (`KaTerrain3dLayoutStudio.cpp:72-320`)**:
     - `"입체지형 도면출력"` 버튼 클릭 시 단순 탭 전환만 하는 것이 아니라, 현재 3D 입체지형 렌더링 뷰를 도면 시트(`placeTerrain3dPicture`, `KaDrawingStudio.cpp:1361-1397`)에 올리거나 입체지형 전용 조판 모드(`입체지형 도면 — 범례는 「범례」로`, 좌측 레이어 + 중앙 3D 조판 시트 + 우측 `도면 항목`/`방위`/`높이 범례`/`축척자` 컨트롤)를 활성화하여 3D 뷰가 도면 용지 안에 들어간 상태로 PDF 출력·인쇄가 되도록 연결한다.

---

### [Task D24] 자료 받기 브라우저 4종(`KaTopographicBrowser` · `KaTopographicImportDialog` · `KaHeritageBrowser` · `CadastralBrowser`) 및 계정 설정(`KaTopographicAccountDialog` · `KaApiKeyDialog` · `KaHeritageSetupDialogs`) 세밀 마감
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/KaTopographicBrowser.cpp` (Lines 117–420)
  - `target/cpp-ref/src/app/KaTopographicImportDialog.cpp` (Lines 260–540)
  - `target/cpp-ref/src/app/KaTopographicScopePanel.cpp` (Lines 380–632)
  - `target/cpp-ref/src/app/KaHeritageBrowser.cpp` (Lines 203–460)
  - `target/cpp-ref/src/app/KaTopographicAccountDialog.cpp`, `KaApiKeyDialog.cpp`, `KaHeritageSetupDialogs.cpp`
  - `target/cpp-ref/src/app/KaStageRail.cpp`
- **Rust 대상 파일**:
  - `crates/strata-app/src/dialogs_download_print.rs`
  - `crates/strata-app/src/dialogs_modal.rs`
  - `crates/strata-app/src/dialogs_account.rs`
- **상세 구현 내용**:
  1. **`KaStageRail` 6단계 시각 레일 (`KaStageRail.cpp`, `KaTopographicBrowser.cpp:176-180`)**:
     - `render_topographic_browser`(`dialogs_download_print.rs:2419`) 및 `DemDownload`, `CadastralDownload`에서 단순 텍스트 화살표 대신 C++ `KaStageRail`처럼 6단계 원형 배지 + 연결선(완료된 단계는 테라코타 채움 `●`, 현재 단계는 강조 테두리 `◉`, 대기 단계는 회색 `○`)을 그린다.
     - 하단 기본 줄에 `"받은 자료 불러오기…"` 폴백 버튼(툴팁: `"공식 화면에서 직접 받은 DXF·ZIP 폴더를 골라 지도에 올립니다. 자동 신청은 다시 하지 않습니다."`, `KaTopographicBrowser.cpp:213-217`)을 함께 노출한다.
  2. **`KaHeritageBrowser` (`dialogs_download_print.rs:1406-1895`)**:
     - `"상세 보기"` 버튼 툴팁 `"공식 사이트 화면을 펼쳐 봅니다. 막혔을 때만 쓰면 됩니다."` (`KaHeritageBrowser.cpp:272`) 추가.
     - 상세 보기 토글 시 `m_outline` 플레이스홀더 `"진행 중 문제가 발생한 경우 확인할 상세 내용입니다."` (`KaHeritageBrowser.cpp:265`) 및 요청 로그 표시.
  3. **반응형 창 크기 보장 (`KaWindowGeometry::placeDownload` / `fit`)**:
     - `NgiiTopoDownload`, `DemDownload`, `CadastralDownload`, `DrawingPrint`, `NewSurvey`, `SurveyContour`, `AccountSettings` 모두 `ctx.screen_rect()` 기준 반응형 클램프를 적용하여 어떤 모니터 해상도·배율에서도 창이 잘리지 않게 한다.

---

### [Task D25] 파일 드래그 앤 드롭(`MainWindowFiles.cpp`) · CAD 가져오기 안내(`KaCadImportNotice.cpp`) · 종료 시 저장 확인(`KaClosePrompt.cpp`) · 전체 해상도(FHD/와이드/4K) 최종 검증
- **C++ 정본 파일**:
  - `target/cpp-ref/src/app/MainWindowFiles.cpp`
  - `target/cpp-ref/src/app/KaCadImport.cpp`, `KaCadImportNotice.cpp`
  - `target/cpp-ref/src/app/KaClosePrompt.cpp`
  - `target/cpp-ref/src/app/MainWindowOffline.cpp`
- **Rust 대상 파일**:
  - `crates/strata-app/src/gui_shell.rs`
  - `crates/strata-app/src/dialogs_modal.rs`
  - `crates/strata-app/src/verify_harness.rs`
- **상세 구현 내용**:
  1. **창으로 파일 드래그 앤 드롭 (`MainWindowFiles.cpp`, `gui_shell.rs`)**:
     - `ctx.input(|i| i.raw.dropped_files.clone())`를 감지하여 사용자가 탐색기에서 `.shp`, `.dxf`, `.gpkg`, `.tif`, `.img`, `.csv`, `.xlsx`, `.hgis` 파일을 창에 끌어다 놓으면 확장자에 따라 즉시 레이어 추가, CAD 좌표계 확인 안내(`KaCadImportNotice`), 측량 등고선 창(`SurveyContour`), 또는 조사 프로젝트 열기를 수행한다.
  2. **CAD 가져오기 안내창 (`KaCadImportNotice.cpp` → `ActiveModalDialog::CadImportNotice`)**:
     - `.dxf` / `.dwg` 불러오기 시 C++ `KaCadImportNotice`와 동일한 레이어·좌표계 확인 안내 및 지도에 올리기 동작을 연결한다.
  3. **종료 전 변경사항 저장 확인 (`KaClosePrompt.cpp` → `ActiveModalDialog::CloseConfirm`)**:
     - `app.is_dirty == true`일 때 창 닫기 시도(`ViewportCommand::Close` / `close_requested()`)를 가로채어 C++ `KaClosePrompt` (`"저장하고 끝내기"`, `"저장하지 않고 끝내기"`, `"취소"`) 모달을 띄운다.
  4. **해상도·배율별 자동 검증 (`verify_harness.rs`)**:
     - `1600×900` (기본), `1920×1080` (FHD), `2560×1080` / `3440×1440` (21:9 울트라와이드), `3840×2160` (4K HiDPI `pixels_per_point = 1.5` / `2.0`) 레이아웃 경계 검사를 단위 테스트로 고정하여 어떤 모니터와 그래픽카드에서도 UI 잘림이나 겹침이 `0`건임을 보증한다.
