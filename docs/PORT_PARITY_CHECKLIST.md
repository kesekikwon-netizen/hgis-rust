# C++ → Rust 기능 대조 (2026-11-18)

기준 Rust: `f1835fa` (`origin/main`). 기준 C++: `kesekikwon-netizen/gogohak1` `b036c0905f6ac1e97b6d142a5e101ac5d30ca7c7`.
이 체크아웃에는 `target/cpp-ref`가 없다. 이번 조사는 그 커밋을 `/tmp/cpp-ref`에만 받아 읽었다. 제품 코드는 바꾸지 않았다.

상태 단어는 네 개다.

| 상태 | 뜻 |
|---|---|
| `done` | 그 조각의 동작이 C++와 같고, 입구가 살아 있다 |
| `partial` | 일부만 연결되어 있다 |
| `missing` | 없거나, 툴팁을 달고 항상 꺼져 있다 |
| `divergent` | 연결되어 있으나 C++와 다르거나, 하지 않은 일을 했다고 말한다 |

`확인` 열:

- `재확인` — 2026-11-18 이 세션에서 그 파일을 열었다.
- `커밋` — 2026-11-14 표 이후 커밋이 그 조각을 바꿨다. 커밋 메시지와 입구만 본다.
- `11-14` — `docs/PARITY_STATUS.md`(트리 `8c2913bf`)의 행이다. 그 파일의 줄 번호는 그때의 Rust다. 지금은 줄이 옮겨졌을 수 있다. 이 열의 `done`은 그 표가 `same`이라고 적은 뜻이지, 2026-11-18에 다시 실행해 확인한 완료가 아니다.

`docs/RUST_PARITY_CHECKLIST.md`(2026-11-12)와 `docs/PARITY_STATUS.md`(2026-11-14)는 기능 이름을 고르는 목록이다. 그 상태 열을 오늘 상태로 복사하지 않았다. 메뉴 감사 113행(`docs/research/menus/CURRENT_MENU_AUDIT.md`)은 전체 기능 수가 아니다.

## 1. 이번 세션에서 다시 연 것

### 1.1 투명도 카드

C++는 카드가 두 개다. 같은 클래스이고 기억 키만 다르다.

| 조각 | 상태 | C++ | Rust | 차이 |
|---|---|---|---|---|
| 지도 카드 생성 | `done` | `src/app/MainWindow.cpp:963` 키 `"map"` | `gui_tab_map_opacity.rs`, `gui_tab_map_opacity_placement.rs` | 카드는 지도에 있다 |
| 프레임 NoFocus, 슬라이더 ClickFocus | `partial` | `KaLayerOpacityRail.cpp:22`, `:47`, `:67` | MENU22가 고친 뒤의 `gui_tab_map_opacity_keys.rs` | 인수인계는 생산 입력 15개로 적는다. 확인은 Windows CI |
| 지도에서 빈 곳 끌기 | `partial` | `KaLayerOpacityRail.cpp:218-249` | `gui_tab_map_opacity_placement.rs:111-135` | 세션 메모리만. 프로세스를 끄면 사라진다 |
| 기본 위치 | `divergent` | `:207` `move(10, 10+inset)` 왼쪽 위 | `gui_tab_map_opacity_placement.rs:69-71` 왼쪽 아래 | 주석이 소유자 UI11이라고 적는다. 이식 버그로 되돌리지 않는다 |
| 디스크 키 `ui/opacityRail/map` | `missing` | `:13-15`, `:101-105`, `:248-249` `QSettings` | egui `Id::new("ka_map_opacity_placement")` `:44-46`, `:160-163` | ini에 쓰지 않는다 |
| 도면 카드 | `missing` | `KaDrawingStudio.cpp:1196` 키 `"drawing"`, `:1198-1230`, `:2563-2575` | `gui_tab_drawing_sheet.rs:19-22`는 고정 안내 사각형만 그린다 | 슬라이더·끌기·레이어 적용이 없다. WU-02는 그 안내를 카드로 바꾼다 |
| 디스크 키 `ui/opacityRail/drawing` | `missing` | 위와 같은 `QSettings` 키 | 없음 | 지도 키와 따로다 |
| 값의 적용 대상 | `divergent` | `MainWindow.cpp:989-1000` 현재 레이어만. 도면은 `m_railLayer` | `apply_rail_opacity` `gui_tab_map_opacity.rs:44-49` | 전역 `layer_opacity`, 배경 투명도, 그림 행을 같이 쓴다 |
| 범위 | `partial` | 투명도 0–100, 밝기 −255–255 (`:41-42`, `:60-61`) | 같은 범위로 주석과 키 처리 | 밝기 기본 저장값은 Rust 새 조사에서 `0.5` (`real_survey_load.rs:29`). C++ 슬라이더 초기값은 0 |
| 새 조사 투명도 | `done` (1.0) | QGIS 레이어 기본 1 | `real_survey_load.rs:28` `layer_opacity: 1.0` | 스크린샷 대화의 「조사구역 0.25」는 이 기본값이 아니다. `opacity_placement_input_tests_a.rs:65-67`은 슬라이더를 트랙의 25%에 둔 시험이다 |
| 움직이지 않은 클릭 | `partial` | 릴리즈 때 `m_placed`이면 저장 | MENU22: 움직이지 않은 클릭은 수동 배치로 기억하지 않음 | 디스크 저장을 넣을 때 이 규칙을 유지한다 |

늦은 읽기: 파일을 연 직후 ini를 적용하다가, 그 사이에 사용자가 이미 끈 자리를 덮으면 안 된다. `CURSOR_HANDOVER.md` 다음 순서 3번과 같다.

### 1.2 꺼져 있거나 거짓으로 말하는 입구

