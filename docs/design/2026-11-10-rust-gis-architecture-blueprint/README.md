# C++ QGIS → 순수 Rust GIS 재설계 도면

C++ QGIS 엔진(`qgis_core` / `qgis_gui`)으로 만든 앱을 순수 Rust 앱으로 옮길 때의
계층 구조(L0~L6), QGIS 클래스 1:1 매핑, 6단계 구축 순서, 기술 스택 결정과 벤치마크를
한 장에 담은 설계 문서다.

- 여는 법: [index.html](index.html) 을 브라우저로 연다. 그림은 같은 폴더 `images/` 에 있어 인터넷 없이도 보인다.
- 날짜: 2026-11-10

## 이 문서의 출처

원본은 Google Docs 에서 내보낸 `RustGIS.html`(137,828바이트)이다. 그 파일은 그림 경로가
`images/imageN.png` 로만 적혀 있고 그림 파일이 함께 오지 않아 **그림 15장이 모두 깨진 상태**였다.
본문 글자도 전부 `&#44036;` 형태의 숫자 엔티티였고, 인용 위첨자와 참고 자료 링크는
Google 리다이렉트 주소였다.

이 사본은 다음만 손댔다. **본문 내용은 고치지 않았다.**

| 한 일 | 내용 |
| --- | --- |
| 그림 넣기 | 첨부로 받은 그림 15장을 `images/` 에 넣고 이름을 뜻이 통하게 바꿨다(설계도 4 · 수식 2 · 본문 기호 9) |
| 글자 되돌리기 | 숫자 엔티티를 한국어 본문으로 디코드하고, 문단·표·목록 구조를 살렸다 |
| 인용 표시 | `10` 같은 위첨자를 `<sup class="cite">` 로 두고, 문서 끝 [참고 자료](index.html#refs) 60개와 이어지게 했다 |
| 링크 복원 | `google.com/url?q=…` 리다이렉트를 실제 주소로 바꿨다 |
| 양식 | Google Docs 클래스를 버리고 이 저장소 문서 양식(지층 띠·차례·표)으로 다시 입혔다 |

## 그림 목록

| 파일 | 무엇 |
| --- | --- |
| `images/fig-01-layer-mindmap.png` | 7계층(L0~L6) 크레이트 마인드맵 |
| `images/fig-02-cpp-to-rust-mapping.png` | QGIS C++ 클래스 10개 → Rust 대응표 |
| `images/fig-03-phase-roadmap.png` | Phase 1~6 착수 순서 + 런타임 양방향 파이프라인 |
| `images/fig-04-benchmarks.png` | 파일 포맷·공간 인덱스·좌표 변환 벤치마크 |
| `images/formula-forward.png`, `formula-inverse.png` | MapToPixel 순방향·역방향 식 |
| `images/sym-*.png` | 본문에 섞여 있던 수식 기호 9개 |

## 원본과 다른 점 하나

원본은 두 GitHub 저장소(`hgis.git`, `hgis-rust.git`)를 "외부에서 접근할 수 없는 상태"라고 적고
표준 QGIS API 와 공개 Rust GIS 프로젝트를 기준으로 썼다. 지금 이 저장소는 그 `hgis-rust` 이므로,
문서의 계층 이름(`hgis-geom`, `hgis-data` …)은 **제안 이름**이고 실제 크레이트 이름
(`strata-types`, `strata-storage`, `strata-canvas2d` …)과 다르다. 실제 현황은
[docs/ENGINE_DESIGN.md](../../ENGINE_DESIGN.md) 와 [docs/research/2026-11-10-gis-rendering-engine-research.md](../../research/2026-11-10-gis-rendering-engine-research.md) 를 본다.
