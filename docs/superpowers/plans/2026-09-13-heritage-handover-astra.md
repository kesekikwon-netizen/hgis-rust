# 주변유적 받기 — 인계 지침 (GPT-6 Astra)

작성 2026-09-13. **확인된 사실**과 **아직 모르는 것**을 구분해 적는다. 추측을 사실처럼 쓰지 않는다.
오늘 같은 자리에서 여러 번 헛돌았다. 그 목록이 6장에 있다 — **먼저 읽어라.**

---

## 0. 그대로 붙여 넣을 머리말

```
D:\hgis 프로젝트다. C++20 / Qt6 / QGIS(OSGeo4W D:\OSGeo4W). 고고학 현장조사 GIS.
지금 하는 일: 리본 「주변유적 받기」 버튼 하나로 국가유산 GIS통합인트라넷에서
조사지역 시/군의 국가유산 SHP 를 받아 지도에 올린다.

먼저 읽어라 (이 순서):
1. docs/superpowers/plans/2026-09-13-heritage-handover-astra.md   ← 이 문서
2. docs/superpowers/plans/2026-09-11-heritage-intranet-nearby-sites.md (설계·색표)

지켜야 할 것:
- 커밋·푸시하지 마라. 포터블 만들지 마라. dist/ 나 L: 에 복사하지 마라. 따로 시킬 때만.
- 미커밋 작업이 많다. 전체 reset·revert·clean 금지. 파일 단위 checkout 도 먼저 물어라.
- 이 자료는 동국문화재연구원의 재산이다. 포터블·제출 SHP 에 절대 싣지 마라.
- 아이디·비밀번호를 소스·문서·로그·화면에 값으로 남기지 마라. 계정은 이미 저장돼 있다.
- **시/군까지만** 받는다. 전국을 받는 경로를 만들지 마라(가드가 있다. 풀지 마라).
- 서약서 동의는 자동으로 하되 시각과 원문을 영수증으로 남겨라.
- 실패를 성공으로 처리하지 마라.

앱 실행은 **반드시** 이렇게 (exe 직접 실행하면 창이 안 뜬다):
  powershell -NoProfile -ExecutionPolicy Bypass -File D:\hgis\scripts\run-ka-hgis.ps1

빌드·검사 (PowerShell):
  $env:PATH = "C:\Program Files\CMake\bin;" + $env:PATH
  . .\scripts\dev-env.ps1
  cmake --build --preset release --target ka-hgis
  build\Release\ka_heritage_flow_tests.exe -o build\qa\hf.txt,txt     # 결과는 -o 로만 보인다
```

---

## 1. 지금 상태 한 줄

### 2026-09-13 최신 검증 — 아래 과거 원인 추정과 구분

현재 상태는 `2026-09-13-heritage-verification.md`와 `.codex/NOW.md`를 우선한다.
**전체 자동화는 미완료다.** 최신 단일 버튼 통합 2회와 별도 브라우저 대조 1회가 첫 파일
수신에서 실패했다. 로그인·동의·포항시 확인·53건 검색·GET 발송은 통과했다.
다운로드 탭 Active/NoProxy 상태로 132초간 HTTP 완료/수신 신호가 없었다.
로컬 MainWindow의 동일 숨은 탭 수신은 419ms에 통과했고 앱 이름별 User-Agent도 같다.
원격 응답 미완결의 원인은 아직 미확정이다. 반복 요청과 추측에 따른 시간 연장은 하지 않는다.
아래 여섯 ZIP·지도/조판 결과는 앞선 분리 검사이며 최신 단일 버튼 성공을 뜻하지 않는다.
사용자 제공 설계 9장의 성공 조건을 기준으로 실제 포항시 **S/P/B/U/R/E 여섯 ZIP** 수신을
194.246초에 확인했다. 같은 ZIP의 실제 MainWindow 적재는 **13 SHP / 3709도형**이다.
SHP/DBF 26개 원본 SHA256 일치, 여섯 종류 지도 화소·참조 그룹·색·실제 유적명 조판 필터를 검사했다.
캡처/실제 자료는 AppData `주변유적-QA/경상북도 포항시/receipts`에만 둔다.
영수증 저장 실패·사유자료 제출 제외·긴 범례 이름의 추가 검사와 최종 Release 결과는 검증 문서에 기록한다.

