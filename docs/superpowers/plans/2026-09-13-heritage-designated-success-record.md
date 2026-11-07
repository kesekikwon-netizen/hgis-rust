# 지정유산 한 종 성공 기록 — 2026-09-13 09:08 경상북도 안동시

작성 2026-09-13. **로그·요청 기록·당시 소스 스냅샷으로 확인한 것만** 적는다. 추정은 따로 표시한다.
계정 아이디·비밀번호는 적지 않는다(로그 인용에서도 지웠다).

---

## 0. 요약

- 「주변유적 받기」 한 번으로 **로그인부터 지도 적재까지 끝난 실행은 딱 한 번**이다. 2026-09-13 **09:08:56 ~ 09:09:33**(37초), 대상은 **경상북도 안동시 · 지정유산 1종**이었다.
- 받은 파일은 `지정유산_260913090927347.zip`(**177,003바이트**)이다. 풀어서 SHP 6종(도형 414개)을 `참조 지도 › 지정유산`에 올렸다.
- 그때 빌드는 09:07:10 빌드다. **내려받기를 새 탭이 아니라 본 창(`m_mainView`)에서 걸었다.** 그 이동으로 화면 준비 상태가 내려가지 않도록 `m_downloadNavigation` 표시를 달았다.
- 이전 인계 문서(`2026-09-13-heritage-handover-astra.md` 4장)에는 「받는 곳: 새 탭 = 유일한 성공 조합」이라고 적혀 있다. **그 기록은 이 실행과 맞지 않는다.** 새 탭은 08:53에 파일을 받았지만 흐름은 실패로 끝났다(1장 표).

---

## 1. 「성공」이 언제였나 — 08:53 과 09:08 을 구분한다

같은 날 아침 지정유산 1종으로 다섯 번 돌렸다. 빌드는 실행 직전 빌드이며, 대화 기록의 빌드 시각으로 짝지었다.

| 실행 | 지역 | 빌드 | 받는 곳 | 기다리는 방식 | 결과 |
|---|---|---|---|---|---|
| 08:53:36 | 제주시 | 08:51 (PID 22940) | **새 탭** | 고정 6틱(≈4초) | 08:53:40에 「파일이 오지 않았습니다」로 실패. **1초 뒤** 08:53:41에 271KB가 도착. 적재 안 됨 |
| 08:58:21 | 제주시 | 08:56 | 새 탭 | 완료 신호, 미시작 시 60틱(≈42초) | 08:59:03 「시작되지 않았습니다」로 실패. 파일은 **09:00:08**에 도착(서버 파일명 08:58:39) |
| 09:01:38 | 제주시 | 09:00 | 새 탭 | 같음 | 42초 동안 시작 안 됨. 파일 없음. 「내려받기 탭 응답」 줄도 없음 |
| 09:05:01 | 제주시 | 09:03 | **본 창** | 같음 | **5초 만에** 270,959바이트 수신. 그러나 흐름이 멈춰 09:05:59 단계 시간 초과 |
| **09:09:26** | **안동시** | **09:07** | **본 창 + 이동 표시** | 같음 | **요청 5초 뒤 수신 → 크기 확인 → 완료 → 적재** ✅ |

PID 22940(08:53) 때 「검사 16개 통과」라고 보고한 것은 **적재 검사**(사용자가 받아 둔 10MB 샘플 ZIP)였다. 인트라넷 다운로드 흐름이 성공했다는 뜻이 아니었다.

---

## 2. 성공 실행의 입력과 결과

