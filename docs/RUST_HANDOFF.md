# Rust 전환 인계 기록 (2026-11-11 기준)

다른 컴퓨터에서 이어서 개발할 때 **이 문서부터** 읽는다. `docs/HANDOFF.md` 와 저장소 루트의 `CLAUDE.md` 는
C++ 판 시절 문서라 Rust 작업에는 맞지 않는 부분이 있다(빌드·시험 명령 등).

- 저장소: `https://github.com/kesekikwon-netizen/hgis-rust.git`
- 작업 가지: `feat/real-survey-file` (`main` 에는 아직 합치지 않았다)
- 요청 기록(사용자 말 그대로 + 진행 경과): `docs/intent/2026-11-11-rust-real-engine-foundation.md`
- 대조 기준인 C++ 앱: `https://github.com/kesekikwon-netizen/hgis.git` (읽기 전용 참고)
- 목표(사용자 지시): C++ 앱의 기능과 디자인을 그대로 Rust 로 옮긴다. 「없는 것은 설치하고 빌드하고 하나하나 대조해 가며 앱이 완성될 때까지」.

## 1. 다른 컴퓨터에서 시작하는 법

1. 준비물 (Windows 11)
   - Rust 1.91 (`rustup`), `rustup component add clippy`
   - Visual Studio **Build Tools 2022** (C++ 빌드 도구). 빌드는 그 환경 안에서 돌려야 한다.
   - WebView2 런타임(Windows 11 에는 기본으로 있다) — 국토정보플랫폼 받기에 쓴다.
   - (대조용, 선택) C++ 앱 빌드에는 OSGeo4W 의 `qgis-dev, qt6-devel, gdal-dev-devel, sqlite3-devel, pdal-dev` 가 필요하다. 아직 설치하지 못했다.
2. 받기: `git clone https://github.com/kesekikwon-netizen/hgis-rust.git` → `git checkout feat/real-survey-file`
3. 빌드·시험·검사 (PowerShell, 저장소 루트에서). Git Bash 로 감싸면 경로 오류가 난다.
   ```powershell
   $vc='"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1'
   cmd /c "$vc && cargo build -p strata-app -p strata-web 2>&1"      # 앱 ka-hgis.exe + 도우미 ka-hgis-web.exe
   cmd /c "$vc && cargo test --workspace 2>&1"                       # 마지막 결과: 460 passed
   cmd /c "$vc && cargo clippy --workspace --all-targets -- -D warnings 2>&1"
   ```
   - `ka-hgis-web.exe` 는 `ka-hgis.exe` **옆에** 있어야 한다(`cargo build -p strata-web` 가 같은 `target\debug` 에 만든다).
   - `cargo build -p A -p B --example X` 는 예제만 만든다. 도우미를 고쳤으면 `cargo build -p strata-web` 를 따로 돌린다.