다운로드 폼의 가시성, 숨겨진 체크상자의 보이는 라벨, 해당 동의 팝업 확인 버튼을 확인해야 한다.
전역 `tabContentAjax` 존재나 숨겨진 오래된 폼을 다운로드 페이지 도착으로 보지 않는다.
지정유산의 실제 탭 코드는 **S**이며 이미 활성화된 유효 폼이면 재요청하지 않는다.
다른 자료 탭은 한 번 요청하고 새 `#searchForm` 교체·mode 일치까지 기다린다.
B/U/R/E는 같은 다운로드 폼 안의 `bjdCd`/`bjdCd1`을 쓴다. 지도 검색 폼의 비슷한 필드와 구분한다.
과다 접속 차단·영속 프로필 오염은 확인된 원인이 아니다. 실제 404/주 화면 실패만 한 번 홈에서 복구한다.
추가 GIS 검증은 받은 원본 파일을 재사용한다. 아래 되는/실패한 다운로드 조합은 유지한다.

**아래 단락은 최신 수정 이전의 08시~10시 이력이다.**

로그인 → 튜토리얼 → 서약서 → 탭 → 시/군 → 검색(8,302→174) → **다운로드까지 한 번 성공**
(2026-09-13 08:53, 지정유산 271KB ZIP, 6종 SHP 포함). 적재·색·범례·그룹은 **실제 ZIP 으로 검사 통과**.
**막힌 곳**: 다운로드가 재현되지 않는다. 그리고 로그인 뒤 화면이 404 로 뜨는 일이 생겼다.

---

## 2. 진단 도구 — 이것부터 써라

앱이 스스로 기록을 남긴다. **화면을 주고받지 말고 이 파일을 읽어라.**

```
%LOCALAPPDATA%\ka-hgis\ka-hgis\주변유적\<시도 시군>\receipts\진단.txt
D:\hgis\build\qa\heritage-debug.txt
```

들어 있는 것: 단계 전환, **모든 스크립트 결과**, 만든 내려받기 주소(전체), 받은 파일과 크기,
사이트가 보낸 요청 목록, 실패 사유. **비밀번호는 안 들어간다**(스크립트 본문은 기록하지 않는다).

받은 HTML 을 못 읽으면 그 HTML 도 `receipts/` 에 저장된다. 추측하지 말고 그 파일을 열어라.

---

## 3. 확인된 사이트 사실 (실제 DOM·JS·요청기록으로 본 것만)

### 3.1 로그인 — `/user/checkNet.do`
`#userId` · `#pwd` 채우고 페이지의 **`goLogin()`** 을 부른다.
비밀번호를 브라우저에서 RSA 로 암호화한다. **평문 POST 는 로그인되지 않는다.**

### 3.2 로그인 후
루트가 **frameset** 이다:
```html
<frameset rows="0%,100%"><frame name="top"><frame src="/user/checkNet.do" name="main"></frameset>
```
`top` 은 비어 있다. **프레임 안으로 이동하지 마라** — 빈 top 으로 가면 하얀 화면이 된다.
안쪽 문서는 `cands()` 로 이동 없이 읽는다.

튜토리얼이 매번 뜬다. 「다시 보지 않기」는 **체크상자만** 눌러라 —
감싼 요소를 누르면 그 안의 「다음」이 눌려 1/5 → 5/5 로 넘어간다.

### 3.3 엔드포인트 (사이트 JS 에서 읽음)

| 동작 | 값 |
|---|---|
| 다운로드 화면 진입 | `showAgreePopup()` |
| 목록·검색 | `POST /user/data/heritageDownloadList.do` (`tabContentAjax` 로 부른다) |
| 결과가 들어가는 곳 | `#tabContentDiv` |
| 전체다운로드 | `GET /user/data/heritageDownload/downloadFilesAll.do?…` |
| 행별 다운로드 | `downloadFiles('one','<번호>,CHL_SPCN_AS,<코드>','<코드>')` |

**우리 쪽에서 직접 POST 하면 404 다.** `X-Requested-With` 를 붙여도 마찬가지.
그래서 요청은 브라우저(사이트 자신의 ajax)가 보내게 한다.

### 3.4 함수 이름 충돌 — 중요

`searchGisChaRirList` 는 **이름만 같은 함수가 둘** 있다.
지도(`/js/ngis/bury/select.js:1067`)의 「군부대유산조사」용이 나중에 로드되며 덮어쓴다.
그 함수는 `document.getElementById("searchHeName").value` 를 읽어
`TypeError: Cannot read properties of null (reading 'value')` 로 터진다.
→ **사이트 함수를 부르지 말고 주소를 만들어 받는다.**