| 항목 | 값 |
|---|---|
| 조사 파일 | `OneDrive\바탕 화면\안동\안동시.qgz` (09:08 새로 만든 조사) |
| 확인한 시·군 | 경상북도 안동시 |
| 받을 자료 | `{HeritageDataset::DesignatedHeritage}` 한 종 |
| 받는 자리 | `%LOCALAPPDATA%\ka-hgis\ka-hgis\주변유적\경상북도 안동시\원본\<UUID>\` (OneDrive 밖) |
| 검색 전/후 건수 | 8,302 → **248** |
| 받은 파일 | `지정유산_260913090927347.zip` · 177,003바이트 |
| ZIP SHA-256 | `befcbfba91949642cc6abfc44542aa70bbbd6710946ed37d130fd2c7a3dda2f8` |
| 풀린 자리 | `%LOCALAPPDATA%\ka-hgis\ka-hgis\주변유적\SHP\befcbfba…\payload\` + `manifest.json`(24개 파일, 원본 해시 일치) |
| 서약서 영수증 | `OneDrive\바탕 화면\안동\주변유적\receipts\서약서-20260913-090909.txt` |
| 진단 로그 | `%LOCALAPPDATA%\ka-hgis\ka-hgis\주변유적\경상북도 안동시\receipts\진단.txt` |

풀린 SHP 6종이다. 도형 수는 SHX 크기에서 셌다: (크기 − 100) ÷ 8.

| SHP | 도형 수 |
|---|---|
| 국가지정유산 | 72 |
| 시도지정유산 | 164 |
| 국가등록문화유산 | 2 |
| 시도등록문화유산 | 0 |
| 국가지정유산보호구역 | 32 |
| 시도지정유산보호구역 | 144 |

지도에 올라간 것은 사용자가 09:10에 「진행이 완료되었다」고 확인했다. 레이어별 도형 수는 앱 로그에 남지 않았다.

---

## 3. 당시 소스를 어디서 확인했나

작업 트리는 그 뒤 여러 번 바뀌었다(6종 확장, 숨은 창, 이후 다른 에이전트의 수정). 그래서 **Claude Code 파일 기록 스냅샷**으로 09:07 빌드 상태를 확인했다.

| 파일 | 스냅샷 | 근거 |
|---|---|---|
| `src/app/KaHeritageBrowser.cpp` | `8bd6ff898bb9acf6@v39` (09:06:59) | 마지막 수정 09:06:57(`m_downloadNavigation`), 다음 수정은 09:11 |
| `src/app/KaHeritageBrowser.h` | `839023c010575e01@v17` (09:06:59) | 같음 |
| `src/core/HeritageIntranetFlow.cpp` | `1bcfffe457d1a312@v32` (08:34:19) | 그 뒤 09:07까지 이 파일 수정 없음 |
| `src/core/HeritageImport.cpp` | `0e798f3612ef9853@v6` (08:50:48) | 그 뒤 09:07까지 수정 없음 |
| `src/app/MainWindow.cpp` | 스냅샷 없음 | 08:27 수정(로컬 저장·1종 한정) 뒤 09:11까지 수정 없음 |

스냅샷 위치: `C:\Users\kyi25\.claude\file-history\2e2c42ee-dd59-4933-9cd7-188d0a019bbe\`

---

## 4. 처리 순서 — 단계마다 한 일과 넘어간 근거

### 4.0 공통 틀

- **창**: `KaHeritageBrowser` 대화상자 하나. 웹 화면은 숨은 탭에 있고 「자세히」로만 펼친다.
- **프로필**: `QWebEngineProfile("ka-heritage")`. 이름 있는 프로필이라 쿠키가 앱을 다시 켜도 남는다.
  - Chromium은 `--no-sandbox`로 뜬다(`KaPortableRuntime::applyWebEngineFlags`).
- **페이지**: `HeritagePage`
  - `createWindow`는 사이트 팝업을 탭으로 받는다.
  - `javaScriptAlert`/`Confirm`은 자동으로 닫되 문구를 표시한다. 예: EPSG:5179 안내.
- **폴링**: 700ms 타이머가 `runStage()`를 부른다.
  - 단계마다 최대 75틱(≈52초)을 준다.
  - 단계에 들어가면 `m_settle` 3틱(≈2초)을 쉬고, 시·군 선택 전에는 6틱을 쉰다.
  - 스크립트가 도는 중이거나(`m_scriptInFlight`) 본 창이 준비 전이면(`m_pageReady == false`) 그 틱은 건너뛴다.
- **스크립트 실행**: 모든 단계 스크립트는 `wrap()` 안에서 돈다.
  - 프레임을 재귀로 훑는다(`cands()`).
  - **codedeta select가 있는 창**을 먼저 고르고 `document`를 그 창의 것으로 가린다.
  - 프레임 안으로 **이동하지 않는다.**
- **기록**: 스크립트 **결과만** `진단.txt`에 남긴다. 본문은 남기지 않는다(로그인 스크립트에 비밀번호가 있다).

### 4.1 준비 — MainWindow

1. 리본 「주변유적 받기」 → `fetchNearbyHeritage()`
   - 조사구역 레이어·도형과 조사 파일 경로를 확인한다.
   - 창을 먼저 띄운다.
2. `HeritageRegionResolver`가 조사구역으로 시·군을 판정한다. `openHeritageBrowserFor()`가 확인 창을 한 번 띄운다. → 「경상북도 안동시」
3. 받는 자리를 AppLocalData `주변유적/경상북도 안동시/원본`으로 정한다.
4. `setTarget(sido, city, {DesignatedHeritage})` → `start()`
   - 계정·시군·폴더가 있는지 확인한다.
   - 탭을 모두 비운다.
   - 첫 탭(`m_mainView`)에 로그인 주소를 연다.

### 4.2 단계별

로그의 계정 표기는 지웠다.

| # | 단계 | 한 일 (스크립트 / C++) | 넘어간 근거 | 로그 |
|---|---|---|---|---|
| ① | 로그인 | `loginProbeScript` → 아직 로그인 칸이 있으면 `loginScript`: `#userId`·`#pwd` 채우고 사이트 `goLogin()` 호출(RSA 암호화는 사이트가 함) | `submitted` | 09:08:59 `still-login` → `submitted` |
| ② | 튜토리얼 닫기 | `loginProbeScript`로 로그인 통과 확인 → `dismissTutorialScript`: **보이는** 「다시 보지 않기」의 **체크상자만** 체크, 「튜토리얼 나가기」 클릭 | 다음 틱에 `none`(보이는 튜토리얼 없음) | 09:09:02 `past-login`·`closed` → 09:09:03 `past-login`·`none` |
| ③ | 다운로드 화면 열기 | `downloadPageProbeScript`: codedeta select나 「서약서에동의」 글자가 있으면 `ready`. 없을 때만 `openDownloadPageScript`(`showAgreePopup()`) | **첫 확인에서 `ready`** — 이 실행에서는 `showAgreePopup`을 부르지 않았다 | 09:09:06 `ready` |
| ④ | 서약서 동의 | `agreeTermsScript`: 주변 글자에 「서약서에동의」가 있는 체크상자 체크 → 글자가 정확히 「확인」인 버튼 클릭. C++은 `captureOutline` + `agreementAccepted` → 영수증 저장 | `agreed` | 09:09:09 `agreed` |
| ⑤ | 자료 종류 선택 | `selectDatasetScript(지정유산)`: 당시 지정유산 탭 코드가 **빈 값**이라 글자가 정확히 「지정유산」인 `a/li/button`을 클릭. 이어서 `resultPageInfoScript`로 **검색 전 건수**를 읽어 `m_totalBeforeSearch`에 둔다 | `selected` + total > 0 | 09:09:12 `selected`, `{"rows":24,"lastPage":5,"total":8302}` |
| ⑥ | 시·군 선택 | 6틱 쉰 뒤 `selectRegionScript`: `bjdcd`(지도 패널) 제외. codedeta + 첫 옵션 「시도선택」 select를 찾고, **같은 폼 안** 다음의 「시/군/구선택」 select를 찾는다. 옵션 글자로 고르고 `input`·`change`·`onchange` 발생. `selected`면 `verifyRegionScript`로 **값이 남았는지 다시** 본다 | `ok:<시도값>\|<시군값>` | 09:09:16 `sigungu:0`(목록 아직 비어 있음) → 09:09:17 `selected` → `ok:470000,ADDR470000\|ADD1471700` |
| ⑦ | 검색 | `searchScript`: `#searchForm` 안의 「검색」 클릭. 없으면 codedeta와 「검색」을 함께 품은 가장 가까운 조상. **페이지 이동·form.submit 없음** | `searching` | 09:09:20 `searching` |
| ⑧ | 결과 모으기 | `resultPageInfoScript`: 행 수·쪽 수·「검색결과 N건」. **전국 가드**: total이 검색 전과 같으면 최대 20틱 기다렸다가 멈춤 | rows > 0, total(248) ≠ 검색 전(8302) | 09:09:23 `{"rows":24,"lastPage":5,"total":248}` |
| ⑨ | 내려받기 | 4.3 | 4.3 | 09:09:26 ~ 09:09:33 |
| ⑩ | 완료·적재 | `datasetReady` → `MainWindow::importHeritageDataset` → `HeritageImport::loadDataset` (4.4) | 4.4 | 09:09:33 `단계 → 완료` |