| 조각 | 상태 | C++ | Rust |
|---|---|---|---|
| 제출 꾸러미, 레이어 5179 변환 | `missing` | `KaSubmitFlow.cpp`의 `makeSubmitPackage` | `modal_export_footer.rs:9`, `:23-40` 항상 꺼짐, 툴팁 `지시 R2` |
| 도면 PDF | `partial` | `KaDrawingStudio.cpp:3819-3855` 벡터 PDF | `gui_tab_drawing_right_b.rs:83-91`가 `export_sheet_pdf`를 호출. `gui_tab_drawing_print.rs:100-121`은 300 DPI 그림. 제출용 벡터 PDF가 아니다 |
| 나눈 장 PDF | `missing` | `KaPrintDialog.cpp:204` 근처 | `dlg_drawing_print_actions.rs:20` 툴팁 `지시 W-50` |
| GeoTIFF | `missing` | `MainWindowExport.cpp` | `gui_ribbon_draw.rs:13` 툴팁 `지시 R7` |
| 버퍼 엑셀 | `missing` | `MainWindowExport.cpp:512` | `gui_ribbon_subtoolbar.rs:39` 툴팁 `지시 W-H1` |
| 수치지형 도엽 확인 | `missing` | `KaTopographicBrowser.cpp` | `ngii_topographic_state.rs:13-31` 툴팁 `지시 WB2` |
| 입체지형 「화면을 입체로」 | `missing` | `KaTerrain3dStudio.cpp` | `gui_tab_terrain3d.rs:11` 툴팁 `지시 W-56` |
| 범례의 불러오기·꼭짓점·묶기·시굴·오프라인·제출·모두 지우기 | `missing` | `MainWindowContextMenus.cpp` | `layer_menu_model.rs:182-290` `deferred`, 지시 `W-A12` `CM2a-2` `CM2a-3` `W-T1` `R4b` `R2` |
| 쉽게그리기·구간 분리 | `missing` | 그리기 줄 | `gui_ribbon_draw_row.rs:95` 구간 분리는 `enabled = false`. 쉽게그리기는 `지시 MENU17` 툴팁 |
| 시굴 「구역에 깔기」 | `divergent` | `KaTrenchDialog.cpp`가 격자를 만든다 | `dlg_trench_generator.rs:56-71` 보이기 플래그와 「배치했습니다」만. `trenches`를 채우지 않는다 |
| 등고선 미리보기·만들기 | `divergent` | `KaSurveyContourDialog.cpp`가 파일을 읽는다 | `dlg_survey_contour.rs:146-197` 경로가 비어 있지 않으면 「점 248개, 표고 24.5–48.2 m」와 「표고점 248개」. 빈 `section_line` 레이어를 연다 |
| 더보기의 나머지 | `divergent` | 항목마다 다른 창 | `gui_ribbon_menus_b.rs:99-114`. 화면 보기·맞추기·파일함·벡터 불러오기는 자기 일을 한다. 나머지는 계정 창 |
| OSM / Google | `divergent` | 각 타일 소스 | 같은 파일 `:103-108`이 VWorld 일반·위성으로 바꾼다 |

### 1.3 11월 14일 표보다 앞으로 온 것

| 조각 | 상태 | 근거 |
|---|---|---|
| SHP를 끌어 놓거나 「벡터·도면 불러오기」 | `partial` | `user_files.rs:115-167`. 계획 문서는 점 SHP·GeoJSON·DXF·DWG를 아직 아님으로 적는다. DWG는 LibreDWG(GPL)를 넣지 않는다 |
| 지질·토양·수계 | `partial` | `ref_map.rs:81-152`에 준비 함수, 반경 10 km, C++ 심볼 주석. 11월 14일 표의 「문구만」은 현재가 아니다. 실제 서버 성공은 이 세션에서 실행하지 않았다 |
| 유산 「받기」 | `partial` | `dlg_heritage_browser_footer.rs:60-76`이 조건이 맞으면 `start_heritage_fetch`. 실제 사이트는 `CURSOR_HANDOVER.md`가 미검증으로 적는다 |
| 폴리곤 묶기·나누기 | `partial` | `gui_ribbon_draw_row.rs:104` → `merge_selected_polygons`. `app_state_split.rs:80` `split_polygons_with_line`. 범례의 묶기 항목은 여전히 꺼져 있다 |
| 홈 | `divergent` | `gui_tab_home.rs:25-26`이 `render_home_explorer`만 그린다. C++ `KaStartPage`가 아니다. UI5 `ab8c8f1` |
| 인쇄 | `partial` | 미리보기 탭과 Windows 인쇄 보조 프로세스가 있다. 나눈 장 PDF는 꺼져 있다. 항목별 대조는 5절 |

### 1.4 저장소에 없는 것

- `fix/opacity-selected-layer`, `fea3e42`: GitHub에 그 SHA가 없다. 스크린샷 PC의 로컬 커밋으로 본다. 그 커밋의 「조사구역 0.25, 공유 세션 1.0」을 main의 구현으로 적지 않는다.
- `dotak`, `dotak-independent-rubbing`, `dotak-studio-hgis-rust-workspace`: 이 트리에서 검색되지 않는다.
- 스캔 TIFF의 격자선 지우기: C++ `b036c09`와 Rust 양쪽 없음. 좌표 격자 표시와 다른 일이다.

## 2. 기능 목록

89행은 `docs/PARITY_STATUS.md`의 사용자 기능이다. 상태를 오늘 고친 행만 위 1절에 맞췄다. `11-14` 행은 그 문서의 분류를 네 단어로 옮긴 것이다 (`same`→`done`, `partial`→`partial`, `disabled`→`missing`, `fake`→`divergent`, `missing`→`missing`, `rust_only`→`divergent`).

### 조사 파일