### 3.5 폼 칸 — 두 벌이 있다. 틀리면 전국을 받는다

| id / name | 정체 |
|---|---|
| `bjdcd1_newJbn` / `bjdCdNewAddr` | 왼쪽 **지도 패널**. 여기가 아니다 |
| `bjdcd2_newJbn` / `bjdcd_newJbn` | 지도 패널 짝 |
| **`codedetaCd0` / `codedetaCd`** | **다운로드 폼 시/도** |
| **`codeCdSg0` / `codeCdSg`** | **다운로드 폼 시/군/구** |
| **`bjdCd` / `bjdCd1`** | **B/U/R/E 다운로드 폼의 시/도 및 시/군. 같은 이름의 지도 폼과 구분** |
| `pageUnit0` / `pageUnit` | 「10개씩 보기」. **건드리지 마라** — 목록이 다시 그려져 깨진다 |
| `#searchForm` | 검색 폼(사이트 JS 가 `$("#searchForm").serialize()` 로 쓴다) |

「검색」 버튼도 지도 패널에 하나 더 있다. **`#searchForm` 안에서만** 찾아라.

### 3.6 탭 코드
매장유산유존지역 `B` · 문화유적분포지도 `U` · 지표조사구역 `R` · 발굴조사구역 `E` ·
현상변경허용기준 `P` · **지정유산 `S`**(2026-09-13 실제 `tabS`/`changeTab('S')` 확인).

탭 3~6(매장유산·분포지도·지표조사·발굴조사)은 **폼이 다르다**(읍/면/동·리 칸이 더 있다).
2026-09-13 포항시에서 그 탭들의 시군 선택·검색·전체 다운로드도 확인했다.

### 3.7 시/도 목록 (사이트 실제 값)
서울특별시·부산광역시·대구광역시·인천광역시·세종특별자치시·대전광역시·울산광역시·경기도·
충청북도·충청남도·경상북도·경상남도·제주특별자치도·**강원특별자치도**·**전북특별자치도**·
**전남광주통합특별시**

앱의 `KoreaRegionCatalog` 는 전라남도/광주광역시가 따로다 → 별칭 처리해 두었다.

### 3.8 받은 자료
`지정유산_<타임스탬프>.zip` (제주시 기준 264~271KB). 안에 6종 SHP + `.prj`:
국가지정유산 · 시도지정유산 · 국가등록문화유산 · 시도등록문화유산 ·
국가지정유산보호구역 · 시도지정유산보호구역. 좌표계 **EPSG:5179**.

**ZIP 안의 파일 이름이 CP949 다.** GDAL `/vsizip/` 은 UTF-8 로 읽어 이름이 깨진다
(`국가지정유산` → `?├??┴÷┴n└≫?Ω`). 풀 때만 `CPL_ZIP_ENCODING=CP949` 를 걸어야 한다
(`HeritageImport::loadDataset` 에 되어 있다).

---

## 4. **되는 조합** — 여기서 벗어난 변경은 전부 실패했다

2026-09-13 08:53 에 실제로 271KB 가 온 유일한 조합:

| 항목 | 값 |
|---|---|
| 주소 | 우리가 만든다. 사이트 함수 호출 ✗ |
| 인코딩 | **전부 퍼센트 인코딩** (`%3A`·`%2F`·`%20` 포함). JS 에서는 인코딩하지 않고 **날것 JSON** 으로 넘겨 C++ 이 한 번만 한다 |
| 받는 곳 | **새 탭**(`addPage()` → `load(url)`). 받은 뒤 그 탭을 닫는다 |
| 기다림 | 시간이 아니라 **완료 신호**(`m_pendingDownloads`, `isFinishedChanged`) |
| 검증 | 파일이 **0바이트가 아닌지** 확인. 0바이트면 버린다 |

### 실패한 조합 (다시 시도하지 마라)

