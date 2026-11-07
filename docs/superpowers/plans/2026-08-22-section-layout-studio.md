# 단면도 조판 스튜디오 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 수직 단면 GeoTIFF를 거리·표고 축이 있는 가로 도면으로 구성하고 벡터 PDF로 출력한다.

**Architecture:** 눈금과 조판 생성은 새 `SectionLayoutService`에 두고, 새
`KaSectionDrawingStudio`는 레이어 선택과 속성 입력만 담당한다. `MainWindow`는 아이콘과
탭 생명주기만 연결해 기존 조판과 현재 작업 중인 변경을 보존한다. 입력은 Bentley에서
평면화한 단면 GeoTIFF이므로 기존 2D `section_line` 스키마는 변경하지 않는다.

**Tech Stack:** C++20, Qt6 Widgets/Test, QGIS 3.44 `qgis_core`/`qgis_gui`,
`QgsPrintLayout`, `QgsLayoutView`, `QgsLayoutExporter`

---

## 파일 구조

- Create `src/core/SectionLayoutService.h`: 옵션, 결과, 눈금 API, 조판 생성 API
- Create `src/core/SectionLayoutService.cpp`: 순수 계산과 QGIS 조판 항목 생성
- Create `src/app/KaSectionDrawingStudio.h`: 전용 탭 공개 인터페이스와 신호
- Create `src/app/KaSectionDrawingStudio.cpp`: 3열 UI, 레이어 선택, 재생성, PDF
- Create `tests/test_section_layout.cpp`: 서비스 단위·통합 테스트
- Modify `src/app/MainWindow.h`: 전용 탭 슬롯과 멤버
- Modify `src/app/MainWindow.cpp`: 도구 아이콘, 탭 열기/닫기, GeoTIFF 추가 연결
- Modify `src/app/KaIcons.cpp`: 단면도 아이콘
- Modify `CMakeLists.txt`: 새 소스와 테스트 대상
- Modify `HANDOFF.md`, `docs/HANDOFF.md`: 기능과 파일 지도 동기화

### Task 1: 단면 눈금 계산 RED

**Files:**
- Create: `tests/test_section_layout.cpp`
- Modify: `CMakeLists.txt`

- [ ] `SectionLayoutService::axisTicks(100.01, 100.41, 0.10)`이
      `100.10, 100.20, 100.30, 100.40`을 반환하는 실패 테스트를 작성한다.
- [ ] 상대 거리 `0..2.0m`, 자동 1-2-5 거리 간격, 잘못된 간격과 500개 제한 테스트를 작성한다.
- [ ] `ka_section_layout_tests` 대상을 CMake에 추가한다.
- [ ] Release 테스트 대상을 빌드해 헤더/구현 부재로 실패하는 것을 확인한다.

### Task 2: 단면 눈금 계산 GREEN

**Files:**
- Create: `src/core/SectionLayoutService.h`
- Create: `src/core/SectionLayoutService.cpp`
- Modify: `CMakeLists.txt`

- [ ] 유한값·오름차순·양수 간격을 검증하는 `axisTicks`를 구현한다.
- [ ] 부동소수점 누적 대신 정수 인덱스 곱셈으로 0.10m 눈금을 만든다.
- [ ] `niceDistanceInterval`을 1-2-5 계열로 구현한다.
- [ ] 눈금이 500개를 넘으면 빈 결과와 한국어 오류를 반환한다.
- [ ] 새 테스트 대상이 통과하는지 확인한다.

### Task 3: QGIS 단면 조판 생성 RED/GREEN

**Files:**
- Modify: `tests/test_section_layout.cpp`
- Modify: `src/core/SectionLayoutService.h`
- Modify: `src/core/SectionLayoutService.cpp`

- [ ] 메모리 레이어 범위로 A3 가로 조판을 만들고 `ka_section_map`,
      `ka_section_elevation_*`, `ka_section_distance_*`,
      `ka_section_reference_line`, `ka_section_title_block` 항목을 검증하는 테스트를 먼저 작성한다.
- [ ] `QgsPrintLayout`을 `section_sheet` 이름으로 교체 생성하고 mm 단위를 적용한다.
- [ ] 체크된 레이어의 합성 범위를 지도 프레임에 맞추고 고정 레이어 집합으로 설정한다.
- [ ] GeoTIFF 본체는 `QgsLayoutItemMap`으로 렌더링하되, 표고·거리 축은 지도 좌표
      주기가 아니라 별도 벡터 조판 항목으로 만든다.
