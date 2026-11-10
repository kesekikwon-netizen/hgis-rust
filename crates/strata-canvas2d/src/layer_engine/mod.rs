//! 지도 레이어 엔진 — C++ 의 QGIS 레이어 시스템을 Rust 로 옮긴 것.
//!
//! ## 왜 필요한가
//!
//! C++ 은 QGIS 엔진 위에서 돈다: 레이어 객체(`QgsVectorLayer`, 906회 사용)가
//! 데이터·심볼·표시여부·z순서를 들고 있고, `QgsMapRenderer` 가 트리를 아래에서
//! 위로 그린다. Rust 에는 그 엔진이 없어서 지도를 **그리는 함수 안의 if 문**으로
//! 흉내 냈다. 그 결과:
//!
//! - 레이어를 꺼도 안 꺼졌다(패널과 그리기가 별개였다)
//! - 순서를 바꿀 수 없었다(코드 줄 순서가 곧 z순서였다)
//! - 레이어를 추가하려면 그리는 함수를 고쳐야 했다
//! - 새 조사에 빈 화면이 아니라 합성 그림이 나왔다(데이터 유무를 손으로 판단)
//!
//! ## 구성
//!
//! | 모듈 | 하는 일 | C++ 대응 |
//! |---|---|---|
//! | [`stage`] | 레이어 창고(추가·삭제·표시·z순서) | `QgsProject` + `QgsLayerTree` |
//! | [`style`] | 레이어별 기본 스타일 표 | `LayerStyleDefaults::forKey` |
//! | [`draw`] | 그리기 중립 명령(도형) | `QgsSymbol` 이 내는 QPainter 명령 |
//! | [`symbol`] | 지오메트리 + 스타일 → 도형 | `QgsSymbol::renderFeature` |
//! | [`renderer`] | 창고를 순서대로 그리기 | `QgsMapRenderer` |
//!
//! ## 경계
//!
//! 이 엔진은 **`egui` 에 의존하지 않는다.** 도형 목록(`DrawShape`)만 낸다.
//! 그래서 같은 엔진을 화면(egui)·PDF·SHP 어디에도 쓸 수 있다.
//! C++ 이 QGIS 없이는 못 하는 것과 반대다.

pub mod draw;
pub mod raster_paint;
pub mod rasterize;
pub mod renderer;
pub mod stage;
pub mod style;
pub mod symbol;
pub mod tessellate;

#[cfg(test)]
#[path = "raster_tests.rs"]
mod raster_tests;

pub use draw::{DrawShape, LayerDraw, ScreenTransform};
pub use rasterize::{rasterize, Raster};
pub use renderer::{count_shapes, hit_test, render_stage, RenderOptions};
pub use stage::{Feature, GeometryKind, LayerStage, MapLayer};
pub use style::{label_ko, DomainStyle, DOMAIN_LAYER_KEYS};
pub use symbol::{layer_labels, layer_to_shapes, DEFAULT_LABEL_SIZE_PX};
pub use tessellate::{is_concave, triangulate_polygon, ScreenTriangle};