| 시도 | 결과 |
|---|---|
| JS 에서 `encodeURIComponent` | `%2525EC…` 삼중 인코딩 → 0바이트 |
| `QUrl(문자열)` 로 조립 | `%25EC…` 이중 인코딩 → 0바이트 |
| `QUrl::fromEncoded(…, StrictMode)` | 주소가 **무효** → `load()` 가 조용히 아무것도 안 함 |
| `QUrlQuery` + `setQuery(QUrlQuery)` | Qt 가 인코딩을 **풀어** 한글·공백이 날것으로 나감 |
| `searchParams` 만 `:`·`/` 를 남김 | 0바이트 (성공 사례는 둘 다 인코딩돼 있었다) |
| 본 창(`m_mainView`)에서 받기 | 서버가 404 를 주면 **작업 화면이 404 로 덮인다** |
| 숨은 전용 창에서 받기 | 0바이트 |
| 「10개씩 보기」를 최대로 변경 | 목록이 다시 그려져 다운로드 함수가 터짐 |

---

## 5. 과거 막힘 기록 — 최신 상태는 1장과 검증 문서

아래 프로필 오염 진단은 당시 추정이다. 이후 확인한 DOM/요청 상태 오류와 혼동하지 말 것.
영속 프로필을 지우는 조치는 이번 수정에서 하지 않았다.

**증상**: 로그인은 되는데(`submitted` → `past-login`) 그 뒤 화면이 **404**(「찾을 수 없는 페이지」)라
`showAgreePopup` 도 메뉴 링크도 없어 `not-found` 만 반복한다.

**원인 추정**: `QWebEngineProfile("ka-heritage")` 가 **영속 프로필**이라,
이전 실험에서 본 창을 파일 주소로 보냈다가 받은 404 상태로 되돌아간다.

**넣어 둔 조치(미검증)**: `OpenDownloadPage` 에서 8틱(약 6초) 넘게 못 찾으면
**한 번만** `/ngis/common/main.do?httpYn=Y` 로 되돌린다(`m_homeRecovered`).

**먼저 해 볼 것**
1. 앱을 새로 띄우고 로그를 본다. 「화면이 예상과 달라 홈으로 되돌립니다: <주소>」 가 찍히는지
2. 그래도 404 면 **프로필을 비우고** 시작하는 길을 검토한다
   (`%LOCALAPPDATA%\ka-hgis\ka-hgis\QtWebEngine\ka-heritage` 부근). 사용자에게 먼저 알려라
3. 404 가 사라지면 4장의 「되는 조합」으로 다운로드가 재현되는지 확인한다

---

## 6. 오늘 실제로 당한 함정 (전부 재현됨)

