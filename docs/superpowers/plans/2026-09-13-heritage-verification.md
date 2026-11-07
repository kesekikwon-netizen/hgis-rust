## 2026-09-13 PID 22940 복원 — 최신 작업

사용자가 오전 08:53의 「새 빌드 (PID 22940)」로 복원을 명시 요청했다.
Claude 대화의 2026-09-12T23:52:38Z 실행과 23:53:12Z 보고를 식별하고,
해당 시점의 파일 이력으로 주변유적 Browser/Flow/Import 및 당시 검사를 복원했다.
MainWindow는 당시 AppData 저장 경로와 지정유산 한 종류 대상을 재구성했다.
현재본과 실행파일 백업: `build/recovery/pid22940-before-restore-20260913-121846/before/`.
복원 원본·SHA256·파일별 근거: 같은 폴더의 `manifest.json`과 `target/`.
이후 추가한 브라우저/단일버튼 검사 코드는 백업에 보존하고, 빌드는 당시 API의 검사 구성으로 복원했다.
다른 기능 및 사유 자료의 제출 제외·계정 로그 비노출 변경은 보존했다.
이번 작업은 과거 코드 복원이며, 원격 다운로드 성공을 새로 확인했다는 뜻이 아니다.
복원 Release 빌드 완료. 사용자 지시로 진행 중이던 자동 검사를 중단했으며 전체 통과로 보고하지 않는다. 2026-09-13 12:22에 정상 실행 스크립트로 앱을 열었고 PID 37328의 실제 창을 확인했다. 이후 수정 시 자동 검사를 사용자보다 먼저 강행하지 말고, 빌드 후 앱을 바로 열어 사용자가 확인하도록 한다.
커밋·푸시·포터블·dist/L 복사 및 실제 조사/유산 자료 변경 없음.

# 주변유적 받기 — 사용자 성공 조건 대조

기준은 사용자가 제공한 `2026-09-11-heritage-intranet-nearby-sites.md` 9장이다.
단위 테스트 통과나 ZIP 한 개 수신을 전체 완료로 취급하지 않는다.
이번 실제 지역은 최신 실패 화면과 같은 **경상북도 포항시**다. 과거 계획의 안동시를 검사했다고 쓰지 않는다.

**현재 판정: 전체 자동화 미완료.** 아래 앞선 분리 검사에는 실제 여섯 ZIP과 지도/조판 증거가
있지만, 최신 단일 버튼 실행은 첫 다운로드 응답을 받지 못해 실패했다. 빌드/회귀 통과를
실서버 자동화 성공으로 바꾸어 보고하지 않는다.

## 실제 자료 결과

2026-09-13 별도 QA 브라우저 프로필에서 로그인·동의·시/군 검색 후 여섯 종류를 모두 받았다.
6개 ZIP 수신 완료까지 **194.246초**, 실패 신호 0회였다.
이어서 같은 원본 ZIP을 실제 `MainWindow`의 `datasetReady` 연결로 전달해 지도 적재와 렌더링을 검증했다.
이 검사는 사용자 실행 창을 조작한 검사가 아니며, 현재 사용자 조사 원본을 수정하지 않았다.

**후속 단일 버튼 통합 검사(11:20~11:22)는 실패했다.** 실제 MainWindow 리본 클릭 1회와
포항시 확인 1회, 로그인·동의·53건 검색은 통과했지만 첫 다운로드 요청 후 약 42초의
시작 대기 제한 안에 파일 수신 시작/탭 응답 신호가 오지 않았다. 105.656초에 2 passed /
1 failed로 종료됐으며 받은 자료는 0종이다. 따라서 앞의 분리 검사만으로 전체 완료를
판정할 수 없다. 성공 요청과 세션값을 제외한 14개 파라미터는 인코딩까지 동일하고 실제
GET 발송도 확인했다.

