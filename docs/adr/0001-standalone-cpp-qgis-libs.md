# ADR 0001: C++ 독립 실행 + QGIS 라이브러리 링크 (Architecture B)

## Status

Accepted (user confirmed B)

### Current implementation note (2026-09-08)

Architecture B remains accepted. The C++17 / LTR package names below record the original baseline, not the current build prescription. Current `CMakeLists.txt` uses C++20, Qt6 and OSGeo4W `apps/qgis-dev`; `scripts/dev-env.ps1` discovers the installed SDK. Follow the current build and preserve its ABI. Do not downgrade the standard or switch SDK distributions to match this historical text.

## Context

- ArcGIS 라이선스 종료 → 필드 고고학 도면·문화재 제출 워크플로 대체 필요
- 사용자는 **C++로 전체 설계** 및 **전용 프로그램 형태**를 원함
- 후보:
  - **A** 공식 QGIS + 플러그인(Python/C++)
  - **B** C++/Qt 독립 앱이 QGIS 라이브러리 링크
  - **C** qgis/QGIS 소스 포크 후 삭제·개조

제약: Windows, 한국 발굴 도면 규정, 초보자 UI, GPLv2+ 준수, 장기 유지비.

## Decision

**Architecture B** 채택.

- 제품 바이너리: `ka-hgis.exe` (C++17 / Qt, QGIS LTR과 동일 Qt major)
- 링크: `qgis_core`, `qgis_gui` (필요 시 `qgis_analysis`)
- SDK: OSGeo4W **`qgis-ltr-dev`** (또는 동등 LTR dev 패키지)
- 런타임: `QGIS_PREFIX_PATH` / `OSGEO4W_ROOT`로 prefix·plugins·resources 탐지
- **포크하지 않음** (github.com/qgis/QGIS 모노레포를 제품 소스로 복사·삭제하지 않음)
- PROJ/GDAL/렌더러 **재구현 금지**
- 라이선스: **GPLv2+** (QGIS 링크 파생) — 소스 제공·About 고지

## Alternatives considered

| 옵션 | 결과 | 이유 |
| --- | --- | --- |
| A 플러그인 | 기각(주 제품) | 사용자 C++ 독립 앱 요구; A는 보조 배포 형태로만 잔존 가능 |
| C 포크 | 기각 | QGIS.org “Friends don’t fork”, TCO·보안 패치·Windows 패키징 부담 |

## Consequences

### Positive
- 창·메뉴·IA를 고고학 전용으로 완전 통제
- 검증된 지도·편집·CRS·레이아웃 엔진 재사용
- 업스트림 QGIS LTR 업그레이드 가능(재링크)

### Negative / Risks
- Windows에서 DLL·provider·env 설정이 까다로움 (초기 공수 큼)
- QGIS API 버전에 결합 → LTR 고정 및 회귀 테스트 필요
- 완전 단일 파일 정적 배포는 비현실적 → QGIS 런타임 동반 모델

### Compliance
- 배포 시 대응 소스 제공
- About에 QGIS/Qt/GPL 고지

## References

- QGIS.org enterprise guidance (fork 비권장)
- OSGeo4W custom C++ app patterns
- 계획: `.omo/plans/arcgis-to-qgis-archaeology-ux.md`