| 기능 | 상태 | 확인 | C++ | Rust (표가 가리키던 곳 또는 이번에 연 곳) |
|---|---|---|---|---|
| 새 조사 | `partial` | 커밋 | `MainWindowSession.cpp:668-750`, `KaNewSurveyDialog.cpp` | 미저장 질문은 MENU9. 자 눈금·추천 CRS·`.qgz`는 11-14에 미완으로 남음 |
| 열기 | `partial` | 커밋 | `MainWindowSession.cpp:242-443` | 미저장 질문 MENU9. 내장 작업공간은 없음 |
| 저장 | `partial` | 11-14 | `MainWindowSession.cpp:874-1000` | `app_state_projects.rs`, `survey_gpkg_save.rs` |
| 다른 이름 | `partial` | 11-14 | `MainWindowSession.cpp:1419-1631` | `app_state_projects.rs` |
| 닫기 질문 | `partial` | 커밋 | `KaClosePrompt.cpp:25-52` | MENU10이 입력 소유를 고침. 문구·건수 빵부스러기는 11-14에 차이로 남음 |
| 복구 사본 2분 | `missing` | 11-14 | `KaRecoverySnapshots.cpp` | 호출부 없음. 자동 저장 금지 결정과 충돌하므로 구현 전에 소유자 확인 |
| 창 제목 `*` | `done` | 11-14 | `MainWindowSession.cpp:1327-1334` | `app_state_lifecycle.rs` |
| 탭 배지 | `partial` | 11-14 | `KaSurveyBadge.cpp` | `gui_ribbon_tabbar.rs` |

### 기록

| 기능 | 상태 | 확인 | C++ | Rust |
|---|---|---|---|---|
| 도형선택 | `partial` | 커밋 | `KaFeatureSelectTool.cpp` | Shift·상자·Delete는 11-16 커밋. 꼭짓점 메뉴는 `missing` (`layer_menu_model.rs:220`) |
| 되돌리기 | `partial` | 11-14 | `MainWindowUndo.cpp` | `gui_tab_map_undo.rs`, `app_state_draw_commit.rs` |
| 측거 | `partial` | 11-14 | `KaMeasureMapTool.cpp` | `gui_tab_map_measure.rs` |
| 그리기 줄·스케치 | `partial` | 커밋 | `KaDrawSketchTools.cpp` | MENU17이 세 도구줄을 다시 배치. 쉽게그리기·구간 분리는 `missing` |
| 자석 | `partial` | 커밋 | `KaCaptureMapTool.cpp:153-190` | SNAP1A/B는 자료와 CRS 거절. 캡처 중 실시간 자석은 인수인계가 미연결로 적음 |
| 쉽게그리기 | `missing` | 재확인 | `MainWindowEditing.cpp:1035-1085` | `지시 MENU17` |
| 조사구역 그리기 | `partial` | 11-14 | `KaSurveyAreaDialog.cpp` | `survey_area_setup.rs`, `app_state_draw_commit.rs` |
| 유구 면·이름 번호 | `partial` | 11-14 | `KaFeatureFormDialog.cpp` | `app_state_feature_form.rs` |
| 유구 선·단면선 | `partial` | 커밋 | `MainWindowEditing.cpp:1151-1166` | 캡처 CRS 시험 파일이 있다. 11-14의 「레거시 cad로만 간다」는 그 날짜의 문장 |
| 유물 점 | `partial` | 커밋 | `MainWindowEditing.cpp:1168-1174` | 점 캡처 경로가 생겼다. 11-14의 「scene에만 찍힌다」를 오늘 다시 단정하지 않음 |
| GPS 기준점 | `partial` | 11-14 | `KaControlPointDialog.cpp` | `modal_control.rs` |
| 폴리곤 묶기·나누기 | `partial` | 재확인 | `MainWindowExport.cpp:442-510`, `MainWindowEditing.cpp` | 리본은 연결. 범례 항목과 구간 분리·겹침 지우기는 별도 |
| 시굴격자 | `divergent` | 재확인 | `KaTrenchDialog.cpp`, `TrenchGridGenerator.cpp` | `dlg_trench_generator.rs:56-71`. 계산 라이브러리 `strata-geo`는 화면과 떨어져 있다 |
| 버퍼 500·1000 | `partial` | 커밋 | `BufferAnalysis.cpp` | MENU14 CRS 거절, MENU15 저장. 엑셀은 `missing` |
| 버퍼 엑셀 | `missing` | 재확인 | `MainWindowExport.cpp:512` | `지시 W-H1` |

### 자료 받기·배경

| 기능 | 상태 | 확인 | C++ | Rust |
|---|---|---|---|---|
| 지적 5 km | `partial` | 11-14 | `MainWindowCadastral.cpp`, `CadastralPortal.cpp` | `cadastral_download_ui_*.rs` |
| 지적 선 색·지번 | `partial` | 11-14 | `MainWindowCadastral.cpp:56-80` | `gui_ribbon_menus_a.rs` |
| 수치지형 내려받기 | `partial` | 재확인 | `KaTopographicBrowser.cpp` | 리본 「수치지형」은 `topo_download_run.rs:86-95`가 받아 둔 파일을 올리거나 `press_ngii_order`로 숨은 주문을 시작한다. 그 파일은 Windows 전용 `strata_ngii_run`을 조건 없이 쓴다. 옛 창의 「도엽 확인」은 `ngii_topographic_state.rs:30`에서 `WB2`로 꺼져 있다 |
| 받아 둔 수치지형 폴더 | `partial` | 11-14 | `KaTopographicImportDialog.cpp` | `dlg_topographic_import.rs` |
| 유산 | `partial` | 재확인 | `KaHeritageBrowser.cpp` | `heritage_run_start.rs`. 실제 사이트 미검증 |
| 시추 단추 | `divergent` | 커밋 | C++에 없는 기능 | 전용 단추는 리본 클릭 파일에서 안 보였다. 지질 메뉴 문구에 시추주상도가 남아 있다 (`gui_ribbon_menus_a.rs:24-26`) |
| VWorld 위성·일반 | `partial` | 11-14 | `BasemapOps.cpp:1161-1279` | `basemap_tiles.rs` |
| 연속지적도 WMS | `partial` | 11-14 | `BasemapOps.cpp:1280-1395` | `basemap_cadastral.rs` |
| 지형 음영 | `partial` | 11-14 | `MainWindowOverlay.cpp:239-267` | 11-14는 `divergent`(문구만). 받은 지도 재오픈 커밋 이후라 오늘 다시 단정하지 않음 |
| DEM 받기 | `partial` | 11-14 | `MainWindowDownloads.cpp` | `dem_download_ui_*.rs`. 지도에 그리는지는 11-14에 미완 |
| DEM 표현 | `divergent` | 11-14 | `KaDemClassDialog.cpp` | `modal_style_demclass.rs`의 고정 고도 문장 |
| 국토지리원 DEM 파일·음영 | `partial` | 11-14 | `MainWindow.cpp:1686-1774` | 메뉴가 다른 창을 염 |
| 측량 등고선 | `divergent` | 재확인 | `KaSurveyContourDialog.cpp` | `dlg_survey_contour.rs:146-197` |
| 토양·지질·수계 | `partial` | 재확인 | `SoilMapService.cpp`, `GeologyMapService.cpp`, `RiverMapService.cpp` | `ref_map.rs` |
| 고지형 | `divergent` | 11-14 | `PaleoLandformService.cpp` | `gui_tab_paleo.rs` 표본 분석 |
| 옛 지도 | `partial` | 커밋 | `MainWindowOffline.cpp:456-477` | MENU16은 메뉴가 닫히던 버그와 로컬 선택. 온라인 영상은 인수인계가 미검증 |