### 4.3 내려받기 — 성공의 핵심

**(1) 주소 재료를 페이지에서 날것으로 받는다** — `buildDownloadUrlScript("")`

- 사이트 함수 `searchGisChaRirList`는 부르지 않는다. 지도 스크립트 `bury/select.js`의 같은 이름 함수가 덮어써서 `TypeError`가 난다.
- `#searchForm`(또는 `codedetaCd0`)이 있는 문서를 찾는다.
- 시·도 `codedetaCd0`, 시·군 `codeCdSg0`의 값이 비었으면 `no-region`을 돌려 멈춘다(전국 방지).
- `#searchForm` 안의 `input/select/textarea`를 이름·값 쌍으로 모은다. submit·button과 체크 안 된 checkbox·radio는 뺀다.
- `codeCd` = 시·도 값의 쉼표 뒤(`ADDR470000`)를 붙인다.
- `tab` = `#mode` 값을 쓴다. 우리 탭 코드가 빈 값이었어도 페이지의 `#mode`가 `S`였다.
- `searchParams` = `지역:<시도 글자> <시군 글자> / `
- **인코딩하지 않고** JSON `{"path":…,"params":[[이름,값],…]}`으로 돌려준다.

**(2) C++이 한 번만 인코딩한다**

```cpp
QByteArray query;
for (pair : params) { query += toPercentEncoding(name) + '=' + toPercentEncoding(value); /* & 로 잇는다 */ }
QUrl url = portalUrl();
url.setPath(path);
url.setQuery(QString::fromUtf8(query), QUrl::StrictMode);
if (!url.isValid()) { fail(...); return; }
m_downloadMode = "all";
logLine("내려받기 주소/요청: ...");
m_downloadNavigation = true;        // ← 09:07 빌드에서 추가
if (m_mainView) m_mainView->load(url);   // ← 새 탭이 아니라 본 창
```

