# 주변유적 받기 — 인계 지침 (Grok 4.6 이어받기용)

작성 2026-09-11. 이 문서 하나만 읽고 이어서 개발할 수 있게 쓴다.
**확인한 사실**과 **아직 모르는 것**을 반드시 구분한다. 추측을 확인된 것처럼 쓰지 않는다.

---

## 0. 처음에 그대로 붙여 넣을 머리말

```
D:\hgis 프로젝트다. C++20 / Qt6 / QGIS(OSGeo4W) 로 만든 고고학 현장조사 GIS 앱이다.
지금 하는 일은 「주변유적 받기」 버튼 하나로 국가유산 GIS통합인트라넷에서
조사지역 시/군의 국가유산 SHP 를 받아 지도에 올리는 것이다.

읽을 문서:
- docs/superpowers/plans/2026-09-11-heritage-handover-grok.md  (이 문서, 최우선)
- docs/superpowers/plans/2026-09-11-heritage-intranet-nearby-sites.md (설계)

지켜야 할 것:
- 커밋·푸시하지 마라. 포터블을 만들지 마라. dist/ 나 L: 에 복사하지 마라. 따로 시킬 때만 한다.
- 기존 미커밋 작업이 많다. 전체 reset·revert·clean 으로 되돌리지 마라.
- 이 자료는 동국문화재연구원의 재산이다. 배포 경로(포터블·제출 SHP)에 절대 싣지 마라.
- 아이디·비밀번호를 소스·문서·로그·화면에 값으로 남기지 마라.
- **시/군까지만** 받는다. 읍면동·리로 쪼개거나 전국을 훑는 경로를 만들지 마라.
- 서약서 동의는 자동으로 하되 동의 시각과 원문을 영수증으로 남겨라.
- **실패를 성공으로 처리하지 마라.** 화면에서 못 찾으면 그 자리에서 멈추고 그렇다고 말해라.
- 앱을 빌드하려면 실행 중인 ka-hgis.exe 를 먼저 닫아야 한다(LNK1104).

빌드·검사 (PowerShell):
  . .\scripts\dev-env.ps1        # 이걸 먼저 안 하면 rcc.exe 에서 깨진다
  cmake --preset vs
  cmake --build --preset release
  ctest --preset release
```

---

## 1. 지금 상태 한 줄

로그인·서약·자료종류까지는 간다. **시·군 단계의 스크립트는 지도 document를 읽어 codedeta가 없다.**
화면에는 다운로드 폼과 검색결과 8,300이 보이지만 outline은 `main.do` 튜토리얼/지도다.
C++가 iframe `src`를 탭으로 여는 진단·우회를 넣었다. **산 검색 감소는 아직 미증명.**
다음 AI 붙여넣기: `docs/superpowers/plans/2026-09-11-heritage-next-ai-prompt.md`

---

## 2. 확인된 사이트 사실 (실제 DOM 으로 본 것만)

### 2.1 로그인 — `https://intranet.gis-heritage.go.kr/user/checkNet.do`

| 요소 | 값 |
|---|---|
| form | `name="loginForm"` · `action="/j_spring_security_check"` · POST |
| 아이디 | `input#userId` (text) |
| 비밀번호 | `input#pwd` (password) |
| 버튼 | `onclick="goLogin();"` |
| 숨은 칸 | `#RSAModulus` `#RSAExponent` `#USER_ID` `#USER_PW` |

**중요**: 브라우저에서 RSA 로 암호화한다. 평문을 `/j_spring_security_check` 에 POST 하면 로그인되지 않는다.
반드시 `#userId`·`#pwd` 를 채우고 **페이지의 `goLogin()` 을 부른다.**
이 규칙은 `tests/test_heritage_flow.cpp` 가 붙잡는다(직접 POST 코드가 들어오면 실패한다).

### 2.2 로그인 후 — `https://intranet.gis-heritage.go.kr/ngis/common/main.do?httpYn=Y`

- **프레임이 2개다.** 지도와 다운로드 패널이 서로 다른 창에 있다
- 로그인하면 **튜토리얼이 매번 뜬다**. 우측 위 「튜토리얼 나가기」, 가운데 「다시 보지 않기」 · 1/5 · 이전/다음
- 튜토리얼이 떠 있으면 상단 메뉴를 누를 수 없다

### 2.3 사이트 함수 (버튼을 찾는 것보다 이걸 직접 부르는 편이 확실하다)

| 동작 | 호출 |
|---|---|
| 다운로드 화면 진입 | `showAgreePopup()` |
| 검색 | `searchGisChaRirList('')` ← **추정**. 아래 2.5 참고 |
| 선택다운로드 | `searchGisChaRirList('', 'excel')` |
| 전체다운로드 | `searchGisChaRirList('', 'excel', 'all')` |
| 행별 다운로드 | `downloadFiles('one', '<번호>,CHL_SPCN_AS,<코드>', '<코드>')` |
| 탭 전환 | `changeTab('B')` 등 (아래 표) |