### 정합·내보내기·화면

| 기능 | 상태 | 확인 | C++ | Rust |
|---|---|---|---|---|
| 정합 세션 | `partial` | 재확인 | `MainWindowAlign.cpp:627-880`, `GeorefService.cpp` | 래스터는 점 2개 이상이면 `gui_ribbon_align_actions.rs:11-37`이 `create_aligned_raster`로 월드 파일·`.prj`·`정합백업`을 쓴다 (`raster_alignment.rs:18-89`). 벡터·CAD 세션과 먹선 Multiply 스타일은 없다 |
| CAD DXF·DWG 안내 | `partial` | 11-14 | `KaCadImport.cpp` | `dlg_cad_import_notice.rs`. DWG는 GPL LibreDWG를 쓰지 않음. DXF 드라이버는 엔진에 켜져 있다 |
| 용지 설정 | `partial` | 커밋 | `KaPaperDialog.cpp` | MENU19. A4 가로 317×220(C++ 현장 용지) 대 297×210은 11-14에 차이로 남음. 오늘 수치를 다시 재지 않음 |
| 도면 스튜디오 | `partial` | 커밋 | `KaDrawingStudio.cpp` | 범례 둥근 테 MENU20. 투명도 카드는 `missing`(1.1) |
| PDF 내보내기 | `partial` | 재확인 | `KaDrawingStudio.cpp:3819-3855` | 그림 PDF. 벡터 아님 |
| 인쇄 | `partial` | 커밋 | `KaPrintDialog.cpp` | UI3. Windows에서만 실물 확인 |
| 나눈 장 PDF | `missing` | 재확인 | `KaPrintDialog.cpp:592-620` | `지시 W-50` |
| GeoTIFF | `missing` | 재확인 | `MainWindowExport.cpp:57-98` | `지시 R7` |
| 검수 27규칙 | `partial` | 11-14 | `KaSubmitDialog.cpp` | `checklist_state.rs`. 창은 살아 있다 |
| 제출 꾸러미·5179 변환 | `missing` | 재확인 | `KaSubmitFlow.cpp:81-189` | `지시 R2` |
| 입체지형 도면 | `divergent` | 11-14 | `KaTerrain3dLayoutStudio.cpp` | `gui_tab_terrain3d.rs` |
| 주소 검색 | `partial` | 11-14 | `MainWindow.cpp:2757-2882` | UI 스레드에서 동기 검색으로 11-14에 적힘 |
| 지역 17칸 | `partial` | 11-14 | `KaRegionLocator.cpp` | `gui_ribbon_region.rs` |
| 벡터·도면 불러오기 | `partial` | 재확인 | `MainWindow.cpp:2592-2639` | `user_files.rs`. 점·GeoJSON·DXF 입구는 계획에 미완 |
| 참조 벡터 분리·임시 정리 | `divergent` | 재확인 | `MainWindowSession.cpp:1038-1115` | 더보기 나머지가 계정 창 |
| CSV 기준점 | `divergent` | 재확인 | `MainWindow.cpp:2538-2578` | 같은 sink |
| OSM·Google·웹 자료 | `divergent` | 재확인 | `MainWindowOffline.cpp:479-500` | VWorld로 바꿈. 웹 자료 하위 메뉴는 11-14에 없음 |
| 자료 준비 상태 | `divergent` | 11-14 | `KaReferenceStatusDialog.cpp` | 고정 시각·크기가 11-14에 적힘. 더보기에서는 계정 창 |
| 정보(About, GPL 고지) | `missing` | 재확인 | `MainWindow.cpp:3048-3062` | 더보기가 계정 창. About 소스 검색에서 창이 안 나옴 |
| 테마·F9/F11 | `partial` | 11-14 | `KaThemeOptions.cpp`, `KaShellFocus.cpp` | 단축키는 있다. 더보기 항목은 계정 창. 테마 저장은 11-14에 없음 |
| 지도 위 +/−, 축척, 격자, 투명도 | `partial` | 재확인 | `MainWindow.cpp:964-1028` | 투명도는 1.1. 경위도 격자·원점·범위 이력은 11-14에 미완 |
| 입체지형 탭 | `divergent` | 재확인 | `KaTerrain3dStudio.cpp` | `지시 W-56`이 남아 있다 |
| 한글 보고서 | `missing` | 11-14 | `KaHwpReportStudio.cpp` | 없음 |
| 레이어 목록 | `partial` | 커밋 | `KaLayerListChrome.cpp` | MENU1 이름 있는 세트. 끌어서 순서 바꾸기는 11-14에 없음 |
| 표시 설정 | `divergent` | 11-14 | `KaLayerInformationPanel.cpp` | `gui_layer_panel_card.rs`가 고른 레이어 칸을 안 쓴다고 11-14에 적힘 |
| 파일함 | `partial` | 커밋 | `KaFileBrowserPanel.cpp` | SHP 적재는 `user_files.rs`. 11-14의 「5개만, 적재 문구만」은 그 날짜 |
| 레이어 우클릭 동작 항목 | `partial` | 11-14 | `MainWindowContextMenus.cpp` | 이름·라벨·투명도 메뉴·삭제는 연결되어 있다고 11-14에 적힘 |
| 레이어 우클릭 보류 항목 | `missing` | 재확인 | 같은 파일 | `layer_menu_model.rs:182-290` |
| 지도 우클릭 | `partial` | 11-14 | `MainWindowContextMenus.cpp:112-170` | 범위 이력이 안 쌓이면 이전·다음이 죽도록 11-14에 적힘 |
| 도형 속성 창 | `partial` | 11-14 | `KaRecordPlate.cpp` | `dlg_feature_record.rs` |
| 선택 도구 우클릭(점 삭제·구멍) | `missing` | 11-14 | `KaFeatureSelectTool.cpp:276-415` | `gui_tab_map_context.rs`가 일찍 반환 |
| 오른쪽 검사기 | `partial` | 11-14 | `KaInspectorPanel.cpp` | 복수 선택 개수 표시가 약함 |
| 속성 카드 | `partial` | 11-14 | `KaFeatureCard.cpp` | `ri_attr_card.rs` |
| 스타일 탭 | `divergent` | 11-14 | `KaInspectorStyle.cpp` | `ri_layer_card.rs`가 그린 색을 조사 표 렌더가 안 읽는다고 11-14에 적힘 |
| 배경 빠른 카드 | `partial` | 11-14 | `KaBasemapQuickCard.cpp` | `ri_body_bottom.rs`. 지형이 일반 지도로 감 |
| 상태줄 | `partial` | 11-14 | `KaStatusBar.cpp` | `status_bar_text.rs` |
| 자석 칩 | `missing` | 11-14 | `KaStatusBar.cpp:48-52` | 없음 |
| 축척 입력 | `partial` | 11-14 | `MainWindowOffline.cpp:376-432` | `app_state_status_bar.rs` |
| 작업 CRS 전환 | `divergent` | 11-14 | `MainWindow.cpp:700-716` | 메뉴 없이 토글하고 화면 중심을 다시 투영하지 않는다고 11-14에 적힘 |
| 지도갱신 멈춤 | `divergent` | 11-14 | `MainWindow.cpp:717-721` | 글자만 바뀌고 그리기를 멈추지 않는다고 11-14에 적힘 |
| 홈 히어로·최근·작업 순서 | `divergent` | 재확인 | `KaStartPage.cpp` | 폴더 탐색으로 교체. 11-14의 카드별 `fake`는 그 화면의 이야기 |
| 연결 상태 | `partial` | 11-14 | `KaHomeConnectionCard.cpp` | 홈 탐색 안으로 옮겨졌는지 오늘 다시 열지 않음 |
| 시작 스플래시 | `missing` | 11-14 | `KaStartupSplash.cpp` | `main.rs`가 홈으로 바로 연다. 제품 결정은 시작 시 홈만 |
| API 키 | `partial` | 11-14 | `KaApiKeyDialog.cpp` | `dialogs_account.rs`, `key_stores.rs`. 키 값은 읽지 않음 |
| 포털 계정 | `partial` | 11-14 | `KaHeritageSetupDialogs.cpp` 등 | DPAPI와 계정 저장은 Windows. 확인은 Windows CI |
| ini의 최근 조사·마지막 폴더 | `partial` | 11-14 | `MainWindowSession.cpp:1336-1364` | `recent_surveys.rs`. 테마·창 위치는 11-14에 없음. 투명도 위치 키도 없음 |