실제로 나간 주소는 다음과 같다(로그의 「내려받기 요청」).

```
https://intranet.gis-heritage.go.kr/user/data/heritageDownload/downloadFilesAll.do
  ?_csrf=1789258160772&dataSt=L&pageIndex=1&mode=S
  &cphNm=&jjgb=&specSubCd=&cphHoNmStart=&cphHoNmEnd=
  &codedetaCd=470000%2CADDR470000&codeCdSg=ADD1471700&SaveHtNm=
  &codeCd=ADDR470000&tab=S
  &searchParams=%EC%A7%80%EC%97%AD%3A%EA%B2%BD%EC%83%81%EB%B6%81%EB%8F%84%20%EC%95%88%EB%8F%99%EC%8B%9C%20%2F%20
```

| 파라미터 | 값 | 출처 |
|---|---|---|
| `_csrf` | `1789258160772` | `#searchForm` 숨은 칸 |
| `dataSt` / `pageIndex` / `mode` | `L` / `1` / `S` | `#searchForm` 숨은 칸 |
| `cphNm`·`jjgb`·`specSubCd`·`cphHoNmStart`·`cphHoNmEnd`·`SaveHtNm` | 빈 값 | `#searchForm` |
| `codedetaCd` | `470000,ADDR470000` | 시·도 select |
| `codeCdSg` | `ADD1471700` | 시·군 select |
| `codeCd` | `ADDR470000` | 스크립트가 붙임 |
| `tab` | `S` | `#mode` 값 |
| `searchParams` | `지역:경상북도 안동시 / ` | 스크립트가 붙임 |

인코딩 규칙은 `,`→`%2C`, `:`→`%3A`, `/`→`%2F`, 공백→`%20`, 한글→UTF-8 퍼센트다. `%25`(이중 인코딩)는 없다.

**(3) 본 창 이동이 흐름을 멈추지 않게 한다**

- `m_mainView`의 `loadStarted`에서 `m_downloadNavigation`이 켜져 있으면 **표시만 끄고 돌아간다.**
  - `m_pageReady`를 내리지 않는다.
  - 09:05 실행에는 이 처리가 없어 파일이 와도 폴링이 영영 건너뛰었다.
- 응답이 첨부 파일이면 Chromium은 화면을 바꾸지 않고 내려받기로 넘긴다. 작업 화면(검색 결과)은 그대로 남았다.

**(4) 파일을 받는다** — `handleDownload` (프로필 `downloadRequested`)

- `원본/<UUID>/` 폴더를 만든다. `suggestedFileName`을 안전한 이름으로 바꿔 넣고 `accept()`한다.
- `m_pendingDownloads++`, `m_downloadStarted = true`로 둔다.
  - 09:09:30 「내려받기 시작: 지정유산_260913090927347.zip」