**11:29~11:33 진단 실행도 실패했다.** 실패 후 새 요청 없이 90초를 더 관측했지만,
최초 GET 이후 총 132초 동안 파일 수신/HTTP 완료 신호가 없었다. 다운로드 페이지는
`Active`, `isLoading=true`였고 프록시는 `NoProxy`였다. 동결·폐기 전이는 없었다.
전체 203.671초, 완료 0회·적재 0종이다. 시간 제한을 조금 늘리거나 탭을 보이게 하는
제품 변경을 원인 확인 없이 적용하지 않았다. QA 계정/API 설정 복사본은 삭제했다.

**후속 대조:** 로컬 MainWindow/QGIS 숨은 실제 탭은 HTTP attachment 32,772바이트를
419ms에 완전히 수신했다. QA 앱 이름 변경 전후 WebEngine User-Agent는 같았다.
이 로컬 검사 5개가 통과했고 원격 요청은 0회였다. 이전에 성공한 별도 QApplication
브라우저 경로로 지정유산 1종만 대조한 실행도 105.052초에 같은 다운로드 시작 실패로
끝났다(2 passed / 1 failed). 통합 초기화만의 차단이나 단순 42초 초과 지연으로는 설명되지
않는다. 현재 원격 다운로드 응답이 끝나지 않는 원인은 아직 확정하지 못했으며 자동
재접속·추가 다운로드는 중단했다.

| 자료 | 코드 | 서버 검색 건수 | ZIP bytes | SHP | 도형 | 고정 색 |
|---|---|---:|---:|---:|---:|---|
| 지정유산 | S | 53 | 50,481 | 6 | 88 | #8E44AD |
| 현상변경허용기준 | P | 50 | 330,889 | 1 | 150 | #E67E22 |
| 매장유산유존지역 | B | 451 | 1,189,568 | 1 | 451 | #2E86C1 |
| 문화유적분포지도 | U | 1,584 | 4,516,396 | 1 | 1,584 | #27AE60 |
| 지표조사구역 | R | 228 | 2,027,445 | 2 | 961 | #16A085 |
| 발굴조사구역 | E | 192 | 558,262 | 2 | 475 | #A0522D |
| 합계 | | | 8,673,041 | 13 | 3,709 | |

서버 검색 건수는 유산/사업 목록이고 SHP 개수·도형 개수와 같은 단위가 아니다.
S의 고유 유산코드 53개는 검색 건수와 같고, B/U의 도형 수는 각각 전체 검색 건수와 같다.
P/R/E의 식별자와 검색 목록 단위가 다르므로 단순 개수 비교로 누락 없음 또는 누락을 단정하지 않는다.
실제 요청은 시/군 전체 다운로드이며 첫 화면의 10개 목록만 받은 것이 아니다.

## 조건별 검증

