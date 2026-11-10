//! 오목 다각형을 **삼각형으로 쪼갠다**. 화면이 오목을 틀리게 칠하는 것을 막는다.
//!
//! ## 왜 필요한가
//!
//! egui 의 `Shape::convex_polygon` 은 이름대로 **볼록 다각형 전용**이다.
//! epaint 소스(`path_shape.rs`)에 *"Fill is only supported for convex polygons"* 라고
//! 적혀 있고, 실제 채움은 첫 꼭짓점에서 나머지로 부채꼴을 펴는 **삼각팬**이다
//! (`tessellator.rs` `fill_closed_path`). 그래서 오목한 부분이 메워진다.
//!
//! 그런데 검증 경로(`layer_engine::rasterize`)는 자체 스캔라인이라 오목도 정확하다.
//! 즉 **화면과 시험이 서로 다른 결과**를 낸다 — 시험은 통과하는데 화면은 틀린다.
//!
//! 실제로 이런 도형이 나온다: L자 시굴 트렌치(`strata_geo::trench_lshape`),
//! 클릭으로 그린 조사구역, GPKG 에서 읽은 필지.
//!
//! ## 어떻게 고치나
//!
//! 직접 삼각분할을 만들지 않는다. 워크스페이스에 이미 있는 `geo` 0.29 의
//! `TriangulateEarcut`(내부 `earcutr`)이 오목·구멍을 모두 처리한다.
//! 쪼갠 삼각형을 egui `Shape::Mesh` 로 넘기면 화면과 시험이 같은 결과가 된다.
//!
//! ## 좌표 정밀도
//!
//! 문서 Phase 2 요구대로 기준점(origin)을 **f64 에서 먼저 빼고** f32 로 바꾼다.
//! 그렇게 해야 고배율에서 꼭짓점이 떨리지 않는다(floating-origin jitter).

use geo::{Coord, LineString, Polygon, TriangulateEarcut};

/// 삼각형 하나. 화면 좌표(px).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenTriangle {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub c: [f32; 2],
}

/// 오목 다각형을 화면 좌표 삼각형들로 쪼갠다.
///
/// `ring` 은 외곽 링, `holes` 는 구멍 링들(지도 좌표 f64).
/// `to_screen` 은 지도 좌표 → 화면 좌표 변환(기준점 차감이 여기서 일어난다).
///
/// 삼각형이 하나도 안 나오면 빈 벡터를 준다(호출자가 원래 도형을 그대로 그릴 수 있게).
#[must_use]
pub fn triangulate_polygon<F>(ring: &[[f64; 2]], holes: &[Vec<[f64; 2]>], to_screen: F) -> Vec<ScreenTriangle>
where
    F: Fn([f64; 2]) -> [f32; 2],
{
    if ring.len() < 3 {
        return Vec::new();
    }
    let exterior: Vec<Coord<f64>> = ring.iter().map(|p| Coord { x: p[0], y: p[1] }).collect();
    let interiors: Vec<LineString<f64>> = holes
        .iter()
        .filter(|h| h.len() >= 3)
        .map(|h| LineString::new(h.iter().map(|p| Coord { x: p[0], y: p[1] }).collect()))
        .collect();
    let poly = Polygon::new(LineString::new(exterior), interiors);

    poly.earcut_triangles()
        .into_iter()
        .map(|t| ScreenTriangle {
            a: to_screen([t.v1().x, t.v1().y]),
            b: to_screen([t.v2().x, t.v2().y]),
            c: to_screen([t.v3().x, t.v3().y]),
        })
        .collect()
}