- [ ] 지도 항목 CRS는 프로젝트 CRS로 덮어쓰지 않고 첫 단면 GeoTIFF의 CRS를 사용한다.
- [ ] 기존 `layersForRecipe()`를 우회해 사용자가 체크한 래스터 레이어 목록을
      `map->setLayers()`에 직접 넣고 래스터도 유효 콘텐츠로 판정한다.
- [ ] 좌측 표고선/라벨과 하단 상대 거리선/라벨을 `QgsLayoutItemPolyline`과
      `QgsLayoutItemLabel`로 생성한다.
- [ ] 붉은 점선 `#D7191C`, 0.20mm 기준선과 축척자·축척·좌표계·작성일을 생성한다.
- [ ] A3/A4 가로 페이지 크기와 항목 위치가 용지 안인지 테스트한다.

### Task 4: 단면도 전용 탭 UI

**Files:**
- Create: `src/app/KaSectionDrawingStudio.h`
- Create: `src/app/KaSectionDrawingStudio.cpp`
- Modify: `CMakeLists.txt`

- [ ] 좌측 `QTreeWidget`에 프로젝트 레이어를 표시하고 체크, 위/아래, 제거, 갱신 버튼을 연결한다.
- [ ] **GeoTIFF 추가**가 파일 경로 신호를 보내도록 한다.
- [ ] 중앙 `QgsLayoutView`에 `section_sheet`를 연결하고 선택·이동·전체보기 도구를 둔다.
- [ ] 우측에 A3/A4, 도면명, 축척, CRS, 표고 보정, 0.10m 표고 간격,
      자동/수동 거리 간격, 기준선 표시·0.20mm 속성을 둔다.
- [ ] **단면도 만들기**가 서비스 옵션을 조립해 조판을 다시 만들고 보기를 갱신하도록 한다.
- [ ] **PDF 저장**은 `LayoutService::exportLayoutPdf`를 사용해 300 DPI 벡터 PDF를 만든다.

### Task 5: 메인 창 연결

**Files:**
- Modify: `src/app/MainWindow.h`
- Modify: `src/app/MainWindow.cpp`
- Modify: `src/app/KaIcons.cpp`

- [ ] 기존 **도면출력** 옆에 **단면도** 아이콘을 추가한다.
- [ ] `openSectionDesigner()`가 전용 탭을 한 번 만들고 이후 재사용하도록 한다.
- [ ] GeoTIFF 추가 신호에서 기존 `addRasterFromPath()`를 호출해 레이어 생명주기와
      참조 레이어 규칙을 유지한다.
- [ ] 탭 전환 시 레이어를 갱신하고 닫기 시 지도 탭으로 돌아가도록 한다.
- [ ] 기존 `KaDrawingStudio` 동작과 현재 미커밋 DPI 수정은 변경하지 않는다.

### Task 6: 문서와 검증

**Files:**
- Modify: `HANDOFF.md`
- Modify: `docs/HANDOFF.md`

- [ ] 두 SSOT 문서에 단면도 조판과 새 파일 지도를 같은 내용으로 기록한다.
- [ ] `cmake --build build --config Release`를 실행한다.
- [ ] `ctest --test-dir build -C Release --output-on-failure`를 실행한다.
- [ ] `.\scripts\run-ka-hgis.ps1 --smoke-quit`를 실행한다.
- [ ] 변경 diff를 검토해 `removeAllMapLayers`, VWorld 키, 제출 CRS 5179 불변을 확인한다.

## 구현 제약

- 사용자가 요청하지 않았으므로 커밋하지 않는다.
- 기존 `KaDrawingStudio.cpp`, `LayoutService.cpp`, `tests/test_workflow.cpp`의 미커밋 변경을 덮어쓰지 않는다.
- 단면 GeoTIFF의 세로축을 일반 지도 북ing으로 표기하지 않는다.
- 기존 `section_line`을 Z 지오메트리로 바꾸거나 GPKG 마이그레이션을 추가하지 않는다.
- 레이어가 없거나 좌표 범위가 유효하지 않으면 빈 도면을 만들지 않는다.