4. 계정 (저장소에 없다 — 컴퓨터마다 따로 넣는다. 값은 절대 출력·기록·커밋하지 않는다)
   - 위치: `%LOCALAPPDATA%\ka-hgis\ka-hgis\` 의 `ka-hgis-vworld.ini`(VWorld 키), `vworld-account.ini`(VWorld 사이트 계정),
     `ngii-account.ini`(국토지리정보원 계정), `heritage-account.ini`(국가유산청). C++ 앱이 쓰는 것과 같은 파일이다.
   - 새 컴퓨터에는 C++ 앱의 계정 설정으로 넣거나 기존 컴퓨터의 그 폴더를 옮긴다.
5. 실제 파일로 확인하는 시험은 `target\probe\…` 의 받은 파일이 있을 때만 돈다(없으면 건너뛴다 — `target` 은 git 에 없다).
   - DEM 타일: `target\probe\dem\N36E127.tif` (Copernicus `Copernicus_DSM_COG_10_N36_00_E127_00_DEM.tif`)
   - 공개DEM: `target\probe\dem_survey\DEM\원본\(B080)공개DEM_36705_img_2025\36705\36705.img` (앱의 「DEM」 단추로 받은 것)

## 2. 꼭 알아야 할 것

- **「DEM」「수치지형」 단추는 누를 때마다 사용자 이름으로 국토정보플랫폼에 공식 신청 1건을 낸다.** 그래서 시험(`cfg!(test)`)과
  화면 캡처 실행(`--snap`)에서는 시작하지 않는다. 캡처에서 실제로 받으려면 `KA_HGIS_SNAP_REAL_ORDER=1` 을 준다.
  실제 신청을 내는 확인은 **사용자에게 먼저 묻는다**. 2026-11-11 에 공개DEM 신청이 3회 들어갔다(같은 범위).
- 화면 캡처: `target\debug\ka-hgis.exe --snap <폴더> --snap-steps "<동작>:<이름>,…"`, 환경 변수 `KA_HGIS_WINDOWED=1`.
  동작: 리본 단추 이름(예: `토양`, `DEM`) 또는 `@open_env`(`KA_HGIS_SNAP_OPEN`), `@zoom_out`, `@wait_overlay`, `@dem_env`(`KA_HGIS_SNAP_DEM`),
  `@wait_dem`, `@wait_topo`, `@cadastral_env`, `@trench_fill`, `@buffer500` (`crates/strata-app/src/snap_shot_run.rs`).
  캡처 뒤 앱이 스스로 안 끝나는 일이 있다(PNG 는 써진다) — 시간 제한을 두고 끈다.
- 규칙: Rust 파일 300줄 이하, `unsafe` 금지, clippy `-D warnings`. 원래 있던 시험의 기댓값을 바꾸면 커밋 메시지에 `시험 변경: <이유>`.
- 보안: 스크립트 실행·계정 입력은 https 의 `ngii.go.kr` 화면에서만(도우미와 화면 안에서 두 번 확인). 이동 허용은 `ngii.go.kr` 와
  정부 통합인증 `sso.anyid.go.kr`(지나가기만). 받는 파일은 믿는 사이트 것만, 덮어쓰지 않는다.
- 저장소의 `.md` 31개가 받자마자 「수정됨」으로 보인다(줄바꿈 설정 차이). 건드리지 않았다 — 커밋에 넣지 않는다.
- 올리기(push): 저장소 주인 계정 `kesekikwon-netizen` 만 쓰기 권한이 있다.

## 3. 어디까지 됐나

처음 대조(C++ 기능 150개): 동작 20 / 부분 58 / 껍데기 55 / 없음 13. 그 뒤 옮긴 것:

### 실제로 확인한 것 (명령 결과·화면 캡처·실제 서버)
- **조사 파일**: `.gpkg` 읽기·쓰기(C++ 견본과 호환, GDAL 로 확인), 열기·새 조사·저장·닫을 때 묻기, 최근 조사 목록.
- **지도 표시**: 조사 레이어 기본 모양·글자, 배경지도 타일(위성·지형), 참조 레이어(SHP·ZIP 끌어 놓기).
- **편집**: 선택·삭제·여러 개 선택, 겹친 곳 지우기, 합치기, 되돌리기/다시 하기, 500/1000 m 버퍼, 시굴 격자 채우기.
- **내보내기**: 제출 꾸러미(EPSG:5179 SHP + MANIFEST), 검수(체크리스트). 좌표 변환은 PROJ 와 1 mm 안.
- **자료 받기**
  - 지적도: VWorld 로그인 → 5 km 받기 → 지도에 올리기 (실제 서버).
  - 지질도(KIGAM), 토양도(흙토람), 수계도(VWorld): 단추 → 받기 → 색 입힌 면 (실제 서버).
  - **DEM(국토정보플랫폼 공개DEM)**: 단추 한 번 → 로그인 → 신청 → 도엽 2장 받기까지 실제로 됨. 받은 `.img` 읽기는 GDAL 값과 일치.
    두 장을 붙여 표고 색 + 음영으로 올린 화면 확인.
  - 수치지형도: 도엽 확인 → 자동 로그인 → 신청서 열림까지 실제로 됨(시험 프로그램 `ngii_portal_probe`).

### 만들었지만 실제로 확인하지 못한 것
- 「DEM」 단추 한 번으로 **받기부터 지도 표시까지 한 번에** 이어지는 것(받기와 표시는 따로 확인했다).
- 「수치지형」 단추의 신청 제출 → 파일 받기 → DXF 를 지도에 올리기(코드는 연결돼 있다. 받은 DXF 의 실제 모양·좌표계를 아직 못 봤다).

### 아직 안 된 것 (다음 순서)
1. 「수치지형」 단추 실제 받기 확인(신청 1건 — 사용자에게 먼저 묻는다). 그다음 지형지물 부호별 색·굵기, 지명, 점 기호, 받은 도엽 다시 쓰기.
2. 「DEM」 한 번 누르기 전체 흐름 확인, 레이어 목록의 DEM 항목·범례, 표시 방식(전국/저지대/화면 범위), 받은 DEM 다시 쓰기(다시 신청하지 않기).
3. 주변유적(국가유산청 인트라넷): C++ `HeritageIntranetFlow.cpp` 를 같은 도우미로 옮긴다. 지금 Rust 의 `heritage_http.rs` 는 가짜 결과가 섞여 있다.
4. 지적도 계정 창 자동 열기·다시 시도, 지질도 공식 도폭 색·범례.
5. 레이어 목록 ↔ 지도(순서, 삭제·복원, 레이어별 모양), 다른 형식 가져오기(DXF·GPKG·CSV).
6. 출력: PDF·인쇄·GeoTIFF·xlsx (지금은 껍데기).
7. 편집 도구 나머지: 폴리곤 나누기, 구간 분리, 정점 편집, ㄱㄴ자 트렌치, 지도 위 ↶↷ 단추.
8. C++ 앱을 빌드해 나란히 화면 대조(OSGeo4W 설치 필요 — 사용자가 설치).
9. 알려진 찌꺼기: 시험이 `%TEMP%` 에 임시 폴더를 남긴다, `--snap` 뒤 종료 멈춤, 비밀번호 저장(`strata-secrets/dpapi.rs`)이 진짜 DPAPI 가 아니라 C++ 이 못 읽는다.

## 4. 국토정보플랫폼 받기의 구조 (새로 만든 부분)

```
리본 단추(「DEM」/「수치지형」)
  └ strata-app: dem_download_ui_state.rs / topo_download.rs  ── 뒤 스레드 ──▶ dem_ngii_worker.rs::receive_from_portal
        └ strata-net: ngii_portal_runner.rs (도우미를 띄워 0.4초마다 스크립트를 시킴)
              ├ ngii_portal_flow.rs  (단계: 도엽 확인→로그인→신청→파일 준비→다운로드, C++ pollAutomation 그대로)
              └ ka-hgis-web.exe  (crates/strata-web, WebView2 — C++ 의 QWebEngineView 자리, 표준 입출력 JSON)
                    └ data/providers/ngii-topographic.js (C++ 과 같은 파일 + 사용목적 한 곳), ngii-dem.js (공개DEM 용, 새로 씀)