/// 꼭짓점 외적 부호가 섞이면 오목이다.
///
/// 볼록한 다각형은 굳이 쪼갤 필요가 없으므로(삼각팬이 정확하다) 호출 전에 이걸로
/// 판단하면 삼각분할 비용을 아낄 수 있다.
#[must_use]
pub fn is_concave(ring: &[[f64; 2]]) -> bool {
    if ring.len() < 4 {
        return false;
    }
    // 마지막 점이 첫 점과 같으면(닫힌 링) 중복을 빼고 본다.
    let mut pts: &[[f64; 2]] = ring;
    if pts.first() == pts.last() && pts.len() > 1 {
        pts = &pts[..pts.len() - 1];
    }
    let n = pts.len();
    if n < 4 {
        return false;
    }
    let (mut pos, mut neg) = (false, false);
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let c = pts[(i + 2) % n];
        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        if cross > 1e-12 {
            pos = true;
        } else if cross < -1e-12 {
            neg = true;
        }
        if pos && neg {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ㄷ자(오목) — 위쪽 가운데가 파여 있다.
    fn u_shape() -> Vec<[f64; 2]> {
        vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 100.0],
            [70.0, 100.0],
            [70.0, 30.0],
            [30.0, 30.0],
            [30.0, 100.0],
            [0.0, 100.0],
        ]
    }

    /// 항등 변환(화면 = 지도).
    fn ident(p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32, p[1] as f32]
    }

    fn tri_area(t: &ScreenTriangle) -> f32 {
        ((t.b[0] - t.a[0]) * (t.c[1] - t.a[1]) - (t.b[1] - t.a[1]) * (t.c[0] - t.a[0])).abs() * 0.5
    }

    #[test]
    fn detects_concave_shapes() {
        assert!(is_concave(&u_shape()), "ㄷ자는 오목");
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(!is_concave(&square), "정사각형은 볼록");
        // 닫힌 링(마지막 = 처음)도 판정이 같아야 한다.
        let mut closed = u_shape();
        closed.push(closed[0]);
        assert!(is_concave(&closed), "닫힌 ㄷ자도 오목");
    }

    /// 오목 다각형을 쪼개면 삼각형 **면적 합**이 원래 면적과 같다.
    ///
    /// 이게 핵심이다. 삼각팬으로 잘못 채우면 면적이 커진다(파인 홈이 메워지므로).
    #[test]
    fn triangulated_area_equals_polygon_area() {
        let ring = u_shape();
        let tris = triangulate_polygon(&ring, &[], ident);
        assert!(!tris.is_empty(), "삼각형이 나와야 한다");

        let sum: f32 = tris.iter().map(tri_area).sum();
        // ㄷ자 면적 = 100*100 - 40*70 = 7200
        assert!(
            (sum - 7200.0).abs() < 0.5,
            "삼각형 면적 합 {sum} 이 7200 이어야 한다(파인 홈이 빠져야 함)"
        );
    }

    /// 구멍이 있는 다각형도 처리한다.
    #[test]
    fn triangulation_handles_holes() {
        let outer = vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 100.0],
            [0.0, 100.0],
        ];
        let hole = vec![
            [30.0, 30.0],
            [70.0, 30.0],
            [70.0, 70.0],
            [30.0, 70.0],
        ];
        let tris = triangulate_polygon(&outer, &[hole], ident);
        let sum: f32 = tris.iter().map(tri_area).sum();
        // 10000 - 1600 = 8400
        assert!((sum - 8400.0).abs() < 0.5, "면적 합 {sum} 이 8400 이어야");
    }

    /// 볼록 다각형도 정상 동작한다(회귀 방지).
    #[test]
    fn triangulation_works_for_convex_too() {
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let tris = triangulate_polygon(&square, &[], ident);
        let sum: f32 = tris.iter().map(tri_area).sum();
        assert!((sum - 100.0).abs() < 0.01, "정사각형 면적 100");
    }

    /// 점이 3개 미만이면 아무 것도 안 낸다(패닉 금지).
    #[test]
    fn degenerate_input_returns_empty() {
        assert!(triangulate_polygon(&[], &[], ident).is_empty());
        assert!(triangulate_polygon(&[[0.0, 0.0], [1.0, 1.0]], &[], ident).is_empty());
    }

    /// 기준점을 f64 에서 먼저 빼면 f32 정밀도가 살아난다.
    #[test]
    fn origin_subtraction_keeps_f32_precision() {
        // 큰 좌표(한국 TM)에서 1cm 차이를 구분할 수 있어야 한다.
        let origin = [200_000.0_f64, 500_000.0];
        let ring = vec![
            [origin[0], origin[1]],
            [origin[0] + 10.0, origin[1]],
            [origin[0] + 10.0, origin[1] + 10.0],
            [origin[0], origin[1] + 10.0],
        ];
        let tris = triangulate_polygon(&ring, &[], |p| {
            // f64 에서 먼저 차감 → f32 변환(문서 Phase 2 요구)
            [(p[0] - origin[0]) as f32, (p[1] - origin[1]) as f32]
        });
        assert!(!tris.is_empty());
        let sum: f32 = tris.iter().map(tri_area).sum();
        assert!((sum - 100.0).abs() < 0.01, "차감 후 면적 {sum} 이 100");
        // 화면 좌표가 0..10 범위에 있어야 한다(큰 값이 f32 로 뭉개지지 않았다는 뜻).
        for t in &tris {
            for p in [t.a, t.b, t.c] {
                assert!((-0.5..=10.5).contains(&p[0]), "x={} 가 범위 밖", p[0]);
                assert!((-0.5..=10.5).contains(&p[1]), "y={} 가 범위 밖", p[1]);
            }
        }
    }
}