- 받는 동안 `receivedBytesChanged`가 `m_waitTicks`를 0으로 되돌린다. 받는 중에는 단계 시간이 흐르지 않는다.
- `isFinishedChanged`에서 `DownloadCompleted`면 경로를 `m_currentFiles`에 넣고 본 창 외 탭을 닫는다.
  - 09:09:31 「받음: …」

**(5) 완료를 확인하고 넘긴다** — `runStage` Download

- `m_pendingDownloads > 0`이면 기다린다. 시간 제한은 없다.
- 시작했고 받는 중이 아니면 뒤따를 파일을 4틱(≈2~3초) 더 본다.
- 파일마다 크기를 확인한다. 0바이트면 지우고, 전부 비면 실패다.
  - 09:09:33 「확인: 지정유산 파일 1개 · 172KB」
- `emit datasetReady(지정유산, 파일들)` → 다음 종류가 없으니 `Done`, `allFinished`.
  - 로그 문구 「여섯 종을 모두 받았습니다」는 1종일 때도 그대로 찍혔다(문구 오류).

### 4.4 적재 — `HeritageImport::loadDataset` (08:50 판)

1. `archiveRoot` = AppLocalData `주변유적/SHP`(로컬)로 정한다.
2. `CPL_ZIP_ENCODING=CP949`를 걸고 `TopographicArchive::prepare(zip)`을 부른다. → `<ZIP SHA-256>/payload/`에 풀고 `manifest.json`을 쓴다.
   - ZIP 안의 이름이 CP949라 이 설정이 없으면 레이어 이름이 깨진다.
   - 설정은 가드가 원래대로 되돌린다.
3. 풀린 `.shp`마다 다음을 한다.
   - `prepareShapefileEncoding`: `.cpg`는 `CP949`다.
   - `QgsVectorLayer(shp, <파일 이름>, "ogr")`로 연다. 레이어 이름은 「국가지정유산」 같은 파일 이름 그대로다.
   - `chooseNameField`: 실제 필드 중 유적명 후보를 고른다.
   - `HeritageStyle::apply(layer, 지정유산, nameField)`
     - 6종 모두 **같은 색 `#8E44AD`**, 채움 없는 윤곽선이다.
     - 유적명별로 분류한다.
   - `HeritageSiteLegend::install`: 레이어창에는 이름만, 도면 범례에는 유적명 줄을 둔다.
   - 읽기 전용·참조 레이어로 표시한다.
4. `project->addMapLayers(layers, false)`로 한 번에 등록한다. 범례 자동 추가는 하지 않는다.
5. `참조 지도` 그룹, 그 아래 `지정유산` 그룹을 만들어 넣는다. 보이게, 접힌 채로 둔다.
6. MainWindow가 `applyLayerOrderToLabels`를 부르고 캔버스를 새로 그린다.

---

## 5. 이 성공을 만든 조건 — 확인된 것과 아닌 것

**로그로 확인된 것**

| 조건 | 근거 |
|---|---|
| 기다림은 시간이 아니라 **완료 신호** | 08:53은 고정 4초로 기다려 1초 차이로 놓쳤다. 09:09는 신호 기반이라 받았다 |
| 본 창 내려받기에는 **이동 표시**(`m_downloadNavigation`)가 있어야 흐름이 이어진다 | 09:05는 표시 없이 파일이 왔지만 멈췄다. 09:09는 표시가 있어 완료됐다 |
| 주소는 **C++에서 한 번만** 인코딩하고 `setQuery(StrictMode)` + `isValid()` | 09:05·09:09 두 번 모두 요청 5초 뒤 파일이 왔다 |
| 시·군은 **다시 확인한 뒤** 검색하고, **건수가 바뀐 것**을 보고 받는다 | 8302 → 248 |
| 저장·해제는 **로컬 AppData** | 177,003바이트 온전한 ZIP, 해시 일치 |
| ZIP 해제 시 **CP949** | 레이어 이름 정상 |

**확인되지 않은 것 (단정하지 말 것)**

- 본 창과 새 탭 중 **서버 응답에 차이가 있는지**는 모른다.
  - 새 탭도 08:53에는 5초 만에 파일을 받았다.
  - 새 탭으로 08:58에는 요청 1분 47초 뒤에야 받았고, 09:01에는 아예 못 받았다.
  - 표본은 다섯 번뿐이다.