받은 파일 ─▶ strata-paleo: dem_hfa_read.rs(.img) → dem_shade.rs(색띠+음영) → strata-app: dem_layer.rs(지도에 그림)
           strata-storage: dxf_lines_reader.rs(DXF) → topo_download.rs(참조 레이어)
```

실제 사이트에서 알아낸 것(2026-11-11):
- 공개DEM 검색 `POST /ms/map/demQuery.do`, 내려받기 `downloadMapData('4','online',[{types:'index',years,nums,names}])`,
  제출 `/pd/shbtManage/shbtInsertTritVidoDem.do`, 전송 화면 `nlippd.ngii.go.kr/nlippd/pd/innorix/innorixdownLoad.do`.
- 로그인 길: `/mn/loginPopup.do` → `/anyid/common/login.do` → `sso.anyid.go.kr` → `/anyid/common/anyidLogin.do` → `www.ngii.go.kr/member/login.do`.
- 신청서의 사용목적 목록이 바뀌었다: 예전 「기타(BAAAC00008)」가 없고 「제작 및 활용」 아래 `CAAAC00001~7`.
  앱 기본값은 `CAAAC00006`(교육, 학술, 연구) — Claude 가 정한 값이고 사용자가 바꿀 수 있다. 생년월일은 신청서에 계정 값으로 미리 채워져 있다.
- 공개DEM 파일: `(B080)공개DEM_<도엽>_img_<연도>.zip` 안의 `<도엽>.img` — ERDAS HFA, Float32, 90 m, EPSG:5179, 자료 없음 -9999.

진단용: `cargo run -p strata-net --example ngii_portal_probe -- <ka-hgis-web.exe> <받을 폴더> <x5179> <y5179> <반경m>`
(기본은 신청서가 열리면 멈춘다. `KA_HGIS_PROBE_DEM=1` 공개DEM, `KA_HGIS_PROBE_SUBMIT=1` 실제 신청, `KA_HGIS_WEB_SHOW=1` 창 보이기,
`KA_HGIS_WEB_STDERR=1` 도우미 오류·막은 이동 보기).