| 사용자 조건 | 확인한 결과와 근거 |
|---|---|
| 시/군까지만 검색 | 실제 S/P의 시도·시군 값, B/U/R/E의 `bjdCd=47`, `bjdCd1=4711` 확인. 읍면동·리 비선택. 전국 요청 가드와 잘못된 폼·동명이 포함된 구 선택의 회귀 검사 |
| 로그인부터 실제 파일 수신 | 앞선 분리 검사 `heritage-live-six-datasets.txt`는 여섯 종류 완료. 최신 통합 2회와 별도 브라우저 대조는 첫 수신 단계 실패. 전체 성공으로 판정하지 않음 |
| 버튼 한 번으로 지도 적재까지 | 실제 버튼 1회·포항시 확인 1회·로그인·동의·53건 검색·GET 발송 확인. 이후 수신 신호 없음, 완료 0회/적재 0종으로 **미충족** |
| 원본 보존·속성 유지 | 독립 압축 해제본과 SHP 13개 + DBF 13개의 SHA256 일치. ZIP 6개 SHA256 불변. 실제 필드·피처 수·표본 속성·WKB 대조 |
| 조사 데이터와 분리 | 실제 MainWindow에서 참조 지도 아래 여섯 그룹, 읽기 전용. 기존 조사 레이어·조사 파일 bytes·프로젝트 CRS 보존 |
| 좌표계 | 원본 13개 SHP 모두 EPSG:5179, 검증 프로젝트/캔버스 EPSG:5187. 기존 Qgs 좌표변환과 QGIS 렌더러 사용 |
| 실제 지도 색 | 여섯 종류 각각 지도 렌더 화소 확인. 기대 색 근처 화소 수 S/P/B/U/R/E = 1205 / 785 / 897 / 326 / 18989 / 300 (RGB 성분 허용차 10) |
| 레이어창은 종류만 | 기존 패널 범례 한 노드 검사 및 실제 여섯 종류 그룹 캡처. 같은 레이어의 도면 범례에는 실제 이름 사용 |
| 도면 범례에 유적명 | 실제 자료마다 `LayoutService::tuneSheetLegend`를 거쳐 `sheetLegendLabelDump`에 인쇄 범위 안 실제 유적명이 포함되는지 확인 |
| 범위 밖 유적 제외·범례 색 | 같은 실제 레이어에서 멀리 있는 다른 이름이 범위 밖이면 범례에서 빠짐. 6종 모두 지도와 범례의 기대 색 화소 확인. PNG/PDF 출력 |
| 빈 이름·범례 상한 | 기존 heritage_style 회귀 검사에서 무명 도형의 렌더 카테고리 및 2000개 상한 안내 확인 |
| 실패를 완료로 표시하지 않음 | 깨진 ZIP/SHP, ZIP 안 SHP 없음, DBF 없음, 적재 중 조사 닫힘 등을 실패로 유지. 한 묶음 일부만 성공한 경우 준비 레이어 등록 전 전부 실패 처리 |
| 서약서 영수증 | 실제 동의 팝업의 체크상자에 연결된 보이는 라벨·해당 팝업 확인 버튼을 사용. 클릭 전에 원문 확보. MainWindow 연결에서 시각·원문 저장 확인. 디렉터리/최종 파일 저장 실패를 재현하고 실패 상태 유지 검사 |
| 제출 SHP 제외 | 실제 HeritageImport로 준비한 합성 6종을 ExportService로 제출해 조사구역 SHP 1개만 생성되는지 확인. 참조 레이어 이름을 feature_poly로 바꾼 누출을 RED로 재현한 뒤 명시 참조 역할 제외로 GREEN |
| 포터블 제외 | 실행 없이 복사 원본·스크립트 검사: AppData 주변유적/국가유산 계정 미복사, data 10개/docs/user 9개 파일에 사유 공간자료 후보 없음, 두 스크립트 구문 오류 없음. 기존 모든 dist 내용을 검사한 것은 아니며 포터블 제작·복사 실행 없음 |
| 긴 유적명 전체 표시 | E의 같은 실제 이름을 유지한 채 QGIS 자동 줄바꿈. 필요 폭 144.403→78.8072mm, PNG 오른쪽 여백 침범 383→0화소, 실제 PDF를 Poppler로 다시 렌더한 결과 382→0화소. 나머지 5종도 원문·범위 필터·색·범례 폭 통과 |
| 범례 설정 회귀 방지 | 주변유적 해제 후 원래 자동 줄바꿈 0mm/24mm가 복원되는 두 경우 RED→GREEN. 독립 코드 검토에서 발견한 범례 전체 설정의 잔류 문제를 수정 |

## 수정한 원인

