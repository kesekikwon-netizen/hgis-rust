# 제3자 고지 (ka-hgis)

이 앱은 OSGeo4W `qgis-dev`의 `qgis_core` / `qgis_gui`에 링크한다(Architecture B).  
앱 자체는 GNU GPLv2 이상이다. 짧은 고지는 `LICENSE`, **GPL v2 전문은 `COPYING`**(QGIS SDK `apps/qgis-dev/doc/LICENSE` 원문 그대로)이다. 아래는 **배포 시 같이 두는** 의존 고지다. 전문은 공식 URL과 포터블 `licenses/`를 본다.

측정 시점 2026-09-20. 이 PC 핀: `VERSION_QGIS_PIN.txt` — QGIS 4.3.0-Master, Qt 6.11.1 (OSGeo4W).  
오늘 공식 페이지: [QGIS GPL v2+·Qt 링크 예외](https://qgis.org/license/), [Qt 6.11 라이선스](https://doc.qt.io/qt-6/licensing.html), [Qt WebEngine](https://doc.qt.io/qt-6/qtwebengine-licensing.html), [GDAL MIT](https://gdal.org/en/stable/license.html), [PROJ MIT · contributors (2026)](https://proj.org/en/stable/about.html), [SQLite public domain](https://www.sqlite.org/copyright.html). API 키·계정은 이 파일에 없다.

## 앱이 링크하는 핵심

| 구성 | 라이선스 | 공식 근거 |
| --- | --- | --- |
| QGIS | GNU GPL v2 이상. Qt 링크 예외 | https://www.qgis.org/license/ |
| Qt 6 | LGPLv3 / GPLv2 / GPLv3 (모듈별). 상용 옵션 별도 | https://doc.qt.io/qt-6/licensing.html |
| Qt WebEngine / Chromium | Qt 쪽 LGPLv3 또는 GPLv2/3. Chromium은 구성 요소별(가장 제한적 LGPLv2.1) | https://doc.qt.io/qt-6/qtwebengine-licensing.html |
| GDAL/OGR | 대체로 MIT. 일부 파일 BSD 등. 빌드 의존에 따라 더 제한될 수 있음 | https://gdal.org/en/stable/license.html |
| PROJ | MIT (X/MIT). 인용: PROJ contributors (2026) | https://proj.org/en/stable/about.html |
| GEOS | LGPLv2.1 | 스플래시·정보 창 고지. https://libgeos.org/ |
| SQLite | Public domain | https://www.sqlite.org/copyright.html |
| Lucide 1.49.0 | ISC. Feather에서 온 일부 아이콘은 MIT. 전문은 `data/icons/lucide/LICENSE-lucide.txt` | https://github.com/lucide-icons/lucide |

Qt 제3자 구성 목록: https://doc.qt.io/QT-6/licenses-used-in-qt.html  
OSGeo4W에서 복사한 QGIS `LICENSE`/`AUTHORS`는 포터블 `licenses/QGIS-*`.

## Rust 앱(hgis-rust) 엔진 — `crates/strata-engine`

Rust 판은 QGIS 를 링크하지 않는다. 좌표 변환·도형 고치기 엔진은 아래 라이브러리를 실행 파일 안에 정적으로 넣는다(측정 2026-11-13, 판은 `crates/strata-engine/Cargo.toml` 에 고정). 앱 자체는 GPLv2 이상이므로 아래 라이선스와 함께 쓸 수 있다.

| 구성 | 판 | 라이선스 | 공식 근거 |
| --- | --- | --- | --- |
| PROJ (소스 빌드, 정적) | 9.6.2 | MIT | https://proj.org/en/stable/about.html |
| EPSG Geodetic Parameter Dataset (PROJ `proj.db` 안, 실행 파일에 들어 있음) | v12.013 (2025-05-26) | EPSG 이용 약관: 무료 사용·재배포, 출처 「EPSG Geodetic Parameter Dataset, IOGP」 표기, 자료를 고쳐 EPSG 이름으로 내지 않음 | https://epsg.org/terms-of-use.html |
| GEOS (소스 빌드, 정적) | 3.15.1 | LGPL-2.1. 정적 링크해도 GPLv2+ 앱에 넣을 수 있다(LGPL 2.1 제3조). 받은 사람은 앱 소스(GPL)와 GEOS 소스(https://libgeos.org/)를 받을 수 있다 | https://libgeos.org/project/license/ |
| SQLite (PROJ 가 proj.db 를 읽는 데 씀, libsqlite3-sys bundled) | 3.46.0 | Public domain | https://www.sqlite.org/copyright.html |
| Rust 크레이트 `proj` 0.31.0 · `proj-sys` 0.27.0 | — | MIT 또는 Apache-2.0 | https://github.com/georust/proj |
| Rust 크레이트 `geos` 11.3.1 · `geos-sys` 2.0.10 · `geos-src` 0.2.5 | — | MIT | https://github.com/georust/geos |
| Rust 크레이트 `libsqlite3-sys` 0.30.1 | — | MIT | https://github.com/rusqlite/rusqlite |
| Rust 크레이트 `link-cplusplus` 1.0.12 | — | MIT 또는 Apache-2.0 | https://github.com/dtolnay/link-cplusplus |

C++ 앱(OSGeo4W)은 PROJ 9.8.1(EPSG v12.029)·GEOS 3.15.0 이다. 한반도 기준점 변환은 1 mm 안, makeValid 결과는 정규화 비교로 같다(`crates/strata-engine/src/*_tests.rs`).

## 함께 두는 별도 프로그램

| 구성 | 라이선스 | 받은 파일·소스 주소·sha256 |
| --- | --- | --- |
| LibreDWG 0.14 (`tools/libredwg/dwg2dxf.exe`) | GPLv3 이상, 별도 프로세스(링크하지 않는다). 전문은 `tools/libredwg/COPYING` | https://github.com/LibreDWG/libredwg/releases/download/0.14/libredwg-0.14.tar.xz · sha256 `62ebb73b984f865960f20ed26619ea5f8789d5e3fd088fa40a2598384da81275`. 앱에 넣은 Windows 판: https://github.com/LibreDWG/libredwg/releases/download/0.14/libredwg-0.14-win64.zip · sha256 `1ad7e15344d20b3426c3435b078d82fb84b35062815946b2cca9c5fc9810fea8`. 자세한 내용은 `tools/libredwg/SOURCE.txt` |

## 글꼴

맑은 고딕(Malgun Gothic)은 **Windows에 설치된 글꼴을 이름만 지정**한다. 기본 화면 글꼴이다(아래 IBM Plex 는 선택 사항). 재배포 조건은 Microsoft 글꼴 라이선스를 따른다.

`data/fonts/` 에 동봉하는 글꼴 (2026-09-29, 사용자 승인으로 추가; 기본은 꺼져 있고 `KA_HGIS_BUNDLED_FONTS=1` 로 켠다):

| 글꼴 | 버전 | 출처 | 라이선스 |
|---|---|---|---|
| IBM Plex Sans KR (Regular·Medium·SemiBold·Bold, hinted TTF) | 1.1.0 | github.com/IBM/plex 릴리스 `@ibm/plex-sans-kr@1.1.0` | SIL Open Font License 1.1 — `data/fonts/LICENSE-IBM-Plex-Sans-KR.txt` |
| IBM Plex Mono (Regular·Medium·Bold, TTF) | 2.5.0 | github.com/IBM/plex 릴리스 `@ibm/plex-mono@2.5.0` | SIL Open Font License 1.1 — `data/fonts/LICENSE-IBM-Plex-Mono.txt` |

OFL 1.1 은 글꼴 파일을 소프트웨어와 함께 재배포하는 것을 허용한다(글꼴 자체를 단독 판매하지 않고, 이름을 바꾸어 개작하지 않는 한). 출처 URL 과 sha256 은 `data/fonts/README.md` 에 있다.

## 지도·자료 (런타임 다운로드)

VWorld, 국토정보플랫폼, 국가유산 공간정보, 흙토람, KIGAM 등은 **제공처 이용조건**이 별도이다. API 키는 앱에 넣지 않는다.

## 포터블 폴더에서 위치

- 이 파일: `THIRD_PARTY_NOTICES.md` (폴더 루트)
- 앱 GPL: `LICENSE`(고지), `COPYING`(GNU GPL v2 전문)
- QGIS 원본 고지: `licenses/QGIS-LICENSE`, `QGIS-AUTHORS`, `QGIS-CONTRIBUTORS` (`scripts/make-portable.ps1`이 복사)
- **번들 DLL 목록:** `licenses/BUNDLED-COMPONENTS.txt`. 폴더에 넣은 DLL 마다 어느 OSGeo4W 패키지(이름-판)에서 왔는지 적는다(OSGeo4W `etc/setup` 의 설치 목록으로 만든다). 목록에 없는 VC++ 런타임은 Windows System32 의 재배포 파일이다.
- **대응 소스:** `source/ka-hgis-source.zip` 에 그 EXE 를 만든 ka-hgis 소스(src·tests·data·cmake·scripts·CMake 파일·라이선스)를 함께 넣는다. 저장소 공개 여부와 상관없이 받은 사람이 소스를 갖는다. 판 식별은 `PORTABLE-MANIFEST.json` 의 `gitDescribe`·`executableSha256`.
- QGIS·Qt·GDAL 등 번들 라이브러리의 소스: `BUNDLED-COMPONENTS.txt` 의 패키지 이름-판으로 OSGeo4W 배포처(https://download.osgeo.org/osgeo4w/v2/)의 소스 묶음(`-src`)을 찾거나 각 프로젝트 공식 저장소의 같은 판을 받는다.