## 3. 닫는 작업 단위

각 단위는 따로 맡길 수 있다. 검증은 그 단위의 시험이 실패했다가 통과하는 것과, 아래 파일 밖의 동작이 그대로인 것이다. 사용자 조사 원본은 쓰지 않는다. 실제 사이트 로그인은 넣지 않는다.

### WU-01 슬라이더는 고른 레이어만

- 목표: `apply_rail_opacity`가 `MainWindow.cpp:989-1000`처럼 현재 레이어의 투명도만 바꾼다. 전역 `layer_opacity`와 배경 전체를 같이 쓰지 않는다.
- 볼 파일: `crates/strata-app/src/gui_tab_map_opacity.rs`, 그 시험.
- 검증: 조사구역을 0.25로 둔 뒤 위성 행의 투명도는 그대로다. 그림 행을 고르면 그 행만 바뀐다. 밝기는 래스터가 아니면 비활성(이미 MENU21 범위).
- 끝: 그 시험이 수정 전에 실패하고, 수정 후에 통과한다.

### WU-02 도면 투명도 카드

- 목표: `KaDrawingStudio.cpp:1196-1234`, `:2563-2575`처럼 도면에 카드를 붙이고, 조판이 기억하는 레이어에 투명도·밝기를 적용하고, 캐시를 다시 그리게 한다. `gui_tab_drawing_sheet.rs:19-22`의 고정 안내 사각형은 카드가 아니다. 카드가 생기면 그 안내를 남기지 않는다.
- 의존: WU-01의 적용 함수를 재사용한다. 위치 저장(WU-03)은 하지 않아도 이 단위는 끝이다.
- 검증: 도면에서 고른 그림 레이어의 값만 바뀐다. 지도를 건드리지 않는다. 생산 egui 입력으로 빈 곳 끌기와 슬라이더 클릭이 도면 이동으로 새지 않는다.

### WU-03 두 위치를 디스크에 따로