- 다운로드 화면에 전역 `tabContentAjax` 함수나 숨겨진 폼이 남아 있어도 도착했다고 보지 않는다. 보이는 유효 다운로드 폼 또는 실제 동의 팝업을 확인한다.
- 동의 체크상자 자체는 공식 사이트에서 숨겨져 있고 라벨만 보인다. 해당 라벨과 팝업 안 확인 버튼을 함께 찾는다. 페이지의 다른 확인 버튼을 누르지 않는다.
- 지정유산의 실제 탭 코드는 **S**다. 이미 활성화된 올바른 폼이면 재요청하지 않고, 다른 탭이면 한 번 요청한 뒤 새 폼 교체를 기다린다.
- B/U/R/E의 지역 필드는 S/P와 다르다. 실제 다운로드 폼 안에서 종류별 필드를 선택한다. `포항시`를 `포항시 남구`와 부분문자열로 혼동하지 않는다.
- AJAX 요청을 기다리는 동안 다시 진입/탭 요청을 보내지 않는다. change 이벤트 후 동일 핸들러를 다시 직접 호출하던 중복도 제거했다.
- 내부 404/주 화면 로드 실패는 한 번만 홈에서 복구한다. 폼을 못 찾았다는 이유로 프레임 URL로 이동하지 않는다. 재실패는 중단한다.
- 다운로드 URL의 한 번 퍼센트 인코딩 + 새 탭 + 실제 파일 완료 신호 조합은 유지했다.
- ZIP 수신과 QGIS 적재는 별도 성공 조건이다. 적재 실패를 브라우저에 돌려주어 마지막 완료 신호가 덮어쓰지 못하게 했다.
- 동의 영수증은 QSaveFile의 실제 기록·commit을 확인한다. 실패 신호 뒤 자료 선택으로 상태가 바뀌던 경로도 중단했다.
- 참조 자료의 표시 이름을 조사 도메인 이름으로 바꿔도 제출 SHP에 포함되지 않도록 명시 역할을 확인한다.
- 긴 이름은 원문을 잘라내지 않고 현재 범례 폭에 맞게 QGIS API로 줄바꿈한다. 설치 SDK `qgslayoutitemlegend.h`와 [QGIS 3.44 공식 범례 문서](https://docs.qgis.org/3.44/en/docs/user_manual/print_layout/layout_items/layout_legend.html#word-wrapping)의 mm 단위 자동 줄바꿈을 확인했다. 로컬 매뉴얼 디렉터리에는 해당 Desktop User Guide PDF가 없고 Cookbook만 있어 공식 문서를 보완 근거로 썼다.

과다 접속 차단 또는 영속 프로필 오염을 원인으로 확정할 근거는 없다.
현재 재현한 원인은 위 DOM 선택·요청 상태 관리 오류다. 후속 GIS 검증에는 이미 받은 파일만 재사용했다.

## 증거 위치

원본·유적명이 들어간 캡처·검증 PDF는 저장소에 복사하지 않고 다음 로컬 비공개 위치에 보관한다.

`%LOCALAPPDATA%/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/`

- `qa-manifest.json`: 여섯 자료 원본 ZIP 위치
- `qa-six-datasets-window.png`: 실제 MainWindow 레이어창과 지도
- `qa-dataset-0-map.png` ~ `qa-dataset-5-map.png`: 종류별 지도
- `qa-dataset-0-layout-legend.{png,pdf,json}` ~ `qa-dataset-5-layout-legend.{png,pdf,json}`: 실제 조판 화소, PDF 및 범례 대조
- `qa-dataset-5-layout-legend-before.{png,pdf}` 및 `qa-legend-wrap-comparison.json`: 긴 이름의 잘림 전후

로컬 캡처 바로 열기:

- [여섯 종류가 적재된 앱 화면](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-six-datasets-window.png>)
- [긴 사업명 수정 전](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-5-layout-legend-before.png>) · [수정 후 실제 PDF 렌더](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-5-layout-legend-pdf-render.png>)

| 조판 | 실제 PNG | 실제 PDF |
|---|---|---|
| 지정유산 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-0-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-0-layout-legend.pdf>) |
| 현상변경허용기준 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-1-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-1-layout-legend.pdf>) |
| 매장유산유존지역 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-2-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-2-layout-legend.pdf>) |
| 문화유적분포지도 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-3-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-3-layout-legend.pdf>) |
| 지표조사구역 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-4-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-4-layout-legend.pdf>) |
| 발굴조사구역 | [보기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-5-layout-legend.png>) | [열기](<C:/Users/kyi25/AppData/Local/ka-hgis/ka-hgis/주변유적-QA/경상북도 포항시/receipts/qa-dataset-5-layout-legend.pdf>) |