- 09:01에 파일이 안 온 이유는 모른다. 그 뒤 「fromEncoded 주소가 무효였다」고 진단했지만, 같은 방식으로 08:53·08:58에는 파일이 왔으므로 **그 진단은 틀렸다.**
- ③에서 첫 확인이 바로 `ready`였던 이유는 모른다. 서약서 문구가 이미 DOM에 있었을 수 있다(추정). 이후 검증 문서는 숨겨진 오래된 폼을 도착으로 오판할 수 있다고 적었다.

---

## 6. 성공 직후 바뀐 것과 그 결과

같은 날 09:11부터 6종으로 넓히면서 한 번에 여러 가지를 바꿨다. **성공 조건과 달라진 점**만 적는다.

| 빌드 | 바뀐 것 (성공 빌드 대비) | 실행 | 결과 |
|---|---|---|---|
| 09:11 | 6종 전부 + 종류마다 내려받기 상태 초기화. 내려받기 방식은 성공 때와 같음 | 09:14:08 서귀포시 | 다운로드 화면 `not-yet`/`not-found` 반복 |
| 〃 | 〃 | 09:14:24 서귀포시(다시 누름) | 서약서 `agreed` 뒤 **자료 종류 선택에서 시간 초과**(첫 종류 지정유산) |
| 09:15 | 본 창 대신 **숨은 전용 창**에서 받기 | 09:17 서귀포시 | 파일은 왔으나 **0바이트** |
| 09:19 | 숨은 창 + `searchParams`의 `:`·`/`를 인코딩하지 않음 | 09:20 서귀포시, 09:22 포항시 | 0바이트 |
| 09:24 | **새 탭**으로 되돌림, 인코딩 원복 | 09:24:57 포항시 | 로그인은 됐으나 다운로드 화면 `not-yet`/`not-found` 반복(사용자 화면에는 404) |
| 09:27 | + 404면 한 번 홈으로 되돌림 | 09:28 포항시 | 0바이트 |

- 09:11 이후 실행 중 **성공 빌드와 내려받기 방식이 같은 것은 09:14 두 번뿐**이다. 둘 다 내려받기에 가기 전에 멈췄다.
- 그래서 「본 창 + 이동 표시」 방식이 6종에서 되는지는 **한 번도 시험되지 않았다.**
- 0바이트는 모두 성공 이후의 세션에서 났다. 받는 곳이 숨은 창일 때도, 새 탭일 때도 났다.

---

## 7. 그대로 재현하려면

1. `KaHeritageBrowser.cpp`의 내려받기·대기·이동 표시를 3장의 스냅샷(v39)과 같게 둔다.
2. `HeritageIntranetFlow.cpp`의 `buildDownloadUrlScript`·`selectRegionScript`·`verifyRegionScript`·`searchScript`를 v32와 같게 둔다.
3. 대상은 **1종**으로 둔다. 조사를 새로 만든 뒤 한 번 누른다(당시 조건).
4. `진단.txt`에서 4.2 표의 결과 줄이 같은 순서로 나오는지 대조한다. 특히 `내려받기 요청`부터 `확인: … KB`까지 5초 안팎인지 본다.

1종이 다시 같은 순서로 끝나면, 그 빌드에서 **대상만 6종으로 바꾸고 다른 것은 아무것도 바꾸지 않은 채** 한 번 돌린다. 첫째와 둘째 종류의 `내려받기 요청`·`받음`·파일 크기를 비교한다.

---

## 8. 이 실행에서 드러난 빈틈 (성공했지만 고칠 것)

- **서약서 영수증에 원문이 비어 있다.** 170바이트, 머리글만 있다.
  - `captureOutline`은 비동기다. 그런데 `agreementAccepted`를 바로 내보내 비어 있던 `m_lastOutline`이 넘어갔다.
  - 로그의 「[자료 종류 선택] 결과: 1469바이트」가 뒤늦게 온 그 원문 결과다.
- 영수증은 조사폴더(**OneDrive** 바탕 화면)에 저장됐다. 받은 자료·SHP는 로컬이다.
- 당시 로그와 영수증에 **계정 아이디가 찍혔다**(`describeForLog`). 지금 코드는 「저장된 계정 사용」으로 바뀌어 있다.
- 완료 문구 「여섯 종을 모두 받았습니다」가 받은 종류 수와 무관하게 나온다.
- 본 창에서 받으므로 서버가 파일 대신 HTML(404 등)을 주면 **작업 화면이 그 화면으로 바뀐다.** 이 실행에서는 일어나지 않았다.