- 목표: C++ 키 `ui/opacityRail/map`과 `ui/opacityRail/drawing` (`KaLayerOpacityRail.cpp:13-15`, `:101-105`, `:248-249`). 기존 `strata-storage`의 Qt ini 읽기·쓰기를 쓴다. 새 설정 파일을 만들지 않는다.
- 의존: 지도 키는 WU-01과 독립. 도면 키는 WU-02 다음.
- 유지: 기본 모서리는 왼쪽 아래(UI11). 움직이지 않은 클릭은 저장하지 않는다(MENU22). 연 뒤 도착한 ini가 이미 끈 자리를 덮지 않는다.
- 검증: 임시 ini에 지도 점과 도면 점을 다르게 쓰고, 새 상태가 각각 복원한다. 끌기 시작 뒤에 같은 키를 다시 읽어도 끈 자리가 남는다. C++ `QSettings`가 쓰는 `QPoint` 바이트와 맞출지는 `qt_ini_tests.rs`의 기존 Qt 바이트 규칙으로 확인한다. 맞지 않으면 두 앱이 자리를 공유하지 못한다.
- 끝: 프로세스 메모리만 보던 배치 시험이 디스크 왕복을 포함한다.

### WU-04 시굴 「구역에 깔기」가 도형을 만들거나, 만들기 전에는 침묵

- 목표: `TrenchGridGenerator` / `strata-geo` 배치 결과를 GPKG `trial_trench`에 쓰거나, 쓰기 전에는 「배치했습니다」를 내지 않고 단추를 끈다.
- 볼 파일: `dlg_trench_generator.rs:56-71`. 그 클릭은 `trenches.len()`으로 문장만 만든다. 대화상자가 `strata_geo::trench_grid` 타입을 쓰는 것과, 메모리의 `real_survey.trenches`는 그 표가 아니다. 열 때 그 벡터는 비운다.
- 검증: 빈 목록에서 단추를 누르면 성공 문장만 생기지 않는다. 만드는 경로를 고르면 임시 조사의 `trial_trench` 개수가 계산 결과와 같고, 저장 뒤 다시 열어도 그 개수가 남는다.

### WU-05 등고선의 고정 248점 제거

- 목표: `dlg_survey_contour.rs:146-197`의 「점 248개」「24.5–48.2 m」「표고점 248개」를 없앤다. 파일을 읽어 TIN을 만들거나, 그 전까지 만들기 단추를 끄고 빈 `section_line`을 추가하지 않는다.
- 검증: 아무 경로 문자열이나 넣고 열어도 그 세 숫자가 화면에 없다. 빈 경로의 「좌표 파일을 고르세요.」는 유지한다.

### WU-06 제출 꾸러미의 최소 실물

- 목표: `지시 R2`를 끄고, 5179 SHP와 `MANIFEST.sha256`, 오류가 있으면 한 번 묻고 `README_submit.txt`에 적는다. 조성된 도면이 없으면 `조사도면.pdf`를 만들지 않고 이유를 알린다(`docs/RUST_PLAN.md` R2). DXF는 제출 경로가 아니다.
- 쓸 코드: `strata-check-export`의 `submit_builder` (`#[cfg(feature = "gdal")]`, `lib.rs:21-22`). 앱은 그 기능을 켜지 않는다 (`crates/strata-check-export/Cargo.toml:17`, `strata-app/Cargo.toml`은 기능 없이 의존). `submit_export.rs`의 고정 `(0,0)–(100,100)` 다각형과 자리 표시 PDF는 제품 경로에 붙이지 않는다.
- 검증: 임시 조사에서 패키지 폴더의 SHP를 다시 열어 도형 수와 EPSG:5179를 확인한다. 그 도형은 고정 네모가 아니다. 체크리스트 실패가 패키지를 막지 않고 README에 남는다. 지도에는 패키지를 올리지 않는다.

### WU-07 더보기가 계정 창으로 떨어지지 않게

- 목표: `gui_ribbon_menus_b.rs:113`의 `else`를 항목별로 나눈다. CSV 기준점, 정보(GPL 고지), 임시 자료 정리, 메뉴의 지도 넓게·패널 접기.
- 한 단위에 구현을 다 넣지 않는다. 첫 단위는 sink를 없애고, 아직 없는 기능은 계정 창이 아니라 꺼 둔 툴팁으로 보내는 것이다.
- 검증: 각 항목의 클릭 시험이 `AccountSettings`가 아닌지 본다. 「API 키」만 계정 창이다.

### WU-08 정합의 남은 한 단계 (래스터 저장은 하지 않음)

- 이미 있음: 점 2개 이상에서 `apply_align_move`가 `create_aligned_raster`로 월드 파일, `.prj`, `정합백업`을 쓴다 (`gui_ribbon_align_actions.rs:15-37`, `raster_alignment.rs:88-89`). 이 단위에서 그 쓰기를 다시 만들지 않는다.
- 목표: 벡터·CAD 정합 세션이거나, 맞춘 스캔의 먹선 스타일(`GeorefService.cpp`의 Multiply·대비) 중 하나. 둘을 한 변경에 넣지 않는다. 추천은 먹선 스타일인데, 원본 그림 바이트는 그대로 두고 표시만 바꾼다.
- 검증: 고른 쪽의 시험이 수정 전에 실패한다. 래스터 「이동」이 쓰던 월드 파일 바이트는 그대로다.

### WU-09 수치지형 옛 창을 리본 주문과 겹치지 않게

- 이미 있음: 리본 「수치지형」은 `open_topographic_download` (`topo_download_run.rs:86-95`)가 받아 둔 합본을 올리거나 숨은 주문을 시작한다. 새 「도엽 확인」 클라이언트를 만들지 않는다.
- 남은 것: `ngii_topographic_state.rs`의 꺼 둔 「도엽 확인」은 그 리본과 다른 옛 창이다. 그 단추를 두 번째 주문으로 살리지 말고, 리본과 같은 진입으로 붙이거나 옛 창을 닫는다.
- 검증: Windows CI에서 리본 경로의 기존 시험이 유지된다. 실제 로그인·신청은 넣지 않는다.