1. **`ka-hgis.exe` 직접 실행하면 창이 안 뜬다.** 반드시 `scripts\run-ka-hgis.ps1`
2. **QTest 결과는 `-o <파일>,txt` 로만 보인다.** stdout 리다이렉트로는 안 나온다
3. **C++ 문자열 안의 `\s` 는 잘못된 이스케이프** → `s` 가 되어 정규식이 망가진다. `\\s` 로 써라
4. **bash heredoc 이 `\\` 를 `\` 로 줄인다.** 파이썬으로 C++ 을 쓸 때 `chr(92)` 로 역슬래시를 만들어라
5. **소스는 CRLF.** 파이썬은 `newline=''` 로 읽고 써라
6. **인덱스로 코드를 잘라 붙이지 마라.** 오늘 `runStage()` 함수를 통째로 날렸다.
   반드시 **유일한 앵커 문자열**로 치환하고, 치환 뒤 함수 개수를 확인하라
7. **`createWindow` 없으면 사이트 팝업이 사라진다**
8. **요소가 닫혀도 DOM 에 숨은 채 남는다.** 보이는 것만 세라
9. **한 틱에 스크립트 두 번 돌리면 뒤엣것이 버려진다**(`m_scriptInFlight`).
   틱마다 반복 검사를 넣으면 본 작업이 영영 안 돈다 — 로그인이 이것 때문에 멈췄다
10. **OneDrive 폴더에 받지 마라.** 동기화가 가로채 0바이트로 보인다. 지금은 `%LOCALAPPDATA%` 에 받는다
11. **WebEngine 은 `--no-sandbox` 가 없으면 모든 로드가 실패한다**(`applyWebEngineFlags`)
12. **탭을 바꾼 뒤 바로 값을 넣지 마라.** 폼이 ajax 로 다시 그려진다.
    딜레이(`m_settle`)를 주고, 고른 뒤 **다시 확인**하라(`verifyRegionScript`)

이번 후속 수정에서 추가로 재현한 함정:

- 보이지 않는 공식 체크상자는 연결된 보이는 라벨로 판별한다. 반대로 숨겨진 팝업 전체를 유효하다고 보지 않는다.
- 숨긴 네이티브 WebEngine view는 DOM 크기가 0일 수 있다. 브라우저 창을 숨겨도 CSS상 활성 폼은 탐색해야 한다.
- 검색 폼 종류에 따라 시군 필드 이름이 다르다. 지정유산 안의 6개 SHP와 사이트의 여섯 자료 종류는 다른 개념이다.
- 영수증 저장·QGIS 적재에서 실패한 뒤 다음 단계나 최종 완료 신호를 내보내면 안 된다.
- 범례 이름 dump 통과만으로 PDF가 맞는 것은 아니다. 실제 E PDF는 원문이 있지만 폭 밖에서 잘렸고, 자동 줄바꿈으로 수정했다.
- QGIS 자동 줄바꿈은 범례 전체 속성이다. 주변유적을 끈 뒤에는 원래 설정을 복원해야 다른 범례에 영향이 남지 않는다.
- 참조 레이어를 `feature_poly`로 이름만 바꿔도 과거 이름 폴백이 제출 SHP로 잡았다. 표시 이름이 아니라 명시 참조 역할로 제외한다.

---

## 7. 만들어 둔 것

| 파일 | 하는 일 |
|---|---|
| `src/core/HeritageStyle.{h,cpp}` | 여섯 종 색·이름·탭코드 표(SSOT), 유적명 카테고리 렌더러 |
| `src/core/HeritageSiteLegend.{h,cpp}` | 레이어창엔 종류 한 줄, 도면 범례엔 유적명 전부 |
| `src/core/HeritageRegionResolver.{h,cpp}` | 조사구역 → 시/군 |
| `src/core/HeritageIntranetSettings.{h,cpp}` | 인트라넷 계정(NGII 와 다른 자리) |
| `src/core/HeritageIntranetFlow.{h,cpp}` | 단계와 스크립트 전부 |
| `src/core/HeritageFormParser.{h,cpp}` | HTML 에서 검색 폼 읽기 |
| `src/core/HeritageHttpClient.{h,cpp}` | HTTP 직접 경로(지금은 404 로 막혀 미사용) |
| `src/core/HeritageImport.{h,cpp}` | ZIP → 레이어 + 색 + 범례 + 「참조 지도 › 종류」 그룹 |
| `src/app/KaHeritageBrowser.{h,cpp}` | 창·단계 진행·다운로드·진단 기록 |
| `MainWindow::fetchNearbyHeritage` | 리본 「주변유적 받기」 버튼 |

이전 인계 시점의 검사 수(최신 결과는 검증 문서): `test_heritage_flow`(30) · `test_heritage_form`(14) · `test_heritage_style`(13) ·
`test_heritage_region`(14) · `test_heritage_import`(4, **실제 10MB ZIP 으로**) · `test_webengine_smoke`(2)

`test_heritage_import` 는 `build/qa/heritage-sample/지정유산.zip` 이 있어야 돈다(없으면 건너뛴다).
**리포에 자료를 넣지 마라.**

---

## 8. 현재 완료 판정과 재현 범위

- **미완료:** 최신 단일 버튼 실행은 첫 다운로드 응답을 받지 못했다. 이전에 성공한 별도 브라우저 경로도 지금은 같은 단계에서 실패한다. `heritage-oneclick-diagnostic-live.txt`와 `heritage-control-designated.txt`를 먼저 읽는다. 앞의 지도·조판 검사 통과를 전체 성공으로 바꾸어 보고하지 않는다.
- 사용자 제공 설계 9장과 `2026-09-13-heritage-verification.md`의 실제 결과를 대조한다. 일부 단위 검사만으로 끝났다고 보고하지 않는다.
- 포항시 여섯 자료 전체 수신, 실제 DBF 이름/원본 보존, 지도·조판·실제 PDF, 저장/적재 실패, 제출 SHP 제외의 확인 결과가 있다.
- 검색 목록의 건수와 SHP 도형 수를 같은 단위로 취급하지 않는다. B/U는 전체 검색 수와 실제 도형 수가 같고 S의 고유 유산코드 수는 검색 수와 같다. P/R/E는 단순 수치 비교로 누락 여부를 단정하지 않는다.
- 안동시나 모든 시군에서 실서버 재현했다고 확대 해석하지 않는다. 최종 Release 검사 결과와 실행파일 해시는 검증 문서에 기록한다.
- 저장 위치는 현재 로컬 AppData다. 별도 요청 없이 원본을 조사폴더·저장소·포터블로 옮기지 않는다.