### 2.4 다운로드 폼의 칸 — **두 벌이 있다. 틀리면 전국을 받는다**

| id / name | 첫 옵션 | 정체 |
|---|---|---|
| `bjdcd1_newJbn` / `bjdCdNewAddr` | "시도 선택" | **왼쪽 지도의 지번검색. 여기가 아니다** |
| `bjdcd2_newJbn` / `bjdcd_newJbn` | "시구군 선택" | 지도 패널 짝 |
| `codedetaCd0` / `codedetaCd` | "시도선택" | **다운로드 폼. 여기다** |
| `jjgb0` / `jjgb` | "선택" | 지정구분 |
| `specSubCd0` / `specSubCd` | "선택" | 지정종목 |

DOM 순서는 지도 패널이 먼저다. **첫 옵션 글자만 보고 고르면 반드시 틀린다.**
현재 코드는 `codedeta` 로 먼저 찾고, 시/군/구는 **같은 form 안에서 그 다음 select** 로 잡는다.
「검색」 버튼도 지도 패널에 하나 더 있다 — 같은 함정이다.

### 2.5 탭 코드

| 자료 | 코드 |
|---|---|
| 매장유산유존지역 | `B` |
| 문화유적분포지도 | `U` |
| 지표조사구역 | `R` |
| 발굴조사구역 | `E` |
| 현상변경허용기준 | `P` |
| 지정유산 | **미확인** (글자로 찾는다) |

### 2.6 시/도 목록 (사이트 실제 값)

서울특별시 · 부산광역시 · 대구광역시 · 인천광역시 · 세종특별자치시 · 대전광역시 · 울산광역시 ·
경기도 · 충청북도 · 충청남도 · 경상북도 · 경상남도 · 제주특별자치도 · **강원특별자치도** ·
**전북특별자치도** · **전남광주통합특별시**

앱의 `KoreaRegionCatalog` 는 전라남도·광주광역시를 따로 두므로 **전남광주통합특별시를 못 찾는다.**
현재 코드는 별칭으로 처리했다(`HeritageIntranetFlow::selectRegionScript` 의 `merged()`).

### 2.7 내려받기를 누르면 JavaScript 알림이 뜬다

「국가유산 공간정보 좌표체계 안내 / EPSG: 5179」. **아무도 닫지 않으면 거기서 멈춘다.**
`HeritagePage::javaScriptAlert/Confirm` 에서 자동으로 닫고 글자는 화면에 남긴다.

---

## 3. 방금 고쳤고 **아직 산 검증 못 한 것** ← 여기서부터 시작하라

**증상 (2026-09-11 11:15·11:37 증거)**: 웹뷰는 다운로드 폼+8,300건.
아래 JSON은 `codedeta:false`, selects는 `bjdcd*`, buttons는 튜토리얼나가기/이전/다음.

**원인**: `QWebEnginePage::runJavaScript` 는 메인만. `children()` 이 다운로드 iframe을
안 주면 hint가 메인을 보고 `requireForm` 이 `not-found`. outline은 `requireForm=false`라 계속 지도.

**조치 (워킹트리 / 이 커밋)**:
- `frameInventoryScript` — 태그 src/name/id만. 자식 document 금지
- `runOnPreferredDocument` — 프레임별 hint, `cpp`/`html` 프로브
- codedeta 없으면 같은 호스트 iframe `src`를 탭으로 연다 (`m_openedFrameSrc`)
- `n<=1` 조건은 제거함. 지도 iframe만 `children()`에 있어도 src 탭을 연다
- `runStage` 매 poll `++m_waitTicks` 먼저
- 검색은 `W.searchGisChaRirList.call` 만. 전국 가드 유지

**해야 할 일**: 옛 창을 모두 닫고 새 exe로 다시 받기.
outline에 `cppFrames`/`cpp[].hint`/`html[].src` 가 보여야 새 빌드다.
검색 건수가 줄거나 「시·군 조건이 걸리지 않았습니다」면 3장 판정 가능.
선택자 추측·`bjdcd`를 폼으로 인정하는 것은 금지.

---

## 4. 만들어 둔 것

| 파일 | 하는 일 |
|---|---|
| `src/core/HeritageStyle.{h,cpp}` | 여섯 종 색·이름·탭 코드 표(SSOT), 유적명 카테고리 렌더러 |
| `src/core/HeritageSiteLegend.{h,cpp}` | 레이어창엔 종류 한 줄, 도면 범례엔 유적명 전부 |
| `src/core/HeritageRegionResolver.{h,cpp}` | 조사구역 → 시/군 (VWorld 좌표→주소, 실패 시 행정경계 레이어) |
| `src/core/HeritageIntranetSettings.{h,cpp}` | 인트라넷 계정(`heritage-account.ini`). NGII 계정과 **다른 자리** |
| `src/core/HeritageIntranetFlow.{h,cpp}` | 단계와 스크립트. `isVerified()` 로 확인 여부 표시 |
| `src/core/HeritageImport.{h,cpp}` | 받은 ZIP/SHP → 레이어 + 색 + 범례 + 「참조 지도」 그룹 |
| `src/app/KaHeritageBrowser.{h,cpp}` | 창. 탭·팝업·다운로드·단계 진행 |
| `MainWindow::fetchNearbyHeritage` | 리본 「좌표 정합」의 **「주변유적 받기」 버튼** |