### WU-10 작업공간 읽기의 첫 조각

- 목표: C++가 저장한 조사의 내장 프로젝트에서 레이어 켜짐과 순서만 읽는다. 쓰는 쪽은 아직 C++ 바이트를 유지한다(`docs/RUST_PLAN.md` 2D의 「안 되면」).
- 검증: 저장소에 이미 있는 데모 사본(사용자 원본 아님)을 열면 켜짐이 C++ 저장과 같다. 저장 뒤 `qgis_projects` 바이트가 그대로다.

### WU-11 떨어짐 (소유자 결정 2026-11-18, Windows only)

- 상태: dropped. Linux 빌드, Linux 시험, Linux CI, Windows 전용 코드를 Linux에서 컴파일되게 하는 수정은 하지 않는다.
- 검증: Windows CI. 이 단위를 다시 열지 않는다.

### WU-12 인쇄·조판 대조

Windows만. 항목별 근거는 5절. 한 번에 하지 않고 아래 세 조각으로 나눈다. 용지 210×297, 현장 317×220, A3, 방향, 300/600 DPI 선택, 미리보기는 다시 만들지 않는다.

#### WU-12a 조판 PDF를 벡터로

- 목표: 「PDF 내보내기」가 지금 화면의 300 DPI 그림(`sheet_pdf_file.rs:10`, `gui_tab_drawing_print.rs:99-121`)이 아니라, C++ `KaDrawingStudio.cpp:3819-3855`처럼 글자를 고를 수 있는 조판 PDF가 된다. 표제, 범례, 방위, 지도 프레임, 축척 막대의 자리와 문구가 5절의 C++ 줄과 같다. 글꼴은 맑은 고딕(`LayoutService.cpp:394`).
- 파일: `crates/strata-layout/src/sheet_pdf_export.rs`(빈 네모 경로, 제품 단추에 붙이지 않음), `crates/strata-app/src/gui_tab_drawing_print.rs`, `gui_tab_drawing_chrome.rs`, `gui_tab_drawing_legend.rs`.
- 검증: Windows에서 임시 용지의 PDF를 열어 페이지 크기가 용지 mm와 같고, 제목 문자열이 텍스트로 들어 있다. 그림만 있는 PDF는 실패다.
- 끝: 그 검사가 수정 전에 실패하고 수정 후에 통과한다. 제출 꾸러미(WU-06)에 넣는 일은 이 단위가 아니다.

#### WU-12b 나눈 장 PDF

- 목표: `KaPrintDialog.cpp:193-204`의 「나눈 장을 PDF로 저장…」. 지금은 `dlg_drawing_print_actions.rs:20`에서 `지시 W-50`으로 꺼져 있다. 쪽 나눔은 `print_plan.rs`의 타일 계산을 쓴다.
- 검증: Windows에서 용지보다 큰 도면이 겹침만큼 겹치는 여러 PDF 쪽이 되고, 취소하면 파일이 없다.
- 끝: 단추를 켜고, 그 검사가 통과한다.

#### WU-12c 프린터 여백과 인쇄 영수증

- 목표: 프린터를 모를 때 C++는 사방 5 mm다(`KaPrintDialog.cpp:36`, `:363-378`). Rust 계획(`print_plan.rs`)은 장치 카탈로그의 인쇄 가능 사각형만 쓴다. 그 5 mm와, 고른 Windows 프린터의 최소 여백을 맞춘다.
- 파일: `print_plan.rs`, `print_device.rs`, `scripts/print-device.ps1`.
- 검증: Windows CI 또는 Windows 시험이 카탈로그 없이 5 mm를 쓰고, 기록된 프린터 응답으로는 그 최소 여백을 쓴다. 실제 용지를 뽑는 일은 시험에 넣지 않는다. 영수증의 쪽 수·용지 mm가 요청과 같다(`print_device.rs:47-58`).
- 끝: 그 시험이 통과한다. 미리보기(`dlg_drawing_print_preview.rs`)는 그대로 둔다.

WU-01과 WU-04, WU-05, WU-07은 파일이 겹치지 않으면 동시에 맡길 수 있다. WU-02는 WU-01 다음이 안전하다. WU-03의 도면 키는 WU-02 다음이다. WU-06은 그림 PDF와 벡터 PDF를 한 일로 합치지 않는다. WU-12a가 벡터 PDF다. WU-11은 하지 않는다. 모든 단위의 확인은 Windows CI다.

## 4. 이 목록이 아닌 것

- 11-14 행 전부를 오늘 줄 단위로 다시 읽지는 않았다. `확인=11-14`를 완료 선언으로 쓰지 않는다.
- 스크린샷의 로컬 브랜치 내용은 추정하지 않는다.
- 격자선 지우기와 dotak 저장소는 이 이식의 구멍이 아니다.
- Google Drive의 연구 문서는 이 환경에서 열리지 않았다. 공개 트리에 Drive 본문을 옮기지 않는다.

## 5. 인쇄·조판 대조

2026-11-18. C++ `gogohak1` `b036c09` (`/tmp/cpp-ref`)와 Rust `f1835fa`의 용지·조판·인쇄·PDF. 확인은 Windows CI. 이 절을 Linux에서 돌리거나 고치지 않는다.

