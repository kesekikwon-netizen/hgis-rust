# 요청 기록: 단면도(단면도 만들기 화면) 기능 삭제

- 날짜: 2026-10-10
- 상태: 확정
- 관련 문서: docs/intent/2026-10-10-monitor-3440x1440-fit.md(단면도는 화면 맞춤에서 제외) · docs/archive/2026-08-22-section-layout-studio-design.md(없어지는 기능의 설계)

## 요청 원문
> 여기 단면도는 안하는게 좋겠다.
>
> 단면도는 gis에서 제외하라.

(「어떻게 제외할까요」 질문에 「단면도 기능 완전히 삭제」를 골랐다.)

## 확인한 사실
- 단면도는 리본 「단면도」 → 「단면도」 탭(KaSectionDrawingStudio)에서 단면 GeoTIFF를 올려 용지(section_sheet)를 만들고, 제출 꾸러미에 단면도.pdf를 넣는 기능이다. 코드: src/app/KaSectionDrawingStudio.*, src/core/SectionLayoutService.*, src/core/SectionSheetDecor.* 와 시험 section_layout_engine, section_studio_engine.
- 지도에 그리는 조사 레이어 「단면선」(section_line, 제출 section_line.shp)은 이 화면과 따로 동작한다.
- 입체지형은 단면도와 따로다.

## 정한 것 (2026-10-10, 모두 추천 답)
1. 단면도 화면·단면 GeoTIFF·단면도.pdf·단면도 전용 코드와 시험을 지운다.
2. 「단면선」 그리기와 section_line.shp 는 그대로 둔다.
3. 검수의 층위도 항목(REQ_DRAWING_SECTION)과 「단면도 열기」 단추를 없앤다. 필수 도면 목록은 4종(조사구역도·유적위치도·유구배치도·개별유구실측도).
4. 예전 조사 파일에 남은 section_sheet 용지와 숨은 단면 GeoTIFF 레이어(ka_hgis/section_raster)는 그대로 둔다(원본 보호). 앱은 그 용지를 무시하고, 입체지형이 DEM을 고를 때 단면 GeoTIFF는 셈에서 뺀다.
5. 도면 틀 목록의 「층위도」 자동 틀(LayoutService DrawingKind::Section)도 지운다(추가 질문에 추천 답). 틀은 5종에서 4종. 예전 조사 파일의 층위도 틀은 그대로 남는다.

## 설계 (경로: architectural, 한 번에 지우고 빌드·시험으로 확인)
- 진입점(리본 단추, 탭, MainWindow 멤버·함수, 세션 복원·탭 닫기, 검수 동작 open_section)을 먼저 없애고, 그다음 전용 파일·CMake 목록·시험 대상·길이 기준 줄을 지운다.
- ExportService 의 단면도.pdf 단계와 SubmitReadme 의 hasSectionPdf, ProjectStateBuilder 의 layout_exists:section(층위도 규칙과 함께)을 지운다. LayoutService 의 층위도 자동 틀(section)도 검수 항목이 없어지면 쓸모가 없으니 함께 본다.
- 바뀌는 시험(시험 변경): 층위도 규칙·open_section 을 세는 검수 시험, 단면도 꾸러미를 만드는 workflow 시험.