저장소 `build/qa/`에는 검사 결과와 비밀 없는 회귀 로그를 둔다. 계정 값을 보고서에 남기지 않는다.

## 수정 파일

| 파일 | 이번 수정 |
|---|---|
| src/core/HeritageIntranetFlow.cpp | 실제 폼·동의 UI·자료별 지역 필드 판별, 시군 정확 일치, 스크립트 중복 호출 제거 |
| src/core/HeritageStyle.cpp/.h | 확인된 지정유산 탭 코드 S |
| src/app/KaHeritageBrowser.cpp/.h | 404 1회 복구, 요청/응답 대기 구분, 영수증·적재 실패 상태 보존, 실제 검색 건수 안내 |
| src/app/MainWindow.cpp/.h | 적재·동의 영수증 저장 실패 전파, 영수증 원자적 저장 |
| src/core/HeritageImport.cpp/.h | 묶음 준비 실패 처리, SHP/DBF 누락 확인, 레이어 소유권과 등록의 실패 정리 |
| src/core/HeritageIntranetSettings.cpp | 진단용 계정 문자열은 값 대신 설정 상태만 반환 |
| src/core/ExportService.cpp | 표시 이름에 관계없이 명시 참조 자료의 제출 SHP 제외 |
| src/core/LayoutService.cpp | 주변유적이 있는 범례의 긴 이름 줄바꿈 |
| tests/test_heritage_browser.cpp, test_heritage_flow.cpp, test_heritage_import.cpp | 요청 상태·숨긴 폼·동의·시군·손상 파일·실서버 선택 실행 회귀 |
| tests/test_save_open.cpp, test_workflow.cpp, test_heritage_style.cpp | 실제 MainWindow 연결, 원본 bytes·지도·조판·영수증·제출 결과 검사 |

기존 미커밋 파일 전체를 이번 수정으로 주장하지 않는다. HTTP 직접 경로 등 이전 작업도 보존했다.

## 최종 검사 기록

| 집중 검사 로그 (build/qa/) | 결과 |
|---|---|
| heritage-live-six-datasets.txt | 실제 서버 6종: 3 passed / 0 failed / 0 skipped, 194.246초 |
| heritage-browser-acceptance.txt | 22 passed / 0 failed / 3 skipped. 실서버 선택 실행 3개는 평시 건너뜀 |
| heritage-source-completeness-green.txt | 13 passed / 0 failed / 1 skipped. 손상·누락 파일의 부분 성공 금지 |
| heritage-receipt-green.txt | 9 passed / 0 failed. 정상/실패 영수증, 적재 실패, 실제 6종 지도·조판 |
| heritage-private-export-green.txt | 4 passed / 0 failed. 실제 제출 패키지에서 참조 자료 제외 |
| heritage-legend-wrap-green.txt | 실제 6종: 3 passed / 0 failed, 18.345초. 이름·색·범위·필요 크기·여백 |
| heritage-legend-toggle-green.txt | 4 passed / 0 failed. 긴 합성 이름·원문 보존·이전 줄바꿈 복원 |
| heritage-export-portable-audit.txt | 포터블 스크립트 실행 없이 구문·복사 원본/범위 확인 |

영수증 저장 실패와 제출 제외의 추가 회귀 확인 후 최신 Release 검사 결과를 기록한다.
앞선 Release 전체 검사: 43개 통과, 기존 topographic_browser 1개 비활성, 221.72초.
앞선 시작 검사 종료 코드 0. 이 결과를 추가 수정 후 바이너리의 검사로 혼동하지 않는다.

바탕 화면 `고고학 전용 HGIS.lnk` → `scripts/start-ka-hgis.vbs` → `launch.ps1` → `build/Release/ka-hgis.exe` 연결을 확인했다.
커밋·푸시·포터블 제작·dist/L: 복사·사용자 앱 종료/조작은 하지 않았다. 기존 미커밋 변경을 보존했다.
