# 다음 AI에게 그대로 붙여 넣을 프롬프트

작성 2026-09-11. 계정·비밀번호·VWorld 키를 이 파일에 넣지 말 것.

```
D:\hgis 다. C++20 / Qt6.11 / QGIS(OSGeo4W, 이 PC는 D:\OSGeo4W).
고고학 현장 HGIS. 모델은 Grok만. 커밋·푸시·포터블·dist/L: 복사는 사용자가 말한 뒤에만.
기존 dirty/dist 를 reset·revert 하지 마라. 계정 값을 소스·로그·채팅에 출력하지 마라.

지금 일: 리본 「주변유적 받기」가 국가유산 GIS통합인트라넷에서
조사지역 시/군 SHP만 받아 「참조 지도」에 올리게 하는 것.
시/군보다 좁게 받거나 전국(~8300)을 받으면 실패다.

읽을 것 (이 순서):
1. docs/superpowers/plans/2026-09-11-heritage-next-ai-prompt.md  (이 프롬프트)
2. docs/superpowers/plans/2026-09-11-heritage-handover-grok.md
3. docs/superpowers/plans/2026-09-11-heritage-intranet-nearby-sites.md
4. .grok/NOW.md 맨 위 「2026-09-11 주변유적 프레임」
5. src/app/KaHeritageBrowser.cpp 의 runOnPreferredDocument
6. src/core/HeritageIntranetFlow.cpp 의 wrap / formHintScript / frameInventoryScript

완료 조건 한 줄:
시·군을 고른 뒤 검색으로 넘어가고, 실패 outline에 codedeta:true 이거나
검색결과가 8300에서 줄거나, 「시·군 조건이 걸리지 않았습니다」로 멈춘다.
전국 받기 금지.

이미 한 일 (푸시됨 또는 이어서 푸시할 워킹트리):
- 로그인 goLogin() RSA. 평문 POST 금지.
- 폼은 codedetaCd0/codedetaCd 만. 지도 bjdcd* 는 폼이 아니다.
- 검색/다운은 그 창의 W.searchGisChaRirList.call(W,…) 만.
  fn('searchGisChaRirList')·new Function·부모 메뉴 함수 금지.
- formHint 는 codedeta|other|error. 지정유산/발굴조사구역 글자 오탐 제거.
- requireForm 이면 codedeta 없는 문서에 검색을 넣지 않는다 (not-found).
- 검색 전후 건수가 같으면 전국 가드로 실패.
- QWebEngineFrame children() + findFrameByName.
- frameInventoryScript: iframe/frame 태그 name/id/src 만. 자식 document 금지.
- codedeta 가 C++ 프레임에 없고 html src 가 있으면 같은 호스트 URL을 탭으로 연다.
  경로 창작 금지. m_openedFrameSrc 한 번.
- fail outline에 cppFrames / jsFrames / cpp[] (url,name,htmlName,hint) / html[] 를 붙인다.
- runStage 는 매 poll ++m_waitTicks 먼저 (인플라이트여도 타임아웃).
- 세대 불일치 마지막 콜백에서 m_scriptInFlight 를 끈다.

확인된 산 증거 (2026-09-11):
- 11:15 완전 JSON: main.do, frames=2, selects는 bjdcd만, codedeta 없음,
  buttons=튜토리얼나가기/이전/다음. 화면에 보이던 8302는 그 document total에 없음.
- 11:37 화면: 다운로드 폼+검색결과 8,300 인데 outline codedeta:false.
- 단계 글자는 「강원특별자치도 강릉시」인데 스크립트는 지도 문서를 읽음.
- 12:48 이후 디스크에 새 pageOutline 덤프 없음. 산 검색 감소는 미증명.
- .unlazy/heritage-ch3/GATES.md G1–G4 pending. G4는 산 수동.

다음이 할 일 (한 목표만):
1. 옛 「주변유적 받기」 창과 ka-hgis 를 모두 닫고
   .\scripts\run-ka-hgis.ps1 또는 바탕 「고고학 전용 HGIS」로 새 exe.
2. 주변유적 받기를 시·군까지 다시 돌린다. AI가 사용자 앱을 조작하지 마라.
3. 실패/대기 outline에 cppFrames, cpp[].hint, html[].src 가 있는지 본다.
   없으면 옛 exe다.
4. codedeta:true 가 되고 검색 건수가 줄면 3장이 끝난 것이다.
   같으면 전국 가드 메시지를 유지한 채, html src 탭이 실제로 열렸는지 본다.
5. 선택자를 추측으로 늘리지 마라. bjdcd를 폼으로 인정하지 마라.

금지:
- 커밋·푸시·포터블·사용자 창 강제 종료(사용자가 다시 말하기 전)
- 계정 값 출력
- loadSurveyLayers 에서 removeAllMapLayers
- 업로드 CRS를 5179 아닌 값으로
- codedeta 별칭 확대, 부모 search 함수, new Function

빌드 (PowerShell):
  $env:PATH = "C:\Program Files\CMake\bin;" + $env:PATH
  . .\scripts\dev-env.ps1
  cmake --build --preset release --target ka-hgis ka_heritage_flow_tests
  .\build\Release\ka_heritage_flow_tests.exe -o build\qa\heritage-flow-iframe.txt,txt
슬롯은 22개여야 한다 (frameInventoryReadsIframeTagsWithoutEnteringThem 포함).

함정: C++ \s → \\s. QTest는 -o file,txt. WebEngine --no-sandbox.
createWindow 없으면 팝업 소실. 한 틱에 스크립트 둘 금지.
```