검사 (전부 통과, 전체 40개 통과):
`tests/test_heritage_style.cpp`(11) · `test_heritage_region.cpp`(14) ·
`test_heritage_flow.cpp`(15) · `test_webengine_smoke.cpp`(2)

**계정은 이미 넣어 두었다.** `%LOCALAPPDATA%\ka-hgis\ka-hgis\heritage-account.ini` (리포 바깥).
값을 출력하지 마라.

---

## 5. 남은 일 (순서대로)

1. **3장 검증** — 검색이 실제로 걸리는지. 이게 되면 그다음이 줄줄이 풀린다
2. **받은 파일 확인** — `searchGisChaRirList(..., 'excel', 'all')` 의 `'excel'` 이 걸린다.
   SHP 묶음이 아니라 엑셀 목록이 오면 행별 `downloadFiles('one', ...)` 로 바꿔야 한다.
   행 인자 형식은 2.3 에 있다
3. **적재 확인** — `HeritageImport::loadDataset` 으로 레이어가 뜨는지, 유적명 컬럼이 실제로 무엇인지.
   `chooseNameField` 는 이름을 추측해 박지 않고 **실제 필드에서 고른다**. 못 고르면 그렇다고 말한다
4. **여섯 종 반복** 확인, 여러 쪽 넘기기 확인
5. **배포 금지 회귀 테스트** — 포터블·제출 SHP 에 안 실리는지, 시/군보다 좁은 요청 경로가 없는지

---

## 6. 이 코드에서 반드시 피해야 할 함정 (전부 실제로 당한 것)

1. **C++ 문자열 안의 `\s` 는 잘못된 이스케이프다.** `"replace(/\s+/g,'')"` 를 쓰면 MSVC 가 `s` 로 만들어
   정규식이 「s 를 지우는」 것이 된다. 화면 글자를 하나도 못 찾게 된다. 반드시 `\\s` 로 쓴다.
   `tests/test_heritage_flow.cpp::scriptsUseValidBackslashEscapes` 가 이걸 잡는다
2. **bash heredoc 이 `\\` 를 `\` 로 줄인다.** 파이썬 heredoc 으로 C++ 을 쓸 때
   `\n`·`\s` 가 실제 줄바꿈이나 깨진 이스케이프가 된다. `chr(92)` 로 역슬래시를 만들어라
3. **소스는 CRLF 다.** 파이썬으로 쓸 때 `newline=''` 로 읽고 쓴다.
   문자열 리터럴 안에 실제 CR/LF 가 들어가면 컴파일 오류이고, 떠돌이 CR 하나면
   git 이 파일 전체를 바뀐 것으로 본다
4. **QTest 결과는 `-o <파일>,txt` 로만 보인다.** stdout 리다이렉트로는 PASS/FAIL 줄이 안 나온다.
   비어 있다고 크래시라고 단정하지 마라
5. **WebEngine 은 `--no-sandbox` 가 없으면 모든 페이지 로드가 실패한다.**
   `KaPortableRuntime::applyWebEngineFlags()` 가 QApplication 전에 건다. `--disable-gpu` 는 무관하다.
   `tests/test_webengine_smoke.cpp` 가 지킨다
6. **`createWindow` 를 구현하지 않으면 사이트 팝업이 통째로 사라진다.** 이 사이트는 `window.open` 을 쓴다
7. **요소가 닫혀도 DOM 에 숨은 채 남는다.** 보이는 것만 세지 않으면 「아직 떠 있다」로 영영 멈춘다
8. **한 틱에 스크립트를 두 번 돌리려 하면 뒤엣것이 버려진다**(`m_scriptInFlight`).
   틱마다 반복 검사를 넣으면 본 작업이 영영 안 돈다 — 실제로 로그인이 이것 때문에 멈췄다

---

## 7. 막혔을 때 쓰는 도구

창 아래 칸에 **그 화면의 요약(JSON)** 이 나온다 — 버튼·링크 글자, select 의 id/name/첫옵션/**현재 선택값**,
「다운로드」·「마이페이지」 링크의 **href·onclick**, 검색결과 건수, 프레임 수.
`HeritageIntranetFlow::pageOutlineScript()` 가 만든다. 비밀번호·입력값은 담지 않는다.

**이 요약을 보고 선택자를 정하라. 추측으로 채우지 마라.** 이 문서의 2장 사실은 전부 이 요약에서 나왔다.