| id | 항목 | 상태 | C++ | Rust | 차이 |
|---|---|---|---|---|---|
| PL-01 | A4 세로 210×297 mm | `done` | `KaDrawingStudio.h:58-59` | `sheet_drawing.rs:46-47`, `modal_paper.rs:40-41` | 기본 용지가 같다 |
| PL-02 | A4 가로 현장 용지 317×220 mm | `done` | `KaDrawingStudio.h:60-67`, `KaPaperDialog.cpp:149-168` | `sheet_drawing.rs:48-54`, `preset_dimensions_mm` `:99-103` | 좌·우·위 10 mm. 297×210 레거시는 따로 구분 |
| PL-03 | A3 297×420 mm | `done` | `KaPaperDialog.cpp:162-164` | `sheet_geometry.rs:19`, 인쇄 선택 `dlg_drawing_print_options.rs:41` | 가로면 바꾼다 |
| PL-04 | 방향 (세로/가로) | `done` | `KaPaperDialog.cpp:145-170` | `modal_paper.rs:57-62`, `preset_dimensions_mm` | A4 가로만 현장 용지 |
| PL-05 | 직접 입력 용지 | `partial` | `KaPaperDialog.cpp:146-170` 프리셋 id 2, 스핀 | `modal_paper.rs`와 MENU19 카드 | 창은 있다. 프린터 용지와의 일치는 PL-16 |
| PL-06 | 축척 분모 | `partial` | `TilePrint::scaleLabel`, `StandardScales` | 도면 칩 `gui_tab_drawing_right_b.rs:15`, 목록 `standard_scales.rs:17` | 칩과 표준 목록이 한 집합인지는 이 절에서 재지 않음 |
| PL-07 | 축척 막대 | `partial` | `LayoutService.cpp` 조판 항목 | `sheet_drawing.rs:218`, `gui_tab_drawing_chrome.rs` | 자리와 스타일 단추는 있다. PDF에는 그림으로만 들어간다 |
| PL-08 | 인쇄 미리보기 | `partial` | `KaPrintDialog.cpp:455-456` | `dlg_drawing_print_preview.rs:5-29` | 현재 용지를 축소해 그린다. QPrinter 미리보기는 아니다 |
| PL-09 | 프린터 목록 | `partial` | `KaPrintDialog.cpp:90-96` `QPrinterInfo` | `print_device.rs:8-11`, `print_device_process.rs:8-18` `powershell.exe` | Windows 보조 프로세스. 시험은 빈 카탈로그 |
| PL-10 | 해상도 300 / 600 DPI | `partial` | `TilePrint.cpp:352-353`, 기본 `TilePrint.h:85` 300. 600 문구는 「가는 선이 많은 실측도」 | `print_state.rs:47` 기본 300, `dlg_drawing_print_options.rs:58` 「600 DPI (고정밀 제출용)」 | 두 값은 같다. 600의 문구가 다르다 |
| PL-11 | 여러 장 나누기·겹침·재단선·전체 장 | `partial` | `TilePrint.cpp:37-40`, `:271-290` | `print_plan.rs:92-115`, 기본 겹침 10 mm·재단선·전체 장 `print_state.rs:48-50` | 계산은 있다. 나눈 장 파일은 PL-14 |
| PL-12 | 표제란 | `partial` | `LayoutService.cpp:411-421` 맑은 고딕 15 pt | `gui_tab_drawing_chrome.rs:75-119` | 칸과 줄은 있다. 글꼴은 PL-18 |
| PL-13 | 범례 | `partial` | `LayoutService.cpp` `QgsLayoutItemLegend` | `gui_tab_drawing_legend.rs`, 둥근 테 MENU20 | 화면 범례다. QGIS 범례 렌더는 아니다 |
| PL-14 | 나눈 장 PDF | `missing` | `KaPrintDialog.cpp:193-204`, `:238` `saveTilesAs` | `dlg_drawing_print_actions.rs:19-20` 툴팁 `지시 W-50` | 단추가 꺼져 있다 |
| PL-15 | 도면을 조사 파일에 저장 | `missing` | `KaDrawingStudio.cpp`의 `user_sheet`, `KaDrawingSheetSet` | `gui_tab_drawing_sheets.rs` 세션 메모리. 저장은 GPKG와 `.strata-maps.json` | 다시 열면 조판이 없다. 작업공간 읽기(WU-10)와 겹친다 |
| PL-16 | 여백 | `partial` | 프린터를 모르면 사방 5 mm, 알면 그 최소값 `KaPrintDialog.cpp:36`, `:363-378` | `print_plan.rs`는 장치 카탈로그의 인쇄 가능 사각형만 사용 | 5 mm 기본이 없다 |
| PL-17 | 방위표 | `partial` | `LayoutService.cpp:488-494` PNG `north_arrow` | `gui_tab_drawing_put.rs` 「방위 (도북)」 항목 | 항목은 있다. C++ PNG와 같은 그림인지는 확인하지 않음 |
| PL-18 | 글꼴 | `divergent` | `LayoutService.cpp:243`, `:394` 맑은 고딕 | 조판 글자는 `egui::FontId::proportional`. 맑은 고딕 등록은 지도 라벨 `gui_theme_fonts.rs:13` | 조판 PDF·표제에 그 글꼴이 고정돼 있지 않다 |
| PL-19 | PDF 내보내기 | `divergent` | `KaDrawingStudio.cpp:3819-3855` 조판을 300 DPI로. 글자가 텍스트 | `sheet_pdf_file.rs:10` 300 DPI, `gui_tab_drawing_print.rs:99-121`는 화면 그림. `sheet_pdf_export.rs:27-40`의 빈 네모는 이 단추 경로가 아니다 | 해상도 숫자는 같고 결과물은 그림이다. 큰 용지는 1억 화소 아래로 낮춘다 (`sheet_pdf_file.rs:21-26`) |
| PL-20 | 지도 프레임 | `partial` | `LayoutService` 조판 지도 항목 | `DrawingSheet::map_frame`, 도면 탭이 그린다 | 화면 프레임은 있다. PDF에서는 그림 안에 들어간다 |
| PL-21 | 도면 좌표 격자 기본값 | `divergent` | `KaDrawingStudio.cpp:1553` 근처 사용자 설정 전 꺼짐 | `sheet_drawing.rs:73` `grid_enabled: true` | 처음 용지에 격자가 켜져 있다 |

합계 21행: `done` 4, `partial` 12, `missing` 2, `divergent` 3.

WU-12a가 PL-12·13·17·18·19·20, WU-12b가 PL-14, WU-12c가 PL-16이다. PL-01–04·08·10·11은 다시 만들지 않는다. PL-15는 WU-10에 둔다.
