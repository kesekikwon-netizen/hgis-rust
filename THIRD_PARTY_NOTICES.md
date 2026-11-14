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

Rust 판은 QGIS 나 OSGeo4W DLL 묶음을 링크하지 않는다. 네이티브 라이브러리와 자료 현황(측정 2026-11-14, 판은 `crates/strata-engine/Cargo.toml` 에 고정):

| 라이브러리 | 판 | 라이선스 | 정적 / DLL | 배포 여부 | 공식 근거·출처 |
| --- | --- | --- | --- | --- | --- |
| GDAL/OGR (`gdal-src` `0.3.0+3.12.1` 소스 빌드: 드라이버 `ESRI Shapefile`·`GPKG`/`SQLite`, `GDAL_USE_GEOS=OFF`, 외부 의존 없음) | 3.12.1 | 본체(`gcore`, `ogr`, `port`, `alg`, `gnm`, `appslib`, `ogr_Shape`, `ogr_GeoPackage`, `ogr_SQLite`, `gdal_MEM`, `gdal_vrt`): **MIT/X**. `gdal-src` 안 `source/LICENSE.TXT` 중 이 빌드에 실제로 컴파일되는 제3자 코드: `port/cpl_float.cpp`(**BSD-3-Clause**, Industrial Light & Magic), `port/cpl_minizip*`(**Info-ZIP**), `alg/thinplatespline.cpp`(**SunSoft** 허용 고지, Sun Microsystems), `alg/internal_libqhull`(`alg/libqhull`, **Qhull** 라이선스, C.B. Barber & The Geometry Center) — 비활성 드라이버(`hdf4`, `pcraster`, `grib`, `dxf`, `pdf`, `mrf`, `flatgeobuf`) 코드는 컴파일되지 않음. `GDAL_USE_INTERNAL_LIBS=ON` 으로 함께 정적 컴파일되는 내장 라이브러리: `ogr/ogrsf_frmts/shape/shapelib`(Shapelib 1.6, `MIT OR LGPL-2.0-or-later` 중 **MIT**), `ogr/ogrsf_frmts/geojson/libjson`(json-c, **MIT**), `frmts/zlib`·`contrib/infback9`(zlib 1.3.1, **zlib**), `frmts/png/libpng`(libpng 1.6.43, **libpng v2**), `frmts/jpeg/libjpeg`·`libjpeg12`(**IJG**), `frmts/gtiff/libtiff`(**libtiff** BSD 계열), `frmts/gtiff/libgeotiff`(**MIT/X** 계열), `third_party/LercLib`(Esri LERC, **Apache-2.0**), 머리말 전용 `third_party/fast_float`(**Apache-2.0 OR MIT OR BSL-1.0**)·`third_party/libdivide`(**BSL-1.0 OR zlib**) | 정적 | CI 전용 (`gdal` 기능에서만 링크; `strata-app` 은 지시 R2-3c 에서 켜며 그때 실행 파일에 동봉) | https://gdal.org/en/stable/license.html · `gdal-src` `source/LICENSE.TXT` |
| PROJ 9.8.1 `proj.db` (`crates/strata-engine/data/proj-9.8.1/proj.db`, 제출 SHP `.prj` 기준 자료) | PROJ 9.8.1 · EPSG Dataset v12.029 | PROJ 자료: **MIT**; **EPSG Geodetic Parameter Dataset v12.029**: EPSG 이용 약관(무료 사용·재배포, 출처 「EPSG Geodetic Parameter Dataset, IOGP」 표기, 자료를 고쳐 EPSG 이름으로 내지 않음) | 정적 내장 자료 (`include_bytes!`, 첫 사용 시 `proj.db` 로 기록; DLL 아님) | 저장소 동봉 · 실행 파일에는 현재 CI 전용 (`gdal` 기능에서만 포함; 지시 R2-3c 에서 앱 동봉) | https://proj.org/en/stable/about.html · https://epsg.org/terms-of-use.html · 출처 OSGeo4W `proj-runtime-data-9.8.1-1.tar.bz2` (sha256 `599b2307ed72001ab1b28b9cf4ac007523687571ee0cc1ec54c511a893f74f1d`), `proj.db` 10,280,960 바이트 (sha256 `37165492257d87ad504c0f7097b96f3bd504d1238bed292ecb73273ab5ae4ebf`) |
| PROJ (`proj-sys` `0.27.0` bundled 소스 빌드, 내장 `proj.db` 포함) | 9.6.2 (내장 EPSG v12.013, 2025-05-26) | **MIT** · 내장 `proj.db` 는 EPSG 이용 약관(무료 사용·재배포, 출처 「EPSG Geodetic Parameter Dataset, IOGP」 표기, 자료를 고쳐 EPSG 이름으로 내지 않음) | 정적 | 앱 동봉 (`ka-hgis.exe` 안에 포함) | https://proj.org/en/stable/about.html · https://epsg.org/terms-of-use.html |
| SQLite (`libsqlite3-sys` `0.30.1` bundled — `rusqlite`, PROJ, `gdal` 기능의 GDAL GPKG/SQLite 드라이버 공유) | 3.46.0 | **Public domain** | 정적 | 앱 동봉 (`ka-hgis.exe` 안에 포함; 빌드용 명령줄 도구 `sqlite3.exe` 3.46.0 은 CI·빌드 전용) | https://www.sqlite.org/copyright.html |
| GEOS (`geos-src` `0.2.5` 소스 빌드) | 3.15.1 | **LGPL-2.1** (알려진 항목 `LC2`, 지시 R2-3a 에서 변경 없음. 현재 앱 자체는 GPL-2.0-or-later 라 정적 링크 가능하며 받은 사람은 앱 소스와 GEOS 소스를 받을 수 있다; 카피레프트 제거 전 `LC2` 에서 전환 예정) | 정적 | 앱 동봉 (`ka-hgis.exe` 안에 포함) | https://libgeos.org/project/license/ |

Rust 래퍼·빌드 크레이트(`crates/strata-engine/Cargo.toml`):

| 크레이트 | 판 | 라이선스 | 공식 근거 |
| --- | --- | --- | --- |
| `proj` · `proj-sys` | 0.31.0 · 0.27.0 | MIT 또는 Apache-2.0 | https://github.com/georust/proj |
| `geos` · `geos-sys` · `geos-src` | 11.3.1 · 2.0.10 · 0.2.5 | 크레이트 MIT (컴파일되는 GEOS C++ 소스는 위 표의 LGPL-2.1) | https://github.com/georust/geos |
| `libsqlite3-sys` | 0.30.1 | 크레이트 MIT (컴파일되는 SQLite C 소스는 Public domain) | https://github.com/rusqlite/rusqlite |
| `link-cplusplus` | 1.0.12 | MIT 또는 Apache-2.0 | https://github.com/dtolnay/link-cplusplus |
| `gdal` · `gdal-sys` (기능 `gdal`) | 0.19.0 · 0.12.0 | MIT | https://github.com/georust/gdal |
| `gdal-src` (기능 `gdal`) | 0.3.0+3.12.1 | 빌드 스크립트 MIT, 동봉 GDAL 3.12.1 소스는 `source/LICENSE.TXT` (위 GDAL/OGR 행 참조) | https://github.com/georust/gdal |

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
